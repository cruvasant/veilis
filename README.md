# Veilis

Veilis, gizlilik ve performans odaklı, açık kaynak bir işletim sistemidir.
Çekirdeği (kernel) Rust ile sıfırdan yazılıyor.

> **Durum:** Aşama 3. Çekirdek açılıyor, `println!` ile ekrana yazabiliyor ve işlemci istisnalarını yakalıyor.
> Henüz günlük kullanıma uygun değildir; bir öğrenme ve geliştirme projesidir.

## Neler var?

- `no_std` ve `no_main` ile yazılmış bağımsız (freestanding) bir Rust çekirdeği
- VGA metin tamponu üzerinden ekrana yazı: `print!` / `println!`, satır kaydırma
- Panik olduğunda hata mesajını ekrana basma
- IDT ile işlemci istisnalarını yakalama: breakpoint ve double fault
- `bootloader` 0.9 ile BIOS üzerinden önyükleme
- QEMU sanal makinesinde çalıştırma

## Gereksinimler

| Araç | Ne işe yarar |
|------|--------------|
| [rustup](https://rustup.rs) | Rust sürümlerini kurar ve yönetir |
| Rust nightly + `rust-src` + `llvm-tools-preview` | `rust-toolchain.toml` sayesinde proje klasöründe otomatik seçilir |
| [`bootimage`](https://github.com/rust-osdev/bootimage) | Çekirdekten önyüklenebilir disk imajı üretir |
| [QEMU](https://www.qemu.org/download/) | Çekirdeği sanal makinede çalıştırır |

`bootimage` kurulumu:

```sh
cargo install bootimage
```

QEMU kurulduktan sonra `qemu-system-x86_64` komutunun PATH'te olduğundan emin olun.
Windows'ta bu klasör genellikle `C:\Program Files\qemu` olur.

## Çalıştırma

```sh
git clone <bu deponun adresi>
cd veilis
cargo run
```

`cargo run` önce çekirdeği derler, sonra disk imajını üretir ve QEMU'da açar.
Açılan pencerede siyah ekranın sol üst köşesinde `Merhaba, ben Veilis` yazısı görünür.

Yalnızca disk imajını üretmek için:

```sh
cargo bootimage
```

İmaj `target/x86_64-veilis/debug/bootimage-veilis.bin` konumunda oluşur.

## Proje yapısı

| Dosya | Açıklama |
|-------|----------|
| `src/main.rs` | Çekirdeğin kaynak kodu (`_start` başlangıç noktası) |
| `src/vga.rs` | VGA ekran yazıcısı ve `print!` / `println!` makroları |
| `src/interrupts.rs` | Kesme tablosu (IDT) ve istisna fonksiyonları |
| `x86_64-veilis.json` | Derleme hedefi: işletim sistemi olmayan x86_64 |
| `.cargo/config.toml` | `core` kütüphanesini kaynaktan derleme ve `cargo run` ayarları |
| `rust-toolchain.toml` | Kullanılacak Rust sürümü (nightly) |

## Teşekkür

Bu proje, Philipp Oppermann'ın
[Writing an OS in Rust](https://os.phil-opp.com/) yazı dizisinden öğrenilerek başlatılmıştır.

## Lisans

[MIT](LICENSE)
