//! # SwiftLightモジュールシステム
//!
//! モジュール管理、インポート解決、依存グラフ構築、および関連する機能を提供します。
//! このモジュールにより、コードの整理と再利用性が促進されます。

use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use crate::frontend::error::{CompilerError, ErrorKind, Result, SourceLocation};
use crate::frontend::ast::*;
use crate::typesystem::{TypeRegistry, Symbol};

mod resolver;
mod dependency;
mod interface;
mod export;
mod visibility;

pub use resolver::*;
pub use dependency::*;
pub use interface::*;
pub use export::*;
pub use visibility::*;

/// モジュール識別子
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ModuleId {
    /// 絶対パス
    Absolute(Vec<Symbol>),
    
    /// 相対パス
    Relative(Vec<Symbol>),
    
    /// ファイルパス
    FilePath(PathBuf),
}

impl fmt::Display for ModuleId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ModuleId::Absolute(path) => {
                write!(f, "::")?;
                for (i, segment) in path.iter().enumerate() {
                    if i > 0 {
                        write!(f, "::")?;
                    }
                    write!(f, "{}", segment.as_str())?;
                }
                Ok(())
            },
            ModuleId::Relative(path) => {
                for (i, segment) in path.iter().enumerate() {
                    if i > 0 {
                        write!(f, "::")?;
                    }
                    write!(f, "{}", segment.as_str())?;
                }
                Ok(())
            },
            ModuleId::FilePath(path) => {
                write!(f, "{}", path.display())
            },
        }
    }
}

impl ModuleId {
    /// 絶対パスからモジュールIDを作成
    pub fn from_absolute_path(path: Vec<Symbol>) -> Self {
        ModuleId::Absolute(path)
    }
    
    /// 相対パスからモジュールIDを作成
    pub fn from_relative_path(path: Vec<Symbol>) -> Self {
        ModuleId::Relative(path)
    }
    
    /// ファイルパスからモジュールIDを作成
    pub fn from_file_path<P: AsRef<Path>>(path: P) -> Self {
        ModuleId::FilePath(path.as_ref().to_path_buf())
    }
    
    /// 親モジュールのIDを取得
    pub fn parent(&self) -> Option<Self> {
        match self {
            ModuleId::Absolute(path) => {
                if path.is_empty() {
                    None
                } else {
                    let mut parent_path = path.clone();
                    parent_path.pop();
                    Some(ModuleId::Absolute(parent_path))
                }
            },
            ModuleId::Relative(path) => {
                if path.is_empty() {
                    None
                } else {
                    let mut parent_path = path.clone();
                    parent_path.pop();
                    Some(ModuleId::Relative(parent_path))
                }
            },
            ModuleId::FilePath(path) => {
                path.parent().map(|p| ModuleId::FilePath(p.to_path_buf()))
            },
        }
    }
    
    /// 子モジュールのIDを取得
    pub fn child(&self, name: Symbol) -> Self {
        match self {
            ModuleId::Absolute(path) => {
                let mut child_path = path.clone();
                child_path.push(name);
                ModuleId::Absolute(child_path)
            },
            ModuleId::Relative(path) => {
                let mut child_path = path.clone();
                child_path.push(name);
                ModuleId::Relative(child_path)
            },
            ModuleId::FilePath(path) => {
                let mut child_path = path.clone();
                child_path.push(name.as_str());
                ModuleId::FilePath(child_path)
            },
        }
    }
    
    /// 最後のセグメントを取得
    pub fn last_segment(&self) -> Option<Symbol> {
        match self {
            ModuleId::Absolute(path) => path.last().copied(),
            ModuleId::Relative(path) => path.last().copied(),
            ModuleId::FilePath(path) => {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .map(Symbol::intern)
            },
        }
    }
    
    /// 相対パスを絶対パスに解決
    pub fn resolve_relative(&self, base: &ModuleId) -> Result<Self> {
        match (self, base) {
            (ModuleId::Relative(path), ModuleId::Absolute(base_path)) => {
                let mut resolved_path = base_path.clone();
                resolved_path.extend_from_slice(path);
                Ok(ModuleId::Absolute(resolved_path))
            },
            (ModuleId::Relative(path), ModuleId::Relative(_)) => {
                return Err(CompilerError::new(
                    ErrorKind::ModuleSystem,
                    "相対パスは絶対パスを基準にしてのみ解決できます".to_string(),
                    SourceLocation::default(),
                ));
            },
            (ModuleId::Relative(path), ModuleId::FilePath(base_path)) => {
                let mut resolved_path = base_path.clone();
                
                // ディレクトリに移動
                if resolved_path.is_file() {
                    if let Some(parent) = resolved_path.parent() {
                        resolved_path = parent.to_path_buf();
                    }
                }
                
                // 相対パスの各セグメントを適用
                for segment in path {
                    resolved_path.push(segment.as_str());
                }
                
                Ok(ModuleId::FilePath(resolved_path))
            },
            (_, _) => Ok(self.clone()),
        }
    }
}

/// モジュール参照
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModuleRef {
    /// 直接参照（モジュールID）
    Direct(ModuleId),
    
    /// インデックス参照（モジュールマネージャ内の位置）
    Index(usize),
}

/// モジュール定義
#[derive(Debug, Clone)]
pub struct Module {
    /// モジュールID
    pub id: ModuleId,
    
    /// モジュール名
    pub name: Symbol,
    
    /// ソースファイルパス（オプション）
    pub source_path: Option<PathBuf>,
    
    /// AST（構文木）
    pub ast: Option<Arc<ModuleNode>>,
    
    /// インターフェース
    pub interface: ModuleInterface,
    
    /// 親モジュール
    pub parent: Option<ModuleRef>,
    
    /// 子モジュール
    pub children: HashMap<Symbol, ModuleRef>,
    
    /// インポート
    pub imports: Vec<Import>,
    
    /// エクスポート
    pub exports: ExportSet,
    
    /// 依存関係
    pub dependencies: Vec<ModuleRef>,
    
    /// 循環依存フラグ
    pub has_circular_dependency: bool,
    
    /// ロード済みフラグ
    pub is_loaded: bool,
    
    /// 型チェック済みフラグ
    pub is_type_checked: bool,
}

/// インポート定義
#[derive(Debug, Clone)]
pub struct Import {
    /// インポート元モジュール
    pub source: ModuleRef,
    
    /// インポート項目
    pub items: ImportItems,
    
    /// 可視性
    pub visibility: Visibility,
    
    /// エイリアス（リネーム）
    pub alias: Option<Symbol>,
    
    /// 位置情報
    pub location: SourceLocation,
}

/// インポート項目
#[derive(Debug, Clone)]
pub enum ImportItems {
    /// モジュール全体
    All,
    
    /// 特定の項目
    Specific(Vec<ImportItem>),
}

/// インポート項目
#[derive(Debug, Clone)]
pub struct ImportItem {
    /// 項目名
    pub name: Symbol,
    
    /// エイリアス（リネーム）
    pub alias: Option<Symbol>,
    
    /// 可視性
    pub visibility: Visibility,
    
    /// 位置情報
    pub location: SourceLocation,
}

/// モジュールマネージャ
pub struct ModuleManager {
    /// モジュール一覧
    modules: Vec<Module>,
    
    /// モジュールIDとインデックスのマッピング
    id_map: HashMap<ModuleId, usize>,
    
    /// ルートモジュール
    root: Option<ModuleRef>,
    
    /// ファイルローダー
    file_loader: Box<dyn ModuleFileLoader + Send>,
    
    /// 型レジストリ
    type_registry: Arc<TypeRegistry>,
}

/// モジュールファイルローダートレイト
pub trait ModuleFileLoader: Send {
    /// ファイルパスからモジュールを読み込む
    fn load_file(&self, path: &Path) -> Result<String>;
    
    /// ディレクトリからモジュールを検索
    fn find_modules(&self, dir_path: &Path) -> Result<Vec<PathBuf>>;
    
    /// モジュール名からファイルパスを解決
    fn resolve_module_path(&self, module_id: &ModuleId, base_path: Option<&Path>) -> Result<PathBuf>;
}

impl Module {
    /// 新しい空のモジュールを作成
    pub fn new(id: ModuleId, name: Symbol) -> Self {
        Self {
            id,
            name,
            source_path: None,
            ast: None,
            interface: ModuleInterface::new(),
            parent: None,
            children: HashMap::new(),
            imports: Vec::new(),
            exports: ExportSet::new(),
            dependencies: Vec::new(),
            has_circular_dependency: false,
            is_loaded: false,
            is_type_checked: false,
        }
    }
    
    /// モジュールがロード済みかどうかをチェック
    pub fn is_loaded(&self) -> bool {
        self.is_loaded
    }
    
    /// モジュールが型チェック済みかどうかをチェック
    pub fn is_type_checked(&self) -> bool {
        self.is_type_checked
    }
    
    /// シンボルが可視かどうかをチェック
    pub fn is_symbol_visible(&self, symbol: Symbol, in_module: &Module) -> bool {
        if let Some(item) = self.interface.get_item(symbol) {
            match item.visibility {
                Visibility::Public => true,
                Visibility::Private => self.id == in_module.id,
                Visibility::Protected => {
                    // 同じモジュールまたは子モジュールなら可視
                    self.id == in_module.id || self.is_ancestor_of(in_module)
                },
                Visibility::Internal => {
                    // 同じプログラム内なら可視
                    true
                },
            }
        } else {
            false
        }
    }
    
    /// このモジュールが指定されたモジュールの祖先かどうかをチェック
    pub fn is_ancestor_of(&self, other: &Module) -> bool {
        let mut current = other.parent.as_ref();
        
        while let Some(parent_ref) = current {
            if let ModuleRef::Direct(ref parent_id) = parent_ref {
                if parent_id == &self.id {
                    return true;
                }
            }
            
            // 次の親をチェック
            match parent_ref {
                ModuleRef::Direct(_) => {
                    // これ以上チェックできない
                    break;
                },
                ModuleRef::Index(_) => {
                    // インデックス参照は解決できないためチェック終了
                    break;
                },
            }
        }
        
        false
    }
    
    /// インターフェースを構築
    pub fn build_interface(&mut self) -> Result<()> {
        if let Some(ast) = &self.ast {
            // ASTからシンボルを収集
            for item in &ast.items {
                match item {
                    ModuleItem::Function(func) => {
                        self.interface.add_function(
                            func.name,
                            FunctionInfo {
                                name: func.name,
                                params: func.params.clone(),
                                return_type: func.return_type,
                                generic_params: func.generic_params.clone(),
                                visibility: func.visibility,
                                location: func.location,
                            },
                        );
                    },
                    
                    ModuleItem::TypeDecl(type_decl) => {
                        self.interface.add_type(
                            type_decl.name,
                            TypeInfo {
                                name: type_decl.name,
                                generic_params: type_decl.generic_params.clone(),
                                visibility: type_decl.visibility,
                                kind: match &type_decl.kind {
                                    TypeDeclKind::Struct(_) => TypeInfoKind::Struct,
                                    TypeDeclKind::Enum(_) => TypeInfoKind::Enum,
                                    TypeDeclKind::Interface(_) => TypeInfoKind::Interface,
                                    TypeDeclKind::Alias(_) => TypeInfoKind::Alias,
                                },
                                location: type_decl.location,
                            },
                        );
                    },
                    
                    ModuleItem::Const(const_decl) => {
                        self.interface.add_const(
                            const_decl.name,
                            ConstInfo {
                                name: const_decl.name,
                                type_id: const_decl.type_expr,
                                visibility: const_decl.visibility,
                                location: const_decl.location,
                            },
                        );
                    },
                    
                    ModuleItem::Module(submodule) => {
                        self.interface.add_module(
                            submodule.name,
                            ModuleInfo {
                                name: submodule.name,
                                visibility: submodule.visibility,
                                location: submodule.location,
                            },
                        );
                    },
                    
                    // その他の項目タイプ...
                }
            }
            
            // エクスポート項目を処理
            for export in &ast.exports {
                match export {
                    ExportDecl::All { source, location } => {
                        // ソースモジュールのすべての項目をエクスポート
                        self.exports.add_all_from(source.clone());
                    },
                    
                    ExportDecl::Selected { source, items, location } => {
                        // ソースモジュールの特定項目をエクスポート
                        for item in items {
                            self.exports.add_item_from(
                                source.clone(), 
                                item.name, 
                                item.alias.unwrap_or(item.name)
                            );
                        }
                    },
                    
                    ExportDecl::Item { name, alias, location } => {
                        // ローカル項目をエクスポート
                        self.exports.add_local_item(*name, alias.unwrap_or(*name));
                    },
                }
            }
            
            // インポート項目を処理
            for import in &ast.imports {
                match import {
                    ImportDecl::All { source, alias, visibility, location } => {
                        // すべての項目をインポート
                        self.imports.push(Import {
                            source: ModuleRef::Direct(source.clone()),
                            items: ImportItems::All,
                            visibility: *visibility,
                            alias: *alias,
                            location: *location,
                        });
                    },
                    
                    ImportDecl::Selected { source, items, visibility, location } => {
                        // 特定の項目をインポート
                        let specific_items = items.iter().map(|item| {
                            ImportItem {
                                name: item.name,
                                alias: item.alias,
                                visibility: item.visibility.unwrap_or(*visibility),
                                location: item.location,
                            }
                        }).collect();
                        
                        self.imports.push(Import {
                            source: ModuleRef::Direct(source.clone()),
                            items: ImportItems::Specific(specific_items),
                            visibility: *visibility,
                            alias: None,
                            location: *location,
                        });
                    },
                }
            }
            
            self.is_loaded = true;
            Ok(())
        } else {
            Err(CompilerError::new(
                ErrorKind::ModuleSystem,
                format!("モジュール {} にASTがありません", self.name.as_str()),
                SourceLocation::default(),
            ))
        }
    }
}

impl ModuleManager {
    /// 新しいモジュールマネージャを作成
    pub fn new(
        file_loader: Box<dyn ModuleFileLoader + Send>,
        type_registry: Arc<TypeRegistry>,
    ) -> Self {
        Self {
            modules: Vec::new(),
            id_map: HashMap::new(),
            root: None,
            file_loader,
            type_registry,
        }
    }
    
    /// モジュールを追加
    pub fn add_module(&mut self, module: Module) -> ModuleRef {
        let id = module.id.clone();
        let index = self.modules.len();
        
        self.modules.push(module);
        self.id_map.insert(id, index);
        
        ModuleRef::Index(index)
    }
    
    /// モジュールを取得
    pub fn get_module(&self, module_ref: &ModuleRef) -> Result<&Module> {
        match module_ref {
            ModuleRef::Direct(id) => {
                if let Some(&index) = self.id_map.get(id) {
                    Ok(&self.modules[index])
                } else {
                    Err(CompilerError::new(
                        ErrorKind::ModuleSystem,
                        format!("モジュール {} が見つかりません", id),
                        SourceLocation::default(),
                    ))
                }
            },
            ModuleRef::Index(index) => {
                if *index < self.modules.len() {
                    Ok(&self.modules[*index])
                } else {
                    Err(CompilerError::new(
                        ErrorKind::ModuleSystem,
                        format!("インデックス {} は範囲外です", index),
                        SourceLocation::default(),
                    ))
                }
            },
        }
    }
    
    /// モジュールの可変参照を取得
    pub fn get_module_mut(&mut self, module_ref: &ModuleRef) -> Result<&mut Module> {
        match module_ref {
            ModuleRef::Direct(id) => {
                if let Some(&index) = self.id_map.get(id) {
                    Ok(&mut self.modules[index])
                } else {
                    Err(CompilerError::new(
                        ErrorKind::ModuleSystem,
                        format!("モジュール {} が見つかりません", id),
                        SourceLocation::default(),
                    ))
                }
            },
            ModuleRef::Index(index) => {
                if *index < self.modules.len() {
                    Ok(&mut self.modules[*index])
                } else {
                    Err(CompilerError::new(
                        ErrorKind::ModuleSystem,
                        format!("インデックス {} は範囲外です", index),
                        SourceLocation::default(),
                    ))
                }
            },
        }
    }
    
    /// IDからモジュールを取得
    pub fn get_module_by_id(&self, id: &ModuleId) -> Result<&Module> {
        if let Some(&index) = self.id_map.get(id) {
            Ok(&self.modules[index])
        } else {
            Err(CompilerError::new(
                ErrorKind::ModuleSystem,
                format!("モジュール {} が見つかりません", id),
                SourceLocation::default(),
            ))
        }
    }
    
    /// IDからモジュール参照を取得
    pub fn get_module_ref(&self, id: &ModuleId) -> Result<ModuleRef> {
        if let Some(&index) = self.id_map.get(id) {
            Ok(ModuleRef::Index(index))
        } else {
            Err(CompilerError::new(
                ErrorKind::ModuleSystem,
                format!("モジュール {} が見つかりません", id),
                SourceLocation::default(),
            ))
        }
    }
    
    /// ルートモジュールを設定
    pub fn set_root(&mut self, root_ref: ModuleRef) {
        self.root = Some(root_ref);
    }
    
    /// ルートモジュールを取得
    pub fn get_root(&self) -> Option<&ModuleRef> {
        self.root.as_ref()
    }
    
    /// モジュールをロード
    pub fn load_module(&mut self, id: &ModuleId) -> Result<ModuleRef> {
        // 既にロード済みかチェック
        if let Ok(module_ref) = self.get_module_ref(id) {
            return Ok(module_ref);
        }
        
        // ファイルパスを解決
        let file_path = self.file_loader.resolve_module_path(id, None)?;
        
        // ファイルを読み込み
        let source = self.file_loader.load_file(&file_path)?;
        
        // パース
        let ast = parse_module(&source, Some(file_path.clone()))?;
        
        // モジュール名
        let name = id.last_segment().unwrap_or_else(|| Symbol::intern("unknown"));
        
        // モジュールを作成
        let mut module = Module::new(id.clone(), name);
        module.source_path = Some(file_path);
        module.ast = Some(Arc::new(ast));
        
        // インターフェースを構築
        module.build_interface()?;
        
        // モジュールを追加
        let module_ref = self.add_module(module);
        
        Ok(module_ref)
    }
    
    /// パスからモジュールをロード
    pub fn load_module_from_path<P: AsRef<Path>>(&mut self, path: P) -> Result<ModuleRef> {
        let path = path.as_ref();
        let id = ModuleId::from_file_path(path);
        self.load_module(&id)
    }
    
    /// プロジェクト全体をロード
    pub fn load_project<P: AsRef<Path>>(&mut self, root_path: P) -> Result<ModuleRef> {
        let root_path = root_path.as_ref();
        
        // ルートモジュールをロード
        let root_id = ModuleId::from_file_path(root_path);
        let root_ref = self.load_module(&root_id)?;
        
        // ルートとして設定
        self.set_root(root_ref.clone());
        
        // 依存関係を解決
        self.resolve_dependencies(&root_ref)?;
        
        Ok(root_ref)
    }
    
    /// 依存関係を解決
    pub fn resolve_dependencies(&mut self, module_ref: &ModuleRef) -> Result<()> {
        // 依存グラフを構築
        let mut dependency_graph = DependencyGraph::new();
        
        // モジュールの依存関係を追加
        self.add_module_to_dependency_graph(&mut dependency_graph, module_ref, None)?;
        
        // 循環依存のチェック
        dependency_graph.check_cycles()?;
        
        // 依存順にモジュールを並べ替え
        let sorted_modules = dependency_graph.topological_sort()?;
        
        // 結果を処理
        for &module_id in &sorted_modules {
            if let Some(&index) = self.id_map.get(module_id) {
                // 循環依存のフラグを更新
                let has_circular = dependency_graph.is_in_cycle(module_id);
                self.modules[index].has_circular_dependency = has_circular;
                
                // 依存リストを更新
                let dependencies: Vec<_> = dependency_graph
                    .get_dependencies(module_id)
                    .iter()
                    .map(|dep_id| ModuleRef::Direct(dep_id.clone()))
                    .collect();
                    
                self.modules[index].dependencies = dependencies;
            }
        }
        
        Ok(())
    }
    
    /// 依存グラフにモジュールを追加
    fn add_module_to_dependency_graph(
        &mut self,
        graph: &mut DependencyGraph,
        module_ref: &ModuleRef,
        source_ref: Option<&ModuleRef>,
    ) -> Result<()> {
        let module = self.get_module(module_ref)?;
        let module_id = module.id.clone();
        
        // ノードを追加
        graph.add_node(module_id.clone());
        
        // ソースモジュールがあれば依存エッジを追加
        if let Some(source_ref) = source_ref {
            let source = self.get_module(source_ref)?;
            graph.add_dependency(source.id.clone(), module_id.clone());
        }
        
        // 依存するモジュールを探索
        for import in &module.imports {
            match &import.source {
                ModuleRef::Direct(dep_id) => {
                    // 未ロードのモジュールがあればロード
                    if self.id_map.get(dep_id).is_none() {
                        let dep_ref = self.load_module(dep_id)?;
                        
                        // 深さ優先で依存を解決
                        self.add_module_to_dependency_graph(graph, &dep_ref, Some(module_ref))?;
                    } else {
                        // 既にロード済みなら依存関係を追加
                        graph.add_dependency(module_id.clone(), dep_id.clone());
                    }
                },
                ModuleRef::Index(index) => {
                    if *index < self.modules.len() {
                        let dep_id = self.modules[*index].id.clone();
                        graph.add_dependency(module_id.clone(), dep_id);
                    }
                },
            }
        }
        
        // サブモジュールも処理
        for (_, child_ref) in &module.children {
            self.add_module_to_dependency_graph(graph, child_ref, Some(module_ref))?;
        }
        
        Ok(())
    }
    
    /// シンボルの可視性をチェック
    pub fn is_symbol_visible(
        &self,
        symbol: Symbol,
        containing_module: &ModuleRef,
        accessing_module: &ModuleRef,
    ) -> Result<bool> {
        let container = self.get_module(containing_module)?;
        let accessor = self.get_module(accessing_module)?;
        
        Ok(container.is_symbol_visible(symbol, accessor))
    }
    
    /// シンボル名を解決
    pub fn resolve_symbol(
        &self,
        name: Symbol,
        module_ref: &ModuleRef,
    ) -> Result<SymbolResolution> {
        let resolver = SymbolResolver::new(self);
        resolver.resolve_symbol(name, module_ref)
    }
    
    /// 修飾されたシンボル名を解決
    pub fn resolve_qualified_symbol(
        &self,
        path: &[Symbol],
        module_ref: &ModuleRef,
    ) -> Result<SymbolResolution> {
        let resolver = SymbolResolver::new(self);
        resolver.resolve_qualified_symbol(path, module_ref)
    }
}

/// モジュールのパース関数（実際の実装はfrontendモジュールに依存）
fn parse_module(source: &str, file_path: Option<PathBuf>) -> Result<ModuleNode> {
    // Note: この実装はフロントエンドのパーサに委譲する必要があります
    
    // ダミー実装
    Ok(ModuleNode {
        name: Symbol::intern("dummy"),
        items: Vec::new(),
        imports: Vec::new(),
        exports: Vec::new(),
        location: SourceLocation::default(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    
    // TODO: テストケースを実装
} 