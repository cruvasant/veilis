// 2. adım: sözcükleri sanal makinenin komutlarına (bytecode) çevirir.
// Kurallar (dilbilgisi):
//   program = deyim ((";" | yeni satır) deyim)*
//   deyim   = ad "=" ifade | "yaz" ifade | eğer | "döngü" ifade blok | ifade
//   eğer    = "eğer" ifade blok ["değilse" (eğer | blok)]
//   blok    = "{" program "}"
//   ifade   = toplam [("==" | "!=" | "<" | ">" | "<=" | ">=") toplam]
//   toplam  = terim  (("+" | "-") terim)*
//   terim   = çarpan (("*" | "/" | "%") çarpan)*
//   çarpan  = sayı | metin | ad | "-" çarpan | "(" ifade ")"
// Çarpma toplamadan önce yapılsın diye her kural bir alttakini çağırır.
// Makine yığın (stack) ile çalışır: "2 + 3" -> Sayi(2), Sayi(3), Topla.
// eğer ve döngü, atlama komutlarına dönüşür:
//   döngü k { g }  ->  baş: k, YanlissaAtla(son), g, Atla(baş), son:

use alloc::format;
use alloc::rc::Rc;
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
    d.program()?;
    if d.sonraki().is_some() {
        return Err("fazladan '}' var".into());
    }
    Ok(d.kod)
}

impl<'a> Derleyici<'a, '_> {
    fn sonraki(&self) -> Option<Sozcuk<'a>> {
        self.sozcukler.get(self.sira).copied()
    }

    fn bekle(&mut self, sozcuk: Sozcuk, mesaj: &str) -> Result<(), String> {
        if self.sonraki() != Some(sozcuk) {
            return Err(mesaj.into());
        }
        self.sira += 1;
        Ok(())
    }

    fn program(&mut self) -> Result<(), String> {
        loop {
            match self.sonraki() {
                None | Some(Sozcuk::KapaSuslu) => return Ok(()),
                Some(Sozcuk::Ayrac) => self.sira += 1,
                Some(_) => {
                    self.deyim()?;
                    if !matches!(self.sonraki(), None | Some(Sozcuk::KapaSuslu | Sozcuk::Ayrac)) {
                        return Err("burada ';' ya da yeni satır bekleniyordu".into());
                    }
                }
            }
        }
    }

    fn deyim(&mut self) -> Result<(), String> {
        let ikinci = self.sozcukler.get(self.sira + 1).copied();
        match (self.sonraki(), ikinci) {
            (Some(Sozcuk::Ad(ad)), Some(Sozcuk::Esittir)) => {
                self.sira += 2;
                self.ifade()?;
                // Değişkene ifadeden sonra yer açıyoruz: "a = a + 1" a tanımsızken hata versin.
                let yer = match self.adlar.iter().position(|a| a == ad) {
                    Some(yer) => yer,
                    None => {
                        self.adlar.push(String::from(ad));
                        self.adlar.len() - 1
                    }
                };
                self.kod.push(Komut::Ata(yer));
            }
            (Some(Sozcuk::Yaz), _) => {
                self.sira += 1;
                self.ifade()?;
                self.kod.push(Komut::Yaz);
            }
            (Some(Sozcuk::Eger), _) => {
                self.sira += 1;
                self.eger()?;
            }
            (Some(Sozcuk::Dongu), _) => {
                self.sira += 1;
                let bas = self.kod.len();
                self.ifade()?;
                let cikis = self.atlama_yaz(Komut::YanlissaAtla(0));
                self.blok()?;
                self.kod.push(Komut::Atla(bas));
                self.atlama_duzelt(cikis);
            }
            _ => {
                // "yaz" yazılmasa da sonuç gösterilsin.
                self.ifade()?;
                self.kod.push(Komut::Yaz);
            }
        }
        Ok(())
    }

    fn eger(&mut self) -> Result<(), String> {
        self.ifade()?;
        let yanlissa = self.atlama_yaz(Komut::YanlissaAtla(0));
        self.blok()?;
        if self.sonraki() != Some(Sozcuk::Degilse) {
            self.atlama_duzelt(yanlissa);
            return Ok(());
        }
        self.sira += 1;
        let son = self.atlama_yaz(Komut::Atla(0));
        self.atlama_duzelt(yanlissa);
        if self.sonraki() == Some(Sozcuk::Eger) {
            self.sira += 1;
            self.eger()?;
        } else {
            self.blok()?;
        }
        self.atlama_duzelt(son);
        Ok(())
    }

    fn blok(&mut self) -> Result<(), String> {
        self.bekle(Sozcuk::AcSuslu, "'{' bekleniyordu")?;
        self.program()?;
        self.bekle(Sozcuk::KapaSuslu, "'}' eksik")
    }

    // Hedefi henüz bilinmeyen bir atlama yazar; yerini döndürür.
    fn atlama_yaz(&mut self, komut: Komut) -> usize {
        self.kod.push(komut);
        self.kod.len() - 1
    }

    // Atlamanın hedefini "şu an bulunduğumuz yer" yapar.
    fn atlama_duzelt(&mut self, yer: usize) {
        let hedef = self.kod.len();
        match &mut self.kod[yer] {
            Komut::Atla(h) | Komut::YanlissaAtla(h) => *h = hedef,
            _ => unreachable!(),
        }
    }

    fn ifade(&mut self) -> Result<(), String> {
        self.toplam()?;
        let komut = match self.sonraki() {
            Some(Sozcuk::EsitEsit) => Komut::Esit,
            Some(Sozcuk::EsitDegil) => Komut::EsitDegil,
            Some(Sozcuk::Kucuk) => Komut::Kucuk,
            Some(Sozcuk::Buyuk) => Komut::Buyuk,
            Some(Sozcuk::KucukEsit) => Komut::KucukEsit,
            Some(Sozcuk::BuyukEsit) => Komut::BuyukEsit,
            _ => return Ok(()),
        };
        self.sira += 1;
        self.toplam()?;
        self.kod.push(komut);
        Ok(())
    }

    fn toplam(&mut self) -> Result<(), String> {
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
            Sozcuk::Metin(m) => self.kod.push(Komut::Metin(Rc::from(m))),
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
                self.bekle(Sozcuk::KapaParantez, "')' eksik")?;
            }
            _ => return Err("burada bir sayı, metin ya da değişken bekleniyordu".into()),
        }
        Ok(())
    }
}
