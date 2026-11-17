// VGA metin modu: 0xb8000 adresinde 80x25 hücrelik bir tablo.
// Her hücre 16 bit: alttaki 8 bit karakter, üstteki 8 bit renk.

use core::fmt;
use spin::Mutex;

const GENISLIK: usize = 80;
const YUKSEKLIK: usize = 25;
const TAMPON: *mut u16 = 0xb8000 as *mut u16;

// Siyah zemin üstüne açık camgöbeği
const VARSAYILAN_RENK: u8 = 0x0b;

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
}

impl fmt::Write for Yazici {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for c in s.chars() {
            match c {
                ' '..='~' | '\n' => self.bayt_yaz(c as u8),
                '\u{8}' => self.geri_sil(),
                // VGA'da olmayan karakterler (ş, ğ, ı...) yerine ■
                _ => self.bayt_yaz(0xfe),
            }
        }
        Ok(())
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
