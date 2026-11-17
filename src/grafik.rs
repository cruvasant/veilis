// Piksel modu: QEMU'nun Bochs/BGA ekran kartıyla 1024x768, 32 bit doğrusal çerçeve
// belleği. Renkler 0x00RRGGBB.

use core::sync::atomic::{AtomicU64, Ordering};
use spin::Mutex;
use x86_64::instructions::port::Port;

use crate::{pci, vga};

const GENISLIK: usize = 1024;
const YUKSEKLIK: usize = 768;

// Önyükleyicinin fiziksel belleği eşlediği sanal adres; main.rs bir kez ayarlar.
static FIZIKSEL_OFSET: AtomicU64 = AtomicU64::new(0);

// VGA metin yazı tipi: 256 karakter x 16 satır. Piksel moduna geçmeden önce kopyalanır.
static FONT: Mutex<[u8; 256 * 16]> = Mutex::new([0; 256 * 16]);

pub fn ofset_ayarla(ofset: u64) {
    FIZIKSEL_OFSET.store(ofset, Ordering::Relaxed);
}

#[derive(Clone, Copy)]
pub struct Ekran {
    adres: *mut u32,
    pub genislik: usize,
    pub yukseklik: usize,
    // Satır başına piksel sayısı
    pitch: usize,
}

fn bga_yaz(indeks: u16, deger: u16) {
    unsafe {
        Port::<u16>::new(0x1ce).write(indeks);
        Port::<u16>::new(0x1cf).write(deger);
    }
}

fn bga_oku(indeks: u16) -> u16 {
    unsafe {
        Port::<u16>::new(0x1ce).write(indeks);
        Port::<u16>::new(0x1cf).read()
    }
}

pub fn baslat() -> Result<Ekran, &'static str> {
    if !(0xb0c0..=0xb0c5).contains(&bga_oku(0)) {
        return Err("Piksel modu için Bochs/QEMU ekran kartı gerekli, bulunamadı.");
    }
    let bar0 = pci::ekran_karti_bar0().ok_or("Ekran kartı PCI üzerinde bulunamadı.")?;

    // Mod değişince yazı tipi belleği görünmez olur; önce kopyala.
    vga::font_kopyala(&mut FONT.lock());

    bga_yaz(4, 0); // ayar yapmadan önce kapat
    bga_yaz(1, GENISLIK as u16);
    bga_yaz(2, YUKSEKLIK as u16);
    bga_yaz(3, 32);
    bga_yaz(4, 0x41); // açık + doğrusal çerçeve belleği

    Ok(Ekran {
        adres: (FIZIKSEL_OFSET.load(Ordering::Relaxed) + bar0) as *mut u32,
        genislik: GENISLIK,
        yukseklik: YUKSEKLIK,
        pitch: GENISLIK,
    })
}

pub fn demo(ekran: &Ekran) {
    ekran.dikdortgen(0, 0, ekran.genislik, ekran.yukseklik, 0x0020_3040);
    ekran.dikdortgen(262, 234, 500, 300, 0x00e8_e8e8);
    ekran.dikdortgen(262, 234, 500, 24, 0x0030_60a0);
    ekran.metin(270, 238, "Veilis", 0x00ff_ffff, None);
    ekran.metin(278, 290, "Veilis arayüzü: ç ğ ı İ ö ş ü", 0x0010_1010, None);
}

impl Ekran {
    pub fn piksel(&self, x: usize, y: usize, renk: u32) {
        if x < self.genislik && y < self.yukseklik {
            unsafe { self.adres.add(y * self.pitch + x).write_volatile(renk) };
        }
    }

    pub fn dikdortgen(&self, x: usize, y: usize, g: usize, h: usize, renk: u32) {
        for yy in y..y.saturating_add(h).min(self.yukseklik) {
            for xx in x..x.saturating_add(g).min(self.genislik) {
                self.piksel(xx, yy, renk);
            }
        }
    }

    // Her karakter 8x16 hücre; arka plan verilmezse yalnızca harfin pikselleri çizilir.
    pub fn metin(&self, x: usize, y: usize, yazi: &str, on: u32, arka: Option<u32>) {
        let font = FONT.lock();
        for (sira, c) in yazi.chars().enumerate() {
            let resim = &font[vga::karakter_kodu(c) as usize * 16..][..16];
            for (satir, bayt) in resim.iter().enumerate() {
                for sutun in 0..8 {
                    let renk = if bayt & (0x80 >> sutun) != 0 { Some(on) } else { arka };
                    if let Some(renk) = renk {
                        self.piksel(x + sira * 8 + sutun, y + satir, renk);
                    }
                }
            }
        }
    }
}
