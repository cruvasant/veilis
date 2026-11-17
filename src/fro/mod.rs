// Fro: Veilis'in kendi programlama dili.
// Bir satır üç adımda çalışır:
//   1. sozcuk:    metni sözcüklere böler
//   2. derleyici: sözcükleri sanal makine komutlarına (bytecode) çevirir
//   3. makine:    komutları çalıştırır
// Değişkenler derlerken numaraya çevrildiği için çalışırken isim aranmaz.

mod derleyici;
mod makine;
mod sozcuk;

use alloc::string::String;
use alloc::vec::Vec;

/// Bir Fro oturumu: tanımlanan değişkenler satırlar arasında kalır.
pub struct Ortam {
    adlar: Vec<String>,
    degerler: Vec<i64>,
}

impl Ortam {
    pub const fn new() -> Self {
        Ortam { adlar: Vec::new(), degerler: Vec::new() }
    }

    /// Bir satırı çalıştırır, ekrana yazılacak değerleri döndürür.
    pub fn calistir(&mut self, satir: &str) -> Result<Vec<i64>, String> {
        let onceki = self.adlar.len();
        let sonuc = sozcuk::bol(satir)
            .and_then(|sozcukler| derleyici::derle(&sozcukler, &mut self.adlar))
            .and_then(|kod| {
                self.degerler.resize(self.adlar.len(), 0);
                makine::calistir(&kod, &mut self.degerler)
            });
        // Hatalı satır yeni değişken bırakmasın.
        if sonuc.is_err() {
            self.adlar.truncate(onceki);
            self.degerler.truncate(onceki);
        }
        sonuc
    }
}

/// Açılışta (debug derlemede) Fro'nun temel davranışlarını kontrol eder.
#[cfg(debug_assertions)]
pub fn oz_denetim() {
    use alloc::vec;
    let mut f = Ortam::new();
    assert_eq!(f.calistir("2 + 3 * 4"), Ok(vec![14]));
    assert_eq!(f.calistir("(2 + 3) * 4 - -1"), Ok(vec![21]));
    assert_eq!(f.calistir("17 / 5"), Ok(vec![3]));
    assert_eq!(f.calistir("17 % 5"), Ok(vec![2]));
    assert_eq!(f.calistir("çarpım = 6 * 7"), Ok(vec![]));
    assert_eq!(f.calistir("yaz çarpım + 1"), Ok(vec![43]));
    assert_eq!(f.calistir("çarpım = çarpım * 2"), Ok(vec![]));
    assert_eq!(f.calistir("çarpım"), Ok(vec![84]));
    assert!(f.calistir("1 / 0").is_err());
    assert!(f.calistir("9223372036854775807 + 1").is_err());
    assert!(f.calistir("x = 1 / 0").is_err());
    assert!(f.calistir("x").is_err()); // hatalı satır x'i tanımlamamalı
    assert!(f.calistir("y = y + 1").is_err());
    assert!(f.calistir("2 +").is_err());
    assert!(f.calistir("(2").is_err());
    assert!(f.calistir("2 3").is_err());
    assert!(f.calistir("2 # 3").is_err());
    assert_eq!(f.calistir(""), Ok(vec![]));
}
