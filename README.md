# n8n-rust-v3.0

## Skills cache (`cache.kv`)

Seluruh skill dari [Catzpro01/antigravity-skills](https://github.com/Catzpro01/antigravity-skills)
(151 skill) di-install ke [`skills/`](skills/) dan dikemas menjadi satu file
[`cache.kv`](cache.kv) — constant database (CDB) dengan lookup O(1) yang bisa
dibaca dari Rust (`cdb::CDB::open("cache.kv")` atau [`tools/skills_kv.rs`](tools/skills_kv.rs)),
Python ([`tools/skills_kv.py`](tools/skills_kv.py)), maupun reader CDB lain.

Setiap file diverifikasi identik dengan HEAD upstream lewat GitHub API saat
build (versi terbaru), dan `cache.kv` diverifikasi identik dengan `skills/`.

```bash
python tools/skills_kv.py list             # daftar skill
python tools/skills_kv.py get caveman      # isi SKILL.md
python tools/skills_kv.py check-updates    # masih versi terbaru?
python tools/skills_kv.py build            # perbarui dari upstream
```

Dokumentasi lengkap: [`tools/README.md`](tools/README.md).
