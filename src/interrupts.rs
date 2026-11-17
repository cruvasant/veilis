// İşlemci istisnaları (exception): sıfıra bölme, geçersiz bellek erişimi gibi
// durumlarda işlemci, IDT'de (Interrupt Descriptor Table) o hata için
// kayıtlı fonksiyonu çağırır.

use spin::Once;
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};

use crate::println;

static IDT: Once<InterruptDescriptorTable> = Once::new();

pub fn init() {
    IDT.call_once(|| {
        let mut idt = InterruptDescriptorTable::new();
        idt.breakpoint.set_handler_fn(breakpoint);
        idt.double_fault.set_handler_fn(double_fault);
        idt
    })
    .load();
}

// int3 komutuyla tetiklenir. Hata değil; iş bitince program kaldığı yerden devam eder.
extern "x86-interrupt" fn breakpoint(frame: InterruptStackFrame) {
    println!("ISTISNA: breakpoint\n{:#?}", frame);
}

// Bir istisna için fonksiyon yoksa ya da fonksiyonun kendisi hata verirse
// işlemci double fault üretir. Bunu da yakalamazsak bilgisayar yeniden başlar.
extern "x86-interrupt" fn double_fault(frame: InterruptStackFrame, _hata_kodu: u64) -> ! {
    panic!("ISTISNA: double fault\n{:#?}", frame);
}
