// 3. adım: sanal makine. Komutları sırayla çalıştırır, ara sonuçları yığında tutar.
//   Sayi(2), Sayi(3), Topla, Yaz  ->  yığın: [2] -> [2, 3] -> [5] -> çıktı: 5

use alloc::string::String;
use alloc::vec::Vec;

#[derive(Clone, Copy, Debug)]
pub enum Komut {
    Sayi(i64),
    Oku(usize),
    Ata(usize),
    Topla,
    Cikar,
    Carp,
    Bol,
    Kalan,
    Negatif,
    Yaz,
}

const TASMA: &str = "sonuç sığmıyor (64 bit sınırı aşıldı)";

pub fn calistir(kod: &[Komut], degerler: &mut [i64]) -> Result<Vec<i64>, String> {
    let mut yigin: Vec<i64> = Vec::with_capacity(16);
    let mut cikti = Vec::new();

    for &komut in kod {
        match komut {
            Komut::Sayi(n) => yigin.push(n),
            Komut::Oku(yer) => yigin.push(degerler[yer]),
            Komut::Ata(yer) => degerler[yer] = cek(&mut yigin),
            Komut::Yaz => cikti.push(cek(&mut yigin)),
            Komut::Negatif => {
                let a = cek(&mut yigin);
                yigin.push(a.checked_neg().ok_or(TASMA)?);
            }
            islem => {
                let b = cek(&mut yigin);
                let a = cek(&mut yigin);
                let sonuc = match islem {
                    Komut::Topla => a.checked_add(b),
                    Komut::Cikar => a.checked_sub(b),
                    Komut::Carp => a.checked_mul(b),
                    Komut::Bol | Komut::Kalan if b == 0 => return Err("sıfıra bölünemez".into()),
                    Komut::Bol => a.checked_div(b),
                    Komut::Kalan => a.checked_rem(b),
                    _ => unreachable!(),
                };
                yigin.push(sonuc.ok_or(TASMA)?);
            }
        }
    }
    Ok(cikti)
}

// Derleyici yığını hep dengeli kod üretir; boş yığından çekmek derleyici hatası demektir.
fn cek(yigin: &mut Vec<i64>) -> i64 {
    yigin.pop().expect("Fro: bozuk bytecode")
}
