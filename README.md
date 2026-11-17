# Veilis

Veilis, gizlilik ve performans odaklı, açık kaynak bir işletim sistemidir.
Çekirdeği (kernel) Rust ile sıfırdan yazılıyor.

> **Durum:** Aşama 8. Çekirdek açılıyor, işlemci istisnalarını yakalıyor, belleği sayfalarla
> yönetiyor, Türkçe klavyeyle kullanılan bir komut satırı ve Fro dilinin ilk sürümünü sunuyor.
> Henüz günlük kullanıma uygun değildir; bir öğrenme ve geliştirme projesidir.

## Neler var?

- `no_std` ve `no_main` ile yazılmış bağımsız (freestanding) bir Rust çekirdeği
- VGA metin tamponu üzerinden ekrana yazı: `print!` / `println!`, satır kaydırma
- Panik olduğunda hata mesajını ekrana basma
- IDT ile işlemci istisnalarını yakalama: breakpoint ve double fault
- GDT ve TSS ile double fault için ayrı stack, böylece stack taşması da yakalanıyor
- PIC üzerinden donanım kesmeleri: zamanlayıcı ve klavye
- Türkçe Q klavye düzeni (Shift, Caps Lock, AltGr) ve VGA yazı tipine eklenen ı, İ, ş, Ş, ğ, Ğ
- Sayfalama (paging), fiziksel çerçeve dağıtıcı ve 100 KiB heap (`String`, `Vec`, `Box`)
- Page fault yakalama
- Basit komut satırı: `yardım`, `temizle`, `sürüm`, `süre`, `bellek`, `yaz`, `fro`
- Fro dili: Türkçe anahtar kelimeli, bytecode'a derlenip sanal makinede çalışan bir dil
  (metin, karşılaştırma, `eğer`, `döngü`, fonksiyonlar)

## Fro

Fro, Veilis'in kendi programlama dili. Komut satırında `fro` yazarak başlatılır, `çık` ile kapanır.

```
fro> a = 7
fro> eğer a > 3 { yaz "büyük" } değilse { yaz "küçük" }
büyük
fro> n = 3
fro> döngü n > 0 {
...>   yaz "kalan: " + n
...>   n = n - 1
...> }
kalan: 3
kalan: 2
kalan: 1
fro> fonksiyon fakt(n) {
...>   eğer n <= 1 { döndür 1 }
...>   döndür n * fakt(n - 1)
...> }
fro> yaz fakt(20)
2432902008176640000
```

Desteklenenler:

- Tam sayılar ve metinler (`"merhaba"`); metne `+` ile sayı ya da metin eklenebilir
- `+ - * / %`, parantezler, eksi işareti
- Karşılaştırma: `== != < > <= >=` (doğruysa 1, yanlışsa 0)
- Değişken atama, `yaz`, `eğer` / `değilse` / `değilse eğer`, `döngü`
- `fonksiyon` ve `döndür`; parametreler ve fonksiyon içindeki değişkenler yereldir,
  fonksiyonlar kendini çağırabilir (en fazla 200 iç içe çağrı)
- Deyimler `;` ya da yeni satırla ayrılır; `{` açık kaldıkça satırlar birikir
- Çalışan bir döngü Ctrl+C ile durdurulur
Her satır önce sözcüklere ayrılır, sonra bytecode'a derlenir ve sanal makinede çalıştırılır;
değişken adları derleme sırasında numaraya çevrildiği için çalışırken isim aranmaz.
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
Açılan pencerede `Merhaba, ben Veilis` yazısı ve komut satırı görünür. Pencereye tıklayıp
`yardım` yazarak komutları görebilirsiniz.

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
| `src/interrupts.rs` | Kesme tablosu (IDT), istisnalar, zamanlayıcı ve klavye |
| `src/klavye.rs` | Türkçe Q klavye düzeni |
| `src/kabuk.rs` | Komut satırı ve komutlar |
| `src/memory.rs` | Sayfa tabloları ve fiziksel çerçeve dağıtıcı |
| `src/allocator.rs` | Heap |
| `src/fro/` | Fro dili: sözcük ayırıcı, derleyici ve sanal makine |
| `src/gdt.rs` | GDT ve TSS: istisnalar için yedek stack |
| `x86_64-veilis.json` | Derleme hedefi: işletim sistemi olmayan x86_64 |
| `.cargo/config.toml` | `core` kütüphanesini kaynaktan derleme ve `cargo run` ayarları |
| `rust-toolchain.toml` | Kullanılacak Rust sürümü (nightly) |

## Teşekkür

Bu proje, Philipp Oppermann'ın
[Writing an OS in Rust](https://os.phil-opp.com/) yazı dizisinden öğrenilerek başlatılmıştır.

## Lisans

[MIT](LICENSE)
