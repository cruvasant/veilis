// `cargo run`: imajı QEMU'da açar. `cargo run -- uefi` UEFI imajını açar.

use std::path::PathBuf;
use std::process::Command;

const UEFI_YAZILIMI: &str = "edk2-x86_64-code.fd";

// VEILIS_UEFI_YAZILIMI ile verilen yolda, yoksa PATH'teki QEMU'nun yanındaki share klasöründe aranır.
fn uefi_yazilimi() -> PathBuf {
    if let Some(yol) = std::env::var_os("VEILIS_UEFI_YAZILIMI") {
        return yol.into();
    }
    std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default())
        .map(|klasor| klasor.join("share").join(UEFI_YAZILIMI))
        .find(|yol| yol.exists())
        .expect("UEFI yazılımı bulunamadı; VEILIS_UEFI_YAZILIMI ile edk2-x86_64-code.fd yolunu ver")
}

fn main() {
    let uefi = std::env::args().nth(1).is_some_and(|a| a == "uefi");
    let mut qemu = Command::new("qemu-system-x86_64");
    if uefi {
        qemu.arg("-drive").arg(format!("if=pflash,format=raw,readonly=on,file={}", uefi_yazilimi().display()));
    }
    let imaj = if uefi { env!("UEFI_IMAJI") } else { env!("BIOS_IMAJI") };
    qemu.arg("-drive").arg(format!("format=raw,file={imaj},if=ide,index=0"));
    // İkinci disk: Fro dosyalarının saklandığı veri diski (oluşturmak için README'ye bakın).
    qemu.args(["-drive", "format=raw,file=veri.img,if=ide,index=1"]);
    // Hata ayıklama çıktısı bu terminale gelir.
    qemu.args(["-serial", "stdio"]);
    std::process::exit(qemu.status().expect("QEMU başlatılamadı").code().unwrap_or(1));
}
