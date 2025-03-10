//! # SwiftLight言語の文法規則
//! 
//! このモジュールでは、SwiftLight言語の文法規則を定義します。
//! パーサーの実装で使用される構文規則とプロダクションを提供します。

use crate::frontend::lexer::token::TokenKind;

/// 優先順位レベル（低いほど優先度が低い）
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Precedence {
    /// 最低優先度（割り当て、範囲など）
    Lowest = 0,
    /// 代入（=, +=, -=, *=, /=, %=, &=, |=, ^=, <<=, >>=）
    Assignment = 1,
    /// 範囲 (.., ..=)
    Range = 2,
    /// 条件式 (三項演算子)
    Conditional = 3,
    /// 論理OR (||)
    LogicalOr = 4,
    /// 論理AND (&&)
    LogicalAnd = 5,
    /// 等価比較 (==, !=)
    Equality = 6,
    /// 比較 (<, >, <=, >=)
    Comparison = 7,
    /// ビット演算OR, XOR (|, ^)
    BitwiseOr = 8,
    /// ビット演算AND (&)
    BitwiseAnd = 9,
    /// シフト (<<, >>)
    Shift = 10,
    /// 加算・減算 (+, -)
    Term = 11,
    /// 乗算・除算・剰余 (*, /, %)
    Factor = 12,
    /// 単項演算子 (!, -, ~, &, *, ++, --)
    Unary = 13,
    /// メソッド呼び出し、添え字アクセス、メンバーアクセス (obj.method(), array[index], obj.field)
    Call = 14,
    /// 基本式（リテラル、識別子、括弧で囲まれた式など）
    Primary = 15,
}

impl Precedence {
    /// トークンから優先順位を取得
    pub fn from_token(token: &TokenKind) -> Self {
        match token {
            // 代入演算子
            TokenKind::Equal | 
            TokenKind::PlusEqual | TokenKind::MinusEqual | 
            TokenKind::StarEqual | TokenKind::SlashEqual | 
            TokenKind::PercentEqual | 
            TokenKind::AmpersandEqual | TokenKind::PipeEqual | TokenKind::CaretEqual |
            TokenKind::LeftShiftEqual | TokenKind::RightShiftEqual => Precedence::Assignment,
            
            // 範囲演算子
            TokenKind::Range | TokenKind::RangeInclusive => Precedence::Range,
            
            // 論理演算子
            TokenKind::PipePipe => Precedence::LogicalOr,
            TokenKind::AmpersandAmpersand => Precedence::LogicalAnd,
            
            // 等価演算子
            TokenKind::EqualEqual | TokenKind::BangEqual => Precedence::Equality,
            
            // 比較演算子
            TokenKind::Less | TokenKind::Greater | 
            TokenKind::LessEqual | TokenKind::GreaterEqual => Precedence::Comparison,
            
            // ビット演算子
            TokenKind::Pipe | TokenKind::Caret => Precedence::BitwiseOr,
            TokenKind::Ampersand => Precedence::BitwiseAnd,
            TokenKind::LeftShift | TokenKind::RightShift => Precedence::Shift,
            
            // 算術演算子
            TokenKind::Plus | TokenKind::Minus => Precedence::Term,
            TokenKind::Star | TokenKind::Slash | TokenKind::Percent => Precedence::Factor,
            
            // 関数呼び出し、配列アクセス、メンバーアクセス
            TokenKind::LeftParen | TokenKind::LeftBracket | TokenKind::Dot => Precedence::Call,
            
            // その他
            _ => Precedence::Lowest,
        }
    }
}

/// 二項演算子のトークンかどうかを判定
pub fn is_binary_operator(token: &TokenKind) -> bool {
    matches!(token,
        TokenKind::Plus | TokenKind::Minus | 
        TokenKind::Star | TokenKind::Slash | TokenKind::Percent |
        TokenKind::EqualEqual | TokenKind::BangEqual |
        TokenKind::Less | TokenKind::Greater |
        TokenKind::LessEqual | TokenKind::GreaterEqual |
        TokenKind::AmpersandAmpersand | TokenKind::PipePipe |
        TokenKind::Ampersand | TokenKind::Pipe | TokenKind::Caret |
        TokenKind::LeftShift | TokenKind::RightShift |
        TokenKind::Range | TokenKind::RangeInclusive |
        TokenKind::Equal | TokenKind::PlusEqual | TokenKind::MinusEqual |
        TokenKind::StarEqual | TokenKind::SlashEqual | TokenKind::PercentEqual |
        TokenKind::AmpersandEqual | TokenKind::PipeEqual | TokenKind::CaretEqual |
        TokenKind::LeftShiftEqual | TokenKind::RightShiftEqual
    )
}

/// 単項演算子のトークンかどうかを判定
pub fn is_unary_operator(token: &TokenKind) -> bool {
    matches!(token,
        TokenKind::Plus | TokenKind::Minus | 
        TokenKind::Bang | TokenKind::Tilde |
        TokenKind::Ampersand | TokenKind::Star |
        TokenKind::PlusPlus | TokenKind::MinusMinus
    )
}

/// 単項前置演算子のトークンかどうかを判定
pub fn is_prefix_operator(token: &TokenKind) -> bool {
    matches!(token,
        TokenKind::Plus | TokenKind::Minus | 
        TokenKind::Bang | TokenKind::Tilde |
        TokenKind::Ampersand | TokenKind::Star |
        TokenKind::PlusPlus | TokenKind::MinusMinus
    )
}

/// 単項後置演算子のトークンかどうかを判定
pub fn is_postfix_operator(token: &TokenKind) -> bool {
    matches!(token,
        TokenKind::PlusPlus | TokenKind::MinusMinus
    )
}

/// 文を開始するトークンかどうかを判定
pub fn is_statement_start(token: &TokenKind) -> bool {
    matches!(token,
        TokenKind::KeywordLet | TokenKind::KeywordVar | TokenKind::KeywordConst |
        TokenKind::KeywordIf | TokenKind::KeywordWhile | TokenKind::KeywordFor |
        TokenKind::KeywordReturn | TokenKind::KeywordBreak | TokenKind::KeywordContinue |
        TokenKind::LeftBrace | TokenKind::Semicolon |
        TokenKind::KeywordTry | TokenKind::KeywordThrow | TokenKind::KeywordAsync |
        TokenKind::KeywordMatch
    )
}

/// 式を開始するトークンかどうかを判定
pub fn is_expression_start(token: &TokenKind) -> bool {
    matches!(token,
        TokenKind::Identifier | 
        TokenKind::IntLiteral | TokenKind::FloatLiteral | 
        TokenKind::StringLiteral | TokenKind::CharLiteral |
        TokenKind::KeywordTrue | TokenKind::KeywordFalse | TokenKind::KeywordNil |
        TokenKind::LeftParen | TokenKind::LeftBracket | TokenKind::LeftBrace |
        TokenKind::KeywordSelf | TokenKind::KeywordSuper |
        TokenKind::Plus | TokenKind::Minus | TokenKind::Bang | TokenKind::Tilde |
        TokenKind::PlusPlus | TokenKind::MinusMinus |
        TokenKind::Ampersand | TokenKind::Star |
        TokenKind::KeywordIf | TokenKind::KeywordMatch | 
        TokenKind::KeywordTry | TokenKind::KeywordThrow | 
        TokenKind::KeywordAsync | TokenKind::KeywordAwait
    )
}

/// 宣言を開始するトークンかどうかを判定
pub fn is_declaration_start(token: &TokenKind) -> bool {
    matches!(token,
        TokenKind::KeywordFn | TokenKind::KeywordLet | TokenKind::KeywordVar |
        TokenKind::KeywordConst | TokenKind::KeywordStruct | TokenKind::KeywordEnum |
        TokenKind::KeywordTrait | TokenKind::KeywordImpl | TokenKind::KeywordType |
        TokenKind::KeywordImport | TokenKind::KeywordModule |
        TokenKind::KeywordPub | TokenKind::KeywordAsync | TokenKind::KeywordUnsafe
    )
}

/// 型を開始するトークンかどうかを判定
pub fn is_type_start(token: &TokenKind) -> bool {
    matches!(token,
        TokenKind::Identifier | 
        TokenKind::KeywordSelf |
        TokenKind::LeftParen | TokenKind::LeftBracket |
        TokenKind::Ampersand | TokenKind::Star
    )
}

/// 文の終わりを示すトークンかどうかを判定
pub fn is_statement_end(token: &TokenKind) -> bool {
    token == &TokenKind::Semicolon || token == &TokenKind::RightBrace
}

/// ブロックの終わりを示すトークンかどうかを判定
pub fn is_block_end(token: &TokenKind) -> bool {
    token == &TokenKind::RightBrace
}

/// 式の終わりを示すトークンかどうかを判定
pub fn is_expression_end(token: &TokenKind) -> bool {
    matches!(token,
        TokenKind::Semicolon | TokenKind::Comma | 
        TokenKind::RightParen | TokenKind::RightBracket | TokenKind::RightBrace |
        TokenKind::Colon | TokenKind::Arrow | TokenKind::FatArrow
    )
}

/// 型の終わりを示すトークンかどうかを判定
pub fn is_type_end(token: &TokenKind) -> bool {
    matches!(token,
        TokenKind::Semicolon | TokenKind::Comma | 
        TokenKind::RightParen | TokenKind::RightBracket | TokenKind::RightBrace |
        TokenKind::Equal | TokenKind::LeftBrace |
        TokenKind::Colon | TokenKind::Arrow
    )
}

/// リテラルを表すトークンかどうかを判定
pub fn is_literal(token: &TokenKind) -> bool {
    matches!(token,
        TokenKind::IntLiteral | TokenKind::FloatLiteral | 
        TokenKind::StringLiteral | TokenKind::CharLiteral |
        TokenKind::KeywordTrue | TokenKind::KeywordFalse | TokenKind::KeywordNil
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_precedence_ordering() {
        assert!(Precedence::Primary > Precedence::Call);
        assert!(Precedence::Call > Precedence::Unary);
        assert!(Precedence::Unary > Precedence::Factor);
        assert!(Precedence::Factor > Precedence::Term);
        assert!(Precedence::Term > Precedence::Shift);
        assert!(Precedence::Shift > Precedence::BitwiseAnd);
        assert!(Precedence::BitwiseAnd > Precedence::BitwiseOr);
        assert!(Precedence::BitwiseOr > Precedence::Comparison);
        assert!(Precedence::Comparison > Precedence::Equality);
        assert!(Precedence::Equality > Precedence::LogicalAnd);
        assert!(Precedence::LogicalAnd > Precedence::LogicalOr);
        assert!(Precedence::LogicalOr > Precedence::Conditional);
        assert!(Precedence::Conditional > Precedence::Range);
        assert!(Precedence::Range > Precedence::Assignment);
        assert!(Precedence::Assignment > Precedence::Lowest);
    }
    
    #[test]
    fn test_precedence_from_token() {
        assert_eq!(Precedence::from_token(&TokenKind::Plus), Precedence::Term);
        assert_eq!(Precedence::from_token(&TokenKind::Minus), Precedence::Term);
        assert_eq!(Precedence::from_token(&TokenKind::Star), Precedence::Factor);
        assert_eq!(Precedence::from_token(&TokenKind::Slash), Precedence::Factor);
        assert_eq!(Precedence::from_token(&TokenKind::Percent), Precedence::Factor);
        assert_eq!(Precedence::from_token(&TokenKind::EqualEqual), Precedence::Equality);
        assert_eq!(Precedence::from_token(&TokenKind::BangEqual), Precedence::Equality);
        assert_eq!(Precedence::from_token(&TokenKind::Less), Precedence::Comparison);
        assert_eq!(Precedence::from_token(&TokenKind::Greater), Precedence::Comparison);
        assert_eq!(Precedence::from_token(&TokenKind::LessEqual), Precedence::Comparison);
        assert_eq!(Precedence::from_token(&TokenKind::GreaterEqual), Precedence::Comparison);
        assert_eq!(Precedence::from_token(&TokenKind::AmpersandAmpersand), Precedence::LogicalAnd);
        assert_eq!(Precedence::from_token(&TokenKind::PipePipe), Precedence::LogicalOr);
        assert_eq!(Precedence::from_token(&TokenKind::Equal), Precedence::Assignment);
        assert_eq!(Precedence::from_token(&TokenKind::PlusEqual), Precedence::Assignment);
        assert_eq!(Precedence::from_token(&TokenKind::LeftParen), Precedence::Call);
        assert_eq!(Precedence::from_token(&TokenKind::LeftBracket), Precedence::Call);
        assert_eq!(Precedence::from_token(&TokenKind::Dot), Precedence::Call);
    }
    
    #[test]
    fn test_is_binary_operator() {
        assert!(is_binary_operator(&TokenKind::Plus));
        assert!(is_binary_operator(&TokenKind::Minus));
        assert!(is_binary_operator(&TokenKind::Star));
        assert!(is_binary_operator(&TokenKind::Slash));
        assert!(is_binary_operator(&TokenKind::Percent));
        assert!(is_binary_operator(&TokenKind::EqualEqual));
        assert!(is_binary_operator(&TokenKind::BangEqual));
        assert!(is_binary_operator(&TokenKind::Less));
        assert!(is_binary_operator(&TokenKind::Greater));
        assert!(is_binary_operator(&TokenKind::LessEqual));
        assert!(is_binary_operator(&TokenKind::GreaterEqual));
        assert!(is_binary_operator(&TokenKind::AmpersandAmpersand));
        assert!(is_binary_operator(&TokenKind::PipePipe));
        assert!(is_binary_operator(&TokenKind::Equal));
        
        assert!(!is_binary_operator(&TokenKind::LeftParen));
        assert!(!is_binary_operator(&TokenKind::RightParen));
        assert!(!is_binary_operator(&TokenKind::Semicolon));
        assert!(!is_binary_operator(&TokenKind::Identifier));
    }
    
    #[test]
    fn test_is_unary_operator() {
        assert!(is_unary_operator(&TokenKind::Plus));
        assert!(is_unary_operator(&TokenKind::Minus));
        assert!(is_unary_operator(&TokenKind::Bang));
        assert!(is_unary_operator(&TokenKind::Tilde));
        assert!(is_unary_operator(&TokenKind::PlusPlus));
        assert!(is_unary_operator(&TokenKind::MinusMinus));
        
        assert!(!is_unary_operator(&TokenKind::Star)); // これは単項でも二項でもある
        assert!(!is_unary_operator(&TokenKind::EqualEqual));
        assert!(!is_unary_operator(&TokenKind::LeftParen));
    }
    
    #[test]
    fn test_statement_start() {
        assert!(is_statement_start(&TokenKind::KeywordLet));
        assert!(is_statement_start(&TokenKind::KeywordVar));
        assert!(is_statement_start(&TokenKind::KeywordIf));
        assert!(is_statement_start(&TokenKind::KeywordWhile));
        assert!(is_statement_start(&TokenKind::KeywordFor));
        assert!(is_statement_start(&TokenKind::KeywordReturn));
        assert!(is_statement_start(&TokenKind::LeftBrace));
        
        assert!(!is_statement_start(&TokenKind::RightBrace));
        assert!(!is_statement_start(&TokenKind::Identifier));
        assert!(!is_statement_start(&TokenKind::IntLiteral));
    }
}
