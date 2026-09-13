---
name: skill-evaluator
description: >-
  Skill evaluator and utility benchmark (based on agent-skills-eval). Evaluates whether
  a given skill is genuinely useful, effective, redundant, or causing negative side effects
  via paired testing (With-Skill vs Without-Skill) and static quality checks.
---

# Skill Evaluator — Efficacy & Utility Benchmark

## Tujuan Skill Ini
Menilai secara objektif apakah sebuah skill yang terpasang di Antigravity / Agent OS:
1. **Benar-benar berguna (High Lift):** Meningkatkan akurasi, kecepatan, atau kualitas output secara terukur.
2. **Redundan (Zero Lift / Slop):** Hanya mengulang apa yang sudah bisa dilakukan model dasar tanpa skill.
3. **Membahayakan / Membebani (Negative Lift):** Membuang-buang token konteks, memicu halusinasi, atau melanggar guardrail keamanan.

---

## Metodologi Pengujian: Paired Evaluation (A/B Test)

Untuk menguji kegunaan suatu skill, jalankan **3 Uji Banding**:

### 1. Uji Baseline (Tanpa Skill - Kontrol A)
* Jalankan prompt uji yang relevan dengan domain skill **tanpa menyuntikkan skill**.
* Catat: Kualitas kode, jumlah error, kelengkapan edge case, dan jumlah token yang dipakai.

### 2. Uji Treatment (Dengan Skill - Uji B)
* Jalankan prompt uji yang persis sama **dengan menyuntikkan file `SKILL.md`**.
* Catat: Apakah output mengikuti instruksi khusus? Apakah ada peningkatan konkret?

### 3. Penjurian (Judge Evaluation)
Bandingkan A dan B berdasarkan matriks:
* **Delta Kualitas (Lift %):** Apakah B lebih minim bug, lebih terstruktur, atau lebih sesuai spesifikasi dibanding A?
* **Token Overhead:** Berapa banyak token konteks yang dihabiskan oleh file `SKILL.md` dibandingkan perbaikan yang didapat?
* **Adherence:** Apakah agent benar-benar mematuhi aturan unik di dalam skill tersebut?

---

## 5 Kategori Skor Kegunaan (Utility Verdict)

Setiap skill yang dievaluasi akan diberi label:

| Status | Skor | Kriteria | Rekomendasi |
|---|---|---|---|
| 🟢 **ESSENTIAL** | 85–100 | Memberikan workflow baru, format baku, atau proteksi krusial yang gagal dilakukan model polos. | **Pertahankan & jadikan prioritas** |
| 🟡 **USEFUL** | 65–84 | Memperbaiki struktur dan gaya, namun model polos sesekali bisa melakukannya jika diprompt manual. | **Pertahankan sebagai on-demand** |
| ⚪ **REDUNDANT** | 40–64 | Model polos sudah menghasilkan output yang sama bagusnya tanpa skill ini (buang-buang token). | **Hapus / Satukan ke skill lain** |
| 🔴 **DETRIMENTAL** | < 40 | Membatasi fleksibilitas model secara keliru, menghasilkan kode kuno, atau memperlambat eksekusi. | **Hapus segera** |
| ☠️ **UNSAFE** | N/A | Meminta izin destruktif tanpa guardrail atau mengekspos credential. | **Blokir** |

---

## Checklist Audit Statis Cepat (Tanpa Eksekusi)

Sebelum melakukan A/B test, lakukan audit teks pada `SKILL.md`:
1. **Specificity Check:** Apakah isinya aturan teknis konkret (contoh: regex, command, skema) atau hanya motivasi klise umum? (Jika hanya kata-kata mutiara / umum = **Redundan**).
2. **Overlap Check:** Apakah fungsinya sudah tercakup 100% oleh skill lain yang sudah terpasang? (Misal: `taste-skill` vs `hallmark` vs `ui-ux-pro-max`).
3. **Executable Action:** Apakah ada template, checklist, atau perintah slash (`/command`) yang bisa langsung dipakai?

---

## Perintah Evaluasi Cepat
Ketik `/eval-skill [nama-skill]` untuk meminta agent melakukan simulasi audit dan uji kelayakan efektivitas skill tersebut.
