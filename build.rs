// Derlenmiş çekirdeği önyükleyiciyle birleştirip BIOS ve UEFI disk imajları üretir.

use std::path::PathBuf;

fn main() {
    let cekirdek = PathBuf::from(std::env::var_os("CARGO_BIN_FILE_KERNEL_kernel").unwrap());
    // Kolay bulunsun diye imajlar `target/` altına yazılır.
    let hedef = PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap()).join("target");

    let bios = hedef.join("veilis-bios.img");
    bootloader::BiosBoot::new(&cekirdek).create_disk_image(&bios).unwrap();
    let uefi = hedef.join("veilis-uefi.img");
    bootloader::UefiBoot::new(&cekirdek).create_disk_image(&uefi).unwrap();

    println!("cargo:rustc-env=BIOS_IMAJI={}", bios.display());
    println!("cargo:rustc-env=UEFI_IMAJI={}", uefi.display());
}
