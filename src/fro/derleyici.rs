// 2. adım: sözcükleri sanal makinenin komutlarına (bytecode) çevirir.
// Kurallar (dilbilgisi):
//   satır  = "yaz" ifade | ad "=" ifade | ifade
//   ifade  = terim  (("+" | "-") terim)*
//   terim  = çarpan (("*" | "/" | "%") çarpan)*
//   çarpan = sayı | ad | "-" çarpan | "(" ifade ")"
// Çarpma toplamadan önce yapılsın diye her kural bir alttakini çağırır.
// Makine yığın (stack) ile çalışır: "2 + 3" -> Sayi(2), Sayi(3), Topla.

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use super::makine::Komut;
use super::sozcuk::Sozcuk;

struct Derleyici<'a, 'b> {
    sozcukler: &'b [Sozcuk<'a>],
    sira: usize,
    kod: Vec<Komut>,
    adlar: &'b mut Vec<String>,
}

pub fn derle(sozcukler: &[Sozcuk], adlar: &mut Vec<String>) -> Result<Vec<Komut>, String> {
    let mut d = Derleyici { sozcukler, sira: 0, kod: Vec::new(), adlar };
    match sozcukler {
        [] => {}
        [Sozcuk::Ad(ad), Sozcuk::Esittir, ..] => {
            d.sira = 2;
            d.ifade()?;
            d.satir_bitti_mi()?;
            // Değişkene ifadeden sonra yer açıyoruz: "a = a + 1" a tanımsızken hata versin.
            let yer = match d.adlar.iter().position(|a| a == ad) {
                Some(yer) => yer,
                None => {
                    d.adlar.push(String::from(*ad));
                    d.adlar.len() - 1
                }
            };
            d.kod.push(Komut::Ata(yer));
        }
        _ => {
            // "yaz" yazılmasa da komut satırında sonuç gösterilsin.
            d.sira = if sozcukler[0] == Sozcuk::Yaz { 1 } else { 0 };
            d.ifade()?;
            d.satir_bitti_mi()?;
            d.kod.push(Komut::Yaz);
        }
    }
    Ok(d.kod)
}

impl<'a> Derleyici<'a, '_> {
    fn sonraki(&self) -> Option<Sozcuk<'a>> {
        self.sozcukler.get(self.sira).copied()
    }

    fn satir_bitti_mi(&self) -> Result<(), String> {
        match self.sonraki() {
            None => Ok(()),
            Some(_) => Err("satırın sonunda fazladan bir şey var".into()),
        }
    }

    fn ifade(&mut self) -> Result<(), String> {
        self.terim()?;
        loop {
            let komut = match self.sonraki() {
                Some(Sozcuk::Arti) => Komut::Topla,
                Some(Sozcuk::Eksi) => Komut::Cikar,
                _ => return Ok(()),
            };
            self.sira += 1;
            self.terim()?;
            self.kod.push(komut);
        }
    }

    fn terim(&mut self) -> Result<(), String> {
        self.carpan()?;
        loop {
            let komut = match self.sonraki() {
                Some(Sozcuk::Carpi) => Komut::Carp,
                Some(Sozcuk::Bolu) => Komut::Bol,
                Some(Sozcuk::Kalan) => Komut::Kalan,
                _ => return Ok(()),
            };
            self.sira += 1;
            self.carpan()?;
            self.kod.push(komut);
        }
    }

    fn carpan(&mut self) -> Result<(), String> {
        let sozcuk = self.sonraki().ok_or("ifade yarım kaldı")?;
        self.sira += 1;
        match sozcuk {
            Sozcuk::Sayi(n) => self.kod.push(Komut::Sayi(n)),
            Sozcuk::Ad(ad) => {
                // İsim burada numaraya çevrilir; çalışırken isim aranmaz.
                let yer = self
                    .adlar
                    .iter()
                    .position(|a| a == ad)
                    .ok_or_else(|| format!("tanımsız değişken: {}", ad))?;
                self.kod.push(Komut::Oku(yer));
            }
            Sozcuk::Eksi => {
                self.carpan()?;
                self.kod.push(Komut::Negatif);
            }
            Sozcuk::AcParantez => {
                self.ifade()?;
                if self.sonraki() != Some(Sozcuk::KapaParantez) {
                    return Err("')' eksik".into());
                }
                self.sira += 1;
            }
            _ => return Err("burada bir sayı ya da değişken bekleniyordu".into()),
        }
        Ok(())
    }
}
