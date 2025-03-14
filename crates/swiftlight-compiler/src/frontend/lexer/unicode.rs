//! # Unicode文字ユーティリティ
//! 
//! 字句解析で使用するUnicode文字の処理ユーティリティを提供します。
//! 識別子や数値の判定に使用する関数などを含みます。

/// 識別子の先頭に使用できる文字かどうかを判定
///
/// 英字、アンダースコア、ユニコードの文字（特に日本語など）が使用可能
pub fn is_identifier_start(c: char) -> bool {
    c.is_alphabetic() || c == '_' || (c > '\u{80}' && is_xid_start(c))
}

/// 識別子の2文字目以降に使用できる文字かどうかを判定
///
/// 英数字、アンダースコア、ユニコードの文字が使用可能
pub fn is_identifier_continue(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || (c > '\u{80}' && is_xid_continue(c))
}

/// 数値の先頭に使用できる文字かどうかを判定
pub fn is_digit_start(c: char) -> bool {
    c.is_ascii_digit()
}

/// 16進数に使用できる文字かどうかを判定
pub fn is_hex_digit(c: char) -> bool {
    c.is_ascii_digit() || ('a'..='f').contains(&c) || ('A'..='F').contains(&c)
}

/// 8進数に使用できる文字かどうかを判定
pub fn is_octal_digit(c: char) -> bool {
    ('0'..='7').contains(&c)
}

/// 2進数に使用できる文字かどうかを判定
pub fn is_binary_digit(c: char) -> bool {
    c == '0' || c == '1'
}

/// 空白文字かどうかを判定
pub fn is_whitespace(c: char) -> bool {
    // 通常の空白文字に加え、全角空白なども含める
    c.is_whitespace()
}

/// XMLNameStartChar判定
///
/// これは識別子の開始文字として使用できる文字の範囲を定義する
/// Unicode標準に準拠している
fn is_xid_start(c: char) -> bool {
    matches!(c,
        'A'..='Z' | 'a'..='z' | '_' |
        '\u{00C0}'..='\u{00D6}' | '\u{00D8}'..='\u{00F6}' | '\u{00F8}'..='\u{02FF}' |
        '\u{0370}'..='\u{037D}' | '\u{037F}'..='\u{1FFF}' | '\u{200C}'..='\u{200D}' |
        '\u{2070}'..='\u{218F}' | '\u{2C00}'..='\u{2FEF}' | '\u{3001}'..='\u{D7FF}' |
        '\u{F900}'..='\u{FDCF}' | '\u{FDF0}'..='\u{FFFD}' | '\u{10000}'..='\u{EFFFF}'
    )
}

/// XMLNameChar判定
///
/// これは識別子の継続文字として使用できる文字の範囲を定義する
/// Unicode標準に準拠している
fn is_xid_continue(c: char) -> bool {
    is_xid_start(c) || matches!(c,
        '-' | '.' | '0'..='9' | '\u{00B7}' | '\u{0300}'..='\u{036F}' |
        '\u{203F}'..='\u{2040}'
    )
}

/// 文字をエスケープ表現に変換
///
/// 特殊文字をエスケープシーケンスに変換する
pub fn escape_char(c: char) -> String {
    match c {
        '\n' => "\\n".to_string(),
        '\r' => "\\r".to_string(),
        '\t' => "\\t".to_string(),
        '\\' => "\\\\".to_string(),
        '\'' => "\\'".to_string(),
        '\"' => "\\\"".to_string(),
        '\0' => "\\0".to_string(),
        // 制御文字や非表示文字
        c if c < ' ' => format!("\\u{{{:04X}}}", c as u32),
        // 通常の表示可能文字
        _ => c.to_string(),
    }
}

/// 文字列内のエスケープシーケンスを解析する
///
/// エスケープシーケンスを実際の文字に変換する
pub fn unescape_char(s: &str) -> Result<char, String> {
    if s.len() == 1 {
        return Ok(s.chars().next().unwrap());
    }

    if !s.starts_with('\\') {
        return Err(format!("エスケープシーケンスは\\で始まる必要があります: {}", s));
    }

    let escaped = &s[1..];
    match escaped {
        "n" => Ok('\n'),
        "r" => Ok('\r'),
        "t" => Ok('\t'),
        "\\" => Ok('\\'),
        "'" => Ok('\''),
        "\"" => Ok('\"'),
        "0" => Ok('\0'),
        _ if escaped.starts_with('u') && escaped.len() >= 3 => {
            // Unicode エスケープシーケンス (\u{XXXX})
            if !escaped.starts_with("u{") || !escaped.ends_with('}') {
                return Err(format!("無効なUnicodeエスケープシーケンス: {}", s));
            }
            
            let hex_str = &escaped[2..escaped.len() - 1];
            match u32::from_str_radix(hex_str, 16) {
                Ok(code) => match char::try_from(code) {
                    Ok(c) => Ok(c),
                    Err(_) => Err(format!("無効なUnicodeコードポイント: {}", code))
                },
                Err(_) => Err(format!("無効な16進数: {}", hex_str))
            }
        },
        _ => Err(format!("未知のエスケープシーケンス: {}", s))
    }
}

/// 文字が有効なSwiftLight言語の演算子文字かどうかを判定
pub fn is_operator_char(c: char) -> bool {
    matches!(c, 
        '/' | '=' | '-' | '+' | '!' | '*' | '%' | '<' | '>' | '&' | 
        '|' | '^' | '~' | '?' | ':' | '.'
    )
}

/// テスト
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_identifier_chars() {
        // 識別子の先頭として有効な文字
        assert!(is_identifier_start('a'));
        assert!(is_identifier_start('Z'));
        assert!(is_identifier_start('_'));
        assert!(is_identifier_start('あ')); // 日本語
        assert!(is_identifier_start('ñ')); // アクセント付きラテン文字
        
        // 識別子の先頭として無効な文字
        assert!(!is_identifier_start('0'));
        assert!(!is_identifier_start('9'));
        assert!(!is_identifier_start('$'));
        assert!(!is_identifier_start('-'));
        
        // 識別子の2文字目以降として有効な文字
        assert!(is_identifier_continue('a'));
        assert!(is_identifier_continue('Z'));
        assert!(is_identifier_continue('_'));
        assert!(is_identifier_continue('0'));
        assert!(is_identifier_continue('9'));
        assert!(is_identifier_continue('あ'));
        
        // 識別子の2文字目以降として無効な文字
        assert!(!is_identifier_continue('$'));
        assert!(!is_identifier_continue('!'));
    }
    
    #[test]
    fn test_number_chars() {
        // 数値として有効な文字
        assert!(is_digit_start('0'));
        assert!(is_digit_start('9'));
        
        // 16進数として有効な文字
        assert!(is_hex_digit('0'));
        assert!(is_hex_digit('9'));
        assert!(is_hex_digit('a'));
        assert!(is_hex_digit('f'));
        assert!(is_hex_digit('A'));
        assert!(is_hex_digit('F'));
        
        // 16進数として無効な文字
        assert!(!is_hex_digit('g'));
        assert!(!is_hex_digit('G'));
        
        // 8進数として有効な文字
        assert!(is_octal_digit('0'));
        assert!(is_octal_digit('7'));
        
        // 8進数として無効な文字
        assert!(!is_octal_digit('8'));
        assert!(!is_octal_digit('9'));
        
        // 2進数として有効な文字
        assert!(is_binary_digit('0'));
        assert!(is_binary_digit('1'));
        
        // 2進数として無効な文字
        assert!(!is_binary_digit('2'));
    }
    
    #[test]
    fn test_escape_char() {
        assert_eq!(escape_char('\n'), "\\n");
        assert_eq!(escape_char('\t'), "\\t");
        assert_eq!(escape_char('\\'), "\\\\");
        assert_eq!(escape_char('\"'), "\\\"");
        assert_eq!(escape_char('a'), "a");
        assert_eq!(escape_char('あ'), "あ");
        assert_eq!(escape_char('\u{0007}'), "\\u{0007}"); // BEL制御文字
    }
    
    #[test]
    fn test_unescape_char() {
        assert_eq!(unescape_char("a").unwrap(), 'a');
        assert_eq!(unescape_char("\\n").unwrap(), '\n');
        assert_eq!(unescape_char("\\t").unwrap(), '\t');
        assert_eq!(unescape_char("\\\\").unwrap(), '\\');
        assert_eq!(unescape_char("\\'").unwrap(), '\'');
        assert_eq!(unescape_char("\\\"").unwrap(), '\"');
        assert_eq!(unescape_char("\\0").unwrap(), '\0');
        assert_eq!(unescape_char("\\u{3042}").unwrap(), 'あ');
        
        // エラーケース
        assert!(unescape_char("\\x").is_err());
        assert!(unescape_char("\\u").is_err());
        assert!(unescape_char("\\u{ZZZZ}").is_err());
        assert!(unescape_char("\\u{D800}").is_err()); // サロゲートペア
    }
    
    #[test]
    fn test_operator_chars() {
        // 演算子として有効な文字
        assert!(is_operator_char('+'));
        assert!(is_operator_char('-'));
        assert!(is_operator_char('*'));
        assert!(is_operator_char('/'));
        assert!(is_operator_char('%'));
        assert!(is_operator_char('='));
        assert!(is_operator_char('<'));
        assert!(is_operator_char('>'));
        assert!(is_operator_char('!'));
        assert!(is_operator_char('&'));
        assert!(is_operator_char('|'));
        assert!(is_operator_char('^'));
        assert!(is_operator_char('~'));
        assert!(is_operator_char('?'));
        assert!(is_operator_char(':'));
        assert!(is_operator_char('.'));
        
        // 演算子として無効な文字
        assert!(!is_operator_char('a'));
        assert!(!is_operator_char('0'));
        assert!(!is_operator_char('_'));
        assert!(!is_operator_char('@'));
        assert!(!is_operator_char('#'));
    }
    
    #[test]
    fn test_whitespace() {
        // 空白文字
        assert!(is_whitespace(' '));
        assert!(is_whitespace('\t'));
        assert!(is_whitespace('\n'));
        assert!(is_whitespace('\r'));
        assert!(is_whitespace('\u{3000}')); // 全角空白
        
        // 空白でない文字
        assert!(!is_whitespace('a'));
        assert!(!is_whitespace('0'));
        assert!(!is_whitespace('_'));
    }
}
