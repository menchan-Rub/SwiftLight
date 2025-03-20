//! # 意味解析器（Semantic Analyzer）
//!
//! 抽象構文木（AST）の意味解析を行うモジュールです。
//! 名前解決、型チェック、意味検証などの機能を提供します。

use crate::frontend::ast::{Program, NodeId};
use crate::frontend::error::{Result, CompilerError, Diagnostic};
use super::name_resolver::{NameResolver, NameResolutionResult};
use super::symbol_table::SymbolTable;
use super::scope::{ScopeManager, ScopeKind};

/// 意味解析の結果
#[derive(Debug)]
pub struct AnalysisResult {
    /// 解析対象のプログラム
    pub program: Program,
    
    /// 名前解決の結果
    pub name_resolution: NameResolutionResult,
    
    /// エラーのリスト
    pub errors: Vec<CompilerError>,
    
    /// 警告のリスト
    pub warnings: Vec<CompilerError>,
}

impl AnalysisResult {
    /// 新しい解析結果を作成
    pub fn new(program: Program) -> Self {
        Self {
            program,
            name_resolution: NameResolutionResult::new(),
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }
    
    /// エラーを追加
    pub fn add_error(&mut self, error: CompilerError) {
        self.errors.push(error);
    }
    
    /// 警告を追加
    pub fn add_warning(&mut self, warning: CompilerError) {
        self.warnings.push(warning);
    }
    
    /// エラーがあるかどうか
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty() || self.name_resolution.has_errors()
    }
    
    /// 警告があるかどうか
    pub fn has_warnings(&self) -> bool {
        !self.warnings.is_empty() || self.name_resolution.has_warnings()
    }
    
    /// 解析が成功したかどうか
    pub fn is_success(&self) -> bool {
        !self.has_errors()
    }
    
    /// 解析結果をResult型に変換
    pub fn into_result(self) -> Result<Program> {
        if self.has_errors() {
            // エラーがある場合は最初のエラーを返す
            let mut all_errors = Vec::new();
            all_errors.extend(self.errors);
            all_errors.extend(self.name_resolution.errors);
            
            if let Some(error) = all_errors.into_iter().next() {
                Err(error)
            } else {
                // エラーがあるはずだがない場合（ありえない）
                Err(CompilerError::internal_error(
                    "意味解析に失敗しましたが、エラー情報がありません".to_string(),
                    None,
                ))
            }
        } else {
            // 成功の場合はプログラムを返す
            Ok(self.program)
        }
    }
}

/// 意味解析器
///
/// 抽象構文木（AST）の意味解析を行います。
/// 名前解決、型チェック、その他の意味検証を実行します。
pub struct SemanticAnalyzer {
    /// 名前解決器
    name_resolver: NameResolver,
}

impl SemanticAnalyzer {
    /// 新しい意味解析器を作成
    pub fn new() -> Self {
        Self {
            name_resolver: NameResolver::new(),
        }
    }
    
    /// デフォルト値で初期化
    pub fn default() -> Self {
        Self::new()
    }
    
    /// プログラムを解析
    pub fn analyze(&mut self, program: Program) -> AnalysisResult {
        let mut result = AnalysisResult::new(program.clone());
        
        // 名前解決を実行
        match self.name_resolver.resolve_program(&program) {
            Ok(name_resolution) => {
                // 名前解決結果を保存
                result.name_resolution = name_resolution;
                
                // 名前解決のエラーと警告を転送
                result.errors.extend(result.name_resolution.errors.clone());
                result.warnings.extend(result.name_resolution.warnings.clone());
                
                // 型チェックは名前解決が成功した場合のみ実行
                if !result.has_errors() {
                    // 型チェックを実行
                    let type_checker = super::type_checker::TypeChecker::new(
                        result.name_resolution.clone(),
                        self.name_resolver.get_symbol_table().clone()
                    );
                    
                    match type_checker.check_program(&program) {
                        Ok(type_check_result) => {
                            // 型チェック結果のエラーと警告を転送
                            result.errors.extend(type_check_result.errors);
                            result.warnings.extend(type_check_result.warnings);
                            
                            // 追加の型検証が必要な場合はここに実装
                            
                            // 診断情報の表示（デバッグ用）
                            #[cfg(debug_assertions)]
                            println!("{}", type_check_result.get_diagnostics_summary());
                        },
                        Err(error) => {
                            // 型チェックでエラーが発生した場合
                            result.add_error(error);
                        }
                    }
                }
            },
            Err(error) => {
                // 名前解決でエラーが発生した場合
                result.add_error(error);
            }
        }
        
        result
    }
    
    /// 名前解決結果からシンボルの参照先を取得
    pub fn get_symbol_reference(&self, result: &AnalysisResult, node_id: NodeId) -> Option<NodeId> {
        result.name_resolution.resolved_nodes.get(&node_id).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend::ast::{
        self, Declaration, DeclarationKind, Expression, ExpressionKind,
        Statement, StatementKind, Identifier, VariableDeclaration,
    };
    
    // テスト用のヘルパー関数：シンプルなプログラムを作成
    fn create_test_program() -> Program {
        // 変数宣言 let x = 5
        let var_decl = Declaration {
            id: 1,
            kind: DeclarationKind::Variable(VariableDeclaration {
                name: Identifier {
                    id: 2,
                    name: "x".to_string(),
                },
                is_mutable: false,
                type_annotation: None,
                initializer: Some(Expression {
                    id: 3,
                    kind: ExpressionKind::Literal(ast::Literal::Integer(5)),
                    location: None,
                }),
            }),
            location: None,
        };
        
        // 変数参照 x (式)
        let var_ref = Expression {
            id: 4,
            kind: ExpressionKind::Identifier(Identifier {
                id: 5,
                name: "x".to_string(),
            }),
            location: None,
        };
        
        // 式文 (x)
        let expr_stmt = Statement {
            id: 6,
            kind: StatementKind::Expression(var_ref),
            location: None,
        };
        
        // プログラム全体
        Program {
            id: 0,
            declarations: vec![var_decl],
            statements: vec![expr_stmt],
            source_file: "test.sl".to_string(),
        }
    }
    
    #[test]
    fn test_analyze_simple_program() {
        let program = create_test_program();
        let mut analyzer = SemanticAnalyzer::new();
        
        let result = analyzer.analyze(program);
        
        // 解析が成功すること
        assert!(result.is_success());
        
        // 名前解決が行われていること（変数xの参照がxの宣言を指していること）
        assert!(result.name_resolution.resolved_nodes.contains_key(&5));
        assert_eq!(result.name_resolution.resolved_nodes[&5], 1);
    }
    
    #[test]
    fn test_analyze_with_undefined_variable() {
        // 未定義変数への参照を含むプログラム
        let mut program = create_test_program();
        
        // 未定義変数 y への参照を追加
        let undefined_var_ref = Expression {
            id: 7,
            kind: ExpressionKind::Identifier(Identifier {
                id: 8,
                name: "y".to_string(),
            }),
            location: None,
        };
        
        let undefined_var_stmt = Statement {
            id: 9,
            kind: StatementKind::Expression(undefined_var_ref),
            location: None,
        };
        
        program.statements.push(undefined_var_stmt);
        
        let mut analyzer = SemanticAnalyzer::new();
        let result = analyzer.analyze(program);
        
        // 解析でエラーが検出されること
        assert!(result.has_errors());
        
        // エラーメッセージが「シンボル 'y' が見つかりません」を含むこと
        let contains_error = result.name_resolution.errors.iter().any(|e| {
            e.message.contains("'y'") && e.message.contains("見つかりません")
        });
        
        assert!(contains_error);
    }
} 