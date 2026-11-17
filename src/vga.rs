// VGA metin modu: 0xb8000 adresinde 80x25 hücrelik bir tablo.
// Her hücre 16 bit: alttaki 8 bit karakter, üstteki 8 bit renk.

use core::fmt;
use spin::Mutex;

pub const GENISLIK: usize = 80;
pub const YUKSEKLIK: usize = 25;
const TAMPON: *mut u16 = 0xb8000 as *mut u16;

// Siyah zemin üstüne açık camgöbeği
const VARSAYILAN_RENK: u8 = 0x0b;

// Türkçe harflerin VGA yazı tipindeki yerleri. ç ö ü zaten var (CP437);
// ı İ ş Ş ğ Ğ yok, onları kullanmadığımız yuvalara kendimiz çiziyoruz.
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
        unsafe { TAMPON.add(satir * GENISLIK + sutun) }
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
        Ok(())
    }
}

/// Ekranın istenen hücresine doğrudan yazar (düzenleyici tüm ekranı kendisi çizer).
pub fn hucre_yaz(satir: usize, sutun: usize, c: char, renk: u8) {
    if satir < YUKSEKLIK && sutun < GENISLIK {
        let deger = (renk as u16) << 8 | karakter_kodu(c) as u16;
        unsafe { TAMPON.add(satir * GENISLIK + sutun).write_volatile(deger) };
    }
}

// Yazı tipi, ekran kartı belleğinin 2. düzleminde (plane 2) durur: her karakter
// 32 baytlık bir resim, her bayt 8 piksellik bir satır. Normalde oraya erişilemez;
// birkaç ayarla geçici olarak açıp `islem`i çalıştırır, sonra ayarları geri alırız.
fn font_belleginde(islem: impl FnOnce(*mut u8)) {
    unsafe {
        vga_ayar(0x3c4, 0x02, 0x04); // yalnızca 2. düzleme yaz
        vga_ayar(0x3c4, 0x04, 0x07); // düz (sıralı) adresleme
        vga_ayar(0x3ce, 0x04, 0x02); // 2. düzlemden oku
        vga_ayar(0x3ce, 0x05, 0x00);
        vga_ayar(0x3ce, 0x06, 0x04); // belleği 0xa0000'a bağla
    }

    islem(0xa0000 as *mut u8);

    unsafe {
        // Metin modunun normal ayarlarına dön.
        vga_ayar(0x3c4, 0x02, 0x03);
        vga_ayar(0x3c4, 0x04, 0x03);
        vga_ayar(0x3ce, 0x04, 0x00);
        vga_ayar(0x3ce, 0x05, 0x10);
        vga_ayar(0x3ce, 0x06, 0x0e);
    }
}

// 256 karakterin 8x16 resimleri (karakter başına 16 bayt). Piksel moduna geçince
// bu bellek görünmez olur, o yüzden geçmeden önce kopyalanır.
pub fn font_kopyala(hedef: &mut [u8; 256 * 16]) {
    font_belleginde(|font| {
        for (karakter, resim) in hedef.chunks_exact_mut(16).enumerate() {
            for (satir, bayt) in resim.iter_mut().enumerate() {
                *bayt = unsafe { font.add(karakter * 32 + satir).read_volatile() };
            }
        }
    });
}

pub fn turkce_harfleri_yukle() {
    font_belleginde(|font| {
        let oku = |karakter: u8| {
            let mut resim = [0u8; 16];
            for (satir, bayt) in resim.iter_mut().enumerate() {
                *bayt = unsafe { font.add(karakter as usize * 32 + satir).read_volatile() };
            }
            resim
        };
        let yaz = |yuva: u8, resim: [u8; 16]| {
            for (satir, bayt) in resim.iter().enumerate() {
                unsafe { font.add(yuva as usize * 32 + satir).write_volatile(*bayt) };
            }
        };

        let mut noktasiz_i = oku(b'i');
        noktasiz_i[..5].fill(0); // noktayı sil
        yaz(0x98, noktasiz_i);

        let mut i_buyuk = oku(b'I');
        i_buyuk[0] = 0x18; // nokta ekle
        yaz(0x9b, i_buyuk);

        for (kaynak, yuva) in [(b's', 0x9c), (b'S', 0x9d)] {
            let mut s = oku(kaynak);
            s[12..15].copy_from_slice(&[0x18, 0x0c, 0x38]); // altına çengel
            yaz(yuva, s);
        }

        let mut g = oku(b'g');
        g[2..4].copy_from_slice(&[0x44, 0x38]); // üstüne kısa çizgi (breve)
        yaz(0x9e, g);

        let mut g_buyuk = oku(b'G');
        g_buyuk[0..2].copy_from_slice(&[0x42, 0x3c]);
        yaz(0x9f, g_buyuk);
    });
}

// VGA ayarları "indeks portu + veri portu" çiftiyle yazılır.
unsafe fn vga_ayar(port: u16, indeks: u8, deger: u8) {
    use x86_64::instructions::port::Port;
    unsafe {
        Port::<u8>::new(port).write(indeks);
        Port::<u8>::new(port + 1).write(deger);
    }
}

#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    use core::fmt::Write;
    // Yazarken kesmeleri kapatıyoruz: yoksa klavye kesmesi de yazmaya çalışıp
    // kilidi sonsuza kadar bekleyebilir.
    x86_64::instructions::interrupts::without_interrupts(|| {
        YAZICI.lock().write_fmt(args).unwrap();
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
