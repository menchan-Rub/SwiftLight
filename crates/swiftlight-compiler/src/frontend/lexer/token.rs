//! # トークン定義
//! 
//! SwiftLight言語の字句解析で使用するトークンを定義します。
//! 各トークンの種類と属性を提供します。

use std::fmt;
use crate::frontend::error::SourceLocation;

/// トークンの種類を表す列挙型
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenKind {
    // 識別子とリテラル
    /// 識別子（変数名、関数名など）
    Identifier(String),
    /// 整数リテラル
    IntLiteral(String),
    /// 浮動小数点リテラル
    FloatLiteral(String),
    /// 文字列リテラル
    StringLiteral(String),
    /// 文字リテラル
    CharLiteral(char),
    
    // キーワード
    /// `let` キーワード（変数宣言）
    Let,
    /// `const` キーワード（定数宣言）
    Const,
    /// `func` キーワード（関数宣言）
    Func,
    /// `return` キーワード
    Return,
    /// `if` キーワード
    If,
    /// `else` キーワード
    Else,
    /// `while` キーワード
    While,
    /// `for` キーワード
    For,
    /// `in` キーワード（for文で使用）
    In,
    /// `break` キーワード
    Break,
    /// `continue` キーワード
    Continue,
    /// `struct` キーワード
    Struct,
    /// `enum` キーワード
    Enum,
    /// `trait` キーワード
    Trait,
    /// `impl` キーワード
    Impl,
    /// `import` キーワード
    Import,
    /// `as` キーワード（型キャストやインポートエイリアス）
    As,
    /// `true` キーワード
    True,
    /// `false` キーワード
    False,
    /// `nil` キーワード
    Nil,
    /// `self` キーワード
    SelfValue,
    /// `Self` キーワード（型名）
    SelfType,
    /// `pub` キーワード（公開アクセス）
    Pub,
    /// `mut` キーワード（可変）
    Mut,
    
    // 記号と演算子
    /// 加算演算子 `+`
    Plus,
    /// 減算演算子 `-`
    Minus,
    /// 乗算演算子 `*`
    Star,
    /// 除算演算子 `/`
    Slash,
    /// 剰余演算子 `%`
    Percent,
    /// べき乗演算子 `**`
    DoubleStar,
    
    /// 等価演算子 `==`
    EqualEqual,
    /// 非等価演算子 `!=`
    BangEqual,
    /// 小なり演算子 `<`
    Less,
    /// 大なり演算子 `>`
    Greater,
    /// 以下演算子 `<=`
    LessEqual,
    /// 以上演算子 `>=`
    GreaterEqual,
    
    /// 論理AND演算子 `&&`
    AmpersandAmpersand,
    /// 論理OR演算子 `||`
    PipePipe,
    /// 論理NOT演算子 `!`
    Bang,
    
    /// ビットAND演算子 `&`
    Ampersand,
    /// ビットOR演算子 `|`
    Pipe,
    /// ビットXOR演算子 `^`
    Caret,
    /// ビット左シフト演算子 `<<`
    LessLess,
    /// ビット右シフト演算子 `>>`
    GreaterGreater,
    /// ビットNOT演算子 `~`
    Tilde,
    
    /// 代入演算子 `=`
    Equal,
    /// 複合代入: 加算 `+=`
    PlusEqual,
    /// 複合代入: 減算 `-=`
    MinusEqual,
    /// 複合代入: 乗算 `*=`
    StarEqual,
    /// 複合代入: 除算 `/=`
    SlashEqual,
    /// 複合代入: 剰余 `%=`
    PercentEqual,
    /// 複合代入: ビットAND `&=`
    AmpersandEqual,
    /// 複合代入: ビットOR `|=`
    PipeEqual,
    /// 複合代入: ビットXOR `^=`
    CaretEqual,
    
    /// セミコロン `;`
    Semicolon,
    /// コロン `:`
    Colon,
    /// ダブルコロン `::`
    DoubleColon,
    /// コンマ `,`
    Comma,
    /// ドット `.`
    Dot,
    /// アロー `->`
    Arrow,
    /// ファットアロー `=>`
    FatArrow,
    /// 疑問符 `?`
    Question,
    /// 省略記号 `...`
    Ellipsis,
    
    /// 開き括弧 `(`
    LeftParen,
    /// 閉じ括弧 `)`
    RightParen,
    /// 開き波括弧 `{`
    LeftBrace,
    /// 閉じ波括弧 `}`
    RightBrace,
    /// 開き角括弧 `[`
    LeftBracket,
    /// 閉じ角括弧 `]`
    RightBracket,
    
    /// 行コメント
    Comment,
    /// ドキュメントコメント
    DocComment(String),
    
    /// ファイル終端
    Eof,
    /// 不明なトークン
    Unknown(char),
}

impl TokenKind {
    /// キーワードの文字列からトークン種別を取得
    pub fn from_keyword(ident: &str) -> Self {
        match ident {
            "let" => TokenKind::Let,
            "const" => TokenKind::Const,
            "func" => TokenKind::Func,
            "return" => TokenKind::Return,
            "if" => TokenKind::If,
            "else" => TokenKind::Else,
            "while" => TokenKind::While,
            "for" => TokenKind::For,
            "in" => TokenKind::In,
            "break" => TokenKind::Break,
            "continue" => TokenKind::Continue,
            "struct" => TokenKind::Struct,
            "enum" => TokenKind::Enum,
            "trait" => TokenKind::Trait,
            "impl" => TokenKind::Impl,
            "import" => TokenKind::Import,
            "as" => TokenKind::As,
            "true" => TokenKind::True,
            "false" => TokenKind::False,
            "nil" => TokenKind::Nil,
            "self" => TokenKind::SelfValue,
            "Self" => TokenKind::SelfType,
            "pub" => TokenKind::Pub,
            "mut" => TokenKind::Mut,
            _ => TokenKind::Identifier(ident.to_string()),
        }
    }
    
    /// トークンが識別子かどうかを判定
    pub fn is_identifier(&self) -> bool {
        matches!(self, TokenKind::Identifier(_))
    }
    
    /// トークンがリテラルかどうかを判定
    pub fn is_literal(&self) -> bool {
        matches!(
            self,
            TokenKind::IntLiteral(_)
                | TokenKind::FloatLiteral(_)
                | TokenKind::StringLiteral(_)
                | TokenKind::CharLiteral(_)
                | TokenKind::True
                | TokenKind::False
                | TokenKind::Nil
        )
    }
    
    /// トークンがキーワードかどうかを判定
    pub fn is_keyword(&self) -> bool {
        matches!(
            self,
            TokenKind::Let
                | TokenKind::Const
                | TokenKind::Func
                | TokenKind::Return
                | TokenKind::If
                | TokenKind::Else
                | TokenKind::While
                | TokenKind::For
                | TokenKind::In
                | TokenKind::Break
                | TokenKind::Continue
                | TokenKind::Struct
                | TokenKind::Enum
                | TokenKind::Trait
                | TokenKind::Impl
                | TokenKind::Import
                | TokenKind::As
                | TokenKind::True
                | TokenKind::False
                | TokenKind::Nil
                | TokenKind::SelfValue
                | TokenKind::SelfType
                | TokenKind::Pub
                | TokenKind::Mut
        )
    }
}

impl fmt::Display for TokenKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenKind::Identifier(name) => write!(f, "識別子 '{}'", name),
            TokenKind::IntLiteral(val) => write!(f, "整数リテラル '{}'", val),
            TokenKind::FloatLiteral(val) => write!(f, "浮動小数点リテラル '{}'", val),
            TokenKind::StringLiteral(val) => write!(f, "文字列リテラル \"{}\"", val),
            TokenKind::CharLiteral(c) => write!(f, "文字リテラル '{}'", c),
            
            TokenKind::Let => write!(f, "キーワード 'let'"),
            TokenKind::Const => write!(f, "キーワード 'const'"),
            TokenKind::Func => write!(f, "キーワード 'func'"),
            TokenKind::Return => write!(f, "キーワード 'return'"),
            TokenKind::If => write!(f, "キーワード 'if'"),
            TokenKind::Else => write!(f, "キーワード 'else'"),
            TokenKind::While => write!(f, "キーワード 'while'"),
            TokenKind::For => write!(f, "キーワード 'for'"),
            TokenKind::In => write!(f, "キーワード 'in'"),
            TokenKind::Break => write!(f, "キーワード 'break'"),
            TokenKind::Continue => write!(f, "キーワード 'continue'"),
            TokenKind::Struct => write!(f, "キーワード 'struct'"),
            TokenKind::Enum => write!(f, "キーワード 'enum'"),
            TokenKind::Trait => write!(f, "キーワード 'trait'"),
            TokenKind::Impl => write!(f, "キーワード 'impl'"),
            TokenKind::Import => write!(f, "キーワード 'import'"),
            TokenKind::As => write!(f, "キーワード 'as'"),
            TokenKind::True => write!(f, "キーワード 'true'"),
            TokenKind::False => write!(f, "キーワード 'false'"),
            TokenKind::Nil => write!(f, "キーワード 'nil'"),
            TokenKind::SelfValue => write!(f, "キーワード 'self'"),
            TokenKind::SelfType => write!(f, "キーワード 'Self'"),
            TokenKind::Pub => write!(f, "キーワード 'pub'"),
            TokenKind::Mut => write!(f, "キーワード 'mut'"),
            
            TokenKind::Plus => write!(f, "演算子 '+'"),
            TokenKind::Minus => write!(f, "演算子 '-'"),
            TokenKind::Star => write!(f, "演算子 '*'"),
            TokenKind::Slash => write!(f, "演算子 '/'"),
            TokenKind::Percent => write!(f, "演算子 '%'"),
            TokenKind::DoubleStar => write!(f, "演算子 '**'"),
            
            TokenKind::EqualEqual => write!(f, "演算子 '=='"),
            TokenKind::BangEqual => write!(f, "演算子 '!='"),
            TokenKind::Less => write!(f, "演算子 '<'"),
            TokenKind::Greater => write!(f, "演算子 '>'"),
            TokenKind::LessEqual => write!(f, "演算子 '<='"),
            TokenKind::GreaterEqual => write!(f, "演算子 '>='"),
            
            TokenKind::AmpersandAmpersand => write!(f, "演算子 '&&'"),
            TokenKind::PipePipe => write!(f, "演算子 '||'"),
            TokenKind::Bang => write!(f, "演算子 '!'"),
            
            TokenKind::Ampersand => write!(f, "演算子 '&'"),
            TokenKind::Pipe => write!(f, "演算子 '|'"),
            TokenKind::Caret => write!(f, "演算子 '^'"),
            TokenKind::LessLess => write!(f, "演算子 '<<'"),
            TokenKind::GreaterGreater => write!(f, "演算子 '>>'"),
            TokenKind::Tilde => write!(f, "演算子 '~'"),
            
            TokenKind::Equal => write!(f, "演算子 '='"),
            TokenKind::PlusEqual => write!(f, "演算子 '+='"),
            TokenKind::MinusEqual => write!(f, "演算子 '-='"),
            TokenKind::StarEqual => write!(f, "演算子 '*='"),
            TokenKind::SlashEqual => write!(f, "演算子 '/='"),
            TokenKind::PercentEqual => write!(f, "演算子 '%='"),
            TokenKind::AmpersandEqual => write!(f, "演算子 '&='"),
            TokenKind::PipeEqual => write!(f, "演算子 '|='"),
            TokenKind::CaretEqual => write!(f, "演算子 '^='"),
            
            TokenKind::Semicolon => write!(f, "';'"),
            TokenKind::Colon => write!(f, "':'"),
            TokenKind::DoubleColon => write!(f, "'::'"),
            TokenKind::Comma => write!(f, "','"),
            TokenKind::Dot => write!(f, "'.'"),
            TokenKind::Arrow => write!(f, "'->'"),
            TokenKind::FatArrow => write!(f, "'=>'"),
            TokenKind::Question => write!(f, "'?'"),
            TokenKind::Ellipsis => write!(f, "'...'"),
            
            TokenKind::LeftParen => write!(f, "'('"),
            TokenKind::RightParen => write!(f, "')'"),
            TokenKind::LeftBrace => write!(f, "'{{'"),
            TokenKind::RightBrace => write!(f, "'}}'"),
            TokenKind::LeftBracket => write!(f, "'['"),
            TokenKind::RightBracket => write!(f, "']'"),
            
            TokenKind::Comment => write!(f, "コメント"),
            TokenKind::DocComment(text) => write!(f, "ドキュメントコメント '{}'", text),
            
            TokenKind::Eof => write!(f, "EOF"),
            TokenKind::Unknown(c) => write!(f, "不明な文字 '{}'", c),
        }
    }
}

/// トークン
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    /// トークンの種類
    pub kind: TokenKind,
    /// ソースコード内の位置
    pub location: SourceLocation,
    /// トークンのソース文字列
    pub lexeme: String,
}

impl Token {
    /// 新しいトークンを作成
    pub fn new(kind: TokenKind, location: SourceLocation, lexeme: impl Into<String>) -> Self {
        Self {
            kind,
            location,
            lexeme: lexeme.into(),
        }
    }
    
    /// トークンが指定された種類かどうかを判定
    pub fn is(&self, kind: &TokenKind) -> bool {
        std::mem::discriminant(&self.kind) == std::mem::discriminant(kind)
    }
    
    /// トークンが識別子かどうかを判定
    pub fn is_identifier(&self) -> bool {
        self.kind.is_identifier()
    }
    
    /// トークンがリテラルかどうかを判定
    pub fn is_literal(&self) -> bool {
        self.kind.is_literal()
    }
    
    /// トークンがキーワードかどうかを判定
    pub fn is_keyword(&self) -> bool {
        self.kind.is_keyword()
    }
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} at {}", self.kind, self.location)
    }
}

/// テスト
#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend::error::SourceLocation;
    
    #[test]
    fn test_token_creation() {
        let location = SourceLocation::new("test.swl", 1, 1, 0, 3);
        let token = Token::new(TokenKind::Let, location.clone(), "let");
        
        assert!(token.is(&TokenKind::Let));
        assert!(token.is_keyword());
        assert!(!token.is_identifier());
        assert!(!token.is_literal());
    }
    
    #[test]
    fn test_token_display() {
        let location = SourceLocation::new("test.swl", 1, 1, 0, 3);
        let token = Token::new(TokenKind::Let, location, "let");
        
        assert_eq!(format!("{}", token), "キーワード 'let' at test.swl:1:1");
    }
    
    #[test]
    fn test_keyword_from_string() {
        assert_eq!(TokenKind::from_keyword("let"), TokenKind::Let);
        assert_eq!(TokenKind::from_keyword("func"), TokenKind::Func);
        assert_eq!(TokenKind::from_keyword("xyz"), TokenKind::Identifier("xyz".to_string()));
    }
}
