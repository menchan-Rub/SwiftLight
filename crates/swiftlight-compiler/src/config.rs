//! コンパイラ設定モジュール
//!
//! コンパイラの動作を設定するためのオプションと機能を提供します。

use std::path::{Path, PathBuf};
use std::collections::HashMap;
use std::str::FromStr;
use std::fmt;
use std::fs;

/// コンパイラの設定
#[derive(Debug, Clone)]
pub struct CompilerConfig {
    /// 最適化レベル（0-3）
    pub opt_level: OptimizationLevel,
    
    /// デバッグ情報を含めるか
    pub debug_info: bool,
    
    /// 警告をエラーとして扱うか
    pub warnings_as_errors: bool,
    
    /// 診断情報の詳細レベル（0-3）
    pub verbosity: u8,
    
    /// クロスコンパイルターゲット
    pub target: Option<String>,
    
    /// 定義マクロ
    pub defines: HashMap<String, String>,
    
    /// インクルードパス
    pub include_paths: Vec<PathBuf>,
    
    /// ライブラリパス
    pub lib_paths: Vec<PathBuf>,
    
    /// リンクするライブラリ
    pub libs: Vec<String>,
    
    /// 出力形式
    pub output_type: OutputType,
    
    /// スレッド数
    pub thread_count: usize,
    
    /// キャッシュを使用するか
    pub use_cache: bool,
    
    /// キャッシュディレクトリ
    pub cache_dir: PathBuf,
    
    /// インクリメンタルコンパイルを使用するか
    pub incremental: bool,
    
    /// インクリメンタルコンパイルディレクトリ
    pub incremental_dir: PathBuf,
    
    /// プラグインパス
    pub plugin_paths: Vec<PathBuf>,
    
    /// プラグイン引数
    pub plugin_args: HashMap<String, Vec<String>>,
    
    /// コンパイル対象の言語標準
    pub language_standard: LanguageStandard,
    
    /// コンパイル対象の言語拡張機能
    pub language_extensions: Vec<LanguageExtension>,
    
    /// カスタム設定
    pub custom: HashMap<String, String>,
}

/// 最適化レベル
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptimizationLevel {
    /// 最適化なし（デバッグビルド）
    None,
    
    /// 基本的な最適化
    Less,
    
    /// デフォルトの最適化
    Default,
    
    /// 積極的な最適化
    Aggressive,
    
    /// サイズ優先の最適化
    Size,
}

impl FromStr for OptimizationLevel {
    type Err = String;
    
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "0" => Ok(OptimizationLevel::None),
            "1" => Ok(OptimizationLevel::Less),
            "2" => Ok(OptimizationLevel::Default),
            "3" => Ok(OptimizationLevel::Aggressive),
            "s" | "z" => Ok(OptimizationLevel::Size),
            _ => Err(format!("無効な最適化レベル: {}", s)),
        }
    }
}

impl From<u8> for OptimizationLevel {
    fn from(level: u8) -> Self {
        match level {
            0 => OptimizationLevel::None,
            1 => OptimizationLevel::Less,
            2 => OptimizationLevel::Default,
            3 | _ => OptimizationLevel::Aggressive,
        }
    }
}

impl fmt::Display for OptimizationLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OptimizationLevel::None => write!(f, "0"),
            OptimizationLevel::Less => write!(f, "1"),
            OptimizationLevel::Default => write!(f, "2"),
            OptimizationLevel::Aggressive => write!(f, "3"),
            OptimizationLevel::Size => write!(f, "s"),
        }
    }
}

/// 出力形式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputType {
    /// 実行可能ファイル
    Executable,
    
    /// 共有ライブラリ
    SharedLibrary,
    
    /// 静的ライブラリ
    StaticLibrary,
    
    /// オブジェクトファイル
    ObjectFile,
    
    /// LLVM IR
    LLVMIR,
    
    /// アセンブリ
    Assembly,
}

impl FromStr for OutputType {
    type Err = String;
    
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "exe" | "executable" => Ok(OutputType::Executable),
            "dylib" | "shared" | "sharedlibrary" => Ok(OutputType::SharedLibrary),
            "staticlib" | "static" => Ok(OutputType::StaticLibrary),
            "obj" | "object" => Ok(OutputType::ObjectFile),
            "ir" | "llvm" | "llvmir" => Ok(OutputType::LLVMIR),
            "asm" | "assembly" => Ok(OutputType::Assembly),
            _ => Err(format!("無効な出力形式: {}", s)),
        }
    }
}

impl fmt::Display for OutputType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OutputType::Executable => write!(f, "executable"),
            OutputType::SharedLibrary => write!(f, "shared"),
            OutputType::StaticLibrary => write!(f, "static"),
            OutputType::ObjectFile => write!(f, "object"),
            OutputType::LLVMIR => write!(f, "llvmir"),
            OutputType::Assembly => write!(f, "assembly"),
        }
    }
}

/// 言語標準
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LanguageStandard {
    /// 最新標準
    Latest,
    
    /// SwiftLight 1.0
    V1_0,
    
    /// SwiftLight 0.5
    V0_5,
    
    /// SwiftLight 0.1
    V0_1,
}

impl FromStr for LanguageStandard {
    type Err = String;
    
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "latest" => Ok(LanguageStandard::Latest),
            "1.0" => Ok(LanguageStandard::V1_0),
            "0.5" => Ok(LanguageStandard::V0_5),
            "0.1" => Ok(LanguageStandard::V0_1),
            _ => Err(format!("無効な言語標準: {}", s)),
        }
    }
}

impl fmt::Display for LanguageStandard {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LanguageStandard::Latest => write!(f, "latest"),
            LanguageStandard::V1_0 => write!(f, "1.0"),
            LanguageStandard::V0_5 => write!(f, "0.5"),
            LanguageStandard::V0_1 => write!(f, "0.1"),
        }
    }
}

/// 言語拡張機能
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LanguageExtension {
    /// 型レベルプログラミング
    TypeLevelProgramming,
    
    /// 依存型
    DependentTypes,
    
    /// 量子型
    QuantumTypes,
    
    /// エフェクトシステム
    EffectSystem,
    
    /// 線形型
    LinearTypes,
    
    /// 精製型
    RefinementTypes,
    
    /// カスタム拡張
    Custom(String),
}

impl FromStr for LanguageExtension {
    type Err = String;
    
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "typelevel" => Ok(LanguageExtension::TypeLevelProgramming),
            "dependent" => Ok(LanguageExtension::DependentTypes),
            "quantum" => Ok(LanguageExtension::QuantumTypes),
            "effect" => Ok(LanguageExtension::EffectSystem),
            "linear" => Ok(LanguageExtension::LinearTypes),
            "refinement" => Ok(LanguageExtension::RefinementTypes),
            _ => Ok(LanguageExtension::Custom(s.to_string())),
        }
    }
}

impl fmt::Display for LanguageExtension {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LanguageExtension::TypeLevelProgramming => write!(f, "typelevel"),
            LanguageExtension::DependentTypes => write!(f, "dependent"),
            LanguageExtension::QuantumTypes => write!(f, "quantum"),
            LanguageExtension::EffectSystem => write!(f, "effect"),
            LanguageExtension::LinearTypes => write!(f, "linear"),
            LanguageExtension::RefinementTypes => write!(f, "refinement"),
            LanguageExtension::Custom(name) => write!(f, "{}", name),
        }
    }
}

impl Default for CompilerConfig {
    fn default() -> Self {
        Self {
            opt_level: OptimizationLevel::None,
            debug_info: true,
            warnings_as_errors: false,
            verbosity: 1,
            target: None,
            defines: HashMap::new(),
            include_paths: Vec::new(),
            lib_paths: Vec::new(),
            libs: Vec::new(),
            output_type: OutputType::Executable,
            thread_count: num_cpus::get(),
            use_cache: true,
            cache_dir: PathBuf::from(".cache"),
            incremental: true,
            incremental_dir: PathBuf::from(".incremental"),
            plugin_paths: Vec::new(),
            plugin_args: HashMap::new(),
            language_standard: LanguageStandard::Latest,
            language_extensions: Vec::new(),
            custom: HashMap::new(),
        }
    }
}

impl CompilerConfig {
    /// 新しいコンパイラ設定を作成
    pub fn new() -> Self {
        Self::default()
    }
    
    /// 設定ファイルから読み込み
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        let path = path.as_ref();
        
        let content = fs::read_to_string(path)
            .map_err(|e| format!("設定ファイルを読み込めませんでした: {}", e))?;
        
        Self::from_str(&content)
    }
    
    /// 最適化レベルを設定
    pub fn with_opt_level(mut self, level: OptimizationLevel) -> Self {
        self.opt_level = level;
        self
    }
    
    /// デバッグ情報を設定
    pub fn with_debug_info(mut self, debug: bool) -> Self {
        self.debug_info = debug;
        self
    }
    
    /// 警告をエラーとして扱うかを設定
    pub fn with_warnings_as_errors(mut self, warnings_as_errors: bool) -> Self {
        self.warnings_as_errors = warnings_as_errors;
        self
    }
    
    /// 詳細レベルを設定
    pub fn with_verbosity(mut self, verbosity: u8) -> Self {
        self.verbosity = verbosity;
        self
    }
    
    /// ターゲットを設定
    pub fn with_target(mut self, target: String) -> Self {
        self.target = Some(target);
        self
    }
    
    /// 定義を追加
    pub fn with_define(mut self, name: String, value: String) -> Self {
        self.defines.insert(name, value);
        self
    }
    
    /// インクルードパスを追加
    pub fn with_include_path<P: AsRef<Path>>(mut self, path: P) -> Self {
        self.include_paths.push(path.as_ref().to_path_buf());
        self
    }
    
    /// ライブラリパスを追加
    pub fn with_lib_path<P: AsRef<Path>>(mut self, path: P) -> Self {
        self.lib_paths.push(path.as_ref().to_path_buf());
        self
    }
    
    /// ライブラリを追加
    pub fn with_lib(mut self, lib: String) -> Self {
        self.libs.push(lib);
        self
    }
    
    /// 出力形式を設定
    pub fn with_output_type(mut self, output_type: OutputType) -> Self {
        self.output_type = output_type;
        self
    }
    
    /// スレッド数を設定
    pub fn with_thread_count(mut self, thread_count: usize) -> Self {
        self.thread_count = thread_count;
        self
    }
    
    /// キャッシュ設定
    pub fn with_cache(mut self, use_cache: bool) -> Self {
        self.use_cache = use_cache;
        self
    }
    
    /// キャッシュディレクトリを設定
    pub fn with_cache_dir<P: AsRef<Path>>(mut self, path: P) -> Self {
        self.cache_dir = path.as_ref().to_path_buf();
        self
    }
    
    /// インクリメンタルコンパイル設定
    pub fn with_incremental(mut self, incremental: bool) -> Self {
        self.incremental = incremental;
        self
    }
    
    /// インクリメンタルコンパイルディレクトリを設定
    pub fn with_incremental_dir<P: AsRef<Path>>(mut self, path: P) -> Self {
        self.incremental_dir = path.as_ref().to_path_buf();
        self
    }
    
    /// プラグインパスを追加
    pub fn with_plugin_path<P: AsRef<Path>>(mut self, path: P) -> Self {
        self.plugin_paths.push(path.as_ref().to_path_buf());
        self
    }
    
    /// プラグイン引数を追加
    pub fn with_plugin_arg(mut self, plugin: String, arg: String) -> Self {
        self.plugin_args
            .entry(plugin)
            .or_insert_with(Vec::new)
            .push(arg);
        self
    }
    
    /// 言語標準を設定
    pub fn with_language_standard(mut self, standard: LanguageStandard) -> Self {
        self.language_standard = standard;
        self
    }
    
    /// 言語拡張を追加
    pub fn with_language_extension(mut self, extension: LanguageExtension) -> Self {
        self.language_extensions.push(extension);
        self
    }
    
    /// カスタム設定を追加
    pub fn with_custom(mut self, key: String, value: String) -> Self {
        self.custom.insert(key, value);
        self
    }
    
    /// 設定をファイルに保存
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), String> {
        let path = path.as_ref();
        
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("ディレクトリを作成できませんでした: {}", e))?;
        }
        
        let content = self.to_string();
        
        fs::write(path, content)
            .map_err(|e| format!("設定ファイルを保存できませんでした: {}", e))?;
        
        Ok(())
    }
}

impl FromStr for CompilerConfig {
    type Err = String;
    
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // TOML形式の設定ファイルを解析する実装
        let toml_value: toml::Value = toml::from_str(s)
            .map_err(|e| format!("TOML解析エラー: {}", e))?;
        
        let toml_table = match toml_value {
            toml::Value::Table(table) => table,
            _ => return Err("設定はTOMLテーブルでなければなりません".to_string()),
        };
        
        let mut config = CompilerConfig::default();
        
        // 最適化レベル
        if let Some(opt_level) = toml_table.get("optimization_level") {
            if let Some(level_str) = opt_level.as_str() {
                config.opt_level = level_str.parse()
                    .map_err(|e| format!("無効な最適化レベル: {}", e))?;
            } else if let Some(level_int) = opt_level.as_integer() {
                config.opt_level = OptimizationLevel::from(level_int as u8);
            }
        }
        
        // デバッグ情報
        if let Some(debug_info) = toml_table.get("debug_info") {
            if let Some(value) = debug_info.as_bool() {
                config.debug_info = value;
            }
        }
        
        // 警告をエラーとして扱う
        if let Some(warnings_as_errors) = toml_table.get("warnings_as_errors") {
            if let Some(value) = warnings_as_errors.as_bool() {
                config.warnings_as_errors = value;
            }
        }
        
        // 詳細レベル
        if let Some(verbosity) = toml_table.get("verbosity") {
            if let Some(value) = verbosity.as_integer() {
                config.verbosity = value as u8;
            }
        }
        
        // ターゲット
        if let Some(target) = toml_table.get("target") {
            if let Some(value) = target.as_str() {
                config.target = Some(value.to_string());
            }
        }
        
        // 定義マクロ
        if let Some(defines) = toml_table.get("defines") {
            if let Some(defines_table) = defines.as_table() {
                for (key, value) in defines_table {
                    if let Some(value_str) = value.as_str() {
                        config.defines.insert(key.clone(), value_str.to_string());
                    }
                }
            }
        }
        
        // インクルードパス
        if let Some(include_paths) = toml_table.get("include_paths") {
            if let Some(paths) = include_paths.as_array() {
                for path in paths {
                    if let Some(path_str) = path.as_str() {
                        config.include_paths.push(PathBuf::from(path_str));
                    }
                }
            }
        }
        
        // ライブラリパス
        if let Some(lib_paths) = toml_table.get("lib_paths") {
            if let Some(paths) = lib_paths.as_array() {
                for path in paths {
                    if let Some(path_str) = path.as_str() {
                        config.lib_paths.push(PathBuf::from(path_str));
                    }
                }
            }
        }
        
        // リンクするライブラリ
        if let Some(libs) = toml_table.get("libs") {
            if let Some(libs_array) = libs.as_array() {
                for lib in libs_array {
                    if let Some(lib_str) = lib.as_str() {
                        config.libs.push(lib_str.to_string());
                    }
                }
            }
        }
        
        // 出力形式
        if let Some(output_type) = toml_table.get("output_type") {
            if let Some(type_str) = output_type.as_str() {
                config.output_type = type_str.parse()
                    .map_err(|e| format!("無効な出力形式: {}", e))?;
            }
        }
        
        // スレッド数
        if let Some(thread_count) = toml_table.get("thread_count") {
            if let Some(count) = thread_count.as_integer() {
                config.thread_count = count as usize;
            }
        }
        
        // キャッシュを使用するか
        if let Some(use_cache) = toml_table.get("use_cache") {
            if let Some(value) = use_cache.as_bool() {
                config.use_cache = value;
            }
        }
        
        // キャッシュディレクトリ
        if let Some(cache_dir) = toml_table.get("cache_dir") {
            if let Some(dir_str) = cache_dir.as_str() {
                config.cache_dir = PathBuf::from(dir_str);
            }
        }
        
        // インクリメンタルコンパイル
        if let Some(incremental) = toml_table.get("incremental") {
            if let Some(value) = incremental.as_bool() {
                config.incremental = value;
            }
        }
        
        // インクリメンタルディレクトリ
        if let Some(incremental_dir) = toml_table.get("incremental_dir") {
            if let Some(dir_str) = incremental_dir.as_str() {
                config.incremental_dir = PathBuf::from(dir_str);
            }
        }
        
        // プラグインパス
        if let Some(plugin_paths) = toml_table.get("plugin_paths") {
            if let Some(paths) = plugin_paths.as_array() {
                for path in paths {
                    if let Some(path_str) = path.as_str() {
                        config.plugin_paths.push(PathBuf::from(path_str));
                    }
                }
            }
        }
        
        // プラグイン引数
        if let Some(plugin_args) = toml_table.get("plugin_args") {
            if let Some(args_table) = plugin_args.as_table() {
                for (plugin, args) in args_table {
                    if let Some(args_array) = args.as_array() {
                        let mut plugin_args = Vec::new();
                        for arg in args_array {
                            if let Some(arg_str) = arg.as_str() {
                                plugin_args.push(arg_str.to_string());
                            }
                        }
                        config.plugin_args.insert(plugin.clone(), plugin_args);
                    }
                }
            }
        }
        
        // 言語標準
        if let Some(language_standard) = toml_table.get("language_standard") {
            if let Some(standard_str) = language_standard.as_str() {
                config.language_standard = standard_str.parse()
                    .map_err(|e| format!("無効な言語標準: {}", e))?;
            }
        }
        
        // 言語拡張
        if let Some(language_extensions) = toml_table.get("language_extensions") {
            if let Some(extensions_array) = language_extensions.as_array() {
                for extension in extensions_array {
                    if let Some(ext_str) = extension.as_str() {
                        let ext = ext_str.parse()
                            .map_err(|e| format!("無効な言語拡張: {}", e))?;
                        config.language_extensions.push(ext);
                    }
                }
            }
        }
        
        // カスタム設定
        if let Some(custom) = toml_table.get("custom") {
            if let Some(custom_table) = custom.as_table() {
                for (key, value) in custom_table {
                    if let Some(value_str) = value.as_str() {
                        config.custom.insert(key.clone(), value_str.to_string());
                    }
                }
            }
        }
        
        Ok(config)
    }
}

impl fmt::Display for CompilerConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // TOML形式の設定ファイルに変換する実装
        writeln!(f, "# SwiftLight コンパイラ設定")?;
        writeln!(f, "")?;
        
        // 基本設定
        writeln!(f, "# 基本設定")?;
        writeln!(f, "optimization_level = \"{}\"", self.opt_level)?;
        writeln!(f, "debug_info = {}", self.debug_info)?;
        writeln!(f, "warnings_as_errors = {}", self.warnings_as_errors)?;
        writeln!(f, "verbosity = {}", self.verbosity)?;
        
        // ターゲット設定
        if let Some(target) = &self.target {
            writeln!(f, "target = \"{}\"", target)?;
        }
        
        // 出力設定
        writeln!(f, "")?;
        writeln!(f, "# 出力設定")?;
        writeln!(f, "output_type = \"{}\"", self.output_type)?;
        
        // パフォーマンス設定
        writeln!(f, "")?;
        writeln!(f, "# パフォーマンス設定")?;
        writeln!(f, "thread_count = {}", self.thread_count)?;
        writeln!(f, "use_cache = {}", self.use_cache)?;
        writeln!(f, "cache_dir = \"{}\"", self.cache_dir.display())?;
        writeln!(f, "incremental = {}", self.incremental)?;
        writeln!(f, "incremental_dir = \"{}\"", self.incremental_dir.display())?;
        
        // 言語設定
        writeln!(f, "")?;
        writeln!(f, "# 言語設定")?;
        writeln!(f, "language_standard = \"{}\"", self.language_standard)?;
        
        if !self.language_extensions.is_empty() {
            write!(f, "language_extensions = [")?;
            let mut first = true;
            for ext in &self.language_extensions {
                if !first {
                    write!(f, ", ")?;
                }
                write!(f, "\"{}\"", ext)?;
                first = false;
            }
            writeln!(f, "]")?;
        }
        
        // 定義マクロ
        if !self.defines.is_empty() {
            writeln!(f, "")?;
            writeln!(f, "[defines]")?;
            for (key, value) in &self.defines {
                writeln!(f, "{} = \"{}\"", key, value)?;
            }
        }
        
        // パス設定
        if !self.include_paths.is_empty() {
            writeln!(f, "")?;
            writeln!(f, "# インクルードパス")?;
            write!(f, "include_paths = [")?;
            let mut first = true;
            for path in &self.include_paths {
                if !first {
                    write!(f, ", ")?;
                }
                write!(f, "\"{}\"", path.display())?;
                first = false;
            }
            writeln!(f, "]")?;
        }
        
        if !self.lib_paths.is_empty() {
            writeln!(f, "")?;
            writeln!(f, "# ライブラリパス")?;
            write!(f, "lib_paths = [")?;
            let mut first = true;
            for path in &self.lib_paths {
                if !first {
                    write!(f, ", ")?;
                }
                write!(f, "\"{}\"", path.display())?;
                first = false;
            }
            writeln!(f, "]")?;
        }
        
        if !self.libs.is_empty() {
            writeln!(f, "")?;
            writeln!(f, "# リンクするライブラリ")?;
            write!(f, "libs = [")?;
            let mut first = true;
            for lib in &self.libs {
                if !first {
                    write!(f, ", ")?;
                }
                write!(f, "\"{}\"", lib)?;
                first = false;
            }
            writeln!(f, "]")?;
        }
        
        // プラグイン設定
        if !self.plugin_paths.is_empty() {
            writeln!(f, "")?;
            writeln!(f, "# プラグインパス")?;
            write!(f, "plugin_paths = [")?;
            let mut first = true;
            for path in &self.plugin_paths {
                if !first {
                    write!(f, ", ")?;
                }
                write!(f, "\"{}\"", path.display())?;
                first = false;
            }
            writeln!(f, "]")?;
        }
        
        if !self.plugin_args.is_empty() {
            writeln!(f, "")?;
            writeln!(f, "[plugin_args]")?;
            for (plugin, args) in &self.plugin_args {
                write!(f, "{} = [", plugin)?;
                let mut first = true;
                for arg in args {
                    if !first {
                        write!(f, ", ")?;
                    }
                    write!(f, "\"{}\"", arg)?;
                    first = false;
                }
                writeln!(f, "]")?;
            }
        }
        
        // カスタム設定
        if !self.custom.is_empty() {
            writeln!(f, "")?;
            writeln!(f, "[custom]")?;
            for (key, value) in &self.custom {
                writeln!(f, "{} = \"{}\"", key, value)?;
            }
        }
        
        Ok(())
    }
}

/// 設定パーサー
pub struct ConfigParser {
    /// 設定ファイル検索パス
    search_paths: Vec<PathBuf>,
}

impl ConfigParser {
    /// 新しい設定パーサーを作成
    pub fn new() -> Self {
        Self {
            search_paths: vec![
                PathBuf::from("."),
                PathBuf::from("config"),
            ],
        }
    }
    
    /// 検索パスを追加
    pub fn add_search_path<P: AsRef<Path>>(&mut self, path: P) {
        self.search_paths.push(path.as_ref().to_path_buf());
    }
    
    /// 設定ファイルを検索して読み込み
    pub fn find_and_parse(&self, filename: &str) -> Result<CompilerConfig, String> {
        for path in &self.search_paths {
            let file_path = path.join(filename);
            if file_path.exists() {
                return CompilerConfig::from_file(file_path);
            }
        }
        
        Err(format!("設定ファイル '{}' が見つかりませんでした", filename))
    }
    
    /// コマンドライン引数から設定を解析
    pub fn parse_args(&self, args: &[String]) -> Result<CompilerConfig, String> {
        let mut config = CompilerConfig::default();
        
        let mut i = 0;
        while i < args.len() {
            let arg = &args[i];
            
            match arg.as_str() {
                "-O0" => config.opt_level = OptimizationLevel::None,
                "-O1" => config.opt_level = OptimizationLevel::Less,
                "-O2" => config.opt_level = OptimizationLevel::Default,
                "-O3" => config.opt_level = OptimizationLevel::Aggressive,
                "-Os" | "-Oz" => config.opt_level = OptimizationLevel::Size,
                
                "-g" => config.debug_info = true,
                "-g0" => config.debug_info = false,
                
                "-Werror" => config.warnings_as_errors = true,
                
                "-v" => config.verbosity = 1,
                "-vv" => config.verbosity = 2,
                "-vvv" => config.verbosity = 3,
                
                "--target" => {
                    if i + 1 < args.len() {
                        config.target = Some(args[i + 1].clone());
                        i += 1;
                    } else {
                        return Err("--target オプションには引数が必要です".to_string());
                    }
                },
                
                "-D" | "--define" => {
                    if i + 1 < args.len() {
                        let define = &args[i + 1];
                        let parts: Vec<&str> = define.split('=').collect();
                        if parts.len() == 2 {
                            config.defines.insert(parts[0].to_string(), parts[1].to_string());
                        } else {
                            config.defines.insert(parts[0].to_string(), "1".to_string());
                        }
                        i += 1;
                    } else {
                        return Err("-D オプションには引数が必要です".to_string());
                    }
                },
                
                "-I" | "--include" => {
                    if i + 1 < args.len() {
                        config.include_paths.push(PathBuf::from(&args[i + 1]));
                        i += 1;
                    } else {
                        return Err("-I オプションには引数が必要です".to_string());
                    }
                },
                
                "-L" | "--lib-path" => {
                    if i + 1 < args.len() {
                        config.lib_paths.push(PathBuf::from(&args[i + 1]));
                        i += 1;
                    } else {
                        return Err("-L オプションには引数が必要です".to_string());
                    }
                },
                
                "-l" | "--lib" => {
                    if i + 1 < args.len() {
                        config.libs.push(args[i + 1].clone());
                        i += 1;
                    } else {
                        return Err("-l オプションには引数が必要です".to_string());
                    }
                },
                
                "--output-type" => {
                    if i + 1 < args.len() {
                        config.output_type = OutputType::from_str(&args[i + 1])
                            .map_err(|e| format!("無効な出力形式: {}", e))?;
                        i += 1;
                    } else {
                        return Err("--output-type オプションには引数が必要です".to_string());
                    }
                },
                
                "-j" | "--jobs" => {
                    if i + 1 < args.len() {
                        config.thread_count = args[i + 1].parse()
                            .map_err(|_| format!("無効なスレッド数: {}", args[i + 1]))?;
                        i += 1;
                    } else {
                        return Err("-j オプションには引数が必要です".to_string());
                    }
                },
                
                "--no-cache" => config.use_cache = false,
                
                "--cache-dir" => {
                    if i + 1 < args.len() {
                        config.cache_dir = PathBuf::from(&args[i + 1]);
                        i += 1;
                    } else {
                        return Err("--cache-dir オプションには引数が必要です".to_string());
                    }
                },
                
                "--no-incremental" => config.incremental = false,
                
                "--incremental-dir" => {
                    if i + 1 < args.len() {
                        config.incremental_dir = PathBuf::from(&args[i + 1]);
                        i += 1;
                    } else {
                        return Err("--incremental-dir オプションには引数が必要です".to_string());
                    }
                },
                
                "--plugin" => {
                    if i + 1 < args.len() {
                        config.plugin_paths.push(PathBuf::from(&args[i + 1]));
                        i += 1;
                    } else {
                        return Err("--plugin オプションには引数が必要です".to_string());
                    }
                },
                
                "--plugin-arg" => {
                    if i + 2 < args.len() {
                        let plugin = args[i + 1].clone();
                        let arg = args[i + 2].clone();
                        config.plugin_args
                            .entry(plugin)
                            .or_insert_with(Vec::new)
                            .push(arg);
                        i += 2;
                    } else {
                        return Err("--plugin-arg オプションには2つの引数が必要です".to_string());
                    }
                },
                
                "--std" => {
                    if i + 1 < args.len() {
                        config.language_standard = LanguageStandard::from_str(&args[i + 1])
                            .map_err(|e| format!("無効な言語標準: {}", e))?;
                        i += 1;
                    } else {
                        return Err("--std オプションには引数が必要です".to_string());
                    }
                },
                
                "--extension" => {
                    if i + 1 < args.len() {
                        let extension = LanguageExtension::from_str(&args[i + 1])
                            .map_err(|e| format!("無効な言語拡張: {}", e))?;
                        config.language_extensions.push(extension);
                        i += 1;
                    } else {
                        return Err("--extension オプションには引数が必要です".to_string());
                    }
                },
                
                _ => {
                    if arg.starts_with("--") {
                        if i + 1 < args.len() && !args[i + 1].starts_with("-") {
                            config.custom.insert(
                                arg[2..].to_string(),
                                args[i + 1].clone(),
                            );
                            i += 1;
                        } else {
                            config.custom.insert(arg[2..].to_string(), "1".to_string());
                        }
                    }
                },
            }
            
            i += 1;
        }
        
        Ok(config)
    }
    
    /// 環境変数から設定を解析
    pub fn parse_env_vars(&self, prefix: &str) -> CompilerConfig {
        let mut config = CompilerConfig::default();
        
        for (key, value) in std::env::vars() {
            if key.starts_with(prefix) {
                let key = key[prefix.len()..].to_lowercase();
                
                match key.as_str() {
                    "opt_level" | "optimization_level" => {
                        if let Ok(level) = OptimizationLevel::from_str(&value) {
                            config.opt_level = level;
                        }
                    },
                    
                    "debug_info" | "debug" => {
                        config.debug_info = value == "1" || value.to_lowercase() == "true";
                    },
                    
                    "warnings_as_errors" | "werror" => {
                        config.warnings_as_errors = value == "1" || value.to_lowercase() == "true";
                    },
                    
                    "verbosity" | "verbose" => {
                        if let Ok(level) = value.parse::<u8>() {
                            config.verbosity = level;
                        }
                    },
                    
                    "target" => {
                        config.target = Some(value);
                    },
                    
                    "thread_count" | "jobs" => {
                        if let Ok(count) = value.parse::<usize>() {
                            config.thread_count = count;
                        }
                    },
                    
                    "use_cache" | "cache" => {
                        config.use_cache = value == "1" || value.to_lowercase() == "true";
                    },
                    
                    "cache_dir" => {
                        config.cache_dir = PathBuf::from(value);
                    },
                    
                    "incremental" => {
                        config.incremental = value == "1" || value.to_lowercase() == "true";
                    },
                    
                    "incremental_dir" => {
                        config.incremental_dir = PathBuf::from(value);
                    },
                    
                    "language_standard" | "std" => {
                        if let Ok(std) = LanguageStandard::from_str(&value) {
                            config.language_standard = std;
                        }
                    },
                    
                    _ => {
                        config.custom.insert(key, value);
                    },
                }
            }
        }
        
        config
    }
}

/// 設定マージャー
pub struct ConfigMerger;

impl ConfigMerger {
    /// 複数の設定をマージ
    pub fn merge(configs: &[CompilerConfig]) -> CompilerConfig {
        if configs.is_empty() {
            return CompilerConfig::default();
        }
        
        let mut result = configs[0].clone();
        
        for config in &configs[1..] {
            // マージロジック（優先度は後の設定が高い）
            result.opt_level = config.opt_level;
            result.debug_info = config.debug_info;
            result.warnings_as_errors = config.warnings_as_errors;
            result.verbosity = config.verbosity;
            
            if let Some(target) = &config.target {
                result.target = Some(target.clone());
            }
            
            for (key, value) in &config.defines {
                result.defines.insert(key.clone(), value.clone());
            }
            
            result.include_paths.extend(config.include_paths.iter().cloned());
            result.lib_paths.extend(config.lib_paths.iter().cloned());
            result.libs.extend(config.libs.iter().cloned());
            
            result.output_type = config.output_type;
            result.thread_count = config.thread_count;
            result.use_cache = config.use_cache;
            result.cache_dir = config.cache_dir.clone();
            result.incremental = config.incremental;
            result.incremental_dir = config.incremental_dir.clone();
            
            result.plugin_paths.extend(config.plugin_paths.iter().cloned());
            
            for (plugin, args) in &config.plugin_args {
                let entry = result.plugin_args
                    .entry(plugin.clone())
                    .or_insert_with(Vec::new);
                entry.extend(args.iter().cloned());
            }
            
            result.language_standard = config.language_standard;
            
            for extension in &config.language_extensions {
                if !result.language_extensions.contains(extension) {
                    result.language_extensions.push(extension.clone());
                }
            }
            
            for (key, value) in &config.custom {
                result.custom.insert(key.clone(), value.clone());
            }
        }
        
        result
    }
} 