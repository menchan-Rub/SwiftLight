// モジュール管理を担当するモジュール
// コンパイル単位としてのモジュールの登録、検索、管理を行います

use std::path::{Path, PathBuf};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use crate::middleend::ir;
use crate::frontend::error::Result;

/// モジュール情報
#[derive(Debug, Clone)]
pub struct ModuleInfo {
    /// モジュール名
    pub name: String,
    /// ソースファイルパス
    pub source_path: PathBuf,
    /// 依存モジュール
    pub dependencies: HashSet<String>,
    /// エクスポートシンボル
    pub exports: HashSet<String>,
    /// モジュールのハッシュ値
    pub hash: Option<String>,
    /// IRモジュールへの参照
    pub ir_module: Option<Arc<ir::Module>>,
}

impl ModuleInfo {
    /// 新しいモジュール情報を作成
    pub fn new<P: AsRef<Path>>(name: String, source_path: P) -> Self {
        Self {
            name,
            source_path: source_path.as_ref().to_path_buf(),
            dependencies: HashSet::new(),
            exports: HashSet::new(),
            hash: None,
            ir_module: None,
        }
    }

    /// 依存モジュールを追加
    pub fn add_dependency(&mut self, module_name: &str) {
        self.dependencies.insert(module_name.to_string());
    }

    /// エクスポートシンボルを追加
    pub fn add_export(&mut self, symbol_name: &str) {
        self.exports.insert(symbol_name.to_string());
    }

    /// IRモジュールを設定
    pub fn set_ir_module(&mut self, module: Arc<ir::Module>) {
        self.ir_module = Some(module);
    }

    /// ハッシュ値を設定
    pub fn set_hash(&mut self, hash: String) {
        self.hash = Some(hash);
    }
}

/// モジュールマネージャー
#[derive(Debug, Default)]
pub struct ModuleManager {
    /// 登録されたモジュール
    modules: HashMap<String, ModuleInfo>,
    /// モジュールの検索パス
    search_paths: Vec<PathBuf>,
}

impl ModuleManager {
    /// 新しいモジュールマネージャーを作成
    pub fn new() -> Self {
        Self {
            modules: HashMap::new(),
            search_paths: Vec::new(),
        }
    }

    /// モジュールを登録
    pub fn register_module<P: AsRef<Path>>(&mut self, name: &str, source_path: P) -> &mut ModuleInfo {
        let module_info = ModuleInfo::new(name.to_string(), source_path);
        self.modules.insert(name.to_string(), module_info);
        self.modules.get_mut(name).unwrap()
    }

    /// モジュールを取得
    pub fn get_module(&self, name: &str) -> Option<&ModuleInfo> {
        self.modules.get(name)
    }

    /// モジュールを可変で取得
    pub fn get_module_mut(&mut self, name: &str) -> Option<&mut ModuleInfo> {
        self.modules.get_mut(name)
    }

    /// モジュールが登録されているかどうかを確認
    pub fn has_module(&self, name: &str) -> bool {
        self.modules.contains_key(name)
    }

    /// モジュールを削除
    pub fn remove_module(&mut self, name: &str) -> Option<ModuleInfo> {
        self.modules.remove(name)
    }

    /// 全てのモジュールを取得
    pub fn get_all_modules(&self) -> &HashMap<String, ModuleInfo> {
        &self.modules
    }

    /// モジュールの数を取得
    pub fn module_count(&self) -> usize {
        self.modules.len()
    }

    /// 検索パスを追加
    pub fn add_search_path<P: AsRef<Path>>(&mut self, path: P) {
        self.search_paths.push(path.as_ref().to_path_buf());
    }

    /// モジュールファイルを検索
    pub fn find_module_file(&self, module_name: &str) -> Option<PathBuf> {
        // モジュール名からファイル名を生成
        let file_name = format!("{}.sl", module_name);
        
        // 登録済みモジュールをチェック
        if let Some(info) = self.modules.get(module_name) {
            return Some(info.source_path.clone());
        }
        
        // 検索パスを順に探索
        for path in &self.search_paths {
            let full_path = path.join(&file_name);
            if full_path.exists() {
                return Some(full_path);
            }
        }
        
        None
    }

    /// モジュールのトポロジカルソート
    pub fn topological_sort(&self) -> Result<Vec<String>> {
        // 入力次数（依存されている数）を計算
        let mut in_degree: HashMap<String, usize> = HashMap::new();
        
        // 全モジュールの入力次数を0で初期化
        for name in self.modules.keys() {
            in_degree.insert(name.clone(), 0);
        }
        
        // 依存関係から入力次数を更新
        for module in self.modules.values() {
            for dep in &module.dependencies {
                *in_degree.entry(dep.clone()).or_insert(0) += 1;
            }
        }
        
        // 入力次数が0のモジュールをキューに追加
        let mut queue = Vec::new();
        for (name, &degree) in &in_degree {
            if degree == 0 {
                queue.push(name.clone());
            }
        }
        
        // トポロジカルソート
        let mut result = Vec::new();
        
        while let Some(name) = queue.pop() {
            result.push(name.clone());
            
            if let Some(module) = self.modules.get(&name) {
                for dep in &module.dependencies {
                    if let Some(degree) = in_degree.get_mut(dep) {
                        *degree -= 1;
                        if *degree == 0 {
                            queue.push(dep.clone());
                        }
                    }
                }
            }
        }
        
        // 全てのモジュールが処理されたかチェック
        if result.len() != self.modules.len() {
            // 循環依存がある場合
            Err(crate::frontend::error::CompilerError::new(
                crate::frontend::error::ErrorKind::CircularDependency,
                "モジュール間に循環依存関係が検出されました".to_string(),
                None
            ))
        } else {
            Ok(result)
        }
    }
} 