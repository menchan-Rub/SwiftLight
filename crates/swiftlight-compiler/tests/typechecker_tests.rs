use swiftlight_compiler::frontend::{
    lexer::Lexer,
    parser::{Parser, ast::{Program, Declaration, Type, Expression}},
    error::SourceLocation,
    semantic::{
        type_checker::TypeChecker,
        name_resolver::{NameResolver, NameResolutionResult},
        symbol_table::SymbolTable,
    },
};

// モックの名前解決結果とシンボルテーブルを作成するヘルパー関数
fn create_mock_name_resolution() -> (NameResolutionResult, SymbolTable) {
    let name_resolver = NameResolver::new();
    let symbol_table = name_resolver.get_symbol_table().clone();
    (NameResolutionResult::default(), symbol_table)
}

#[test]
fn test_typechecker_basic_types() {
    let source = r#"
    fn test() {
        let a: i32 = 42;
        let b: f64 = 3.14;
        let c: bool = true;
        let d: String = "hello";
    }
    "#;
    
    let lexer = Lexer::new(source, "test.sl");
    let source_location = SourceLocation::new(1, 1, 0, source.len());
    let mut parser = Parser::new(lexer, source_location);
    let program = parser.parse().unwrap();
    
    // モックの名前解決とシンボルテーブルを取得
    let (name_resolution, symbol_table) = create_mock_name_resolution();
    
    // 型チェックの実行
    let mut type_checker = TypeChecker::new(name_resolution, symbol_table);
    let result = type_checker.check_program(&program);
    
    assert!(result.is_ok(), "型チェックに失敗: {:?}", result.err());
}

#[test]
fn test_typechecker_function_call() {
    let source = r#"
    fn add(a: i32, b: i32) -> i32 {
        return a + b;
    }
    
    fn test() {
        let result = add(1, 2);
        let x: i32 = result;
    }
    "#;
    
    let lexer = Lexer::new(source, "test.sl");
    let source_location = SourceLocation::new(1, 1, 0, source.len());
    let mut parser = Parser::new(lexer, source_location);
    let program = parser.parse().unwrap();
    
    // モックの名前解決とシンボルテーブルを取得
    let (name_resolution, symbol_table) = create_mock_name_resolution();
    
    // 型チェックの実行
    let mut type_checker = TypeChecker::new(name_resolution, symbol_table);
    let result = type_checker.check_program(&program);
    
    assert!(result.is_ok(), "型チェックに失敗: {:?}", result.err());
}

#[test]
fn test_typechecker_type_mismatch() {
    let source = r#"
    fn test() {
        let a: i32 = "string"; // エラー: 文字列をi32に代入
    }
    "#;
    
    let lexer = Lexer::new(source, "test.sl");
    let source_location = SourceLocation::new(1, 1, 0, source.len());
    let mut parser = Parser::new(lexer, source_location);
    let program = parser.parse().unwrap();
    
    // モックの名前解決とシンボルテーブルを取得
    let (name_resolution, symbol_table) = create_mock_name_resolution();
    
    // 型チェックの実行
    let mut type_checker = TypeChecker::new(name_resolution, symbol_table);
    let result = type_checker.check_program(&program);
    
    assert!(result.is_err(), "型チェックエラーが期待されたが成功した");
    let error = result.err().unwrap();
    assert!(error.to_string().contains("型の不一致"), "エラーメッセージが型の不一致に関するものでない: {}", error);
}

#[test]
fn test_typechecker_arithmetic_operations() {
    let source = r#"
    fn test() {
        let a: i32 = 10;
        let b: i32 = 20;
        let c = a + b;
        let d = a * b;
        let e = a / b;
        let f = a - b;
        let g = a % b;
    }
    "#;
    
    let lexer = Lexer::new(source, "test.sl");
    let source_location = SourceLocation::new(1, 1, 0, source.len());
    let mut parser = Parser::new(lexer, source_location);
    let program = parser.parse().unwrap();
    
    // モックの名前解決とシンボルテーブルを取得
    let (name_resolution, symbol_table) = create_mock_name_resolution();
    
    // 型チェックの実行
    let mut type_checker = TypeChecker::new(name_resolution, symbol_table);
    let result = type_checker.check_program(&program);
    
    assert!(result.is_ok(), "型チェックに失敗: {:?}", result.err());
}

#[test]
fn test_typechecker_comparison_operations() {
    let source = r#"
    fn test() {
        let a: i32 = 10;
        let b: i32 = 20;
        let c: bool = a < b;
        let d: bool = a <= b;
        let e: bool = a > b;
        let f: bool = a >= b;
        let g: bool = a == b;
        let h: bool = a != b;
    }
    "#;
    
    let lexer = Lexer::new(source, "test.sl");
    let source_location = SourceLocation::new(1, 1, 0, source.len());
    let mut parser = Parser::new(lexer, source_location);
    let program = parser.parse().unwrap();
    
    // モックの名前解決とシンボルテーブルを取得
    let (name_resolution, symbol_table) = create_mock_name_resolution();
    
    // 型チェックの実行
    let mut type_checker = TypeChecker::new(name_resolution, symbol_table);
    let result = type_checker.check_program(&program);
    
    assert!(result.is_ok(), "型チェックに失敗: {:?}", result.err());
}

#[test]
fn test_typechecker_logic_operations() {
    let source = r#"
    fn test() {
        let a: bool = true;
        let b: bool = false;
        let c: bool = a && b;
        let d: bool = a || b;
        let e: bool = !a;
    }
    "#;
    
    let lexer = Lexer::new(source, "test.sl");
    let source_location = SourceLocation::new(1, 1, 0, source.len());
    let mut parser = Parser::new(lexer, source_location);
    let program = parser.parse().unwrap();
    
    // モックの名前解決とシンボルテーブルを取得
    let (name_resolution, symbol_table) = create_mock_name_resolution();
    
    // 型チェックの実行
    let mut type_checker = TypeChecker::new(name_resolution, symbol_table);
    let result = type_checker.check_program(&program);
    
    assert!(result.is_ok(), "型チェックに失敗: {:?}", result.err());
}

#[test]
fn test_typechecker_if_statement() {
    let source = r#"
    fn test() {
        let a: i32 = 10;
        
        if a > 5 {
            let b: i32 = 20;
        } else {
            let b: i32 = 30;
        }
    }
    "#;
    
    let lexer = Lexer::new(source, "test.sl");
    let source_location = SourceLocation::new(1, 1, 0, source.len());
    let mut parser = Parser::new(lexer, source_location);
    let program = parser.parse().unwrap();
    
    // モックの名前解決とシンボルテーブルを取得
    let (name_resolution, symbol_table) = create_mock_name_resolution();
    
    // 型チェックの実行
    let mut type_checker = TypeChecker::new(name_resolution, symbol_table);
    let result = type_checker.check_program(&program);
    
    assert!(result.is_ok(), "型チェックに失敗: {:?}", result.err());
}

#[test]
fn test_typechecker_if_condition_type() {
    let source = r#"
    fn test() {
        let a: i32 = 10;
        
        if a { // エラー: 条件式はbool型である必要がある
            let b: i32 = 20;
        }
    }
    "#;
    
    let lexer = Lexer::new(source, "test.sl");
    let source_location = SourceLocation::new(1, 1, 0, source.len());
    let mut parser = Parser::new(lexer, source_location);
    let program = parser.parse().unwrap();
    
    // モックの名前解決とシンボルテーブルを取得
    let (name_resolution, symbol_table) = create_mock_name_resolution();
    
    // 型チェックの実行
    let mut type_checker = TypeChecker::new(name_resolution, symbol_table);
    let result = type_checker.check_program(&program);
    
    assert!(result.is_err(), "型チェックエラーが期待されたが成功した");
    let error = result.err().unwrap();
    assert!(error.to_string().contains("条件式"), "エラーメッセージが条件式に関するものでない: {}", error);
}

#[test]
fn test_typechecker_struct() {
    let source = r#"
    struct Point {
        x: f64,
        y: f64,
    }
    
    fn test() {
        let p = Point { x: 1.0, y: 2.0 };
        let a: f64 = p.x;
        let b: f64 = p.y;
    }
    "#;
    
    let lexer = Lexer::new(source, "test.sl");
    let source_location = SourceLocation::new(1, 1, 0, source.len());
    let mut parser = Parser::new(lexer, source_location);
    let program = parser.parse().unwrap();
    
    // モックの名前解決とシンボルテーブルを取得
    let (name_resolution, symbol_table) = create_mock_name_resolution();
    
    // 型チェックの実行
    let mut type_checker = TypeChecker::new(name_resolution, symbol_table);
    let result = type_checker.check_program(&program);
    
    assert!(result.is_ok(), "型チェックに失敗: {:?}", result.err());
}

#[test]
fn test_typechecker_struct_field_access() {
    let source = r#"
    struct Point {
        x: f64,
        y: f64,
    }
    
    fn test() {
        let p = Point { x: 1.0, y: 2.0 };
        let a: i32 = p.z; // エラー: 存在しないフィールド
    }
    "#;
    
    let lexer = Lexer::new(source, "test.sl");
    let source_location = SourceLocation::new(1, 1, 0, source.len());
    let mut parser = Parser::new(lexer, source_location);
    let program = parser.parse().unwrap();
    
    // モックの名前解決とシンボルテーブルを取得
    let (name_resolution, symbol_table) = create_mock_name_resolution();
    
    // 型チェックの実行
    let mut type_checker = TypeChecker::new(name_resolution, symbol_table);
    let result = type_checker.check_program(&program);
    
    assert!(result.is_err(), "型チェックエラーが期待されたが成功した");
    let error = result.err().unwrap();
    assert!(error.to_string().contains("フィールド") || error.to_string().contains("field"), 
            "エラーメッセージがフィールドに関するものでない: {}", error);
}

#[test]
fn test_typechecker_function_return_type() {
    let source = r#"
    fn get_value() -> i32 {
        return "string"; // エラー: 戻り値の型が一致しない
    }
    "#;
    
    let lexer = Lexer::new(source, "test.sl");
    let source_location = SourceLocation::new(1, 1, 0, source.len());
    let mut parser = Parser::new(lexer, source_location);
    let program = parser.parse().unwrap();
    
    // モックの名前解決とシンボルテーブルを取得
    let (name_resolution, symbol_table) = create_mock_name_resolution();
    
    // 型チェックの実行
    let mut type_checker = TypeChecker::new(name_resolution, symbol_table);
    let result = type_checker.check_program(&program);
    
    assert!(result.is_err(), "型チェックエラーが期待されたが成功した");
    let error = result.err().unwrap();
    assert!(error.to_string().contains("戻り値") || error.to_string().contains("return"), 
            "エラーメッセージが戻り値に関するものでない: {}", error);
}

#[test]
fn test_typechecker_function_parameter_type() {
    let source = r#"
    fn sum(a: i32, b: i32) -> i32 {
        return a + b;
    }
    
    fn test() {
        let result = sum("string", 10); // エラー: 引数の型が一致しない
    }
    "#;
    
    let lexer = Lexer::new(source, "test.sl");
    let source_location = SourceLocation::new(1, 1, 0, source.len());
    let mut parser = Parser::new(lexer, source_location);
    let program = parser.parse().unwrap();
    
    // モックの名前解決とシンボルテーブルを取得
    let (name_resolution, symbol_table) = create_mock_name_resolution();
    
    // 型チェックの実行
    let mut type_checker = TypeChecker::new(name_resolution, symbol_table);
    let result = type_checker.check_program(&program);
    
    assert!(result.is_err(), "型チェックエラーが期待されたが成功した");
    let error = result.err().unwrap();
    assert!(error.to_string().contains("引数") || error.to_string().contains("parameter"), 
            "エラーメッセージが引数に関するものでない: {}", error);
}
