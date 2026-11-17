// Piksel modu: QEMU'nun Bochs/BGA ekran kartıyla 1024x768, 32 bit doğrusal çerçeve
// belleği. Renkler 0x00RRGGBB. Çizimler önce RAM'deki bir tuvale yapılır, Ekran
// yalnızca bitmiş tuvali çerçeve belleğine kopyalar.

use core::sync::atomic::{AtomicU64, Ordering};
use spin::Mutex;
use x86_64::instructions::port::Port;

use crate::{pci, vga};

pub const GENISLIK: usize = 1024;
pub const YUKSEKLIK: usize = 768;

// Önyükleyicinin fiziksel belleği eşlediği sanal adres; main.rs bir kez ayarlar.
static FIZIKSEL_OFSET: AtomicU64 = AtomicU64::new(0);

// VGA metin yazı tipi: 256 karakter x 16 satır. Piksel moduna geçmeden önce kopyalanır.
static FONT: Mutex<[u8; 256 * 16]> = Mutex::new([0; 256 * 16]);

pub fn ofset_ayarla(ofset: u64) {
    FIZIKSEL_OFSET.store(ofset, Ordering::Relaxed);
}

// Çerçeve belleğinin başı; satır uzunluğu GENISLIK piksel.
pub struct Ekran {
    adres: *mut u32,
}

// Ekran yalnızca masaüstü kilidi altında kullanılır.
unsafe impl Send for Ekran {}

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

    Ok(Ekran { adres: (FIZIKSEL_OFSET.load(Ordering::Relaxed) + bar0) as *mut u32 })
}

impl Ekran {
    // Tuvali tek seferde kopyalıyoruz; piksel piksel volatile yazmaktan çok daha hızlı.
    pub fn kopyala(&self, tuval: &[u32]) {
        assert_eq!(tuval.len(), GENISLIK * YUKSEKLIK);
        unsafe { core::ptr::copy_nonoverlapping(tuval.as_ptr(), self.adres, tuval.len()) };
    }
}

// Aşağıdakiler GENISLIK x YUKSEKLIK piksellik bir tuvale çizer; ekran dışına
// taşan kısımlar kırpılır.

pub fn piksel(tuval: &mut [u32], x: i32, y: i32, renk: u32) {
    // Negatifler işaretsize çevrilince çok büyür; tek karşılaştırma iki sınırı da yakalar.
    if (x as u32) < GENISLIK as u32 && (y as u32) < YUKSEKLIK as u32 {
        tuval[y as usize * GENISLIK + x as usize] = renk;
    }
}

pub fn dikdortgen(tuval: &mut [u32], x: i32, y: i32, g: i32, h: i32, renk: u32) {
    let (x0, x1) = (x.max(0) as usize, (x + g).clamp(0, GENISLIK as i32) as usize);
    let (y0, y1) = (y.max(0) as usize, (y + h).clamp(0, YUKSEKLIK as i32) as usize);
    // İlk satırı doldurup gerisini kopyalıyoruz; kopyalama bellek işlemi, hata ayıklama derlemesinde bile hızlı.
    if x0 < x1 && y0 < y1 {
        let ilk = y0 * GENISLIK + x0;
        tuval[ilk..ilk + x1 - x0].fill(renk);
        for yy in y0 + 1..y1 {
            tuval.copy_within(ilk..ilk + x1 - x0, yy * GENISLIK + x0);
        }
    }
}

// Her karakter 8x16 hücre; arka plan verilmezse yalnızca harfin pikselleri çizilir.
pub fn harf(tuval: &mut [u32], x: i32, y: i32, kod: u8, on: u32, arka: Option<u32>) {
    let font = FONT.lock();
    // Döngüler elle yazıldı: hata ayıklama derlemesinde yineleyiciler çok yavaş.
    for satir in 0..16 {
        let bayt = font[kod as usize * 16 + satir];
        for sutun in 0..8 {
            if bayt & (0x80 >> sutun) != 0 {
                piksel(tuval, x + sutun, y + satir as i32, on);
            } else if let Some(arka) = arka {
                piksel(tuval, x + sutun, y + satir as i32, arka);
            }
        }
    }
}

pub fn metin(tuval: &mut [u32], x: i32, y: i32, yazi: &str, on: u32, arka: Option<u32>) {
    for (sira, c) in yazi.chars().enumerate() {
        harf(tuval, x + sira as i32 * 8, y, vga::karakter_kodu(c), on, arka);
    }
}
