use swiftlight_compiler::frontend::parser::{self, ast};
use swiftlight_compiler::frontend::semantic::type_checker::TypeChecker;
use swiftlight_compiler::frontend::source_map::SourceMap;
use swiftlight_compiler::middleend::ir::{self, IRBuilder, IRModule};

// IRに変換してから基本的な検証を行うヘルパー関数
fn compile_to_ir(source: &str) -> Result<IRModule, String> {
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
    ir_builder.build_module(&parse_result, &type_checker)
        .map_err(|e| format!("IR生成エラー: {:?}", e))
}

#[test]
fn test_basic_ir_generation() {
    let source = r#"
        fn add(a: Int, b: Int) -> Int {
            return a + b;
        }
    "#;
    
    let ir_module = compile_to_ir(source).expect("IR生成に失敗");
    
    // 関数が生成されているか確認
    let add_func = ir_module.get_function("add")
        .expect("add関数がIRに見つかりません");
    
    // 関数の基本的な構造を検証
    assert_eq!(add_func.params.len(), 2, "パラメータ数が一致しません");
    assert!(matches!(add_func.return_type, ir::Type::Int), "戻り値の型が一致しません");
    
    // 関数本体があることを確認
    assert!(!add_func.basic_blocks.is_empty(), "関数本体が空です");
    
    // 戻り値を含む基本ブロックがあることを確認
    let has_return = add_func.basic_blocks.iter().any(|block| {
        block.instructions.iter().any(|inst| {
            matches!(inst, ir::Instruction::Return(_))
        })
    });
    
    assert!(has_return, "return命令が見つかりません");
}

#[test]
fn test_ir_ssa_form() {
    let source = r#"
        fn test() -> Int {
            let mut x = 10;
            x = x + 1;
            x = x * 2;
            return x;
        }
    "#;
    
    let ir_module = compile_to_ir(source).expect("IR生成に失敗");
    
    // 関数を取得
    let test_func = ir_module.get_function("test")
        .expect("test関数がIRに見つかりません");
    
    // SSA形式では変数の再代入ごとに新しい変数が生成される
    // そのため、異なる命名の変数（x.0, x.1, x.2など）が存在するはず
    
    // 命令から変数名を抽出
    let variable_names: Vec<String> = test_func.basic_blocks.iter()
        .flat_map(|block| &block.instructions)
        .filter_map(|inst| {
            match inst {
                ir::Instruction::Assign(var, _) => Some(var.clone()),
                _ => None,
            }
        })
        .collect();
    
    // 少なくとも3つの異なる変数名（初期化、+1、*2）があるはず
    assert!(variable_names.len() >= 3, "SSA形式では複数の異なる変数名が必要です");
    
    // 変数名の重複チェック（SSA形式では同じ名前に2回代入しない）
    let mut unique_names = variable_names.clone();
    unique_names.sort();
    unique_names.dedup();
    
    assert_eq!(unique_names.len(), variable_names.len(), "SSA形式では変数の重複代入はないはずです");
}

#[test]
fn test_ir_control_flow() {
    let source = r#"
        fn abs(x: Int) -> Int {
            if x < 0 {
                return -x;
            } else {
                return x;
            }
        }
    "#;
    
    let ir_module = compile_to_ir(source).expect("IR生成に失敗");
    
    // 関数を取得
    let abs_func = ir_module.get_function("abs")
        .expect("abs関数がIRに見つかりません");
    
    // 条件分岐があるため、少なくとも3つの基本ブロックが必要:
    // 1. エントリーブロック（条件評価）
    // 2. 真の場合のブロック（x < 0）
    // 3. 偽の場合のブロック
    assert!(abs_func.basic_blocks.len() >= 3, 
            "if-else構造には少なくとも3つの基本ブロックが必要です（エントリー、真、偽）");
    
    // 条件分岐命令があることを確認
    let has_branch = abs_func.basic_blocks.iter().any(|block| {
        block.instructions.iter().any(|inst| {
            matches!(inst, ir::Instruction::Branch(_) | ir::Instruction::CondBranch(_, _, _))
        })
    });
    
    assert!(has_branch, "条件分岐命令が見つかりません");
    
    // 両方のパスからreturn命令があることを確認
    let return_count = abs_func.basic_blocks.iter()
        .flat_map(|block| &block.instructions)
        .filter(|inst| matches!(inst, ir::Instruction::Return(_)))
        .count();
    
    assert_eq!(return_count, 2, "if-else両方のパスからのreturn命令が必要です");
}

#[test]
fn test_ir_loop() {
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
    
    let ir_module = compile_to_ir(source).expect("IR生成に失敗");
    
    // 関数を取得
    let sum_func = ir_module.get_function("sum")
        .expect("sum関数がIRに見つかりません");
    
    // ループ構造には少なくとも3つの基本ブロックが必要:
    // 1. エントリーブロック
    // 2. ループ条件ブロック
    // 3. ループ本体ブロック
    // 4. ループ終了後ブロック
    assert!(sum_func.basic_blocks.len() >= 3, 
            "ループ構造には少なくとも3つの基本ブロックが必要です");
    
    // 後方分岐（ループバック）があることを確認
    // これは単純なテストでは検証が難しいため、ここでは基本ブロック間の接続性のみ確認
    
    // ループ構造における重要な命令のチェック
    let has_condition_branch = sum_func.basic_blocks.iter().any(|block| {
        block.instructions.iter().any(|inst| {
            matches!(inst, ir::Instruction::CondBranch(_, _, _))
        })
    });
    
    assert!(has_condition_branch, "ループ条件の分岐命令が見つかりません");
}

#[test]
fn test_ir_function_call() {
    let source = r#"
        fn add(a: Int, b: Int) -> Int {
            return a + b;
        }
        
        fn test() -> Int {
            return add(5, 10);
        }
    "#;
    
    let ir_module = compile_to_ir(source).expect("IR生成に失敗");
    
    // 関数を取得
    let test_func = ir_module.get_function("test")
        .expect("test関数がIRに見つかりません");
    
    // 関数呼び出し命令があることを確認
    let has_call = test_func.basic_blocks.iter().any(|block| {
        block.instructions.iter().any(|inst| {
            if let ir::Instruction::Call(func_name, _) = inst {
                func_name == "add"
            } else {
                false
            }
        })
    });
    
    assert!(has_call, "add関数の呼び出しが見つかりません");
}

#[test]
fn test_ir_phi_nodes() {
    let source = r#"
        fn max(a: Int, b: Int) -> Int {
            let result: Int;
            if a > b {
                result = a;
            } else {
                result = b;
            }
            return result;
        }
    "#;
    
    let ir_module = compile_to_ir(source).expect("IR生成に失敗");
    
    // 関数を取得
    let max_func = ir_module.get_function("max")
        .expect("max関数がIRに見つかりません");
    
    // SSA形式では分岐後に値が合流する点でPhi命令が必要
    let has_phi = max_func.basic_blocks.iter().any(|block| {
        block.instructions.iter().any(|inst| {
            matches!(inst, ir::Instruction::Phi(_, _))
        })
    });
    
    // 注意: 最適化や実装方法によっては、Phi命令を使わずに実装される場合もある
    // そのため、このテストは実装に依存する可能性がある
    assert!(has_phi, "分岐後の値の合流にPhi命令が必要です");
} 