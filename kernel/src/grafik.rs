// Önyükleyicinin verdiği ekran belleği (UEFI'de GOP, BIOS'ta VBE). Başlangıçta metin
// konsolu doğrudan buraya çizilir; masaüstü önce RAM'deki bir tuvale çizilip tek seferde
// kopyalanır. Renkler 0x00RRGGBB; satır uzunluğu (stride) ve piksel düzeni ekrana göre değişir.

use alloc::vec::Vec;
use bootloader_api::info::{FrameBuffer, FrameBufferInfo, PixelFormat};
use spin::Mutex;

use crate::vga::{self, GENISLIK as SUTUN, YUKSEKLIK as SATIR};
use crate::yazitipi::HARFLER;

// Konsolun piksel boyutu: 80x25 hücre, hücre 8x16.
const KONSOL_G: usize = SUTUN * 8;
const KONSOL_Y: usize = SATIR * 16;

// VGA'nın 16 rengi: 0 siyah, 1 mavi, 2 yeşil, 3 camgöbeği, 4 kırmızı, 5 mor, 6 kahverengi,
// 7 açık gri, sonra aynı sırayla parlak olanlar.
pub const VGA_RENKLERI: [u32; 16] = [
    0x0000_0000, 0x0000_00aa, 0x0000_aa00, 0x0000_aaaa, 0x00aa_0000, 0x00aa_00aa, 0x00aa_5500, 0x00aa_aaaa,
    0x0055_5555, 0x0055_55ff, 0x0055_ff55, 0x0055_ffff, 0x00ff_5555, 0x00ff_55ff, 0x00ff_ff55, 0x00ff_ffff,
];

static EKRAN: Mutex<Option<FrameBuffer>> = Mutex::new(None);

pub fn kaydet(ekran: FrameBuffer) {
    *EKRAN.lock() = Some(ekran);
}

/// (genişlik, yükseklik) piksel; ekran yoksa None.
pub fn boyut() -> Option<(usize, usize)> {
    EKRAN.lock().as_ref().map(|e| (e.info().width, e.info().height))
}

// Rengi ekranın bayt düzenine çevirir; ilk `bytes_per_pixel` bayt kullanılır.
fn renk_bayt(bilgi: &FrameBufferInfo, renk: u32) -> [u8; 4] {
    let (k, y, m) = ((renk >> 16) as u8, (renk >> 8) as u8, renk as u8);
    match bilgi.pixel_format {
        PixelFormat::Rgb => [k, y, m, 0],
        PixelFormat::Bgr => [m, y, k, 0],
        // Gri tonlamalı ekranda yaklaşık parlaklık yeter.
        PixelFormat::U8 => [((k as u16 + y as u16 + m as u16) / 3) as u8, 0, 0, 0],
        _ => [0; 4],
    }
}

// Bütün ekranı tek renge boyar.
pub fn doldur(renk: u32) {
    let mut kilit = EKRAN.lock();
    let Some(ekran) = kilit.as_mut() else { return };
    let bilgi = ekran.info();
    let (bayt, piksel) = (bilgi.bytes_per_pixel, renk_bayt(&bilgi, renk));
    let tampon = ekran.buffer_mut();
    for satir in 0..bilgi.height {
        let bas = satir * bilgi.stride * bayt;
        for hedef in tampon[bas..bas + bilgi.width * bayt].chunks_exact_mut(bayt) {
            hedef.copy_from_slice(&piksel[..bayt]);
        }
    }
}

// vga.rs'in 80x25 hücre tablosunu ekranın ortasına çizer. Kilit başkasındaysa (panikte
// olabilir) beklemeden vazgeçer.
pub fn konsol_ciz() {
    let mut hucreler = [0u16; SUTUN * SATIR];
    let (imlec_satir, imlec_sutun, imlec_gorunur) = vga::goruntu(&mut hucreler);
    let Some(mut kilit) = EKRAN.try_lock() else { return };
    let Some(ekran) = kilit.as_mut() else { return };
    let bilgi = ekran.info();
    let bayt = bilgi.bytes_per_pixel;
    let renkler = VGA_RENKLERI.map(|r| renk_bayt(&bilgi, r));
    let x0 = bilgi.width.saturating_sub(KONSOL_G) / 2;
    let y0 = bilgi.height.saturating_sub(KONSOL_Y) / 2;
    let tampon = ekran.buffer_mut();
    for (sira, hucre) in hucreler.iter().enumerate() {
        let (hx, hy) = (x0 + sira % SUTUN * 8, y0 + sira / SUTUN * 16);
        // Ekran konsoldan küçükse sığmayan hücreler atlanır.
        if hx + 8 > bilgi.width || hy + 16 > bilgi.height {
            continue;
        }
        let renk = (hucre >> 8) as usize;
        let (on, arka) = (renkler[renk & 0xf], renkler[renk >> 4]);
        for satir in 0..16 {
            let desen = HARFLER[(*hucre & 0xff) as usize][satir];
            let bas = ((hy + satir) * bilgi.stride + hx) * bayt;
            for (sutun, hedef) in tampon[bas..bas + 8 * bayt].chunks_exact_mut(bayt).enumerate() {
                let p = if desen & (0x80 >> sutun) != 0 { on } else { arka };
                hedef.copy_from_slice(&p[..bayt]);
            }
        }
        if imlec_gorunur && (sira / SUTUN, sira % SUTUN) == (imlec_satir, imlec_sutun) {
            for satir in 14..16 {
                let bas = ((hy + satir) * bilgi.stride + hx) * bayt;
                for hedef in tampon[bas..bas + 8 * bayt].chunks_exact_mut(bayt) {
                    hedef.copy_from_slice(&renkler[15][..bayt]);
                }
            }
        }
    }
}

// RAM'deki çizim yüzeyi; ekran dışına taşan kısımlar kırpılır.
pub struct Tuval {
    veri: Vec<u32>,
    pub g: i32,
    pub y: i32,
}

impl Tuval {
    // Bellek yetmezse panik yerine hata verir (4K ekranda 33 MB gerekir).
    pub fn yeni(g: usize, y: usize) -> Result<Self, &'static str> {
        let mut veri = Vec::new();
        veri.try_reserve_exact(g * y).map_err(|_| "Çizim için yeterli bellek yok.")?;
        veri.resize(g * y, 0);
        Ok(Self { veri, g: g as i32, y: y as i32 })
    }

    pub fn piksel(&mut self, x: i32, y: i32, renk: u32) {
        // Negatifler işaretsize çevrilince çok büyür; tek karşılaştırma iki sınırı da yakalar.
        if (x as u32) < self.g as u32 && (y as u32) < self.y as u32 {
            self.veri[(y * self.g + x) as usize] = renk;
        }
    }

    pub fn dikdortgen(&mut self, x: i32, y: i32, g: i32, h: i32, renk: u32) {
        let (x0, x1) = (x.max(0) as usize, (x + g).clamp(0, self.g) as usize);
        let (y0, y1) = (y.max(0) as usize, (y + h).clamp(0, self.y) as usize);
        // İlk satırı doldurup gerisini kopyalıyoruz; kopyalama bellek işlemi, hata ayıklama derlemesinde bile hızlı.
        if x0 < x1 && y0 < y1 {
            let satir = self.g as usize;
            let ilk = y0 * satir + x0;
            self.veri[ilk..ilk + x1 - x0].fill(renk);
            for yy in y0 + 1..y1 {
                self.veri.copy_within(ilk..ilk + x1 - x0, yy * satir + x0);
            }
        }
    }

    // Her karakter 8x16 hücre; arka plan verilmezse yalnızca harfin pikselleri çizilir.
    pub fn harf(&mut self, x: i32, y: i32, kod: u8, on: u32, arka: Option<u32>) {
        for satir in 0..16 {
            let desen = HARFLER[kod as usize][satir as usize];
            for sutun in 0..8 {
                if desen & (0x80 >> sutun) != 0 {
                    self.piksel(x + sutun, y + satir, on);
                } else if let Some(arka) = arka {
                    self.piksel(x + sutun, y + satir, arka);
                }
            }
        }
    }

    pub fn metin(&mut self, x: i32, y: i32, yazi: &str, on: u32, arka: Option<u32>) {
        for (sira, c) in yazi.chars().enumerate() {
            self.harf(x + sira as i32 * 8, y, vga::karakter_kodu(c), on, arka);
        }
    }

    // Ekranın sol üst köşesine kopyalar. 4 baytlık Bgr ekranda 0x00RRGGBB zaten ekranın
    // bayt düzeninde olduğundan satır satır doğrudan kopyalanır.
    pub fn ekrana_kopyala(&self) {
        let mut kilit = EKRAN.lock();
        let Some(ekran) = kilit.as_mut() else { return };
        let bilgi = ekran.info();
        let bayt = bilgi.bytes_per_pixel;
        let (g, y) = ((self.g as usize).min(bilgi.width), (self.y as usize).min(bilgi.height));
        let hizli = bilgi.pixel_format == PixelFormat::Bgr && bayt == 4;
        let tampon = ekran.buffer_mut();
        for satir in 0..y {
            let kaynak = &self.veri[satir * self.g as usize..][..g];
            let hedef = &mut tampon[satir * bilgi.stride * bayt..][..g * bayt];
            if hizli {
                hedef.copy_from_slice(unsafe { core::slice::from_raw_parts(kaynak.as_ptr() as *const u8, g * 4) });
            } else if bilgi.pixel_format == PixelFormat::Bgr && bayt == 3 {
                // Sabit uzunluklu parçalar, piksel başına bellek kopyalama çağrısından çok daha hızlı.
                for (p, &renk) in hedef.chunks_exact_mut(3).zip(kaynak) {
                    p.copy_from_slice(&renk.to_le_bytes()[..3]);
                }
            } else {
                let bgr =bilgi.pixel_format == PixelFormat::Bgr;
                for (p, &renk) in hedef.chunks_exact_mut(bayt).zip(kaynak) {
                    // 3 baytlık Bgr'de de bayt sırası küçük uçlu sayıyla aynı.
                    let b = if bgr { renk.to_le_bytes() } else { renk_bayt(&bilgi, renk) };
                    p.copy_from_slice(&b[..bayt]);
                }
            }
        }
    }
}
