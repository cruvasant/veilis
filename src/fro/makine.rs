// 3. adım: sanal makine. Komutları sırayla çalıştırır, ara sonuçları yığında tutar.
//   Sayi(2), Sayi(3), Topla, Yaz  ->  yığın: [2] -> [2, 3] -> [5] -> ekrana: 5
// `sira` hangi komutta olduğumuzu tutar; atlama komutları onu değiştirir.
// Fonksiyon çağrısında o anki yer bir "çerçeve" olarak saklanır; döndür ile geri dönülür.

use alloc::format;
use alloc::rc::Rc;
use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;
use core::mem::size_of;
use core::sync::atomic::Ordering;

use super::DURDUR;

/// Bir değişkenin yeri: genel değişkenler ya da o anki çağrının yerelleri.
#[derive(Clone, Copy, Debug)]
pub enum Yer {
    Genel(usize),
    Yerel(usize),
}

#[derive(Clone, Debug)]
pub enum Komut {
    Sayi(i64),
    Metin(Rc<str>),
    // Yığındaki son n değerden liste yapar.
    Liste(usize),
    Oku(usize),
    Ata(usize),
    YerelOku(usize),
    YerelAta(usize),
    // (fonksiyon numarası, argüman sayısı)
    Cagir(usize, usize),
    Dondur,
    Indeks,
    IndeksAta(Yer),
    Ekle(Yer),
    Uzunluk,
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

#[derive(Debug)]
pub struct Fonksiyon {
    pub ad: String,
    pub parametre: usize,
    // Parametreler dahil yerel değişken sayısı.
    pub yerel_sayisi: usize,
    pub kod: Vec<Komut>,
}

impl Fonksiyon {
    pub fn new(ad: &str, parametre: usize, yerel_sayisi: usize, kod: Vec<Komut>) -> Self {
        Fonksiyon { ad: String::from(ad), parametre, yerel_sayisi, kod }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Deger {
    Sayi(i64),
    // Rc: kopyalarken metnin kendisini değil, yalnızca adresini kopyalarız.
    Metin(Rc<str>),
    // Liste bir değerdir: "b = a" sonrası b'yi değiştirmek a'yı etkilemez.
    // Rc::make_mut, liste paylaşılıyorsa değiştirmeden önce kopyalar; paylaşılmıyorsa kopyalamaz.
    Liste(Rc<Vec<Deger>>),
    // döndür kullanmadan biten fonksiyonun sonucu; ekrana yazılmaz.
    Yok,
}

impl Deger {
    // Karşılaştırmalar 1 (doğru) ya da 0 (yanlış) üretir. 0, boş metin, boş liste ve yok yanlıştır.
    fn dogru_mu(&self) -> bool {
        match self {
            Deger::Sayi(n) => *n != 0,
            Deger::Metin(m) => !m.is_empty(),
            Deger::Liste(l) => !l.is_empty(),
            Deger::Yok => false,
        }
    }

    // shortcut: yazıya çevrilince kaplayacağı yer için kaba tahmin; bellek kontrolü için yeterli.
    fn tahmini_boyut(&self) -> usize {
        match self {
            Deger::Metin(m) => m.len(),
            Deger::Liste(l) => l.len() * 24,
            _ => 24,
        }
    }
}

impl fmt::Display for Deger {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Deger::Sayi(n) => write!(f, "{}", n),
            Deger::Metin(m) => write!(f, "{}", m),
            Deger::Liste(l) => {
                write!(f, "[")?;
                for (i, d) in l.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    match d {
                        Deger::Metin(m) => write!(f, "\"{}\"", m)?,
                        d => write!(f, "{}", d)?,
                    }
                }
                write!(f, "]")
            }
            Deger::Yok => write!(f, "yok"),
        }
    }
}

const TASMA: &str = "sonuç sığmıyor (64 bit sınırı aşıldı)";
const TUR_HATASI: &str = "bu işlem bu türlerle yapılamaz";
const ATANMAMIS: &str = "değişkene değer atanmadan kullanıldı";
// Heap küçük; sonsuz özyineleme belleği bitirmeden dursun.
const EN_DERIN_CAGRI: usize = 200;
// Çekirdeğin geri kalanı için heap'te hep bu kadar boş yer bırakıyoruz.
const YEDEK_BELLEK: usize = 16 * 1024;
const DEGER: usize = size_of::<Deger>();

// Bellek biterse çekirdek çöker; büyüyen her işlemden önce yer var mı diye bakıyoruz.
// shortcut: toplam boş yere bakıyor; heap çok parçalanırsa yine de yetmeyebilir.
fn yer_var_mi(bos_bellek: fn() -> usize, gereken: usize) -> Result<(), String> {
    if bos_bellek() >= gereken.saturating_add(YEDEK_BELLEK) {
        Ok(())
    } else {
        Err("bellek yetmedi".into())
    }
}

struct Cerceve<'k> {
    kod: &'k [Komut],
    sira: usize,
    // Bu çağrının yerel değişkenlerinin `yereller` içindeki başlangıcı.
    taban: usize,
}

pub fn calistir(
    ana_kod: &[Komut],
    fonksiyonlar: &[Fonksiyon],
    degerler: &mut [Option<Deger>],
    adlar: &[String],
    bos_bellek: fn() -> usize,
    yaz: &mut dyn FnMut(&Deger),
) -> Result<(), String> {
    let mut yigin: Vec<Deger> = Vec::with_capacity(16);
    let mut yereller: Vec<Option<Deger>> = Vec::new();
    let mut cagiranlar: Vec<Cerceve> = Vec::new();
    let mut c = Cerceve { kod: ana_kod, sira: 0, taban: 0 };

    loop {
        let Some(komut) = c.kod.get(c.sira) else {
            // Kod bitti: ana programsa çık, fonksiyonsa "yok" döndür.
            let Some(onceki) = cagiranlar.pop() else { return Ok(()) };
            yereller.truncate(c.taban);
            yigin.push(Deger::Yok);
            c = onceki;
            continue;
        };
        c.sira += 1;
        match komut {
            Komut::Sayi(n) => yigin.push(Deger::Sayi(*n)),
            Komut::Metin(m) => yigin.push(Deger::Metin(m.clone())),
            Komut::Liste(adet) => {
                yer_var_mi(bos_bellek, adet * DEGER)?;
                let elemanlar: Vec<Deger> = yigin.drain(yigin.len() - adet..).collect();
                yigin.push(Deger::Liste(Rc::new(elemanlar)));
            }
            Komut::Oku(yer) => {
                let deger = degerler[*yer]
                    .clone()
                    .ok_or_else(|| format!("tanımsız değişken: {}", adlar[*yer]))?;
                yigin.push(deger);
            }
            Komut::Ata(yer) => degerler[*yer] = Some(cek(&mut yigin)),
            Komut::YerelOku(yer) => {
                let deger = yereller[c.taban + yer].clone().ok_or(ATANMAMIS)?;
                yigin.push(deger);
            }
            Komut::YerelAta(yer) => yereller[c.taban + yer] = Some(cek(&mut yigin)),
            Komut::Indeks => {
                let sira = cek(&mut yigin);
                let hedef = cek(&mut yigin);
                yigin.push(indeks(&hedef, &sira)?);
            }
            Komut::IndeksAta(yer) => {
                let deger = cek(&mut yigin);
                let sira = cek(&mut yigin);
                let liste = liste_yuvasi(*yer, degerler, &mut yereller, c.taban, bos_bellek, false)?;
                let i = sira_bul(&sira, liste.len())?;
                liste[i] = deger;
            }
            Komut::Ekle(yer) => {
                let deger = cek(&mut yigin);
                liste_yuvasi(*yer, degerler, &mut yereller, c.taban, bos_bellek, true)?.push(deger);
                yigin.push(Deger::Yok);
            }
            Komut::Uzunluk => {
                let uzunluk = match cek(&mut yigin) {
                    Deger::Liste(l) => l.len(),
                    Deger::Metin(m) => m.chars().count(),
                    _ => return Err("uzunluk yalnızca liste ve metin için kullanılır".into()),
                };
                yigin.push(Deger::Sayi(uzunluk as i64));
            }
            Komut::Cagir(numara, arguman) => {
                let f = &fonksiyonlar[*numara];
                if f.parametre != *arguman {
                    return Err(format!("{} {} değer bekliyor, {} verildi", f.ad, f.parametre, arguman));
                }
                if cagiranlar.len() >= EN_DERIN_CAGRI {
                    return Err("çağrılar çok derinleşti (sonsuz özyineleme olabilir)".into());
                }
                if DURDUR.swap(false, Ordering::Relaxed) {
                    return Err("durduruldu (Ctrl+C)".into());
                }
                // Argümanlar yığından yerel değişkenlere taşınır; kalan yereller boş başlar.
                let taban = yereller.len();
                yereller.extend(yigin.drain(yigin.len() - arguman..).map(Some));
                yereller.resize(taban + f.yerel_sayisi, None);
                let yeni = Cerceve { kod: &f.kod, sira: 0, taban };
                cagiranlar.push(core::mem::replace(&mut c, yeni));
            }
            Komut::Dondur => {
                let sonuc = cek(&mut yigin);
                yereller.truncate(c.taban);
                c = cagiranlar.pop().expect("Fro: fonksiyon dışında döndür");
                yigin.push(sonuc);
            }
            Komut::Yaz => match cek(&mut yigin) {
                Deger::Yok => {}
                deger => yaz(&deger),
            },
            Komut::Atla(hedef) => {
                // Geriye atlamak döngü demek: sonsuz döngüden Ctrl+C ile çıkılabilsin.
                if *hedef < c.sira && DURDUR.swap(false, Ordering::Relaxed) {
                    return Err("durduruldu (Ctrl+C)".into());
                }
                c.sira = *hedef;
            }
            Komut::YanlissaAtla(hedef) => {
                if !cek(&mut yigin).dogru_mu() {
                    c.sira = *hedef;
                }
            }
            Komut::Negatif => match cek(&mut yigin) {
                Deger::Sayi(n) => yigin.push(Deger::Sayi(n.checked_neg().ok_or(TASMA)?)),
                _ => return Err(TUR_HATASI.into()),
            },
            islem => {
                let b = cek(&mut yigin);
                let a = cek(&mut yigin);
                yigin.push(iki_degerle(islem, a, b, bos_bellek)?);
            }
        }
    }
}

// Bir değişkendeki listeyi değiştirmek için açar. Liste başka yerde de kullanılıyorsa
// make_mut önce kopyalar; o yüzden kopya için de yer var mı diye bakıyoruz.
fn liste_yuvasi<'d>(
    yer: Yer,
    degerler: &'d mut [Option<Deger>],
    yereller: &'d mut [Option<Deger>],
    taban: usize,
    bos_bellek: fn() -> usize,
    buyuyecek: bool,
) -> Result<&'d mut Vec<Deger>, String> {
    let yuva = match yer {
        Yer::Genel(i) => &mut degerler[i],
        Yer::Yerel(i) => &mut yereller[taban + i],
    };
    match yuva {
        Some(Deger::Liste(l)) => {
            let kopya = if Rc::strong_count(l) > 1 { l.len() } else { 0 };
            // Dolu bir Vec büyürken eski ve yeni (2 kat) alan bir süre birlikte durur.
            let buyume = if buyuyecek && l.len() == l.capacity() {
                (l.capacity() * 2).max(4)
            } else {
                0
            };
            yer_var_mi(bos_bellek, (kopya + buyume) * DEGER)?;
            Ok(Rc::make_mut(l))
        }
        Some(_) => Err("bu değişken bir liste değil".into()),
        None => Err(ATANMAMIS.into()),
    }
}

fn sira_bul(sira: &Deger, uzunluk: usize) -> Result<usize, String> {
    match sira {
        Deger::Sayi(i) if *i >= 0 && (*i as u64) < uzunluk as u64 => Ok(*i as usize),
        Deger::Sayi(i) => Err(format!("uzunluk {}, {}. sıra yok (sıralar 0'dan başlar)", uzunluk, i)),
        _ => Err("sıra bir sayı olmalı".into()),
    }
}

fn indeks(hedef: &Deger, sira: &Deger) -> Result<Deger, String> {
    match hedef {
        Deger::Liste(l) => Ok(l[sira_bul(sira, l.len())?].clone()),
        Deger::Metin(m) => {
            let i = sira_bul(sira, m.chars().count())?;
            let harf = m.chars().nth(i).unwrap_or_default();
            Ok(Deger::Metin(Rc::from(harf.encode_utf8(&mut [0; 4]) as &str)))
        }
        _ => Err("sıra yalnızca liste ve metinde kullanılır".into()),
    }
}

fn iki_degerle(islem: &Komut, a: Deger, b: Deger, bos_bellek: fn() -> usize) -> Result<Deger, String> {
    use Deger::{Liste, Metin, Sayi};
    let mantik = |dogru: bool| Ok(Sayi(dogru as i64));
    let sayi = |sonuc: Option<i64>| sonuc.map(Sayi).ok_or_else(|| String::from(TASMA));

    match (islem, a, b) {
        (Komut::Esit, a, b) => mantik(a == b),
        (Komut::EsitDegil, a, b) => mantik(a != b),
        (Komut::Topla, Sayi(a), Sayi(b)) => sayi(a.checked_add(b)),
        // En az biri metinse yan yana ekle: "sonuç: " + 5 -> "sonuç: 5"
        (Komut::Topla, a @ Metin(_), b) | (Komut::Topla, a, b @ Metin(_)) => {
            yer_var_mi(bos_bellek, 2 * (a.tahmini_boyut() + b.tahmini_boyut()))?;
            Ok(Metin(Rc::from(format!("{}{}", a, b))))
        }
        (Komut::Topla, Liste(a), Liste(b)) => {
            yer_var_mi(bos_bellek, (a.len() + b.len()) * DEGER)?;
            Ok(Liste(Rc::new(a.iter().chain(b.iter()).cloned().collect())))
        }
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
