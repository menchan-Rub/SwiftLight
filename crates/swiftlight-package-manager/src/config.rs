use std::fs;
use std::io::{self, Read, Write};
use serde::{Serialize, Deserialize};
use toml;
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use std::env;
use anyhow::{Result, anyhow, Context};
use toml::Value as TomlValue;

use crate::error::PackageError;

/// パッケージマネージャーの設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageManagerConfig {
    /// 一般設定
    #[serde(default)]
    pub general: GeneralConfig,
    
    /// レジストリ設定
    #[serde(default)]
    pub registry: RegistryConfig,
    
    /// ビルド設定
    #[serde(default)]
    pub build: BuildConfig,
    
    /// キャッシュ設定
    #[serde(default)]
    pub cache: CacheConfig,
    
    /// ネットワーク設定
    #[serde(default)]
    pub network: NetworkConfig,
    
    /// セキュリティ設定
    #[serde(default)]
    pub security: SecurityConfig,
}

/// 一般設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralConfig {
    /// デフォルトのリポジトリ
    #[serde(default = "default_repository")]
    pub default_repository: String,
    
    /// カラー出力モード ("auto", "always", "never")
    #[serde(default = "default_color_mode")]
    pub color_mode: String,
    
    /// プログレスバーを表示
    #[serde(default = "default_true")]
    pub show_progress: bool,
    
    /// テレメトリデータを送信
    #[serde(default = "default_true")]
    pub telemetry_enabled: bool,
    
    /// プラグインを有効化
    #[serde(default = "default_true")]
    pub plugins_enabled: bool,
}

/// レジストリ設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryConfig {
    /// デフォルトレジストリ
    #[serde(default = "default_registry_url")]
    pub default_registry: String,
    
    /// 登録済みレジストリ
    #[serde(default)]
    pub registries: Vec<Registry>,
    
    /// セキュアレジストリのみ使用
    #[serde(default = "default_true")]
    pub secure_only: bool,
}

/// ビルド設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildConfig {
    /// 最大並列ジョブ数
    #[serde(default = "default_jobs")]
    pub jobs: usize,
    
    /// ターゲットディレクトリ
    #[serde(default)]
    pub target_dir: Option<PathBuf>,
    
    /// デフォルトフィーチャーを有効化
    #[serde(default = "default_true")]
    pub default_features: bool,
    
    /// リリースモードをデフォルトにする
    #[serde(default)]
    pub default_release: bool,
}

/// キャッシュ設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    /// キャッシュディレクトリ
    #[serde(default)]
    pub cache_dir: Option<PathBuf>,
    
    /// キャッシュの最大サイズ (MB)
    #[serde(default = "default_cache_size")]
    pub max_size: usize,
    
    /// キャッシュの有効期限 (日)
    #[serde(default = "default_cache_ttl")]
    pub ttl_days: usize,
    
    /// 自動クリーンアップを有効化
    #[serde(default = "default_true")]
    pub auto_cleanup: bool,
}

/// ネットワーク設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// プロキシURL
    #[serde(default)]
    pub proxy: Option<String>,
    
    /// タイムアウト (秒)
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u64,
    
    /// 同時接続数
    #[serde(default = "default_connections")]
    pub max_connections: usize,
    
    /// リトライ回数
    #[serde(default = "default_retries")]
    pub retry_count: usize,
}

/// セキュリティ設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// 脆弱性スキャンを有効化
    #[serde(default = "default_true")]
    pub vulnerability_scan: bool,
    
    /// 署名検証を有効化
    #[serde(default = "default_true")]
    pub signature_verification: bool,
    
    /// 信頼済み署名の鍵
    #[serde(default)]
    pub trusted_keys: Vec<String>,
    
    /// 使用禁止ライセンス
    #[serde(default)]
    pub forbidden_licenses: Vec<String>,
}

/// レジストリ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Registry {
    /// レジストリ名
    pub name: String,
    
    /// レジストリURL
    pub url: String,
    
    /// APIトークン
    #[serde(default)]
    pub token: Option<String>,
    
    /// デフォルトレジストリかどうか
    #[serde(default)]
    pub is_default: bool,
}

// デフォルト値関数
fn default_repository() -> String {
    "https://registry.swiftlight.dev".to_string()
}

fn default_color_mode() -> String {
    "auto".to_string()
}

fn default_registry_url() -> String {
    "https://registry.swiftlight.dev".to_string()
}

fn default_jobs() -> usize {
    std::thread::available_parallelism().map(|p| p.get()).unwrap_or(4)
}

fn default_cache_size() -> usize {
    1024 // 1GB
}

fn default_cache_ttl() -> usize {
    30 // 30日
}

fn default_timeout() -> u64 {
    30 // 30秒
}

fn default_connections() -> usize {
    8
}

fn default_retries() -> usize {
    3
}

fn default_true() -> bool {
    true
}

impl Default for PackageManagerConfig {
    fn default() -> Self {
        Self {
            general: GeneralConfig::default(),
            registry: RegistryConfig::default(),
            build: BuildConfig::default(),
            cache: CacheConfig::default(),
            network: NetworkConfig::default(),
            security: SecurityConfig::default(),
        }
    }
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            default_repository: default_repository(),
            color_mode: default_color_mode(),
            show_progress: default_true(),
            telemetry_enabled: default_true(),
            plugins_enabled: default_true(),
        }
    }
}

impl Default for RegistryConfig {
    fn default() -> Self {
        Self {
            default_registry: default_registry_url(),
            registries: Vec::new(),
            secure_only: default_true(),
        }
    }
}

impl Default for BuildConfig {
    fn default() -> Self {
        Self {
            jobs: default_jobs(),
            target_dir: None,
            default_features: default_true(),
            default_release: false,
        }
    }
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            cache_dir: None,
            max_size: default_cache_size(),
            ttl_days: default_cache_ttl(),
            auto_cleanup: default_true(),
        }
    }
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            proxy: None,
            timeout_seconds: default_timeout(),
            max_connections: default_connections(),
            retry_count: default_retries(),
        }
    }
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            vulnerability_scan: default_true(),
            signature_verification: default_true(),
            trusted_keys: Vec::new(),
            forbidden_licenses: Vec::new(),
        }
    }
}

impl PackageManagerConfig {
    /// 設定をロード
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        
        if !path.exists() {
            let default_config = PackageManagerConfig::default();
            return Ok(default_config);
        }
        let mut file = fs::File::open(path).map_err(|err| {
            PackageError::FilesystemError {
                path: path.to_path_buf(),
                message: format!("設定ファイルのオープンに失敗: {}", err),
            }
        })?;
        
        let mut contents = String::new();
        file.read_to_string(&mut contents).map_err(|err| {
            PackageError::ParseError(format!(
                "設定ファイルの読み込みに失敗しました [パス: {}]: {}",
                path.display(),
                err
            ))
        })?;
        
        let config: PackageManagerConfig = toml::from_str(&contents).map_err(|err| {
            PackageError::ParseError(format!(
                "設定ファイルの解析に失敗しました [パス: {}]: {}",
                path.display(),
                err
            ))
        })?;
        
        Ok(config)
    }
    
    /// 設定をファイルに保存
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let path = path.as_ref();
        
        // 親ディレクトリが存在しない場合は作成
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent).map_err(|err| {
                    PackageError::FilesystemError {
                        path: parent.to_path_buf(),
                        message: format!("ディレクトリを作成できませんでした: {}", err),
                    }
                })?;
            }
        }
        
        let content = toml::to_string_pretty(self).map_err(|err| {
            PackageError::ParseError(format!("設定をTOML形式に変換できませんでした: {}", err))
        })?;
        
        fs::write(path, content).map_err(|err| {
            PackageError::FilesystemError {
                path: path.to_path_buf(),
                message: format!("設定ファイルを書き込めませんでした: {}", err),
            }
        })?;
        
        Ok(())
    }
    
    /// グローバル設定ファイルのパスを取得
    pub fn global_config_path() -> PathBuf {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        home.join(".config").join("swiftlight").join("config.toml")
    }
    
    /// プロジェクト設定ファイルのパスを取得
    pub fn project_config_path<P: AsRef<Path>>(project_dir: P) -> PathBuf {
        project_dir.as_ref().join(".swiftlight").join("config.toml")
    }
}

/// 設定オプション
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// グローバル設定
    pub global: HashMap<String, ConfigValue>,
    
    /// ユーザー設定
    pub user: HashMap<String, ConfigValue>,
    
    /// プロジェクト設定
    pub project: HashMap<String, ConfigValue>,
    
    /// 一時設定
    pub temp: HashMap<String, ConfigValue>,
    
    /// 設定ファイルパス
    pub config_paths: ConfigPaths,
}

/// 設定ファイルパス
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigPaths {
    /// グローバル設定ファイルパス
    pub global: PathBuf,
    
    /// ユーザー設定ファイルパス
    pub user: PathBuf,
    
    /// プロジェクト設定ファイルパス
    pub project: Option<PathBuf>,
}

/// 設定値
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ConfigValue {
    /// 文字列
    String(String),
    
    /// 整数
    Integer(i64),
    
    /// 浮動小数点
    Float(f64),
    
    /// 真偽値
    Boolean(bool),
    
    /// 配列
    Array(Vec<ConfigValue>),
    
    /// テーブル
    Table(HashMap<String, ConfigValue>),
}

impl Config {
    /// 新しい設定を作成
    pub fn new() -> Result<Self> {
        let home_dir = dirs::home_dir().ok_or_else(|| anyhow!("ホームディレクトリが見つかりません"))?;
        let global_config_path = PathBuf::from("/etc/swiftlight/config.toml");
        let user_config_path = home_dir.join(".config").join("swiftlight").join("config.toml");
        
        let current_dir = env::current_dir()?;
        let project_config_path = find_project_config(&current_dir);
        
        let config_paths = ConfigPaths {
            global: global_config_path,
            user: user_config_path,
            project: project_config_path,
        };
        
        let mut config = Config {
            global: HashMap::new(),
            user: HashMap::new(),
            project: HashMap::new(),
            temp: HashMap::new(),
            config_paths,
        };
        
        // グローバル設定を読み込む
        if config.config_paths.global.exists() {
            config.global = read_config(&config.config_paths.global)?;
        }
        
        // ユーザー設定を読み込む
        if config.config_paths.user.exists() {
            config.user = read_config(&config.config_paths.user)?;
        } else {
            // ユーザー設定ディレクトリを作成
            fs::create_dir_all(config.config_paths.user.parent().unwrap())?;
        }
        
        // プロジェクト設定を読み込む
        if let Some(ref path) = config.config_paths.project {
            if path.exists() {
                config.project = read_config(path)?;
            }
        }
        
        Ok(config)
    }

    /// 設定値を取得
    pub fn get(&self, key: &str) -> Option<&ConfigValue> {
        // まず一時設定から検索
        if let Some(value) = self.temp.get(key) {
            return Some(value);
        }
        
        // 次にプロジェクト設定から検索
        if let Some(value) = self.project.get(key) {
            return Some(value);
        }
        
        // 次にユーザー設定から検索
        if let Some(value) = self.user.get(key) {
            return Some(value);
        }
        
        // 最後にグローバル設定から検索
        self.global.get(key)
    }

    /// 文字列設定値を取得
    pub fn get_string(&self, key: &str) -> Option<String> {
        match self.get(key) {
            Some(ConfigValue::String(s)) => Some(s.clone()),
            _ => None,
        }
    }

    /// 整数設定値を取得
    pub fn get_int(&self, key: &str) -> Option<i64> {
        match self.get(key) {
            Some(ConfigValue::Integer(i)) => Some(*i),
            Some(ConfigValue::Float(f)) => Some(*f as i64),
            Some(ConfigValue::String(s)) => s.parse::<i64>().ok(),
            _ => None,
        }
    }

    /// 浮動小数点設定値を取得
    pub fn get_float(&self, key: &str) -> Option<f64> {
        match self.get(key) {
            Some(ConfigValue::Float(f)) => Some(*f),
            Some(ConfigValue::Integer(i)) => Some(*i as f64),
            Some(ConfigValue::String(s)) => s.parse::<f64>().ok(),
            _ => None,
        }
    }

    /// 真偽値設定値を取得
    pub fn get_bool(&self, key: &str) -> Option<bool> {
        match self.get(key) {
            Some(ConfigValue::Boolean(b)) => Some(*b),
            Some(ConfigValue::String(s)) => {
                match s.to_lowercase().as_str() {
                    "true" | "yes" | "1" | "on" => Some(true),
                    "false" | "no" | "0" | "off" => Some(false),
                    _ => None,
                }
            },
            Some(ConfigValue::Integer(i)) => Some(*i != 0),
            _ => None,
        }
    }

    /// 配列設定値を取得
    pub fn get_array(&self, key: &str) -> Option<Vec<ConfigValue>> {
        match self.get(key) {
            Some(ConfigValue::Array(a)) => Some(a.clone()),
            _ => None,
        }
    }

    /// テーブル設定値を取得
    pub fn get_table(&self, key: &str) -> Option<HashMap<String, ConfigValue>> {
        match self.get(key) {
            Some(ConfigValue::Table(t)) => Some(t.clone()),
            _ => None,
        }
    }

    /// usize値を取得
    pub fn get_usize(&self, key: &str) -> Option<usize> {
        self.get_int(key).map(|i| i as usize)
    }

    /// PathBuf値を取得
    pub fn get_path(&self, key: &str) -> Option<PathBuf> {
        self.get_string(key).map(PathBuf::from)
    }

    /// 設定値を設定
    pub fn set(&mut self, key: &str, value: ConfigValue, level: ConfigLevel) -> &mut Self {
        match level {
            ConfigLevel::Global => {
                self.global.insert(key.to_string(), value);
            },
            ConfigLevel::User => {
                self.user.insert(key.to_string(), value);
            },
            ConfigLevel::Project => {
                self.project.insert(key.to_string(), value);
            },
            ConfigLevel::Temp => {
                self.temp.insert(key.to_string(), value);
            },
        }
        self
    }

    /// 設定値を削除
    pub fn remove(&mut self, key: &str, level: ConfigLevel) -> Option<ConfigValue> {
        match level {
            ConfigLevel::Global => self.global.remove(key),
            ConfigLevel::User => self.user.remove(key),
            ConfigLevel::Project => self.project.remove(key),
            ConfigLevel::Temp => self.temp.remove(key),
        }
    }

    /// 設定を保存
    pub fn save(&self, level: ConfigLevel) -> Result<()> {
        let (config_map, path) = match level {
            ConfigLevel::Global => (&self.global, &self.config_paths.global),
            ConfigLevel::User => (&self.user, &self.config_paths.user),
            ConfigLevel::Project => {
                if let Some(ref path) = self.config_paths.project {
                    (&self.project, path)
                } else {
                    return Err(anyhow!("プロジェクト設定ファイルのパスが設定されていません"));
                }
            },
            ConfigLevel::Temp => return Ok(()),  // 一時設定は保存しない
        };
        
        // 設定をTOML形式に変換
        let mut toml_map = toml::value::Table::new();
        for (key, value) in config_map {
            let toml_value = config_value_to_toml(value);
            toml_map.insert(key.clone(), toml_value);
        }
        
        let toml_string = toml::to_string_pretty(&toml_map)?;
        
        // 設定ファイルディレクトリを作成
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        
        // 設定ファイルに書き込む
        let mut file = fs::File::create(path)?;
        file.write_all(toml_string.as_bytes())?;
        
        Ok(())
    }

    /// キャッシュディレクトリを取得
    pub fn get_cache_dir(&self) -> PathBuf {
        if let Some(path) = self.get_path("cache.dir") {
            return path;
        }
        
        let home_dir = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        home_dir.join(".cache").join("swiftlight")
    }

    /// テンポラリディレクトリを取得
    pub fn get_temp_dir(&self) -> PathBuf {
        if let Some(path) = self.get_path("temp.dir") {
            return path;
        }
        
        env::temp_dir().join("swiftlight")
    }

    /// デフォルト設定を作成
    pub fn create_default() -> Self {
        let home_dir = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        
        let global_config_path = PathBuf::from("/etc/swiftlight/config.toml");
        let user_config_path = home_dir.join(".config").join("swiftlight").join("config.toml");
        
        let config_paths = ConfigPaths {
            global: global_config_path,
            user: user_config_path,
            project: None,
        };
        
        let mut global = HashMap::new();
        let mut user = HashMap::new();
        
        // デフォルトのグローバル設定
        global.insert("registry.url".to_string(), ConfigValue::String("https://registry.swiftlight.io".to_string()));
        global.insert("registry.secure".to_string(), ConfigValue::Boolean(true));
        global.insert("build.threads".to_string(), ConfigValue::Integer(4));
        global.insert("network.timeout".to_string(), ConfigValue::Integer(30));
        global.insert("cache.max_size".to_string(), ConfigValue::Integer(1024));  // 1GB
        
        // デフォルトのユーザー設定
        user.insert("user.name".to_string(), ConfigValue::String("".to_string()));
        user.insert("user.email".to_string(), ConfigValue::String("".to_string()));
        user.insert("user.token".to_string(), ConfigValue::String("".to_string()));
        user.insert("build.debug".to_string(), ConfigValue::Boolean(true));
        user.insert("editor".to_string(), ConfigValue::String("".to_string()));
        
        Config {
            global,
            user,
            project: HashMap::new(),
            temp: HashMap::new(),
            config_paths,
        }
    }
}

/// 設定レベル
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigLevel {
    /// グローバル設定（システム全体）
    Global,
    
    /// ユーザー設定（ユーザー固有）
    User,
    
    /// プロジェクト設定（プロジェクト固有）
    Project,
    
    /// 一時設定（メモリ内のみ）
    Temp,
}

/// 設定値をTOML値に変換
fn config_value_to_toml(value: &ConfigValue) -> TomlValue {
    match value {
        ConfigValue::String(s) => TomlValue::String(s.clone()),
        ConfigValue::Integer(i) => TomlValue::Integer(*i),
        ConfigValue::Float(f) => TomlValue::Float(*f),
        ConfigValue::Boolean(b) => TomlValue::Boolean(*b),
        ConfigValue::Array(a) => {
            let values: Vec<TomlValue> = a.iter().map(config_value_to_toml).collect();
            TomlValue::Array(values)
        },
        ConfigValue::Table(t) => {
            let mut table = toml::value::Table::new();
            for (key, value) in t {
                table.insert(key.clone(), config_value_to_toml(value));
            }
            TomlValue::Table(table)
        },
    }
}

/// TOML値を設定値に変換
fn toml_to_config_value(value: &TomlValue) -> ConfigValue {
    match value {
        TomlValue::String(s) => ConfigValue::String(s.clone()),
        TomlValue::Integer(i) => ConfigValue::Integer(*i),
        TomlValue::Float(f) => ConfigValue::Float(*f),
        TomlValue::Boolean(b) => ConfigValue::Boolean(*b),
        TomlValue::Array(a) => {
            let values: Vec<ConfigValue> = a.iter().map(toml_to_config_value).collect();
            ConfigValue::Array(values)
        },
        TomlValue::Table(t) => {
            let mut table = HashMap::new();
            for (key, value) in t {
                table.insert(key.clone(), toml_to_config_value(value));
            }
            ConfigValue::Table(table)
        },
        _ => ConfigValue::String("".to_string()),  // Datetime等は文字列に変換
    }
}

/// 設定ファイルを読み込む
fn read_config(path: &Path) -> Result<HashMap<String, ConfigValue>> {
    let mut file = fs::File::open(path)
        .with_context(|| format!("設定ファイルを開けません: {}", path.display()))?;
    
    let mut contents = String::new();
    file.read_to_string(&mut contents)
        .with_context(|| format!("設定ファイルを読み込めません: {}", path.display()))?;
    
    let toml_value: TomlValue = toml::from_str(&contents)
        .with_context(|| format!("設定ファイルの解析に失敗しました: {}", path.display()))?;
    
    let table = match toml_value {
        TomlValue::Table(table) => table,
        _ => return Err(anyhow!("設定ファイルのルートがテーブルではありません")),
    };
    
    let mut config = HashMap::new();
    for (key, value) in table {
        config.insert(key, toml_to_config_value(&value));
    }
    
    Ok(config)
}

/// プロジェクト設定ファイルを探す
pub fn find_project_config(start_dir: &Path) -> Option<PathBuf> {
    let mut current_dir = start_dir.to_path_buf();
    
    loop {
        let config_path = current_dir.join("swiftlight.config.toml");
        if config_path.exists() {
            return Some(config_path);
        }
        
        let manifest_path = current_dir.join("swiftlight.toml");
        if manifest_path.exists() {
            let config_path = current_dir.join(".swiftlight").join("config.toml");
            return Some(config_path);
        }
        
        if !current_dir.pop() {
            break;
        }
    }
    
    None
}

/// デフォルト設定ファイルを作成
pub fn create_default_config() -> Result<()> {
    let config = Config::create_default();
    
    // グローバル設定ファイルを作成
    if !config.config_paths.global.exists() {
        config.save(ConfigLevel::Global)?;
    }
    
    // ユーザー設定ファイルを作成
    if !config.config_paths.user.exists() {
        config.save(ConfigLevel::User)?;
    }
    
    Ok(())
}

/// ホームディレクトリを取得
pub fn get_swiftlight_home() -> Result<PathBuf> {
    if let Ok(path) = env::var("SWIFTLIGHT_HOME") {
        return Ok(PathBuf::from(path));
    }
    
    let home_dir = dirs::home_dir().ok_or_else(|| anyhow!("ホームディレクトリが見つかりません"))?;
    Ok(home_dir.join(".swiftlight"))
}

/// 設定のリセット
pub fn reset_config(level: ConfigLevel) -> Result<()> {
    let default_config = Config::create_default();
    
    match level {
        ConfigLevel::Global => {
            fs::remove_file(&default_config.config_paths.global)?;
            let mut config = Config::new()?;
            config.global = default_config.global;
            config.save(ConfigLevel::Global)?;
        },
        ConfigLevel::User => {
            fs::remove_file(&default_config.config_paths.user)?;
            let mut config = Config::new()?;
            config.user = default_config.user;
            config.save(ConfigLevel::User)?;
        },
        ConfigLevel::Project => {
            let config = Config::new()?;
            if let Some(path) = config.config_paths.project {
                fs::remove_file(path)?;
            }
        },
        ConfigLevel::Temp => {},  // 一時設定はリセットしない
    }
    
    Ok(())
}

impl Default for Config {
    fn default() -> Self {
        Config::create_default()
    }
}
 