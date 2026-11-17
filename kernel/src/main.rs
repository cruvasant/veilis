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
mod fare;
mod fro;
mod gdt;
mod grafik;
mod interrupts;
mod kabuk;
mod klavye;
mod masaustu;
mod memory;
mod seri;
mod vga;
mod yazitipi;

use bootloader_api::config::Mapping;
use bootloader_api::{BootInfo, BootloaderConfig, entry_point};
use core::panic::PanicInfo;
use x86_64::VirtAddr;

const BOOTLOADER_AYARI: BootloaderConfig = {
    let mut ayar = BootloaderConfig::new_default();
    // Sayfa tablolarını okuyup değiştirebilmek için bütün fiziksel belleği eşle.
    ayar.mappings.physical_memory = Some(Mapping::Dynamic);
    // Fro'nun iç içe çağrıları derleme türüne göre bol stack isteyebilir.
    ayar.kernel_stack_size = 1024 * 1024;
    ayar
};

// Önyükleyici `_start`'a atlar; bu makro `_start`'ı bizim için yazar ve
// fonksiyonumuza bellek haritasını (BootInfo) doğru türle verir.
// `-> !`: bu fonksiyon asla geri dönmez (dönecek bir yer yok).
entry_point!(cekirdek_main, config = &BOOTLOADER_AYARI);

fn cekirdek_main(boot_info: &'static mut BootInfo) -> ! {
    seri::baslat();
    if let Some(ekran) = boot_info.framebuffer.take() {
        let b = ekran.info();
        println!("Ekran: {}x{}, stride {}, {} bayt/piksel, {:?}", b.width, b.height, b.stride, b.bytes_per_pixel, b.pixel_format);
        grafik::kaydet(ekran);
        grafik::doldur(0x0020_3040);
    }
    println!("Merhaba, ben Veilis");
    println!("Sürüm {}", env!("CARGO_PKG_VERSION"));

    gdt::init();
    interrupts::init();

    let fiziksel_ofset = boot_info.physical_memory_offset.into_option().expect("fiziksel bellek eşlenmedi");
    let mut sayfalar = unsafe { memory::init(VirtAddr::new(fiziksel_ofset)) };
    let bolgeler = &*boot_info.memory_regions;
    let mut cerceveler = unsafe { memory::CerceveDagitici::new(bolgeler) };
    allocator::init(&mut sayfalar, &mut cerceveler).expect("heap kurulamadı");

    #[cfg(debug_assertions)]
    fro::oz_denetim();

    match dosya::hazirla() {
        Ok(()) => println!("Veri diski hazır."),
        Err(hata) => println!("Veri diski kullanılamıyor: {}", hata),
    }
    println!("Komutlar için 'yardım' yaz.\n");

    grafik::konsol_ciz();
    kabuk::calistir(memory::kullanilabilir_bayt(bolgeler))
}

// Boş döngüde işlemciyi yormak yerine hlt ile bir sonraki kesmeye kadar uyutuyoruz.
fn bekle() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}

// Bir panik (kurtarılamaz hata) olursa çağrılır. std olmadığı için
// bunu kendimiz yazmak zorundayız. Hatayı yazıp bekliyoruz.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    if !masaustu::panikte_ciz() {
        grafik::konsol_ciz();
    }
    bekle()
}
