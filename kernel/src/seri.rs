// COM1 seri portu (16550): ekran henüz çizilmediği ve gerçek donanımda hata ayıklamak için
// yazıları buradan da dışarı veriyoruz.

use core::fmt;
use x86_64::instructions::port::Port;

const COM1: u16 = 0x3f8;

pub struct Seri;

pub static SERI: spin::Mutex<Seri> = spin::Mutex::new(Seri);

fn yaz_port(kayit: u16, deger: u8) {
    unsafe { Port::<u8>::new(COM1 + kayit).write(deger) };
}

pub fn baslat() {
    yaz_port(1, 0x00); // kesmeler kapalı, yoklayarak yazıyoruz
    yaz_port(3, 0x80); // bölen yazmaçlarını aç
    yaz_port(0, 0x01); // bölen 1: 115200 baud
    yaz_port(1, 0x00);
    yaz_port(3, 0x03); // 8 bit, eşlik yok, 1 durma biti
    yaz_port(2, 0xc7); // FIFO'yu aç ve temizle
    yaz_port(4, 0x0b); // DTR, RTS, OUT2
}

impl Seri {
    fn bayt_yaz(&mut self, bayt: u8) {
        // Hat durum yazmacının 5. biti: gönderme yazmacı boş.
        while unsafe { Port::<u8>::new(COM1 + 5).read() } & 0x20 == 0 {}
        yaz_port(0, bayt);
    }
}

impl fmt::Write for Seri {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for bayt in s.bytes() {
            if bayt == b'\n' {
                self.bayt_yaz(b'\r');
            }
            self.bayt_yaz(bayt);
        }
        Ok(())
    }
}
