//! # シンボルテーブル
//!
//! 名前解決のためのシンボルテーブルを提供します。
//! 変数、関数、型などの名前とその定義情報を管理します。

use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

use crate::frontend::ast::{
    Expression, ExpressionKind, Identifier, NodeId,
    Parameter, Statement, StatementKind, TypeAnnotation,
};
use crate::frontend::error::{SourceLocation, CompilerError, ErrorKind, Result, Diagnostic};

/// シンボルの種類
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SymbolKind {
    /// 変数
    Variable,
    /// 定数
    Constant,
    /// 関数
    Function,
    /// パラメータ
    Parameter,
    /// 構造体
    Struct,
    /// 列挙型
    Enum,
    /// 列挙型のバリアント
    EnumVariant,
    /// トレイト
    Trait,
    /// 型エイリアス
    TypeAlias,
    /// モジュール
    Module,
    /// インポート
    Import,
}

impl fmt::Display for SymbolKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SymbolKind::Variable => write!(f, "変数"),
            SymbolKind::Constant => write!(f, "定数"),
            SymbolKind::Function => write!(f, "関数"),
            SymbolKind::Parameter => write!(f, "パラメータ"),
            SymbolKind::Struct => write!(f, "構造体"),
            SymbolKind::Enum => write!(f, "列挙型"),
            SymbolKind::EnumVariant => write!(f, "列挙型バリアント"),
            SymbolKind::Trait => write!(f, "トレイト"),
            SymbolKind::TypeAlias => write!(f, "型エイリアス"),
            SymbolKind::Module => write!(f, "モジュール"),
            SymbolKind::Import => write!(f, "インポート"),
        }
    }
}

/// シンボル定義
#[derive(Debug, Clone)]
pub struct Symbol {
    /// シンボル名
    pub name: String,
    /// シンボルの種類
    pub kind: SymbolKind,
    /// 対応するAST要素のノードID
    pub node_id: NodeId,
    /// シンボルの型情報（オプション）
    pub type_info: Option<TypeAnnotation>,
    /// ソースコード内の位置情報
    pub location: Option<SourceLocation>,
    /// 可変かどうか（変数の場合）
    pub is_mutable: bool,
    /// スコープID
    pub scope_id: usize,
    /// 可視性（公開レベル）
    pub visibility: Visibility,
}

impl Symbol {
    /// 新しいシンボルを作成
    pub fn new(
        name: impl Into<String>,
        kind: SymbolKind,
        node_id: NodeId,
        type_info: Option<TypeAnnotation>,
        location: Option<SourceLocation>,
        is_mutable: bool,
        scope_id: usize,
        visibility: Visibility,
    ) -> Self {
        Self {
            name: name.into(),
            kind,
            node_id,
            type_info,
            location,
            is_mutable,
            scope_id,
            visibility,
        }
    }
    
    /// 変数シンボルを作成
    pub fn variable(
        name: impl Into<String>,
        node_id: NodeId,
        type_info: Option<TypeAnnotation>,
        location: Option<SourceLocation>,
        is_mutable: bool,
        scope_id: usize,
        visibility: Visibility,
    ) -> Self {
        Self::new(
            name,
            SymbolKind::Variable,
            node_id,
            type_info,
            location,
            is_mutable,
            scope_id,
            visibility,
        )
    }
    
    /// 定数シンボルを作成
    pub fn constant(
        name: impl Into<String>,
        node_id: NodeId,
        type_info: Option<TypeAnnotation>,
        location: Option<SourceLocation>,
        scope_id: usize,
        visibility: Visibility,
    ) -> Self {
        Self::new(
            name,
            SymbolKind::Constant,
            node_id,
            type_info,
            location,
            false, // 定数は不変
            scope_id,
            visibility,
        )
    }
    
    /// 関数シンボルを作成
    pub fn function(
        name: impl Into<String>,
        node_id: NodeId,
        type_info: Option<TypeAnnotation>,
        location: Option<SourceLocation>,
        scope_id: usize,
        visibility: Visibility,
    ) -> Self {
        Self::new(
            name,
            SymbolKind::Function,
            node_id,
            type_info,
            location,
            false,
            scope_id,
            visibility,
        )
    }
    
    /// パラメータシンボルを作成
    pub fn parameter(
        name: impl Into<String>,
        node_id: NodeId,
        type_info: Option<TypeAnnotation>,
        location: Option<SourceLocation>,
        is_mutable: bool,
        scope_id: usize,
    ) -> Self {
        Self::new(
            name,
            SymbolKind::Parameter,
            node_id,
            type_info,
            location,
            is_mutable,
            scope_id,
            Visibility::Private, // パラメータは常にプライベート
        )
    }
    
    /// 型シンボル（構造体、列挙型、トレイトなど）を作成
    pub fn type_symbol(
        name: impl Into<String>,
        kind: SymbolKind,
        node_id: NodeId,
        location: Option<SourceLocation>,
        scope_id: usize,
        visibility: Visibility,
    ) -> Self {
        Self::new(
            name,
            kind,
            node_id,
            None,
            location,
            false,
            scope_id,
            visibility,
        )
    }
}

/// シンボルの可視性レベル
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Visibility {
    /// プライベート（同一モジュール内からのみアクセス可能）
    Private,
    /// パブリック（どこからでもアクセス可能）
    Public,
    /// 派生したトレイトのみアクセス可能
    Trait,
    /// 同一クレート内からのみアクセス可能
    Crate,
}

impl Default for Visibility {
    fn default() -> Self {
        Visibility::Private
    }
}

/// シンボルテーブル
///
/// プログラム内のすべてのシンボルを管理します。
/// スコープの階層構造も管理します。
#[derive(Debug, Default)]
pub struct SymbolTable {
    /// シンボル情報（ノードIDをキーにしたマップ）
    symbols: HashMap<NodeId, Symbol>,
    
    /// 名前からシンボルへのマップ（スコープIDごと）
    /// (scope_id, name) -> node_id
    scoped_symbols: HashMap<(usize, String), NodeId>,
    
    /// スコープの階層構造（親スコープへのマップ）
    /// key: スコープID, value: 親スコープID
    scope_parents: HashMap<usize, usize>,
    
    /// 次に割り当てるスコープID
    next_scope_id: usize,
    
    /// グローバルスコープID
    global_scope_id: usize,
    
    /// 現在のスコープID
    current_scope_id: usize,
}

impl SymbolTable {
    /// 新しいシンボルテーブルを作成
    pub fn new() -> Self {
        let mut table = Self {
            symbols: HashMap::new(),
            scoped_symbols: HashMap::new(),
            scope_parents: HashMap::new(),
            next_scope_id: 1, // 0はグローバルスコープ用に予約
            global_scope_id: 0,
            current_scope_id: 0,
        };
        
        // グローバルスコープを初期化
        table.scope_parents.insert(table.global_scope_id, table.global_scope_id);
        table.current_scope_id = table.global_scope_id;
        
        table
    }
    
    /// 新しいスコープを作成
    ///
    /// 親スコープは現在のスコープになります。
    pub fn enter_scope(&mut self) -> usize {
        let new_scope_id = self.next_scope_id;
        self.next_scope_id += 1;
        
        // 親スコープを現在のスコープに設定
        self.scope_parents.insert(new_scope_id, self.current_scope_id);
        
        // 現在のスコープを更新
        self.current_scope_id = new_scope_id;
        
        new_scope_id
    }
    
    /// 現在のスコープを抜ける
    ///
    /// 親スコープに戻ります。
    pub fn exit_scope(&mut self) -> usize {
        // 親スコープに戻る
        let parent_id = self.scope_parents[&self.current_scope_id];
        self.current_scope_id = parent_id;
        
        parent_id
    }
    
    /// 現在のスコープIDを取得
    pub fn current_scope_id(&self) -> usize {
        self.current_scope_id
    }
    
    /// グローバルスコープIDを取得
    pub fn global_scope_id(&self) -> usize {
        self.global_scope_id
    }
    
    /// シンボルを追加
    pub fn add_symbol(&mut self, symbol: Symbol) -> Result<()> {
        let name = symbol.name.clone();
        let node_id = symbol.node_id;
        let scope_id = symbol.scope_id;
        
        // 現在のスコープに同名のシンボルがないかチェック
        if let Some(existing_id) = self.scoped_symbols.get(&(scope_id, name.clone())) {
            let existing = &self.symbols[existing_id];
            
            // エラーを作成
            let error = CompilerError::semantic_error(
                format!(
                    "シンボル '{}' は既に{}として定義されています",
                    name,
                    existing.kind
                ),
                symbol.location.clone(),
            );
            
            // 既存の定義を指摘する診断情報を追加
            if let Some(loc) = &existing.location {
                let diagnostic = Diagnostic::note(
                    format!("以前の定義はここにあります"),
                    Some(loc.clone()),
                );
                return Err(error.with_diagnostic(diagnostic));
            }
            
            return Err(error);
        }
        
        // シンボルを追加
        self.scoped_symbols.insert((scope_id, name), node_id);
        self.symbols.insert(node_id, symbol);
        
        Ok(())
    }
    
    /// 指定されたスコープで名前からシンボルを検索
    fn lookup_in_scope(&self, name: &str, scope_id: usize) -> Option<&Symbol> {
        if let Some(node_id) = self.scoped_symbols.get(&(scope_id, name.to_string())) {
            self.symbols.get(node_id)
        } else {
            None
        }
    }
    
    /// 名前からシンボルを検索（現在のスコープとその親スコープを検索）
    pub fn lookup(&self, name: &str) -> Option<&Symbol> {
        let mut current_scope = self.current_scope_id;
        
        // 現在のスコープから親スコープへと順に検索
        loop {
            if let Some(symbol) = self.lookup_in_scope(name, current_scope) {
                return Some(symbol);
            }
            
            // 親スコープに移動
            let parent_scope = self.scope_parents[&current_scope];
            
            // グローバルスコープに達したら終了
            if parent_scope == current_scope {
                break;
            }
            
            current_scope = parent_scope;
        }
        
        None
    }
    
    /// ノードIDからシンボルを検索
    pub fn get_symbol(&self, node_id: NodeId) -> Option<&Symbol> {
        self.symbols.get(&node_id)
    }
    
    /// シンボルが定義されているスコープがアクセス可能かチェック
    pub fn is_accessible(&self, symbol: &Symbol) -> bool {
        match symbol.visibility {
            Visibility::Public => true,
            Visibility::Private => {
                // 同一スコープまたはその子スコープからのみアクセス可能
                let mut current_scope = self.current_scope_id;
                
                loop {
                    if current_scope == symbol.scope_id {
                        return true;
                    }
                    
                    // 親スコープに移動
                    let parent_scope = self.scope_parents[&current_scope];
                    
                    // グローバルスコープに達したら終了
                    if parent_scope == current_scope {
                        break;
                    }
                    
                    current_scope = parent_scope;
                }
                
                false
            },
            Visibility::Crate => true, // 同一クレート内はすべてアクセス可能（現状では簡略化）
            Visibility::Trait => false, // トレイト可視性は現時点ではサポート外
        }
    }
}

/// テスト
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_symbol_table_basic() {
        let mut table = SymbolTable::new();
        
        // グローバルスコープに変数を追加
        let symbol = Symbol::variable(
            "x",
            1,
            None,
            None,
            false,
            table.current_scope_id(),
            Visibility::Private,
        );
        
        table.add_symbol(symbol).unwrap();
        
        // 変数を検索
        let found = table.lookup("x").unwrap();
        assert_eq!(found.name, "x");
        assert_eq!(found.kind, SymbolKind::Variable);
    }
    
    #[test]
    fn test_symbol_table_scopes() {
        let mut table = SymbolTable::new();
        
        // グローバルスコープに変数を追加
        let global_symbol = Symbol::variable(
            "global",
            1,
            None,
            None,
            false,
            table.current_scope_id(),
            Visibility::Private,
        );
        
        table.add_symbol(global_symbol).unwrap();
        
        // 新しいスコープに入る
        let inner_scope = table.enter_scope();
        
        // 内側のスコープに変数を追加
        let inner_symbol = Symbol::variable(
            "inner",
            2,
            None,
            None,
            false,
            table.current_scope_id(),
            Visibility::Private,
        );
        
        table.add_symbol(inner_symbol).unwrap();
        
        // 内側のスコープでは両方の変数が見える
        assert!(table.lookup("global").is_some());
        assert!(table.lookup("inner").is_some());
        
        // スコープを抜ける
        table.exit_scope();
        
        // 外側のスコープでは内側の変数は見えない
        assert!(table.lookup("global").is_some());
        assert!(table.lookup("inner").is_none());
    }
    
    #[test]
    fn test_symbol_table_shadowing() {
        let mut table = SymbolTable::new();
        
        // グローバルスコープに変数を追加
        let global_x = Symbol::variable(
            "x",
            1,
            None,
            None,
            false,
            table.current_scope_id(),
            Visibility::Private,
        );
        
        table.add_symbol(global_x).unwrap();
        
        // 新しいスコープに入る
        let inner_scope = table.enter_scope();
        
        // 内側のスコープに同名の変数を追加（シャドーイング）
        let inner_x = Symbol::variable(
            "x",
            2,
            None,
            None,
            true,
            table.current_scope_id(),
            Visibility::Private,
        );
        
        table.add_symbol(inner_x).unwrap();
        
        // 内側のスコープでは内側の変数が見える
        let found = table.lookup("x").unwrap();
        assert_eq!(found.node_id, 2);
        assert!(found.is_mutable);
        
        // スコープを抜ける
        table.exit_scope();
        
        // 外側のスコープでは外側の変数が見える
        let found = table.lookup("x").unwrap();
        assert_eq!(found.node_id, 1);
        assert!(!found.is_mutable);
    }
} 