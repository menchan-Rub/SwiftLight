// プラグイン管理を担当するモジュール
// コンパイラプラグインの読み込み、実行、管理を行います

use std::path::{Path, PathBuf};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::any::Any;
use crate::frontend::error::{Result, CompilerError, ErrorKind};

/// プラグインインターフェース
pub trait Plugin: Send + Sync {
    /// プラグイン名を取得
    fn name(&self) -> &str;
    
    /// プラグインのバージョンを取得
    fn version(&self) -> &str;
    
    /// プラグインの説明を取得
    fn description(&self) -> &str;
    
    /// プラグインの初期化
    fn initialize(&mut self) -> Result<()>;
    
    /// プラグインの終了処理
    fn shutdown(&mut self) -> Result<()>;
    
    /// コンパイル前のフック
    fn pre_compile(&self, context: &mut PluginContext) -> Result<()>;
    
    /// コンパイル後のフック
    fn post_compile(&self, context: &mut PluginContext) -> Result<()>;
    
    /// 任意のメソッド呼び出し
    fn call_method(&self, method_name: &str, args: &[&dyn Any]) -> Result<Box<dyn Any>>;
}

/// プラグインコンテキスト
pub struct PluginContext {
    /// コンパイル対象のファイルパス
    pub source_files: Vec<PathBuf>,
    /// 出力ファイルパス
    pub output_file: Option<PathBuf>,
    /// コンパイルオプション
    pub options: HashMap<String, String>,
    /// 共有データ
    pub shared_data: HashMap<String, Box<dyn Any + Send + Sync>>,
}

impl PluginContext {
    /// 新しいプラグインコンテキストを作成
    pub fn new() -> Self {
        Self {
            source_files: Vec::new(),
            output_file: None,
            options: HashMap::new(),
            shared_data: HashMap::new(),
        }
    }
    
    /// ソースファイルを追加
    pub fn add_source_file<P: AsRef<Path>>(&mut self, path: P) {
        self.source_files.push(path.as_ref().to_path_buf());
    }
    
    /// 出力ファイルを設定
    pub fn set_output_file<P: AsRef<Path>>(&mut self, path: P) {
        self.output_file = Some(path.as_ref().to_path_buf());
    }
    
    /// オプションを設定
    pub fn set_option(&mut self, key: &str, value: &str) {
        self.options.insert(key.to_string(), value.to_string());
    }
    
    /// 共有データを設定
    pub fn set_shared_data<T: 'static + Send + Sync>(&mut self, key: &str, value: T) {
        self.shared_data.insert(key.to_string(), Box::new(value));
    }
    
    /// 共有データを取得
    pub fn get_shared_data<T: 'static + Send + Sync>(&self, key: &str) -> Option<&T> {
        self.shared_data.get(key).and_then(|data| data.downcast_ref::<T>())
    }
    
    /// 共有データを可変で取得
    pub fn get_shared_data_mut<T: 'static + Send + Sync>(&mut self, key: &str) -> Option<&mut T> {
        self.shared_data.get_mut(key).and_then(|data| data.downcast_mut::<T>())
    }
}

/// プラグインマネージャー
pub struct PluginManager {
    /// 登録されたプラグイン
    plugins: Vec<Box<dyn Plugin>>,
    /// プラグイン検索パス
    search_paths: Vec<PathBuf>,
    /// プラグインコンテキスト
    context: Arc<Mutex<PluginContext>>,
}

impl PluginManager {
    /// 新しいプラグインマネージャーを作成
    pub fn new<P: AsRef<Path>>(search_paths: Vec<P>) -> Self {
        let paths = search_paths.iter()
            .map(|p| p.as_ref().to_path_buf())
            .collect();
            
        Self {
            plugins: Vec::new(),
            search_paths: paths,
            context: Arc::new(Mutex::new(PluginContext::new())),
        }
    }
    
    /// プラグインをロード
    pub fn load_plugin<P: AsRef<Path>>(&mut self, _path: P) -> Result<()> {
        // 動的ライブラリのロードは省略
        // 実際のコードでは、libloadingなどを使用して動的ライブラリをロードし、
        // プラグインのエントリポイント関数を取得して呼び出す
        
        Ok(())
    }
    
    /// ディレクトリからプラグインをロード
    pub fn load_plugins_from_directory<P: AsRef<Path>>(&mut self, _dir: P) -> Result<()> {
        // ディレクトリからプラグインを探して読み込む処理
        // 実際のコードでは、ディレクトリ内のすべての適合するファイルを探し、
        // load_pluginを呼び出す
        
        Ok(())
    }
    
    /// 検索パスからプラグインをロード
    pub fn load_plugins_from_search_paths(&mut self) -> Result<()> {
        for path in &self.search_paths {
            self.load_plugins_from_directory(path)?;
        }
        
        Ok(())
    }
    
    /// プラグインを直接登録
    pub fn register_plugin(&mut self, plugin: Box<dyn Plugin>) {
        self.plugins.push(plugin);
    }
    
    /// 名前でプラグインを取得
    pub fn get_plugin(&self, name: &str) -> Option<&dyn Plugin> {
        self.plugins.iter()
            .find(|p| p.name() == name)
            .map(|p| p.as_ref())
    }
    
    /// 全てのプラグインを初期化
    pub fn initialize_all(&mut self) -> Result<()> {
        for plugin in &mut self.plugins {
            plugin.initialize()?;
        }
        
        Ok(())
    }
    
    /// 全てのプラグインの終了処理
    pub fn shutdown_all(&mut self) -> Result<()> {
        for plugin in &mut self.plugins {
            plugin.shutdown()?;
        }
        
        Ok(())
    }
    
    /// コンパイル前のフックを全て実行
    pub fn run_pre_compile_hooks(&self) -> Result<()> {
        let mut context = self.context.lock().unwrap();
        
        for plugin in &self.plugins {
            plugin.pre_compile(&mut context)?;
        }
        
        Ok(())
    }
    
    /// コンパイル後のフックを全て実行
    pub fn run_post_compile_hooks(&self) -> Result<()> {
        let mut context = self.context.lock().unwrap();
        
        for plugin in &self.plugins {
            plugin.post_compile(&mut context)?;
        }
        
        Ok(())
    }
    
    /// コンテキストを取得
    pub fn context(&self) -> Arc<Mutex<PluginContext>> {
        self.context.clone()
    }
    
    /// コンテキストを設定
    pub fn set_context(&mut self, context: PluginContext) {
        *self.context.lock().unwrap() = context;
    }
    
    /// プラグイン数を取得
    pub fn plugin_count(&self) -> usize {
        self.plugins.len()
    }
} 