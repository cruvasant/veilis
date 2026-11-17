// Heap: çalışırken boyutu belli olmayan veriler (String, Vec, Box) için bellek havuzu.
// Sabit bir sanal adres aralığı seçip her sayfasını boş bir RAM çerçevesine eşliyoruz,
// sonra bu alanı linked_list_allocator'a yönetmesi için veriyoruz.

use linked_list_allocator::LockedHeap;
use x86_64::VirtAddr;
use x86_64::structures::paging::mapper::MapToError;
use x86_64::structures::paging::{FrameAllocator, Mapper, Page, PageTableFlags, Size4KiB};

pub const HEAP_BASLANGIC: u64 = 0x4444_4444_0000;
pub const HEAP_BOYUTU: u64 = 16 * 1024 * 1024;

#[global_allocator]
static HEAP: LockedHeap = LockedHeap::empty();

pub fn init(
    sayfalar: &mut impl Mapper<Size4KiB>,
    cerceveler: &mut impl FrameAllocator<Size4KiB>,
) -> Result<(), MapToError<Size4KiB>> {
    let ilk = Page::containing_address(VirtAddr::new(HEAP_BASLANGIC));
    let son = Page::containing_address(VirtAddr::new(HEAP_BASLANGIC + HEAP_BOYUTU - 1));

    for sayfa in Page::range_inclusive(ilk, son) {
        let cerceve = cerceveler.allocate_frame().ok_or(MapToError::FrameAllocationFailed)?;
        let izinler = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;
        unsafe { sayfalar.map_to(sayfa, cerceve, izinler, cerceveler)?.flush() };
    }

    unsafe { HEAP.lock().init(HEAP_BASLANGIC as *mut u8, HEAP_BOYUTU as usize) };
    Ok(())
}

/// (kullanılan, toplam) bayt
pub fn kullanim() -> (usize, usize) {
    let heap = HEAP.lock();
    (heap.used(), heap.size())
}

pub fn bos() -> usize {
    HEAP.lock().free()
}
