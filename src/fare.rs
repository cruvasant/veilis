// PS/2 fare sürücüsü. Fare, klavyeyle aynı 8042 denetleyicisine bağlı: komutlar
// 0x64'e, veri 0x60'a gider. Fare her hareketinde 3 baytlık paket yollar:
// [düğmeler ve işaretler, X farkı, Y farkı].

use core::sync::atomic::{AtomicI32, AtomicU32, AtomicU8, Ordering};
use x86_64::instructions::port::Port;

const VERI: u16 = 0x60;
const DURUM_KOMUT: u16 = 0x64;

const CIKIS_DOLU: u8 = 0x01;
const GIRIS_DOLU: u8 = 0x02;

const FARE_PORTUNU_AC: u8 = 0xa8;
const AYAR_OKU: u8 = 0x20;
const AYAR_YAZ: u8 = 0x60;
const FAREYE_YAZ: u8 = 0xd4;

const IRQ12_ACIK: u8 = 1 << 1;
const FARE_SAATI_KAPALI: u8 = 1 << 5;

const VARSAYILAN: u8 = 0xf6;
const RAPORLAMAYI_AC: u8 = 0xf4;
const ONAY: u8 = 0xfa;

// Fare takılı değilse sonsuza kadar beklemeyelim.
const EN_FAZLA_DENEME: u32 = 100_000;

pub struct FareDurumu {
    pub x: i32,
    pub y: i32,
    pub sol: bool,
    pub sag: bool,
    pub orta: bool,
}

// Kesme yöneticisi bunlara yalnızca atomik erişir; kilit alırsa ana döngü
// aynı kilidi tutarken kilitlenirdik.
static X: AtomicI32 = AtomicI32::new(0);
static Y: AtomicI32 = AtomicI32::new(0);
static GENISLIK: AtomicI32 = AtomicI32::new(1024);
static YUKSEKLIK: AtomicI32 = AtomicI32::new(768);
static DUGMELER: AtomicU8 = AtomicU8::new(0);
static DEGISIM: AtomicU32 = AtomicU32::new(0);

// Yarım kalan paketin baytları.
static PAKET: [AtomicU8; 2] = [AtomicU8::new(0), AtomicU8::new(0)];
static SIRA: AtomicU8 = AtomicU8::new(0);

fn oku() -> u8 {
    unsafe { Port::<u8>::new(VERI).read() }
}

fn denetleyici_durumu() -> u8 {
    unsafe { Port::<u8>::new(DURUM_KOMUT).read() }
}

fn yazilabilir() -> bool {
    (0..EN_FAZLA_DENEME).any(|_| denetleyici_durumu() & GIRIS_DOLU == 0)
}

fn okunabilir() -> bool {
    (0..EN_FAZLA_DENEME).any(|_| denetleyici_durumu() & CIKIS_DOLU != 0)
}

fn komut(kod: u8) -> bool {
    if !yazilabilir() {
        return false;
    }
    unsafe { Port::<u8>::new(DURUM_KOMUT).write(kod) };
    true
}

fn veri_yaz(deger: u8) -> bool {
    if !yazilabilir() {
        return false;
    }
    unsafe { Port::<u8>::new(VERI).write(deger) };
    true
}

fn fareye_gonder(deger: u8) -> bool {
    komut(FAREYE_YAZ) && veri_yaz(deger) && okunabilir() && oku() == ONAY
}

// Kesmeler kapalıyken çağrılmalı; yoksa klavye yöneticisi onay baytlarını yutar.
// Başarısız olursa fare sessiz kalır, klavye etkilenmez.
pub fn baslat() -> bool {
    // Eski artık baytlar onay okumasını şaşırtmasın.
    while denetleyici_durumu() & CIKIS_DOLU != 0 {
        oku();
    }
    if !komut(FARE_PORTUNU_AC) || !komut(AYAR_OKU) || !okunabilir() {
        return false;
    }
    let ayar = (oku() | IRQ12_ACIK) & !FARE_SAATI_KAPALI;
    if !komut(AYAR_YAZ) || !veri_yaz(ayar) {
        return false;
    }
    fareye_gonder(VARSAYILAN) && fareye_gonder(RAPORLAMAYI_AC)
}

// IRQ12 yöneticisi her gelen baytı buraya verir.
pub fn bayt_isle(b: u8) {
    let sira = SIRA.load(Ordering::Relaxed) as usize;
    // İlk baytın 3. biti hep 1'dir; değilse paket kaymıştır, o baytı atıp bekleriz.
    if sira == 0 && b & 0x08 == 0 {
        return;
    }
    if sira < 2 {
        PAKET[sira].store(b, Ordering::Relaxed);
        SIRA.store(sira as u8 + 1, Ordering::Relaxed);
        return;
    }
    SIRA.store(0, Ordering::Relaxed);

    let ilk = PAKET[0].load(Ordering::Relaxed);
    // Taşma bitleri (6 ve 7) varsa fark güvenilmez.
    if ilk & 0xc0 != 0 {
        return;
    }
    // Fark 9 bit: 8 bit değer + işaret biti.
    let dx = PAKET[1].load(Ordering::Relaxed) as i32 - if ilk & 0x10 != 0 { 256 } else { 0 };
    let dy = b as i32 - if ilk & 0x20 != 0 { 256 } else { 0 };
    // Fare Y'si yukarı doğru artar, ekranınki aşağı doğru.
    let x = (X.load(Ordering::Relaxed) + dx).clamp(0, GENISLIK.load(Ordering::Relaxed) - 1);
    let y = (Y.load(Ordering::Relaxed) - dy).clamp(0, YUKSEKLIK.load(Ordering::Relaxed) - 1);
    X.store(x, Ordering::Relaxed);
    Y.store(y, Ordering::Relaxed);
    DUGMELER.store(ilk & 0x07, Ordering::Relaxed);
    DEGISIM.fetch_add(1, Ordering::Relaxed);
}

pub fn durum() -> FareDurumu {
    let d = DUGMELER.load(Ordering::Relaxed);
    FareDurumu {
        x: X.load(Ordering::Relaxed),
        y: Y.load(Ordering::Relaxed),
        sol: d & 1 != 0,
        sag: d & 2 != 0,
        orta: d & 4 != 0,
    }
}

// Her paketten sonra artar; GUI bununla yeniden çizmesi gerekip gerekmediğini anlar.
pub fn degisim() -> u32 {
    DEGISIM.load(Ordering::Relaxed)
}

pub fn sinir_ayarla(genislik: i32, yukseklik: i32) {
    let (g, y) = (genislik.max(1), yukseklik.max(1));
    GENISLIK.store(g, Ordering::Relaxed);
    YUKSEKLIK.store(y, Ordering::Relaxed);
    X.store(X.load(Ordering::Relaxed).clamp(0, g - 1), Ordering::Relaxed);
    Y.store(Y.load(Ordering::Relaxed).clamp(0, y - 1), Ordering::Relaxed);
}
