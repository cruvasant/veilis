// Önyükleyicinin verdiği ekran belleği (UEFI'de GOP, BIOS'ta VBE). Metin ve masaüstü
// çizimi sonraki adımda buradan yapılacak.

use bootloader_api::info::{FrameBuffer, PixelFormat};
use spin::Mutex;

static EKRAN: Mutex<Option<FrameBuffer>> = Mutex::new(None);

pub fn kaydet(ekran: FrameBuffer) {
    *EKRAN.lock() = Some(ekran);
}

// Bütün ekranı tek renge (0x00RRGGBB) boyar; satır uzunluğu (stride) genişlikten büyük olabilir.
pub fn doldur(renk: u32) {
    let mut kilit = EKRAN.lock();
    let Some(ekran) = kilit.as_mut() else { return };
    let bilgi = ekran.info();
    let (k, y, m) = ((renk >> 16) as u8, (renk >> 8) as u8, renk as u8);
    let piksel = match bilgi.pixel_format {
        PixelFormat::Rgb => [k, y, m, 0],
        PixelFormat::Bgr => [m, y, k, 0],
        // Gri tonlamalı ekranda yaklaşık parlaklık yeter.
        PixelFormat::U8 => [((k as u16 + y as u16 + m as u16) / 3) as u8, 0, 0, 0],
        _ => return,
    };
    let bayt = bilgi.bytes_per_pixel;
    let tampon = ekran.buffer_mut();
    for satir in 0..bilgi.height {
        for sutun in 0..bilgi.width {
            let yer = (satir * bilgi.stride + sutun) * bayt;
            tampon[yer..yer + bayt].copy_from_slice(&piksel[..bayt]);
        }
    }
}
