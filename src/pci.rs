// En küçük PCI erişimi: 0xCF8'e "hangi aygıt, hangi kayıt" yazılır, 0xCFC'den okunur.

use x86_64::instructions::port::Port;

fn oku(aygit: u8, kayit: u8) -> u32 {
    let adres = 0x8000_0000u32 | (aygit as u32) << 11 | kayit as u32;
    unsafe {
        Port::<u32>::new(0xcf8).write(adres);
        Port::<u32>::new(0xcfc).read()
    }
}

// QEMU'nun standart ekran kartını (1234:1111) bulur, BAR0'ın fiziksel adresini verir.
// shortcut: yalnızca 0. veri yoluna bakıyor (QEMU kartı orada), köprülü yolları taramaz.
pub fn ekran_karti_bar0() -> Option<u64> {
    (0..32)
        .find(|&aygit| oku(aygit, 0) == 0x1111_1234)
        .map(|aygit| (oku(aygit, 0x10) & !0xf) as u64)
}
