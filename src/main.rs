// Standart kütüphaneyi (std) kullanma: altımızda dosya, ekran, bellek
// yöneticisi sunan bir işletim sistemi yok. Yalnızca `core` var.
#![no_std]
// Normal `main` başlangıç noktasını kullanma: onu çağıracak bir çalışma
// zamanı (runtime) yok. Başlangıç noktasını kendimiz tanımlıyoruz.
#![no_main]
// İstisna fonksiyonları için özel çağrı biçimi (henüz yalnızca nightly'de).
#![feature(abi_x86_interrupt)]

// Heap'ten bellek isteyen türler (String, Vec, Box) için.
extern crate alloc;

mod allocator;
mod ata;
mod dosya;
mod duzenleyici;
mod fro;
mod gdt;
mod interrupts;
mod kabuk;
mod klavye;
mod memory;
mod vga;

use bootloader::{BootInfo, entry_point};
use core::panic::PanicInfo;
use x86_64::VirtAddr;

// Önyükleyici `_start`'a atlar; bu makro `_start`'ı bizim için yazar ve
// fonksiyonumuza bellek haritasını (BootInfo) doğru türle verir.
// `-> !`: bu fonksiyon asla geri dönmez (dönecek bir yer yok).
entry_point!(cekirdek_main);

fn cekirdek_main(boot_info: &'static BootInfo) -> ! {
    vga::turkce_harfleri_yukle();
    println!("Merhaba, ben Veilis");
    println!("Sürüm {}", env!("CARGO_PKG_VERSION"));

    gdt::init();
    interrupts::init();

    let fiziksel_ofset = VirtAddr::new(boot_info.physical_memory_offset);
    let mut sayfalar = unsafe { memory::init(fiziksel_ofset) };
    let mut cerceveler = unsafe { memory::CerceveDagitici::new(&boot_info.memory_map) };
    allocator::init(&mut sayfalar, &mut cerceveler).expect("heap kurulamadı");

    #[cfg(debug_assertions)]
    fro::oz_denetim();

    match dosya::hazirla() {
        Ok(()) => println!("Veri diski hazır."),
        Err(hata) => println!("Veri diski kullanılamıyor: {}", hata),
    }
    println!("Komutlar için 'yardım' yaz.\n");

    kabuk::calistir(memory::kullanilabilir_bayt(&boot_info.memory_map))
}

// Boş döngüde işlemciyi yormak yerine hlt ile bir sonraki kesmeye kadar uyutuyoruz.
fn bekle() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}

// Bir panik (kurtarılamaz hata) olursa çağrılır. std olmadığı için
// bunu kendimiz yazmak zorundayız. Hatayı ekrana basıp bekliyoruz.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    bekle()
}
