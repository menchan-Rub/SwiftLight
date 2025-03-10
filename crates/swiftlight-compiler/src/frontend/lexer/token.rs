//! # トークン定義
//! 
//! SwiftLight言語の字句解析で使用するトークンを定義します。
//! 各トークンの種類と属性を提供します。

use std::fmt;
use crate::frontend::error::SourceLocation;

/// トークンの種類
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenKind {
    // 単一文字トークン
    LeftParen,      // (
    RightParen,     // )
    LeftBrace,      // {
    RightBrace,     // }
    LeftBracket,    // [
    RightBracket,   // ]
    Comma,          // ,
    Dot,            // .
    Semicolon,      // ;
    Colon,          // :
    At,             // @
    
    // 1〜2文字のトークン
    Plus,           // +
    PlusPlus,       // ++
    PlusEqual,      // +=
    Minus,          // -
    MinusMinus,     // --
    MinusEqual,     // -=
    Star,           // *
    StarEqual,      // *=
    Slash,          // /
    SlashEqual,     // /=
    Percent,        // %
    PercentEqual,   // %=
    
    // 比較演算子
    Equal,          // =
    EqualEqual,     // ==
    Bang,           // !
    BangEqual,      // !=
    Greater,        // >
    GreaterEqual,   // >=
    Less,           // <
    LessEqual,      // <=
    
    // ビット演算子
    Ampersand,      // &
    AmpersandEqual, // &=
    AmpersandAmpersand, // &&
    Pipe,           // |
    PipeEqual,      // |=
    PipePipe,       // ||
    Caret,          // ^
    CaretEqual,     // ^=
    Tilde,          // ~
    LeftShift,      // <<
    LeftShiftEqual, // <<=
    RightShift,     // >>
    RightShiftEqual,// >>=
    
    // 特殊演算子
    QuestionMark,   // ?
    Arrow,          // ->
    FatArrow,       // =>
    Range,          // ..
    RangeInclusive, // ..=
    
    // リテラル
    Identifier,     // 識別子
    StringLiteral,  // 文字列リテラル
    CharLiteral,    // 文字リテラル
    IntLiteral,     // 整数リテラル
    FloatLiteral,   // 浮動小数点リテラル
    
    // キーワード
    KeywordLet,     // let
    KeywordVar,     // var
    KeywordConst,   // const
    KeywordFn,      // fn
    KeywordReturn,  // return
    KeywordIf,      // if
    KeywordElse,    // else
    KeywordWhile,   // while
    KeywordFor,     // for
    KeywordIn,      // in
    KeywordBreak,   // break
    KeywordContinue,// continue
    KeywordStruct,  // struct
    KeywordEnum,    // enum
    KeywordTrait,   // trait
    KeywordImpl,    // impl
    KeywordType,    // type
    KeywordTrue,    // true
    KeywordFalse,   // false
    KeywordNil,     // nil
    KeywordSelf,    // self
    KeywordSuper,   // super
    KeywordPub,     // pub
    KeywordAs,      // as
    KeywordMatch,   // match
    KeywordImport,  // import
    KeywordModule,  // module
    KeywordAsync,   // async
    KeywordAwait,   // await
    KeywordTry,     // try
    KeywordCatch,   // catch
    KeywordThrow,   // throw
    KeywordMut,     // mut
    KeywordUnsafe,  // unsafe
    KeywordWhere,   // where
    
    // その他
    Comment,        // コメント
    Whitespace,     // 空白文字
    EOF,            // ファイル終端
    Error,          // エラー
}

impl TokenKind {
    /// キーワードの文字列からトークン種別を取得
    pub fn from_keyword(ident: &str) -> Self {
        match ident {
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
        }
    }
    
    /// トークンが識別子かどうかを判定
    pub fn is_identifier(&self) -> bool {
        matches!(self, TokenKind::Identifier)
    }
    
    /// トークンがリテラルかどうかを判定
    pub fn is_literal(&self) -> bool {
        matches!(
            self,
            TokenKind::IntLiteral
                | TokenKind::FloatLiteral
                | TokenKind::StringLiteral
                | TokenKind::CharLiteral
                | TokenKind::KeywordTrue
                | TokenKind::KeywordFalse
                | TokenKind::KeywordNil
        )
    }
    
    /// トークンがキーワードかどうかを判定
    pub fn is_keyword(&self) -> bool {
        matches!(
            self,
            TokenKind::KeywordLet
                | TokenKind::KeywordVar
                | TokenKind::KeywordConst
                | TokenKind::KeywordFn
                | TokenKind::KeywordReturn
                | TokenKind::KeywordIf
                | TokenKind::KeywordElse
                | TokenKind::KeywordWhile
                | TokenKind::KeywordFor
                | TokenKind::KeywordIn
                | TokenKind::KeywordBreak
                | TokenKind::KeywordContinue
                | TokenKind::KeywordStruct
                | TokenKind::KeywordEnum
                | TokenKind::KeywordTrait
                | TokenKind::KeywordImpl
                | TokenKind::KeywordType
                | TokenKind::KeywordTrue
                | TokenKind::KeywordFalse
                | TokenKind::KeywordNil
                | TokenKind::KeywordSelf
                | TokenKind::KeywordSuper
                | TokenKind::KeywordPub
                | TokenKind::KeywordAs
                | TokenKind::KeywordMatch
                | TokenKind::KeywordImport
                | TokenKind::KeywordModule
                | TokenKind::KeywordAsync
                | TokenKind::KeywordAwait
                | TokenKind::KeywordTry
                | TokenKind::KeywordCatch
                | TokenKind::KeywordThrow
                | TokenKind::KeywordMut
                | TokenKind::KeywordUnsafe
                | TokenKind::KeywordWhere
        )
    }
    
    /// トークンが演算子かどうかを判定
    pub fn is_operator(&self) -> bool {
        matches!(
            self,
            TokenKind::Plus
                | TokenKind::PlusPlus
                | TokenKind::PlusEqual
                | TokenKind::Minus
                | TokenKind::MinusMinus
                | TokenKind::MinusEqual
                | TokenKind::Star
                | TokenKind::StarEqual
                | TokenKind::Slash
                | TokenKind::SlashEqual
                | TokenKind::Percent
                | TokenKind::PercentEqual
                | TokenKind::Equal
                | TokenKind::EqualEqual
                | TokenKind::Bang
                | TokenKind::BangEqual
                | TokenKind::Greater
                | TokenKind::GreaterEqual
                | TokenKind::Less
                | TokenKind::LessEqual
                | TokenKind::Ampersand
                | TokenKind::AmpersandEqual
                | TokenKind::AmpersandAmpersand
                | TokenKind::Pipe
                | TokenKind::PipeEqual
                | TokenKind::PipePipe
                | TokenKind::Caret
                | TokenKind::CaretEqual
                | TokenKind::Tilde
                | TokenKind::LeftShift
                | TokenKind::LeftShiftEqual
                | TokenKind::RightShift
                | TokenKind::RightShiftEqual
                | TokenKind::Arrow
                | TokenKind::FatArrow
                | TokenKind::Range
                | TokenKind::RangeInclusive
        )
    }
}

impl fmt::Display for TokenKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use TokenKind::*;
        match self {
            // 単一文字トークン
            LeftParen => write!(f, "("),
            RightParen => write!(f, ")"),
            LeftBrace => write!(f, "{{"),
            RightBrace => write!(f, "}}"),
            LeftBracket => write!(f, "["),
            RightBracket => write!(f, "]"),
            Comma => write!(f, ","),
            Dot => write!(f, "."),
            Semicolon => write!(f, ";"),
            Colon => write!(f, ":"),
            At => write!(f, "@"),
            
            // 1〜2文字のトークン
            Plus => write!(f, "+"),
            PlusPlus => write!(f, "++"),
            PlusEqual => write!(f, "+="),
            Minus => write!(f, "-"),
            MinusMinus => write!(f, "--"),
            MinusEqual => write!(f, "-="),
            Star => write!(f, "*"),
            StarEqual => write!(f, "*="),
            Slash => write!(f, "/"),
            SlashEqual => write!(f, "/="),
            Percent => write!(f, "%"),
            PercentEqual => write!(f, "%="),
            
            // 比較演算子
            Equal => write!(f, "="),
            EqualEqual => write!(f, "=="),
            Bang => write!(f, "!"),
            BangEqual => write!(f, "!="),
            Greater => write!(f, ">"),
            GreaterEqual => write!(f, ">="),
            Less => write!(f, "<"),
            LessEqual => write!(f, "<="),
            
            // ビット演算子
            Ampersand => write!(f, "&"),
            AmpersandEqual => write!(f, "&="),
            AmpersandAmpersand => write!(f, "&&"),
            Pipe => write!(f, "|"),
            PipeEqual => write!(f, "|="),
            PipePipe => write!(f, "||"),
            Caret => write!(f, "^"),
            CaretEqual => write!(f, "^="),
            Tilde => write!(f, "~"),
            LeftShift => write!(f, "<<"),
            LeftShiftEqual => write!(f, "<<="),
            RightShift => write!(f, ">>"),
            RightShiftEqual => write!(f, ">>="),
            
            // 特殊演算子
            QuestionMark => write!(f, "?"),
            Arrow => write!(f, "->"),
            FatArrow => write!(f, "=>"),
            Range => write!(f, ".."),
            RangeInclusive => write!(f, "..="),
            
            // リテラル
            Identifier => write!(f, "識別子"),
            StringLiteral => write!(f, "文字列リテラル"),
            CharLiteral => write!(f, "文字リテラル"),
            IntLiteral => write!(f, "整数リテラル"),
            FloatLiteral => write!(f, "浮動小数点リテラル"),
            
            // キーワード
            KeywordLet => write!(f, "let"),
            KeywordVar => write!(f, "var"),
            KeywordConst => write!(f, "const"),
            KeywordFn => write!(f, "fn"),
            KeywordReturn => write!(f, "return"),
            KeywordIf => write!(f, "if"),
            KeywordElse => write!(f, "else"),
            KeywordWhile => write!(f, "while"),
            KeywordFor => write!(f, "for"),
            KeywordIn => write!(f, "in"),
            KeywordBreak => write!(f, "break"),
            KeywordContinue => write!(f, "continue"),
            KeywordStruct => write!(f, "struct"),
            KeywordEnum => write!(f, "enum"),
            KeywordTrait => write!(f, "trait"),
            KeywordImpl => write!(f, "impl"),
            KeywordType => write!(f, "type"),
            KeywordTrue => write!(f, "true"),
            KeywordFalse => write!(f, "false"),
            KeywordNil => write!(f, "nil"),
            KeywordSelf => write!(f, "self"),
            KeywordSuper => write!(f, "super"),
            KeywordPub => write!(f, "pub"),
            KeywordAs => write!(f, "as"),
            KeywordMatch => write!(f, "match"),
            KeywordImport => write!(f, "import"),
            KeywordModule => write!(f, "module"),
            KeywordAsync => write!(f, "async"),
            KeywordAwait => write!(f, "await"),
            KeywordTry => write!(f, "try"),
            KeywordCatch => write!(f, "catch"),
            KeywordThrow => write!(f, "throw"),
            KeywordMut => write!(f, "mut"),
            KeywordUnsafe => write!(f, "unsafe"),
            KeywordWhere => write!(f, "where"),
            
            // その他
            Comment => write!(f, "コメント"),
            Whitespace => write!(f, "空白"),
            EOF => write!(f, "EOF"),
            Error => write!(f, "エラー"),
        }
    }
}

/// トークン
#[derive(Debug, Clone)]
pub struct Token {
    /// トークンの種類
    pub kind: TokenKind,
    /// トークンの内容（レキシカル値）
    pub lexeme: String,
    /// ソースコード内の位置
    pub location: SourceLocation,
}

impl Token {
    /// 新しいトークンを作成
    pub fn new(kind: TokenKind, lexeme: String, location: SourceLocation) -> Self {
        Self {
            kind,
            lexeme,
            location,
        }
    }
    
    /// トークンが指定した種類かどうか判定
    pub fn is(&self, kind: &TokenKind) -> bool {
        self.kind == *kind
    }
    
    /// キーワードトークンかどうか判定
    pub fn is_keyword(&self) -> bool {
        self.kind.is_keyword()
    }
    
    /// 演算子トークンかどうか判定
    pub fn is_operator(&self) -> bool {
        self.kind.is_operator()
    }
    
    /// リテラルトークンかどうか判定
    pub fn is_literal(&self) -> bool {
        self.kind.is_literal()
    }
    
    /// 識別子トークンかどうか判定
    pub fn is_identifier(&self) -> bool {
        self.kind.is_identifier()
    }
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let token_type = match self.kind {
            TokenKind::Identifier => "識別子",
            TokenKind::StringLiteral | TokenKind::CharLiteral | 
            TokenKind::IntLiteral | TokenKind::FloatLiteral => "リテラル",
            _ if self.kind.is_keyword() => "キーワード",
            _ if self.kind.is_operator() => "演算子",
            _ => "トークン",
        };
        
        write!(f, "{} '{}' at {}:{}:{}", 
            token_type, 
            self.lexeme, 
            self.location.file, 
            self.location.line, 
            self.location.column
        )
    }
}

/// テスト
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_token_creation() {
        let location = SourceLocation::new("test.swl", 1, 1, 0, 3);
        let token = Token::new(TokenKind::KeywordLet, "let".to_string(), location.clone());
        
        assert!(token.is(&TokenKind::KeywordLet));
        assert!(token.is_keyword());
        assert!(!token.is_identifier());
        assert!(!token.is_literal());
    }
    
    #[test]
    fn test_token_display() {
        let location = SourceLocation::new("test.swl", 1, 1, 0, 3);
        let token = Token::new(TokenKind::KeywordLet, "let".to_string(), location);
        
        assert_eq!(format!("{}", token), "キーワード 'let' at test.swl:1:1");
    }
    
    #[test]
    fn test_keyword_from_string() {
        assert_eq!(TokenKind::from_keyword("let"), TokenKind::KeywordLet);
        assert_eq!(TokenKind::from_keyword("fn"), TokenKind::KeywordFn);
        assert_eq!(TokenKind::from_keyword("xyz"), TokenKind::Identifier);
    }
    
    #[test]
    fn test_operator_detection() {
        let location = SourceLocation::new("test.swl", 1, 1, 0, 1);
        let token = Token::new(TokenKind::Plus, "+".to_string(), location);
        
        assert!(token.is_operator());
        assert!(!token.is_keyword());
        assert!(!token.is_literal());
    }
    
    #[test]
    fn test_literal_detection() {
        let location = SourceLocation::new("test.swl", 1, 1, 0, 5);
        let token = Token::new(TokenKind::IntLiteral, "12345".to_string(), location);
        
        assert!(token.is_literal());
        assert!(!token.is_operator());
        assert!(!token.is_keyword());
    }
}
