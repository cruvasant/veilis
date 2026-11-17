// Masaüstü: pencereler, görev çubuğu ve fare. Kabuk bir terminal penceresinde
// çalışmaya devam eder; vga.rs'in RAM'deki 80x25 hücre tablosunu çizeriz.
// Çizim kabuk tuş beklerken (kabuk::sonraki_tus) guncelle() ile tetiklenir.

use alloc::format;
use alloc::vec;
use alloc::vec::Vec;
use spin::Mutex;

use crate::grafik::{self, Tuval, VGA_RENKLERI};
use crate::vga::{self, GENISLIK as SUTUN, YUKSEKLIK as SATIR};
use crate::{fare, interrupts};

const GOREV_CUBUGU: i32 = 32;
const BASLIK: i32 = 24;
// Pencere sürüklenirken ekranda en az bu kadarı görünür kalır.
const EN_AZ_GORUNEN: i32 = 40;

const ZEMIN: u32 = 0x0020_3040;
const CUBUK: u32 = 0x0014_1820;
const ODAKLI: u32 = 0x0030_60a0;
const ODAKSIZ: u32 = 0x0060_6870;
const KENAR: u32 = 0x0010_1418;
const BEYAZ: u32 = 0x00ff_ffff;
const KAPAT: u32 = 0x00c0_3030;
const KAGIT: u32 = 0x00e8_e8e8;
const MUREKKEP: u32 = 0x0010_1010;

// Fare oku: # çerçeve, . iç.
const OK: [&str; 16] = [
    "#           ",
    "##          ",
    "#.#         ",
    "#..#        ",
    "#...#       ",
    "#....#      ",
    "#.....#     ",
    "#......#    ",
    "#.......#   ",
    "#........#  ",
    "#.....#####.",
    "#..#..#     ",
    "#.# #..#    ",
    "##  #..#    ",
    "#    #..#   ",
    "     ###    ",
];

#[derive(PartialEq)]
enum Tur {
    Terminal,
    Hakkinda,
}

struct Pencere {
    tur: Tur,
    baslik: &'static str,
    // Başlık çubuğunun sol üst köşesi; g x h içerik alanının boyutu.
    x: i32,
    y: i32,
    g: i32,
    h: i32,
}

impl Pencere {
    fn icinde(&self, px: i32, py: i32) -> bool {
        (self.x..self.x + self.g).contains(&px) && (self.y..self.y + BASLIK + self.h).contains(&py)
    }

    // Yalnızca "Veilis hakkında" kapatılabilir.
    fn kapat_dugmesinde(&self, px: i32, py: i32) -> bool {
        self.tur == Tur::Hakkinda
            && (self.x + self.g - 22..self.x + self.g - 6).contains(&px)
            && (self.y + 4..self.y + 20).contains(&py)
    }
}

struct Masaustu {
    tuval: Tuval,
    // Arkadan öne doğru; sonuncusu odaktaki pencere.
    pencereler: Vec<Pencere>,
    // Sürüklenen pencerede farenin başlık köşesine uzaklığı.
    surukle: Option<(i32, i32)>,
    sol_basili: bool,
    fare_degisim: u32,
    saniye: u64,
}

// Kesme yöneticileri bu kilide hiç dokunmaz.
static MASAUSTU: Mutex<Option<Masaustu>> = Mutex::new(None);

pub fn acik() -> bool {
    MASAUSTU.lock().is_some()
}

pub fn ac() -> Result<(), &'static str> {
    let (g, y) = grafik::boyut().ok_or("Ekran bulunamadı.")?;
    if g < 800 || y < 600 {
        return Err("Masaüstü için en az 800x600 ekran gerekli.");
    }
    let tuval = Tuval::yeni(g, y)?;
    fare::sinir_ayarla(g as i32, y as i32);

    let mut masaustu = Masaustu {
        tuval,
        pencereler: vec![
            Pencere { tur: Tur::Hakkinda, baslik: "Veilis hakkında", x: g as i32 - 360, y: 160, g: 320, h: 96 },
            Pencere { tur: Tur::Terminal, baslik: "Terminal", x: 40, y: 40, g: (SUTUN * 8) as i32, h: (SATIR * 16) as i32 },
        ],
        surukle: None,
        sol_basili: false,
        fare_degisim: fare::degisim(),
        saniye: interrupts::acik_kalma_saniye(),
    };
    masaustu.ciz();
    *MASAUSTU.lock() = Some(masaustu);
    Ok(())
}

// Kesmeler açıkken çağrılır; bir şey değiştiyse yeniden çizer.
pub fn guncelle() {
    if let Some(m) = MASAUSTU.lock().as_mut() {
        m.guncelle();
    }
}

// Panikte son bir kez çizer ki hata mesajı terminal penceresinde görünsün. Panik kilit
// tutulurken olduysa beklemek kilitlenmek olurdu; o zaman vazgeçiyoruz.
pub fn panikte_ciz() -> bool {
    let Some(mut kilit) = MASAUSTU.try_lock() else { return false };
    let Some(m) = kilit.as_mut() else { return false };
    m.ciz();
    true
}

impl Masaustu {
    fn guncelle(&mut self) {
        let kirli = vga::kirli_al();
        let degisim = fare::degisim();
        let saniye = interrupts::acik_kalma_saniye();
        if !kirli && degisim == self.fare_degisim && saniye == self.saniye {
            return;
        }
        if degisim != self.fare_degisim {
            self.fare_isle();
        }
        self.fare_degisim = degisim;
        self.saniye = saniye;
        self.ciz();
    }

    fn fare_isle(&mut self) {
        let f = fare::durum();
        if f.sol && !self.sol_basili {
            self.tikla(f.x, f.y);
        } else if let (true, Some((dx, dy)), Some(p)) = (f.sol, self.surukle, self.pencereler.last_mut()) {
            p.x = (f.x - dx).clamp(EN_AZ_GORUNEN - p.g, self.tuval.g - EN_AZ_GORUNEN);
            p.y = (f.y - dy).clamp(0, self.tuval.y - GOREV_CUBUGU - BASLIK);
        }
        if !f.sol {
            self.surukle = None;
        }
        self.sol_basili = f.sol;
    }

    fn tikla(&mut self, x: i32, y: i32) {
        let Some(sira) = self.pencereler.iter().rposition(|p| p.icinde(x, y)) else {
            return;
        };
        let pencere = self.pencereler.remove(sira);
        // Kapatılan pencere burada düşer, yani silinir.
        if pencere.kapat_dugmesinde(x, y) {
            return;
        }
        if y < pencere.y + BASLIK {
            self.surukle = Some((x - pencere.x, y - pencere.y));
        }
        self.pencereler.push(pencere);
    }

    fn ciz(&mut self) {
        let tuval = &mut self.tuval;
        tuval.dikdortgen(0, 0, tuval.g, tuval.y, ZEMIN);
        let odak = self.pencereler.len() - 1;
        for (sira, pencere) in self.pencereler.iter().enumerate() {
            pencere_ciz(tuval, pencere, sira == odak);
        }
        gorev_cubugu_ciz(tuval, self.saniye);
        let f = fare::durum();
        for (satir, desen) in OK.iter().enumerate() {
            for (sutun, c) in desen.chars().enumerate() {
                match c {
                    '#' => tuval.piksel(f.x + sutun as i32, f.y + satir as i32, 0),
                    '.' => tuval.piksel(f.x + sutun as i32, f.y + satir as i32, BEYAZ),
                    _ => {}
                }
            }
        }
        tuval.ekrana_kopyala();
    }
}

fn pencere_ciz(tuval: &mut Tuval, p: &Pencere, odakli: bool) {
    tuval.dikdortgen(p.x - 1, p.y - 1, p.g + 2, BASLIK + p.h + 2, KENAR);
    tuval.dikdortgen(p.x, p.y, p.g, BASLIK, if odakli { ODAKLI } else { ODAKSIZ });
    tuval.metin(p.x + 8, p.y + 4, p.baslik, BEYAZ, None);
    if p.tur == Tur::Hakkinda {
        tuval.dikdortgen(p.x + p.g - 22, p.y + 4, 16, 16, KAPAT);
        tuval.metin(p.x + p.g - 18, p.y + 4, "X", BEYAZ, None);
    }

    let icerik_y = p.y + BASLIK;
    match p.tur {
        Tur::Terminal => konsol_ciz(tuval, p.x, icerik_y),
        Tur::Hakkinda => {
            tuval.dikdortgen(p.x, icerik_y, p.g, p.h, KAGIT);
            tuval.metin(p.x + 16, icerik_y + 16, &format!("Veilis {}", env!("CARGO_PKG_VERSION")), MUREKKEP, None);
            tuval.metin(p.x + 16, icerik_y + 48, "Fro dili ve tam ekran düzenleyici", MUREKKEP, None);
        }
    }
}

// VGA hücresi: alt bayt karakter, üst bayt renk (alt 4 bit yazı, üst 4 bit zemin).
fn konsol_ciz(tuval: &mut Tuval, x: i32, y: i32) {
    let mut hucreler = [0u16; SUTUN * SATIR];
    let (imlec_satir, imlec_sutun, imlec_gorunur) = vga::goruntu(&mut hucreler);
    for (sira, hucre) in hucreler.iter().enumerate() {
        let renk = (hucre >> 8) as usize;
        let (hx, hy) = (x + (sira % SUTUN) as i32 * 8, y + (sira / SUTUN) as i32 * 16);
        tuval.harf(hx, hy, *hucre as u8, VGA_RENKLERI[renk & 0xf], Some(VGA_RENKLERI[renk >> 4]));
    }
    if imlec_gorunur {
        let (hx, hy) = (x + imlec_sutun as i32 * 8, y + imlec_satir as i32 * 16);
        tuval.dikdortgen(hx, hy + 14, 8, 2, BEYAZ);
    }
}

fn gorev_cubugu_ciz(tuval: &mut Tuval, saniye: u64) {
    let y = tuval.y - GOREV_CUBUGU;
    tuval.dikdortgen(0, y, tuval.g, GOREV_CUBUGU, CUBUK);
    tuval.dikdortgen(8, y + 4, 72, 24, ODAKLI);
    tuval.metin(16, y + 8, "Veilis", BEYAZ, None);
    tuval.metin(tuval.g - 52, y + 8, &format!("{:02}:{:02}", saniye / 60, saniye % 60), BEYAZ, None);
}
