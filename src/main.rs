// Standart kütüphaneyi (std) kullanma: altımızda dosya, ekran, bellek
// yöneticisi sunan bir işletim sistemi yok. Yalnızca `core` var.
#![no_std]
// Normal `main` başlangıç noktasını kullanma: onu çağıracak bir çalışma
// zamanı (runtime) yok. Başlangıç noktasını kendimiz tanımlıyoruz.
#![no_main]
// İstisna fonksiyonları için özel çağrı biçimi (henüz yalnızca nightly'de).
#![feature(abi_x86_interrupt)]

mod gdt;
mod interrupts;
mod vga;

use core::panic::PanicInfo;

// Önyükleyici çekirdeği belleğe yükleyince `_start` adlı fonksiyona atlar.
// no_mangle: derleyici adı değiştirmesin, tam olarak `_start` kalsın.
// `-> !`: bu fonksiyon asla geri dönmez (dönecek bir yer yok).
#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    println!("Merhaba, ben Veilis");
    println!("Surum {}", env!("CARGO_PKG_VERSION"));

    gdt::init();
    interrupts::init();

    // Deneme: breakpoint istisnası tetikle, sonra devam ettiğimizi göster.
    x86_64::instructions::interrupts::int3();
    println!("Breakpoint sonrasi devam ediyoruz");

    loop {}
}

// Bir panik (kurtarılamaz hata) olursa çağrılır. std olmadığı için
// bunu kendimiz yazmak zorundayız. Hatayı ekrana basıp bekliyoruz.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    loop {}
}
