// Tam ekran metin düzenleyici. Dosyayı satır satır bellekte tutar ve her tuşta
// ekranı baştan çizer: 80x25 hücre için bu, işlemciye göre çok küçük bir iş.

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use crate::kabuk::{ASAGI, BAS, SAG, SOL, SON, YUKARI, sonraki_tus};
use crate::vga::{self, GENISLIK, YUKSEKLIK};
use crate::{dosya, println};

// Son ekran satırı durum çubuğu.
const METIN_SATIRI: usize = YUKSEKLIK - 1;
// İmleç satır sonunda da durabilsin diye bir sütun boş kalıyor.
const EN_UZUN: usize = GENISLIK - 1;

const NORMAL: u8 = 0x0b;
const IMLEC: u8 = 0xb0;
const CUBUK: u8 = 0x70;

const ESC: char = '\u{1b}';
const CTRL_S: char = '\u{13}';
const SIL: char = '\u{7f}';

pub fn ac(ad: &str) {
    if let Err(hata) = dosya::ad_kontrol(ad) {
        println!("Kullanım: düzenle <ad> ({})", hata);
        return;
    }
    let metin = match dosya::oku(ad) {
        Ok(veri) => String::from_utf8_lossy(&veri).into_owned(),
        Err("böyle bir dosya yok") => String::new(),
        Err(hata) => {
            println!("Hata: {}", hata);
            return;
        }
    };
    let mut satirlar: Vec<Vec<char>> = metin.split('\n').map(|s| s.chars().collect()).collect();
    if satirlar.iter().any(|s| s.len() > EN_UZUN) {
        println!("Hata: dosyada {} karakterden uzun satır var, açılamıyor.", EN_UZUN);
        return;
    }

    let (mut y, mut x, mut ust): (usize, usize, usize) = (0, 0, 0);
    let mut degisti = false;
    let mut uyarildi = false;
    let mut durum = String::from("Ctrl+S kaydet, Esc çık");
    loop {
        // Ekran imleci takip etsin.
        ust = ust.min(y).max((y + 1).saturating_sub(METIN_SATIRI));
        ciz(&satirlar, ust, y, x, ad, &durum);

        let tus = sonraki_tus();
        if tus != ESC {
            durum.clear();
            uyarildi = false;
        }
        match tus {
            YUKARI => y = y.saturating_sub(1),
            ASAGI => y = (y + 1).min(satirlar.len() - 1),
            SOL if x > 0 => x -= 1,
            SOL if y > 0 => {
                y -= 1;
                x = satirlar[y].len();
            }
            SAG if x < satirlar[y].len() => x += 1,
            SAG if y + 1 < satirlar.len() => {
                y += 1;
                x = 0;
            }
            BAS => x = 0,
            SON => x = satirlar[y].len(),
            '\n' => {
                let kalan = satirlar[y].split_off(x);
                satirlar.insert(y + 1, kalan);
                y += 1;
                x = 0;
                degisti = true;
            }
            '\u{8}' if x > 0 => {
                satirlar[y].remove(x - 1);
                x -= 1;
                degisti = true;
            }
            // Satır başında Backspace: bu satırı yukarıdakine ekle.
            '\u{8}' if y > 0 && satirlar[y - 1].len() + satirlar[y].len() <= EN_UZUN => {
                let satir = satirlar.remove(y);
                y -= 1;
                x = satirlar[y].len();
                satirlar[y].extend(satir);
                degisti = true;
            }
            SIL if x < satirlar[y].len() => {
                satirlar[y].remove(x);
                degisti = true;
            }
            SIL if y + 1 < satirlar.len() && satirlar[y].len() + satirlar[y + 1].len() <= EN_UZUN => {
                let satir = satirlar.remove(y + 1);
                satirlar[y].extend(satir);
                degisti = true;
            }
            CTRL_S => {
                let veri: Vec<String> = satirlar.iter().map(|s| s.iter().collect()).collect();
                let veri = veri.join("\n");
                match dosya::yaz(ad, veri.as_bytes()) {
                    Ok(()) => {
                        durum = format!("kaydedildi ({} bayt)", veri.len());
                        degisti = false;
                    }
                    Err(hata) => durum = format!("Hata: {}", hata),
                }
            }
            // Kaydedilmemiş değişiklik varsa ilk Esc yalnızca uyarır.
            ESC if degisti && !uyarildi => {
                durum = String::from("Kaydedilmedi! Kaydetmeden çıkmak için yine Esc");
                uyarildi = true;
            }
            ESC => break,
            '\t' => {
                for _ in 0..4 {
                    if satirlar[y].len() < EN_UZUN {
                        satirlar[y].insert(x, ' ');
                        x += 1;
                    }
                }
                degisti = true;
            }
            c if !c.is_control() && !(YUKARI..=SON).contains(&c) && satirlar[y].len() < EN_UZUN => {
                satirlar[y].insert(x, c);
                x += 1;
                degisti = true;
            }
            _ => {}
        }
        // Yukarı/aşağı giderken imleç kısa satırın dışında kalmasın.
        x = x.min(satirlar[y].len());
    }
    vga::temizle();
}

fn ciz(satirlar: &[Vec<char>], ust: usize, y: usize, x: usize, ad: &str, durum: &str) {
    for ekran_satiri in 0..METIN_SATIRI {
        let satir = satirlar.get(ust + ekran_satiri);
        for sutun in 0..GENISLIK {
            let c = satir.and_then(|s| s.get(sutun)).copied().unwrap_or(' ');
            let renk = if ust + ekran_satiri == y && sutun == x { IMLEC } else { NORMAL };
            vga::hucre_yaz(ekran_satiri, sutun, c, renk);
        }
    }
    let cubuk = format!(" {}  satır {}/{}, sütun {}   {}", ad, y + 1, satirlar.len(), x + 1, durum);
    let mut harfler = cubuk.chars();
    for sutun in 0..GENISLIK {
        vga::hucre_yaz(METIN_SATIRI, sutun, harfler.next().unwrap_or(' '), CUBUK);
    }
}
