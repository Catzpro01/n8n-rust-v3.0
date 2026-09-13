---
name: humanise-ai
description: >-
  Skill untuk mengubah teks AI yang terdengar kaku/robotik menjadi tulisan yang
  alami, mengalir, dan terdengar seperti manusia sungguhan. Mendeteksi dan mengganti
  pola bahasa khas AI dengan ekspresi yang lebih otentik dan personal.
---

# Humanise AI — Writing Naturalizer

## Masalah yang Diselesaikan
AI menghasilkan teks dengan pola yang mudah dikenali:
- Kalimat dengan panjang seragam
- Transisi formal berlebihan
- Frasa klise yang berulang
- Nada yang terlalu sopan dan hati-hati
- Struktur paragraf yang terlalu rapi

## Deteksi Pola AI ("AI Tells")

### Frasa yang Harus Diganti
| Frasa AI | Pengganti Alami |
|----------|----------------|
| "It is worth noting that..." | (langsung saja ke poinnya) |
| "In today's rapidly evolving landscape..." | (mulai dengan konteks spesifik) |
| "This serves as a testament to..." | (tunjukkan faktanya, bukan komentar) |
| "I would be happy to assist..." | (langsung bantu) |
| "Certainly! Absolutely!" | (langsung jawab) |
| "In conclusion..." | (wrap up tanpa label) |
| "Firstly... Secondly... Thirdly..." | (variasikan transisi) |
| "It is important to note..." | (langsung sebutkan apa yang penting) |
| "...of the utmost importance" | "...sangat penting" |
| "Leverage" (sebagai kata kerja) | "gunakan" / "manfaatkan" |
| "Utilize" | "gunakan" |
| "Facilitate" | "bantu" |
| "Comprehensive" (tanpa spesifik) | jelaskan apa yang comprehensive-nya |

## Teknik Humanisasi

### 1. Variasi Panjang Kalimat
❌ AI:
> "This function processes user input. It validates the data against the schema. It then stores the result in the database. Finally, it returns a success response."

✅ Human:
> "This function processes user input — validates it, stores it, done. Simple enough, but the edge cases are where things get interesting."

### 2. Masukkan Ketidakpastian Alami
AI terdengar terlalu yakin. Manusia ragu-ragu secara alami:
- "Saya rasa..." / "Menurut saya..."
- "Mungkin yang terbaik adalah..."
- "Ini bisa jadi pendekatan yang masuk akal, tapi..."
- "Jujur saja, saya belum pernah coba ini di production"

### 3. Kontraksi dan Bahasa Percakapan
❌ "It is recommended that you configure..."
✅ "You'll want to configure..." / "Best to configure..."

❌ "The user should not..."
✅ "Don't..." / "Avoid..."

### 4. Spesifikasi > Generalisasi
❌ "This is a powerful tool"
✅ "This cuts our build time from 8 minutes to 40 seconds"

❌ "Significantly improves performance"
✅ "Reduces API latency by ~60ms on average"

### 5. Cadence dan Ritme
Baca teks dengan keras. Jika terasa seperti membaca manual:
- Pecah kalimat panjang menjadi 2-3 kalimat pendek
- Tambahkan satu kalimat sangat pendek setelah paragraf panjang
- Mulai beberapa kalimat dengan kata pendek: "But.", "And.", "So."

## Cara Menggunakan Skill Ini

**Input**: Paste teks AI yang ingin di-humanize
**Output**: Versi yang lebih alami dengan penjelasan perubahan utama

### Process
1. Scan: Identifikasi semua "AI tells" dalam teks
2. Tone check: Tentukan tone yang sesuai (casual/professional/technical)
3. Rewrite: Aplikasikan teknik humanisasi
4. Read aloud test: Apakah terasa natural diucapkan?
5. Output: Teks yang sudah di-humanize + catatan perubahan besar

## Skala Humanisasi

| Level | Deskripsi | Gunakan Untuk |
|-------|-----------|---------------|
| **1 - Minimal** | Hanya ganti frasa klise | Dokumen teknis/formal |
| **2 - Moderate** | + Variasi panjang kalimat | Blog, dokumentasi |
| **3 - Full** | + Personal nuance, contractions | Email, copy marketing |
| **4 - Conversational** | Bahasa sangat kasual, opini personal | Chat, social media |

## Peringatan Keamanan
Skill ini berfokus pada gaya bahasa — TIDAK mengubah:
- Fakta atau data
- Klaim teknis
- Referensi atau kutipan
- Angka dan statistik
