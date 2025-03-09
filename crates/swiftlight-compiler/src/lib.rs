// SwiftLight Compiler Library
// 言語コンパイラのメインライブラリ

//! # SwiftLight Compiler
//! 
//! SwiftLight言語のコンパイラライブラリです。
//! このライブラリは、SwiftLight言語のソースコードを解析し、
//! 中間表現(IR)を経由してLLVMバックエンドを通じて
//! ネイティブコードまたはその他のターゲットにコンパイルします。

// 標準ライブラリのインポート
use std::error::Error;
use std::path::Path;
use std::fs;

// 外部クレートのインポート
// 必要に応じて追加予定

// 内部モジュールの宣言
pub mod frontend;
pub mod middleend;
pub mod backend;
pub mod driver;

// 再エクスポート
pub use self::driver::Driver;
pub use self::frontend::ast;
pub use self::frontend::parser;
pub use self::frontend::lexer;
pub use self::frontend::error::{CompilerError, ErrorKind, Result};

/// コンパイラのバージョン
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// SwiftLightのコンパイル処理を実行する主要関数
/// 
/// # 引数
/// 
/// * `input_path` - ソースファイルまたはディレクトリのパス
/// * `output_path` - 出力先のパス
/// * `options` - コンパイルオプション
/// 
/// # 戻り値
/// 
/// * `Result<(), Box<dyn Error>>` - 成功時は`()`、失敗時はエラー
pub fn compile<P: AsRef<Path>>(
    input_path: P,
    output_path: P,
    options: driver::CompileOptions,
) -> Result<(), Box<dyn Error>> {
    let driver = Driver::new(options);
    driver.compile(input_path, output_path)
}

/// ソースコードから抽象構文木(AST)を生成する
/// 
/// # 引数
/// 
/// * `source` - SwiftLightのソースコード
/// * `file_name` - ソースファイル名（エラーメッセージに使用）
/// 
/// # 戻り値
/// 
/// * `Result<ast::Program>` - 成功時はAST、失敗時はエラー
pub fn parse_source(source: &str, file_name: &str) -> frontend::error::Result<frontend::ast::Program> {
    let lexer = frontend::lexer::Lexer::new(source, file_name);
    let mut parser = frontend::parser::Parser::new(lexer);
    parser.parse_program()
}

/// ソースファイルから抽象構文木(AST)を生成する
/// 
/// # 引数
/// 
/// * `file_path` - ソースファイルのパス
/// 
/// # 戻り値
/// 
/// * `Result<ast::Program>` - 成功時はAST、失敗時はエラー
pub fn parse_file<P: AsRef<Path>>(file_path: P) -> frontend::error::Result<frontend::ast::Program> {
    let path = file_path.as_ref();
    let source = fs::read_to_string(path)
        .map_err(|e| frontend::error::CompilerError::new(
            frontend::error::ErrorKind::IO,
            format!("ファイル読み込みエラー: {}", e),
            None
        ))?;
    
    parse_source(&source, path.to_string_lossy().as_ref())
}

/// コンパイラのテスト用ヘルパー関数
#[cfg(test)]
pub mod tests {
    use super::*;
    
    /// テスト用にソースコードからASTを生成し、検証する
    pub fn parse_and_validate(source: &str) -> frontend::error::Result<frontend::ast::Program> {
        let ast = parse_source(source, "<テスト>")?;
        // TODO: ここに追加の検証ロジックを実装
        Ok(ast)
    }
}
