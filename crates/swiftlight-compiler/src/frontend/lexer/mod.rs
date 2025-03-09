//! # 字句解析器
//! 
//! SwiftLight言語のソースコードを解析し、トークンの列に変換する字句解析器を提供します。
//! 字句解析はコンパイラフロントエンドの最初の段階として、ソースコードを構文解析可能な
//! トークンに分割します。

use std::iter::Peekable;
use std::str::Chars;

use crate::frontend::error::{CompilerError, ErrorKind, Result, SourceLocation};
pub use self::token::{Token, TokenKind};
use self::unicode;

pub mod token;
pub mod unicode;

/// SwiftLight言語の字句解析器
pub struct Lexer<'a> {
    /// ソースコード
    source: &'a str,
    /// ファイル名
    file_name: String,
    /// 文字イテレータ
    chars: Peekable<Chars<'a>>,
    /// 現在の文字位置
    position: usize,
    /// 現在の行番号（1から始まる）
    line: usize,
    /// 現在の列番号（1から始まる）
    column: usize,
    /// 各行の開始位置（バイトオフセット）
    line_starts: Vec<usize>,
    /// 現在のトークンの開始位置
    start_position: usize,
    /// 現在のトークンの開始行
    start_line: usize,
    /// 現在のトークンの開始列
    start_column: usize,
}

impl<'a> Lexer<'a> {
    /// 新しい字句解析器を作成
    pub fn new(source: &'a str, file_name: &str) -> Self {
        // 行の開始位置を計算
        let mut line_starts = vec![0]; // 最初の行は0から始まる
        for (i, c) in source.char_indices() {
            if c == '\n' {
                line_starts.push(i + 1);
            }
        }
        
        Self {
            source,
            file_name: file_name.to_owned(),
            chars: source.chars().peekable(),
            position: 0,
            line: 1,
            column: 1,
            line_starts,
            start_position: 0,
            start_line: 1,
            start_column: 1,
        }
    }
    
    /// 次のトークンを取得
    pub fn next_token(&mut self) -> Result<Token> {
        // 空白文字をスキップ
        self.skip_whitespace();
        
        // 現在位置を保存
        self.start_position = self.position;
        self.start_line = self.line;
        self.start_column = self.column;
        
        // ファイル終端に達した場合
        if self.is_at_end() {
            return Ok(self.make_token(TokenKind::Eof));
        }
        
        // 次の文字を取得
        let c = self.advance();
        
        // 文字種別に応じたトークン生成
        match c {
            // 識別子または予約語
            c if unicode::is_identifier_start(c) => self.identifier(c),
            
            // 数値リテラル
            c if unicode::is_digit_start(c) => self.number(c),
            
            // 文字列リテラル
            '"' => self.string(),
            
            // 文字リテラル
            '\'' => self.char_literal(),
            
            // 記号と演算子
            '+' => self.match_char('=', TokenKind::PlusEqual, TokenKind::Plus),
            '-' => self.match_two_chars('=', TokenKind::MinusEqual, '>', TokenKind::Arrow, TokenKind::Minus),
            '*' => self.match_two_chars('=', TokenKind::StarEqual, '*', TokenKind::DoubleStar, TokenKind::Star),
            '/' => {
                if self.match_char('/', true) {
                    // 行コメント
                    self.line_comment()
                } else if self.match_char('*', true) {
                    // ブロックコメント
                    self.block_comment()
                } else {
                    self.match_char('=', TokenKind::SlashEqual, TokenKind::Slash)
                }
            },
            '%' => self.match_char('=', TokenKind::PercentEqual, TokenKind::Percent),
            
            '=' => self.match_two_chars('=', TokenKind::EqualEqual, '>', TokenKind::FatArrow, TokenKind::Equal),
            '!' => self.match_char('=', TokenKind::BangEqual, TokenKind::Bang),
            '<' => self.match_three_chars('=', TokenKind::LessEqual, '<', TokenKind::LessLess, '<', TokenKind::LessLess, TokenKind::Less),
            '>' => self.match_three_chars('=', TokenKind::GreaterEqual, '>', TokenKind::GreaterGreater, '>', TokenKind::GreaterGreater, TokenKind::Greater),
            
            '&' => self.match_two_chars('&', TokenKind::AmpersandAmpersand, '=', TokenKind::AmpersandEqual, TokenKind::Ampersand),
            '|' => self.match_two_chars('|', TokenKind::PipePipe, '=', TokenKind::PipeEqual, TokenKind::Pipe),
            '^' => self.match_char('=', TokenKind::CaretEqual, TokenKind::Caret),
            '~' => Ok(self.make_token(TokenKind::Tilde)),
            
            // 区切り記号
            ';' => Ok(self.make_token(TokenKind::Semicolon)),
            ':' => self.match_char(':', TokenKind::DoubleColon, TokenKind::Colon),
            ',' => Ok(self.make_token(TokenKind::Comma)),
            '.' => self.dot_token(),
            '?' => Ok(self.make_token(TokenKind::Question)),
            
            // 括弧
            '(' => Ok(self.make_token(TokenKind::LeftParen)),
            ')' => Ok(self.make_token(TokenKind::RightParen)),
            '{' => Ok(self.make_token(TokenKind::LeftBrace)),
            '}' => Ok(self.make_token(TokenKind::RightBrace)),
            '[' => Ok(self.make_token(TokenKind::LeftBracket)),
            ']' => Ok(self.make_token(TokenKind::RightBracket)),
            
            // 不明な文字
            c => {
                let error = CompilerError::lexical_error(
                    format!("不明な文字です: '{}'", unicode::escape_char(c)),
                    Some(self.current_location()),
                );
                
                Ok(self.make_token(TokenKind::Unknown(c)))
            }
        }
    }
    
    /// ドット（.）から始まるトークンを処理
    fn dot_token(&mut self) -> Result<Token> {
        if self.match_char('.', true) {
            if self.match_char('.', true) {
                // 省略記号 ...
                Ok(self.make_token(TokenKind::Ellipsis))
            } else {
                // 範囲演算子 .. は現在サポート外
                let error = CompilerError::lexical_error(
                    "範囲演算子はサポートされていません",
                    Some(self.current_location()),
                );
                Ok(self.make_token(TokenKind::Unknown('.')))
            }
        } else {
            // 単なるドット .
            Ok(self.make_token(TokenKind::Dot))
        }
    }
    
    /// 識別子または予約語の処理
    fn identifier(&mut self, first: char) -> Result<Token> {
        let mut ident = first.to_string();
        
        // 識別子の2文字目以降を読み込む
        while let Some(&c) = self.chars.peek() {
            if !unicode::is_identifier_continue(c) {
                break;
            }
            
            ident.push(c);
            self.advance();
        }
        
        // 予約語かどうかチェック
        let kind = TokenKind::from_keyword(&ident);
        Ok(self.make_token(kind))
    }
    
    /// 数値リテラルの処理
    fn number(&mut self, first: char) -> Result<Token> {
        let mut num = first.to_string();
        let mut is_float = false;
        
        // 接頭辞による基数の判定
        if first == '0' && self.chars.peek().copied() == Some('x') {
            // 16進数
            num.push('x');
            self.advance(); // 'x'を消費
            
            // 16進数の桁を読み込む
            while let Some(&c) = self.chars.peek() {
                if !unicode::is_hex_digit(c) {
                    break;
                }
                
                num.push(c);
                self.advance();
            }
            
            return Ok(self.make_token(TokenKind::IntLiteral(num)));
        } else if first == '0' && self.chars.peek().copied() == Some('o') {
            // 8進数
            num.push('o');
            self.advance(); // 'o'を消費
            
            // 8進数の桁を読み込む
            while let Some(&c) = self.chars.peek() {
                if !unicode::is_octal_digit(c) {
                    break;
                }
                
                num.push(c);
                self.advance();
            }
            
            return Ok(self.make_token(TokenKind::IntLiteral(num)));
        } else if first == '0' && self.chars.peek().copied() == Some('b') {
            // 2進数
            num.push('b');
            self.advance(); // 'b'を消費
            
            // 2進数の桁を読み込む
            while let Some(&c) = self.chars.peek() {
                if !unicode::is_binary_digit(c) {
                    break;
                }
                
                num.push(c);
                self.advance();
            }
            
            return Ok(self.make_token(TokenKind::IntLiteral(num)));
        }
        
        // 10進数の処理
        while let Some(&c) = self.chars.peek() {
            if c.is_ascii_digit() {
                // 数字
                num.push(c);
                self.advance();
            } else if c == '.' && !is_float {
                // 小数点（初回のみ）
                if let Some(&next) = self.chars.clone().nth(1) {
                    if next.is_ascii_digit() {
                        num.push(c);
                        self.advance();
                        is_float = true;
                    } else {
                        // 次が数字でない場合は小数点として扱わない（ドット演算子など）
                        break;
                    }
                } else {
                    // 小数点の後に文字がない場合
                    break;
                }
            } else if c == 'e' || c == 'E' {
                // 指数表記
                let mut peek = self.chars.clone();
                peek.next(); // 'e'または'E'をスキップ
                
                let has_sign = peek.next().map_or(false, |c| c == '+' || c == '-');
                let has_digit = if has_sign {
                    peek.next().map_or(false, |c| c.is_ascii_digit())
                } else {
                    peek.next().map_or(false, |c| c.is_ascii_digit())
                };
                
                if has_digit {
                    // 有効な指数表記
                    num.push(c);
                    self.advance(); // 'e'または'E'を消費
                    
                    if has_sign {
                        num.push(self.advance()); // '+'または'-'を消費
                    }
                    
                    // 指数の数字を読み込む
                    while let Some(&c) = self.chars.peek() {
                        if !c.is_ascii_digit() {
                            break;
                        }
                        
                        num.push(c);
                        self.advance();
                    }
                    
                    is_float = true;
                } else {
                    // 無効な指数表記
                    break;
                }
            } else {
                // 数値以外の文字
                break;
            }
        }
        
        if is_float {
            Ok(self.make_token(TokenKind::FloatLiteral(num)))
        } else {
            Ok(self.make_token(TokenKind::IntLiteral(num)))
        }
    }
    
    /// 文字列リテラルの処理
    fn string(&mut self) -> Result<Token> {
        let mut value = String::new();
        let mut terminated = false;
        
        while let Some(&c) = self.chars.peek() {
            if c == '"' {
                self.advance(); // 閉じ引用符を消費
                terminated = true;
                break;
            } else if c == '\\' {
                // エスケープシーケンス
                self.advance(); // バックスラッシュを消費
                
                if let Some(escaped) = self.escape_sequence() {
                    value.push(escaped);
                } else {
                    // 無効なエスケープシーケンス
                    let error = CompilerError::lexical_error(
                        "無効なエスケープシーケンスです",
                        Some(self.current_location()),
                    );
                    value.push('\\');
                }
            } else if c == '\n' {
                // 改行を含む文字列はエラー
                let error = CompilerError::lexical_error(
                    "文字列リテラル内に改行があります",
                    Some(self.current_location()),
                );
                break;
            } else {
                // 通常の文字
                value.push(c);
                self.advance();
            }
        }
        
        if !terminated {
            let error = CompilerError::lexical_error(
                "文字列リテラルが終了していません",
                Some(self.current_location()),
            );
        }
        
        Ok(self.make_token(TokenKind::StringLiteral(value)))
    }
    
    /// 文字リテラルの処理
    fn char_literal(&mut self) -> Result<Token> {
        let c = if let Some(&c) = self.chars.peek() {
            if c == '\\' {
                // エスケープシーケンス
                self.advance(); // バックスラッシュを消費
                
                if let Some(escaped) = self.escape_sequence() {
                    escaped
                } else {
                    // 無効なエスケープシーケンス
                    let error = CompilerError::lexical_error(
                        "無効なエスケープシーケンスです",
                        Some(self.current_location()),
                    );
                    '?'
                }
            } else {
                // 通常の文字
                self.advance()
            }
        } else {
            let error = CompilerError::lexical_error(
                "文字リテラルが空です",
                Some(self.current_location()),
            );
            '?'
        };
        
        // 閉じ引用符を期待
        if !self.match_char('\'', true) {
            let error = CompilerError::lexical_error(
                "文字リテラルが終了していません",
                Some(self.current_location()),
            );
        }
        
        Ok(self.make_token(TokenKind::CharLiteral(c)))
    }
    
    /// 行コメントの処理
    fn line_comment(&mut self) -> Result<Token> {
        // 行末まで読み飛ばす
        while let Some(&c) = self.chars.peek() {
            if c == '\n' {
                break;
            }
            self.advance();
        }
        
        Ok(self.make_token(TokenKind::Comment))
    }
    
    /// ブロックコメントの処理
    fn block_comment(&mut self) -> Result<Token> {
        let mut terminated = false;
        let mut nesting = 1; // ネストレベル
        
        while let Some(&c) = self.chars.peek() {
            if c == '*' {
                self.advance();
                
                if let Some(&next) = self.chars.peek() {
                    if next == '/' {
                        self.advance();
                        nesting -= 1;
                        
                        if nesting == 0 {
                            terminated = true;
                            break;
                        }
                    }
                }
            } else if c == '/' {
                self.advance();
                
                if let Some(&next) = self.chars.peek() {
                    if next == '*' {
                        self.advance();
                        nesting += 1;
                    }
                }
            } else {
                self.advance();
            }
        }
        
        if !terminated {
            let error = CompilerError::lexical_error(
                "ブロックコメントが終了していません",
                Some(self.current_location()),
            );
        }
        
        Ok(self.make_token(TokenKind::Comment))
    }
    
    /// エスケープシーケンスの処理
    fn escape_sequence(&mut self) -> Option<char> {
        if let Some(&c) = self.chars.peek() {
            let escaped = match c {
                'n' => '\n',
                'r' => '\r',
                't' => '\t',
                '\\' => '\\',
                '\'' => '\'',
                '"' => '"',
                '0' => '\0',
                'u' => {
                    // Unicode文字（\u{XXXX}形式）
                    self.advance(); // 'u'を消費
                    
                    if !self.match_char('{', true) {
                        return None;
                    }
                    
                    let mut hex = String::new();
                    while let Some(&c) = self.chars.peek() {
                        if c == '}' {
                            self.advance(); // '}'を消費
                            break;
                        } else if unicode::is_hex_digit(c) {
                            hex.push(c);
                            self.advance();
                        } else {
                            return None;
                        }
                    }
                    
                    if let Ok(code) = u32::from_str_radix(&hex, 16) {
                        if let Some(c) = std::char::from_u32(code) {
                            return Some(c);
                        }
                    }
                    
                    return None;
                }
                _ => return None,
            };
            
            self.advance(); // エスケープ文字を消費
            Some(escaped)
        } else {
            None
        }
    }
    
    /// 次の文字が期待する文字と一致するかチェック
    fn match_char(&mut self, expected: char, match_kind: TokenKind, else_kind: TokenKind) -> Result<Token> {
        if self.match_char(expected, true) {
            Ok(self.make_token(match_kind))
        } else {
            Ok(self.make_token(else_kind))
        }
    }
    
    /// 次の文字が期待する2つの文字のいずれかと一致するかチェック
    fn match_two_chars(
        &mut self,
        expected1: char,
        match_kind1: TokenKind,
        expected2: char,
        match_kind2: TokenKind,
        else_kind: TokenKind,
    ) -> Result<Token> {
        if self.match_char(expected1, true) {
            Ok(self.make_token(match_kind1))
        } else if self.match_char(expected2, true) {
            Ok(self.make_token(match_kind2))
        } else {
            Ok(self.make_token(else_kind))
        }
    }
    
    /// 次の文字が期待する3つの文字のいずれかと一致するかチェック
    fn match_three_chars(
        &mut self,
        expected1: char,
        match_kind1: TokenKind,
        expected2: char,
        match_kind2: TokenKind,
        expected3: char,
        match_kind3: TokenKind,
        else_kind: TokenKind,
    ) -> Result<Token> {
        if self.match_char(expected1, true) {
            Ok(self.make_token(match_kind1))
        } else if self.match_char(expected2, true) {
            // expected2が続く場合は expected3 もチェック
            if self.match_char(expected3, true) {
                Ok(self.make_token(match_kind3))
            } else {
                Ok(self.make_token(match_kind2))
            }
        } else {
            Ok(self.make_token(else_kind))
        }
    }
    
    /// 次の文字が期待する文字と一致するかチェック
    fn match_char(&mut self, expected: char, consume: bool) -> bool {
        if let Some(&c) = self.chars.peek() {
            if c == expected {
                if consume {
                    self.advance();
                }
                return true;
            }
        }
        false
    }
    
    /// 空白文字をスキップ
    fn skip_whitespace(&mut self) {
        while let Some(&c) = self.chars.peek() {
            if unicode::is_whitespace(c) {
                self.advance();
            } else {
                break;
            }
        }
    }
    
    /// 次の文字を取得して位置を進める
    fn advance(&mut self) -> char {
        if let Some(c) = self.chars.next() {
            self.position += c.len_utf8();
            
            if c == '\n' {
                self.line += 1;
                self.column = 1;
            } else {
                self.column += 1;
            }
            
            c
        } else {
            '\0'
        }
    }
    
    /// ファイル終端に達したかどうかを判定
    fn is_at_end(&self) -> bool {
        self.chars.peek().is_none()
    }
    
    /// 現在のトークンの位置情報を取得
    fn current_location(&self) -> SourceLocation {
        SourceLocation::new(
            &self.file_name,
            self.start_line,
            self.start_column,
            self.start_position,
            self.position,
        )
    }
    
    /// トークンを作成
    fn make_token(&self, kind: TokenKind) -> Token {
        let location = self.current_location();
        let lexeme = if let TokenKind::Eof = kind {
            "".to_string()
        } else {
            self.source[self.start_position..self.position].to_string()
        };
        
        Token::new(kind, location, lexeme)
    }
}

impl<'a> Iterator for Lexer<'a> {
    type Item = Result<Token>;
    
    fn next(&mut self) -> Option<Self::Item> {
        match self.next_token() {
            Ok(token) if token.kind == TokenKind::Eof => None,
            Ok(token) => Some(Ok(token)),
            Err(e) => Some(Err(e)),
        }
    }
}

/// トークン化
/// 
/// 与えられたソースコードをトークンに分割
pub fn tokenize(source: &str, file_name: &str) -> Result<Vec<Token>> {
    let lexer = Lexer::new(source, file_name);
    lexer.collect()
}

/// テスト
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_tokenize_keywords() {
        let source = "let func if else while";
        let tokens = tokenize(source, "test.swl").unwrap();
        
        assert_eq!(tokens.len(), 5);
        assert_eq!(tokens[0].kind, TokenKind::Let);
        assert_eq!(tokens[1].kind, TokenKind::Func);
        assert_eq!(tokens[2].kind, TokenKind::If);
        assert_eq!(tokens[3].kind, TokenKind::Else);
        assert_eq!(tokens[4].kind, TokenKind::While);
    }
    
    #[test]
    fn test_tokenize_operators() {
        let source = "+ - * / % == != < > <= >= && ||";
        let tokens = tokenize(source, "test.swl").unwrap();
        
        assert_eq!(tokens.len(), 13);
        assert_eq!(tokens[0].kind, TokenKind::Plus);
        assert_eq!(tokens[1].kind, TokenKind::Minus);
        assert_eq!(tokens[2].kind, TokenKind::Star);
        assert_eq!(tokens[3].kind, TokenKind::Slash);
        assert_eq!(tokens[4].kind, TokenKind::Percent);
        assert_eq!(tokens[5].kind, TokenKind::EqualEqual);
        assert_eq!(tokens[6].kind, TokenKind::BangEqual);
        assert_eq!(tokens[7].kind, TokenKind::Less);
        assert_eq!(tokens[8].kind, TokenKind::Greater);
        assert_eq!(tokens[9].kind, TokenKind::LessEqual);
        assert_eq!(tokens[10].kind, TokenKind::GreaterEqual);
        assert_eq!(tokens[11].kind, TokenKind::AmpersandAmpersand);
        assert_eq!(tokens[12].kind, TokenKind::PipePipe);
    }
    
    #[test]
    fn test_tokenize_literals() {
        let source = r#"42 3.14 "hello" 'c'"#;
        let tokens = tokenize(source, "test.swl").unwrap();
        
        assert_eq!(tokens.len(), 4);
        assert!(matches!(tokens[0].kind, TokenKind::IntLiteral(ref s) if s == "42"));
        assert!(matches!(tokens[1].kind, TokenKind::FloatLiteral(ref s) if s == "3.14"));
        assert!(matches!(tokens[2].kind, TokenKind::StringLiteral(ref s) if s == "hello"));
        assert!(matches!(tokens[3].kind, TokenKind::CharLiteral(c) if c == 'c'));
    }
    
    #[test]
    fn test_tokenize_identifiers() {
        let source = "foo bar baz";
        let tokens = tokenize(source, "test.swl").unwrap();
        
        assert_eq!(tokens.len(), 3);
        assert!(matches!(tokens[0].kind, TokenKind::Identifier(ref s) if s == "foo"));
        assert!(matches!(tokens[1].kind, TokenKind::Identifier(ref s) if s == "bar"));
        assert!(matches!(tokens[2].kind, TokenKind::Identifier(ref s) if s == "baz"));
    }
    
    #[test]
    fn test_tokenize_comments() {
        let source = "let x = 5; // This is a comment\n/* Block comment */ let y = 10;";
        let tokens = tokenize(source, "test.swl").unwrap();
        
        // コメントはスキップされないので含まれる
        assert_eq!(tokens.len(), 11);
        assert_eq!(tokens[0].kind, TokenKind::Let);
        assert!(matches!(tokens[1].kind, TokenKind::Identifier(ref s) if s == "x"));
        assert_eq!(tokens[2].kind, TokenKind::Equal);
        assert!(matches!(tokens[3].kind, TokenKind::IntLiteral(ref s) if s == "5"));
        assert_eq!(tokens[4].kind, TokenKind::Semicolon);
        assert_eq!(tokens[5].kind, TokenKind::Comment);
        assert_eq!(tokens[6].kind, TokenKind::Comment);
        assert_eq!(tokens[7].kind, TokenKind::Let);
        assert!(matches!(tokens[8].kind, TokenKind::Identifier(ref s) if s == "y"));
        assert_eq!(tokens[9].kind, TokenKind::Equal);
        assert!(matches!(tokens[10].kind, TokenKind::IntLiteral(ref s) if s == "10"));
    }
    
    #[test]
    fn test_tokenize_complex_source() {
        let source = r#"
            func factorial(n: Int) -> Int {
                if n <= 1 {
                    return 1;
                } else {
                    return n * factorial(n - 1);
                }
            }
        "#;
        
        let tokens = tokenize(source, "test.swl").unwrap();
        
        // トークン数を検証
        assert!(tokens.len() > 20); // 正確な数ではなく十分な数があるか
        
        // いくつかの重要なトークンを検証
        let kinds: Vec<_> = tokens.iter().map(|t| &t.kind).collect();
        
        assert!(kinds.contains(&&TokenKind::Func));
        assert!(kinds.contains(&&TokenKind::If));
        assert!(kinds.contains(&&TokenKind::Else));
        assert!(kinds.contains(&&TokenKind::Return));
        assert!(kinds.contains(&&TokenKind::Arrow));
        
        // 識別子 "factorial" が含まれているか検証
        let has_factorial = tokens.iter().any(|t| matches!(&t.kind, TokenKind::Identifier(ref s) if s == "factorial"));
        assert!(has_factorial);
    }
}
