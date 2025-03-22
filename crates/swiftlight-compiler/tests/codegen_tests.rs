use swiftlight_compiler::frontend::parser::{self, ast};
use swiftlight_compiler::frontend::semantic::type_checker::TypeChecker;
use swiftlight_compiler::frontend::source_map::SourceMap;
use swiftlight_compiler::middleend::ir::{self, IRBuilder};
use swiftlight_compiler::backend::{self, CodeGenerator, Target};

// ソースコードからLLVMモジュールに変換するヘルパー関数
fn compile_to_llvm(source: &str, target: &Target) -> Result<backend::llvm::LLVMModule, String> {
    let source_map = SourceMap::new();
    let file_id = source_map.add_file("test.swl", source.to_string());
    
    let parse_result = parser::parse(file_id, source, &source_map)
        .map_err(|e| format!("パースエラー: {:?}", e))?;
    
    let mut type_checker = TypeChecker::new(&source_map);
    type_checker.check_module(&parse_result)
        .map_err(|e| format!("型チェックエラー: {:?}", e))?;
    
    if !type_checker.diagnostics().is_empty() {
        return Err(format!("型チェック診断エラー: {:?}", type_checker.diagnostics()));
    }
    
    let ir_builder = IRBuilder::new();
    let ir_module = ir_builder.build_module(&parse_result, &type_checker)
        .map_err(|e| format!("IR生成エラー: {:?}", e))?;
    
    let codegen = backend::llvm::LLVMCodeGenerator::new(target);
    codegen.generate(&ir_module)
        .map_err(|e| format!("コード生成エラー: {:?}", e))
}

// LLVMモジュールが有効かどうか検証するヘルパー関数
fn verify_llvm_module(module: &backend::llvm::LLVMModule) -> Result<(), String> {
    module.verify().map_err(|e| format!("LLVMモジュール検証エラー: {}", e))
}

#[test]
fn test_basic_function_codegen() {
    let source = r#"
        fn add(a: Int, b: Int) -> Int {
            return a + b;
        }
    "#;
    
    let target = Target::default_host();
    let llvm_module = compile_to_llvm(source, &target).expect("LLVM IR生成に失敗");
    
    // LLVMモジュールが有効かを検証
    verify_llvm_module(&llvm_module).expect("LLVMモジュール検証に失敗");
    
    // add関数が存在するか確認
    assert!(llvm_module.has_function("add"), "add関数がLLVMモジュールに存在しません");
}

#[test]
fn test_control_flow_codegen() {
    let source = r#"
        fn abs(x: Int) -> Int {
            if x < 0 {
                return -x;
            } else {
                return x;
            }
        }
    "#;
    
    let target = Target::default_host();
    let llvm_module = compile_to_llvm(source, &target).expect("LLVM IR生成に失敗");
    
    // LLVMモジュールが有効かを検証
    verify_llvm_module(&llvm_module).expect("LLVMモジュール検証に失敗");
    
    // abs関数が存在するか確認
    assert!(llvm_module.has_function("abs"), "abs関数がLLVMモジュールに存在しません");
    
    // LLVM IRテキスト表現に条件分岐が含まれているか確認
    let ir_text = llvm_module.to_string();
    assert!(ir_text.contains("icmp"), "条件比較命令が見つかりません");
    assert!(ir_text.contains("br"), "分岐命令が見つかりません");
}

#[test]
fn test_loop_codegen() {
    let source = r#"
        fn sum(n: Int) -> Int {
            let mut result = 0;
            let mut i = 1;
            
            while i <= n {
                result = result + i;
                i = i + 1;
            }
            
            return result;
        }
    "#;
    
    let target = Target::default_host();
    let llvm_module = compile_to_llvm(source, &target).expect("LLVM IR生成に失敗");
    
    // LLVMモジュールが有効かを検証
    verify_llvm_module(&llvm_module).expect("LLVMモジュール検証に失敗");
    
    // sum関数が存在するか確認
    assert!(llvm_module.has_function("sum"), "sum関数がLLVMモジュールに存在しません");
    
    // LLVM IRテキスト表現にループ構造が含まれているか確認
    let ir_text = llvm_module.to_string();
    assert!(ir_text.contains("icmp"), "条件比較命令が見つかりません");
    assert!(ir_text.contains("br"), "分岐命令が見つかりません");
    
    // ループには通常、後方エッジ（バック・エッジ）がある
    // これは単純なテキスト分析では検証が難しいため、ここでは構造的な検証のみを行う
}

#[test]
fn test_struct_codegen() {
    let source = r#"
        struct Point {
            x: Int,
            y: Int,
        }
        
        fn create_point(x: Int, y: Int) -> Point {
            return Point { x: x, y: y };
        }
        
        fn add_points(p1: Point, p2: Point) -> Point {
            return Point { x: p1.x + p2.x, y: p1.y + p2.y };
        }
    "#;
    
    let target = Target::default_host();
    let llvm_module = compile_to_llvm(source, &target).expect("LLVM IR生成に失敗");
    
    // LLVMモジュールが有効かを検証
    verify_llvm_module(&llvm_module).expect("LLVMモジュール検証に失敗");
    
    // 関数が存在するか確認
    assert!(llvm_module.has_function("create_point"), "create_point関数がLLVMモジュールに存在しません");
    assert!(llvm_module.has_function("add_points"), "add_points関数がLLVMモジュールに存在しません");
    
    // 構造体型が定義されているか確認
    let ir_text = llvm_module.to_string();
    assert!(ir_text.contains("type"), "型定義が見つかりません");
    
    // フィールドアクセスが生成されているか確認
    assert!(ir_text.contains("getelementptr") || ir_text.contains("extractvalue"), 
            "構造体フィールドアクセス命令が見つかりません");
}

#[test]
fn test_array_codegen() {
    let source = r#"
        fn sum_array(arr: [Int; 5]) -> Int {
            let mut sum = 0;
            
            for i in 0..5 {
                sum = sum + arr[i];
            }
            
            return sum;
        }
    "#;
    
    let target = Target::default_host();
    let llvm_module = compile_to_llvm(source, &target).expect("LLVM IR生成に失敗");
    
    // LLVMモジュールが有効かを検証
    verify_llvm_module(&llvm_module).expect("LLVMモジュール検証に失敗");
    
    // sum_array関数が存在するか確認
    assert!(llvm_module.has_function("sum_array"), "sum_array関数がLLVMモジュールに存在しません");
    
    // 配列操作命令が生成されているか確認
    let ir_text = llvm_module.to_string();
    assert!(ir_text.contains("getelementptr") || ir_text.contains("extractvalue"), 
            "配列要素アクセス命令が見つかりません");
}

#[test]
fn test_optimization_codegen() {
    let source = r#"
        fn constant_folding() -> Int {
            return 10 + 20 * 30;  // 最適化により600になるはず
        }
        
        fn dead_code() -> Int {
            let x = 10;
            let y = 20;
            
            // y は使用されないため最適化で削除される可能性がある
            return x;
        }
    "#;
    
    let target = Target::default_host();
    
    // 最適化なしでコンパイル
    let unopt_llvm_module = compile_to_llvm(source, &target).expect("LLVM IR生成に失敗");
    verify_llvm_module(&unopt_llvm_module).expect("LLVMモジュール検証に失敗");
    
    // 最適化ありでコンパイル
    let mut opt_target = target.clone();
    opt_target.optimization_level = backend::OptimizationLevel::Aggressive;
    
    let opt_llvm_module = compile_to_llvm(source, &opt_target).expect("LLVM IR生成に失敗");
    verify_llvm_module(&opt_llvm_module).expect("LLVMモジュール検証に失敗");
    
    // 最適化のレベルによって結果が異なるため、具体的な検証は難しい
    // ここでは単にコンパイルが成功することを確認する
    
    // 定数畳み込みの検証は可能かもしれない
    let opt_ir_text = opt_llvm_module.to_string();
    
    // 最適化レベルによっては、定数畳み込みにより600という値がIRに直接現れる可能性がある
    // ただし、これは最適化の実装に依存するため、確実なテストにはならない
} 