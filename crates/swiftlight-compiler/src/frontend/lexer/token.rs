//! # トークン定義
//! 
//! SwiftLight言語のレキサーが生成するトークンの定義を提供します。
//! トークンの種類、位置情報、リテラル値などが含まれています。

use crate::frontend::error::SourceLocation;
use std::fmt;

/// トークンの種類
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenKind {
    // 識別子とリテラル
    /// 識別子
    Identifier(String),
    /// 整数リテラル
    IntLiteral(i64),
    /// 浮動小数点リテラル
    FloatLiteral(f64),
    /// 文字列リテラル
    StringLiteral(String),
    /// 文字リテラル
    CharLiteral(char),
    /// 真偽値リテラル（true）
    TrueLiteral,
    /// 真偽値リテラル（false）
    FalseLiteral,
    /// nilリテラル
    NilLiteral,

    // 区切り記号
    /// 左括弧 (
    LeftParen,
    /// 右括弧 )
    RightParen,
    /// 左波括弧 {
    LeftBrace,
    /// 右波括弧 }
    RightBrace,
    /// 左角括弧 [
    LeftBracket,
    /// 右角括弧 ]
    RightBracket,
    /// コンマ ,
    Comma,
    /// ドット .
    Dot,
    /// セミコロン ;
    Semicolon,
    /// コロン :
    Colon,
    /// ダブルコロン ::
    DoubleColon,
    /// アロー ->
    Arrow,
    /// ファットアロー =>
    FatArrow,
    /// アットマーク @
    At,
    /// ハッシュ #
    Hash,
    /// ダラー $
    Dollar,
    /// バックスラッシュ \
    Backslash,

    // 演算子
    /// プラス +
    Plus,
    /// マイナス -
    Minus,
    /// アスタリスク *
    Star,
    /// スラッシュ /
    Slash,
    /// パーセント %
    Percent,
    /// キャレット ^
    Caret,
    /// アンパサンド &
    Ampersand,
    /// パイプ |
    Pipe,
    /// チルダ ~
    Tilde,
    /// 感嘆符 !
    Bang,
    /// 等号 =
    Equal,
    /// 小なり <
    Less,
    /// 大なり >
    Greater,
    /// プラスイコール +=
    PlusEqual,
    /// マイナスイコール -=
    MinusEqual,
    /// アスタリスクイコール *=
    StarEqual,
    /// スラッシュイコール /=
    SlashEqual,
    /// パーセントイコール %=
    PercentEqual,
    /// キャレットイコール ^=
    CaretEqual,
    /// アンパサンドイコール &=
    AmpersandEqual,
    /// パイプイコール |=
    PipeEqual,
    /// チルダイコール ~=
    TildeEqual,
    /// バングイコール !=
    BangEqual,
    /// イコールイコール ==
    EqualEqual,
    /// 小なりイコール <=
    LessEqual,
    /// 大なりイコール >=
    GreaterEqual,
    /// ダブルアンパサンド &&
    AmpersandAmpersand,
    /// ダブルパイプ ||
    PipePipe,
    /// ダブルプラス ++
    PlusPlus,
    /// ダブルマイナス --
    MinusMinus,
    /// 左シフト <<
    LeftShift,
    /// 右シフト >>
    RightShift,
    /// 疑問符 ?
    Question,
    /// 疑問符ドット ?.
    QuestionDot,
    /// 疑問符疑問符 ??
    QuestionQuestion,
    /// ダブルドット ..
    DotDot,
    /// トリプルドット ...
    DotDotDot,

    // キーワード
    /// let キーワード
    Let,
    /// var キーワード
    Var,
    /// const キーワード
    Const,
    /// func キーワード
    Func,
    /// class キーワード
    Class,
    /// struct キーワード
    Struct,
    /// enum キーワード
    Enum,
    /// interface キーワード
    Interface,
    /// trait キーワード
    Trait,
    /// impl キーワード
    Impl,
    /// type キーワード
    Type,
    /// if キーワード
    If,
    /// else キーワード
    Else,
    /// for キーワード
    For,
    /// while キーワード
    While,
    /// do キーワード
    Do,
    /// break キーワード
    Break,
    /// continue キーワード
    Continue,
    /// return キーワード
    Return,
    /// yield キーワード
    Yield,
    /// match キーワード
    Match,
    /// case キーワード
    Case,
    /// default キーワード
    Default,
    /// switch キーワード
    Switch,
    /// try キーワード
    Try,
    /// catch キーワード
    Catch,
    /// throw キーワード
    Throw,
    /// pub キーワード
    Pub,
    /// private キーワード
    Private,
    /// protected キーワード
    Protected,
    /// internal キーワード
    Internal,
    /// static キーワード
    Static,
    /// async キーワード
    Async,
    /// await キーワード
    Await,
    /// mut キーワード
    Mut,
    /// ref キーワード
    Ref,
    /// unsafe キーワード
    Unsafe,
    /// module キーワード
    Module,
    /// import キーワード
    Import,
    /// export キーワード
    Export,
    /// as キーワード
    As,
    /// from キーワード
    From,
    /// where キーワード
    Where,
    /// inline キーワード
    Inline,
    /// extern キーワード
    Extern,
    /// sizeof キーワード
    Sizeof,
    /// typeof キーワード
    Typeof,
    /// in キーワード
    In,
    /// is キーワード
    Is,
    /// self キーワード
    SelfLower,
    /// Self キーワード
    SelfUpper,
    /// super キーワード
    Super,
    /// pure キーワード
    Pure,
    /// dependent キーワード
    Dependent,
    /// forall キーワード
    Forall,
    /// exists キーワード
    Exists,
    /// operator キーワード
    Operator,
    /// precedence キーワード
    Precedence,
    /// associativity キーワード
    Associativity,
    /// protocol キーワード
    Protocol,
    /// extension キーワード
    Extension,
    /// typealias キーワード
    Typealias,
    /// meta キーワード
    Meta,
    /// guard キーワード
    Guard,
    /// defer キーワード
    Defer,

    // その他
    /// コメント
    Comment(String),
    /// ドキュメントコメント
    DocComment(String),
    /// 空白文字（スペース、タブ、改行など）
    Whitespace,
    /// 無効なトークン
    Invalid(String),
    /// ファイルの終端
    Eof,
}

impl TokenKind {
    /// このトークンが宣言の開始を示すかどうかを判定
    pub fn is_declaration_start(&self) -> bool {
        use TokenKind::*;
        matches!(
            self,
            Let | Var | Const | Func | Struct | Enum | Trait | Impl | Type | Module | Import | Export
        )
    }
}

impl fmt::Display for TokenKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use TokenKind::*;
        match self {
            // 識別子とリテラル
            Identifier(name) => write!(f, "識別子 '{}'", name),
            IntLiteral(value) => write!(f, "整数リテラル {}", value),
            FloatLiteral(value) => write!(f, "浮動小数点リテラル {}", value),
            StringLiteral(value) => write!(f, "文字列リテラル \"{}\"", value),
            CharLiteral(value) => write!(f, "文字リテラル '{}'", value),
            TrueLiteral => write!(f, "true"),
            FalseLiteral => write!(f, "false"),
            NilLiteral => write!(f, "nil"),

            // 区切り記号
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
            DoubleColon => write!(f, "::"),
            Arrow => write!(f, "->"),
            FatArrow => write!(f, "=>"),
            At => write!(f, "@"),
            Hash => write!(f, "#"),
            Dollar => write!(f, "$"),
            Backslash => write!(f, "\\"),

            // 演算子
            Plus => write!(f, "+"),
            Minus => write!(f, "-"),
            Star => write!(f, "*"),
            Slash => write!(f, "/"),
            Percent => write!(f, "%"),
            Caret => write!(f, "^"),
            Ampersand => write!(f, "&"),
            Pipe => write!(f, "|"),
            Tilde => write!(f, "~"),
            Bang => write!(f, "!"),
            Equal => write!(f, "="),
            Less => write!(f, "<"),
            Greater => write!(f, ">"),
            PlusEqual => write!(f, "+="),
            MinusEqual => write!(f, "-="),
            StarEqual => write!(f, "*="),
            SlashEqual => write!(f, "/="),
            PercentEqual => write!(f, "%="),
            CaretEqual => write!(f, "^="),
            AmpersandEqual => write!(f, "&="),
            PipeEqual => write!(f, "|="),
            TildeEqual => write!(f, "~="),
            BangEqual => write!(f, "!="),
            EqualEqual => write!(f, "=="),
            LessEqual => write!(f, "<="),
            GreaterEqual => write!(f, ">="),
            AmpersandAmpersand => write!(f, "&&"),
            PipePipe => write!(f, "||"),
            PlusPlus => write!(f, "++"),
            MinusMinus => write!(f, "--"),
            LeftShift => write!(f, "<<"),
            RightShift => write!(f, ">>"),
            Question => write!(f, "?"),
            QuestionDot => write!(f, "?."),
            QuestionQuestion => write!(f, "??"),
            DotDot => write!(f, ".."),
            DotDotDot => write!(f, "..."),

            // キーワード
            Let => write!(f, "let"),
            Var => write!(f, "var"),
            Const => write!(f, "const"),
            Func => write!(f, "func"),
            Class => write!(f, "class"),
            Struct => write!(f, "struct"),
            Enum => write!(f, "enum"),
            Interface => write!(f, "interface"),
            Trait => write!(f, "trait"),
            Impl => write!(f, "impl"),
            Type => write!(f, "type"),
            If => write!(f, "if"),
            Else => write!(f, "else"),
            For => write!(f, "for"),
            While => write!(f, "while"),
            Do => write!(f, "do"),
            Break => write!(f, "break"),
            Continue => write!(f, "continue"),
            Return => write!(f, "return"),
            Yield => write!(f, "yield"),
            Match => write!(f, "match"),
            Case => write!(f, "case"),
            Default => write!(f, "default"),
            Switch => write!(f, "switch"),
            Try => write!(f, "try"),
            Catch => write!(f, "catch"),
            Throw => write!(f, "throw"),
            Pub => write!(f, "pub"),
            Private => write!(f, "private"),
            Protected => write!(f, "protected"),
            Internal => write!(f, "internal"),
            Static => write!(f, "static"),
            Async => write!(f, "async"),
            Await => write!(f, "await"),
            Mut => write!(f, "mut"),
            Ref => write!(f, "ref"),
            Unsafe => write!(f, "unsafe"),
            Module => write!(f, "module"),
            Import => write!(f, "import"),
            Export => write!(f, "export"),
            As => write!(f, "as"),
            From => write!(f, "from"),
            Where => write!(f, "where"),
            Inline => write!(f, "inline"),
            Extern => write!(f, "extern"),
            Sizeof => write!(f, "sizeof"),
            Typeof => write!(f, "typeof"),
            In => write!(f, "in"),
            Is => write!(f, "is"),
            SelfLower => write!(f, "self"),
            SelfUpper => write!(f, "Self"),
            Super => write!(f, "super"),
            Pure => write!(f, "pure"),
            Dependent => write!(f, "dependent"),
            Forall => write!(f, "forall"),
            Exists => write!(f, "exists"),
            Operator => write!(f, "operator"),
            Precedence => write!(f, "precedence"),
            Associativity => write!(f, "associativity"),
            Protocol => write!(f, "protocol"),
            Extension => write!(f, "extension"),
            Typealias => write!(f, "typealias"),
            Meta => write!(f, "meta"),
            Guard => write!(f, "guard"),
            Defer => write!(f, "defer"),

            // その他
            Comment(text) => write!(f, "コメント '{}'", text),
            DocComment(text) => write!(f, "ドキュメントコメント '{}'", text),
            Whitespace => write!(f, "空白文字"),
            Invalid(reason) => write!(f, "無効なトークン: {}", reason),
            Eof => write!(f, "EOF"),
        }
    }
}

/// トークン
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    /// トークンの種類
    pub kind: TokenKind,
    /// トークンの位置情報
    pub location: SourceLocation,
}

impl Token {
    /// 新しいトークンを作成
    pub fn new(kind: TokenKind, location: SourceLocation) -> Self {
        Self { kind, location }
    }

    /// トークンがキーワードかどうかを判定
    pub fn is_keyword(&self) -> bool {
        use TokenKind::*;
        matches!(
            self.kind,
            Let | Var | Const | Func | Class | Struct | Enum | Interface | Trait | Impl | Type
                | If | Else | For | While | Do | Break | Continue | Return | Yield
                | Match | Case | Default | Switch | Try | Catch | Throw
                | Pub | Private | Protected | Internal | Static | Async | Await | Mut | Ref | Unsafe
                | Module | Import | Export | As | From | Where | Inline | Extern | Sizeof | Typeof
                | In | Is | SelfLower | SelfUpper | Super | Pure | Dependent | Forall | Exists
                | Operator | Precedence | Associativity | Protocol | Extension | Typealias
                | Meta | Guard | Defer
        )
    }

    /// トークンが識別子かどうかを判定
    pub fn is_identifier(&self) -> bool {
        matches!(self.kind, TokenKind::Identifier(_))
    }

    /// トークンがリテラルかどうかを判定
    pub fn is_literal(&self) -> bool {
        use TokenKind::*;
        matches!(
            self.kind,
            IntLiteral(_) | FloatLiteral(_) | StringLiteral(_) | CharLiteral(_) | TrueLiteral | FalseLiteral | NilLiteral
        )
    }

    /// トークンが演算子かどうかを判定
    pub fn is_operator(&self) -> bool {
        use TokenKind::*;
        matches!(
            self.kind,
            Plus | Minus | Star | Slash | Percent | Caret | Ampersand | Pipe | Tilde | Bang
                | Equal | Less | Greater | PlusEqual | MinusEqual | StarEqual | SlashEqual
                | PercentEqual | CaretEqual | AmpersandEqual | PipeEqual | TildeEqual
                | BangEqual | EqualEqual | LessEqual | GreaterEqual | AmpersandAmpersand
                | PipePipe | PlusPlus | MinusMinus | LeftShift | RightShift | Question
                | QuestionDot | QuestionQuestion | DotDot | DotDotDot
        )
    }

    /// トークンが指定した種類かどうかを判定
    pub fn is(&self, kind: TokenKind) -> bool {
        // 識別子とリテラルの場合、内容はチェックせずに種類のみチェック
        match (&self.kind, &kind) {
            (TokenKind::Identifier(_), TokenKind::Identifier(_)) => true,
            (TokenKind::IntLiteral(_), TokenKind::IntLiteral(_)) => true,
            (TokenKind::FloatLiteral(_), TokenKind::FloatLiteral(_)) => true,
            (TokenKind::StringLiteral(_), TokenKind::StringLiteral(_)) => true,
            (TokenKind::CharLiteral(_), TokenKind::CharLiteral(_)) => true,
            (TokenKind::Comment(_), TokenKind::Comment(_)) => true,
            (TokenKind::DocComment(_), TokenKind::DocComment(_)) => true,
            (_, _) => self.kind == kind,
        }
    }

    /// トークンが文の終端を示すかどうかを判定
    pub fn is_statement_terminator(&self) -> bool {
        matches!(self.kind, TokenKind::Semicolon | TokenKind::Eof)
    }
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} at {}", self.kind, self.location)
    }
}

/// キーワードの文字列からTokenKindへの変換
pub fn lookup_keyword(identifier: &str) -> TokenKind {
    match identifier {
        "let" => TokenKind::Let,
        "var" => TokenKind::Var,
        "const" => TokenKind::Const,
        "func" => TokenKind::Func,
        "class" => TokenKind::Class,
        "struct" => TokenKind::Struct,
        "enum" => TokenKind::Enum,
        "interface" => TokenKind::Interface,
        "trait" => TokenKind::Trait,
        "impl" => TokenKind::Impl,
        "type" => TokenKind::Type,
        "if" => TokenKind::If,
        "else" => TokenKind::Else,
        "for" => TokenKind::For,
        "while" => TokenKind::While,
        "do" => TokenKind::Do,
        "break" => TokenKind::Break,
        "continue" => TokenKind::Continue,
        "return" => TokenKind::Return,
        "yield" => TokenKind::Yield,
        "match" => TokenKind::Match,
        "case" => TokenKind::Case,
        "default" => TokenKind::Default,
        "switch" => TokenKind::Switch,
        "try" => TokenKind::Try,
        "catch" => TokenKind::Catch,
        "throw" => TokenKind::Throw,
        "pub" => TokenKind::Pub,
        "private" => TokenKind::Private,
        "protected" => TokenKind::Protected,
        "internal" => TokenKind::Internal,
        "static" => TokenKind::Static,
        "async" => TokenKind::Async,
        "await" => TokenKind::Await,
        "mut" => TokenKind::Mut,
        "ref" => TokenKind::Ref,
        "unsafe" => TokenKind::Unsafe,
        "module" => TokenKind::Module,
        "import" => TokenKind::Import,
        "export" => TokenKind::Export,
        "as" => TokenKind::As,
        "from" => TokenKind::From,
        "where" => TokenKind::Where,
        "inline" => TokenKind::Inline,
        "extern" => TokenKind::Extern,
        "sizeof" => TokenKind::Sizeof,
        "typeof" => TokenKind::Typeof,
        "in" => TokenKind::In,
        "is" => TokenKind::Is,
        "self" => TokenKind::SelfLower,
        "Self" => TokenKind::SelfUpper,
        "super" => TokenKind::Super,
        "true" => TokenKind::TrueLiteral,
        "false" => TokenKind::FalseLiteral,
        "nil" => TokenKind::NilLiteral,
        "pure" => TokenKind::Pure,
        "dependent" => TokenKind::Dependent,
        "forall" => TokenKind::Forall,
        "exists" => TokenKind::Exists,
        "operator" => TokenKind::Operator,
        "precedence" => TokenKind::Precedence,
        "associativity" => TokenKind::Associativity,
        "protocol" => TokenKind::Protocol,
        "extension" => TokenKind::Extension,
        "typealias" => TokenKind::Typealias,
        "meta" => TokenKind::Meta,
        "guard" => TokenKind::Guard,
        "defer" => TokenKind::Defer,
        _ => TokenKind::Identifier(identifier.to_string()),
    }
}
