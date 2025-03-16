//! # レキサー（字句解析器）
//! 
//! SwiftLight言語のソースコードを字句解析し、トークン列に変換するモジュールです。

use std::iter::{Iterator, Peekable};
use std::str::Chars;

use crate::frontend::error::{CompilerError, Result, SourceLocation};
use token::Token;
use token::TokenKind;

pub mod token;

/// レキサー
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
    /// 新しいレキサーを作成
    pub fn new(source: &'a str, file_name: &str) -> Self {
        let mut line_starts = Vec::new();
        line_starts.push(0); // 最初の行の開始位置
        
        // 行の開始位置を計算
        for (i, c) in source.char_indices() {
            if c == '\n' {
                line_starts.push(i + 1);
            }
        }
        
        Self {
            source,
            file_name: file_name.to_string(),
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
        // 現在位置を保存
        self.start_position = self.position;
        self.start_line = self.line;
        self.start_column = self.column;
        
        // 空白文字をスキップ
        self.skip_whitespace();
        
        // ファイル終端チェック
        if self.is_at_end() {
            return Ok(self.make_token(TokenKind::EOF));
        }
        
        // 文字を取得して、トークン解析
        let c = self.advance();
        
        // 識別子、キーワード
        if is_alpha(c) {
            return self.identifier(c);
        }
        
        // 数値
        if is_digit(c) {
            return self.number(c);
        }
        
        // 各種トークン解析
        match c {
            // 単一文字トークン
            '(' => Ok(self.make_token(TokenKind::LeftParen)),
            ')' => Ok(self.make_token(TokenKind::RightParen)),
            '{' => Ok(self.make_token(TokenKind::LeftBrace)),
            '}' => Ok(self.make_token(TokenKind::RightBrace)),
            '[' => Ok(self.make_token(TokenKind::LeftBracket)),
            ']' => Ok(self.make_token(TokenKind::RightBracket)),
            ',' => Ok(self.make_token(TokenKind::Comma)),
            ';' => Ok(self.make_token(TokenKind::Semicolon)),
            ':' => Ok(self.make_token(TokenKind::Colon)),
            '@' => Ok(self.make_token(TokenKind::At)),
            
            // 複合トークン
            '.' => self.dot_token(),
            
            // 演算子
            '+' => {
                if self.match_char('+') {
                    Ok(self.make_token(TokenKind::PlusPlus))
                } else if self.match_char('=') {
                    Ok(self.make_token(TokenKind::PlusEqual))
                } else {
                    Ok(self.make_token(TokenKind::Plus))
                }
            },
            '-' => {
                if self.match_char('-') {
                    Ok(self.make_token(TokenKind::MinusMinus))
                } else if self.match_char('=') {
                    Ok(self.make_token(TokenKind::MinusEqual))
                } else if self.match_char('>') {
                    Ok(self.make_token(TokenKind::Arrow))
                } else {
                    Ok(self.make_token(TokenKind::Minus))
                }
            },
            '*' => {
                if self.match_char('=') {
                    Ok(self.make_token(TokenKind::StarEqual))
                } else {
                    Ok(self.make_token(TokenKind::Star))
                }
            },
            '/' => {
                if self.match_char('/') {
                    self.line_comment()
                } else if self.match_char('*') {
                    self.block_comment()
                } else if self.match_char('=') {
                    Ok(self.make_token(TokenKind::SlashEqual))
                } else {
                    Ok(self.make_token(TokenKind::Slash))
                }
            },
            '%' => {
                if self.match_char('=') {
                    Ok(self.make_token(TokenKind::PercentEqual))
                } else {
                    Ok(self.make_token(TokenKind::Percent))
                }
            },
            
            // 比較演算子
            '=' => {
                if self.match_char('=') {
                    Ok(self.make_token(TokenKind::EqualEqual))
                } else if self.match_char('>') {
                    Ok(self.make_token(TokenKind::FatArrow))
                } else {
                    Ok(self.make_token(TokenKind::Equal))
                }
            },
            '!' => {
                if self.match_char('=') {
                    Ok(self.make_token(TokenKind::BangEqual))
                } else {
                    Ok(self.make_token(TokenKind::Bang))
                }
            },
            '<' => {
                if self.match_char('=') {
                    Ok(self.make_token(TokenKind::LessEqual))
                } else if self.match_char('<') {
                    if self.match_char('=') {
                        Ok(self.make_token(TokenKind::LeftShiftEqual))
                    } else {
                        Ok(self.make_token(TokenKind::LeftShift))
                    }
                } else {
                    Ok(self.make_token(TokenKind::Less))
                }
            },
            '>' => {
                if self.match_char('=') {
                    Ok(self.make_token(TokenKind::GreaterEqual))
                } else if self.match_char('>') {
                    if self.match_char('=') {
                        Ok(self.make_token(TokenKind::RightShiftEqual))
                    } else {
                        Ok(self.make_token(TokenKind::RightShift))
                    }
                } else {
                    Ok(self.make_token(TokenKind::Greater))
                }
            },
            
            // ビット演算子
            '&' => {
                if self.match_char('&') {
                    Ok(self.make_token(TokenKind::AmpersandAmpersand))
                } else if self.match_char('=') {
                    Ok(self.make_token(TokenKind::AmpersandEqual))
                } else {
                    Ok(self.make_token(TokenKind::Ampersand))
                }
            },
            '|' => {
                if self.match_char('|') {
                    Ok(self.make_token(TokenKind::PipePipe))
                } else if self.match_char('=') {
                    Ok(self.make_token(TokenKind::PipeEqual))
                } else {
                    Ok(self.make_token(TokenKind::Pipe))
                }
            },
            '^' => {
                if self.match_char('=') {
                    Ok(self.make_token(TokenKind::CaretEqual))
                } else {
                    Ok(self.make_token(TokenKind::Caret))
                }
            },
            '~' => Ok(self.make_token(TokenKind::Tilde)),
            
            // 特殊文字
            '?' => Ok(self.make_token(TokenKind::QuestionMark)),
            
            // 文字列、文字リテラル
            '"' => self.string(),
            '\'' => self.char_literal(),
            
            // 不明な文字
            _ => Err(CompilerError::lexical_error(
                format!("不明な文字です: '{}'", c),
                self.current_location(),
            )),
        }
    }
    
    /// ドット関連のトークン（., .., ..=）
    fn dot_token(&mut self) -> Result<Token> {
        if self.match_char('.') {
            if self.match_char('=') {
                Ok(self.make_token(TokenKind::RangeInclusive))
            } else {
                Ok(self.make_token(TokenKind::Range))
            }
        } else {
            Ok(self.make_token(TokenKind::Dot))
        }
    }
    
    /// 識別子またはキーワードの解析
    fn identifier(&mut self, first: char) -> Result<Token> {
        let mut ident = first.to_string();
        
        // 2文字目以降を解析
        while let Some(&c) = self.chars.peek() {
            if is_alpha(c) || is_digit(c) || c == '_' {
                ident.push(self.advance());
            } else {
                break;
            }
        }
        
        // キーワードかどうかを判定
        let kind = match ident.as_str() {
            "let" => TokenKind::KeywordLet,
            "var" => TokenKind::KeywordVar,
            "const" => TokenKind::KeywordConst,
            "fn" => TokenKind::KeywordFn,
            "return" => TokenKind::KeywordReturn,
            "if" => TokenKind::KeywordIf,
            "else" => TokenKind::KeywordElse,
            "while" => TokenKind::KeywordWhile,
            "for" => TokenKind::KeywordFor,
            "in" => TokenKind::KeywordIn,
            "break" => TokenKind::KeywordBreak,
            "continue" => TokenKind::KeywordContinue,
            "struct" => TokenKind::KeywordStruct,
            "enum" => TokenKind::KeywordEnum,
            "trait" => TokenKind::KeywordTrait,
            "impl" => TokenKind::KeywordImpl,
            "type" => TokenKind::KeywordType,
            "true" => TokenKind::KeywordTrue,
            "false" => TokenKind::KeywordFalse,
            "nil" => TokenKind::KeywordNil,
            "self" => TokenKind::KeywordSelf,
            "super" => TokenKind::KeywordSuper,
            "pub" => TokenKind::KeywordPub,
            "as" => TokenKind::KeywordAs,
            "match" => TokenKind::KeywordMatch,
            "import" => TokenKind::KeywordImport,
            "module" => TokenKind::KeywordModule,
            "async" => TokenKind::KeywordAsync,
            "await" => TokenKind::KeywordAwait,
            "try" => TokenKind::KeywordTry,
            "catch" => TokenKind::KeywordCatch,
            "throw" => TokenKind::KeywordThrow,
            "mut" => TokenKind::KeywordMut,
            "unsafe" => TokenKind::KeywordUnsafe,
            "where" => TokenKind::KeywordWhere,
            _ => TokenKind::Identifier,
        };
        
        Ok(Token::new(kind, ident, self.current_location()))
    }
    
    /// 数値リテラルの解析
    fn number(&mut self, first: char) -> Result<Token> {
        let mut number = first.to_string();
        let mut is_float = false;
        
        // 整数部分
        while let Some(&c) = self.chars.peek() {
            if is_digit(c) {
                number.push(self.advance());
            } else if c == '_' {
                // 数値の可読性のためのアンダースコアはスキップ
                self.advance();
            } else {
                break;
            }
        }
        
        // 小数部分
        if let Some(&c) = self.chars.peek() {
            if c == '.' {
                // 次の文字もピークしてドットドット演算子（..）と区別
                if let Some(&next) = self.chars.clone().nth(1) {
                    if is_digit(next) {
                        is_float = true;
                        number.push(self.advance()); // '.' を追加
                        
                        // 小数点以下の数字を解析
                        while let Some(&c) = self.chars.peek() {
                            if is_digit(c) {
                                number.push(self.advance());
                            } else if c == '_' {
                                self.advance(); // アンダースコアはスキップ
                            } else {
                                break;
                            }
                        }
                    }
                }
            }
        }
        
        // 指数部分 (1e10, 1.5e-3 など)
        if let Some(&c) = self.chars.peek() {
            if c == 'e' || c == 'E' {
                let mut has_exponent = false;
                
                // 次の文字をチェック
                if let Some(&next) = self.chars.clone().nth(1) {
                    if is_digit(next) || next == '+' || next == '-' {
                        is_float = true;
                        has_exponent = true;
                        number.push(self.advance()); // 'e' または 'E' を追加
                        
                        // 符号があれば追加
                        if let Some(&c) = self.chars.peek() {
                            if c == '+' || c == '-' {
                                number.push(self.advance());
                            }
                        }
                        
                        // 指数の数字を解析
                        let mut has_digit = false;
                        while let Some(&c) = self.chars.peek() {
                            if is_digit(c) {
                                number.push(self.advance());
                                has_digit = true;
                            } else if c == '_' {
                                self.advance(); // アンダースコアはスキップ
                            } else {
                                break;
                            }
                        }
                        
                        // 指数部分に数字がない場合はエラー
                        if !has_digit {
                            return Err(CompilerError::lexical_error(
                                "指数部分に数字がありません",
                                self.current_location(),
                            ));
                        }
                    }
                }
                
                // 指数記法でない場合は、eは識別子の一部として扱う
                if !has_exponent {
                    // 後続の文字が識別子の一部であれば、数値リテラルは終了
                    return Ok(Token::new(
                        if is_float { TokenKind::Float } else { TokenKind::Integer },
                        number,
                        self.current_location(),
                    ));
                }
            }
        }
        
        // 型サフィックス (i32, f64 など)
        let mut has_suffix = false;
        if let Some(&c) = self.chars.peek() {
            if is_alpha(c) {
                // 型サフィックスの開始
                let suffix_start = self.position;
                
                // サフィックスを収集
                let mut suffix = String::new();
                while let Some(&c) = self.chars.peek() {
                    if is_alpha(c) || is_digit(c) {
                        suffix.push(self.advance());
                    } else {
                        break;
                    }
                }
                
                // 有効な型サフィックスかチェック
                match suffix.as_str() {
                    // 整数型サフィックス
                    "i8" | "i16" | "i32" | "i64" | "i128" |
                    "u8" | "u16" | "u32" | "u64" | "u128" => {
                        has_suffix = true;
                        // サフィックスは数値リテラルの一部として扱わない
                    },
                    // 浮動小数点型サフィックス
                    "f32" | "f64" => {
                        has_suffix = true;
                        is_float = true;
                        // サフィックスは数値リテラルの一部として扱わない
                    },
                    _ => {
                        // 無効なサフィックスの場合は、数値リテラルの終了とし、
                        // サフィックスは別の識別子として扱う
                        // 位置を戻す
                        self.position = suffix_start;
                    }
                }
            }
        }
        
        // トークンの種類を決定
        let kind = if is_float {
            TokenKind::FloatLiteral
        } else {
            TokenKind::IntLiteral
        };
        
        Ok(Token::new(kind, number, self.current_location()))
    }
    
    /// 文字列リテラルの解析
    fn string(&mut self) -> Result<Token> {
        let mut string = String::new();
        let mut raw_lexeme = String::from("\"");
        
        while let Some(&c) = self.chars.peek() {
            if c == '"' {
                // 終端の引用符
                raw_lexeme.push(self.advance());
                break;
            } else if c == '\\' {
                // エスケープシーケンス
                raw_lexeme.push(self.advance()); // バックスラッシュ
                
                if let Some(escaped) = self.escape_sequence() {
                    string.push(escaped);
                    // エスケープシーケンスの文字もraw_lexemeに追加
                    if let Some(&next) = self.chars.peek() {
                        raw_lexeme.push(next);
                    }
                } else {
                    return Err(CompilerError::lexical_error(
                        "無効なエスケープシーケンスです",
                        self.current_location(),
                    ));
                }
            } else if c == '\n' {
                // 改行は許可されない（マルチライン文字列は別途対応）
                return Err(CompilerError::lexical_error(
                    "文字列リテラル内で改行が検出されました",
                    self.current_location(),
                ));
            } else {
                // 通常の文字
                let ch = self.advance();
                string.push(ch);
                raw_lexeme.push(ch);
            }
        }
        
        // 文字列の終端チェック
        if self.is_at_end() {
            return Err(CompilerError::lexical_error(
                "文字列リテラルが閉じられていません",
                self.current_location(),
            ));
        }
        
        Ok(Token::new(TokenKind::StringLiteral, raw_lexeme, self.current_location()))
    }
    
    /// 文字リテラルの解析
    fn char_literal(&mut self) -> Result<Token> {
        let mut raw_lexeme = String::from("'");
        let mut ch = '\0';
        let mut count = 0;
        
        while let Some(&c) = self.chars.peek() {
            if c == '\'' {
                // 終端のクォート
                raw_lexeme.push(self.advance());
                break;
            } else if c == '\\' {
                // エスケープシーケンス
                raw_lexeme.push(self.advance()); // バックスラッシュ
                
                if let Some(escaped) = self.escape_sequence() {
                    ch = escaped;
                    count += 1;
                    // エスケープシーケンスの文字もraw_lexemeに追加
                    if let Some(&next) = self.chars.peek() {
                        raw_lexeme.push(next);
                    }
                } else {
                    return Err(CompilerError::lexical_error(
                        "無効なエスケープシーケンスです",
                        self.current_location(),
                    ));
                }
            } else if c == '\n' {
                // 改行は許可されない
                return Err(CompilerError::lexical_error(
                    "文字リテラル内で改行が検出されました",
                    self.current_location(),
                ));
            } else {
                // 通常の文字
                ch = self.advance();
                count += 1;
                raw_lexeme.push(ch);
            }
        }
        
        // 文字リテラルの終端チェック
        if self.is_at_end() {
            return Err(CompilerError::lexical_error(
                "文字リテラルが閉じられていません",
                self.current_location(),
            ));
        }
        
        // 文字リテラルは1文字のみ
        if count != 1 {
            return Err(CompilerError::lexical_error(
                format!("文字リテラルには1文字のみ指定できます（{}文字検出）", count),
                self.current_location(),
            ));
        }
        
        Ok(Token::new(TokenKind::CharLiteral, raw_lexeme, self.current_location()))
    }
    
    /// 行コメントの解析
    fn line_comment(&mut self) -> Result<Token> {
        let mut comment = String::from("//");
        
        // 行末または改行までの文字を消費
        while let Some(&c) = self.chars.peek() {
            if c == '\n' {
                break;
            } else {
                comment.push(self.advance());
            }
        }
        
        Ok(Token::new(TokenKind::Comment, comment, self.current_location()))
    }
    
    /// ブロックコメントの解析
    fn block_comment(&mut self) -> Result<Token> {
        let mut comment = String::from("/*");
        let mut nesting = 1;
        
        while nesting > 0 {
            if self.is_at_end() {
                return Err(CompilerError::lexical_error(
                    "ブロックコメントが閉じられていません",
                    self.current_location(),
                ));
            }
            
            let c = self.advance();
            comment.push(c);
            
            if c == '/' && self.match_char('*') {
                comment.push('*');
                nesting += 1;
            } else if c == '*' && self.match_char('/') {
                comment.push('/');
                nesting -= 1;
            }
        }
        
        Ok(Token::new(TokenKind::Comment, comment, self.current_location()))
    }
    
    /// エスケープシーケンスの解析
    fn escape_sequence(&mut self) -> Option<char> {
        if let Some(&c) = self.chars.peek() {
            let escaped = match c {
                'n' => '\n',   // 改行
                'r' => '\r',   // キャリッジリターン
                't' => '\t',   // タブ
                '\\' => '\\',  // バックスラッシュ
                '\'' => '\'',  // シングルクォート
                '"' => '"',    // ダブルクォート
                '0' => '\0',   // ヌル文字
                'u' => {
                    // Unicodeエスケープシーケンス
                    self.advance(); // 'u'を消費
                    
                    // 次の文字が'{'であることを確認
                    if !self.match_char('{') {
                        return None;
                    }
                    
                    // 16進数の桁を読み取る（最大6桁）
                    let mut hex = String::new();
                    for _ in 0..6 {
                        if let Some(&c) = self.chars.peek() {
                            if c == '}' {
                                break;
                            } else if c.is_digit(16) {
                                hex.push(self.advance());
                            } else {
                                return None;
                            }
                        } else {
                            return None;
                        }
                    }
                    
                    // 終了の'}'を確認
                    if !self.match_char('}') {
                        return None;
                    }
                    
                    // 16進数をUnicode値に変換
                    if let Ok(code) = u32::from_str_radix(&hex, 16) {
                        if let Some(unicode) = std::char::from_u32(code) {
                            return Some(unicode);
                        }
                    }
                    return None;
                },
                'x' => {
                    // 16進数エスケープシーケンス（2桁）
                    self.advance(); // 'x'を消費
                    
                    let mut hex = String::new();
                    for _ in 0..2 {
                        if let Some(&c) = self.chars.peek() {
                            if c.is_digit(16) {
                                hex.push(self.advance());
                            } else {
                                return None;
                            }
                        } else {
                            return None;
                        }
                    }
                    
                    // 16進数をUnicode値に変換
                    if let Ok(code) = u8::from_str_radix(&hex, 16) {
                        return Some(code as char);
                    }
                    return None;
                },
                _ => return None,
            };
            
            self.advance(); // エスケープ文字を消費
            Some(escaped)
        } else {
            None
        }
    }
    
    /// 指定した文字とマッチするか確認し、マッチすれば消費する
    fn match_char(&mut self, expected: char) -> bool {
        if let Some(&c) = self.chars.peek() {
            if c == expected {
                self.advance();
                return true;
            }
        }
        false
    }
    
    /// 空白文字をスキップ
    fn skip_whitespace(&mut self) {
        while let Some(&c) = self.chars.peek() {
            if c.is_whitespace() {
                self.advance();
            } else {
                break;
            }
        }
    }
    
    /// 次の文字を取得して進める
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
    
    /// ファイル終端かどうか
    fn is_at_end(&self) -> bool {
        self.chars.peek().is_none()
    }
    
    /// 現在の位置情報
    fn current_location(&self) -> SourceLocation {
        SourceLocation::new(
            &self.file_name,
            self.start_line,
            self.start_column,
            self.start_position,
            self.position - self.start_position,
        )
    }
    
    /// トークンを作成
    fn make_token(&self, kind: TokenKind) -> Token {
        let lexeme = self.source[self.start_position..self.position].to_string();
        Token::new(kind, lexeme, self.current_location())
    }
}

impl<'a> Iterator for Lexer<'a> {
    type Item = Result<Token>;
    
    fn next(&mut self) -> Option<Self::Item> {
        match self.next_token() {
            Ok(token) => {
                if token.kind == TokenKind::EOF {
                    None
                } else {
                    Some(Ok(token))
                }
            },
            Err(err) => Some(Err(err)),
        }
    }
}

/// ソースコードをトークン列に変換する
pub fn tokenize(source: &str, file_name: &str) -> Result<Vec<Token>> {
    let lexer = Lexer::new(source, file_name);
    lexer.collect()
}

// ユーティリティ関数

/// 文字がアルファベットまたはアンダースコアかどうか
fn is_alpha(c: char) -> bool {
    c.is_alphabetic() || c == '_'
}

/// 文字が数字かどうか
fn is_digit(c: char) -> bool {
    c.is_digit(10)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_tokenize_keywords() {
        let source = "let var const fn if else while";
        let tokens = tokenize(source, "test.swl").unwrap();
        
        assert_eq!(tokens.len(), 7);
        assert_eq!(tokens[0].kind, TokenKind::KeywordLet);
        assert_eq!(tokens[1].kind, TokenKind::KeywordVar);
        assert_eq!(tokens[2].kind, TokenKind::KeywordConst);
        assert_eq!(tokens[3].kind, TokenKind::KeywordFn);
        assert_eq!(tokens[4].kind, TokenKind::KeywordIf);
        assert_eq!(tokens[5].kind, TokenKind::KeywordElse);
        assert_eq!(tokens[6].kind, TokenKind::KeywordWhile);
    }
    
    #[test]
    fn test_tokenize_operators() {
        let source = "+ - * / % == != < > <= >= && || !";
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
        assert_eq!(tokens[12].kind, TokenKind::Bang);
    }
    
    #[test]
    fn test_tokenize_literals() {
        let source = r#"123 3.14 "hello" 'a' true false nil"#;
        let tokens = tokenize(source, "test.swl").unwrap();
        
        assert_eq!(tokens.len(), 7);
        assert_eq!(tokens[0].kind, TokenKind::IntLiteral);
        assert_eq!(tokens[1].kind, TokenKind::FloatLiteral);
        assert_eq!(tokens[2].kind, TokenKind::StringLiteral);
        assert_eq!(tokens[3].kind, TokenKind::CharLiteral);
        assert_eq!(tokens[4].kind, TokenKind::KeywordTrue);
        assert_eq!(tokens[5].kind, TokenKind::KeywordFalse);
        assert_eq!(tokens[6].kind, TokenKind::KeywordNil);
    }
    
    #[test]
    fn test_tokenize_identifiers() {
        let source = "foo bar baz snake_case camelCase";
        let tokens = tokenize(source, "test.swl").unwrap();
        
        assert_eq!(tokens.len(), 5);
        assert_eq!(tokens[0].kind, TokenKind::Identifier);
        assert_eq!(tokens[0].lexeme, "foo");
        assert_eq!(tokens[1].kind, TokenKind::Identifier);
        assert_eq!(tokens[1].lexeme, "bar");
        assert_eq!(tokens[2].kind, TokenKind::Identifier);
        assert_eq!(tokens[2].lexeme, "baz");
        assert_eq!(tokens[3].kind, TokenKind::Identifier);
        assert_eq!(tokens[3].lexeme, "snake_case");
        assert_eq!(tokens[4].kind, TokenKind::Identifier);
        assert_eq!(tokens[4].lexeme, "camelCase");
    }
    
    #[test]
    fn test_tokenize_comments() {
        let source = r#"// This is a line comment
/* This is a
   block comment */
let x = 5; // End of line comment"#;
        
        let tokens = tokenize(source, "test.swl").unwrap();
        
        assert_eq!(tokens.len(), 5);
        assert_eq!(tokens[0].kind, TokenKind::Comment);
        assert_eq!(tokens[1].kind, TokenKind::Comment);
        assert_eq!(tokens[2].kind, TokenKind::KeywordLet);
        assert_eq!(tokens[3].kind, TokenKind::Identifier);
        assert_eq!(tokens[3].lexeme, "x");
        assert_eq!(tokens[4].kind, TokenKind::Equal);
    }
    
    #[test]
    fn test_tokenize_with_locations() {
        let source = "let x = 5";
        let tokens = tokenize(source, "test.swl").unwrap();
        
        assert_eq!(tokens.len(), 4);
        assert_eq!(tokens[0].location.line, 1);
        assert_eq!(tokens[0].location.column, 1);
        assert_eq!(tokens[1].location.line, 1);
        assert_eq!(tokens[1].location.column, 5);
        assert_eq!(tokens[2].location.line, 1);
        assert_eq!(tokens[2].location.column, 7);
        assert_eq!(tokens[3].location.line, 1);
        assert_eq!(tokens[3].location.column, 9);
    }
    
    #[test]
    fn test_tokenize_complex_source() {
        let source = r#"
fn factorial(n: i32) -> i32 {
    if n <= 1 {
        return 1;
    } else {
        return n * factorial(n - 1);
    }
}

// Call the function
let result = factorial(5);
"#;
        
        let tokens = tokenize(source, "test.swl").unwrap();
        
        // 簡易的な確認
        assert!(tokens.len() > 20);
        
        // 重要なトークンをチェック
        let keywords = tokens.iter()
            .filter(|t| t.is_keyword())
            .count();
        assert!(keywords >= 5); // fn, if, return, else, let
        
        let identifiers = tokens.iter()
            .filter(|t| t.kind == TokenKind::Identifier)
            .count();
        assert!(identifiers >= 3); // factorial, n, result
        
        let operators = tokens.iter()
            .filter(|t| t.is_operator())
            .count();
        assert!(operators >= 4); // <=, *, -, =
    }
}
