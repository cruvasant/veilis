// 3. adım: sanal makine. Komutları sırayla çalıştırır, ara sonuçları yığında tutar.
//   Sayi(2), Sayi(3), Topla, Yaz  ->  yığın: [2] -> [2, 3] -> [5] -> ekrana: 5
// `sira` hangi komutta olduğumuzu tutar; atlama komutları onu değiştirir.

use alloc::format;
use alloc::rc::Rc;
use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;
use core::sync::atomic::Ordering;

use super::DURDUR;

#[derive(Clone, Debug)]
pub enum Komut {
    Sayi(i64),
    Metin(Rc<str>),
    Oku(usize),
    Ata(usize),
    Topla,
    Cikar,
    Carp,
    Bol,
    Kalan,
    Negatif,
    Esit,
    EsitDegil,
    Kucuk,
    Buyuk,
    KucukEsit,
    BuyukEsit,
    Yaz,
    Atla(usize),
    // Yığından bir değer çeker; yanlışsa atlar.
    YanlissaAtla(usize),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Deger {
    Sayi(i64),
    // Rc: kopyalarken metnin kendisini değil, yalnızca adresini kopyalarız.
    Metin(Rc<str>),
}

impl Deger {
    // Karşılaştırmalar 1 (doğru) ya da 0 (yanlış) üretir. 0 ve boş metin yanlıştır.
    fn dogru_mu(&self) -> bool {
        match self {
            Deger::Sayi(n) => *n != 0,
            Deger::Metin(m) => !m.is_empty(),
        }
    }
}

impl fmt::Display for Deger {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Deger::Sayi(n) => write!(f, "{}", n),
            Deger::Metin(m) => write!(f, "{}", m),
        }
    }
}

const TASMA: &str = "sonuç sığmıyor (64 bit sınırı aşıldı)";
const TUR_HATASI: &str = "bu işlem bu türlerle yapılamaz";

pub fn calistir(
    kod: &[Komut],
    degerler: &mut [Option<Deger>],
    adlar: &[String],
    yaz: &mut dyn FnMut(&Deger),
) -> Result<(), String> {
    let mut yigin: Vec<Deger> = Vec::with_capacity(16);
    let mut sira = 0;

    while let Some(komut) = kod.get(sira) {
        sira += 1;
        match komut {
            Komut::Sayi(n) => yigin.push(Deger::Sayi(*n)),
            Komut::Metin(m) => yigin.push(Deger::Metin(m.clone())),
            Komut::Oku(yer) => {
                let deger = degerler[*yer]
                    .clone()
                    .ok_or_else(|| format!("tanımsız değişken: {}", adlar[*yer]))?;
                yigin.push(deger);
            }
            Komut::Ata(yer) => degerler[*yer] = Some(cek(&mut yigin)),
            Komut::Yaz => yaz(&cek(&mut yigin)),
            Komut::Atla(hedef) => {
                // Geriye atlamak döngü demek: sonsuz döngüden Ctrl+C ile çıkılabilsin.
                if *hedef < sira && DURDUR.swap(false, Ordering::Relaxed) {
                    return Err("durduruldu (Ctrl+C)".into());
                }
                sira = *hedef;
            }
            Komut::YanlissaAtla(hedef) => {
                if !cek(&mut yigin).dogru_mu() {
                    sira = *hedef;
                }
            }
            Komut::Negatif => match cek(&mut yigin) {
                Deger::Sayi(n) => yigin.push(Deger::Sayi(n.checked_neg().ok_or(TASMA)?)),
                _ => return Err(TUR_HATASI.into()),
            },
            islem => {
                let b = cek(&mut yigin);
                let a = cek(&mut yigin);
                yigin.push(iki_degerle(islem, a, b)?);
            }
        }
    }
    Ok(())
}

fn iki_degerle(islem: &Komut, a: Deger, b: Deger) -> Result<Deger, String> {
    use Deger::{Metin, Sayi};
    let mantik = |dogru: bool| Ok(Sayi(dogru as i64));
    let sayi = |sonuc: Option<i64>| sonuc.map(Sayi).ok_or_else(|| String::from(TASMA));

    match (islem, a, b) {
        (Komut::Esit, a, b) => mantik(a == b),
        (Komut::EsitDegil, a, b) => mantik(a != b),
        (Komut::Topla, Sayi(a), Sayi(b)) => sayi(a.checked_add(b)),
        // En az biri metinse yan yana ekle: "sonuç: " + 5 -> "sonuç: 5"
        (Komut::Topla, a, b) => Ok(Metin(Rc::from(format!("{}{}", a, b)))),
        (Komut::Cikar, Sayi(a), Sayi(b)) => sayi(a.checked_sub(b)),
        (Komut::Carp, Sayi(a), Sayi(b)) => sayi(a.checked_mul(b)),
        (Komut::Bol | Komut::Kalan, Sayi(_), Sayi(0)) => Err("sıfıra bölünemez".into()),
        (Komut::Bol, Sayi(a), Sayi(b)) => sayi(a.checked_div(b)),
        (Komut::Kalan, Sayi(a), Sayi(b)) => sayi(a.checked_rem(b)),
        (Komut::Kucuk | Komut::Buyuk | Komut::KucukEsit | Komut::BuyukEsit, a, b) => {
            let sira = match (&a, &b) {
                (Sayi(x), Sayi(y)) => x.cmp(y),
                (Metin(x), Metin(y)) => x.cmp(y),
                _ => return Err(TUR_HATASI.into()),
            };
            mantik(match islem {
                Komut::Kucuk => sira.is_lt(),
                Komut::Buyuk => sira.is_gt(),
                Komut::KucukEsit => sira.is_le(),
                _ => sira.is_ge(),
            })
        }
        _ => Err(TUR_HATASI.into()),
    }
}

// Derleyici yığını hep dengeli kod üretir; boş yığından çekmek derleyici hatası demektir.
fn cek(yigin: &mut Vec<Deger>) -> Deger {
    yigin.pop().expect("Fro: bozuk bytecode")
}
