//! 診断情報モジュール
//! 
//! コンパイラエラーや警告を管理するためのモジュールです。

use std::fmt;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;

use crate::driver::compiler::CompilerError;
use crate::driver::compiler::ErrorKind;

/// 診断レベル
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticLevel {
    /// エラー
    Error,
    /// 警告
    Warning,
    /// 情報
    Info,
    /// ヒント
    Hint,
    /// 注意
    Note,
}

impl fmt::Display for DiagnosticLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DiagnosticLevel::Error => write!(f, "エラー"),
            DiagnosticLevel::Warning => write!(f, "警告"),
            DiagnosticLevel::Info => write!(f, "情報"),
            DiagnosticLevel::Hint => write!(f, "ヒント"),
            DiagnosticLevel::Note => write!(f, "注意"),
        }
    }
}

/// 診断情報
#[derive(Debug, Clone)]
pub struct Diagnostic {
    /// レベル
    pub level: DiagnosticLevel,
    /// メッセージ
    pub message: String,
    /// コード
    pub code: Option<String>,
    /// ファイルパス
    pub file_path: Option<PathBuf>,
    /// 行番号
    pub line: Option<usize>,
    /// 列番号
    pub column: Option<usize>,
    /// 関連する診断
    pub related: Vec<Diagnostic>,
    /// 修正候補
    pub suggestions: Vec<String>,
}

impl Diagnostic {
    /// 新しい診断情報を作成
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
    
    /// 診断情報をコンパイルエラーに変換
    pub fn to_compiler_error(&self) -> CompilerError {
        let kind = match self.level {
            DiagnosticLevel::Error => ErrorKind::SyntaxError,
            DiagnosticLevel::Warning => ErrorKind::Warning,
            _ => ErrorKind::Info,
        };
        
        CompilerError::new(
            kind, 
            self.message.clone(),
            self.file_path.clone()
        )
    }
}

/// 診断レポーター
pub struct DiagnosticReporter {
    /// 診断リスト
    diagnostics: Arc<Mutex<Vec<Diagnostic>>>,
    /// エラー数
    error_count: Arc<Mutex<usize>>,
    /// 警告数
    warning_count: Arc<Mutex<usize>>,
    /// 最大エラー数
    max_errors: usize,
    /// 無視するエラーコード
    ignored_codes: Vec<String>,
    /// ファイルごとの診断情報
    file_diagnostics: HashMap<PathBuf, Vec<Diagnostic>>,
}

impl DiagnosticReporter {
    /// 新しい診断レポーターを作成
    pub fn new(max_errors: usize) -> Self {
        Self {
            diagnostics: Arc::new(Mutex::new(Vec::new())),
            error_count: Arc::new(Mutex::new(0)),
            warning_count: Arc::new(Mutex::new(0)),
            max_errors,
            ignored_codes: Vec::new(),
            file_diagnostics: HashMap::new(),
        }
    }
    
    /// 診断情報を報告
    pub fn report(&mut self, diagnostic: Diagnostic) {
        let mut diagnostics = self.diagnostics.lock().unwrap();
        
        match diagnostic.level {
            DiagnosticLevel::Error => {
                let mut error_count = self.error_count.lock().unwrap();
                *error_count += 1;
            },
            DiagnosticLevel::Warning => {
                let mut warning_count = self.warning_count.lock().unwrap();
                *warning_count += 1;
            },
            _ => {}
        }
        
        // ファイル別に診断情報を記録
        if let Some(path) = &diagnostic.file_path {
            self.file_diagnostics
                .entry(path.clone())
                .or_insert_with(Vec::new)
                .push(diagnostic.clone());
        }
        
        diagnostics.push(diagnostic);
    }
    
    /// エラーがあるかチェック
    pub fn has_errors(&self) -> bool {
        let error_count = self.error_count.lock().unwrap();
        *error_count > 0
    }
    
    /// 警告があるかチェック
    pub fn has_warnings(&self) -> bool {
        let warning_count = self.warning_count.lock().unwrap();
        *warning_count > 0
    }
    
    /// 診断情報を取得
    pub fn get_diagnostics(&self) -> Vec<CompilerError> {
        let diagnostics = self.diagnostics.lock().unwrap();
        diagnostics.iter()
            .map(|d| d.to_compiler_error())
            .collect()
    }
    
    /// エラー数を取得
    pub fn error_count(&self) -> usize {
        let error_count = self.error_count.lock().unwrap();
        *error_count
    }
    
    /// 警告数を取得
    pub fn warning_count(&self) -> usize {
        let warning_count = self.warning_count.lock().unwrap();
        *warning_count
    }
    
    /// 最大エラー数に達したかチェック
    pub fn max_errors_reached(&self) -> bool {
        let error_count = self.error_count.lock().unwrap();
        *error_count >= self.max_errors
    }
    
    /// 特定のファイルの診断情報を取得
    pub fn get_file_diagnostics(&self, path: &PathBuf) -> Vec<Diagnostic> {
        self.file_diagnostics.get(path)
            .map(|v| v.clone())
            .unwrap_or_else(Vec::new)
    }
}

/// 診断フォーマッタ
pub struct DiagnosticFormatter {
    /// 色付き出力を使用するか
    use_colors: bool,
    /// 詳細表示
    verbose: bool,
}

impl DiagnosticFormatter {
    /// 新しい診断フォーマッタを作成
    pub fn new(use_colors: bool, verbose: bool) -> Self {
        Self {
            use_colors,
            verbose,
        }
    }
    
    /// 診断情報をフォーマット
    pub fn format(&self, diagnostic: &Diagnostic) -> String {
        let mut result = String::new();
        
        let level_str = match diagnostic.level {
            DiagnosticLevel::Error => {
                if self.use_colors {
                    "\x1b[1;31mエラー\x1b[0m"
                } else {
                    "エラー"
                }
            },
            DiagnosticLevel::Warning => {
                if self.use_colors {
                    "\x1b[1;33m警告\x1b[0m"
                } else {
                    "警告"
                }
            },
            DiagnosticLevel::Info => {
                if self.use_colors {
                    "\x1b[1;34m情報\x1b[0m"
                } else {
                    "情報"
                }
            },
            DiagnosticLevel::Hint => {
                if self.use_colors {
                    "\x1b[1;36mヒント\x1b[0m"
                } else {
                    "ヒント"
                }
            },
            DiagnosticLevel::Note => {
                if self.use_colors {
                    "\x1b[1;37m注意\x1b[0m"
                } else {
                    "注意"
                }
            },
        };
        
        result.push_str(&format!("{}: {}", level_str, diagnostic.message));
        
        if let Some(code) = &diagnostic.code {
            result.push_str(&format!(" [{}]", code));
        }
        
        if let (Some(path), Some(line), Some(column)) = (&diagnostic.file_path, diagnostic.line, diagnostic.column) {
            result.push_str(&format!("\n  --> {}:{}:{}", path.display(), line, column));
        }
        
        if !diagnostic.suggestions.is_empty() && self.verbose {
            result.push_str("\n\n修正候補:");
            for (i, suggestion) in diagnostic.suggestions.iter().enumerate() {
                result.push_str(&format!("\n  {}: {}", i + 1, suggestion));
            }
        }
        
        if !diagnostic.related.is_empty() && self.verbose {
            result.push_str("\n\n関連する診断:");
            for related in &diagnostic.related {
                let related_str = self.format(related);
                let indented = related_str.lines()
                    .map(|line| format!("  {}", line))
                    .collect::<Vec<_>>()
                    .join("\n");
                result.push_str(&format!("\n{}", indented));
            }
        }
        
        result
    }
} 