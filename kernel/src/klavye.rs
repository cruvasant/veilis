// Türkçe Q klavye düzeni. Klavye bize harfi değil, tuşun yerini (scancode) söyler;
// "bu konumdaki tuş Türkçe klavyede hangi harf?" sorusunu burada cevaplıyoruz.
// Türkçe'de farklı olan tuşları tanımlıyoruz, gerisini (harfler, Esc, numpad...)
// US düzenine bırakıyoruz.

use pc_keyboard::layouts::Us104Key;
use pc_keyboard::{DecodedKey, HandleControl, KeyCode, KeyboardLayout, Modifiers, PhysicalKeyboard};

pub struct TurkceQ;

impl KeyboardLayout for TurkceQ {
    #[rustfmt::skip]
    fn map_keycode(&self, tus: KeyCode, m: &Modifiers, ctrl: HandleControl) -> DecodedKey {
        let k = match tus {
            KeyCode::Oem8      => sembol(m, '"', 'é', '"'),
            KeyCode::Key1      => sembol(m, '1', '!', '>'),
            KeyCode::Key2      => sembol(m, '2', '\'', '£'),
            KeyCode::Key3      => sembol(m, '3', '^', '#'),
            KeyCode::Key4      => sembol(m, '4', '+', '$'),
            KeyCode::Key5      => sembol(m, '5', '%', '½'),
            KeyCode::Key6      => sembol(m, '6', '&', '6'),
            KeyCode::Key7      => sembol(m, '7', '/', '{'),
            KeyCode::Key8      => sembol(m, '8', '(', '['),
            KeyCode::Key9      => sembol(m, '9', ')', ']'),
            KeyCode::Key0      => sembol(m, '0', '=', '}'),
            KeyCode::OemMinus  => sembol(m, '*', '?', '\\'),
            KeyCode::OemPlus   => sembol(m, '-', '_', '|'),
            KeyCode::Q if m.is_altgr() => '@',
            KeyCode::I         => harf(m, 'ı', 'I'),
            KeyCode::Oem4      => harf(m, 'ğ', 'Ğ'),
            KeyCode::Oem6      => if m.is_altgr() { '~' } else { harf(m, 'ü', 'Ü') },
            KeyCode::Oem7      => sembol(m, ',', ';', '`'),
            KeyCode::Oem1      => harf(m, 'ş', 'Ş'),
            KeyCode::Oem3      => harf(m, 'i', 'İ'),
            KeyCode::Oem5      => sembol(m, '<', '>', '|'),
            KeyCode::OemComma  => harf(m, 'ö', 'Ö'),
            KeyCode::OemPeriod => harf(m, 'ç', 'Ç'),
            KeyCode::Oem2      => sembol(m, '.', ':', '.'),
            _ => return Us104Key.map_keycode(tus, m, ctrl),
        };
        DecodedKey::Unicode(k)
    }

    fn get_physical(&self) -> PhysicalKeyboard {
        PhysicalKeyboard::Iso
    }
}

// Caps Lock harfleri etkiler, sembolleri etkilemez.
fn harf(m: &Modifiers, kucuk: char, buyuk: char) -> char {
    if m.is_caps() { buyuk } else { kucuk }
}

fn sembol(m: &Modifiers, duz: char, shift: char, altgr: char) -> char {
    if m.is_altgr() {
        altgr
    } else if m.is_shifted() {
        shift
    } else {
        duz
    }
}
