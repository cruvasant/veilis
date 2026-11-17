// İşlemci istisnaları (exception): sıfıra bölme, geçersiz bellek erişimi gibi
// durumlarda işlemci, IDT'de (Interrupt Descriptor Table) o hata için
// kayıtlı fonksiyonu çağırır.

use pc_keyboard::{DecodedKey, HandleControl, PS2Keyboard, ScancodeSet1};
use pic8259::ChainedPics;
use spin::{Mutex, Once};
use x86_64::instructions::port::Port;
use x86_64::registers::control::Cr2;
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode};

use core::sync::atomic::{AtomicU64, Ordering};

use crate::klavye::TurkceQ;
use crate::println;

// Donanım kesmeleri (zamanlayıcı, klavye...) PIC denen yonga üzerinden gelir.
// İlk 32 numara işlemci istisnalarına ayrıldığı için PIC'i 32'den başlatıyoruz.
const PIC_1: u8 = 32;
const PIC_2: u8 = PIC_1 + 8;
const ZAMANLAYICI: u8 = PIC_1;
const KLAVYE: u8 = PIC_1 + 1;

static PICS: Mutex<ChainedPics> = Mutex::new(unsafe { ChainedPics::new(PIC_1, PIC_2) });

static KLAVYE_DURUMU: Mutex<PS2Keyboard<TurkceQ, ScancodeSet1>> = Mutex::new(
    // MapLettersToUnicode: Ctrl+C gibi tuşlar kontrol karakterine dönüşür ('\u{3}').
    PS2Keyboard::new(ScancodeSet1::new(), TurkceQ, HandleControl::MapLettersToUnicode),
);

static IDT: Once<InterruptDescriptorTable> = Once::new();

pub fn init() {
    IDT.call_once(|| {
        let mut idt = InterruptDescriptorTable::new();
        idt.breakpoint.set_handler_fn(breakpoint);
        // Stack taşınca da çalışabilsin diye double fault yedek stack'te çalışır.
        unsafe {
            idt.double_fault
                .set_handler_fn(double_fault)
                .set_stack_index(crate::gdt::DOUBLE_FAULT_IST);
        }
        idt.page_fault.set_handler_fn(page_fault);
        idt[ZAMANLAYICI].set_handler_fn(zamanlayici);
        idt[KLAVYE].set_handler_fn(klavye);
        idt
    })
    .load();

    unsafe {
        let mut pics = PICS.lock();
        pics.initialize();
        // Yalnızca zamanlayıcı (IRQ0) ve klavye (IRQ1) açık; fonksiyonu olmayan bir
        // donanım kesmesi gelirse çekirdek çökerdi.
        pics.write_masks(0b1111_1100, 0b1111_1111);
    }
    x86_64::instructions::interrupts::enable();
}

// Sayfa tablosunda karşılığı olmayan (ya da izin verilmeyen) bir adrese erişince gelir.
// İşlemci, sorunlu adresi CR2 yazmacına koyar.
extern "x86-interrupt" fn page_fault(frame: InterruptStackFrame, hata: PageFaultErrorCode) {
    panic!(
        "ISTISNA: page fault\nAdres: {:?}\nSebep: {:?}\n{:#?}",
        Cr2::read(),
        hata,
        frame
    );
}

static TIK: AtomicU64 = AtomicU64::new(0);

// Zamanlayıcı yongası (PIT) saniyede 1193182 / 65536 ≈ 18.2 kez çalar.
pub fn acik_kalma_saniye() -> u64 {
    TIK.load(Ordering::Relaxed) * 65536 / 1_193_182
}

extern "x86-interrupt" fn zamanlayici(_frame: InterruptStackFrame) {
    TIK.fetch_add(1, Ordering::Relaxed);
    unsafe { PICS.lock().notify_end_of_interrupt(ZAMANLAYICI) };
}

// Her tuşa basıp bırakınca gelir. Tuşun kodunu (scancode) 0x60 portundan okuyoruz.
extern "x86-interrupt" fn klavye(_frame: InterruptStackFrame) {
    let kod: u8 = unsafe { Port::new(0x60).read() };
    let mut klavye = KLAVYE_DURUMU.lock();
    if let Ok(Some(olay)) = klavye.add_byte(kod) {
        match klavye.process_keyevent(olay) {
            // Ctrl+C sıraya girmez: o an çalışan Fro kodunu hemen durdurmalı.
            Some(DecodedKey::Unicode('\u{3}')) => crate::fro::DURDUR.store(true, Ordering::Relaxed),
            Some(DecodedKey::Unicode(c)) => crate::kabuk::tus_ekle(c),
            _ => {}
        }
    }
    // PIC'e "bitti" demezsek bir sonraki kesmeyi göndermez.
    unsafe { PICS.lock().notify_end_of_interrupt(KLAVYE) };
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
