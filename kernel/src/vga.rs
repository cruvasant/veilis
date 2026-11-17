// Metin tablosu: RAM'de 80x25 hücre (VGA metin modundaki düzen). Ekrana çizmek
// grafik tarafının işi; burada yalnızca hücreler tutulur.

use core::fmt;
use core::sync::atomic::{AtomicBool, AtomicPtr, Ordering};
use spin::Mutex;

pub const GENISLIK: usize = 80;
pub const YUKSEKLIK: usize = 25;

// Başlangıçta boş hücreler; yazılar buraya gider.
static mut TABLO: [u16; GENISLIK * YUKSEKLIK] = [(VARSAYILAN_RENK as u16) << 8 | b' ' as u16; GENISLIK * YUKSEKLIK];
static TAMPON: AtomicPtr<u16> = AtomicPtr::new(&raw mut TABLO as *mut u16);
// Her yazıda kurulur; arayüz bununla yeniden çizmesi gerekip gerekmediğini anlar.
static DEGISTI: AtomicBool = AtomicBool::new(false);
// Düzenleyici hücreleri kendisi yazar; o zaman kabuğun imleci ekranda anlamsızdır.
static METIN_IMLECI: AtomicBool = AtomicBool::new(true);

// Siyah zemin üstüne açık camgöbeği
const VARSAYILAN_RENK: u8 = 0x0b;

// Türkçe harflerin hücre tablosundaki kodları (VGA yazı tipi düzeni). ç ö ü CP437'de var;
// ı İ ş Ş ğ Ğ için kullanılmayan yuvalar ayrıldı, çizimleri ekran tarafında yapılacak.
const TURKCE: [(char, u8); 12] = [
    ('ç', 0x87), ('Ç', 0x80), ('ö', 0x94), ('Ö', 0x99), ('ü', 0x81), ('Ü', 0x9a),
    ('ı', 0x98), ('İ', 0x9b), ('ş', 0x9c), ('Ş', 0x9d), ('ğ', 0x9e), ('Ğ', 0x9f),
];

pub struct Yazici {
    satir: usize,
    sutun: usize,
    renk: u8,
}

pub static YAZICI: Mutex<Yazici> = Mutex::new(Yazici {
    satir: 0,
    sutun: 0,
    renk: VARSAYILAN_RENK,
});

impl Yazici {
    // Hücreye hep volatile okuyup yazıyoruz; yoksa derleyici ekrana yazmayı
    // "sonucu kullanılmıyor" diye silebilir.
    fn hucre(&self, satir: usize, sutun: usize) -> *mut u16 {
        unsafe { TAMPON.load(Ordering::Relaxed).add(satir * GENISLIK + sutun) }
    }

    fn bayt_yaz(&mut self, bayt: u8) {
        if bayt == b'\n' {
            self.yeni_satir();
            return;
        }
        if self.sutun >= GENISLIK {
            self.yeni_satir();
        }
        let deger = (self.renk as u16) << 8 | bayt as u16;
        unsafe { self.hucre(self.satir, self.sutun).write_volatile(deger) };
        self.sutun += 1;
    }

    // Backspace: yalnızca bulunduğumuz satırda geri gider.
    fn geri_sil(&mut self) {
        if self.sutun > 0 {
            self.sutun -= 1;
            let bos = (self.renk as u16) << 8 | b' ' as u16;
            unsafe { self.hucre(self.satir, self.sutun).write_volatile(bos) };
        }
    }

    fn yeni_satir(&mut self) {
        self.sutun = 0;
        if self.satir < YUKSEKLIK - 1 {
            self.satir += 1;
            return;
        }
        // Son satırdayız: her şeyi bir satır yukarı kaydır.
        for satir in 1..YUKSEKLIK {
            for sutun in 0..GENISLIK {
                unsafe {
                    let deger = self.hucre(satir, sutun).read_volatile();
                    self.hucre(satir - 1, sutun).write_volatile(deger);
                }
            }
        }
        let bos = (self.renk as u16) << 8 | b' ' as u16;
        for sutun in 0..GENISLIK {
            unsafe { self.hucre(YUKSEKLIK - 1, sutun).write_volatile(bos) };
        }
    }

    fn temizle(&mut self) {
        let bos = (self.renk as u16) << 8 | b' ' as u16;
        for satir in 0..YUKSEKLIK {
            for sutun in 0..GENISLIK {
                unsafe { self.hucre(satir, sutun).write_volatile(bos) };
            }
        }
        self.satir = 0;
        self.sutun = 0;
        METIN_IMLECI.store(true, Ordering::Relaxed);
        DEGISTI.store(true, Ordering::Relaxed);
    }
}

pub fn temizle() {
    x86_64::instructions::interrupts::without_interrupts(|| YAZICI.lock().temizle());
}

// Karakterin VGA yazı tipindeki numarası.
pub(crate) fn karakter_kodu(c: char) -> u8 {
    match c {
        ' '..='~' => c as u8,
        _ => match TURKCE.iter().find(|(harf, _)| *harf == c) {
            Some(&(_, yuva)) => yuva,
            // Gösteremediğimiz karakterler yerine ■
            None => 0xfe,
        },
    }
}

impl fmt::Write for Yazici {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for c in s.chars() {
            match c {
                '\n' => self.bayt_yaz(b'\n'),
                '\u{8}' => self.geri_sil(),
                _ => self.bayt_yaz(karakter_kodu(c)),
            }
        }
        DEGISTI.store(true, Ordering::Relaxed);
        Ok(())
    }
}

/// Ekranın istenen hücresine doğrudan yazar (düzenleyici tüm ekranı kendisi çizer).
pub fn hucre_yaz(satir: usize, sutun: usize, c: char, renk: u8) {
    if satir < YUKSEKLIK && sutun < GENISLIK {
        let deger = (renk as u16) << 8 | karakter_kodu(c) as u16;
        unsafe { TAMPON.load(Ordering::Relaxed).add(satir * GENISLIK + sutun).write_volatile(deger) };
        METIN_IMLECI.store(false, Ordering::Relaxed);
        DEGISTI.store(true, Ordering::Relaxed);
    }
}

pub fn kirli_al() -> bool {
    DEGISTI.swap(false, Ordering::Relaxed)
}

/// Hücreleri `hedef`e kopyalar; (imleç satırı, imleç sütunu, imleç görünsün mü) döner.
pub fn goruntu(hedef: &mut [u16; GENISLIK * YUKSEKLIK]) -> (usize, usize, bool) {
    x86_64::instructions::interrupts::without_interrupts(|| {
        let yazici = YAZICI.lock();
        for (i, hucre) in hedef.iter_mut().enumerate() {
            *hucre = unsafe { TAMPON.load(Ordering::Relaxed).add(i).read_volatile() };
        }
        (yazici.satir, yazici.sutun.min(GENISLIK - 1), METIN_IMLECI.load(Ordering::Relaxed))
    })
}

#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    use core::fmt::Write;
    // Yazarken kesmeleri kapatıyoruz: yoksa klavye kesmesi de yazmaya çalışıp
    // kilidi sonsuza kadar bekleyebilir.
    x86_64::instructions::interrupts::without_interrupts(|| {
        YAZICI.lock().write_fmt(args).unwrap();
        crate::seri::SERI.lock().write_fmt(args).unwrap();
    });
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::vga::_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}
