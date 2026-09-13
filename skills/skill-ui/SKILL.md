---
name: skill-ui
description: >-
  Task-oriented agent skill untuk menghasilkan komponen UI yang polished,
  accessible, dan framework-compliant. Terinspirasi dari ui.sh (Tailwind CSS team).
  Aktifkan dengan /design, /component, atau /ideas untuk generasi UI terstruktur.
---

# Skill UI — Structured UI Component Generation

## Cara Pakai
Aktifkan dengan slash commands:
- `/design` — rancang layout atau halaman baru
- `/component` — buat komponen UI spesifik
- `/ideas` — brainstorm variasi UI dari requirement
- `/audit-ui` — audit komponen yang ada terhadap standar

## Fase Generasi Komponen (/component)

### Phase 1: Intent
Sebelum menulis kode apapun, klarifikasi:
- Framework apa? (React/Next.js/Vue/Svelte)
- Component library? (shadcn/ui, Radix, MUI, Tailwind-only)
- State management? (local, Zustand, Redux)
- Apakah perlu animasi?

### Phase 2: Structure Mapping
Petakan requirement ke komponen konkret:
```
Requirement: "tombol kirim form dengan loading state"
→ Button (primary variant)
  ├── State: idle | loading | success | error
  ├── Icon: Spinner saat loading
  └── Disabled: true saat loading/success
```

### Phase 3: Generation Rules
- Gunakan HANYA komponen yang ada di library yang dipilih
- Jangan invent tag atau prop yang tidak ada
- Selalu sertakan TypeScript interface untuk props
- Selalu handle loading, error, dan empty state
- Minimal satu aria attribute pada setiap interactive element

### Phase 4: Validation
Sebelum output, verifikasi:
- [ ] Props typed dengan interface TypeScript
- [ ] Tidak ada inline style (kecuali dynamic values)
- [ ] Contrast ratio WCAG AA minimum (4.5:1)
- [ ] Keyboard navigation berfungsi
- [ ] Mobile-responsive (min 375px)

## /design — Layout Generation

Untuk layout baru:
1. Tentukan grid system (12-column recommended)
2. Identifikasi komponen utama (navbar, sidebar, main, footer)
3. Gambar wireframe text terlebih dahulu
4. Baru generate kode

```
Layout wireframe:
+--[Navbar: Logo | Nav links | CTA]--+
|                                    |
| [Sidebar: 280px] | [Main: flex-1]  |
|   - Filter A     |   Content area  |
|   - Filter B     |                 |
|                  |                 |
+----[Footer: Copyright | Links]-----+
```

## /ideas — Variation Brainstorm

Berikan 3 variasi berbeda untuk setiap request:
- **Variant A**: Minimal (Tailwind only, no library)
- **Variant B**: Standard (dengan shadcn/ui atau Radix)
- **Variant C**: Rich (animasi Framer Motion / full interaction)

Sertakan trade-off masing-masing variasi.

## Framework-Specific Rules

### React/Next.js (App Router)
- Server Components by default, `"use client"` hanya jika diperlukan
- Image: gunakan next/image, bukan `<img>`
- Link: gunakan next/link, bukan `<a>`
- Font: gunakan next/font

### Tailwind CSS
- Mobile-first: `text-sm md:text-base lg:text-lg`
- Gunakan design token (`text-primary`, `bg-background`)
- Jangan arbitrary values kecuali benar-benar perlu (`w-[347px]` → hindari)

### shadcn/ui
- Baca components.json sebelum menambah komponen baru
- Gunakan `cn()` utility untuk conditional classes
- Ikuti naming convention: `PascalCase` untuk components
