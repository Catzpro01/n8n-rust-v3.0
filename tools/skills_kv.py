#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
skills_kv.py — bangun, verifikasi, dan baca `cache.kv`.

`cache.kv` adalah *constant database* (format CDB milik D. J. Bernstein,
32-bit little-endian) yang berisi SELURUH skill dari
https://github.com/Catzpro01/antigravity-skills. Format ini dipilih karena:

  * lookup O(1): paling banyak 2 pembacaan acak (probe tabel hash -> record),
  * file bisa di-mmap langsung tanpa parsing apa pun, ukuran kecil,
  * standar terbuka: bisa dibaca dari Rust (crate `cdb`), Python (`pure-cdb`),
    C (tinycdb), dll. tanpa kode khusus.

Layout key (semua UTF-8):

  _meta                 JSON  info build, sumber, hasil pengecekan versi terbaru
  _index                JSON  daftar nama skill (terurut)
  _manifest             JSON  {nama -> metadata skill}
  skill:<nama>          isi SKILL.md apa adanya (byte asli upstream)
  desc:<nama>           deskripsi skill (teks) dari frontmatter
  meta:<nama>           JSON  metadata satu skill (sha, commit terakhir, lint, ...)
  file:<nama>/<path>    file tambahan milik skill (mis. scripts/sync_skills.py)

Perintah:

  build           clone upstream, cek versi terbaru vs GitHub, install ke skills/,
                  tulis cache.kv
  check-updates   bandingkan cache.kv (atau skills/) dengan HEAD upstream saat ini
  verify          verifikasi integritas cache.kv (hash, index, manifest, skills/)
  list | get | search | info | keys
  install         ekstrak semua skill dari cache.kv ke direktori (mis. registry
                  Antigravity: ~/.gemini/config/skills)

Hanya butuh Python 3.8+ standard library.
"""

import argparse
import hashlib
import io
import json
import mmap
import os
import re
import shutil
import struct
import subprocess
import sys
import tempfile
import urllib.error
import urllib.request
from datetime import datetime, timezone

DEFAULT_REPO = "Catzpro01/antigravity-skills"
DEFAULT_BRANCH = "main"
DEFAULT_KV = "cache.kv"
DEFAULT_SKILLS_DIR = "skills"
UPSTREAM_JSON = "UPSTREAM.json"
SCHEMA_VERSION = 1
GENERATOR = "tools/skills_kv.py"

KEY_META = b"_meta"
KEY_INDEX = b"_index"
KEY_MANIFEST = b"_manifest"


# --------------------------------------------------------------------------- #
# Util
# --------------------------------------------------------------------------- #
def now_iso():
    epoch = os.environ.get("SOURCE_DATE_EPOCH")
    if epoch:
        return datetime.fromtimestamp(int(epoch), tz=timezone.utc).isoformat(timespec="seconds")
    return datetime.now(tz=timezone.utc).isoformat(timespec="seconds")


def sha256_hex(data):
    return hashlib.sha256(data).hexdigest()


def git_blob_sha(data):
    """SHA-1 blob persis seperti yang dihitung git (dipakai untuk membandingkan
    dengan GitHub Trees API tanpa perlu mengunduh isi file)."""
    h = hashlib.sha1()
    h.update(b"blob %d\0" % len(data))
    h.update(data)
    return h.hexdigest()


def dumps(obj):
    return json.dumps(obj, ensure_ascii=False, sort_keys=True, separators=(",", ":")).encode("utf-8")


def die(msg, code=2):
    print("ERROR: " + msg, file=sys.stderr)
    sys.exit(code)


def run(cmd, cwd=None):
    return subprocess.run(cmd, cwd=cwd, check=True, capture_output=True, text=True).stdout


# --------------------------------------------------------------------------- #
# CDB — writer & reader (implementasi mandiri, tanpa dependency)
# Spesifikasi: https://cr.yp.to/cdb/cdb.txt
# --------------------------------------------------------------------------- #
def cdb_hash(data):
    h = 5381
    for b in data:
        h = (((h << 5) + h) ^ b) & 0xFFFFFFFF
    return h


def write_cdb(path, items):
    """Tulis CDB secara atomik. `items` = iterable (key: bytes, value: bytes).
    Key harus unik. Key diurutkan supaya output deterministik."""
    items = sorted(items, key=lambda kv: kv[0])
    seen = set()
    buf = io.BytesIO()
    buf.write(b"\0" * 2048)  # header: 256 x (pos u32, len u32), diisi belakangan
    pos = 2048
    tables = [[] for _ in range(256)]
    for key, val in items:
        if key in seen:
            raise ValueError("duplicate key: %r" % key)
        seen.add(key)
        h = cdb_hash(key)
        buf.write(struct.pack("<II", len(key), len(val)))
        buf.write(key)
        buf.write(val)
        tables[h & 0xFF].append((h, pos))
        pos += 8 + len(key) + len(val)

    header = []
    for table in tables:
        n = 2 * len(table)
        slots = [(0, 0)] * n
        for h, p in table:
            i = (h >> 8) % n
            while slots[i][1] != 0:
                i = (i + 1) % n
            slots[i] = (h, p)
        header.append((pos, n))
        for h, p in slots:
            buf.write(struct.pack("<II", h, p))
        pos += 8 * n
    if pos > 0xFFFFFFFF:
        raise ValueError("CDB tidak boleh lebih besar dari 4 GiB")

    buf.seek(0)
    for p, n in header:
        buf.write(struct.pack("<II", p, n))
    data = buf.getvalue()

    tmp = path + ".tmp"
    with open(tmp, "wb") as f:
        f.write(data)
        f.flush()
        os.fsync(f.fileno())
    os.replace(tmp, path)
    return len(data)


class CdbReader(object):
    """Pembaca CDB berbasis mmap: get() = O(1), iterasi urut sesuai file."""

    def __init__(self, path):
        self.path = path
        self._f = open(path, "rb")
        try:
            self._mm = mmap.mmap(self._f.fileno(), 0, access=mmap.ACCESS_READ)
        except ValueError:
            self._f.close()
            raise ValueError("%s: file kosong / bukan CDB" % path)
        if len(self._mm) < 2048:
            self.close()
            raise ValueError("%s: terlalu kecil untuk CDB" % path)

    def close(self):
        try:
            self._mm.close()
        finally:
            self._f.close()

    def __enter__(self):
        return self

    def __exit__(self, *exc):
        self.close()

    def get(self, key):
        if isinstance(key, str):
            key = key.encode("utf-8")
        mm = self._mm
        h = cdb_hash(key)
        tpos, tlen = struct.unpack_from("<II", mm, (h & 0xFF) * 8)
        if tlen == 0:
            return None
        start = (h >> 8) % tlen
        for i in range(tlen):
            slot = tpos + ((start + i) % tlen) * 8
            sh, sp = struct.unpack_from("<II", mm, slot)
            if sp == 0:
                return None
            if sh != h:
                continue
            klen, vlen = struct.unpack_from("<II", mm, sp)
            if klen == len(key) and mm[sp + 8:sp + 8 + klen] == key:
                return bytes(mm[sp + 8 + klen:sp + 8 + klen + vlen])
        return None

    def get_text(self, key):
        v = self.get(key)
        return None if v is None else v.decode("utf-8", errors="replace")

    def get_json(self, key):
        v = self.get(key)
        return None if v is None else json.loads(v.decode("utf-8"))

    def items(self):
        mm = self._mm
        end = struct.unpack_from("<I", mm, 0)[0]  # posisi tabel #0 = akhir area record
        pos = 2048
        while pos < end:
            klen, vlen = struct.unpack_from("<II", mm, pos)
            key = bytes(mm[pos + 8:pos + 8 + klen])
            val = bytes(mm[pos + 8 + klen:pos + 8 + klen + vlen])
            yield key, val
            pos += 8 + klen + vlen

    def keys(self):
        for k, _ in self.items():
            yield k

    def __len__(self):
        return sum(1 for _ in self.items())


# --------------------------------------------------------------------------- #
# SKILL.md — frontmatter & lint
# --------------------------------------------------------------------------- #
FM_RE = re.compile(r"\A---[ \t]*\r?\n(.*?)\r?\n---[ \t]*(?:\r?\n|\Z)", re.S)
CTRL_RE = re.compile(rb"[\x00-\x08\x0b\x0c\x0e-\x1f]")


def parse_frontmatter(text):
    """Parser YAML minimal untuk frontmatter skill: mendukung skalar biasa,
    multi-baris, dan block scalar (>, >-, |, |-). Cukup untuk name/description."""
    m = FM_RE.match(text)
    if not m:
        return {}
    lines = m.group(1).split("\n")
    out = {}
    i = 0
    while i < len(lines):
        km = re.match(r"^([A-Za-z_][\w-]*):[ \t]*(.*)$", lines[i])
        if not km:
            i += 1
            continue
        key, rest = km.group(1), km.group(2).rstrip()
        i += 1
        cont = []
        while i < len(lines) and (lines[i][:1] in (" ", "\t") or lines[i].strip() == ""):
            cont.append(lines[i])
            i += 1
        while cont and cont[-1].strip() == "":
            cont.pop()
        bm = re.match(r"^([>|])([+-]?)[ \t]*$", rest)
        if bm:
            sep = " " if bm.group(1) == ">" else "\n"
            val = sep.join(c.strip() for c in cont).strip()
        else:
            val = " ".join([rest] + [c.strip() for c in cont]).strip()
            if len(val) >= 2 and val[0] == val[-1] and val[0] in "\"'":
                val = val[1:-1]
        out[key] = val
    return out


def lint_bytes(data):
    issues = []
    hits = list(CTRL_RE.finditer(data))
    if hits:
        lines = sorted({data.count(b"\n", 0, h.start()) + 1 for h in hits})
        chars = sorted({"\\x%02x" % h.group(0)[0] for h in hits})
        issues.append(
            "control characters %s on line(s) %s (kemungkinan bug escape '\\a','\\b','\\f' di generator upstream)"
            % (",".join(chars), ",".join(map(str, lines)))
        )
    try:
        data.decode("utf-8")
    except UnicodeDecodeError:
        issues.append("bukan UTF-8 valid")
    if b"\r\n" in data:
        issues.append("CRLF line endings")
    return issues


# --------------------------------------------------------------------------- #
# Git & GitHub
# --------------------------------------------------------------------------- #
def repo_url(repo):
    return "https://github.com/%s.git" % repo


def clone_upstream(repo, branch, dest):
    # Full clone (bukan shallow) supaya tanggal commit terakhir per skill bisa dihitung.
    run(["git", "clone", "--quiet", "--branch", branch, repo_url(repo), dest])


def git_head_info(src):
    return {
        "commit": run(["git", "rev-parse", "HEAD"], src).strip(),
        "commit_date": run(["git", "log", "-1", "--format=%cI"], src).strip(),
        "tree": run(["git", "rev-parse", "HEAD^{tree}"], src).strip(),
    }


def git_last_commit_per_topdir(src):
    """{top_dir -> {"sha","date"}} = commit terbaru yang menyentuh direktori itu."""
    out = run(["git", "log", "--format=%x01%H %cI", "--name-only"], src)
    result = {}
    cur = None
    for line in out.splitlines():
        if line.startswith("\x01"):
            sha, date = line[1:].split(" ", 1)
            cur = {"sha": sha, "date": date}
            continue
        line = line.strip()
        if not line or cur is None or "/" not in line:
            continue
        result.setdefault(line.split("/", 1)[0], cur)
    return result


def github_token():
    tok = os.environ.get("GITHUB_TOKEN") or os.environ.get("GH_TOKEN")
    if tok:
        return tok
    if shutil.which("gh"):
        try:
            tok = subprocess.run(["gh", "auth", "token"], capture_output=True, text=True, timeout=10).stdout.strip()
            return tok or None
        except Exception:
            pass
    return None


def api_get(url):
    headers = {"User-Agent": "skills-kv/%d" % SCHEMA_VERSION, "Accept": "application/vnd.github+json"}
    tok = github_token()
    if tok:
        headers["Authorization"] = "Bearer " + tok
    req = urllib.request.Request(url, headers=headers)
    with urllib.request.urlopen(req, timeout=30) as resp:
        return json.loads(resp.read().decode("utf-8"))


def fetch_remote_state(repo, branch):
    """Ambil HEAD upstream + SHA setiap blob (tanpa mengunduh isi file)."""
    c = api_get("https://api.github.com/repos/%s/commits/%s" % (repo, branch))
    sha = c["sha"]
    t = api_get("https://api.github.com/repos/%s/git/trees/%s?recursive=1" % (repo, sha))
    if t.get("truncated"):
        raise RuntimeError("GitHub tree response truncated; repo terlalu besar untuk satu request")
    blobs = {e["path"]: e["sha"] for e in t["tree"] if e["type"] == "blob"}
    return {
        "commit": sha,
        "commit_date": c["commit"]["committer"]["date"],
        # Trees API menggemakan SHA yang diminta (commit), jadi ambil tree SHA dari objek commit.
        "tree": c["commit"]["tree"]["sha"],
        "blobs": blobs,
    }


def skill_blobs_only(blobs):
    """Hanya file di dalam direktori skill (abaikan file root seperti .gitignore)."""
    tops = {p.split("/", 1)[0] for p in blobs if "/" in p}
    skill_tops = {t for t in tops if (t + "/SKILL.md") in blobs}
    return {p: s for p, s in blobs.items() if "/" in p and p.split("/", 1)[0] in skill_tops}


def diff_blobs(local, remote):
    """Bandingkan {path->blob_sha}. Hasil per path & ringkasan per skill."""
    local = skill_blobs_only(local)
    remote = skill_blobs_only(remote)
    changed = sorted(p for p in local if p in remote and local[p] != remote[p])
    new_upstream = sorted(p for p in remote if p not in local)
    removed_upstream = sorted(p for p in local if p not in remote)
    per_skill = {}
    for p in changed:
        per_skill.setdefault(p.split("/", 1)[0], set()).add("outdated")
    for p in new_upstream:
        top = p.split("/", 1)[0]
        per_skill.setdefault(top, set()).add("new-upstream" if (top + "/SKILL.md") not in local else "outdated")
    for p in removed_upstream:
        top = p.split("/", 1)[0]
        per_skill.setdefault(top, set()).add("removed-upstream" if (top + "/SKILL.md") not in remote else "outdated")
    return {
        "up_to_date": not (changed or new_upstream or removed_upstream),
        "changed": changed,
        "new_upstream": new_upstream,
        "removed_upstream": removed_upstream,
        "skills": {k: sorted(v) for k, v in sorted(per_skill.items())},
        "checked_local_files": len(local),
        "checked_remote_files": len(remote),
    }


# --------------------------------------------------------------------------- #
# Discovery & entri KV
# --------------------------------------------------------------------------- #
def discover_skills(src):
    """[(name, [(relpath, bytes), ...])] untuk setiap direktori yang punya SKILL.md."""
    skills = []
    for name in sorted(os.listdir(src)):
        d = os.path.join(src, name)
        if name.startswith(".") or not os.path.isdir(d) or not os.path.isfile(os.path.join(d, "SKILL.md")):
            continue
        files = []
        for root, dirs, fnames in os.walk(d):
            dirs[:] = sorted(x for x in dirs if x not in ("__pycache__", ".git"))
            for fn in sorted(fnames):
                if fn.endswith((".pyc", ".tmp")):
                    continue
                full = os.path.join(root, fn)
                rel = os.path.relpath(full, d).replace(os.sep, "/")
                with open(full, "rb") as f:
                    files.append((rel, f.read()))
        skills.append((name, files))
    return skills


def build_entries(skills, last_commits):
    """Susun semua (key, value) + manifest dari daftar skill."""
    entries = []
    manifest = {}
    problems = []
    for name, files in skills:
        fmap = dict(files)
        body = fmap["SKILL.md"]
        fm = parse_frontmatter(body.decode("utf-8", errors="replace"))
        fm_name = fm.get("name", "").strip()
        desc = re.sub(r"\s+", " ", fm.get("description", "")).strip()
        if fm_name != name:
            problems.append("%s: frontmatter name=%r tidak sama dengan nama folder" % (name, fm_name))
        if not desc:
            problems.append("%s: description kosong" % name)

        extra = []
        lint = ["SKILL.md: " + x for x in lint_bytes(body)]
        for rel, data in files:
            if rel == "SKILL.md":
                continue
            key = "file:%s/%s" % (name, rel)
            entries.append((key.encode("utf-8"), data))
            extra.append({"path": rel, "key": key, "size": len(data),
                          "sha256": sha256_hex(data), "blob_sha": git_blob_sha(data)})
            lint += ["%s: %s" % (rel, x) for x in lint_bytes(data)]

        meta = {
            "name": name,
            "description": desc,
            "path": name + "/SKILL.md",
            "size": len(body),
            "sha256": sha256_hex(body),
            "blob_sha": git_blob_sha(body),
            "last_commit": last_commits.get(name),
            "files": extra,
            "lint": lint,
        }
        manifest[name] = meta
        entries.append((("skill:" + name).encode("utf-8"), body))
        entries.append((("desc:" + name).encode("utf-8"), desc.encode("utf-8")))
        entries.append((("meta:" + name).encode("utf-8"), dumps(meta)))
    return entries, manifest, problems


def local_blob_map_from_manifest(manifest):
    blobs = {}
    for name, m in manifest.items():
        blobs[m["path"]] = m["blob_sha"]
        for f in m.get("files", []):
            blobs["%s/%s" % (name, f["path"])] = f["blob_sha"]
    return blobs


# --------------------------------------------------------------------------- #
# Perintah: build
# --------------------------------------------------------------------------- #
def install_tree(skills_dir, skills, force):
    """Tulis skill ke skills_dir. Direktori lama hanya dihapus bila dikelola
    tool ini (ada UPSTREAM.json), kosong, atau --force."""
    if os.path.isdir(skills_dir):
        managed = os.path.isfile(os.path.join(skills_dir, UPSTREAM_JSON))
        if not managed and os.listdir(skills_dir) and not force:
            die("%s sudah ada dan bukan hasil tool ini (tidak ada %s). Pakai --force untuk menimpa."
                % (skills_dir, UPSTREAM_JSON))
        shutil.rmtree(skills_dir)
    os.makedirs(skills_dir)
    n = 0
    for name, files in skills:
        for rel, data in files:
            dst = os.path.join(skills_dir, name, *rel.split("/"))
            os.makedirs(os.path.dirname(dst), exist_ok=True)
            with open(dst, "wb") as f:
                f.write(data)
            n += 1
    return n


def cmd_build(args):
    repo, branch = args.repo, args.branch
    tmp = None
    try:
        if args.source:
            src = os.path.abspath(args.source)
            if not os.path.isdir(os.path.join(src, ".git")):
                die("--source harus berupa checkout git dari %s" % repo)
            print("Sumber   : checkout lokal %s" % src)
        else:
            tmp = tempfile.mkdtemp(prefix="antigravity-skills-")
            src = os.path.join(tmp, "repo")
            print("Sumber   : clone %s (branch %s) ..." % (repo_url(repo), branch))
            clone_upstream(repo, branch, src)

        head = git_head_info(src)
        last_commits = git_last_commit_per_topdir(src)
        skills = discover_skills(src)
        if not skills:
            die("tidak ada skill (folder dengan SKILL.md) ditemukan di %s" % src)
        print("HEAD     : %s (%s)" % (head["commit"], head["commit_date"]))
        print("Skill    : %d folder, %d file" % (len(skills), sum(len(f) for _, f in skills)))

        entries, manifest, problems = build_entries(skills, last_commits)
        for p in problems:
            print("PERINGATAN: " + p)
        if problems and not args.force:
            die("frontmatter bermasalah; perbaiki di upstream atau pakai --force", 1)

        # --- cek versi terbaru: setiap blob lokal harus identik dengan HEAD upstream sekarang
        freshness = {"verified": False, "method": None, "checked_at": now_iso(), "result": "skipped"}
        if args.no_remote_check:
            print("Cek versi: DILEWATI (--no-remote-check)")
        else:
            print("Cek versi: membandingkan setiap blob dengan HEAD upstream via GitHub API ...")
            try:
                remote = fetch_remote_state(repo, branch)
            except (urllib.error.URLError, RuntimeError, KeyError) as e:
                die("gagal menghubungi GitHub API (%s). Ulangi, atau pakai --no-remote-check." % e)
            diff = diff_blobs(local_blob_map_from_manifest(manifest), remote["blobs"])
            freshness.update({
                "verified": True,
                "method": "git blob SHA-1 setiap file dibandingkan dengan GitHub Trees API pada HEAD %s/%s" % (repo, branch),
                "remote_head": remote["commit"],
                "remote_head_date": remote["commit_date"],
                "remote_tree": remote["tree"],
                "files_checked": diff["checked_local_files"],
                "result": "up-to-date" if diff["up_to_date"] and remote["commit"] == head["commit"] else "mismatch",
            })
            if remote["commit"] != head["commit"] or not diff["up_to_date"]:
                print_diff(diff)
                die("sumber (%s) TIDAK sama dengan HEAD upstream (%s). Upstream berubah saat build; ulangi build."
                    % (head["commit"][:12], remote["commit"][:12]), 1)
            print("Cek versi: OK — %d/%d file identik dengan HEAD upstream %s; semua %d skill versi terbaru"
                  % (diff["checked_local_files"], diff["checked_remote_files"], remote["commit"][:12], len(skills)))

        # --- install ke skills/
        nfiles = install_tree(args.skills_dir, skills, args.force)
        upstream = {
            "source": {"repo": repo, "url": repo_url(repo), "branch": branch,
                       "commit": head["commit"], "commit_date": head["commit_date"], "tree": head["tree"]},
            "installed_at": now_iso(),
            "freshness": freshness,
            "skill_count": len(skills),
            "files": local_blob_map_from_manifest(manifest),
        }
        with open(os.path.join(args.skills_dir, UPSTREAM_JSON), "w", encoding="utf-8") as f:
            json.dump(upstream, f, ensure_ascii=False, indent=2, sort_keys=True)
            f.write("\n")
        print("Install  : %d file -> %s/" % (nfiles, args.skills_dir))

        # --- tulis cache.kv
        names = sorted(manifest)
        meta = {
            "schema_version": SCHEMA_VERSION,
            "format": "cdb",
            "format_note": "D. J. Bernstein constant database, 32-bit little-endian; bisa dibuka reader CDB apa pun",
            "generator": GENERATOR,
            "built_at": now_iso(),
            "source": upstream["source"],
            "freshness": freshness,
            "skill_count": len(names),
            "file_count": sum(len(f) for _, f in skills),
            "keys": {
                "_meta": "JSON build/source/freshness info", "_index": "JSON [skill names]",
                "_manifest": "JSON {name: meta}", "skill:<name>": "SKILL.md bytes",
                "desc:<name>": "description text", "meta:<name>": "JSON meta",
                "file:<name>/<path>": "extra file bytes",
            },
            "lint_summary": {n: m["lint"] for n, m in manifest.items() if m["lint"]},
        }
        entries.append((KEY_META, dumps(meta)))
        entries.append((KEY_INDEX, dumps(names)))
        entries.append((KEY_MANIFEST, dumps(manifest)))
        size = write_cdb(args.kv, entries)
        print("Cache    : %s (%d key, %s)" % (args.kv, len(entries), human(size)))

        ok = verify_kv(args.kv, args.skills_dir, quiet=True)
        if not ok:
            die("verifikasi cache.kv gagal setelah build", 1)
        print("Verify   : OK")
        if meta["lint_summary"]:
            print("Lint     : %d skill mengandung karakter kontrol dari upstream (lihat `info`)" % len(meta["lint_summary"]))
    finally:
        if tmp:
            shutil.rmtree(tmp, ignore_errors=True)


def human(n):
    for unit in ("B", "KiB", "MiB", "GiB"):
        if n < 1024 or unit == "GiB":
            return "%.1f %s" % (n, unit) if unit != "B" else "%d B" % n
        n /= 1024.0


def print_diff(diff):
    for p in diff["changed"]:
        print("  OUTDATED        %s" % p)
    for p in diff["new_upstream"]:
        print("  NEW UPSTREAM    %s" % p)
    for p in diff["removed_upstream"]:
        print("  REMOVED UPSTREAM %s" % p)


# --------------------------------------------------------------------------- #
# Perintah: check-updates
# --------------------------------------------------------------------------- #
def load_installed_state(args):
    """Ambil (source_commit, {path->blob_sha}) dari cache.kv, atau skills/UPSTREAM.json."""
    if os.path.isfile(args.kv):
        with CdbReader(args.kv) as db:
            meta = db.get_json(KEY_META)
            manifest = db.get_json(KEY_MANIFEST)
        if not meta or not manifest:
            die("%s tidak berisi _meta/_manifest" % args.kv)
        return args.kv, meta["source"], local_blob_map_from_manifest(manifest)
    up = os.path.join(args.skills_dir, UPSTREAM_JSON)
    if os.path.isfile(up):
        with open(up, encoding="utf-8") as f:
            data = json.load(f)
        return up, data["source"], data["files"]
    die("tidak ditemukan %s maupun %s" % (args.kv, up))


def cmd_check_updates(args):
    where, source, local = load_installed_state(args)
    repo, branch = source.get("repo", args.repo), source.get("branch", args.branch)
    try:
        remote = fetch_remote_state(repo, branch)
    except (urllib.error.URLError, RuntimeError, KeyError) as e:
        die("gagal menghubungi GitHub API: %s" % e)
    diff = diff_blobs(local, remote["blobs"])
    print("Terpasang : %s @ %s (%s)  [%s]" % (repo, source["commit"][:12], source.get("commit_date", "?"), where))
    print("Upstream  : %s @ %s (%s)" % (repo, remote["commit"][:12], remote["commit_date"]))
    print("File dicek: %d lokal vs %d upstream" % (diff["checked_local_files"], diff["checked_remote_files"]))
    if args.json:
        print(json.dumps({"installed": source, "remote": {k: v for k, v in remote.items() if k != "blobs"},
                          "diff": diff}, indent=2, ensure_ascii=False))
    if diff["up_to_date"]:
        note = "" if remote["commit"] == source["commit"] else "  (commit upstream berbeda tetapi isi semua skill identik)"
        print("Status    : UP-TO-DATE — semua skill adalah versi terbaru%s" % note)
        return 0
    print("Status    : ADA PEMBARUAN — %d skill terdampak:" % len(diff["skills"]))
    for name, st in diff["skills"].items():
        print("  %-40s %s" % (name, ", ".join(st)))
    if args.verbose:
        print_diff(diff)
    print("Jalankan : python %s build   # untuk memperbarui skills/ dan %s" % (GENERATOR, args.kv))
    return 1


# --------------------------------------------------------------------------- #
# Perintah: verify
# --------------------------------------------------------------------------- #
def verify_kv(kv_path, skills_dir=None, quiet=False):
    say = (lambda *a: None) if quiet else print
    try:
        return _verify_kv(kv_path, skills_dir, say)
    except (ValueError, KeyError, TypeError, struct.error, UnicodeDecodeError, OSError) as e:
        # file terpotong / bit-flip / bukan CDB: laporkan sebagai kegagalan, bukan traceback
        return _report(["%s tidak bisa dibaca sebagai cache.kv yang valid: %s: %s" % (kv_path, type(e).__name__, e)], say)


def _verify_kv(kv_path, skills_dir, say):
    errors = []
    with CdbReader(kv_path) as db:
        meta = db.get_json(KEY_META)
        index = db.get_json(KEY_INDEX)
        manifest = db.get_json(KEY_MANIFEST)
        if not (meta and index is not None and manifest is not None):
            return _report(["_meta/_index/_manifest hilang"], say)
        if sorted(manifest) != index:
            errors.append("_index tidak sama dengan kunci _manifest")
        if meta.get("skill_count") != len(index):
            errors.append("_meta.skill_count=%r != %d" % (meta.get("skill_count"), len(index)))
        expected_keys = {KEY_META, KEY_INDEX, KEY_MANIFEST}
        for name in index:
            m = manifest[name]
            body = db.get("skill:" + name)
            if body is None:
                errors.append("%s: key skill: hilang" % name)
                continue
            expected_keys.add(("skill:" + name).encode())
            if sha256_hex(body) != m["sha256"] or len(body) != m["size"]:
                errors.append("%s: sha256/size SKILL.md tidak cocok dengan manifest" % name)
            if git_blob_sha(body) != m["blob_sha"]:
                errors.append("%s: blob_sha tidak cocok" % name)
            if db.get_text("desc:" + name) != m["description"]:
                errors.append("%s: desc: tidak cocok" % name)
            expected_keys.add(("desc:" + name).encode())
            if db.get_json("meta:" + name) != m:
                errors.append("%s: meta: tidak cocok dengan _manifest" % name)
            expected_keys.add(("meta:" + name).encode())
            fm = parse_frontmatter(body.decode("utf-8", errors="replace"))
            if fm.get("name", "").strip() != name:
                errors.append("%s: frontmatter name berbeda" % name)
            for f in m.get("files", []):
                data = db.get(f["key"])
                expected_keys.add(f["key"].encode())
                if data is None or sha256_hex(data) != f["sha256"]:
                    errors.append("%s: file %s hilang/rusak" % (name, f["path"]))
            if skills_dir and os.path.isdir(skills_dir):
                for rel, expect in [("SKILL.md", body)] + [(f["path"], db.get(f["key"])) for f in m.get("files", [])]:
                    p = os.path.join(skills_dir, name, *rel.split("/"))
                    if not os.path.isfile(p):
                        errors.append("%s/%s tidak ada di %s" % (name, rel, skills_dir))
                    else:
                        with open(p, "rb") as fh:
                            if fh.read() != expect:
                                errors.append("%s/%s berbeda antara %s dan %s" % (name, rel, skills_dir, kv_path))
        actual_keys = set(db.keys())
        if actual_keys != expected_keys:
            errors.append("key tak terduga/hilang: %s" % sorted(actual_keys ^ expected_keys)[:5])
        # setiap key harus bisa di-lookup lewat hash table (bukan hanya lewat iterasi)
        for k, v in db.items():
            if db.get(k) != v:
                errors.append("lookup hash gagal untuk %r" % k)
                break
        say("cache.kv : %s  (%d key, %s, %d skill, upstream %s)" % (
            kv_path, len(actual_keys), human(os.path.getsize(kv_path)), len(index), meta["source"]["commit"][:12]))
        if skills_dir and os.path.isdir(skills_dir):
            on_disk = {n for n in os.listdir(skills_dir) if os.path.isfile(os.path.join(skills_dir, n, "SKILL.md"))}
            if on_disk != set(index):
                errors.append("isi %s berbeda dengan index: %s" % (skills_dir, sorted(on_disk ^ set(index))[:5]))
    return _report(errors, say)


def _report(errors, say):
    if errors:
        for e in errors:
            say("FAIL: " + e)
        say("verify   : GAGAL (%d masalah)" % len(errors))
        return False
    say("verify   : OK")
    return True


def cmd_verify(args):
    return 0 if verify_kv(args.kv, None if args.no_skills_dir else args.skills_dir) else 1


# --------------------------------------------------------------------------- #
# Perintah baca: list / get / search / info / keys / install
# --------------------------------------------------------------------------- #
def open_kv(args):
    if not os.path.isfile(args.kv):
        die("%s tidak ada. Jalankan: python %s build" % (args.kv, GENERATOR))
    return CdbReader(args.kv)


def cmd_list(args):
    with open_kv(args) as db:
        index = db.get_json(KEY_INDEX)
        if args.json:
            print(json.dumps({n: db.get_text("desc:" + n) for n in index}, indent=2, ensure_ascii=False))
            return 0
        width = max(len(n) for n in index)
        for n in index:
            d = db.get_text("desc:" + n) or ""
            if not args.full and len(d) > 100:
                d = d[:97] + "..."
            print("%-*s  %s" % (width, n, d))
        print("(%d skill)" % len(index))
    return 0


def cmd_get(args):
    with open_kv(args) as db:
        if args.meta:
            m = db.get_json("meta:" + args.name)
            if m is None:
                die("skill '%s' tidak ada" % args.name, 1)
            print(json.dumps(m, indent=2, ensure_ascii=False))
            return 0
        body = db.get("skill:" + args.name) if not args.file else db.get("file:%s/%s" % (args.name, args.file))
        if body is None:
            die("'%s' tidak ada di %s" % (args.name if not args.file else args.name + "/" + args.file, args.kv), 1)
        sys.stdout.buffer.write(body)
        if not body.endswith(b"\n"):
            sys.stdout.buffer.write(b"\n")
    return 0


def cmd_search(args):
    terms = [t.lower() for t in args.terms]
    hits = []
    with open_kv(args) as db:
        for name in db.get_json(KEY_INDEX):
            desc = (db.get_text("desc:" + name) or "").lower()
            body = (db.get_text("skill:" + name) or "").lower()
            score = 0
            for t in terms:
                if t in name.lower():
                    score += 3
                if t in desc:
                    score += 2
                score += min(body.count(t), 5) * 0.2
            if score > 0 and (not args.all or all(t in body or t in name.lower() for t in terms)):
                hits.append((score, name, db.get_text("desc:" + name)))
    hits.sort(key=lambda h: (-h[0], h[1]))
    for score, name, desc in hits[: args.limit]:
        print("%-40s %5.1f  %s" % (name, score, (desc or "")[:90]))
    if not hits:
        print("tidak ada skill yang cocok dengan: %s" % " ".join(args.terms))
        return 1
    return 0


def cmd_info(args):
    with open_kv(args) as db:
        meta = db.get_json(KEY_META)
        if args.json:
            print(json.dumps(meta, indent=2, ensure_ascii=False))
            return 0
        s, fr = meta["source"], meta["freshness"]
        print("file        : %s (%s, %d key)" % (args.kv, human(os.path.getsize(args.kv)), len(db)))
        print("format      : %s — %s" % (meta["format"], meta["format_note"]))
        print("dibangun    : %s oleh %s (schema v%d)" % (meta["built_at"], meta["generator"], meta["schema_version"]))
        print("sumber      : %s @ %s (%s)" % (s["repo"], s["commit"], s["commit_date"]))
        print("cek versi   : %s — %s" % (fr.get("result"), fr.get("method") or "-"))
        if fr.get("remote_head"):
            print("              remote HEAD %s (%s), %d file dicek pada %s"
                  % (fr["remote_head"][:12], fr.get("remote_head_date"), fr.get("files_checked", 0), fr["checked_at"]))
        print("skill       : %d (%d file)" % (meta["skill_count"], meta["file_count"]))
        lint = meta.get("lint_summary") or {}
        if lint:
            print("lint        : %d skill dengan catatan (isi disimpan apa adanya dari upstream):" % len(lint))
            for n, issues in lint.items():
                for i in issues:
                    print("              - %s: %s" % (n, i))
    return 0


def cmd_keys(args):
    with open_kv(args) as db:
        for k, v in db.items():
            print("%-70s %8d" % (k.decode("utf-8", "replace"), len(v)))
    return 0


def cmd_install(args):
    dest = os.path.abspath(os.path.expanduser(args.dest))
    created = updated = same = 0
    with open_kv(args) as db:
        manifest = db.get_json(KEY_MANIFEST)
        for name in sorted(manifest):
            m = manifest[name]
            files = [("SKILL.md", db.get("skill:" + name))] + [(f["path"], db.get(f["key"])) for f in m.get("files", [])]
            for rel, data in files:
                p = os.path.join(dest, name, *rel.split("/"))
                if os.path.isfile(p):
                    with open(p, "rb") as fh:
                        if fh.read() == data:
                            same += 1
                            continue
                    updated += 1
                else:
                    created += 1
                if args.dry_run:
                    continue
                os.makedirs(os.path.dirname(p), exist_ok=True)
                with open(p, "wb") as fh:
                    fh.write(data)
    print("%s%d skill -> %s : %d baru, %d diperbarui, %d sudah sama"
          % ("[dry-run] " if args.dry_run else "", len(manifest), dest, created, updated, same))
    return 0


# --------------------------------------------------------------------------- #
# CLI
# --------------------------------------------------------------------------- #
def main(argv=None):
    ap = argparse.ArgumentParser(prog="skills_kv.py", description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--kv", default=DEFAULT_KV, help="path cache.kv (default: %(default)s)")
    ap.add_argument("--skills-dir", default=DEFAULT_SKILLS_DIR, help="direktori install skill (default: %(default)s)")
    ap.add_argument("--repo", default=DEFAULT_REPO, help="repo upstream owner/name (default: %(default)s)")
    ap.add_argument("--branch", default=DEFAULT_BRANCH, help="branch upstream (default: %(default)s)")
    sub = ap.add_subparsers(dest="cmd", required=True)

    p = sub.add_parser("build", help="clone upstream, cek versi terbaru, install ke skills/, tulis cache.kv")
    p.add_argument("--source", help="pakai checkout lokal upstream alih-alih clone baru")
    p.add_argument("--no-remote-check", action="store_true", help="lewati pengecekan versi ke GitHub API")
    p.add_argument("--force", action="store_true", help="timpa skills/ yang tidak dikelola & abaikan peringatan frontmatter")
    p.set_defaults(fn=cmd_build)

    p = sub.add_parser("check-updates", help="bandingkan skill terpasang dengan HEAD upstream (exit 1 bila ada pembaruan)")
    p.add_argument("--json", action="store_true")
    p.add_argument("-v", "--verbose", action="store_true")
    p.set_defaults(fn=cmd_check_updates)

    p = sub.add_parser("verify", help="verifikasi integritas cache.kv (dan kecocokan dengan skills/)")
    p.add_argument("--no-skills-dir", action="store_true", help="jangan bandingkan dengan skills/")
    p.set_defaults(fn=cmd_verify)

    p = sub.add_parser("list", help="daftar skill + deskripsi")
    p.add_argument("--full", action="store_true")
    p.add_argument("--json", action="store_true")
    p.set_defaults(fn=cmd_list)

    p = sub.add_parser("get", help="cetak SKILL.md (atau file/metadata) sebuah skill")
    p.add_argument("name")
    p.add_argument("--meta", action="store_true", help="cetak metadata JSON, bukan isi")
    p.add_argument("--file", help="cetak file tambahan, mis. scripts/sync_skills.py")
    p.set_defaults(fn=cmd_get)

    p = sub.add_parser("search", help="cari skill berdasarkan kata kunci")
    p.add_argument("terms", nargs="+")
    p.add_argument("--all", action="store_true", help="semua kata harus muncul")
    p.add_argument("--limit", type=int, default=20)
    p.set_defaults(fn=cmd_search)

    p = sub.add_parser("info", help="tampilkan metadata build/sumber/cek versi")
    p.add_argument("--json", action="store_true")
    p.set_defaults(fn=cmd_info)

    p = sub.add_parser("keys", help="daftar semua key + ukuran value")
    p.set_defaults(fn=cmd_keys)

    p = sub.add_parser("install", help="ekstrak semua skill dari cache.kv ke direktori tujuan")
    p.add_argument("--dest", required=True, help="mis. ~/.gemini/config/skills")
    p.add_argument("--dry-run", action="store_true")
    p.set_defaults(fn=cmd_install)

    args = ap.parse_args(argv)
    try:
        rc = args.fn(args)
    except subprocess.CalledProcessError as e:
        die("perintah gagal: %s\n%s" % (" ".join(e.cmd), e.stderr))
    except KeyboardInterrupt:
        return 130
    return rc or 0


if __name__ == "__main__":
    sys.exit(main())
