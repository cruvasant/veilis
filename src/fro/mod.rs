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
use makine::Fonksiyon;

/// Klavye kesmesi Ctrl+C'de bunu açar; çalışan döngü durur.
pub static DURDUR: AtomicBool = AtomicBool::new(false);

/// Bir Fro oturumu: tanımlanan değişkenler çalıştırmalar arasında kalır.
pub struct Ortam {
    adlar: Vec<String>,
    // None: adı derlenmiş ama henüz değer atanmamış.
    degerler: Vec<Option<Deger>>,
    fonksiyonlar: Vec<Fonksiyon>,
    // Heap'te kalan boş yeri söyler; Fro belleği bitirip çekirdeği çökertmesin diye.
    bos_bellek: fn() -> usize,
}

impl Ortam {
    pub const fn new(bos_bellek: fn() -> usize) -> Self {
        Ortam { adlar: Vec::new(), degerler: Vec::new(), fonksiyonlar: Vec::new(), bos_bellek }
    }

    /// Kodu çalıştırır; ekrana yazılacak her değer için `yaz` çağrılır.
    pub fn calistir(&mut self, kod: &str, yaz: &mut dyn FnMut(&Deger)) -> Result<(), String> {
        DURDUR.store(false, Ordering::Relaxed);
        let sozcukler = sozcuk::bol(kod)?;
        let onceki = self.fonksiyonlar.len();
        let komutlar = derleyici::derle(&sozcukler, &mut self.adlar, &mut self.fonksiyonlar)
            // Derlenemeyen satır yarım fonksiyon bırakmasın.
            .inspect_err(|_| self.fonksiyonlar.truncate(onceki))?;
        self.degerler.resize(self.adlar.len(), None);
        makine::calistir(
            &komutlar,
            &self.fonksiyonlar,
            &mut self.degerler,
            &self.adlar,
            self.bos_bellek,
            yaz,
        )
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
    let f = &mut Ortam::new(|| usize::MAX);

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
    // Fonksiyonlar
    assert_eq!(dene(f, "fonksiyon kare(x) { döndür x * x }"), tamam(""));
    assert_eq!(dene(f, "kare(7); yaz kare(2) + 1"), tamam("49 5"));
    assert_eq!(
        dene(f, "fonksiyon fakt(n) {\n eğer n <= 1 { döndür 1 }\n döndür n * fakt(n - 1)\n}"),
        tamam("")
    );
    assert_eq!(dene(f, "fakt(10)"), tamam("3628800"));
    assert_eq!(dene(f, "fonksiyon selam(kim) { yaz \"merhaba \" + kim }"), tamam(""));
    assert_eq!(dene(f, "selam(\"Veilis\")"), tamam("merhaba Veilis"));
    assert_eq!(dene(f, "fonksiyon topla(a, b) { döndür a + b }; topla(2, 3)"), tamam("5"));
    // Yerel değişkenler dışarı sızmaz, parametre genel değişkeni gölgeler.
    assert_eq!(dene(f, "fonksiyon gizli() { g = 5; döndür g }; gizli()"), tamam("5"));
    assert!(dene(f, "g").is_err());
    assert_eq!(dene(f, "v = 1; fonksiyon artir(v) { döndür v + 1 }; artir(10); v"), tamam("11 1"));
    // Listeler
    assert_eq!(dene(f, "l = [1, 2, 3]; l; l[0] + l[2]"), tamam("[1, 2, 3] 4"));
    assert_eq!(dene(f, "l[1] = 20; ekle(l, 4); l; uzunluk(l)"), tamam("[1, 20, 3, 4] 4"));
    // Liste bir değer: kopyayı değiştirmek aslını etkilemez.
    assert_eq!(dene(f, "m = l; m[0] = 99; l[0]; m[0]"), tamam("1 99"));
    assert_eq!(dene(f, "[]; [1] + [2]; [1, [2, \"a\"]]"), tamam("[] [1, 2] [1, [2, \"a\"]]"));
    assert_eq!(dene(f, "uzunluk(\"çay\"); \"çay\"[0]; -l[1]"), tamam("3 ç -20"));
    assert_eq!(dene(f, "fonksiyon ilk(x) { döndür x[0] }; ilk([7, 8])"), tamam("7"));
    assert_eq!(
        dene(f, "s = []; i = 0; döngü i < 5 { ekle(s, i * i); i = i + 1 }; s"),
        tamam("[0, 1, 4, 9, 16]")
    );
    // Hatalar
    for hatali in [
        "l[10]",
        "l[-1]",
        "5[0]",
        "l[\"a\"]",
        "ekle(5, 1)",
        "ekle(l)",
        "uzunluk(5)",
        "uzunluk(l, l)",
        "fonksiyon uzunluk(x) { }",
        "[1, 2",
        "kare(1, 2)",
        "olmayan()",
        "döndür 1",
        "fonksiyon (x) { }",
        "fonksiyon h( { }",
        "fonksiyon d() { fonksiyon i() { } }",
        "fonksiyon sonsuz() { döndür sonsuz() }; sonsuz()",
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

    // Bellek dolunca liste büyümez ama hesap çalışmaya devam eder.
    let dolu = &mut Ortam::new(|| 0);
    assert!(dene(dolu, "[1]").is_err());
    assert_eq!(dene(dolu, "1 + 1"), tamam("2"));
}
