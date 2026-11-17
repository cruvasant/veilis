// 1. adım: metni sözcüklere (token) böler.
//   "a = 5 + 2"  ->  [Ad("a"), Esittir, Sayi(5), Arti, Sayi(2)]

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Sozcuk<'a> {
    Sayi(i64),
    Metin(&'a str),
    Ad(&'a str),
    Yaz,
    Eger,
    Degilse,
    Dongu,
    Esittir,
    EsitEsit,
    EsitDegil,
    Kucuk,
    Buyuk,
    KucukEsit,
    BuyukEsit,
    Arti,
    Eksi,
    Carpi,
    Bolu,
    Kalan,
    AcParantez,
    KapaParantez,
    AcSuslu,
    KapaSuslu,
    // ';' ya da yeni satır: bir deyimi bitirir.
    Ayrac,
}

pub fn bol(metin: &str) -> Result<Vec<Sozcuk<'_>>, String> {
    let mut sozcukler = Vec::new();
    let mut harfler = metin.char_indices().peekable();

    while let Some(&(bas, c)) = harfler.peek() {
        if c.is_whitespace() && c != '\n' {
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
            sozcukler.push(match &metin[bas..son] {
                "yaz" => Sozcuk::Yaz,
                "eğer" | "eger" => Sozcuk::Eger,
                "değilse" | "degilse" => Sozcuk::Degilse,
                "döngü" | "dongu" => Sozcuk::Dongu,
                ad => Sozcuk::Ad(ad),
            });
        } else if c == '"' {
            harfler.next();
            let son = loop {
                match harfler.next() {
                    Some((i, '"')) => break i,
                    Some(_) => {}
                    None => return Err("metin kapanmamış: '\"' eksik".into()),
                }
            };
            sozcukler.push(Sozcuk::Metin(&metin[bas + 1..son]));
        } else {
            harfler.next();
            let esitle = harfler.next_if(|&(_, r)| r == '=').is_some();
            sozcukler.push(match (c, esitle) {
                ('=', true) => Sozcuk::EsitEsit,
                ('=', false) => Sozcuk::Esittir,
                ('!', true) => Sozcuk::EsitDegil,
                ('<', true) => Sozcuk::KucukEsit,
                ('<', false) => Sozcuk::Kucuk,
                ('>', true) => Sozcuk::BuyukEsit,
                ('>', false) => Sozcuk::Buyuk,
                ('+', false) => Sozcuk::Arti,
                ('-', false) => Sozcuk::Eksi,
                ('*', false) => Sozcuk::Carpi,
                ('/', false) => Sozcuk::Bolu,
                ('%', false) => Sozcuk::Kalan,
                ('(', false) => Sozcuk::AcParantez,
                (')', false) => Sozcuk::KapaParantez,
                ('{', false) => Sozcuk::AcSuslu,
                ('}', false) => Sozcuk::KapaSuslu,
                (';' | '\n', false) => Sozcuk::Ayrac,
                _ => return Err(format!("anlaşılmayan işaret: {}", c)),
            });
        }
    }
    Ok(sozcukler)
}
