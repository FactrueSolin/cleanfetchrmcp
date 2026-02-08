pub fn count_words(input: &str) -> u32 {
    let mut count = 0u32;
    let mut in_word = false;

    for ch in input.chars() {
        if is_cjk_like(ch) {
            count += 1;
            in_word = false;
            continue;
        }

        if ch.is_ascii_alphanumeric() || (ch.is_alphanumeric() && !is_cjk_like(ch)) {
            if !in_word {
                count += 1;
                in_word = true;
            }
        } else {
            in_word = false;
        }
    }

    count
}

fn is_cjk_like(ch: char) -> bool {
    is_cjk(ch) || is_hiragana(ch) || is_katakana(ch) || is_hangul(ch)
}

fn is_cjk(ch: char) -> bool {
    matches!(
        ch as u32,
        0x3400..=0x4DBF
            | 0x4E00..=0x9FFF
            | 0xF900..=0xFAFF
            | 0x20000..=0x2A6DF
            | 0x2A700..=0x2B73F
            | 0x2B740..=0x2B81F
            | 0x2B820..=0x2CEAF
            | 0x2F800..=0x2FA1F
    )
}

fn is_hiragana(ch: char) -> bool {
    matches!(ch as u32, 0x3040..=0x309F)
}

fn is_katakana(ch: char) -> bool {
    matches!(ch as u32, 0x30A0..=0x30FF)
}

fn is_hangul(ch: char) -> bool {
    matches!(ch as u32, 0x1100..=0x11FF | 0x3130..=0x318F | 0xAC00..=0xD7AF)
}
