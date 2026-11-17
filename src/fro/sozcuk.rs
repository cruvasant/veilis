// 1. adım: metni sözcüklere (token) böler.
//   "a = 5 + 2"  ->  [Ad("a"), Esittir, Sayi(5), Arti, Sayi(2)]

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Sozcuk<'a> {
    Sayi(i64),
    Ad(&'a str),
    Yaz,
    Esittir,
    Arti,
    Eksi,
    Carpi,
    Bolu,
    Kalan,
    AcParantez,
    KapaParantez,
}

pub fn bol(metin: &str) -> Result<Vec<Sozcuk<'_>>, String> {
    let mut sozcukler = Vec::new();
    let mut harfler = metin.char_indices().peekable();

    while let Some(&(bas, c)) = harfler.peek() {
        if c.is_whitespace() {
            harfler.next();
        } else if c.is_ascii_digit() {
            let mut sayi: i64 = 0;
            while let Some(rakam) = harfler.peek().and_then(|&(_, r)| r.to_digit(10)) {
                sayi = sayi
                    .checked_mul(10)
                    .and_then(|s| s.checked_add(rakam as i64))
                    .ok_or("sayı çok büyük")?;
                harfler.next();
            }
            sozcukler.push(Sozcuk::Sayi(sayi));
        } else if c.is_alphabetic() || c == '_' {
            // Değişken adları Türkçe harf içerebilir: çarpım, sayaç, ...
            let mut son = bas;
            while let Some(&(i, r)) = harfler.peek() {
                if !(r.is_alphanumeric() || r == '_') {
                    break;
                }
                son = i + r.len_utf8();
                harfler.next();
            }
            let ad = &metin[bas..son];
            sozcukler.push(if ad == "yaz" { Sozcuk::Yaz } else { Sozcuk::Ad(ad) });
        } else {
            harfler.next();
            sozcukler.push(match c {
                '=' => Sozcuk::Esittir,
                '+' => Sozcuk::Arti,
                '-' => Sozcuk::Eksi,
                '*' => Sozcuk::Carpi,
                '/' => Sozcuk::Bolu,
                '%' => Sozcuk::Kalan,
                '(' => Sozcuk::AcParantez,
                ')' => Sozcuk::KapaParantez,
                _ => return Err(format!("anlaşılmayan işaret: {}", c)),
            });
        }
    }
    Ok(sozcukler)
}
