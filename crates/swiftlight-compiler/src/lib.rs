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
    use crate::frontend::semantic::{
        name_resolver::NameResolver,
        type_checker::TypeChecker,
        ownership_checker::OwnershipChecker
    };
    use crate::frontend::error::CompilerError;
    
    /// テスト用にソースコードからASTを生成し、検証する
    pub fn parse_and_validate(source: &str) -> frontend::error::Result<frontend::ast::Program> {
        // ソースからASTを生成
        let mut ast = parse_source(source, "<テスト>")?;
        
        // 名前解決の実行
        let mut name_resolver = NameResolver::new();
        name_resolver.resolve(&mut ast)
            .map_err(|e| CompilerError::semantic_error(
                format!("名前解決エラー: {}", e),
                None
            ))?;
        
        // 型チェックの実行
        let mut type_checker = TypeChecker::new();
        type_checker.check(&ast)
            .map_err(|e| CompilerError::semantic_error(
                format!("型チェックエラー: {}", e),
                None
            ))?;
        
        // 所有権チェックの実行
        let mut ownership_checker = OwnershipChecker::new();
        ownership_checker.check(&ast)
            .map_err(|e| CompilerError::semantic_error(
                format!("所有権チェックエラー: {}", e),
                None
            ))?;
        
        // 追加の検証 - 循環依存性チェック
        check_circular_dependencies(&ast)
            .map_err(|e| CompilerError::semantic_error(
                format!("循環依存性エラー: {}", e),
                None
            ))?;
        
        // 追加の検証 - 未使用識別子の警告
        check_unused_identifiers(&ast);
        
        Ok(ast)
    }
    
    /// ASTの循環依存関係をチェック
    fn check_circular_dependencies(ast: &frontend::ast::Program) -> Result<(), String> {
        // モジュール依存関係グラフの構築
        let mut dependencies = std::collections::HashMap::new();
        
        // モジュール依存関係の収集
        for decl in &ast.declarations {
            if let frontend::ast::DeclarationKind::Import(import) = &decl.kind {
                let importing_module = decl.module_id.unwrap_or(0);
                let imported_module = import.module_id;
                
                dependencies
                    .entry(importing_module)
                    .or_insert_with(std::collections::HashSet::new)
                    .insert(imported_module);
            }
        }
        
        // 循環検出
        let mut visited = std::collections::HashSet::new();
        let mut path = std::collections::HashSet::new();
        
        for module_id in dependencies.keys() {
            if detect_cycle(*module_id, &dependencies, &mut visited, &mut path)? {
                return Err(format!("モジュールID {}を含む循環依存関係が検出されました", module_id));
            }
        }
        
        Ok(())
    }
    
    /// グラフの循環検出（DFSベース）
    fn detect_cycle(
        current: u32,
        dependencies: &std::collections::HashMap<u32, std::collections::HashSet<u32>>,
        visited: &mut std::collections::HashSet<u32>,
        path: &mut std::collections::HashSet<u32>,
    ) -> Result<bool, String> {
        if path.contains(&current) {
            return Ok(true); // 循環検出
        }
        
        if visited.contains(&current) {
            return Ok(false); // 既に訪問済み、循環なし
        }
        
        visited.insert(current);
        path.insert(current);
        
        if let Some(deps) = dependencies.get(&current) {
            for &next in deps {
                if detect_cycle(next, dependencies, visited, path)? {
                    return Ok(true);
                }
            }
        }
        
        path.remove(&current);
        Ok(false)
    }
    
    /// 未使用の識別子をチェックして警告
    fn check_unused_identifiers(ast: &frontend::ast::Program) {
        // 識別子の使用状況の追跡
        let mut declared = std::collections::HashMap::new();
        let mut used = std::collections::HashSet::new();
        
        // 宣言された識別子の収集
        for decl in &ast.declarations {
            match &decl.kind {
                frontend::ast::DeclarationKind::Variable(var) => {
                    declared.insert(var.name.clone(), (decl.id, "変数".to_string()));
                },
                frontend::ast::DeclarationKind::Constant(constant) => {
                    declared.insert(constant.name.clone(), (decl.id, "定数".to_string()));
                },
                frontend::ast::DeclarationKind::Function(func) => {
                    declared.insert(func.name.clone(), (decl.id, "関数".to_string()));
                },
                _ => {}
            }
        }
        
        // 使用された識別子の収集
        collect_used_identifiers(ast, &mut used);
        
        // 未使用の識別子を警告
        for (name, (id, kind)) in declared {
            if !used.contains(&name) && !name.starts_with('_') {
                println!("警告: 未使用の{}「{}」（ID: {}）", kind, name, id);
            }
        }
    }
    
    /// 使用された識別子を再帰的に収集
    fn collect_used_identifiers(ast: &frontend::ast::Program, used: &mut std::collections::HashSet<String>) {
        // 単純化のため、実際の実装では全ての式と文を走査して使用された識別子を抽出
        // ASTノードの種類に応じた処理が必要
        
        // 例示的な実装
        for stmt in &ast.statements {
            match &stmt.kind {
                frontend::ast::StatementKind::Expression(expr) => {
                    collect_identifiers_from_expr(expr, used);
                },
                // 他の文タイプの処理
                _ => {}
            }
        }
    }
    
    /// 式から識別子を収集
    fn collect_identifiers_from_expr(expr: &frontend::ast::Expression, used: &mut std::collections::HashSet<String>) {
        match &expr.kind {
            frontend::ast::ExpressionKind::Identifier(ident) => {
                used.insert(ident.name.clone());
            },
            frontend::ast::ExpressionKind::Call { callee, arguments } => {
                collect_identifiers_from_expr(callee, used);
                for arg in arguments {
                    collect_identifiers_from_expr(arg, used);
                }
            },
            // 他の式タイプの処理
            _ => {}
        }
    }
}
