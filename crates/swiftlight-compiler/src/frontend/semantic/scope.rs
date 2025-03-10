//! # スコープ管理
//!
//! コードのスコープを管理するモジュールです。
//! 変数や型の可視性とアクセス可能性を制御します。

use std::collections::{HashMap, HashSet};
use crate::frontend::ast::NodeId;
use crate::frontend::error::{Result, CompilerError, SourceLocation};
use super::symbol_table::{SymbolTable, Symbol, SymbolKind, Visibility};

/// スコープの種類
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScopeKind {
    /// グローバルスコープ
    Global,
    /// モジュールスコープ
    Module,
    /// 関数スコープ
    Function,
    /// ブロックスコープ
    Block,
    /// ループスコープ
    Loop,
    /// 構造体スコープ
    Struct,
    /// 列挙型スコープ
    Enum,
    /// トレイトスコープ
    Trait,
    /// インプリメントスコープ
    Impl,
}

impl ScopeKind {
    /// このスコープが変数宣言を許可するかどうか
    pub fn allows_variable_declarations(&self) -> bool {
        match self {
            ScopeKind::Global | ScopeKind::Module | ScopeKind::Function | 
            ScopeKind::Block | ScopeKind::Loop => true,
            _ => false,
        }
    }
    
    /// このスコープが関数宣言を許可するかどうか
    pub fn allows_function_declarations(&self) -> bool {
        match self {
            ScopeKind::Global | ScopeKind::Module | ScopeKind::Struct | 
            ScopeKind::Trait | ScopeKind::Impl => true,
            _ => false,
        }
    }
    
    /// このスコープが型宣言を許可するかどうか
    pub fn allows_type_declarations(&self) -> bool {
        match self {
            ScopeKind::Global | ScopeKind::Module => true,
            _ => false,
        }
    }
}

/// スコープマネージャー
///
/// プログラム内のスコープ階層を管理します。
pub struct ScopeManager {
    /// シンボルテーブル
    pub symbol_table: SymbolTable,
    
    /// 現在のスコープID
    current_scope_id: usize,
    
    /// スコープIDごとのスコープ種類
    scope_kinds: HashMap<usize, ScopeKind>,
    
    /// 現在の関数スコープID（関数内にいるときのみSome）
    current_function_scope: Option<usize>,
    
    /// ノードIDからスコープIDへのマッピング
    node_scopes: HashMap<NodeId, usize>,
}

impl ScopeManager {
    /// 新しいスコープマネージャーを作成
    pub fn new() -> Self {
        let mut symbol_table = SymbolTable::new();
        let global_scope_id = symbol_table.global_scope_id();
        
        let mut scope_kinds = HashMap::new();
        scope_kinds.insert(global_scope_id, ScopeKind::Global);
        
        Self {
            symbol_table,
            current_scope_id: global_scope_id,
            scope_kinds,
            current_function_scope: None,
            node_scopes: HashMap::new(),
        }
    }
    
    /// 現在のスコープIDを取得
    pub fn current_scope_id(&self) -> usize {
        self.current_scope_id
    }
    
    /// 現在のスコープの種類を取得
    pub fn current_scope_kind(&self) -> ScopeKind {
        self.scope_kinds[&self.current_scope_id]
    }
    
    /// ノードに関連するスコープIDを設定
    pub fn set_node_scope(&mut self, node_id: NodeId, scope_id: usize) {
        self.node_scopes.insert(node_id, scope_id);
    }
    
    /// ノードに関連するスコープIDを取得
    pub fn get_node_scope(&self, node_id: NodeId) -> Option<usize> {
        self.node_scopes.get(&node_id).copied()
    }
    
    /// 新しいスコープを作成して入る
    pub fn enter_scope(&mut self, kind: ScopeKind) -> usize {
        let new_scope_id = self.symbol_table.enter_scope();
        self.scope_kinds.insert(new_scope_id, kind);
        self.current_scope_id = new_scope_id;
        
        // 関数スコープならtrackする
        if kind == ScopeKind::Function {
            self.current_function_scope = Some(new_scope_id);
        }
        
        new_scope_id
    }
    
    /// 現在のスコープから抜ける
    pub fn exit_scope(&mut self) -> Result<usize> {
        let current_kind = self.current_scope_kind();
        
        // 関数スコープから抜ける場合、current_function_scopeをクリア
        if current_kind == ScopeKind::Function {
            self.current_function_scope = None;
        }
        
        let parent_id = self.symbol_table.exit_scope();
        self.current_scope_id = parent_id;
        
        Ok(parent_id)
    }
    
    /// 現在のスコープで変数宣言が許可されているかどうかをチェック
    pub fn check_variable_declaration(&self, name: &str, location: Option<SourceLocation>) -> Result<()> {
        let scope_kind = self.current_scope_kind();
        
        if !scope_kind.allows_variable_declarations() {
            return Err(CompilerError::semantic_error(
                format!("変数の宣言はこのスコープでは許可されていません"),
                location,
            ));
        }
        
        Ok(())
    }
    
    /// 現在のスコープで関数宣言が許可されているかどうかをチェック
    pub fn check_function_declaration(&self, name: &str, location: Option<SourceLocation>) -> Result<()> {
        let scope_kind = self.current_scope_kind();
        
        if !scope_kind.allows_function_declarations() {
            return Err(CompilerError::semantic_error(
                format!("関数の宣言はこのスコープでは許可されていません"),
                location,
            ));
        }
        
        Ok(())
    }
    
    /// 現在のスコープで型宣言が許可されているかどうかをチェック
    pub fn check_type_declaration(&self, name: &str, location: Option<SourceLocation>) -> Result<()> {
        let scope_kind = self.current_scope_kind();
        
        if !scope_kind.allows_type_declarations() {
            return Err(CompilerError::semantic_error(
                format!("型の宣言はこのスコープでは許可されていません"),
                location,
            ));
        }
        
        Ok(())
    }
    
    /// 現在の関数スコープIDを取得（関数内にいる場合のみ）
    pub fn current_function_scope(&self) -> Option<usize> {
        self.current_function_scope
    }
    
    /// シンボルテーブルに新しいシンボルを追加
    pub fn add_symbol(&mut self, symbol: Symbol) -> Result<()> {
        self.symbol_table.add_symbol(symbol)
    }
    
    /// 名前からシンボルを検索
    pub fn lookup_symbol(&self, name: &str) -> Option<&Symbol> {
        self.symbol_table.lookup(name)
    }
    
    /// ノードIDからシンボルを検索
    pub fn get_symbol(&self, node_id: NodeId) -> Option<&Symbol> {
        self.symbol_table.get_symbol(node_id)
    }
    
    /// シンボルが現在のスコープからアクセス可能かどうかを確認
    pub fn is_symbol_accessible(&self, symbol: &Symbol) -> bool {
        self.symbol_table.is_accessible(symbol)
    }
}

/// テスト
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_scope_manager_basic() {
        let mut manager = ScopeManager::new();
        
        // グローバルスコープの確認
        assert_eq!(manager.current_scope_kind(), ScopeKind::Global);
        
        // 関数スコープに入る
        let function_scope = manager.enter_scope(ScopeKind::Function);
        assert_eq!(manager.current_scope_kind(), ScopeKind::Function);
        assert_eq!(manager.current_function_scope(), Some(function_scope));
        
        // ブロックスコープに入る
        let block_scope = manager.enter_scope(ScopeKind::Block);
        assert_eq!(manager.current_scope_kind(), ScopeKind::Block);
        assert_eq!(manager.current_function_scope(), Some(function_scope));
        
        // ブロックスコープから抜ける
        manager.exit_scope().unwrap();
        assert_eq!(manager.current_scope_kind(), ScopeKind::Function);
        
        // 関数スコープから抜ける
        manager.exit_scope().unwrap();
        assert_eq!(manager.current_scope_kind(), ScopeKind::Global);
        assert_eq!(manager.current_function_scope(), None);
    }
    
    #[test]
    fn test_scope_manager_declarations() {
        let manager = ScopeManager::new();
        
        // グローバルスコープでの宣言チェック
        assert!(manager.check_variable_declaration("test", None).is_ok());
        assert!(manager.check_function_declaration("test", None).is_ok());
        assert!(manager.check_type_declaration("test", None).is_ok());
        
        // 関数スコープに入る
        let mut manager = ScopeManager::new();
        manager.enter_scope(ScopeKind::Function);
        
        // 関数スコープでの宣言チェック
        assert!(manager.check_variable_declaration("test", None).is_ok());
        assert!(manager.check_function_declaration("test", None).is_err());
        assert!(manager.check_type_declaration("test", None).is_err());
        
        // 構造体スコープに入る
        let mut manager = ScopeManager::new();
        manager.enter_scope(ScopeKind::Struct);
        
        // 構造体スコープでの宣言チェック
        assert!(manager.check_variable_declaration("test", None).is_err());
        assert!(manager.check_function_declaration("test", None).is_ok());
        assert!(manager.check_type_declaration("test", None).is_err());
    }
} 