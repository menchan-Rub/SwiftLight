//! ユーティリティモジュール
//!
//! コンパイラで使用する共通のユーティリティ関数を提供します。

use std::path::{Path, PathBuf};
use std::fs;
use std::io;
use std::time::{Duration, Instant};
use std::sync::Arc;
use std::collections::HashMap;
use std::ffi::OsStr;

/// ファイル拡張子を取得
pub fn get_extension<P: AsRef<Path>>(path: P) -> Option<String> {
    path.as_ref()
        .extension()
        .and_then(OsStr::to_str)
        .map(String::from)
}

/// ファイルの内容を読み込む
pub fn read_file<P: AsRef<Path>>(path: P) -> io::Result<String> {
    fs::read_to_string(path)
}

/// ファイルに内容を書き込む
pub fn write_file<P: AsRef<Path>>(path: P, content: &str) -> io::Result<()> {
    let parent = path.as_ref().parent();
    if let Some(parent) = parent {
        if !parent.exists() {
            fs::create_dir_all(parent)?;
        }
    }
    
    fs::write(path, content)
}

/// ファイルパスからモジュール名を取得
pub fn path_to_module_name<P: AsRef<Path>>(path: P) -> String {
    path.as_ref()
        .file_stem()
        .and_then(OsStr::to_str)
        .map(String::from)
        .unwrap_or_else(|| "unnamed_module".to_string())
}

/// 関数実行時間を計測
pub fn measure_time<F, T>(f: F) -> (T, Duration)
where
    F: FnOnce() -> T,
{
    let start = Instant::now();
    let result = f();
    let duration = start.elapsed();
    
    (result, duration)
}

/// キーフォーマットの共通インターフェース
pub trait FormatKey {
    /// キーをフォーマット
    fn format(&self) -> String;
}

/// 簡易プロファイラー
pub struct Profiler {
    /// 計測結果
    timings: HashMap<String, Vec<Duration>>,
    /// 現在のセクション
    current_section: Option<(String, Instant)>,
}

impl Profiler {
    /// 新しいプロファイラーを作成
    pub fn new() -> Self {
        Self {
            timings: HashMap::new(),
            current_section: None,
        }
    }
    
    /// セクションを開始
    pub fn start_section(&mut self, name: &str) {
        self.current_section = Some((name.to_string(), Instant::now()));
    }
    
    /// セクションを終了
    pub fn end_section(&mut self) {
        if let Some((name, start)) = self.current_section.take() {
            let duration = start.elapsed();
            self.timings
                .entry(name)
                .or_insert_with(Vec::new)
                .push(duration);
        }
    }
    
    /// クロージャを実行し、時間を計測
    pub fn measure<F, T>(&mut self, name: &str, f: F) -> T
    where
        F: FnOnce() -> T,
    {
        self.start_section(name);
        let result = f();
        self.end_section();
        result
    }
    
    /// 平均実行時間を取得
    pub fn get_average(&self, name: &str) -> Option<Duration> {
        self.timings.get(name).map(|durations| {
            let total = durations.iter().sum::<Duration>();
            let count = durations.len() as u32;
            if count > 0 {
                total / count
            } else {
                Duration::from_secs(0)
            }
        })
    }
    
    /// 合計実行時間を取得
    pub fn get_total(&self, name: &str) -> Option<Duration> {
        self.timings
            .get(name)
            .map(|durations| durations.iter().sum())
    }
    
    /// すべての計測結果を取得
    pub fn get_all_timings(&self) -> &HashMap<String, Vec<Duration>> {
        &self.timings
    }
    
    /// 計測結果をクリア
    pub fn clear(&mut self) {
        self.timings.clear();
        self.current_section = None;
    }
    
    /// レポートを生成
    pub fn generate_report(&self) -> String {
        let mut report = String::new();
        report.push_str("===== プロファイリングレポート =====\n");
        
        let mut sorted_timings: Vec<_> = self.timings.iter().collect();
        sorted_timings.sort_by(|a, b| {
            let total_a: Duration = a.1.iter().sum();
            let total_b: Duration = b.1.iter().sum();
            total_b.cmp(&total_a)
        });
        
        for (name, durations) in sorted_timings {
            let total: Duration = durations.iter().sum();
            let count = durations.len();
            let average = if count > 0 {
                total / count as u32
            } else {
                Duration::from_secs(0)
            };
            
            report.push_str(&format!(
                "{}: 合計 {:?}, 平均 {:?}, 呼び出し回数 {}\n",
                name, total, average, count
            ));
        }
        
        report
    }
}

/// メモリ使用量トラッカー
pub struct MemoryTracker {
    /// 各セクションのメモリ使用量
    allocations: HashMap<String, usize>,
    /// ピーク時のメモリ使用量
    peak_usage: usize,
}

impl MemoryTracker {
    /// 新しいメモリトラッカーを作成
    pub fn new() -> Self {
        Self {
            allocations: HashMap::new(),
            peak_usage: 0,
        }
    }
    
    /// メモリ割り当てを記録
    pub fn record_allocation(&mut self, section: &str, size: usize) {
        let current = self.allocations.entry(section.to_string()).or_insert(0);
        *current += size;
        
        let total: usize = self.allocations.values().sum();
        if total > self.peak_usage {
            self.peak_usage = total;
        }
    }
    
    /// メモリ解放を記録
    pub fn record_deallocation(&mut self, section: &str, size: usize) {
        if let Some(current) = self.allocations.get_mut(section) {
            *current = current.saturating_sub(size);
        }
    }
    
    /// セクションのメモリ使用量を取得
    pub fn get_section_usage(&self, section: &str) -> usize {
        self.allocations.get(section).copied().unwrap_or(0)
    }
    
    /// 合計メモリ使用量を取得
    pub fn get_total_usage(&self) -> usize {
        self.allocations.values().sum()
    }
    
    /// ピーク時のメモリ使用量を取得
    pub fn get_peak_usage(&self) -> usize {
        self.peak_usage
    }
    
    /// レポートを生成
    pub fn generate_report(&self) -> String {
        let mut report = String::new();
        report.push_str("===== メモリ使用レポート =====\n");
        report.push_str(&format!("ピーク使用量: {} バイト\n", self.peak_usage));
        report.push_str(&format!("現在の使用量: {} バイト\n", self.get_total_usage()));
        
        let mut sorted_allocations: Vec<_> = self.allocations.iter().collect();
        sorted_allocations.sort_by(|a, b| b.1.cmp(a.1));
        
        for (section, size) in sorted_allocations {
            report.push_str(&format!("{}: {} バイト\n", section, size));
        }
        
        report
    }
}

/// 文字列インターナー
pub struct StringInterner {
    /// 文字列テーブル
    strings: HashMap<String, Arc<String>>,
}

impl StringInterner {
    /// 新しい文字列インターナーを作成
    pub fn new() -> Self {
        Self {
            strings: HashMap::new(),
        }
    }
    
    /// 文字列をインターン
    pub fn intern(&mut self, s: &str) -> Arc<String> {
        if let Some(interned) = self.strings.get(s) {
            interned.clone()
        } else {
            let interned = Arc::new(s.to_string());
            self.strings.insert(s.to_string(), interned.clone());
            interned
        }
    }
    
    /// インターンされた文字列の数を取得
    pub fn len(&self) -> usize {
        self.strings.len()
    }
    
    /// インターンされた文字列のメモリ使用量を取得（おおよそ）
    pub fn memory_usage(&self) -> usize {
        self.strings.iter().map(|(k, _)| k.len()).sum()
    }
}

/// ファイル変更監視
pub struct FileWatcher {
    // 実際の実装はファイル変更監視ライブラリに依存する
}

impl FileWatcher {
    /// 新しいファイル監視を作成
    pub fn new() -> Self {
        Self {}
    }
    
    /// ファイルの変更を監視
    pub fn watch<P: AsRef<Path>>(&mut self, _path: P) -> io::Result<()> {
        // 実際の実装はここに
        Ok(())
    }
    
    /// 変更を待機
    pub fn wait_for_changes(&self) -> io::Result<Vec<PathBuf>> {
        // 実際の実装はここに
        Ok(Vec::new())
    }
}

/// 変更検出器
pub struct ChangeDetector {
    /// ファイルの前回のハッシュ
    previous_hashes: HashMap<PathBuf, String>,
}

impl ChangeDetector {
    /// 新しい変更検出器を作成
    pub fn new() -> Self {
        Self {
            previous_hashes: HashMap::new(),
        }
    }
    
    /// ファイルが変更されたか確認
    pub fn is_file_changed<P: AsRef<Path>>(&mut self, path: P) -> io::Result<bool> {
        let path = path.as_ref();
        
        // ファイルのハッシュを計算
        let content = fs::read(path)?;
        let hash = calculate_hash(&content);
        
        // 前回のハッシュと比較
        let changed = match self.previous_hashes.get(path) {
            Some(previous_hash) => *previous_hash != hash,
            None => true,
        };
        
        // ハッシュを更新
        self.previous_hashes.insert(path.to_path_buf(), hash);
        
        Ok(changed)
    }
    
    /// 複数のファイルが変更されたか確認
    pub fn get_changed_files<P: AsRef<Path>>(&mut self, paths: &[P]) -> io::Result<Vec<PathBuf>> {
        let mut changed_files = Vec::new();
        
        for path in paths {
            if self.is_file_changed(path)? {
                changed_files.push(path.as_ref().to_path_buf());
            }
        }
        
        Ok(changed_files)
    }
}

/// ハッシュを計算
fn calculate_hash(data: &[u8]) -> String {
    use sha2::{Sha256, Digest};
    
    let mut hasher = Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();
    
    format!("{:x}", result)
}

/// 変更影響分析
pub struct ChangeImpactAnalyzer {
    /// 依存関係グラフ
    dependency_graph: HashMap<PathBuf, Vec<PathBuf>>,
}

impl ChangeImpactAnalyzer {
    /// 新しい変更影響分析を作成
    pub fn new() -> Self {
        Self {
            dependency_graph: HashMap::new(),
        }
    }
    
    /// 依存関係を追加
    pub fn add_dependency<P: AsRef<Path>, Q: AsRef<Path>>(&mut self, source: P, depends_on: Q) {
        let source = source.as_ref().to_path_buf();
        let depends_on = depends_on.as_ref().to_path_buf();
        
        self.dependency_graph
            .entry(source)
            .or_insert_with(Vec::new)
            .push(depends_on);
    }
    
    /// 変更の影響を受けるファイルを取得
    pub fn get_affected_files<P: AsRef<Path>>(&self, changed_file: P) -> Vec<PathBuf> {
        let changed_file = changed_file.as_ref().to_path_buf();
        let mut affected = Vec::new();
        
        for (source, dependencies) in &self.dependency_graph {
            if dependencies.contains(&changed_file) {
                affected.push(source.clone());
            }
        }
        
        affected
    }
    
    /// 変更の影響を受けるファイルを再帰的に取得
    pub fn get_all_affected_files<P: AsRef<Path>>(&self, changed_files: &[P]) -> Vec<PathBuf> {
        let mut affected = Vec::new();
        let mut to_process: Vec<PathBuf> = changed_files
            .iter()
            .map(|p| p.as_ref().to_path_buf())
            .collect();
        
        while let Some(file) = to_process.pop() {
            let new_affected = self.get_affected_files(&file);
            
            for affected_file in new_affected {
                if !affected.contains(&affected_file) && !to_process.contains(&affected_file) {
                    affected.push(affected_file.clone());
                    to_process.push(affected_file);
                }
            }
        }
        
        affected
    }
}

/// ビルドプラン
pub struct BuildPlan {
    /// ビルドステップ
    steps: Vec<BuildStep>,
}

/// ビルドステップ
pub struct BuildStep {
    /// ステップの種類
    kind: BuildStepKind,
    /// 入力ファイル
    inputs: Vec<PathBuf>,
    /// 出力ファイル
    outputs: Vec<PathBuf>,
    /// 依存するステップ
    dependencies: Vec<usize>,
}

/// ビルドステップの種類
pub enum BuildStepKind {
    /// コンパイル
    Compile,
    /// リンク
    Link,
    /// リソース処理
    Resource,
    /// カスタムコマンド
    Custom(String),
}

impl BuildPlan {
    /// 新しいビルドプランを作成
    pub fn new() -> Self {
        Self {
            steps: Vec::new(),
        }
    }
    
    /// ステップを追加
    pub fn add_step(&mut self, step: BuildStep) -> usize {
        let index = self.steps.len();
        self.steps.push(step);
        index
    }
    
    /// すべてのステップを取得
    pub fn get_steps(&self) -> &[BuildStep] {
        &self.steps
    }
    
    /// 依存関係を満たす順にステップを取得
    pub fn get_ordered_steps(&self) -> Vec<&BuildStep> {
        // トポロジカルソートを実装
        // （簡易実装のため、完全なトポロジカルソートは省略）
        let mut result = Vec::new();
        let mut visited = vec![false; self.steps.len()];
        
        for i in 0..self.steps.len() {
            if !visited[i] {
                self.visit(i, &mut visited, &mut result);
            }
        }
        
        result.iter().map(|&i| &self.steps[i]).collect()
    }
    
    /// 再帰的にステップを訪問（トポロジカルソート用）
    fn visit(&self, index: usize, visited: &mut [bool], result: &mut Vec<usize>) {
        visited[index] = true;
        
        for &dep in &self.steps[index].dependencies {
            if !visited[dep] {
                self.visit(dep, visited, result);
            }
        }
        
        result.push(index);
    }
}

impl BuildStep {
    /// 新しいビルドステップを作成
    pub fn new(kind: BuildStepKind) -> Self {
        Self {
            kind,
            inputs: Vec::new(),
            outputs: Vec::new(),
            dependencies: Vec::new(),
        }
    }
    
    /// 入力ファイルを追加
    pub fn add_input<P: AsRef<Path>>(&mut self, path: P) {
        self.inputs.push(path.as_ref().to_path_buf());
    }
    
    /// 出力ファイルを追加
    pub fn add_output<P: AsRef<Path>>(&mut self, path: P) {
        self.outputs.push(path.as_ref().to_path_buf());
    }
    
    /// 依存するステップを追加
    pub fn add_dependency(&mut self, step_index: usize) {
        if !self.dependencies.contains(&step_index) {
            self.dependencies.push(step_index);
        }
    }
    
    /// ステップの種類を取得
    pub fn kind(&self) -> &BuildStepKind {
        &self.kind
    }
    
    /// 入力ファイルを取得
    pub fn inputs(&self) -> &[PathBuf] {
        &self.inputs
    }
    
    /// 出力ファイルを取得
    pub fn outputs(&self) -> &[PathBuf] {
        &self.outputs
    }
    
    /// 依存ステップを取得
    pub fn dependencies(&self) -> &[usize] {
        &self.dependencies
    }
}

/// 診断ユーティリティモジュール
pub mod diagnostics {
    use std::fmt;
    use std::path::{Path, PathBuf};
    use std::collections::HashMap;
    use std::time::{Duration, Instant};
    use colored::*;
    use crate::frontend::error::{CompilerError, ErrorKind, SourceLocation};

    /// 診断メッセージレベル
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
    pub enum DiagnosticLevel {
        /// デバッグ情報（開発者向け）
        Debug,
        /// 情報メッセージ
        Info,
        /// 注意（警告になりうる）
        Notice,
        /// 警告
        Warning,
        /// エラー
        Error,
        /// 致命的エラー
        Fatal,
    }

    /// 診断メッセージ
    #[derive(Debug, Clone)]
    pub struct Diagnostic {
        /// メッセージレベル
        pub level: DiagnosticLevel,
        /// メッセージ
        pub message: String,
        /// ソースコード上の位置
        pub location: Option<SourceLocation>,
        /// ファイルパス
        pub file_path: Option<PathBuf>,
        /// 関連診断メッセージ
        pub related: Vec<Diagnostic>,
        /// 追加メモ
        pub notes: Vec<String>,
        /// タイムスタンプ
        pub timestamp: std::time::SystemTime,
    }

    impl Diagnostic {
        /// 新しい診断メッセージを作成
        pub fn new(level: DiagnosticLevel, message: impl Into<String>) -> Self {
            Self {
                level,
                message: message.into(),
                location: None,
                file_path: None,
                related: Vec::new(),
                notes: Vec::new(),
                timestamp: std::time::SystemTime::now(),
            }
        }

        /// 位置情報を追加
        pub fn with_location(mut self, location: SourceLocation) -> Self {
            self.location = Some(location);
            self
        }

        /// ファイルパスを追加
        pub fn with_file_path(mut self, path: impl Into<PathBuf>) -> Self {
            self.file_path = Some(path.into());
            self
        }

        /// 関連診断を追加
        pub fn with_related(mut self, related: Diagnostic) -> Self {
            self.related.push(related);
            self
        }

        /// メモを追加
        pub fn with_note(mut self, note: impl Into<String>) -> Self {
            self.notes.push(note.into());
            self
        }

        /// コンパイラエラーに変換
        pub fn to_compiler_error(&self) -> CompilerError {
            let kind = match self.level {
                DiagnosticLevel::Warning => ErrorKind::Warning,
                DiagnosticLevel::Error => ErrorKind::Semantic,
                DiagnosticLevel::Fatal => ErrorKind::Fatal,
                _ => ErrorKind::Note,
            };

            CompilerError::new(
                kind,
                self.message.clone(),
                self.location.clone(),
            )
        }
    }

    /// 診断エミッタ
    pub struct DiagnosticEmitter {
        /// 出力形式
        format: DiagnosticFormat,
        /// カラー表示を使用するか
        use_colors: bool,
        /// 診断メッセージリスト
        diagnostics: Vec<Diagnostic>,
        /// エラー数
        error_count: usize,
        /// 警告数
        warning_count: usize,
        /// 最大エラー数（これを超えると停止）
        max_errors: Option<usize>,
    }

    /// 診断出力形式
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum DiagnosticFormat {
        /// プレーンテキスト
        Plain,
        /// リッチテキスト（色付き）
        Rich,
        /// JSON形式
        Json,
        /// マークダウン形式
        Markdown,
        /// HTML形式
        Html,
    }

    impl DiagnosticEmitter {
        /// 新しい診断エミッタを作成
        pub fn new() -> Self {
            Self {
                format: DiagnosticFormat::Rich,
                use_colors: true,
                diagnostics: Vec::new(),
                error_count: 0,
                warning_count: 0,
                max_errors: Some(100),
            }
        }

        /// 診断を発行
        pub fn emit(&mut self, diagnostic: Diagnostic) {
            match diagnostic.level {
                DiagnosticLevel::Error | DiagnosticLevel::Fatal => {
                    self.error_count += 1;
                },
                DiagnosticLevel::Warning => {
                    self.warning_count += 1;
                },
                _ => {},
            }

            self.diagnostics.push(diagnostic);
        }

        /// エラーを発行
        pub fn emit_error(&mut self, message: impl Into<String>, location: Option<SourceLocation>) {
            let mut diag = Diagnostic::new(DiagnosticLevel::Error, message);
            if let Some(loc) = location {
                diag = diag.with_location(loc);
            }
            self.emit(diag);
        }

        /// 警告を発行
        pub fn emit_warning(&mut self, message: impl Into<String>, location: Option<SourceLocation>) {
            let mut diag = Diagnostic::new(DiagnosticLevel::Warning, message);
            if let Some(loc) = location {
                diag = diag.with_location(loc);
            }
            self.emit(diag);
        }

        /// 情報を発行
        pub fn emit_info(&mut self, message: impl Into<String>) {
            let diag = Diagnostic::new(DiagnosticLevel::Info, message);
            self.emit(diag);
        }

        /// デバッグ情報を発行
        pub fn emit_debug(&mut self, message: impl Into<String>) {
            let diag = Diagnostic::new(DiagnosticLevel::Debug, message);
            self.emit(diag);
        }

        /// エラー数を取得
        pub fn error_count(&self) -> usize {
            self.error_count
        }

        /// 警告数を取得
        pub fn warning_count(&self) -> usize {
            self.warning_count
        }

        /// エラーがあるかチェック
        pub fn has_errors(&self) -> bool {
            self.error_count > 0
        }

        /// 警告があるかチェック
        pub fn has_warnings(&self) -> bool {
            self.warning_count > 0
        }

        /// 全診断を取得
        pub fn diagnostics(&self) -> &[Diagnostic] {
            &self.diagnostics
        }

        /// 診断をクリア
        pub fn clear(&mut self) {
            self.diagnostics.clear();
            self.error_count = 0;
            self.warning_count = 0;
        }

        /// 出力形式を設定
        pub fn set_format(&mut self, format: DiagnosticFormat) {
            self.format = format;
        }

        /// カラー出力を設定
        pub fn set_use_colors(&mut self, use_colors: bool) {
            self.use_colors = use_colors;
        }

        /// 最大エラー数を設定
        pub fn set_max_errors(&mut self, max_errors: Option<usize>) {
            self.max_errors = max_errors;
        }

        /// 最大エラー数に達したかチェック
        pub fn max_errors_reached(&self) -> bool {
            if let Some(max) = self.max_errors {
                self.error_count >= max
            } else {
                false
            }
        }
    }

    /// 診断フォーマッタ
    pub struct DiagnosticFormatter;

    impl DiagnosticFormatter {
        /// 診断をフォーマット
        pub fn format(diagnostic: &Diagnostic, format: DiagnosticFormat, use_colors: bool) -> String {
            match format {
                DiagnosticFormat::Plain => Self::format_plain(diagnostic),
                DiagnosticFormat::Rich => Self::format_rich(diagnostic, use_colors),
                DiagnosticFormat::Json => Self::format_json(diagnostic),
                DiagnosticFormat::Markdown => Self::format_markdown(diagnostic),
                DiagnosticFormat::Html => Self::format_html(diagnostic),
            }
        }

        /// プレーンテキスト形式でフォーマット
        fn format_plain(diagnostic: &Diagnostic) -> String {
            let level = match diagnostic.level {
                DiagnosticLevel::Debug => "debug",
                DiagnosticLevel::Info => "info",
                DiagnosticLevel::Notice => "notice",
                DiagnosticLevel::Warning => "warning",
                DiagnosticLevel::Error => "error",
                DiagnosticLevel::Fatal => "fatal",
            };

            let mut result = format!("{}: {}", level, diagnostic.message);

            if let Some(location) = &diagnostic.location {
                result.push_str(&format!(" at {}:{}:{}", 
                    diagnostic.file_path.as_ref().map_or("unknown", |p| p.to_str().unwrap_or("unknown")), 
                    location.line, 
                    location.column
                ));
            }

            for note in &diagnostic.notes {
                result.push_str(&format!("\nnote: {}", note));
            }

            for related in &diagnostic.related {
                result.push_str(&format!("\n{}", Self::format_plain(related)));
            }

            result
        }

        /// リッチテキスト形式でフォーマット
        fn format_rich(diagnostic: &Diagnostic, use_colors: bool) -> String {
            // ここでcoloredクレートを使用して色付きテキストを生成
            let (level_str, level_style) = match diagnostic.level {
                DiagnosticLevel::Debug => ("debug", "blue"),
                DiagnosticLevel::Info => ("info", "green"),
                DiagnosticLevel::Notice => ("notice", "cyan"),
                DiagnosticLevel::Warning => ("warning", "yellow"),
                DiagnosticLevel::Error => ("error", "red"),
                DiagnosticLevel::Fatal => ("fatal", "red"),
            };

            let mut result = String::new();

            if use_colors {
                result = match level_style {
                    "blue" => format!("{}: {}", level_str.blue(), diagnostic.message),
                    "green" => format!("{}: {}", level_str.green(), diagnostic.message),
                    "cyan" => format!("{}: {}", level_str.cyan(), diagnostic.message),
                    "yellow" => format!("{}: {}", level_str.yellow(), diagnostic.message),
                    "red" => format!("{}: {}", level_str.red().bold(), diagnostic.message),
                    _ => format!("{}: {}", level_str, diagnostic.message),
                };
            } else {
                result = format!("{}: {}", level_str, diagnostic.message);
            }

            if let Some(location) = &diagnostic.location {
                result.push_str(&format!(" at {}:{}:{}", 
                    diagnostic.file_path.as_ref().map_or("unknown", |p| p.to_str().unwrap_or("unknown")), 
                    location.line, 
                    location.column
                ));
            }

            for note in &diagnostic.notes {
                if use_colors {
                    result.push_str(&format!("\n{}: {}", "note".cyan(), note));
                } else {
                    result.push_str(&format!("\nnote: {}", note));
                }
            }

            for related in &diagnostic.related {
                result.push_str(&format!("\n{}", Self::format_rich(related, use_colors)));
            }

            result
        }

        /// JSON形式でフォーマット
        fn format_json(diagnostic: &Diagnostic) -> String {
            // 実際の実装ではserde_jsonを使う
            String::from("{}")
        }

        /// マークダウン形式でフォーマット
        fn format_markdown(diagnostic: &Diagnostic) -> String {
            let level = match diagnostic.level {
                DiagnosticLevel::Debug => "Debug",
                DiagnosticLevel::Info => "Info",
                DiagnosticLevel::Notice => "Notice",
                DiagnosticLevel::Warning => "Warning",
                DiagnosticLevel::Error => "Error",
                DiagnosticLevel::Fatal => "Fatal",
            };

            let mut result = format!("## {} - {}\n", level, diagnostic.message);

            if let Some(location) = &diagnostic.location {
                result.push_str(&format!("\nLocation: **{}:{}:{}**\n", 
                    diagnostic.file_path.as_ref().map_or("unknown", |p| p.to_str().unwrap_or("unknown")), 
                    location.line, 
                    location.column
                ));
            }

            if !diagnostic.notes.is_empty() {
                result.push_str("\n### Notes\n");
                for note in &diagnostic.notes {
                    result.push_str(&format!("- {}\n", note));
                }
            }

            if !diagnostic.related.is_empty() {
                result.push_str("\n### Related\n");
                for related in &diagnostic.related {
                    result.push_str(&format!("\n{}\n", Self::format_markdown(related)));
                }
            }

            result
        }

        /// HTML形式でフォーマット
        fn format_html(diagnostic: &Diagnostic) -> String {
            // 実際の実装ではHTMLエスケープなども行う
            String::from("<div></div>")
        }
    }
} 