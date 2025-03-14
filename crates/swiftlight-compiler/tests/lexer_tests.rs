use swiftlight_compiler::frontend::lexer::{Lexer, token::TokenKind};

#[test]
fn test_lexer_basic_tokens() {
    let source = r#"
    fn main() {
        let x = 42;
        let y = "hello";
        if x > 10 {
            println(y);
        }
    }
    "#;
    
    let mut lexer = Lexer::new(source, "test.sl");
    let tokens: Vec<_> = lexer.collect();
    
    // 期待されるトークン数をチェック
    assert!(tokens.len() > 20, "トークンが少なすぎます: {}", tokens.len());
    
    // 基本的なトークンタイプの検証
    let token_kinds: Vec<_> = tokens.iter().map(|t| t.kind).collect();
    
    assert!(token_kinds.contains(&TokenKind::Fn));
    assert!(token_kinds.contains(&TokenKind::Identifier));
    assert!(token_kinds.contains(&TokenKind::LeftParen));
    assert!(token_kinds.contains(&TokenKind::RightParen));
    assert!(token_kinds.contains(&TokenKind::LeftBrace));
    assert!(token_kinds.contains(&TokenKind::RightBrace));
    assert!(token_kinds.contains(&TokenKind::Let));
    assert!(token_kinds.contains(&TokenKind::Equals));
    assert!(token_kinds.contains(&TokenKind::IntLiteral));
    assert!(token_kinds.contains(&TokenKind::StringLiteral));
    assert!(token_kinds.contains(&TokenKind::Semicolon));
    assert!(token_kinds.contains(&TokenKind::If));
    assert!(token_kinds.contains(&TokenKind::GreaterThan));
}

#[test]
fn test_lexer_position_tracking() {
    let source = "let x = 10;\nlet y = 20;";
    let mut lexer = Lexer::new(source, "test.sl");
    
    // 最初の行のトークンを検証
    let tok1 = lexer.next().unwrap();
    assert_eq!(tok1.kind, TokenKind::Let);
    assert_eq!(tok1.location.line, 1);
    
    let tok2 = lexer.next().unwrap();
    assert_eq!(tok2.kind, TokenKind::Identifier);
    assert_eq!(tok2.location.line, 1);
    
    // セミコロンまでスキップ
    let mut last_token = tok2;
    while last_token.kind != TokenKind::Semicolon {
        last_token = lexer.next().unwrap();
    }
    
    // 2行目の先頭のトークンをチェック
    let tok_next_line = lexer.next().unwrap();
    assert_eq!(tok_next_line.kind, TokenKind::Let);
    assert_eq!(tok_next_line.location.line, 2);
}

#[test]
fn test_lexer_error_handling() {
    let source = "let x = @invalid;";
    let mut lexer = Lexer::new(source, "test.sl");
    
    // 無効なトークンまで進める
    let tok1 = lexer.next().unwrap();
    assert_eq!(tok1.kind, TokenKind::Let);
    
    let tok2 = lexer.next().unwrap();
    assert_eq!(tok2.kind, TokenKind::Identifier);
    
    let tok3 = lexer.next().unwrap();
    assert_eq!(tok3.kind, TokenKind::Equals);
    
    // 無効なトークンを検出
    let error_token = lexer.next().unwrap();
    assert_eq!(error_token.kind, TokenKind::Error);
}

#[test]
fn test_lexer_keywords() {
    let source = "fn let if else while for return break continue struct trait impl self Self";
    let mut lexer = Lexer::new(source, "test.sl");
    
    let expected_kinds = vec![
        TokenKind::Fn,
        TokenKind::Let,
        TokenKind::If,
        TokenKind::Else,
        TokenKind::While,
        TokenKind::For,
        TokenKind::Return,
        TokenKind::Break,
        TokenKind::Continue,
        TokenKind::Struct,
        TokenKind::Trait,
        TokenKind::Impl,
        TokenKind::SelfLower,
        TokenKind::SelfUpper,
    ];
    
    for expected in expected_kinds {
        let token = lexer.next().unwrap();
        assert_eq!(token.kind, expected, "期待: {:?}, 実際: {:?}", expected, token.kind);
    }
}

#[test]
fn test_lexer_number_literals() {
    let source = "123 0xff 0b1010 123.456 1e10 1.5e-4";
    let mut lexer = Lexer::new(source, "test.sl");
    
    // 整数リテラル
    let tok1 = lexer.next().unwrap();
    assert_eq!(tok1.kind, TokenKind::IntLiteral);
    
    // 16進数リテラル
    let tok2 = lexer.next().unwrap();
    assert_eq!(tok2.kind, TokenKind::IntLiteral);
    
    // 2進数リテラル
    let tok3 = lexer.next().unwrap();
    assert_eq!(tok3.kind, TokenKind::IntLiteral);
    
    // 小数点リテラル
    let tok4 = lexer.next().unwrap();
    assert_eq!(tok4.kind, TokenKind::FloatLiteral);
    
    // 指数表記リテラル
    let tok5 = lexer.next().unwrap();
    assert_eq!(tok5.kind, TokenKind::FloatLiteral);
    
    // 負の指数表記リテラル
    let tok6 = lexer.next().unwrap();
    assert_eq!(tok6.kind, TokenKind::FloatLiteral);
}

#[test]
fn test_lexer_operators() {
    let source = "+ - * / % == != < <= > >= && || ! << >> & | ^ ~ = += -= *= /= %= &= |= ^= <<= >>=";
    let mut lexer = Lexer::new(source, "test.sl");
    
    let expected_kinds = vec![
        TokenKind::Plus,
        TokenKind::Minus,
        TokenKind::Star,
        TokenKind::Slash,
        TokenKind::Percent,
        TokenKind::EqualsEquals,
        TokenKind::BangEquals,
        TokenKind::LessThan,
        TokenKind::LessThanEquals,
        TokenKind::GreaterThan,
        TokenKind::GreaterThanEquals,
        TokenKind::AmpersandAmpersand,
        TokenKind::PipePipe,
        TokenKind::Bang,
        TokenKind::LessThanLessThan,
        TokenKind::GreaterThanGreaterThan,
        TokenKind::Ampersand,
        TokenKind::Pipe,
        TokenKind::Caret,
        TokenKind::Tilde,
        TokenKind::Equals,
        TokenKind::PlusEquals,
        TokenKind::MinusEquals,
        TokenKind::StarEquals,
        TokenKind::SlashEquals,
        TokenKind::PercentEquals,
        TokenKind::AmpersandEquals,
        TokenKind::PipeEquals,
        TokenKind::CaretEquals,
        TokenKind::LessThanLessThanEquals,
        TokenKind::GreaterThanGreaterThanEquals,
    ];
    
    for expected in expected_kinds {
        let token = lexer.next().unwrap();
        assert_eq!(token.kind, expected, "期待: {:?}, 実際: {:?}", expected, token.kind);
    }
}
