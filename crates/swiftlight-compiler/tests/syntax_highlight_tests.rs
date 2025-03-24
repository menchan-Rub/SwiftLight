use swiftlight_compiler::{
    frontend::{
        lexer::{Lexer, Token},
        parser::{Parser, ast},
        syntax_highlight::{SyntaxHighlighter, SyntaxRangeInfo},
    },
};
use std::collections::HashMap;

#[test]
fn test_basic_syntax_highlighting() {
    let source = r#"
    func add(a: Int, b: Int) -> Int {
        let result = a + b;
        return result;
    }
    "#;

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().unwrap();
    
    let highlighter = SyntaxHighlighter::new(tokens);
    let highlights = highlighter.generate_highlights();
    
    // 基本的なハイライトが生成されることを確認
    assert!(!highlights.is_empty());
    
    // キーワードのハイライトを確認
    let func_token = highlights.iter().find(|h| 
        h.token.text() == "func" && 
        h.highlight_kind.to_string().contains("Keyword")
    );
    assert!(func_token.is_some());
    
    // 演算子のハイライトを確認
    let plus_token = highlights.iter().find(|h| 
        h.token.text() == "+" && 
        h.highlight_kind.to_string().contains("Operator")
    );
    assert!(plus_token.is_some());
}

#[test]
fn test_semantic_highlighting() {
    let source = r#"
    func calculate(x: Int, y: Int) -> Int {
        let sum = x + y;
        let product = x * y;
        return sum + product;
    }
    "#;

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().unwrap();
    
    let mut parser = Parser::new(tokens.clone());
    let ast = parser.parse().unwrap();
    
    // セマンティック情報を手動で設定
    let mut semantic_info = HashMap::new();
    semantic_info.insert("calculate".to_string(), "function".to_string());
    semantic_info.insert("x".to_string(), "parameter".to_string());
    semantic_info.insert("y".to_string(), "parameter".to_string());
    semantic_info.insert("sum".to_string(), "variable".to_string());
    semantic_info.insert("product".to_string(), "variable".to_string());
    
    let highlighter = SyntaxHighlighter::new(tokens)
        .with_ast(ast)
        .with_semantic_info(semantic_info);
    
    let highlights = highlighter.generate_highlights();
    
    // 関数名のハイライトを確認
    let func_name = highlights.iter().find(|h| 
        h.token.text() == "calculate" && 
        h.highlight_kind.to_string().contains("FunctionName")
    );
    assert!(func_name.is_some());
    
    // パラメータのハイライトを確認
    let param = highlights.iter().find(|h| 
        h.token.text() == "x" && 
        h.semantic_info.as_ref().map_or(false, |s| s == "parameter")
    );
    assert!(param.is_some());
    
    // 変数のハイライトを確認
    let variable = highlights.iter().find(|h| 
        h.token.text() == "sum" && 
        h.semantic_info.as_ref().map_or(false, |s| s == "variable")
    );
    assert!(variable.is_some());
}

#[test]
fn test_syntax_ranges() {
    let source = r#"
    struct Point {
        let x: Float;
        let y: Float;
        
        func distance(other: Point) -> Float {
            let dx = self.x - other.x;
            let dy = self.y - other.y;
            return sqrt(dx * dx + dy * dy);
        }
    }
    "#;

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().unwrap();
    
    let mut parser = Parser::new(tokens.clone());
    let ast = parser.parse().unwrap();
    
    let highlighter = SyntaxHighlighter::new(tokens).with_ast(ast);
    let ranges = highlighter.generate_syntax_ranges();
    
    // 構造体名のレンジを確認
    let struct_name = ranges.iter().find(|r| 
        r.semantic_tag.as_ref().map_or(false, |t| t == "type") &&
        r.symbol_info.as_ref().map_or(false, |s| s.name == "Point")
    );
    assert!(struct_name.is_some());
    
    // メソッド名のレンジを確認
    let method_name = ranges.iter().find(|r| 
        r.semantic_tag.as_ref().map_or(false, |t| t == "function") &&
        r.symbol_info.as_ref().map_or(false, |s| s.name == "distance")
    );
    assert!(method_name.is_some());
}

#[test]
fn test_json_output() {
    let source = "let x = 10;";
    
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().unwrap();
    
    let highlighter = SyntaxHighlighter::new(tokens);
    let highlights = highlighter.generate_highlights();
    
    let json = swiftlight_compiler::frontend::syntax_highlight::syntax_highlights_to_json(&highlights);
    
    // JSONが生成されることを確認
    assert!(json.starts_with("["));
    assert!(json.ends_with("]\n"));
    
    // 「let」キーワードがJSON内に含まれることを確認
    assert!(json.contains("\"kind\": \"Keyword\""));
} 