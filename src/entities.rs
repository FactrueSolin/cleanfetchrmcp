pub fn decode_entities(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '&' {
            let mut entity = String::new();
            let mut found_semicolon = false;

            for next_ch in chars.by_ref() {
                if next_ch == ';' {
                    found_semicolon = true;
                    break;
                } else if next_ch == '&' || entity.len() > 10 {
                    result.push('&');
                    result.push_str(&entity);
                    result.push(next_ch);
                    entity.clear();
                    break;
                } else {
                    entity.push(next_ch);
                }
            }

            if found_semicolon {
                if let Some(decoded) = decode_entity(&entity) {
                    result.push_str(&decoded);
                } else {
                    result.push('&');
                    result.push_str(&entity);
                    result.push(';');
                }
            } else if entity.is_empty() {
                result.push('&');
            } else {
                result.push('&');
                result.push_str(&entity);
            }
        } else {
            result.push(ch);
        }
    }

    result
}

fn decode_entity(entity: &str) -> Option<String> {
    if let Some(rest) = entity.strip_prefix('#') {
        return decode_numeric_entity(rest);
    }

    match entity {
        "amp" => Some("&".to_string()),
        "lt" => Some("<".to_string()),
        "gt" => Some(">".to_string()),
        "quot" => Some("\"".to_string()),
        "apos" => Some("'".to_string()),
        "nbsp" => Some("\u{00A0}".to_string()),
        "copy" => Some("©".to_string()),
        "reg" => Some("®".to_string()),
        "trade" => Some("™".to_string()),
        "euro" => Some("€".to_string()),
        "pound" => Some("£".to_string()),
        "yen" => Some("¥".to_string()),
        "times" => Some("×".to_string()),
        "divide" => Some("÷".to_string()),
        "minus" => Some("−".to_string()),
        "plusmn" => Some("±".to_string()),
        "ndash" => Some("–".to_string()),
        "mdash" => Some("—".to_string()),
        "hellip" => Some("…".to_string()),
        _ => None,
    }
}

fn decode_numeric_entity(entity: &str) -> Option<String> {
    if entity.is_empty() {
        return None;
    }
    let code_point = if entity.starts_with('x') || entity.starts_with('X') {
        u32::from_str_radix(&entity[1..], 16).ok()?
    } else {
        entity.parse::<u32>().ok()?
    };
    std::char::from_u32(code_point).map(|ch| ch.to_string())
}
