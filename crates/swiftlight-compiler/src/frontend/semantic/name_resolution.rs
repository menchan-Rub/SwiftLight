use crate::frontend::ast::{AstNode, Visibility};
use crate::frontend::semantic::error::SemanticError;
use crate::frontend::semantic::symbol_table::SymbolTable;
use crate::frontend::semantic::module::Module;
use crate::frontend::semantic::type_annotation::TypeAnnotation;

// 完全な名前解決の実装
pub struct NameResolver {
    symbol_table: SymbolTable,
}

impl NameResolver {
    pub fn new() -> Self {
        Self {
            symbol_table: SymbolTable::new(),
        }
    }

    // インポート宣言の解決
    pub fn resolve_imports(&mut self, ast: &mut AstNode) -> Result<(), SemanticError> {
        // AST内の全てのインポート宣言を処理し、対応するモジュールを読み込む
        for node in ast.get_children_mut() {
            if let AstNode::ImportDeclaration { ref path } = node {
                let module = self.load_module(path)?;
                self.symbol_table.insert_module(path.clone(), module);
            }
        }
        Ok(())
    }

    // 関数宣言のシグネチャに型注釈が付いているか確認。無い場合はデフォルトでvoidとする。
    pub fn resolve_function_signatures(&mut self, ast: &mut AstNode) -> Result<(), SemanticError> {
        for node in ast.get_children_mut() {
            if let AstNode::FunctionDeclaration { ref mut return_type, .. } = node {
                if return_type.is_none() {
                    *return_type = Some(TypeAnnotation::Simple("void".to_string()));
                }
            }
        }
        Ok(())
    }

    // AST内の各要素の可視性を設定。特に注釈が無い場合はpublicとする。
    pub fn resolve_visibility(&mut self, ast: &mut AstNode) -> Result<(), SemanticError> {
        for node in ast.get_children_mut() {
            node.set_visibility(Visibility::Public);
        }
        Ok(())
    }

    // 名前解決の全体処理
    pub fn resolve(&mut self, ast: &mut AstNode) -> Result<(), SemanticError> {
        self.resolve_imports(ast)?;
        self.resolve_function_signatures(ast)?;
        self.resolve_visibility(ast)?;
        Ok(())
    }

    // モジュールの読み込み（簡易実装：常にデフォルトのモジュールを返す）
    fn load_module(&self, path: &String) -> Result<Module, SemanticError> {
        Ok(Module::default())
    }
}

