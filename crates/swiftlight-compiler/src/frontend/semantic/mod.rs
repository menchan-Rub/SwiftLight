//! # 意味解析モジュール
//! 
//! SwiftLight言語の意味解析を担当するモジュールです。
//! 構文解析で生成された抽象構文木（AST）に対して、
//! 名前解決、型検査、意味検証などを行います。

use crate::frontend::ast::{Expression, Program, Statement, NodeId};
use crate::frontend::error::{CompilerError, ErrorKind, Result, Diagnostic};
use crate::frontend::source_map::SourceMap;

// サブモジュール
pub mod name_resolver;
pub mod type_checker;
pub mod ownership_checker;
pub mod dependent_type_checker;
pub mod analyzer;
pub mod scope;
pub mod symbol_table;
pub mod name_resolution;

// 再エクスポート
pub use self::analyzer::SemanticAnalyzer;
pub use self::analyzer::AnalysisResult;
pub use self::symbol_table::{SymbolTable, Symbol, SymbolKind, Visibility};
pub use self::scope::{ScopeManager, ScopeKind};
pub use self::name_resolver::NameResolver;

/// 意味解析を行う
///
/// プログラムの意味解析を行い、名前解決や型チェックなどを実行します。
pub fn analyze(program: Program) -> Result<Program> {
    let mut analyzer = SemanticAnalyzer::new();
    let result = analyzer.analyze(program);
    
    // 結果をResultに変換
    result.into_result()
}

/// ノードIDからそのノードが参照するシンボルのノードIDを取得
pub fn get_reference(result: &AnalysisResult, node_id: NodeId) -> Option<NodeId> {
    result.name_resolution.resolved_nodes.get(&node_id).copied()
}
