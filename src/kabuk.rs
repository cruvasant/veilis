// Basit komut satırı (shell). Klavye kesmesi tuşları sıraya koyar; burada
// sıradan alıp satırı biriktiririz, Enter gelince komutu çalıştırırız.
// Kesme içinde uzun iş yapmamak için ikisini ayırıyoruz.

use spin::Mutex;
use x86_64::instructions::interrupts;

use crate::{print, println, vga};

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

pub fn calistir() -> ! {
    // Satırı UTF-8 bayt olarak tutuyoruz; Türkçe harfler 2 bayt.
    let mut tampon = [0u8; EN_FAZLA_KARAKTER * 4];
    let mut uzunluk = 0;
    let mut karakter = 0;

    print!("> ");
    loop {
        match sonraki_tus() {
            '\n' => {
                println!();
                komut(core::str::from_utf8(&tampon[..uzunluk]).unwrap_or(""));
                uzunluk = 0;
                karakter = 0;
                print!("> ");
            }
            '\u{8}' => {
                if let Some(son) = core::str::from_utf8(&tampon[..uzunluk])
                    .ok()
                    .and_then(|s| s.chars().next_back())
                {
                    uzunluk -= son.len_utf8();
                    karakter -= 1;
                    print!("\u{8}");
                }
            }
            c if !c.is_control() && karakter < EN_FAZLA_KARAKTER => {
                uzunluk += c.encode_utf8(&mut tampon[uzunluk..]).len();
                karakter += 1;
                print!("{}", c);
            }
            _ => {}
        }
    }
}

fn komut(satir: &str) {
    let (ad, geri_kalan) = satir.trim().split_once(' ').unwrap_or((satir.trim(), ""));
    match ad {
        "" => {}
        "yardım" | "yardim" => {
            println!("Komutlar:");
            println!("  yardım      bu listeyi gösterir");
            println!("  temizle     ekranı temizler");
            println!("  sürüm       Veilis sürümünü yazar");
            println!("  süre        ne kadar süredir açık olduğunu yazar");
            println!("  yaz <metin> yazdığın metni geri yazar");
        }
        "temizle" => vga::temizle(),
        "sürüm" | "surum" => println!("Veilis {}", env!("CARGO_PKG_VERSION")),
        "süre" | "sure" => {
            let sn = crate::interrupts::acik_kalma_saniye();
            println!("{} dakika {} saniyedir açık", sn / 60, sn % 60);
        }
        "yaz" => println!("{}", geri_kalan.trim()),
        _ => println!("Bilinmeyen komut: {}. Komutlar için 'yardım' yaz.", ad),
    }
}
