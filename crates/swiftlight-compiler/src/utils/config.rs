// コンパイラの設定管理を行うモジュール
// 様々なコンパイラオプションやフラグを管理します

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::fs;
use std::io;
use std::str::FromStr;
use std::fmt;
use crate::driver::cache::CacheStrategy;

/// 最適化レベル
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptimizationLevel {
    /// 最適化なし (デバッグビルド)
    None,
    /// 基本的な最適化
    Basic,
    /// 標準的な最適化
    Default,
    /// 速度優先の最適化
    Speed,
    /// サイズ優先の最適化
    Size,
}

impl OptimizationLevel {
    /// 文字列からの変換
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "none" | "0" => Some(OptimizationLevel::None),
            "basic" | "1" => Some(OptimizationLevel::Basic),
            "default" | "2" => Some(OptimizationLevel::Default),
            "speed" | "3" => Some(OptimizationLevel::Speed),
            "size" | "s" => Some(OptimizationLevel::Size),
            _ => None,
        }
    }
    
    /// 文字列表現の取得
    pub fn as_str(&self) -> &'static str {
        match self {
            OptimizationLevel::None => "none",
            OptimizationLevel::Basic => "basic",
            OptimizationLevel::Default => "default",
            OptimizationLevel::Speed => "speed",
            OptimizationLevel::Size => "size",
        }
    }
}

impl FromStr for OptimizationLevel {
    type Err = String;
    
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_str(s).ok_or_else(|| format!("不正な最適化レベル: {}", s))
    }
}

impl fmt::Display for OptimizationLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl Default for OptimizationLevel {
    fn default() -> Self {
        OptimizationLevel::Default
    }
}

/// 出力形式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    /// オブジェクトファイル
    Object,
    /// 実行可能ファイル
    Executable,
    /// 静的ライブラリ
    StaticLib,
    /// 動的ライブラリ
    DynamicLib,
    /// LLVM IR
    LLVM,
    /// アセンブリコード
    Assembly,
}

impl OutputFormat {
    /// 文字列からの変換
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "obj" | "object" => Some(OutputFormat::Object),
            "exe" | "executable" => Some(OutputFormat::Executable),
            "staticlib" | "static" => Some(OutputFormat::StaticLib),
            "dynamiclib" | "dynamic" | "dylib" | "dll" | "so" => Some(OutputFormat::DynamicLib),
            "llvm" | "ir" => Some(OutputFormat::LLVM),
            "asm" | "assembly" => Some(OutputFormat::Assembly),
            _ => None,
        }
    }
    
    /// 文字列表現の取得
    pub fn as_str(&self) -> &'static str {
        match self {
            OutputFormat::Object => "object",
            OutputFormat::Executable => "executable",
            OutputFormat::StaticLib => "staticlib",
            OutputFormat::DynamicLib => "dynamiclib",
            OutputFormat::LLVM => "llvm",
            OutputFormat::Assembly => "assembly",
        }
    }
    
    /// デフォルトの拡張子を取得
    pub fn default_extension(&self) -> &'static str {
        match self {
            OutputFormat::Object => "o",
            OutputFormat::Executable => "",
            OutputFormat::StaticLib => "a",
            OutputFormat::DynamicLib => "so",
            OutputFormat::LLVM => "ll",
            OutputFormat::Assembly => "s",
        }
    }
}

impl FromStr for OutputFormat {
    type Err = String;
    
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_str(s).ok_or_else(|| format!("不正な出力形式: {}", s))
    }
}

impl fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl Default for OutputFormat {
    fn default() -> Self {
        OutputFormat::Executable
    }
}

/// コンパイラの設定
#[derive(Debug, Clone)]
pub struct CompilerConfig {
    /// 入力ファイルパス
    pub input_files: Vec<PathBuf>,
    /// 出力ファイルパス
    pub output_file: Option<PathBuf>,
    /// インクルードパス
    pub include_paths: Vec<PathBuf>,
    /// ライブラリパス
    pub library_paths: Vec<PathBuf>,
    /// リンクするライブラリ
    pub libraries: Vec<String>,
    /// 出力形式
    pub output_format: OutputFormat,
    /// 最適化レベル
    pub optimization_level: OptimizationLevel,
    /// デバッグ情報を含めるかどうか
    pub debug_info: bool,
    /// 警告を全てエラーとして扱うかどうか
    pub warnings_as_errors: bool,
    /// 有効にする警告
    pub enabled_warnings: Vec<String>,
    /// 無効にする警告
    pub disabled_warnings: Vec<String>,
    /// プリプロセッサ定義
    pub defines: HashMap<String, Option<String>>,
    /// スレッド数（並列コンパイル用）
    pub thread_count: Option<usize>,
    /// キャッシュ戦略
    pub cache_strategy: CacheStrategy,
    /// 追加の設定オプション
    pub extra_options: HashMap<String, String>,
}

impl Default for CompilerConfig {
    fn default() -> Self {
        Self {
            input_files: Vec::new(),
            output_file: None,
            include_paths: Vec::new(),
            library_paths: Vec::new(),
            libraries: Vec::new(),
            output_format: OutputFormat::default(),
            optimization_level: OptimizationLevel::default(),
            debug_info: false,
            warnings_as_errors: false,
            enabled_warnings: Vec::new(),
            disabled_warnings: Vec::new(),
            defines: HashMap::new(),
            thread_count: None,
            cache_strategy: CacheStrategy::Incremental,
            extra_options: HashMap::new(),
        }
    }
}

impl CompilerConfig {
    /// 新しいコンパイラ設定を作成
    pub fn new() -> Self {
        Self::default()
    }
    
    /// 入力ファイルを追加
    pub fn add_input_file<P: AsRef<Path>>(&mut self, path: P) -> &mut Self {
        self.input_files.push(path.as_ref().to_path_buf());
        self
    }
    
    /// 出力ファイルを設定
    pub fn set_output_file<P: AsRef<Path>>(&mut self, path: P) -> &mut Self {
        self.output_file = Some(path.as_ref().to_path_buf());
        self
    }
    
    /// インクルードパスを追加
    pub fn add_include_path<P: AsRef<Path>>(&mut self, path: P) -> &mut Self {
        self.include_paths.push(path.as_ref().to_path_buf());
        self
    }
    
    /// ライブラリパスを追加
    pub fn add_library_path<P: AsRef<Path>>(&mut self, path: P) -> &mut Self {
        self.library_paths.push(path.as_ref().to_path_buf());
        self
    }
    
    /// ライブラリを追加
    pub fn add_library<S: Into<String>>(&mut self, lib: S) -> &mut Self {
        self.libraries.push(lib.into());
        self
    }
    
    /// 出力形式を設定
    pub fn set_output_format(&mut self, format: OutputFormat) -> &mut Self {
        self.output_format = format;
        self
    }
    
    /// 最適化レベルを設定
    pub fn set_optimization_level(&mut self, level: OptimizationLevel) -> &mut Self {
        self.optimization_level = level;
        self
    }
    
    /// デバッグ情報の有無を設定
    pub fn set_debug_info(&mut self, debug_info: bool) -> &mut Self {
        self.debug_info = debug_info;
        self
    }
    
    /// 警告をエラーとして扱うかどうかを設定
    pub fn set_warnings_as_errors(&mut self, warnings_as_errors: bool) -> &mut Self {
        self.warnings_as_errors = warnings_as_errors;
        self
    }
    
    /// 警告を有効化
    pub fn enable_warning<S: Into<String>>(&mut self, warning: S) -> &mut Self {
        self.enabled_warnings.push(warning.into());
        self
    }
    
    /// 警告を無効化
    pub fn disable_warning<S: Into<String>>(&mut self, warning: S) -> &mut Self {
        self.disabled_warnings.push(warning.into());
        self
    }
    
    /// プリプロセッサ定義を追加
    pub fn add_define<K: Into<String>, V: Into<String>>(&mut self, key: K, value: Option<V>) -> &mut Self {
        self.defines.insert(key.into(), value.map(|v| v.into()));
        self
    }
    
    /// スレッド数を設定
    pub fn set_thread_count(&mut self, thread_count: usize) -> &mut Self {
        self.thread_count = Some(thread_count);
        self
    }
    
    /// キャッシュ戦略を設定
    pub fn set_cache_strategy(&mut self, strategy: CacheStrategy) -> &mut Self {
        self.cache_strategy = strategy;
        self
    }
    
    /// 追加オプションを設定
    pub fn set_extra_option<K: Into<String>, V: Into<String>>(&mut self, key: K, value: V) -> &mut Self {
        self.extra_options.insert(key.into(), value.into());
        self
    }
    
    /// 設定ファイルから読み込み
    pub fn from_file<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        let content = fs::read_to_string(path)?;
        // 実際の実装では、tomlやjsonなどのフォーマットからパースする
        // ここではダミーの実装を返す
        Ok(Self::default())
    }
    
    /// デフォルトの出力ファイル名を決定
    pub fn determine_output_file(&self) -> Option<PathBuf> {
        if self.output_file.is_some() {
            return self.output_file.clone();
        }
        
        if self.input_files.is_empty() {
            return None;
        }
        
        let first_input = &self.input_files[0];
        let stem = first_input.file_stem()?.to_string_lossy().to_string();
        
        let extension = match self.output_format {
            OutputFormat::Executable => if cfg!(windows) { "exe" } else { "" },
            format => format.default_extension(),
        };
        
        let mut output = PathBuf::from(stem);
        if !extension.is_empty() {
            output.set_extension(extension);
        }
        
        Some(output)
    }
    
    /// 設定を検証し、問題があればエラーを返す
    pub fn validate(&self) -> Result<(), String> {
        if self.input_files.is_empty() {
            return Err("入力ファイルが指定されていません".to_string());
        }
        
        for input in &self.input_files {
            if !input.exists() {
                return Err(format!("入力ファイルが存在しません: {}", input.display()));
            }
        }
        
        // その他の検証ロジック
        
        Ok(())
    }
} 