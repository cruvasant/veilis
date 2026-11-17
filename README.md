# Veilis

Veilis, gizlilik ve performans odaklı, açık kaynak bir işletim sistemidir.
Çekirdeği (kernel) Rust ile sıfırdan yazılıyor.

> **Durum:** Aşama 12. Çekirdek açılıyor, işlemci istisnalarını yakalıyor, belleği sayfalarla
> yönetiyor; Türkçe klavyeyle kullanılan bir komut satırı, Fro dili, diske dosya kaydetme, bir metin
> düzenleyici ve fareyle kullanılan pencereli bir masaüstü sunuyor. BIOS ve UEFI ile açılıyor.
> Henüz günlük kullanıma uygun değildir; bir öğrenme ve geliştirme projesidir.

## Neler var?

- `no_std` ve `no_main` ile yazılmış bağımsız (freestanding) bir Rust çekirdeği
- `bootloader` 0.11 ile hem BIOS hem UEFI üzerinden önyükleme, QEMU sanal makinesinde çalıştırma
- `print!` / `println!` ile yazı: RAM'deki 80x25 metin tablosuna ve seri porta (COM1) gider.
  Tablo, önyükleyicinin verdiği çerçeve belleğine gömülü 8x16 yazı tipiyle çizilir (VGA metin
  modu gerekmez); seri port gerçek donanımda hata ayıklamak için kullanılır
- Panik olduğunda hata mesajını ekrana basma
- IDT ile işlemci istisnalarını yakalama: breakpoint ve double fault
- GDT ve TSS ile double fault için ayrı stack, böylece stack taşması da yakalanıyor
- PIC üzerinden donanım kesmeleri: zamanlayıcı ve klavye
- Türkçe Q klavye düzeni (Shift, Caps Lock, AltGr)
- Sayfalama (paging), fiziksel çerçeve dağıtıcı ve 64 MiB heap (`String`, `Vec`, `Box`)
- Page fault yakalama
- ATA disk sürücüsü ve basit bir dosya sistemi (VeilisFS): 32 dosya, dosya başına en fazla 8 KiB
- Basit komut satırı: `yardım`, `temizle`, `sürüm`, `süre`, `bellek`, `yaz`, `fro`,
  `dosyalar`, `göster`, `düzenle`, `sil`
- Tam ekran metin düzenleyici (`düzenle <ad>`): ok tuşları, Home/End, Backspace/Delete,
  Ctrl+S ile kaydetme, Esc ile çıkış; kaydedilmemiş değişiklik varsa çıkmadan önce uyarır
- Masaüstü (`arayüz` komutu): firmware'in verdiği çerçeve belleğinde, her çözünürlükte (en az
  800x600), görev çubuğu ve saat, PS/2 fare, tıklayınca öne gelen ve başlık çubuğundan sürüklenen
  pencereler. Komut satırı, Fro ve düzenleyici bir terminal penceresinin içinde çalışır; çizim önce
  RAM'deki bir tuvale yapılır, sonra tek seferde ekrana kopyalanır
- Fro dili: Türkçe anahtar kelimeli, bytecode'a derlenip sanal makinede çalışan bir dil
  (metin, liste, karşılaştırma, `eğer`, `döngü`, fonksiyonlar)

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
fro> kareler = []
fro> i = 1; döngü i <= 5 { ekle(kareler, i * i); i = i + 1 }
fro> kareler
[1, 4, 9, 16, 25]
fro> kareler[2] + uzunluk(kareler)
14
```

Desteklenenler:

- Tam sayılar ve metinler (`"merhaba"`); metne `+` ile sayı ya da metin eklenebilir
- Listeler: `[1, 2, 3]`, `l[0]`, `l[0] = 5`, `ekle(l, 4)`, `uzunluk(l)`, `[1] + [2]`;
  listeler değer gibi davranır (`b = a` sonrası `b`'yi değiştirmek `a`'yı etkilemez)
- Bellek dolarsa Fro hata verir, çekirdek çalışmaya devam eder
- `+ - * / %`, parantezler, eksi işareti
- Karşılaştırma: `== != < > <= >=` (doğruysa 1, yanlışsa 0)
- Değişken atama, `yaz`, `eğer` / `değilse` / `değilse eğer`, `döngü`
- `fonksiyon` ve `döndür`; parametreler ve fonksiyon içindeki değişkenler yereldir,
  fonksiyonlar kendini çağırabilir (en fazla 200 iç içe çağrı)
- Deyimler `;` ya da yeni satırla ayrılır; `{` açık kaldıkça satırlar birikir
- Çalışan bir döngü Ctrl+C ile durdurulur
- `kaydet <ad>`: o Fro oturumunda hatasız çalışan bütün kodu veri diskine kaydeder;
  `yükle <ad>`: kaydedilen kodu çalıştırır (fonksiyonlar yeniden tanımlanır)
- Uzun programlar komut satırında `düzenle <ad>` ile yazılıp Fro'da `yükle <ad>` ile çalıştırılabilir

Her satır önce sözcüklere ayrılır, sonra bytecode'a derlenir ve sanal makinede çalıştırılır;
değişken adları derleme sırasında numaraya çevrildiği için çalışırken isim aranmaz.

## Gereksinimler

| Araç | Ne işe yarar |
|------|--------------|
| [rustup](https://rustup.rs) | Rust sürümlerini kurar ve yönetir |
| Rust nightly + `rust-src` + `llvm-tools-preview` + `x86_64-unknown-none` hedefi | `rust-toolchain.toml` sayesinde proje klasöründe otomatik kurulur ve seçilir |
| [QEMU](https://www.qemu.org/download/) | Çekirdeği sanal makinede çalıştırır |

Ek bir araç gerekmez: disk imajlarını `bootloader` kütüphanesi `build.rs` içinden üretir.

QEMU kurulduktan sonra `qemu-system-x86_64` komutunun PATH'te olduğundan emin olun.
Windows'ta bu klasör genellikle `C:\Program Files\qemu` olur. UEFI ile açmak için QEMU'nun
`share` klasöründeki `edk2-x86_64-code.fd` yazılımı kullanılır; başka bir yerdeyse
`VEILIS_UEFI_YAZILIMI` ortam değişkenine dosyanın yolunu verin.

## Çalıştırma

```sh
git clone <bu deponun adresi>
cd veilis
qemu-img create -f raw veri.img 1M
cargo run
```

`qemu-img` satırı yalnızca ilk seferde gerekir: Fro dosyalarının saklanacağı boş bir veri diski
(`veri.img`) oluşturur. Veilis boş diski ilk açılışta kendisi biçimlendirir; içinde başka veri olan
bir diske ise güvenlik için hiç yazmaz. `veri.img` kişisel dosyalarını içerdiği için `.gitignore`
ile depo dışında tutulur.

`cargo run` önce çekirdeği derler, sonra BIOS ve UEFI disk imajlarını üretir ve BIOS imajını
QEMU'da açar. UEFI imajını açmak için:

```sh
cargo run -- uefi
```

Açılan pencerede komut satırı görünür; `yardım` yazarak komutları görebilirsiniz. Ekrana
yazılan her şey `cargo run` komutunun çalıştığı terminale de seri port (COM1) üzerinden gelir.
QEMU'ya 256 MB bellek verilir (büyük ekranlarda masaüstü tuvali için).

İmajlar `target/veilis-bios.img` ve `target/veilis-uefi.img` olarak yazılır. Gerçek bir
bilgisayarda denemek için aşağıdaki bölüme bakın.

## Proje yapısı

| Dosya | Açıklama |
|-------|----------|
| `src/main.rs` | `cargo run` başlatıcısı: imajı QEMU'da açar (`uefi` argümanıyla UEFI) |
| `build.rs` | Çekirdeği önyükleyiciyle birleştirip BIOS ve UEFI imajlarını üretir |
| `kernel/src/main.rs` | Çekirdeğin kaynak kodu (`entry_point!` başlangıç noktası) |
| `kernel/src/vga.rs` | RAM'deki metin tablosu ve `print!` / `println!` makroları |
| `kernel/src/seri.rs` | COM1 seri port yazıcısı |
| `kernel/src/interrupts.rs` | Kesme tablosu (IDT), istisnalar, zamanlayıcı ve klavye |
| `kernel/src/klavye.rs` | Türkçe Q klavye düzeni |
| `kernel/src/kabuk.rs` | Komut satırı ve komutlar |
| `kernel/src/memory.rs` | Sayfa tabloları ve fiziksel çerçeve dağıtıcı |
| `kernel/src/allocator.rs` | Heap |
| `kernel/src/fro/` | Fro dili: sözcük ayırıcı, derleyici ve sanal makine |
| `kernel/src/ata.rs` | ATA disk sürücüsü |
| `kernel/src/dosya.rs` | VeilisFS dosya sistemi |
| `kernel/src/duzenleyici.rs` | Tam ekran metin düzenleyici |
| `kernel/src/fare.rs` | PS/2 fare sürücüsü |
| `kernel/src/grafik.rs` | Çerçeve belleği: konsol çizimi, RAM tuvali ve ekrana kopyalama |
| `kernel/src/yazitipi.rs` | Gömülü 8x16 ekran yazı tipi |
| `kernel/src/masaustu.rs` | Pencere yöneticisi: pencereler, görev çubuğu, fare imleci |
| `kernel/src/gdt.rs` | GDT ve TSS: istisnalar için yedek stack |
| `.cargo/config.toml` | Çekirdeği `build.rs`'e bağımlılık olarak vermek için gereken ayar |
| `rust-toolchain.toml` | Kullanılacak Rust sürümü (nightly) |

## Gerçek bilgisayarda deneme

> **Uyarı:** Bir imajı USB belleğe yazmak, bellekteki her şeyi siler. Yazmadan önce doğru diski
> seçtiğinizden emin olun; yanlış diski seçmek o diskteki verileri geri dönüşsüz yok eder.
> Veilis henüz yalnızca QEMU'da denenmiştir.

1. `cargo build` ile imajları üretin. Yeni bilgisayarlar için `target/veilis-uefi.img`, eski
   (BIOS / Legacy / CSM) bilgisayarlar için `target/veilis-bios.img` kullanın.
2. İmajı USB belleğe ham olarak yazın. Windows'ta [Rufus](https://rufus.ie) ile: aygıt olarak USB
   belleği, önyükleme seçimi olarak `.img` dosyasını seçin ve "DD imajı" olarak yazın.
   Linux'ta: `sudo dd if=target/veilis-uefi.img of=/dev/sdX bs=4M status=progress && sync`
   (`/dev/sdX` USB belleğin kendisi olmalı; `lsblk` ile kontrol edin).
3. UEFI ile açacaksanız firmware ayarlarından Secure Boot'u kapatın; Veilis imzalı değildir.
4. Bilgisayarın açılış menüsünden (çoğunlukla F12, F11, F8 ya da Esc) USB belleği seçin.

Bilinen sınırlar:

- Klavye ve fare için yalnızca PS/2 sürücüsü var. Dizüstü bilgisayarların dahili klavyesi
  çoğunlukla PS/2 üzerinden bağlıdır. USB klavye ve fare, BIOS/CSM ile açılışta çoğu bilgisayarda
  firmware'in eski aygıt desteği sayesinde çalışır; UEFI ile açılışta büyük ihtimalle çalışmaz.
- Veri diski için yalnızca eski ATA (IDE) desteği var; modern SATA (AHCI) ve NVMe diskler
  görülmez, dosya komutları "veri diski yok" der. Bu, gerçek diskinize hiç yazılmayacağı anlamına
  da gelir.
- Bilgisayarda seri port (COM1) varsa ekrandaki her şey oradan da 115200 baud ile çıkar; ekran
  açılmazsa hata ayıklamak için kullanılabilir.

## Teşekkür

Bu proje, Philipp Oppermann'ın
[Writing an OS in Rust](https://os.phil-opp.com/) yazı dizisinden öğrenilerek başlatılmıştır.
Ekran yazı tipi, kamu malı olan X11 misc-fixed 8x13 yazı tipinden üretilmiştir.

## Lisans

[MIT](LICENSE)
