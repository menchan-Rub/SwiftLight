// SwiftLight中間表現生成モジュール
//
// このモジュールは、AST（抽象構文木）から中間表現（IR）を生成する機能を提供します。
// 主な役割は以下の通りです：
// - 型チェック済みASTの変換
// - LLVM IR生成の準備
// - 最適化パスの実行
// - データフロー解析

pub mod ir;
pub mod analysis;
pub mod optimization;

use crate::frontend::ast::Program;
use crate::frontend::error::{Result, CompilerError};
use crate::frontend::semantic::type_checker::TypeCheckResult;

/// 中間表現生成のメイン関数
/// 
/// この関数はフロントエンドで構築されたAST（抽象構文木）を受け取り、
/// 中間表現（LLVM IR）に変換します。
pub fn generate_ir(program: &Program, type_info: &TypeCheckResult) -> Result<ir::Module> {
    // IRジェネレーターを初期化
    let mut generator = ir::IRGenerator::new(type_info);
    
    // プログラムからIRモジュールを生成
    let ir_module = generator.generate_module(program)?;
    
    // 最適化パスを実行（必要に応じて）
    let optimized_module = optimization::optimize_module(ir_module)?;
    
    Ok(optimized_module)
}

/// 最適化レベル
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptimizationLevel {
    /// 最適化なし（デバッグビルド用）
    None,
    /// 基本的な最適化
    Basic,
    /// 標準的な最適化
    Standard,
    /// 積極的な最適化
    Aggressive,
}

impl Default for OptimizationLevel {
    fn default() -> Self {
        OptimizationLevel::Standard
    }
}
