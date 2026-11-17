// Standart kütüphaneyi (std) kullanma: altımızda dosya, ekran, bellek
// yöneticisi sunan bir işletim sistemi yok. Yalnızca `core` var.
#![no_std]
// Normal `main` başlangıç noktasını kullanma: onu çağıracak bir çalışma
// zamanı (runtime) yok. Başlangıç noktasını kendimiz tanımlıyoruz.
#![no_main]

use core::panic::PanicInfo;

// VGA metin tamponu yalnızca ASCII ve birkaç sembol gösterir;
// ş, ğ, ı gibi harfler yok. Bu yüzden mesaj ASCII.
static MESAJ: &[u8] = b"Merhaba, ben Veilis";

// Önyükleyici çekirdeği belleğe yükleyince `_start` adlı fonksiyona atlar.
// no_mangle: derleyici adı değiştirmesin, tam olarak `_start` kalsın.
// `-> !`: bu fonksiyon asla geri dönmez (dönecek bir yer yok).
#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    // VGA metin tamponu 0xb8000 adresinde başlar. Her karakter 2 bayttır:
    // 1. bayt ASCII kodu, 2. bayt renk (0x0b = siyah üstüne açık camgöbeği).
    let vga = 0xb8000 as *mut u8;

    for (i, &bayt) in MESAJ.iter().enumerate() {
        // unsafe: ham bir adrese yazıyoruz; derleyici bunun güvenli
        // olduğunu kanıtlayamaz, sorumluluğu biz alıyoruz.
        unsafe {
            *vga.add(i * 2) = bayt;
            *vga.add(i * 2 + 1) = 0x0b;
        }
    }

    loop {}
}

// Bir panik (kurtarılamaz hata) olursa çağrılır. std olmadığı için
// bunu kendimiz yazmak zorundayız. Şimdilik sonsuza kadar bekliyoruz.
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
