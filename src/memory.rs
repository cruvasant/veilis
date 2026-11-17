// Sanal bellek: programın gördüğü adresler (sanal) ile RAM'deki gerçek yerler
// (fiziksel) aynı değil. Aradaki eşleşmeyi sayfa tabloları tutar; her sayfa 4 KiB.
// Önyükleyici bütün fiziksel belleği `fiziksel_ofset` adresinden itibaren
// eşledi, böylece sayfa tablolarını okuyup değiştirebiliyoruz.

use bootloader::bootinfo::{MemoryMap, MemoryRegionType};
use x86_64::registers::control::Cr3;
use x86_64::structures::paging::{FrameAllocator, OffsetPageTable, PageTable, PhysFrame, Size4KiB};
use x86_64::{PhysAddr, VirtAddr};

/// # Safety
/// `fiziksel_ofset` önyükleyicinin verdiği değer olmalı ve yalnızca bir kez çağrılmalı.
pub unsafe fn init(fiziksel_ofset: VirtAddr) -> OffsetPageTable<'static> {
    // CR3 yazmacı, en üst seviye (4. seviye) sayfa tablosunun fiziksel adresini tutar.
    let (cerceve, _) = Cr3::read();
    let adres = fiziksel_ofset + cerceve.start_address().as_u64();
    unsafe {
        let tablo: &'static mut PageTable = &mut *adres.as_mut_ptr();
        OffsetPageTable::new(tablo, fiziksel_ofset)
    }
}

// Boş RAM'i 4 KiB'lık çerçeveler halinde dağıtır.
// shortcut: hiç geri almıyor ve her seferinde baştan sayıyor; çok çerçeve gerekince değiştir.
pub struct CerceveDagitici {
    harita: &'static MemoryMap,
    sonraki: usize,
}

impl CerceveDagitici {
    /// # Safety
    /// Haritada "kullanılabilir" görünen bölgeler gerçekten boş olmalı.
    pub unsafe fn new(harita: &'static MemoryMap) -> Self {
        Self { harita, sonraki: 0 }
    }
}

unsafe impl FrameAllocator<Size4KiB> for CerceveDagitici {
    fn allocate_frame(&mut self) -> Option<PhysFrame> {
        let cerceve = kullanilabilir(self.harita)
            .flat_map(|(bas, son)| (bas..son).step_by(4096))
            .map(|adres| PhysFrame::containing_address(PhysAddr::new(adres)))
            .nth(self.sonraki);
        self.sonraki += 1;
        cerceve
    }
}

pub fn kullanilabilir_bayt(harita: &MemoryMap) -> u64 {
    kullanilabilir(harita).map(|(bas, son)| son - bas).sum()
}

fn kullanilabilir(harita: &MemoryMap) -> impl Iterator<Item = (u64, u64)> + '_ {
    harita
        .iter()
        .filter(|bolge| bolge.region_type == MemoryRegionType::Usable)
        .map(|bolge| (bolge.range.start_addr(), bolge.range.end_addr()))
}
