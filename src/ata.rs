// ATA disk sürücüsü (PIO modu). Disk denetleyicisiyle G/Ç portları üzerinden
// konuşuyoruz: "şu sektörü oku" komutunu yazıp veriyi 2'şer bayt alıyoruz.
// Sektör: diskin en küçük okunup yazılabilen parçası, 512 bayt.
// Yalnızca birincil kanaldaki ikinci diske (slave) erişiyoruz; ilk disk Veilis'in
// açıldığı disk, ona asla yazmıyoruz.

use x86_64::instructions::port::Port;

pub const SEKTOR: usize = 512;

const VERI: u16 = 0x1f0;
const SEKTOR_SAYISI: u16 = 0x1f2;
const LBA_DUSUK: u16 = 0x1f3;
const LBA_ORTA: u16 = 0x1f4;
const LBA_YUKSEK: u16 = 0x1f5;
const SURUCU: u16 = 0x1f6;
const DURUM_KOMUT: u16 = 0x1f7;
const DENETIM: u16 = 0x3f6;

const MESGUL: u8 = 0x80;
const ARIZA: u8 = 0x20;
const VERI_HAZIR: u8 = 0x08;
const HATA: u8 = 0x01;

// 0xF0: ikinci disk (slave), LBA adresleme.
const IKINCI_DISK: u8 = 0xf0;

const OKU: u8 = 0x20;
const YAZ: u8 = 0x30;
const ONBELLEGI_BOSALT: u8 = 0xe7;
const TANIT: u8 = 0xec;

// Disk takılı değilse sonsuza kadar beklemeyelim.
const EN_FAZLA_DENEME: u32 = 1_000_000;

fn oku8(port: u16) -> u8 {
    unsafe { Port::<u8>::new(port).read() }
}

fn yaz8(port: u16, deger: u8) {
    unsafe { Port::<u8>::new(port).write(deger) }
}

// Disk seçildikten sonra ~400 ns beklemek gerekir; durumu 4 kez okumak bu kadar sürer.
fn kisa_bekle() {
    for _ in 0..4 {
        oku8(DENETIM);
    }
}

fn mesgul_bitsin() -> Result<u8, &'static str> {
    for _ in 0..EN_FAZLA_DENEME {
        let durum = oku8(DURUM_KOMUT);
        if durum & MESGUL == 0 {
            return Ok(durum);
        }
    }
    Err("disk yanıt vermiyor")
}

fn veri_hazir_olsun() -> Result<(), &'static str> {
    for _ in 0..EN_FAZLA_DENEME {
        let durum = oku8(DURUM_KOMUT);
        if durum & MESGUL != 0 {
            continue;
        }
        if durum & (HATA | ARIZA) != 0 {
            return Err("disk hatası");
        }
        if durum & VERI_HAZIR != 0 {
            return Ok(());
        }
    }
    Err("disk yanıt vermiyor")
}

fn komut_gonder(lba: u32, komut: u8) -> Result<(), &'static str> {
    mesgul_bitsin()?;
    yaz8(SURUCU, IKINCI_DISK | ((lba >> 24) & 0x0f) as u8);
    kisa_bekle();
    yaz8(SEKTOR_SAYISI, 1);
    yaz8(LBA_DUSUK, lba as u8);
    yaz8(LBA_ORTA, (lba >> 8) as u8);
    yaz8(LBA_YUKSEK, (lba >> 16) as u8);
    yaz8(DURUM_KOMUT, komut);
    veri_hazir_olsun()
}

/// Diski tanır, sektör sayısını döndürür.
pub fn tanit() -> Result<u32, &'static str> {
    // Diskin kesme üretmesini kapat (nIEN); biz durumu kendimiz yokluyoruz.
    yaz8(DENETIM, 0x02);
    yaz8(SURUCU, IKINCI_DISK);
    kisa_bekle();
    for port in [SEKTOR_SAYISI, LBA_DUSUK, LBA_ORTA, LBA_YUKSEK] {
        yaz8(port, 0);
    }
    yaz8(DURUM_KOMUT, TANIT);
    if matches!(oku8(DURUM_KOMUT), 0 | 0xff) {
        return Err("veri diski takılı değil");
    }
    mesgul_bitsin()?;
    if oku8(LBA_ORTA) != 0 || oku8(LBA_YUKSEK) != 0 {
        return Err("ikinci sürücü bir ATA diski değil");
    }
    veri_hazir_olsun()?;
    let mut bilgi = [0u16; 256];
    let mut port = Port::<u16>::new(VERI);
    for kelime in bilgi.iter_mut() {
        *kelime = unsafe { port.read() };
    }
    // 60. ve 61. kelimeler: LBA ile adreslenebilen sektör sayısı.
    Ok(bilgi[60] as u32 | (bilgi[61] as u32) << 16)
}

pub fn sektor_oku(lba: u32, tampon: &mut [u8; SEKTOR]) -> Result<(), &'static str> {
    komut_gonder(lba, OKU)?;
    let mut port = Port::<u16>::new(VERI);
    for parca in tampon.chunks_exact_mut(2) {
        parca.copy_from_slice(&unsafe { port.read() }.to_le_bytes());
    }
    Ok(())
}

pub fn sektor_yaz(lba: u32, tampon: &[u8; SEKTOR]) -> Result<(), &'static str> {
    komut_gonder(lba, YAZ)?;
    let mut port = Port::<u16>::new(VERI);
    for parca in tampon.chunks_exact(2) {
        unsafe { port.write(u16::from_le_bytes([parca[0], parca[1]])) };
    }
    // Diskin kendi önbelleğindeki veriyi kalıcı olarak yazmasını iste.
    yaz8(DURUM_KOMUT, ONBELLEGI_BOSALT);
    mesgul_bitsin()?;
    Ok(())
}
