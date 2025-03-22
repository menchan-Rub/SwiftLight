use swiftlight_compiler::frontend::parser::{self, ast};
use swiftlight_compiler::frontend::semantic::type_checker::TypeChecker;
use swiftlight_compiler::frontend::source_map::SourceMap;

#[test]
fn test_basic_type_checking() {
    let source = r#"
        fn test() {
            let x: Int = 42;
            let y: String = "hello";
            let z: Bool = true;
        }
    "#;
    
    let source_map = SourceMap::new();
    let file_id = source_map.add_file("test.swl", source.to_string());
    
    let parse_result = parser::parse(file_id, source, &source_map);
    assert!(parse_result.is_ok(), "パースエラー: {:?}", parse_result.err());
    
    let ast = parse_result.unwrap();
    
    let mut type_checker = TypeChecker::new(&source_map);
    let result = type_checker.check_module(&ast);
    
    assert!(result.is_ok(), "型チェックエラー: {:?}", result.err());
    let diagnostics = type_checker.diagnostics();
    assert!(diagnostics.is_empty(), "型チェックで診断が発生: {:?}", diagnostics);
}

#[test]
fn test_type_inference() {
    let source = r#"
        fn test() {
            let x = 42;         // Int型と推論されるべき
            let y = "hello";    // String型と推論されるべき
            let z = true;       // Bool型と推論されるべき
            
            let a = x + 10;     // Int型と推論されるべき
            let b = y + " world"; // String型と推論されるべき
            let c = !z;         // Bool型と推論されるべき
        }
    "#;
    
    let source_map = SourceMap::new();
    let file_id = source_map.add_file("test.swl", source.to_string());
    
    let parse_result = parser::parse(file_id, source, &source_map);
    assert!(parse_result.is_ok(), "パースエラー: {:?}", parse_result.err());
    
    let ast = parse_result.unwrap();
    
    let mut type_checker = TypeChecker::new(&source_map);
    let result = type_checker.check_module(&ast);
    
    assert!(result.is_ok(), "型チェックエラー: {:?}", result.err());
    let diagnostics = type_checker.diagnostics();
    assert!(diagnostics.is_empty(), "型チェックで診断が発生: {:?}", diagnostics);
    
    // 推論された型を検証するには、実際の実装に依存
    // ここでは型チェック自体の成功を検証
}

#[test]
fn test_type_mismatches() {
    let source = r#"
        fn test() {
            let x: String = 42;  // 型の不一致
        }
    "#;
    
    let source_map = SourceMap::new();
    let file_id = source_map.add_file("test.swl", source.to_string());
    
    let parse_result = parser::parse(file_id, source, &source_map);
    assert!(parse_result.is_ok(), "パースエラー: {:?}", parse_result.err());
    
    let ast = parse_result.unwrap();
    
    let mut type_checker = TypeChecker::new(&source_map);
    let result = type_checker.check_module(&ast);
    
    // 型の不一致によりエラーが発生するはず
    assert!(result.is_err() || !type_checker.diagnostics().is_empty(), 
           "型の不一致がエラーとして検出されませんでした");
}

#[test]
fn test_function_return_type() {
    let source = r#"
        fn add(a: Int, b: Int) -> Int {
            return a + b;
        }
        
        fn greet() -> String {
            return "Hello";
        }
        
        fn wrong_return() -> String {
            return 42;  // 戻り値の型が不一致
        }
    "#;
    
    let source_map = SourceMap::new();
    let file_id = source_map.add_file("test.swl", source.to_string());
    
    let parse_result = parser::parse(file_id, source, &source_map);
    assert!(parse_result.is_ok(), "パースエラー: {:?}", parse_result.err());
    
    let ast = parse_result.unwrap();
    
    let mut type_checker = TypeChecker::new(&source_map);
    let result = type_checker.check_module(&ast);
    
    // 戻り値の型の不一致によりエラーが発生するはず
    assert!(result.is_err() || !type_checker.diagnostics().is_empty(), 
           "戻り値の型の不一致がエラーとして検出されませんでした");
}

#[test]
fn test_generic_functions() {
    let source = r#"
        // ジェネリック関数の定義
        fn identity<T>(x: T) -> T {
            return x;
        }
        
        fn test() {
            let a = identity(42);       // Int型と推論されるべき
            let b = identity("hello");  // String型と推論されるべき
            let c = identity(true);     // Bool型と推論されるべき
        }
    "#;
    
    let source_map = SourceMap::new();
    let file_id = source_map.add_file("test.swl", source.to_string());
    
    let parse_result = parser::parse(file_id, source, &source_map);
    assert!(parse_result.is_ok(), "パースエラー: {:?}", parse_result.err());
    
    let ast = parse_result.unwrap();
    
    let mut type_checker = TypeChecker::new(&source_map);
    let result = type_checker.check_module(&ast);
    
    assert!(result.is_ok(), "型チェックエラー: {:?}", result.err());
    let diagnostics = type_checker.diagnostics();
    assert!(diagnostics.is_empty(), "型チェックで診断が発生: {:?}", diagnostics);
}

#[test]
fn test_ownership_and_borrowing() {
    let source = r#"
        fn test() {
            let mut x = 42;
            let y = &x;     // 不変参照
            let z = &mut x; // 可変参照
            
            // エラーとなるべき: 可変参照がある間の不変参照の使用
            println(*y);
        }
    "#;
    
    let source_map = SourceMap::new();
    let file_id = source_map.add_file("test.swl", source.to_string());
    
    let parse_result = parser::parse(file_id, source, &source_map);
    assert!(parse_result.is_ok(), "パースエラー: {:?}", parse_result.err());
    
    let ast = parse_result.unwrap();
    
    let mut type_checker = TypeChecker::new(&source_map);
    let result = type_checker.check_module(&ast);
    
    // 所有権規則の違反によりエラーが発生するはず
    assert!(result.is_err() || !type_checker.diagnostics().is_empty(), 
           "所有権/借用規則の違反がエラーとして検出されませんでした");
}

#[test]
fn test_traits_and_impls() {
    let source = r#"
        // トレイト定義
        trait Printable {
            fn print(&self) -> String;
        }
        
        // 構造体定義
        struct Person {
            name: String,
            age: Int,
        }
        
        // 実装
        impl Printable for Person {
            fn print(&self) -> String {
                return "Person: " + self.name + ", " + toString(self.age);
            }
        }
        
        fn test() {
            let p = Person { name: "Alice", age: 30 };
            let result = p.print();
            assert(result == "Person: Alice, 30");
        }
    "#;
    
    let source_map = SourceMap::new();
    let file_id = source_map.add_file("test.swl", source.to_string());
    
    let parse_result = parser::parse(file_id, source, &source_map);
    assert!(parse_result.is_ok(), "パースエラー: {:?}", parse_result.err());
    
    let ast = parse_result.unwrap();
    
    let mut type_checker = TypeChecker::new(&source_map);
    let result = type_checker.check_module(&ast);
    
    assert!(result.is_ok(), "型チェックエラー: {:?}", result.err());
    let diagnostics = type_checker.diagnostics();
    assert!(diagnostics.is_empty(), "型チェックで診断が発生: {:?}", diagnostics);
}

#[test]
fn test_dependent_types() {
    let source = r#"
        // 依存型の例: 長さNの配列
        struct Array<T, const N: Int> {
            elements: [T; N],
        }
        
        fn test() {
            let arr1: Array<Int, 5> = Array { elements: [1, 2, 3, 4, 5] };
            let arr2: Array<String, 3> = Array { elements: ["a", "b", "c"] };
            
            // エラーとなるべき: 長さが一致しない
            let arr3: Array<Int, 4> = Array { elements: [1, 2, 3] };
        }
    "#;
    
    let source_map = SourceMap::new();
    let file_id = source_map.add_file("test.swl", source.to_string());
    
    let parse_result = parser::parse(file_id, source, &source_map);
    assert!(parse_result.is_ok(), "パースエラー: {:?}", parse_result.err());
    
    let ast = parse_result.unwrap();
    
    let mut type_checker = TypeChecker::new(&source_map);
    let result = type_checker.check_module(&ast);
    
    // 依存型の制約違反によりエラーが発生するはず
    assert!(result.is_err() || !type_checker.diagnostics().is_empty(), 
           "依存型の制約違反がエラーとして検出されませんでした");
} 