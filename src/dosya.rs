// VeilisFS: olabildiğince basit bir dosya sistemi.
// Disk düzeni (her sektör 512 bayt):
//   sektör 0      : başlık, "VEILISFS" imzası
//   sektör 1-4    : dizin, 32 kayıt x 64 bayt
//   sektör 8-519  : veri; i. kaydın dosyası 8 + i*16. sektörden başlar, en fazla 16 sektör (8 KiB)
// Kayıt: [0] dolu mu, [1] ad uzunluğu, [2..48] ad (UTF-8), [48..52] boyut (u32).

use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, Ordering};

use crate::ata::{self, SEKTOR};

const IMZA: &[u8; 8] = b"VEILISFS";
const DIZIN_BASI: u32 = 1;
const DIZIN_SEKTORU: usize = 4;
const KAYIT: usize = 64;
const KAYIT_SAYISI: usize = DIZIN_SEKTORU * SEKTOR / KAYIT;
const EN_UZUN_AD: usize = 46;
const VERI_BASI: u32 = 8;
const DOSYA_SEKTORU: usize = 16;
pub const EN_BUYUK_DOSYA: usize = DOSYA_SEKTORU * SEKTOR;
const GEREKEN_SEKTOR: u32 = VERI_BASI + (KAYIT_SAYISI * DOSYA_SEKTORU) as u32;

static HAZIR: AtomicBool = AtomicBool::new(false);

type Dizin = [u8; DIZIN_SEKTORU * SEKTOR];

/// Açılışta bir kez çağrılır. Boş bir diski biçimlendirir; tanımadığı diske dokunmaz.
pub fn hazirla() -> Result<(), &'static str> {
    if ata::tanit()? < GEREKEN_SEKTOR {
        return Err("veri diski çok küçük (en az 260 KiB)");
    }
    let mut baslik = [0u8; SEKTOR];
    ata::sektor_oku(0, &mut baslik)?;
    if &baslik[..8] != IMZA {
        // Başka bir şey yazılı bir diski silmek veri kaybı olur.
        if baslik.iter().any(|&b| b != 0) {
            return Err("tanınmayan disk, güvenlik için dokunulmadı");
        }
        let bos = [0u8; SEKTOR];
        for i in 0..DIZIN_SEKTORU as u32 {
            ata::sektor_yaz(DIZIN_BASI + i, &bos)?;
        }
        baslik[..8].copy_from_slice(IMZA);
        ata::sektor_yaz(0, &baslik)?;
    }
    HAZIR.store(true, Ordering::Relaxed);
    Ok(())
}

fn dizin_oku() -> Result<Dizin, &'static str> {
    if !HAZIR.load(Ordering::Relaxed) {
        return Err("veri diski yok");
    }
    let mut dizin = [0u8; DIZIN_SEKTORU * SEKTOR];
    for (i, parca) in dizin.chunks_exact_mut(SEKTOR).enumerate() {
        ata::sektor_oku(DIZIN_BASI + i as u32, parca.try_into().unwrap())?;
    }
    Ok(dizin)
}

fn dizin_yaz(dizin: &Dizin) -> Result<(), &'static str> {
    for (i, parca) in dizin.chunks_exact(SEKTOR).enumerate() {
        ata::sektor_yaz(DIZIN_BASI + i as u32, parca.try_into().unwrap())?;
    }
    Ok(())
}

// i. kayıt doluysa (ad, boyut)
fn kayit(dizin: &Dizin, i: usize) -> Option<(&str, usize)> {
    let k = &dizin[i * KAYIT..(i + 1) * KAYIT];
    if k[0] != 1 {
        return None;
    }
    let ad = core::str::from_utf8(&k[2..2 + k[1] as usize]).ok()?;
    Some((ad, u32::from_le_bytes([k[48], k[49], k[50], k[51]]) as usize))
}

fn bul(dizin: &Dizin, ad: &str) -> Option<usize> {
    (0..KAYIT_SAYISI).find(|&i| kayit(dizin, i).is_some_and(|(a, _)| a == ad))
}

fn ilk_sektor(kayit_no: usize) -> u32 {
    VERI_BASI + (kayit_no * DOSYA_SEKTORU) as u32
}

pub fn listele() -> Result<Vec<(String, usize)>, &'static str> {
    let dizin = dizin_oku()?;
    Ok((0..KAYIT_SAYISI)
        .filter_map(|i| kayit(&dizin, i))
        .map(|(ad, boyut)| (String::from(ad), boyut))
        .collect())
}

pub fn oku(ad: &str) -> Result<Vec<u8>, &'static str> {
    let dizin = dizin_oku()?;
    let no = bul(&dizin, ad).ok_or("böyle bir dosya yok")?;
    let (_, boyut) = kayit(&dizin, no).unwrap();
    let mut veri = Vec::with_capacity(boyut.div_ceil(SEKTOR) * SEKTOR);
    let mut tampon = [0u8; SEKTOR];
    for i in 0..boyut.div_ceil(SEKTOR) {
        ata::sektor_oku(ilk_sektor(no) + i as u32, &mut tampon)?;
        veri.extend_from_slice(&tampon);
    }
    veri.truncate(boyut);
    Ok(veri)
}

pub fn ad_kontrol(ad: &str) -> Result<(), &'static str> {
    if ad.is_empty() || ad.len() > EN_UZUN_AD || ad.contains(char::is_whitespace) {
        return Err("dosya adı boş olamaz, boşluk içeremez, en fazla 46 bayt olabilir");
    }
    Ok(())
}

/// Dosyayı oluşturur; aynı adda dosya varsa üzerine yazar.
pub fn yaz(ad: &str, veri: &[u8]) -> Result<(), &'static str> {
    ad_kontrol(ad)?;
    if veri.len() > EN_BUYUK_DOSYA {
        return Err("dosya çok büyük (en fazla 8 KiB)");
    }
    let mut dizin = dizin_oku()?;
    let no = bul(&dizin, ad)
        .or_else(|| (0..KAYIT_SAYISI).find(|&i| kayit(&dizin, i).is_none()))
        .ok_or("disk dolu (en fazla 32 dosya)")?;

    // Önce veri, sonra dizin: yarıda kesilirse eski dosya kaydı bozulmadan kalır.
    for (i, parca) in veri.chunks(SEKTOR).enumerate() {
        let mut tampon = [0u8; SEKTOR];
        tampon[..parca.len()].copy_from_slice(parca);
        ata::sektor_yaz(ilk_sektor(no) + i as u32, &tampon)?;
    }
    let k = &mut dizin[no * KAYIT..(no + 1) * KAYIT];
    k.fill(0);
    k[0] = 1;
    k[1] = ad.len() as u8;
    k[2..2 + ad.len()].copy_from_slice(ad.as_bytes());
    k[48..52].copy_from_slice(&(veri.len() as u32).to_le_bytes());
    dizin_yaz(&dizin)
}

pub fn sil(ad: &str) -> Result<(), &'static str> {
    let mut dizin = dizin_oku()?;
    let no = bul(&dizin, ad).ok_or("böyle bir dosya yok")?;
    dizin[no * KAYIT] = 0;
    dizin_yaz(&dizin)
}
