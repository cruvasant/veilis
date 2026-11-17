// GDT (Global Descriptor Table): işlemciye bellek bölümlerini tanıtan eski bir tablo.
// 64-bit modda bize asıl lazım olan, içine koyduğumuz TSS (Task State Segment):
// işlemci, istisna sırasında geçeceği yedek stack'lerin adresini oradan okur.

use spin::Once;
use x86_64::VirtAddr;
use x86_64::instructions::segmentation::{CS, SS, Segment};
use x86_64::instructions::tables::load_tss;
use x86_64::structures::gdt::{Descriptor, GlobalDescriptorTable, SegmentSelector};
use x86_64::structures::tss::TaskStateSegment;

pub const DOUBLE_FAULT_IST: u16 = 0;
const YIGIN_BOYUTU: usize = 4096 * 5;

static TSS: Once<TaskStateSegment> = Once::new();
static GDT: Once<(GlobalDescriptorTable, SegmentSelector, SegmentSelector, SegmentSelector)> = Once::new();

pub fn init() {
    let tss = TSS.call_once(|| {
        let mut tss = TaskStateSegment::new();
        // Double fault için ayrı stack. Stack aşağı doğru büyür, o yüzden sonunu veriyoruz.
        tss.interrupt_stack_table[DOUBLE_FAULT_IST as usize] = {
            static mut YIGIN: [u8; YIGIN_BOYUTU] = [0; YIGIN_BOYUTU];
            VirtAddr::from_ptr(&raw const YIGIN) + YIGIN_BOYUTU as u64
        };
        tss
    });

    let (gdt, kod, veri, tss_secici) = GDT.call_once(|| {
        let mut gdt = GlobalDescriptorTable::new();
        let kod = gdt.append(Descriptor::kernel_code_segment());
        let veri = gdt.append(Descriptor::kernel_data_segment());
        let tss_secici = gdt.append(Descriptor::tss_segment(tss));
        (gdt, kod, veri, tss_secici)
    });

    gdt.load();
    unsafe {
        CS::set_reg(*kod);
        // Önyükleyicinin SS değeri bizim tablomuzda başka bir girdiye denk gelir; yoksa kesme dönüşü çöker.
        SS::set_reg(*veri);
        load_tss(*tss_secici);
    }
}
