//! # 診断機能
//! 
//! コンパイラの診断メッセージを処理するためのユーティリティです。
//! エラー、警告、ノート、ヒントなどの診断メッセージを統一的に扱います。

use std::fmt;
use std::path::PathBuf;
use std::io::{self, Write};
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use crate::frontend::error::{ErrorKind, Result, CompilerError};
use crate::frontend::lexer::Span;

/// 診断レベル
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DiagnosticLevel {
    /// エラー - コンパイルを中断する重大な問題
    Error,
    /// 警告 - 潜在的な問題だが、コンパイルは続行
    Warning,
    /// 注意 - 最適化やベストプラクティスに関する情報
    Note,
    /// ヒント - コードの改善方法を提案
    Hint,
    /// 詳細情報 - 追加的な背景情報
    Info,
    /// 致命的エラー - すぐにコンパイルを中断する
    Fatal,
}

impl fmt::Display for DiagnosticLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DiagnosticLevel::Error => write!(f, "エラー"),
            DiagnosticLevel::Warning => write!(f, "警告"),
            DiagnosticLevel::Note => write!(f, "注意"),
            DiagnosticLevel::Hint => write!(f, "ヒント"),
            DiagnosticLevel::Info => write!(f, "情報"),
            DiagnosticLevel::Fatal => write!(f, "致命的エラー"),
        }
    }
}

/// 診断メッセージ
#[derive(Debug, Clone)]
pub struct Diagnostic {
    /// 診断レベル
    pub level: DiagnosticLevel,
    /// 診断メッセージ
    pub message: String,
    /// 関連するソースコードの場所
    pub location: Option<Location>,
    /// 関連するコード
    pub code: Option<String>,
    /// 追加の注釈
    pub notes: Vec<String>,
    /// 修正候補
    pub suggestions: Vec<Suggestion>,
    /// 関連する診断メッセージ
    pub related: Vec<Diagnostic>,
}

/// ソースコードの位置情報
#[derive(Debug, Clone)]
pub struct Location {
    /// ファイルパス
    pub file: PathBuf,
    /// 行番号（1始まり）
    pub line: usize,
    /// 列番号（1始まり）
    pub column: usize,
    /// スパン情報
    pub span: Option<Span>,
}

/// 修正候補
#[derive(Debug, Clone)]
pub struct Suggestion {
    /// 提案の説明
    pub message: String,
    /// 置換テキスト
    pub replacement: String,
    /// 置換の位置情報
    pub location: Location,
}

/// 診断メッセージの作成と出力を担当するエミッタ
pub struct DiagnosticEmitter {
    /// 出力先
    output: Arc<Mutex<Box<dyn Write + Send>>>,
    /// 色付き出力を使用するかどうか
    colored_output: bool,
    /// 警告をエラーとして扱うかどうか
    warnings_as_errors: bool,
    /// 無視する警告のリスト
    ignored_warnings: Vec<String>,
    /// 発行された診断メッセージの数
    diagnostics_count: HashMap<DiagnosticLevel, usize>,
    /// 最大診断メッセージ数
    max_diagnostics: Option<usize>,
    /// 詳細表示モード
    verbose: bool,
    /// サポート言語
    language: Language,
}

/// サポートする言語
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    /// 英語
    English,
    /// 日本語
    Japanese,
}

impl DiagnosticEmitter {
    /// 新しい診断エミッタを作成
    pub fn new(output: Box<dyn Write + Send>, colored_output: bool) -> Self {
        Self {
            output: Arc::new(Mutex::new(output)),
            colored_output,
            warnings_as_errors: false,
            ignored_warnings: Vec::new(),
            diagnostics_count: HashMap::new(),
            max_diagnostics: None,
            verbose: false,
            language: Language::Japanese,
        }
    }

    /// 標準エラー出力に診断メッセージを出力するエミッタを作成
    pub fn stderr() -> Self {
        Self::new(Box::new(io::stderr()), true)
    }

    /// 警告をエラーとして扱うかどうかを設定
    pub fn set_warnings_as_errors(&mut self, value: bool) -> &mut Self {
        self.warnings_as_errors = value;
        self
    }

    /// 無視する警告を追加
    pub fn add_ignored_warning(&mut self, warning: &str) -> &mut Self {
        self.ignored_warnings.push(warning.to_string());
        self
    }

    /// 最大診断メッセージ数を設定
    pub fn set_max_diagnostics(&mut self, max: Option<usize>) -> &mut Self {
        self.max_diagnostics = max;
        self
    }

    /// 詳細表示モードを設定
    pub fn set_verbose(&mut self, verbose: bool) -> &mut Self {
        self.verbose = verbose;
        self
    }

    /// 言語を設定
    pub fn set_language(&mut self, language: Language) -> &mut Self {
        self.language = language;
        self
    }

    /// 診断メッセージを発行
    pub fn emit(&mut self, diagnostic: Diagnostic) -> Result<()> {
        // 警告をエラーに変換するかどうかチェック
        let level = if self.warnings_as_errors && diagnostic.level == DiagnosticLevel::Warning {
            DiagnosticLevel::Error
        } else {
            diagnostic.level
        };

        // 無視するかどうかチェック
        if level == DiagnosticLevel::Warning && diagnostic.code.as_ref()
            .map_or(false, |code| self.ignored_warnings.contains(code)) {
            return Ok(());
        }

        // 診断メッセージ数をカウント
        *self.diagnostics_count.entry(level).or_insert(0) += 1;

        // 最大診断メッセージ数をチェック
        if let Some(max) = self.max_diagnostics {
            let total = self.diagnostics_count.values().sum::<usize>();
            if total > max {
                return Ok(());
            }
        }

        // 診断メッセージをフォーマット
        let formatted = self.format_diagnostic(&diagnostic);

        // 出力
        let mut output = self.output.lock().unwrap();
        writeln!(output, "{}", formatted)?;

        // 関連する診断メッセージを再帰的に出力
        for related in &diagnostic.related {
            self.emit(related.clone())?;
        }

        Ok(())
    }

    /// 診断メッセージをフォーマット
    fn format_diagnostic(&self, diagnostic: &Diagnostic) -> String {
        let mut result = String::new();

        // レベルと診断コードを表示
        let prefix = match diagnostic.level {
            DiagnosticLevel::Error => "エラー",
            DiagnosticLevel::Warning => "警告",
            DiagnosticLevel::Note => "注意",
            DiagnosticLevel::Hint => "ヒント",
            DiagnosticLevel::Info => "情報",
            DiagnosticLevel::Fatal => "致命的エラー",
        };

        let code_str = diagnostic.code.as_ref()
            .map_or(String::new(), |code| format!("[{}] ", code));

        result.push_str(&format!("{}{}: {}\n", prefix, code_str, diagnostic.message));

        // 位置情報を表示
        if let Some(location) = &diagnostic.location {
            result.push_str(&format!("  --> {}:{}:{}\n", 
                location.file.display(), location.line, location.column));

            // スパン情報があれば表示
            if let Some(span) = &location.span {
                if let Some(source) = span.get_source_line() {
                    result.push_str(&format!("{}\n", source));
                    
                    // 位置を示す矢印
                    let arrow_pos = location.column.saturating_sub(1);
                    let arrow = " ".repeat(arrow_pos) + "^";
                    result.push_str(&format!("{}\n", arrow));
                }
            }
        }

        // 注釈を表示
        for note in &diagnostic.notes {
            result.push_str(&format!("注意: {}\n", note));
        }

        // 修正候補を表示
        if !diagnostic.suggestions.is_empty() {
            result.push_str("修正候補:\n");
            for suggestion in &diagnostic.suggestions {
                result.push_str(&format!("  - {}: `{}`\n", 
                    suggestion.message, suggestion.replacement));
            }
        }

        result
    }

    /// エラーを診断メッセージとして発行
    pub fn emit_error(&mut self, error: &CompilerError) -> Result<()> {
        let diagnostic = Diagnostic {
            level: DiagnosticLevel::Error,
            message: error.message.clone(),
            location: error.span.as_ref().map(|span| Location {
                file: span.file_path.clone(),
                line: span.start_line,
                column: span.start_column,
                span: Some(span.clone()),
            }),
            code: Some(format!("E{:04}", error.kind.as_u16())),
            notes: Vec::new(),
            suggestions: Vec::new(),
            related: Vec::new(),
        };

        self.emit(diagnostic)
    }

    /// 警告を発行
    pub fn emit_warning(&mut self, message: &str, location: Option<Location>, code: Option<&str>) -> Result<()> {
        let diagnostic = Diagnostic {
            level: DiagnosticLevel::Warning,
            message: message.to_string(),
            location,
            code: code.map(String::from),
            notes: Vec::new(),
            suggestions: Vec::new(),
            related: Vec::new(),
        };

        self.emit(diagnostic)
    }

    /// 注意を発行
    pub fn emit_note(&mut self, message: &str) -> Result<()> {
        let diagnostic = Diagnostic {
            level: DiagnosticLevel::Note,
            message: message.to_string(),
            location: None,
            code: None,
            notes: Vec::new(),
            suggestions: Vec::new(),
            related: Vec::new(),
        };

        self.emit(diagnostic)
    }

    /// ヒントを発行
    pub fn emit_hint(&mut self, message: &str) -> Result<()> {
        let diagnostic = Diagnostic {
            level: DiagnosticLevel::Hint,
            message: message.to_string(),
            location: None,
            code: None,
            notes: Vec::new(),
            suggestions: Vec::new(),
            related: Vec::new(),
        };

        self.emit(diagnostic)
    }

    /// 致命的エラーを発行し、プログラムを終了
    pub fn emit_fatal(&mut self, message: &str) -> ! {
        let diagnostic = Diagnostic {
            level: DiagnosticLevel::Fatal,
            message: message.to_string(),
            location: None,
            code: None,
            notes: Vec::new(),
            suggestions: Vec::new(),
            related: Vec::new(),
        };

        let _ = self.emit(diagnostic);
        std::process::exit(1);
    }

    /// 発行された診断メッセージ数を取得
    pub fn get_diagnostics_count(&self) -> &HashMap<DiagnosticLevel, usize> {
        &self.diagnostics_count
    }

    /// エラーが発生したかどうかをチェック
    pub fn has_errors(&self) -> bool {
        self.diagnostics_count.get(&DiagnosticLevel::Error).copied().unwrap_or(0) > 0 ||
        self.diagnostics_count.get(&DiagnosticLevel::Fatal).copied().unwrap_or(0) > 0
    }

    /// 警告が発生したかどうかをチェック
    pub fn has_warnings(&self) -> bool {
        self.diagnostics_count.get(&DiagnosticLevel::Warning).copied().unwrap_or(0) > 0
    }

    /// すべての診断メッセージをクリア
    pub fn clear(&mut self) {
        self.diagnostics_count.clear();
    }
}

/// スパンから位置情報を作成するヘルパー関数
pub fn location_from_span(span: &Span) -> Location {
    Location {
        file: span.file_path.clone(),
        line: span.start_line,
        column: span.start_column,
        span: Some(span.clone()),
    }
}

/// 診断メッセージビルダー
pub struct DiagnosticBuilder {
    level: DiagnosticLevel,
    message: String,
    location: Option<Location>,
    code: Option<String>,
    notes: Vec<String>,
    suggestions: Vec<Suggestion>,
    related: Vec<Diagnostic>,
}

impl DiagnosticBuilder {
    /// 新しい診断メッセージビルダーを作成
    pub fn new(level: DiagnosticLevel, message: &str) -> Self {
        Self {
            level,
            message: message.to_string(),
            location: None,
            code: None,
            notes: Vec::new(),
            suggestions: Vec::new(),
            related: Vec::new(),
        }
    }

    /// 位置情報を設定
    pub fn with_location(mut self, location: Location) -> Self {
        self.location = Some(location);
        self
    }

    /// スパンから位置情報を設定
    pub fn with_span(mut self, span: &Span) -> Self {
        self.location = Some(location_from_span(span));
        self
    }

    /// 診断コードを設定
    pub fn with_code(mut self, code: &str) -> Self {
        self.code = Some(code.to_string());
        self
    }

    /// 注釈を追加
    pub fn add_note(mut self, note: &str) -> Self {
        self.notes.push(note.to_string());
        self
    }

    /// 修正候補を追加
    pub fn add_suggestion(mut self, message: &str, replacement: &str, location: Location) -> Self {
        self.suggestions.push(Suggestion {
            message: message.to_string(),
            replacement: replacement.to_string(),
            location,
        });
        self
    }

    /// 関連する診断メッセージを追加
    pub fn add_related(mut self, related: Diagnostic) -> Self {
        self.related.push(related);
        self
    }

    /// 診断メッセージを構築
    pub fn build(self) -> Diagnostic {
        Diagnostic {
            level: self.level,
            message: self.message,
            location: self.location,
            code: self.code,
            notes: self.notes,
            suggestions: self.suggestions,
            related: self.related,
        }
    }
} 