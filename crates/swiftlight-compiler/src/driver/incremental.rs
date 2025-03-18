// インクリメンタルコンパイルを管理するモジュール
// 変更検出、キャッシュ管理、増分コンパイルなどを担当

use std::path::{Path, PathBuf};
use std::collections::{HashMap, HashSet};
use std::time::{SystemTime, Duration};
use std::fs;
use std::io;

/// ファイル変更検出器
#[derive(Debug, Default)]
pub struct ChangeDetector {
    /// ファイルの最終変更時間キャッシュ
    file_timestamps: HashMap<PathBuf, SystemTime>,
}

impl ChangeDetector {
    /// 新しい変更検出器を作成
    pub fn new() -> Self {
        Self {
            file_timestamps: HashMap::new(),
        }
    }
    
    /// ファイルが変更されたかどうかを検出
    pub fn is_changed<P: AsRef<Path>>(&mut self, file_path: P) -> io::Result<bool> {
        let path = file_path.as_ref().to_path_buf();
        
        // ファイルのメタデータを取得
        let metadata = fs::metadata(&path)?;
        let last_modified = metadata.modified()?;
        
        // 以前のタイムスタンプと比較
        if let Some(prev_time) = self.file_timestamps.get(&path) {
            if last_modified > *prev_time {
                // 変更されている場合、タイムスタンプを更新
                self.file_timestamps.insert(path, last_modified);
                return Ok(true);
            }
            
            Ok(false)
        } else {
            // 初めて見るファイルの場合、タイムスタンプを記録
            self.file_timestamps.insert(path, last_modified);
            Ok(true)
        }
    }
    
    /// 複数のファイルの変更を検出
    pub fn changed_files<P: AsRef<Path>>(&mut self, files: &[P]) -> io::Result<Vec<PathBuf>> {
        let mut changed = Vec::new();
        
        for file in files {
            if self.is_changed(file)? {
                changed.push(file.as_ref().to_path_buf());
            }
        }
        
        Ok(changed)
    }
    
    /// タイムスタンプを更新
    pub fn update_timestamp<P: AsRef<Path>>(&mut self, file_path: P) -> io::Result<()> {
        let path = file_path.as_ref().to_path_buf();
        let metadata = fs::metadata(&path)?;
        let last_modified = metadata.modified()?;
        
        self.file_timestamps.insert(path, last_modified);
        Ok(())
    }
    
    /// タイムスタンプをクリア
    pub fn clear(&mut self) {
        self.file_timestamps.clear();
    }
}

/// 変更影響分析器
#[derive(Debug, Default)]
pub struct ChangeImpactAnalyzer {
    /// モジュール間の依存関係
    dependencies: HashMap<String, HashSet<String>>,
}

impl ChangeImpactAnalyzer {
    /// 新しい変更影響分析器を作成
    pub fn new() -> Self {
        Self {
            dependencies: HashMap::new(),
        }
    }
    
    /// 依存関係を追加
    pub fn add_dependency(&mut self, module: &str, depends_on: &str) {
        self.dependencies
            .entry(module.to_string())
            .or_insert_with(HashSet::new)
            .insert(depends_on.to_string());
    }
    
    /// 複数の依存関係を追加
    pub fn add_dependencies(&mut self, module: &str, deps: &[&str]) {
        let set = self.dependencies
            .entry(module.to_string())
            .or_insert_with(HashSet::new);
            
        for dep in deps {
            set.insert(dep.to_string());
        }
    }
    
    /// 変更の影響を受けるモジュールを分析
    pub fn analyze_impact(&self, changed_modules: &[&str]) -> HashSet<String> {
        let mut impacted = HashSet::new();
        
        // 変更されたモジュール自体を追加
        for module in changed_modules {
            impacted.insert(module.to_string());
        }
        
        // 変更の影響を受けるモジュールを検出
        let mut queue: Vec<String> = changed_modules.iter().map(|&s| s.to_string()).collect();
        let mut visited = HashSet::new();
        
        while let Some(current) = queue.pop() {
            if visited.contains(&current) {
                continue;
            }
            
            visited.insert(current.clone());
            
            // 現在のモジュールに依存する全てのモジュールを探す
            for (module, deps) in &self.dependencies {
                if deps.contains(&current) && !visited.contains(module) {
                    impacted.insert(module.clone());
                    queue.push(module.clone());
                }
            }
        }
        
        impacted
    }
    
    /// 依存関係をクリア
    pub fn clear(&mut self) {
        self.dependencies.clear();
    }
}

/// インクリメンタルコンパイル管理
#[derive(Debug)]
pub struct IncrementalCompilationManager {
    /// インクリメンタルコンパイルを有効にするかどうか
    enabled: bool,
    /// インクリメンタルコンパイル情報の保存ディレクトリ
    storage_dir: PathBuf,
    /// 変更検出器
    change_detector: ChangeDetector,
    /// 変更影響分析器
    impact_analyzer: ChangeImpactAnalyzer,
    /// コンパイル済みモジュールの状態
    compiled_modules: HashMap<String, ModuleState>,
}

/// モジュールの状態
#[derive(Debug, Clone)]
struct ModuleState {
    /// モジュール名
    name: String,
    /// ソースファイルパス
    source_path: PathBuf,
    /// IRデータのハッシュ
    ir_hash: String,
    /// 最終コンパイル時間
    last_compile_time: SystemTime,
    /// 依存モジュール
    dependencies: HashSet<String>,
}

impl IncrementalCompilationManager {
    /// 新しいインクリメンタルコンパイル管理を作成
    pub fn new<P: AsRef<Path>>(storage_dir: Option<P>, enabled: bool) -> Self {
        let dir = storage_dir
            .map(|p| p.as_ref().to_path_buf())
            .unwrap_or_else(|| PathBuf::from(".swiftlight/incremental"));
            
        Self {
            enabled,
            storage_dir: dir,
            change_detector: ChangeDetector::new(),
            impact_analyzer: ChangeImpactAnalyzer::new(),
            compiled_modules: HashMap::new(),
        }
    }
    
    /// インクリメンタルコンパイルが有効かどうか
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
    
    /// インクリメンタルコンパイルを有効化
    pub fn enable(&mut self) {
        self.enabled = true;
    }
    
    /// インクリメンタルコンパイルを無効化
    pub fn disable(&mut self) {
        self.enabled = false;
    }
    
    /// モジュールの状態を記録
    pub fn record_module<P: AsRef<Path>>(
        &mut self,
        name: &str,
        source_path: P,
        ir_hash: &str,
        dependencies: &HashSet<String>,
    ) {
        if !self.enabled {
            return;
        }
        
        let state = ModuleState {
            name: name.to_string(),
            source_path: source_path.as_ref().to_path_buf(),
            ir_hash: ir_hash.to_string(),
            last_compile_time: SystemTime::now(),
            dependencies: dependencies.clone(),
        };
        
        self.compiled_modules.insert(name.to_string(), state);
        
        // 依存関係を変更影響分析器に登録
        for dep in dependencies {
            self.impact_analyzer.add_dependency(name, dep);
        }
    }
    
    /// コンパイルする必要があるモジュールを特定
    pub fn modules_to_recompile<P: AsRef<Path>>(&mut self, source_files: &[P]) -> io::Result<Vec<PathBuf>> {
        if !self.enabled {
            // インクリメンタルコンパイルが無効な場合は全てのファイルをコンパイル
            return Ok(source_files.iter().map(|p| p.as_ref().to_path_buf()).collect());
        }
        
        // 変更されたファイルを検出
        let changed_files = self.change_detector.changed_files(source_files)?;
        
        if changed_files.is_empty() {
            return Ok(Vec::new());
        }
        
        // 変更されたモジュール名を特定
        let mut changed_modules = Vec::new();
        for file in &changed_files {
            for (name, state) in &self.compiled_modules {
                if state.source_path == *file {
                    changed_modules.push(name.as_str());
                }
            }
        }
        
        // 影響を受けるモジュールを分析
        let impacted = self.impact_analyzer.analyze_impact(&changed_modules);
        
        // 影響を受けるモジュールのソースファイルを収集
        let mut to_recompile = Vec::new();
        for module in impacted {
            if let Some(state) = self.compiled_modules.get(&module) {
                to_recompile.push(state.source_path.clone());
            }
        }
        
        Ok(to_recompile)
    }
    
    /// 状態をクリア
    pub fn clear(&mut self) {
        self.change_detector.clear();
        self.impact_analyzer.clear();
        self.compiled_modules.clear();
    }
} 