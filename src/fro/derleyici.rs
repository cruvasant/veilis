// 2. adım: sözcükleri sanal makinenin komutlarına (bytecode) çevirir.
// Kurallar (dilbilgisi):
//   program = deyim ((";" | yeni satır) deyim)*
//   deyim   = ad "=" ifade | "yaz" ifade | eğer | "döngü" ifade blok
//           | "fonksiyon" ad "(" [ad ("," ad)*] ")" blok | "döndür" ifade | ifade
//   eğer    = "eğer" ifade blok ["değilse" (eğer | blok)]
//   blok    = "{" program "}"
//   ifade   = toplam [("==" | "!=" | "<" | ">" | "<=" | ">=") toplam]
//   toplam  = terim  (("+" | "-") terim)*
//   terim   = çarpan (("*" | "/" | "%") çarpan)*
//   çarpan  = sayı | metin | ad | ad "(" [ifade ("," ifade)*] ")" | "-" çarpan | "(" ifade ")"
// Çarpma toplamadan önce yapılsın diye her kural bir alttakini çağırır.
// Makine yığın (stack) ile çalışır: "2 + 3" -> Sayi(2), Sayi(3), Topla.
// eğer ve döngü, atlama komutlarına dönüşür:
//   döngü k { g }  ->  baş: k, YanlissaAtla(son), g, Atla(baş), son:
// Her fonksiyonun kodu ayrı derlenir. İçindeki değişkenler yereldir (Yerel* komutları);
// yalnızca o çağrı süresince yaşar.

use alloc::format;
use alloc::rc::Rc;
use alloc::string::String;
use alloc::vec::Vec;

use super::makine::{Fonksiyon, Komut};
use super::sozcuk::Sozcuk;

struct Derleyici<'a, 'b> {
    sozcukler: &'b [Sozcuk<'a>],
    sira: usize,
    kod: Vec<Komut>,
    adlar: &'b mut Vec<String>,
    fonksiyonlar: &'b mut Vec<Fonksiyon>,
    // Some: bir fonksiyonun içindeyiz; parametreler ve yerel değişkenlerin adları.
    yereller: Option<Vec<&'a str>>,
}

pub fn derle(
    sozcukler: &[Sozcuk],
    adlar: &mut Vec<String>,
    fonksiyonlar: &mut Vec<Fonksiyon>,
) -> Result<Vec<Komut>, String> {
    let mut d = Derleyici {
        sozcukler,
        sira: 0,
        kod: Vec::new(),
        adlar,
        fonksiyonlar,
        yereller: None,
    };
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
                // Fonksiyon içindeki atama her zaman yerel değişkene yapılır.
                let komut = match &mut self.yereller {
                    Some(yereller) => Komut::YerelAta(yer_ac(yereller, ad)),
                    None => match self.adlar.iter().position(|a| a == ad) {
                        Some(yer) => Komut::Ata(yer),
                        None => {
                            self.adlar.push(String::from(ad));
                            Komut::Ata(self.adlar.len() - 1)
                        }
                    },
                };
                self.kod.push(komut);
            }
            (Some(Sozcuk::Fonksiyon), _) => {
                self.sira += 1;
                self.fonksiyon_tanimi()?;
            }
            (Some(Sozcuk::Dondur), _) => {
                if self.yereller.is_none() {
                    return Err("döndür yalnızca fonksiyon içinde kullanılabilir".into());
                }
                self.sira += 1;
                self.ifade()?;
                self.kod.push(Komut::Dondur);
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

    fn fonksiyon_tanimi(&mut self) -> Result<(), String> {
        if self.yereller.is_some() {
            return Err("fonksiyon içinde fonksiyon tanımlanamaz".into());
        }
        let Some(Sozcuk::Ad(ad)) = self.sonraki() else {
            return Err("fonksiyon adı bekleniyordu".into());
        };
        self.sira += 1;
        self.bekle(Sozcuk::AcParantez, "'(' bekleniyordu")?;
        let mut parametreler = Vec::new();
        while let Some(Sozcuk::Ad(p)) = self.sonraki() {
            parametreler.push(p);
            self.sira += 1;
            if self.sonraki() != Some(Sozcuk::Virgul) {
                break;
            }
            self.sira += 1;
        }
        self.bekle(Sozcuk::KapaParantez, "')' bekleniyordu")?;

        // Kendini çağırabilsin (özyineleme) diye adı gövdeden önce kaydediyoruz.
        let yer = match self.fonksiyonlar.iter().position(|f| f.ad == ad) {
            Some(yer) => yer,
            None => {
                self.fonksiyonlar.push(Fonksiyon::new(ad, parametreler.len(), 0, Vec::new()));
                self.fonksiyonlar.len() - 1
            }
        };

        // Gövdeyi ayrı bir kod listesine derle, sonra ana koda geri dön.
        let parametre = parametreler.len();
        self.yereller = Some(parametreler);
        let ana_kod = core::mem::take(&mut self.kod);
        let sonuc = self.blok();
        let govde = core::mem::replace(&mut self.kod, ana_kod);
        let yerel_sayisi = self.yereller.take().map_or(0, |y| y.len());
        sonuc?;

        self.fonksiyonlar[yer] = Fonksiyon::new(ad, parametre, yerel_sayisi, govde);
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
            Sozcuk::Ad(ad) if self.sonraki() == Some(Sozcuk::AcParantez) => {
                self.sira += 1;
                let mut arguman = 0;
                if self.sonraki() != Some(Sozcuk::KapaParantez) {
                    loop {
                        self.ifade()?;
                        arguman += 1;
                        if self.sonraki() != Some(Sozcuk::Virgul) {
                            break;
                        }
                        self.sira += 1;
                    }
                }
                self.bekle(Sozcuk::KapaParantez, "')' eksik")?;
                let yer = self
                    .fonksiyonlar
                    .iter()
                    .position(|f| f.ad == ad)
                    .ok_or_else(|| format!("tanımsız fonksiyon: {}", ad))?;
                self.kod.push(Komut::Cagir(yer, arguman));
            }
            Sozcuk::Ad(ad) => {
                // İsim burada numaraya çevrilir; çalışırken isim aranmaz.
                // Önce yerel değişkenlere, sonra genel değişkenlere bakılır.
                let yerel = self.yereller.as_ref().and_then(|y| y.iter().position(|a| *a == ad));
                let komut = match yerel {
                    Some(yer) => Komut::YerelOku(yer),
                    None => Komut::Oku(
                        self.adlar
                            .iter()
                            .position(|a| a == ad)
                            .ok_or_else(|| format!("tanımsız değişken: {}", ad))?,
                    ),
                };
                self.kod.push(komut);
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

fn yer_ac<'a>(adlar: &mut Vec<&'a str>, ad: &'a str) -> usize {
    adlar.iter().position(|a| *a == ad).unwrap_or_else(|| {
        adlar.push(ad);
        adlar.len() - 1
    })
}
