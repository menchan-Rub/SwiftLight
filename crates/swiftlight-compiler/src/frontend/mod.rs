//! # SwiftLight コンパイラフロントエンド
//! 
//! フロントエンドモジュールはソースコードの解析、抽象構文木(AST)の生成、
//! 名前解決、型検査など、コンパイルの初期段階を担当します。

// サブモジュールの宣言
pub mod lexer;
pub mod parser;
pub mod semantic;
pub mod ast;
pub mod error;
pub mod source_map;
pub mod diagnostic;

// 再エクスポート
pub use self::lexer::Lexer;
pub use self::parser::Parser;
pub use self::semantic::analyzer::SemanticAnalyzer;
pub use self::ast::Program;
pub use self::error::{CompilerError, ErrorKind, Result};
pub use self::source_map::SourceMap;
pub use self::diagnostic::{Diagnostic, DiagnosticLevel};
