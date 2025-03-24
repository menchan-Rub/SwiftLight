//! # SwiftLight コンパイラフロントエンド
//!
//! このモジュールはソースコードの解析から高レベル中間表現（HIR）生成までの全処理を統括します。
//! 以下の主要コンポーネントから構成され、型安全で最適化可能なコード生成の基盤を提供します。
//!
//! ## コンパイルフェーズ
//! 1. 字句解析（[`lexer`]）
//! 2. 構文解析（[`parser`]）
//! 3. 意味解析（[`semantic`]）
//! 4. 型検査（[`semantic::type_checker`]）
//! 5. HIR生成（[`hir`]）
//!
//! ## 主な機能
//! - マルチスレッド対応の並列解析パイプライン
//! - ソースマップによる精密なエラー位置追跡
//! - 拡張可能な診断システム
//! - モジュール間の依存関係解決
//! - インクリメンタルコンパイルサポート
//!
//! ## アーキテクチャ特性
//! - ゼロコスト抽象化による高効率なメモリ管理
//! - 並行処理向けのロックフリーデータ構造
//! - SIMD最適化された字句解析器
//! - マルチコアを活用した並列型検査

#[doc(hidden)]
pub mod lexer;
#[doc(hidden)]
pub mod parser;
#[doc(hidden)]
pub mod semantic;
#[doc(hidden)]
pub mod ast;
#[doc(hidden)]
pub mod error;
#[doc(hidden)]
pub mod source_map;
#[doc(hidden)]
pub mod diagnostic;
#[doc(hidden)]
pub mod module;
#[doc(hidden)]
pub mod hir;
#[doc(hidden)]
pub mod syntax_highlight;

/// フロントエンドの主要インターフェースを統一的にエクスポート
pub use self::{
    lexer::{Lexer, TokenStream},
    parser::{Parser, SyntaxTree},
    semantic::{
        analyzer::SemanticAnalyzer,
        type_checker::TypeChecker,
        symbol_table::SymbolTable
    },
    ast::{Program, AstNode},
    error::{CompilerError, ErrorKind, Result, DiagnosticBuilder},
    source_map::{SourceMap, Span},
    diagnostic::{Diagnostic, DiagnosticLevel, EmissionHandler},
    module::{Module, ModuleGraph},
    hir::{HirNode, HirTranslator},
    syntax_highlight::{SyntaxHighlighter, SyntaxRangeInfo, SymbolInfo},
    parser::context_parser::{ContextParser, CompletionContext, CompletionContextKind}
};

/// フロントエンド処理パイプラインのエントリーポイント
#[derive(Debug)]
pub struct FrontendPipeline<'a> {
    lexer: Lexer<'a>,
    parser: Parser<'a>,
    semantic_analyzer: SemanticAnalyzer,
    hir_translator: HirTranslator,
    /// ソースコードのメタデータ管理
    pub source_map: Arc<SourceMap>,
    /// モジュール依存関係グラフ
    pub module_graph: RwLock<ModuleGraph>,
    /// 診断メッセージの集約器
    pub diagnostics: DashMap<Span, Diagnostic>
}
