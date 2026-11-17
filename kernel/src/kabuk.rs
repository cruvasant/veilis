// Basit komut satırı (shell). Klavye kesmesi tuşları sıraya koyar; burada
// sıradan alıp satırı biriktiririz, Enter gelince komutu çalıştırırız.
// Kesme içinde uzun iş yapmamak için ikisini ayırıyoruz.

use alloc::string::String;
use spin::Mutex;
use x86_64::instructions::interrupts;

use crate::{dosya, fro, grafik, masaustu, print, println, vga};

// Karakter olmayan tuşlar için Unicode'un "özel kullanım" bölgesinden kodlar.
pub const YUKARI: char = '\u{e000}';
pub const ASAGI: char = '\u{e001}';
pub const SOL: char = '\u{e002}';
pub const SAG: char = '\u{e003}';
pub const BAS: char = '\u{e004}';
pub const SON: char = '\u{e005}';

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

pub fn sonraki_tus() -> char {
    loop {
        // shortcut: ekran yalnızca tuş beklerken güncellenir, uzun süren bir Fro işi bitene kadar görüntü donar, gerekirse çizimi ayrı bir göreve taşı
        if masaustu::acik() {
            masaustu::guncelle();
        } else if vga::kirli_al() {
            grafik::konsol_ciz();
        }
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
    // Bu Fro oturumunda hatasız çalışan kod; "kaydet" bunu dosyaya yazar.
    let mut fro_gecmis = String::new();

    print!("> ");
    loop {
        match sonraki_tus() {
            '\n' => {
                println!();
                match (&mut fro, satir.trim()) {
                    (Some(ortam), _) => {
                        if !fro_satiri(&satir, ortam, &mut fro_kod, &mut fro_gecmis) {
                            fro = None;
                        }
                    }
                    (None, "fro") => {
                        println!("Fro {}. Çıkmak için 'çık' yaz.", env!("CARGO_PKG_VERSION"));
                        println!("Durdurmak: Ctrl+C. Kaydetmek: kaydet <ad>. Açmak: yükle <ad>.");
                        fro = Some(fro::Ortam::new(crate::allocator::bos));
                        fro_gecmis.clear();
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
            YUKARI..=SON => {}
            c if !c.is_control() && satir.chars().count() < EN_FAZLA_KARAKTER => {
                satir.push(c);
                print!("{}", c);
            }
            _ => {}
        }
    }
}

// Fro modunda bir satırı işler. false dönerse Fro'dan çıkılır.
fn fro_satiri(satir: &str, ortam: &mut fro::Ortam, kod: &mut String, gecmis: &mut String) -> bool {
    // kaydet, yükle ve çık Fro'nun değil komut satırının işleri; blok ortasında geçerli değil.
    if kod.is_empty() {
        match satir.trim().split_once(' ') {
            None if matches!(satir.trim(), "çık" | "cik") => return false,
            Some(("kaydet", ad)) => {
                match dosya::yaz(ad.trim(), gecmis.as_bytes()) {
                    Ok(()) => println!("{} kaydedildi ({} bayt).", ad.trim(), gecmis.len()),
                    Err(hata) => println!("Hata: {}", hata),
                }
                return true;
            }
            Some(("yükle" | "yukle", ad)) => {
                match dosya::oku(ad.trim()) {
                    Ok(veri) => fro_calistir(&String::from_utf8_lossy(&veri), ortam, gecmis),
                    Err(hata) => println!("Hata: {}", hata),
                }
                return true;
            }
            _ => {}
        }
    }
    kod.push_str(satir);
    kod.push('\n');
    // shortcut: metin içindeki { } de sayılıyor; gerekirse sözcük ayırıcıya bırak.
    if kod.matches('{').count() <= kod.matches('}').count() {
        fro_calistir(kod, ortam, gecmis);
        kod.clear();
    }
    true
}

fn fro_calistir(kod: &str, ortam: &mut fro::Ortam, gecmis: &mut String) {
    match ortam.calistir(kod, &mut |d| println!("{}", d)) {
        Ok(()) => gecmis.push_str(kod),
        Err(hata) => println!("Hata: {}", hata),
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
            println!("  dosyalar    kayıtlı dosyaları listeler");
            println!("  göster <ad> dosyanın içini yazar");
            println!("  düzenle <ad> dosyayı düzenleyicide açar (yoksa oluşturur)");
            println!("  sil <ad>    dosyayı siler");
            println!("  arayüz      masaüstüne geçer (geri dönüş yok)");
            println!("  yaz <metin> yazdığın metni geri yazar");
        }
        "dosyalar" => match dosya::listele() {
            Ok(liste) if liste.is_empty() => println!("Henüz dosya yok."),
            Ok(liste) => liste.iter().for_each(|(ad, boyut)| println!("  {:<20} {} bayt", ad, boyut)),
            Err(hata) => println!("Hata: {}", hata),
        },
        "göster" | "goster" => match dosya::oku(geri_kalan.trim()) {
            Ok(veri) => {
                print!("{}", String::from_utf8_lossy(&veri));
                // Düzenleyicide yazılan dosya yeni satırla bitmeyebilir; komut istemi alt satıra insin.
                if veri.last().is_some_and(|&b| b != b'\n') {
                    println!();
                }
            }
            Err(hata) => println!("Hata: {}", hata),
        },
        "düzenle" | "duzenle" => crate::duzenleyici::ac(geri_kalan.trim()),
        "sil" => match dosya::sil(geri_kalan.trim()) {
            Ok(()) => println!("{} silindi.", geri_kalan.trim()),
            Err(hata) => println!("Hata: {}", hata),
        },
        "arayüz" | "arayuz" if masaustu::acik() => println!("Arayüz zaten açık."),
        "arayüz" | "arayuz" => {
            if let Err(hata) = masaustu::ac() {
                println!("Hata: {}", hata);
            }
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
