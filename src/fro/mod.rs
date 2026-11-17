// Fro: Veilis'in kendi programlama dili.
// Kod üç adımda çalışır:
//   1. sozcuk:    metni sözcüklere böler
//   2. derleyici: sözcükleri sanal makine komutlarına (bytecode) çevirir
//   3. makine:    komutları çalıştırır
// Değişkenler derlerken numaraya çevrildiği için çalışırken isim aranmaz.

mod derleyici;
mod makine;
mod sozcuk;

use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, Ordering};

pub use makine::Deger;

/// Klavye kesmesi Ctrl+C'de bunu açar; çalışan döngü durur.
pub static DURDUR: AtomicBool = AtomicBool::new(false);

/// Bir Fro oturumu: tanımlanan değişkenler çalıştırmalar arasında kalır.
pub struct Ortam {
    adlar: Vec<String>,
    // None: adı derlenmiş ama henüz değer atanmamış.
    degerler: Vec<Option<Deger>>,
}

impl Ortam {
    pub const fn new() -> Self {
        Ortam { adlar: Vec::new(), degerler: Vec::new() }
    }

    /// Kodu çalıştırır; ekrana yazılacak her değer için `yaz` çağrılır.
    pub fn calistir(&mut self, kod: &str, yaz: &mut dyn FnMut(&Deger)) -> Result<(), String> {
        DURDUR.store(false, Ordering::Relaxed);
        let sozcukler = sozcuk::bol(kod)?;
        let komutlar = derleyici::derle(&sozcukler, &mut self.adlar)?;
        self.degerler.resize(self.adlar.len(), None);
        makine::calistir(&komutlar, &mut self.degerler, &self.adlar, yaz)
    }
}

/// Açılışta (debug derlemede) Fro'nun temel davranışlarını kontrol eder.
#[cfg(debug_assertions)]
pub fn oz_denetim() {
    use alloc::format;

    fn dene(f: &mut Ortam, kod: &str) -> Result<String, String> {
        let mut cikti: Vec<String> = Vec::new();
        f.calistir(kod, &mut |d| cikti.push(format!("{}", d)))?;
        Ok(cikti.join(" "))
    }
    let tamam = |s: &str| -> Result<String, String> { Ok(String::from(s)) };
    let f = &mut Ortam::new();

    // Hesap
    assert_eq!(dene(f, "2 + 3 * 4"), tamam("14"));
    assert_eq!(dene(f, "(2 + 3) * 4 - -1"), tamam("21"));
    assert_eq!(dene(f, "17 / 5; 17 % 5"), tamam("3 2"));
    // Değişken ve metin
    assert_eq!(dene(f, "çarpım = 6 * 7"), tamam(""));
    assert_eq!(dene(f, "yaz \"sonuç: \" + çarpım"), tamam("sonuç: 42"));
    // Karşılaştırma
    assert_eq!(dene(f, "3 < 5; 3 >= 5; 2 == 2; \"a\" != \"b\""), tamam("1 0 1 1"));
    // eğer / değilse
    assert_eq!(dene(f, "a = 5"), tamam(""));
    assert_eq!(
        dene(f, "eğer a > 3 { yaz \"büyük\" } değilse { yaz \"küçük\" }"),
        tamam("büyük")
    );
    assert_eq!(dene(f, "eğer a > 9 { 1 } değilse eğer a > 4 { 2 } değilse { 3 }"), tamam("2"));
    assert_eq!(dene(f, "eğer 0 { 1 }"), tamam(""));
    // döngü, birden çok satır
    assert_eq!(dene(f, "i = 0; t = 0; döngü i < 5 { i = i + 1; t = t + i }; t"), tamam("15"));
    assert_eq!(dene(f, "i = 3\ndöngü i > 0 {\n  yaz i\n  i = i - 1\n}"), tamam("3 2 1"));
    // Hatalar
    for hatali in [
        "1 / 0",
        "9223372036854775807 + 1",
        "y = y + 1",
        "\"a\" * 2",
        "-\"a\"",
        "2 +",
        "(2",
        "2 3",
        "2 # 3",
        "eğer 1 { 2",
        "eğer 1 2",
        "}",
        "\"kapanmamış",
    ] {
        assert!(dene(f, hatali).is_err(), "hata bekleniyordu: {}", hatali);
    }
    // Hatalı atama değişkeni tanımlamamalı.
    assert!(dene(f, "x = 1 / 0").is_err());
    assert!(dene(f, "x").is_err());
    assert_eq!(dene(f, ""), tamam(""));
}
