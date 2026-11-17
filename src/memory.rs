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
// shortcut: hiç geri almıyor; çerçeve boşaltmak gerekince serbest liste ekle.
pub struct CerceveDagitici {
    harita: &'static MemoryMap,
    // Şu an dağıtılan bölgenin sırası ve içindeki sonraki adres; her seferinde baştan saymamak için.
    bolge: usize,
    sonraki: u64,
}

impl CerceveDagitici {
    /// # Safety
    /// Haritada "kullanılabilir" görünen bölgeler gerçekten boş olmalı.
    pub unsafe fn new(harita: &'static MemoryMap) -> Self {
        #[cfg(debug_assertions)]
        oz_denetim(harita);
        let sonraki = kullanilabilir(harita).next().map_or(0, |(bas, _)| bas);
        Self { harita, bolge: 0, sonraki }
    }
}

unsafe impl FrameAllocator<Size4KiB> for CerceveDagitici {
    fn allocate_frame(&mut self) -> Option<PhysFrame> {
        loop {
            let (_, son) = kullanilabilir(self.harita).nth(self.bolge)?;
            if self.sonraki + 4096 <= son {
                let adres = self.sonraki;
                self.sonraki += 4096;
                return Some(PhysFrame::containing_address(PhysAddr::new(adres)));
            }
            // Bölge doldu, sonrakine geç.
            self.bolge += 1;
            self.sonraki = kullanilabilir(self.harita).nth(self.bolge)?.0;
        }
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

// Çerçeveler 4 KiB hizalı, artan sırada ve kullanılabilir bölgelerin içinde gelmeli.
#[cfg(debug_assertions)]
fn oz_denetim(harita: &'static MemoryMap) {
    let sonraki = kullanilabilir(harita).next().map_or(0, |(bas, _)| bas);
    let mut dagitici = CerceveDagitici { harita, bolge: 0, sonraki };
    let mut onceki = None;
    for _ in 0..2048 {
        let adres = dagitici.allocate_frame().expect("oz_denetim: cerceve bitti").start_address().as_u64();
        assert!(adres % 4096 == 0, "oz_denetim: hizasiz cerceve");
        assert!(onceki.map_or(true, |o| adres > o), "oz_denetim: cerceve artmiyor");
        assert!(kullanilabilir(harita).any(|(bas, son)| adres >= bas && adres + 4096 <= son), "oz_denetim: bolge disi");
        onceki = Some(adres);
    }
}
