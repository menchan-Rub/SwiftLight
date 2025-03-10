/*
 * ownership_checker.rs - SwiftLight 所有権/借用チェック実装
 *
 * このモジュールでは、SwiftLightのASTに対して所有権と借用のルールを検証します。
 * 変数宣言、代入、関数宣言、ブロックなどのノードを走査し、
 * 変数が適切に初期化され、所有権が維持されているかを確認します。
 */

use crate::frontend::ast::AstNode;
use crate::frontend::semantic::error::SemanticError;
use crate::frontend::semantic::symbol_table::SymbolTable;

/// OwnershipChecker は、所有権/借用ルールの検証を行うための構造体です。
#[derive(Clone)]
pub struct OwnershipChecker {
    /// 変数名とその所有権状態 (true = 有効, false = 移動済みなど) を管理するシンボルテーブル
    symbol_table: SymbolTable,
}

impl OwnershipChecker {
    /// 新しい OwnershipChecker を生成します。
    pub fn new() -> Self {
        Self {
            symbol_table: SymbolTable::new(),
        }
    }

    /// 与えられた AST 全体に対して所有権チェックを実行します。
    pub fn check(&mut self, ast: &AstNode) -> Result<(), SemanticError> {
        self.visit_node(ast)
    }

    /// AST ノードを再帰的に走査して所有権チェックを行います。
    fn visit_node(&mut self, node: &AstNode) -> Result<(), SemanticError> {
        match node {
            // 変数宣言: 初期化されていない場合はエラー、初期化されていれば所有権を登録
            AstNode::VariableDeclaration { name, initializer } => {
                if let Some(expr) = initializer {
                    self.visit_node(expr)?;
                    // 変数は初期化時に所有権が確立する
                    self.symbol_table.insert(name.clone(), true);
                    Ok(())
                } else {
                    Err(SemanticError::new(format!("変数 '{}' は初期化されていません。", name)))
                }
            },
            // 代入: 代入先の変数が既に宣言されていて、所有権が有効かチェック
            AstNode::Assignment { name, expr } => {
                self.visit_node(expr)?;
                if let Some(owned) = self.symbol_table.get(name) {
                    if !*owned {
                        Err(SemanticError::new(format!("変数 '{}' の所有権が無効です。", name)))
                    } else {
                        Ok(())
                    }
                } else {
                    Err(SemanticError::new(format!("未宣言の変数 '{}' への代入です。", name)))
                }
            },
            // 関数宣言: パラメータをローカルシンボルテーブルに登録して、関数本体をチェック
            AstNode::FunctionDeclaration { parameters, body, .. } => {
                let old_table = self.symbol_table.clone();
                for param in parameters {
                    self.symbol_table.insert(param.clone(), true);
                }
                self.visit_node(body)?;
                self.symbol_table = old_table; // スコープ終了時に復元
                Ok(())
            },
            // ブロック: 新たなスコープとしてローカルにチェック
            AstNode::Block(statements) => {
                let old_table = self.symbol_table.clone();
                for stmt in statements {
                    self.visit_node(stmt)?;
                }
                self.symbol_table = old_table;
                Ok(())
            },
            // 変数参照: 既に登録された変数の所有権状態をチェック
            AstNode::Identifier(name) => {
                if let Some(owned) = self.symbol_table.get(name) {
                    if !*owned {
                        Err(SemanticError::new(format!("変数 '{}' は既に所有権が移動しているか、無効です。", name)))
                    } else {
                        Ok(())
                    }
                } else {
                    Err(SemanticError::new(format!("未宣言の変数 '{}' です。", name)))
                }
            },
            // 二項演算などの式の場合、左右の式をチェック
            AstNode::BinaryExpression { left, right, .. } => {
                self.visit_node(left)?;
                self.visit_node(right)?;
                Ok(())
            },
            // 関数呼び出しの場合、呼び出し側と各引数をチェック
            AstNode::CallExpression { callee, arguments } => {
                self.visit_node(callee)?;
                for arg in arguments {
                    self.visit_node(arg)?;
                }
                Ok(())
            },
            // 他のノードは、必要に応じて再帰的にチェック
            _ => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend::ast::AstNode;

    // 補助関数: 変数宣言の AST を生成
    fn var_decl(name: &str, initializer: AstNode) -> AstNode {
        AstNode::VariableDeclaration {
            name: name.to_string(),
            initializer: Some(Box::new(initializer)),
        }
    }

    // 補助関数: 代入の AST を生成
    fn assignment(name: &str, expr: AstNode) -> AstNode {
        AstNode::Assignment {
            name: name.to_string(),
            expr: Box::new(expr),
        }
    }

    // 補助関数: 識別子の AST を生成
    fn identifier(name: &str) -> AstNode {
        AstNode::Identifier(name.to_string())
    }

    // 補助関数: ブロックの AST を生成
    fn block(statements: Vec<AstNode>) -> AstNode {
        AstNode::Block(statements)
    }

    #[test]
    fn test_variable_declaration_success() {
        // 変数 'x' の宣言、初期化に成功すれば Ok
        let ast = var_decl("x", identifier("y"));
        let mut checker = OwnershipChecker::new();
        let result = checker.check(&ast);
        assert!(result.is_ok());
    }

    #[test]
    fn test_assignment_to_undeclared_variable() {
        // 未宣言の変数 'x' に代入するとエラーとなる
        let ast = assignment("x", identifier("y"));
        let mut checker = OwnershipChecker::new();
        let result = checker.check(&ast);
        assert!(result.is_err());
    }
}
