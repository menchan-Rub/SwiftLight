use swiftlight_compiler::frontend::lexer::{self, Token, TokenKind};

#[test]
fn test_lexer_simple_tokens() {
    let source = "let x: Int = 42;";
    let tokens = lexer::tokenize(source, "test.swl").unwrap();
    
    let expected_tokens = vec![
        TokenKind::Let,
        TokenKind::Identifier,
        TokenKind::Colon,
        TokenKind::Identifier,
        TokenKind::Equal,
        TokenKind::IntegerLiteral,
        TokenKind::Semicolon,
        TokenKind::Eof,
    ];
    
    assert_eq!(tokens.len(), expected_tokens.len());
    
    for (token, expected_kind) in tokens.iter().zip(expected_tokens.iter()) {
        assert_eq!(&token.kind, expected_kind);
    }
}

#[test]
fn test_lexer_string_literals() {
    let source = "let message = \"Hello, World!\";";
    let tokens = lexer::tokenize(source, "test.swl").unwrap();
    
    // 文字列リテラルトークンの位置を特定
    let string_literal_idx = tokens.iter()
        .position(|t| matches!(t.kind, TokenKind::StringLiteral))
        .unwrap();
    
    // 文字列リテラルトークンのコンテンツを確認
    let string_token = &tokens[string_literal_idx];
    assert_eq!(string_token.span.extract_source(source), "\"Hello, World!\"");
}

#[test]
fn test_lexer_keywords() {
    let source = "if true { return false; } else { let x = 10; }";
    let tokens = lexer::tokenize(source, "test.swl").unwrap();
    
    let expected_keywords = vec![
        TokenKind::If,
        TokenKind::True,
        TokenKind::Return,
        TokenKind::False,
        TokenKind::Else,
        TokenKind::Let,
    ];
    
    let actual_keywords: Vec<_> = tokens.iter()
        .filter(|t| t.kind.is_keyword())
        .map(|t| &t.kind)
        .collect();
    
    assert_eq!(actual_keywords, expected_keywords);
}

#[test]
fn test_lexer_operators() {
    let source = "a + b - c * d / e % f && g || h == i != j < k <= l > m >= n";
    let tokens = lexer::tokenize(source, "test.swl").unwrap();
    
    let expected_operators = vec![
        TokenKind::Plus,
        TokenKind::Minus,
        TokenKind::Star,
        TokenKind::Slash,
        TokenKind::Percent,
        TokenKind::AmpAmp,
        TokenKind::PipePipe,
        TokenKind::EqualEqual,
        TokenKind::BangEqual,
        TokenKind::Less,
        TokenKind::LessEqual,
        TokenKind::Greater,
        TokenKind::GreaterEqual,
    ];
    
    let actual_operators: Vec<_> = tokens.iter()
        .filter(|t| t.kind.is_operator())
        .map(|t| &t.kind)
        .collect();
    
    assert_eq!(actual_operators, expected_operators);
}

#[test]
fn test_lexer_error_handling() {
    let source = "let x = @invalid;";
    let result = lexer::tokenize(source, "test.swl");
    
    assert!(result.is_err(), "不正なトークンに対してエラーが返されるべき");
}

#[test]
fn test_lexer_unterminated_string() {
    let source = "let message = \"Hello, World!";
    let result = lexer::tokenize(source, "test.swl");
    
    assert!(result.is_err(), "終了していない文字列リテラルに対してエラーが返されるべき");
}

#[test]
fn test_lexer_comments() {
    let source = "
        // これは行コメントです
        let x = 10; /* これはブロックコメントです */
        /* 複数行の
           ブロックコメント
        */
        let y = 20;
    ";
    
    let tokens = lexer::tokenize(source, "test.swl").unwrap();
    
    // コメントは無視されるはず
    let identifiers: Vec<_> = tokens.iter()
        .filter(|t| matches!(t.kind, TokenKind::Identifier))
        .map(|t| t.span.extract_source(source))
        .collect();
    
    assert_eq!(identifiers, vec!["x", "y"]);
}
