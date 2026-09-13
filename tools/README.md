# tools/ — antigravity-skills → `cache.kv`

Alat untuk meng-install seluruh skill dari
[Catzpro01/antigravity-skills](https://github.com/Catzpro01/antigravity-skills),
memastikan semuanya **versi terbaru**, lalu mengemasnya menjadi satu file
`cache.kv` untuk akses cepat.

| File | Fungsi |
| --- | --- |
| `skills_kv.py` | Builder + verifier + pembaca CLI (Python 3.8+, stdlib saja) |
| `skills_kv.rs` | Pembaca untuk Rust, tanpa dependency (modul atau CLI mandiri) |

## Kenapa format CDB?

`cache.kv` memakai format **CDB** (constant database, D. J. Bernstein),
32-bit little-endian:

- **Lookup O(1)** — 256 tabel hash di header; sebuah `get` paling banyak
  membaca header → satu slot tabel → satu record. Tidak ada parsing JSON/YAML
  saat startup; file bisa di-`mmap` dan dipakai langsung.
- **Kecil & tunggal** — 151 skill (152 file, 617 KiB di disk) menjadi satu
  file ±347 KiB; ideal untuk dibundel ke binary/container.
- **Standar terbuka** — bisa dibaca crate Rust [`cdb`](https://docs.rs/cdb)
  (`cdb::CDB::open("cache.kv")`), Python `pure-cdb`, `tinycdb` (C), dsb.
  Build di sini sudah diverifikasi silang dengan `pure-cdb`.
- **Immutable & deterministik** — sesuai sifat cache: dibangun sekali, dibaca
  banyak; byte output identik untuk input yang sama (`SOURCE_DATE_EPOCH`).

## Layout key

| Key | Isi |
| --- | --- |
| `_meta` | JSON: info build, commit sumber, hasil cek versi terbaru, ringkasan lint |
| `_index` | JSON: daftar nama skill terurut |
| `_manifest` | JSON: `{nama → metadata}` |
| `skill:<nama>` | `SKILL.md` apa adanya (byte identik dengan upstream) |
| `desc:<nama>` | deskripsi dari frontmatter (satu baris) |
| `meta:<nama>` | JSON: `sha256`, `blob_sha`, `size`, `last_commit`, `files`, `lint` |
| `file:<nama>/<path>` | file tambahan milik skill (mis. `file:skill-github-sync/scripts/sync_skills.py`) |

## Pengecekan "versi terbaru"

Repo upstream tidak memakai tag/nomor versi — satu-satunya sumber kebenaran
adalah commit di branch `main`. Karena itu `build`:

1. meng-clone upstream (full history, agar `last_commit` per skill diketahui);
2. menghitung **git blob SHA-1** setiap file yang akan dipasang;
3. mengambil HEAD `main` + daftar blob **langsung dari GitHub API**
   (`/commits/main` dan `/git/trees/{sha}?recursive=1`) dan membandingkannya
   file per file. Bila ada satu saja file yang berbeda/baru/dihapus, build
   **gagal** — jadi `cache.kv` yang jadi dijamin identik dengan HEAD upstream
   pada saat build.

Hasilnya disimpan di `_meta.freshness` dan `skills/UPSTREAM.json`. Untuk
mengecek ulang kapan saja (tanpa clone) gunakan `check-updates`.

## Pemakaian

```bash
# build ulang dari upstream (clone → cek versi → install skills/ → tulis cache.kv → verify)
python tools/skills_kv.py build

# apakah sudah versi terbaru? (exit 0 = up-to-date, 1 = ada pembaruan)
python tools/skills_kv.py check-updates

# integritas cache.kv + kecocokan dengan skills/
python tools/skills_kv.py verify

# membaca
python tools/skills_kv.py list
python tools/skills_kv.py get caveman
python tools/skills_kv.py get caveman --meta
python tools/skills_kv.py get skill-github-sync --file scripts/sync_skills.py
python tools/skills_kv.py search debug docker
python tools/skills_kv.py info

# pasang ke registry Antigravity (idempoten; --dry-run untuk melihat dulu)
python tools/skills_kv.py install --dest ~/.gemini/config/skills
```

Opsi global: `--kv PATH` (default `cache.kv`), `--skills-dir DIR` (default
`skills`), `--repo owner/name`, `--branch`. Untuk build offline dari checkout
lokal: `build --source /path/ke/antigravity-skills --no-remote-check`
(pengecekan versi dilewati dan dicatat sebagai `skipped` di `_meta`).

### Dari Rust

```rust
mod skills_kv; // salin tools/skills_kv.rs ke crate Anda

let db = skills_kv::SkillCache::open("cache.kv")?;
if let Some(body) = db.skill("caveman")? {
    println!("{body}");
}
for name in db.names()? {
    println!("{name}: {}", db.description(&name)?.unwrap_or(""));
}
```

Atau tanpa cargo: `rustc -O -o skills-kv tools/skills_kv.rs && ./skills-kv list`.
`./skills-kv selftest` memverifikasi vektor hash dan konsistensi lookup.

### Dari Python (embed)

```python
import sys; sys.path.insert(0, "tools")
from skills_kv import CdbReader

with CdbReader("cache.kv") as db:
    print(db.get_text("desc:caveman"))
    names = db.get_json("_index")
```

## Catatan kualitas upstream

Enam `SKILL.md` di upstream mengandung karakter kontrol (`\x07`, `\x08`,
`\x0c`) — jelas hasil escape `\a`, `\b`, `\f` yang tidak sengaja saat file
dibuat (mis. `\backtick` → backspace, `\feat(...)` → form feed). Skill yang
terdampak: `axe-core-a11y`, `design-md-tokens`, `ralph-wiggum-loop`,
`shell-wizard-explain`, `skill-github-sync`, `web-perf-doctor`. Isi **tidak
diubah** (agar tetap identik dengan upstream dan verifikasi SHA berlaku);
temuan dicatat di `meta:<nama>.lint` dan `_meta.lint_summary`, serta muncul di
`skills_kv.py info`. Perbaikan sebaiknya dilakukan di repo upstream.
