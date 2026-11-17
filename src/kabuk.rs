// Basit komut satırı (shell). Klavye kesmesi tuşları sıraya koyar; burada
// sıradan alıp satırı biriktiririz, Enter gelince komutu çalıştırırız.
// Kesme içinde uzun iş yapmamak için ikisini ayırıyoruz.

use alloc::string::String;
use spin::Mutex;
use x86_64::instructions::interrupts;

use crate::{fro, print, println, vga};

const KUYRUK_BOYUTU: usize = 32;
// "> " ile birlikte bir ekran satırına sığsın.
const EN_FAZLA_KARAKTER: usize = 70;

struct Kuyruk {
    tuslar: [char; KUYRUK_BOYUTU],
    bas: usize,
    adet: usize,
}

static TUSLAR: Mutex<Kuyruk> = Mutex::new(Kuyruk {
    tuslar: ['\0'; KUYRUK_BOYUTU],
    bas: 0,
    adet: 0,
});

// Klavye kesmesinden çağrılır. Sıra doluysa tuş kaybolur.
pub fn tus_ekle(c: char) {
    let mut k = TUSLAR.lock();
    if k.adet < KUYRUK_BOYUTU {
        let yer = (k.bas + k.adet) % KUYRUK_BOYUTU;
        k.tuslar[yer] = c;
        k.adet += 1;
    }
}

fn sonraki_tus() -> char {
    loop {
        // Kesmeleri kapatıp bakıyoruz; sıra boşsa "kesmeleri aç ve uyu" tek adımda
        // yapılır, böylece arada gelen bir tuşu kaçırmayız.
        interrupts::disable();
        let mut k = TUSLAR.lock();
        if k.adet > 0 {
            let c = k.tuslar[k.bas];
            k.bas = (k.bas + 1) % KUYRUK_BOYUTU;
            k.adet -= 1;
            drop(k);
            interrupts::enable();
            return c;
        }
        drop(k);
        interrupts::enable_and_hlt();
    }
}

pub fn calistir(toplam_ram: u64) -> ! {
    // Satır artık heap'te: String gerektiği kadar büyür.
    let mut satir = String::new();
    // Some ise Fro modundayız; değişkenler oturum boyunca kalır.
    let mut fro: Option<fro::Ortam> = None;
    // Açık bir { bloğu varsa satırlar burada birikir, blok kapanınca çalışır.
    let mut fro_kod = String::new();

    print!("> ");
    loop {
        match sonraki_tus() {
            '\n' => {
                println!();
                match (&mut fro, satir.trim()) {
                    (Some(_), "çık" | "cik") if fro_kod.is_empty() => fro = None,
                    (Some(ortam), _) => {
                        fro_kod.push_str(&satir);
                        fro_kod.push('\n');
                        // shortcut: metin içindeki { } de sayılıyor; gerekirse sözcük ayırıcıya bırak.
                        if fro_kod.matches('{').count() <= fro_kod.matches('}').count() {
                            if let Err(hata) = ortam.calistir(&fro_kod, &mut |d| println!("{}", d)) {
                                println!("Hata: {}", hata);
                            }
                            fro_kod.clear();
                        }
                    }
                    (None, "fro") => {
                        println!("Fro {}. Çıkmak için 'çık' yaz.", env!("CARGO_PKG_VERSION"));
                        println!("Çalışan döngüyü durdurmak için Ctrl+C.");
                        fro = Some(fro::Ortam::new());
                    }
                    (None, _) => komut(&satir, toplam_ram),
                }
                satir.clear();
                print!("{}", match &fro {
                    None => "> ",
                    Some(_) if fro_kod.is_empty() => "fro> ",
                    Some(_) => "...> ",
                });
            }
            '\u{8}' => {
                if satir.pop().is_some() {
                    print!("\u{8}");
                }
            }
            c if !c.is_control() && satir.chars().count() < EN_FAZLA_KARAKTER => {
                satir.push(c);
                print!("{}", c);
            }
            _ => {}
        }
    }
}

fn komut(satir: &str, toplam_ram: u64) {
    let (ad, geri_kalan) = satir.trim().split_once(' ').unwrap_or((satir.trim(), ""));
    match ad {
        "" => {}
        "yardım" | "yardim" => {
            println!("Komutlar:");
            println!("  yardım      bu listeyi gösterir");
            println!("  temizle     ekranı temizler");
            println!("  sürüm       Veilis sürümünü yazar");
            println!("  süre        ne kadar süredir açık olduğunu yazar");
            println!("  bellek      RAM ve heap kullanımını gösterir");
            println!("  fro         Fro dilini başlatır");
            println!("  yaz <metin> yazdığın metni geri yazar");
        }
        "temizle" => vga::temizle(),
        "sürüm" | "surum" => println!("Veilis {}", env!("CARGO_PKG_VERSION")),
        "süre" | "sure" => {
            let sn = crate::interrupts::acik_kalma_saniye();
            println!("{} dakika {} saniyedir açık", sn / 60, sn % 60);
        }
        "bellek" => {
            let (kullanilan, heap) = crate::allocator::kullanim();
            println!("RAM:  {} MB kullanılabilir", toplam_ram / 1024 / 1024);
            println!("Heap: {} / {} bayt kullanımda", kullanilan, heap);
        }
        "yaz" => println!("{}", geri_kalan.trim()),
        _ => println!("Bilinmeyen komut: {}. Komutlar için 'yardım' yaz.", ad),
    }
}
