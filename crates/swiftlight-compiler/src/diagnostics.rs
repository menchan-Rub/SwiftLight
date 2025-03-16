//! 診断情報モジュール
//! 
//! コンパイラの診断情報を管理し、整形して出力するためのユーティリティを提供します。

pub use crate::driver::diagnostics::{Diagnostic, DiagnosticLevel, DiagnosticReporter, DiagnosticFormatter};

use std::path::PathBuf;
use std::io;
use std::fmt;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// 診断情報の出力形式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticFormat {
    /// テキスト形式（コンソール用）
    Text,
    /// JSON形式
    Json,
    /// マークダウン形式
    Markdown,
    /// HTML形式
    Html,
}

/// 診断情報の出力設定
#[derive(Debug, Clone)]
pub struct DiagnosticOptions {
    /// 出力形式
    pub format: DiagnosticFormat,
    /// カラー出力を使用するか
    pub use_colors: bool,
    /// 詳細レベル（0-3）
    pub verbosity: u8,
    /// ソースコードを表示するか
    pub show_source: bool,
    /// 修正候補を表示するか
    pub show_suggestions: bool,
    /// 関連する診断を表示するか
    pub show_related: bool,
    /// エラーコードを表示するか
    pub show_error_codes: bool,
    /// 警告をエラーとして扱うか
    pub warnings_as_errors: bool,
    /// 無視するエラーコード
    pub ignored_codes: Vec<String>,
    /// 最大エラー数
    pub max_errors: usize,
}

impl Default for DiagnosticOptions {
    fn default() -> Self {
        Self {
            format: DiagnosticFormat::Text,
            use_colors: true,
            verbosity: 1,
            show_source: true,
            show_suggestions: true,
            show_related: true,
            show_error_codes: true,
            warnings_as_errors: false,
            ignored_codes: Vec::new(),
            max_errors: 20,
        }
    }
}

/// 診断エミッタ - 診断情報を出力する
pub struct DiagnosticEmitter {
    /// 診断レポーター
    reporter: Arc<Mutex<DiagnosticReporter>>,
    /// 診断オプション
    options: DiagnosticOptions,
    /// ソースコードキャッシュ
    source_cache: HashMap<PathBuf, String>,
    /// 出力先
    output: Box<dyn io::Write + Send>,
}

impl DiagnosticEmitter {
    /// 新しい診断エミッタを作成
    pub fn new(
        reporter: Arc<Mutex<DiagnosticReporter>>,
        options: DiagnosticOptions,
        output: Box<dyn io::Write + Send>,
    ) -> Self {
        Self {
            reporter,
            options,
            source_cache: HashMap::new(),
            output,
        }
    }
    
    /// 診断情報を処理して出力
    pub fn emit(&mut self) -> io::Result<()> {
        let diagnostics = {
            let reporter = self.reporter.lock().unwrap();
            reporter.get_diagnostics()
        };
        
        for diagnostic in diagnostics {
            let formatted = self.format_diagnostic(&diagnostic.to_compiler_error());
            writeln!(self.output, "{}", formatted)?;
        }
        
        Ok(())
    }
    
    /// 特定の診断情報を出力
    pub fn emit_diagnostic(&mut self, diagnostic: &Diagnostic) -> io::Result<()> {
        let formatted = self.format_diagnostic(&diagnostic.to_compiler_error());
        writeln!(self.output, "{}", formatted)?;
        Ok(())
    }
    
    /// 診断情報をフォーマット
    fn format_diagnostic(&self, diagnostic: &crate::driver::compiler::CompilerError) -> String {
        // フォーマッタを作成
        let formatter = DiagnosticFormatter::new(self.options.use_colors, self.options.verbosity > 1);
        
        // 診断情報をDiagnostic型に変換
        let diag = self.convert_compiler_error(diagnostic);
        
        // フォーマット
        let mut result = formatter.format(&diag);
        
        // ソースコードを表示
        if self.options.show_source {
            if let Some(source) = self.get_source_snippet(&diag) {
                result.push_str("\n\n");
                result.push_str(&source);
            }
        }
        
        result
    }
    
    /// コンパイラエラーを診断情報に変換
    fn convert_compiler_error(&self, error: &crate::driver::compiler::CompilerError) -> Diagnostic {
        use crate::driver::compiler::ErrorKind;
        
        let level = match error.kind() {
            ErrorKind::Warning => DiagnosticLevel::Warning,
            ErrorKind::Info => DiagnosticLevel::Info,
            _ => DiagnosticLevel::Error,
        };
        
        let mut diag = Diagnostic::new(level, error.message().to_string());
        
        if let Some(file_path) = error.file_path() {
            // ファイルの行と列を取得（仮の実装）
            diag = diag.with_location(file_path.clone(), 1, 1);
        }
        
        diag
    }
    
    /// ソースコードの該当部分を取得
    fn get_source_snippet(&self, diagnostic: &Diagnostic) -> Option<String> {
        let file_path = diagnostic.file_path.as_ref()?;
        let line = diagnostic.line?;
        let column = diagnostic.column?;
        
        // ソースコードをキャッシュから取得またはファイルから読み込み
        let source = if let Some(cached) = self.source_cache.get(file_path) {
            cached
        } else {
            match std::fs::read_to_string(file_path) {
                Ok(content) => {
                    // キャッシュに追加
                    let result = content.clone();
                    let mut cache = self.source_cache.clone();
                    cache.insert(file_path.clone(), content);
                    &result
                }
                Err(_) => return None,
            }
        };
        
        // 該当行の前後数行を抽出
        let context_lines = 2; // 前後2行を表示
        let lines: Vec<&str> = source.lines().collect();
        
        if line == 0 || line > lines.len() {
            return None;
        }
        
        let start_line = line.saturating_sub(context_lines);
        let end_line = std::cmp::min(line + context_lines, lines.len());
        
        let mut result = String::new();
        
        for i in start_line..=end_line {
            let line_content = lines[i - 1];
            let line_num = format!("{:4} | ", i);
            
            if i == line {
                // エラー行
                result.push_str(&line_num);
                result.push_str(line_content);
                result.push('\n');
                
                // エラー位置を指示する矢印
                result.push_str("     | ");
                for _ in 0..column.saturating_sub(1) {
                    result.push(' ');
                }
                result.push_str("^");
            } else {
                // 前後の文脈行
                result.push_str(&line_num);
                result.push_str(line_content);
                result.push('\n');
            }
        }
        
        Some(result)
    }
    
    /// すべての診断情報を文字列としてフォーマット
    pub fn format_all_diagnostics(&self) -> String {
        let diagnostics = {
            let reporter = self.reporter.lock().unwrap();
            reporter.get_diagnostics()
        };
        
        let mut result = String::new();
        
        for diagnostic in diagnostics {
            let formatted = self.format_diagnostic(&diagnostic);
            result.push_str(&formatted);
            result.push_str("\n\n");
        }
        
        result
    }
    
    /// エラー数を取得
    pub fn error_count(&self) -> usize {
        let reporter = self.reporter.lock().unwrap();
        reporter.error_count()
    }
    
    /// 警告数を取得
    pub fn warning_count(&self) -> usize {
        let reporter = self.reporter.lock().unwrap();
        reporter.warning_count()
    }
}

/// コンパイルエラーの種類
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompilerErrorKind {
    /// 字句解析エラー
    Lexical,
    /// 構文エラー
    Syntax,
    /// 名前解決エラー
    NameResolution,
    /// 型チェックエラー
    Type,
    /// 意味解析エラー
    Semantic,
    /// リンクエラー
    Link,
    /// IOエラー
    IO,
    /// 内部エラー
    Internal,
    /// 型システムエラー
    TypeSystem,
    /// 最適化エラー
    Optimization,
    /// コード生成エラー
    CodeGeneration,
    /// 警告
    Warning,
    /// 情報
    Info,
}

impl fmt::Display for CompilerErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CompilerErrorKind::Lexical => write!(f, "字句解析エラー"),
            CompilerErrorKind::Syntax => write!(f, "構文エラー"),
            CompilerErrorKind::NameResolution => write!(f, "名前解決エラー"),
            CompilerErrorKind::Type => write!(f, "型エラー"),
            CompilerErrorKind::Semantic => write!(f, "意味解析エラー"),
            CompilerErrorKind::Link => write!(f, "リンクエラー"),
            CompilerErrorKind::IO => write!(f, "I/Oエラー"),
            CompilerErrorKind::Internal => write!(f, "内部エラー"),
            CompilerErrorKind::TypeSystem => write!(f, "型システムエラー"),
            CompilerErrorKind::Optimization => write!(f, "最適化エラー"),
            CompilerErrorKind::CodeGeneration => write!(f, "コード生成エラー"),
            CompilerErrorKind::Warning => write!(f, "警告"),
            CompilerErrorKind::Info => write!(f, "情報"),
        }
    }
}

/// エラーコード
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErrorCode(String);

impl ErrorCode {
    /// 新しいエラーコードを作成
    pub fn new(code: &str) -> Self {
        Self(code.to_string())
    }
    
    /// コード文字列を取得
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// 診断情報の重要度
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    /// 情報
    Info = 0,
    /// ヒント
    Hint = 1,
    /// 注意
    Note = 2,
    /// 警告
    Warning = 3,
    /// エラー
    Error = 4,
    /// 致命的エラー
    Fatal = 5,
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Severity::Info => write!(f, "情報"),
            Severity::Hint => write!(f, "ヒント"),
            Severity::Note => write!(f, "注意"),
            Severity::Warning => write!(f, "警告"),
            Severity::Error => write!(f, "エラー"),
            Severity::Fatal => write!(f, "致命的エラー"),
        }
    }
}

/// 診断ビルダー
pub struct DiagnosticBuilder {
    /// 診断レベル
    level: DiagnosticLevel,
    /// メッセージ
    message: String,
    /// コード
    code: Option<String>,
    /// ファイルパス
    file_path: Option<PathBuf>,
    /// 行番号
    line: Option<usize>,
    /// 列番号
    column: Option<usize>,
    /// 関連する診断
    related: Vec<Diagnostic>,
    /// 修正候補
    suggestions: Vec<String>,
}

impl DiagnosticBuilder {
    /// 新しい診断ビルダーを作成
    pub fn new(level: DiagnosticLevel, message: String) -> Self {
        Self {
            level,
            message,
            code: None,
            file_path: None,
            line: None,
            column: None,
            related: Vec::new(),
            suggestions: Vec::new(),
        }
    }
    
    /// コードを設定
    pub fn with_code(mut self, code: String) -> Self {
        self.code = Some(code);
        self
    }
    
    /// 位置情報を設定
    pub fn with_location(mut self, file_path: PathBuf, line: usize, column: usize) -> Self {
        self.file_path = Some(file_path);
        self.line = Some(line);
        self.column = Some(column);
        self
    }
    
    /// 関連する診断を追加
    pub fn with_related(mut self, related: Diagnostic) -> Self {
        self.related.push(related);
        self
    }
    
    /// 修正候補を追加
    pub fn with_suggestion(mut self, suggestion: String) -> Self {
        self.suggestions.push(suggestion);
        self
    }
    
    /// 診断を構築
    pub fn build(self) -> Diagnostic {
        let mut diag = Diagnostic::new(self.level, self.message);
        
        if let Some(code) = self.code {
            diag = diag.with_code(code);
        }
        
        if let (Some(path), Some(line), Some(col)) = (self.file_path, self.line, self.column) {
            diag = diag.with_location(path, line, col);
        }
        
        for related in self.related {
            diag = diag.with_related(related);
        }
        
        for suggestion in self.suggestions {
            diag = diag.with_suggestion(suggestion);
        }
        
        diag
    }
}

/// コンパイラの診断サービス
pub struct DiagnosticService {
    /// 診断レポーター
    reporter: Arc<Mutex<DiagnosticReporter>>,
    /// 診断エミッタ
    emitter: DiagnosticEmitter,
}

impl DiagnosticService {
    /// 新しい診断サービスを作成
    pub fn new(options: DiagnosticOptions) -> Self {
        let reporter = Arc::new(Mutex::new(DiagnosticReporter::new(options.max_errors)));
        let output: Box<dyn io::Write + Send> = Box::new(io::stdout());
        let emitter = DiagnosticEmitter::new(reporter.clone(), options, output);
        
        Self {
            reporter,
            emitter,
        }
    }
    
    /// 診断を報告
    pub fn report(&mut self, diagnostic: Diagnostic) {
        let mut reporter = self.reporter.lock().unwrap();
        reporter.report(diagnostic);
    }
    
    /// エラーを報告
    pub fn report_error(&mut self, message: &str) {
        let diagnostic = Diagnostic::new(DiagnosticLevel::Error, message.to_string());
        self.report(diagnostic);
    }
    
    /// 警告を報告
    pub fn report_warning(&mut self, message: &str) {
        let diagnostic = Diagnostic::new(DiagnosticLevel::Warning, message.to_string());
        self.report(diagnostic);
    }
    
    /// 情報を報告
    pub fn report_info(&mut self, message: &str) {
        let diagnostic = Diagnostic::new(DiagnosticLevel::Info, message.to_string());
        self.report(diagnostic);
    }
    
    /// 診断を出力
    pub fn emit(&mut self) -> io::Result<()> {
        self.emitter.emit()
    }
    
    /// エラーがあるかチェック
    pub fn has_errors(&self) -> bool {
        let reporter = self.reporter.lock().unwrap();
        reporter.has_errors()
    }
    
    /// 警告があるかチェック
    pub fn has_warnings(&self) -> bool {
        let reporter = self.reporter.lock().unwrap();
        reporter.has_warnings()
    }
    
    /// エラー数を取得
    pub fn error_count(&self) -> usize {
        let reporter = self.reporter.lock().unwrap();
        reporter.error_count()
    }
    
    /// 警告数を取得
    pub fn warning_count(&self) -> usize {
        let reporter = self.reporter.lock().unwrap();
        reporter.warning_count()
    }
    
    /// 診断レポーターを取得
    pub fn reporter(&self) -> Arc<Mutex<DiagnosticReporter>> {
        self.reporter.clone()
    }
}

/// ソースコードの位置情報
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceLocation {
    /// 開始行
    pub start_line: usize,
    /// 開始列
    pub start_column: usize,
    /// 終了行
    pub end_line: usize,
    /// 終了列
    pub end_column: usize,
}

impl SourceLocation {
    /// 新しい位置情報を作成
    pub fn new(start_line: usize, start_column: usize, end_line: usize, end_column: usize) -> Self {
        Self {
            start_line,
            start_column,
            end_line,
            end_column,
        }
    }
    
    /// 単一位置の位置情報を作成
    pub fn point(line: usize, column: usize) -> Self {
        Self::new(line, column, line, column)
    }
}

impl fmt::Display for SourceLocation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.start_line == self.end_line && self.start_column == self.end_column {
            write!(f, "{}:{}", self.start_line, self.start_column)
        } else if self.start_line == self.end_line {
            write!(f, "{}:{}-{}", self.start_line, self.start_column, self.end_column)
        } else {
            write!(
                f,
                "{}:{}-{}:{}",
                self.start_line, self.start_column, self.end_line, self.end_column
            )
        }
    }
} 