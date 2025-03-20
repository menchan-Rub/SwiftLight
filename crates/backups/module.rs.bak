//! モジュールシステム
//! 
//! このモジュールはSwiftLight言語のモジュールシステムの実装を提供します。
//! モジュールはコードの論理的な単位であり、名前空間を提供します。

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use crate::frontend::ast::{Declaration, Identifier};

/// モジュール
/// 
/// SwiftLight言語のモジュールを表現します。モジュールはコードの論理的な単位であり、
/// 名前空間を提供します。
#[derive(Debug)]
pub struct Module {
    /// モジュール名
    pub name: String,
    
    /// モジュールのパス
    pub path: PathBuf,
    
    /// サブモジュール
    pub submodules: HashMap<String, Module>,
    
    /// エクスポートされたシンボル
    pub exports: HashSet<String>,
    
    /// インポートされたモジュール
    pub imports: Vec<ImportedModule>,
    
    /// 宣言
    pub declarations: Vec<Declaration>,
}

/// インポートされたモジュール
#[derive(Debug)]
pub struct ImportedModule {
    /// モジュール名
    pub name: String,
    
    /// エイリアス（別名）
    pub alias: Option<String>,
    
    /// エクスポートするかどうか
    pub is_exported: bool,
}

impl Module {
    /// 新しいモジュールを作成
    pub fn new<P: AsRef<Path>>(name: String, path: P) -> Self {
        Self {
            name,
            path: path.as_ref().to_path_buf(),
            submodules: HashMap::new(),
            exports: HashSet::new(),
            imports: Vec::new(),
            declarations: Vec::new(),
        }
    }
    
    /// サブモジュールを追加
    pub fn add_submodule(&mut self, module: Module) {
        self.submodules.insert(module.name.clone(), module);
    }
    
    /// モジュールをインポート
    pub fn import_module(&mut self, name: String, alias: Option<String>, is_exported: bool) {
        self.imports.push(ImportedModule {
            name,
            alias,
            is_exported,
        });
    }
    
    /// シンボルをエクスポート
    pub fn export_symbol(&mut self, name: String) {
        self.exports.insert(name);
    }
    
    /// 宣言を追加
    pub fn add_declaration(&mut self, declaration: Declaration) {
        self.declarations.push(declaration);
    }
    
    /// シンボルが存在するかどうかを確認
    pub fn has_symbol(&self, name: &str) -> bool {
        self.declarations.iter().any(|decl| match &decl.name {
            Some(ident) => ident.name == name,
            None => false,
        })
    }
} 