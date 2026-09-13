//! skills_kv.rs — pembaca `cache.kv` (format CDB) untuk Rust, tanpa dependency.
//!
//! `cache.kv` adalah constant database (D. J. Bernstein CDB, 32-bit little
//! endian). Lookup = O(1): 1 pembacaan header + probe tabel hash + 1 record.
//! File ini bisa dipakai dua cara:
//!
//!   1. Sebagai modul: salin ke crate Anda (`mod skills_kv;`) lalu
//!      `let db = skills_kv::SkillCache::open("cache.kv")?;`
//!      `db.get_str("skill:caveman")` / `db.skill("caveman")` / `db.names()`
//!      Jika sudah memakai crate `cdb` (https://docs.rs/cdb), file ini juga
//!      dapat dibuka langsung dengan `cdb::CDB::open("cache.kv")` — formatnya sama.
//!
//!   2. Sebagai CLI mandiri (rustc >= 1.63, tanpa cargo):
//!      rustc -O -o skills-kv tools/skills_kv.rs
//!      ./skills-kv list | get <nama> | desc <nama> | search <kata..> | info | keys | selftest
//!
//! Isi file dibaca sekali ke memori (≈350 KiB) — sederhana dan portabel; untuk
//! file yang jauh lebih besar ganti `data: Vec<u8>` dengan mmap (crate memmap2).

use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::io;
use std::path::Path;

const HEADER_LEN: usize = 2048;
const KEY_INDEX: &[u8] = b"_index";
const KEY_META: &[u8] = b"_meta";

#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    Corrupt(&'static str),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io(e) => write!(f, "I/O error: {e}"),
            Error::Corrupt(why) => write!(f, "cache.kv rusak: {why}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Error::Io(e)
    }
}

/// Hash CDB: h = ((h << 5) + h) ^ byte, mulai dari 5381.
pub fn cdb_hash(key: &[u8]) -> u32 {
    key.iter()
        .fold(5381u32, |h, &b| (h.wrapping_shl(5).wrapping_add(h)) ^ u32::from(b))
}

pub struct SkillCache {
    data: Vec<u8>,
}

impl SkillCache {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, Error> {
        Self::from_bytes(fs::read(path)?)
    }

    pub fn from_bytes(data: Vec<u8>) -> Result<Self, Error> {
        if data.len() < HEADER_LEN {
            return Err(Error::Corrupt("file lebih kecil dari header CDB (2048 byte)"));
        }
        let db = SkillCache { data };
        // header harus menunjuk ke dalam file
        for slot in 0..256 {
            let (pos, len) = db.pair(slot * 8)?;
            let end = (pos as u64) + (len as u64) * 8;
            if end > db.data.len() as u64 {
                return Err(Error::Corrupt("header menunjuk di luar file"));
            }
        }
        Ok(db)
    }

    #[inline]
    fn u32_at(&self, off: usize) -> Result<u32, Error> {
        let b = self
            .data
            .get(off..off + 4)
            .ok_or(Error::Corrupt("offset di luar file"))?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    #[inline]
    fn pair(&self, off: usize) -> Result<(u32, u32), Error> {
        Ok((self.u32_at(off)?, self.u32_at(off + 4)?))
    }

    /// Lookup O(1). `None` bila key tidak ada.
    pub fn get(&self, key: &[u8]) -> Result<Option<&[u8]>, Error> {
        let h = cdb_hash(key);
        let (tpos, tlen) = self.pair(((h & 0xff) as usize) * 8)?;
        if tlen == 0 {
            return Ok(None);
        }
        let tpos = tpos as usize;
        let tlen = tlen as usize;
        let start = ((h >> 8) as usize) % tlen;
        for i in 0..tlen {
            let slot = tpos + ((start + i) % tlen) * 8;
            let (sh, sp) = self.pair(slot)?;
            if sp == 0 {
                return Ok(None);
            }
            if sh != h {
                continue;
            }
            let rp = sp as usize;
            let (klen, vlen) = self.pair(rp)?;
            let (klen, vlen) = (klen as usize, vlen as usize);
            let kstart = rp + 8;
            let vstart = kstart + klen;
            let vend = vstart + vlen;
            if vend > self.data.len() {
                return Err(Error::Corrupt("record menunjuk di luar file"));
            }
            if klen == key.len() && &self.data[kstart..vstart] == key {
                return Ok(Some(&self.data[vstart..vend]));
            }
        }
        Ok(None)
    }

    pub fn get_str(&self, key: &str) -> Result<Option<&str>, Error> {
        match self.get(key.as_bytes())? {
            None => Ok(None),
            Some(v) => std::str::from_utf8(v)
                .map(Some)
                .map_err(|_| Error::Corrupt("value bukan UTF-8")),
        }
    }

    /// Isi SKILL.md sebuah skill.
    pub fn skill(&self, name: &str) -> Result<Option<&str>, Error> {
        self.get_str(&format!("skill:{name}"))
    }

    /// Deskripsi (frontmatter) sebuah skill.
    pub fn description(&self, name: &str) -> Result<Option<&str>, Error> {
        self.get_str(&format!("desc:{name}"))
    }

    /// Nama semua skill (dari key `_index`, JSON array of string).
    pub fn names(&self) -> Result<Vec<String>, Error> {
        let raw = self
            .get_str(std::str::from_utf8(KEY_INDEX).unwrap())?
            .ok_or(Error::Corrupt("key _index tidak ada"))?;
        parse_json_string_array(raw).ok_or(Error::Corrupt("_index bukan JSON array of string"))
    }

    /// Iterasi semua (key, value) berurutan sesuai letak di file.
    pub fn iter(&self) -> Iter<'_> {
        let end = self.u32_at(0).unwrap_or(0) as usize; // tabel #0 = akhir area record
        Iter { db: self, pos: HEADER_LEN, end }
    }
}

pub struct Iter<'a> {
    db: &'a SkillCache,
    pos: usize,
    end: usize,
}

impl<'a> Iterator for Iter<'a> {
    type Item = Result<(&'a [u8], &'a [u8]), Error>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.pos >= self.end {
            return None;
        }
        let rec = (|| {
            let (klen, vlen) = self.db.pair(self.pos)?;
            let ks = self.pos + 8;
            let vs = ks + klen as usize;
            let ve = vs + vlen as usize;
            if ve > self.db.data.len() {
                return Err(Error::Corrupt("record menunjuk di luar file"));
            }
            self.pos = ve;
            Ok((&self.db.data[ks..vs], &self.db.data[vs..ve]))
        })();
        if rec.is_err() {
            self.pos = self.end; // hentikan iterasi setelah error
        }
        Some(rec)
    }
}

/// Parser JSON minimal untuk `["a","b",...]` (cukup untuk `_index`; string di
/// sana hanya nama kebab-case ASCII, tetapi escape dasar tetap ditangani).
fn parse_json_string_array(s: &str) -> Option<Vec<String>> {
    let s = s.trim();
    let inner = s.strip_prefix('[')?.strip_suffix(']')?;
    let mut out = Vec::new();
    let mut chars = inner.chars().peekable();
    loop {
        while matches!(chars.peek(), Some(c) if c.is_whitespace() || *c == ',') {
            chars.next();
        }
        match chars.next() {
            None => return Some(out),
            Some('"') => {}
            Some(_) => return None,
        }
        let mut cur = String::new();
        loop {
            match chars.next()? {
                '"' => break,
                '\\' => match chars.next()? {
                    'n' => cur.push('\n'),
                    't' => cur.push('\t'),
                    'r' => cur.push('\r'),
                    'b' => cur.push('\u{8}'),
                    'f' => cur.push('\u{c}'),
                    'u' => {
                        let hex: String = (0..4).filter_map(|_| chars.next()).collect();
                        let cp = u32::from_str_radix(&hex, 16).ok()?;
                        cur.push(char::from_u32(cp).unwrap_or('\u{fffd}'));
                    }
                    other => cur.push(other),
                },
                c => cur.push(c),
            }
        }
        out.push(cur);
    }
}

// --------------------------------------------------------------------------- //
// CLI
// --------------------------------------------------------------------------- //
fn usage() -> ! {
    eprintln!(
        "pemakaian: skills-kv [--kv cache.kv] <perintah>\n\
         \x20 list                daftar skill + deskripsi\n\
         \x20 get <nama>          cetak SKILL.md\n\
         \x20 desc <nama>         cetak deskripsi\n\
         \x20 search <kata..>     cari di nama/deskripsi/isi\n\
         \x20 info                cetak _meta (JSON)\n\
         \x20 keys                daftar semua key + ukuran\n\
         \x20 selftest            uji vektor hash + konsistensi lookup vs iterasi"
    );
    std::process::exit(2)
}

fn main() {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let mut kv = String::from("cache.kv");
    if args.len() >= 2 && args[0] == "--kv" {
        kv = args.remove(1);
        args.remove(0);
    }
    if args.is_empty() {
        usage();
    }
    if let Err(e) = run(&kv, &args) {
        eprintln!("ERROR: {e}");
        std::process::exit(1);
    }
}

fn run(kv: &str, args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let db = SkillCache::open(kv).map_err(|e| format!("{kv}: {e}"))?;
    match args[0].as_str() {
        "list" => {
            let names = db.names()?;
            let width = names.iter().map(String::len).max().unwrap_or(0);
            for n in &names {
                let d = db.description(n)?.unwrap_or("");
                let d: String = if d.chars().count() > 100 {
                    d.chars().take(97).collect::<String>() + "..."
                } else {
                    d.to_string()
                };
                println!("{n:<width$}  {d}");
            }
            println!("({} skill)", names.len());
        }
        "get" | "desc" => {
            let name = args.get(1).unwrap_or_else(|| usage());
            let v = if args[0] == "get" { db.skill(name)? } else { db.description(name)? };
            match v {
                Some(s) => {
                    print!("{s}");
                    if !s.ends_with('\n') {
                        println!();
                    }
                }
                None => {
                    eprintln!("ERROR: skill '{name}' tidak ada di {kv}");
                    std::process::exit(1);
                }
            }
        }
        "search" => {
            let terms: Vec<String> = args[1..].iter().map(|t| t.to_lowercase()).collect();
            if terms.is_empty() {
                usage();
            }
            let mut hits: Vec<(u32, String, String)> = Vec::new();
            for n in db.names()? {
                let desc = db.description(&n)?.unwrap_or("").to_string();
                let body = db.skill(&n)?.unwrap_or("").to_lowercase();
                let (ln, ld) = (n.to_lowercase(), desc.to_lowercase());
                let mut score = 0u32;
                for t in &terms {
                    if ln.contains(t.as_str()) {
                        score += 30;
                    }
                    if ld.contains(t.as_str()) {
                        score += 20;
                    }
                    score += (body.matches(t.as_str()).count().min(5) as u32) * 2;
                }
                if score > 0 {
                    hits.push((score, n, desc));
                }
            }
            hits.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
            for (score, n, d) in hits.iter().take(20) {
                let d: String = d.chars().take(90).collect();
                println!("{n:<40} {:>5.1}  {d}", *score as f32 / 10.0);
            }
            if hits.is_empty() {
                println!("tidak ada skill yang cocok");
                std::process::exit(1);
            }
        }
        "info" => {
            let meta = db.get_str(std::str::from_utf8(KEY_META).unwrap())?.unwrap_or("{}");
            println!("{meta}");
        }
        "keys" => {
            for item in db.iter() {
                let (k, v) = item?;
                println!("{:<70} {:>8}", String::from_utf8_lossy(k), v.len());
            }
        }
        "selftest" => selftest(&db)?,
        _ => usage(),
    }
    Ok(())
}

fn selftest(db: &SkillCache) -> Result<(), Box<dyn std::error::Error>> {
    // Vektor uji hash CDB (nilai referensi dari implementasi Python & tinycdb).
    let vectors: [(&[u8], u32); 4] = [
        (b"", 5381),
        (b"a", 0x0002_B5C4),
        (b"caveman", 0xD0A8_6676),
        (b"_index", 0xCFFF_49A4),
    ];
    for (k, want) in vectors {
        let got = cdb_hash(k);
        if got != want {
            return Err(format!("hash({:?}) = {got:#x}, seharusnya {want:#x}", String::from_utf8_lossy(k)).into());
        }
    }
    // Setiap record yang ditemukan lewat iterasi harus bisa ditemukan lewat hash lookup.
    let mut n = 0usize;
    let mut by_prefix: BTreeMap<String, usize> = BTreeMap::new();
    for item in db.iter() {
        let (k, v) = item?;
        match db.get(k)? {
            Some(got) if got == v => {}
            _ => return Err(format!("lookup gagal untuk key {:?}", String::from_utf8_lossy(k)).into()),
        }
        let prefix = String::from_utf8_lossy(k).split(':').next().unwrap_or("").to_string();
        *by_prefix.entry(prefix).or_insert(0) += 1;
        n += 1;
    }
    let names = db.names()?;
    for name in &names {
        if db.skill(name)?.is_none() || db.description(name)?.is_none() {
            return Err(format!("skill/desc hilang untuk {name}").into());
        }
    }
    if db.get(b"skill:tidak-ada-skill-ini")?.is_some() {
        return Err("key palsu ditemukan".into());
    }
    println!("selftest OK: {n} record, {} skill, per-prefix {:?}", names.len(), by_prefix);
    Ok(())
}
