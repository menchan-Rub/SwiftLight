//! # エラー処理モジュール
//! 
//! SwiftLightコンパイラのエラー処理を担当するモジュールです。
//! ユーザーフレンドリーなエラーメッセージと正確な位置情報を提供し、
//! 可能な場合は修正案も提示します。
//! このモジュールは、SwiftLight言語の極限のメモリ安全性と開発体験向上の
//! 目標に沿って設計されています。

use std::fmt;
use std::path::{Path, PathBuf};
use std::error::Error as StdError;
use std::io;
use std::sync::Arc;
use std::collections::HashMap;
use colored::*;
use unicode_width::UnicodeWidthStr;

/// エラーの種類
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorKind {
    /// 字句解析エラー
    Lexical,
    /// 構文解析エラー
    Syntax,
    /// 意味解析エラー
    Semantic,
    /// 型チェックエラー
    Type,
    /// 所有権チェックエラー
    Ownership,
    /// 借用チェックエラー
    Borrow,
    /// ライフタイムエラー
    Lifetime,
    /// パターンマッチングエラー
    PatternMatching,
    /// 未定義シンボルエラー
    UndefinedSymbol,
    /// 重複シンボルエラー
    DuplicateSymbol,
    /// 依存型エラー
    DependentType,
    /// コード生成エラー
    CodeGen,
    /// リンクエラー
    LinkError,
    /// I/Oエラー
    IO,
    /// コンパイル時計算制限超過
    CompileTimeComputationLimitExceeded,
    /// メタプログラミングエラー
    MetaProgramming,
    /// 内部エラー
    Internal,
    /// 形式検証エラー
    FormalVerification,
    /// セキュリティチェックエラー
    SecurityCheck,
    /// 並行性エラー
    Concurrency,
    /// メモリ安全性エラー
    MemorySafety,
    /// リソース管理エラー
    ResourceManagement,
    /// 最適化エラー
    Optimization,
    /// モジュール解決エラー
    ModuleResolution,
    /// 循環依存エラー
    CircularDependency,
    /// 非互換性エラー
    Incompatibility,
    /// 実行時エラー（コンパイル時評価中）
    RuntimeDuringCompilation,
    /// 型推論エラー
    TypeInference,
    /// 型境界エラー
    TypeBound,
    /// トレイト実装エラー
    TraitImplementation,
    /// マクロ展開エラー
    MacroExpansion,
    /// 属性エラー
    Attribute,
    /// 可変性エラー
    Mutability,
    /// 定数評価エラー
    ConstEvaluation,
    /// 名前解決エラー
    NameResolution,
    /// 可視性エラー
    Visibility,
    /// 型パラメータエラー
    TypeParameter,
    /// 型推論制約エラー
    TypeConstraint,
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            ErrorKind::Lexical => "字句解析エラー",
            ErrorKind::Syntax => "構文解析エラー",
            ErrorKind::Semantic => "意味解析エラー",
            ErrorKind::Type => "型エラー",
            ErrorKind::Ownership => "所有権エラー",
            ErrorKind::Borrow => "借用エラー",
            ErrorKind::Lifetime => "ライフタイムエラー",
            ErrorKind::PatternMatching => "パターンマッチングエラー",
            ErrorKind::UndefinedSymbol => "未定義シンボルエラー",
            ErrorKind::DuplicateSymbol => "重複シンボルエラー",
            ErrorKind::DependentType => "依存型エラー",
            ErrorKind::CodeGen => "コード生成エラー",
            ErrorKind::LinkError => "リンクエラー",
            ErrorKind::IO => "I/Oエラー",
            ErrorKind::CompileTimeComputationLimitExceeded => "コンパイル時計算制限超過",
            ErrorKind::MetaProgramming => "メタプログラミングエラー",
            ErrorKind::Internal => "内部エラー",
            ErrorKind::FormalVerification => "形式検証エラー",
            ErrorKind::SecurityCheck => "セキュリティチェックエラー",
            ErrorKind::Concurrency => "並行性エラー",
            ErrorKind::MemorySafety => "メモリ安全性エラー",
            ErrorKind::ResourceManagement => "リソース管理エラー",
            ErrorKind::Optimization => "最適化エラー",
            ErrorKind::ModuleResolution => "モジュール解決エラー",
            ErrorKind::CircularDependency => "循環依存エラー",
            ErrorKind::Incompatibility => "非互換性エラー",
            ErrorKind::RuntimeDuringCompilation => "コンパイル時評価中の実行時エラー",
            ErrorKind::TypeInference => "型推論エラー",
            ErrorKind::TypeBound => "型境界エラー",
            ErrorKind::TraitImplementation => "トレイト実装エラー",
            ErrorKind::MacroExpansion => "マクロ展開エラー",
            ErrorKind::Attribute => "属性エラー",
            ErrorKind::Mutability => "可変性エラー",
            ErrorKind::ConstEvaluation => "定数評価エラー",
            ErrorKind::NameResolution => "名前解決エラー",
            ErrorKind::Visibility => "可視性エラー",
            ErrorKind::TypeParameter => "型パラメータエラー",
            ErrorKind::TypeConstraint => "型推論制約エラー",
        };
        write!(f, "{}", message)
    }
}

/// ソースコード内の位置情報
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SourceLocation {
    /// 行番号（1から始まる）
    pub line: usize,
    /// 列番号（1から始まる）
    pub column: usize,
    /// 位置のバイトオフセット
    pub offset: usize,
    /// 範囲の長さ（バイト単位）
    pub length: usize,
    /// 終了行（範囲が複数行にまたがる場合）
    pub end_line: Option<usize>,
    /// 終了列（範囲が複数行にまたがる場合）
    pub end_column: Option<usize>,
}

impl SourceLocation {
    /// 新しい位置情報を作成
    pub fn new(line: usize, column: usize, offset: usize, length: usize) -> Self {
        Self {
            line,
            column,
            offset,
            length,
            end_line: None,
            end_column: None,
        }
    }

    /// 範囲を持つ位置情報を作成
    pub fn with_range(line: usize, column: usize, offset: usize, length: usize, 
                     end_line: usize, end_column: usize) -> Self {
        Self {
            line,
            column,
            offset,
            length,
            end_line: Some(end_line),
            end_column: Some(end_column),
        }
    }

    /// 位置情報が有効かどうかを確認
    pub fn is_valid(&self) -> bool {
        self.line > 0 && self.column > 0
    }
    
    /// 位置情報が複数行にまたがるかどうかを確認
    pub fn is_multiline(&self) -> bool {
        self.end_line.is_some() && self.end_line != Some(self.line)
    }
    
    /// 位置情報の範囲を文字列で表現
    pub fn range_string(&self) -> String {
        if let (Some(end_line), Some(end_column)) = (self.end_line, self.end_column) {
            if end_line == self.line {
                format!("{}:{}-{}", self.line, self.column, end_column)
            } else {
                format!("{}:{}-{}:{}", self.line, self.column, end_line, end_column)
            }
        } else {
            format!("{}:{}", self.line, self.column)
        }
    }
    
    /// 別の位置情報と結合して範囲を作成
    pub fn merge_with(&self, other: &SourceLocation) -> Self {
        let (start, end) = if self.offset <= other.offset {
            (self, other)
        } else {
            (other, self)
        };
        
        let end_offset = end.offset + end.length;
        let total_length = end_offset - start.offset;
        
        Self {
            line: start.line,
            column: start.column,
            offset: start.offset,
            length: total_length,
            end_line: Some(end.end_line.unwrap_or(end.line)),
            end_column: Some(end.end_column.unwrap_or(end.column + end.length)),
        }
    }
}

impl fmt::Display for SourceLocation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let (Some(end_line), Some(end_column)) = (self.end_line, self.end_column) {
            if end_line == self.line {
                write!(f, "{}:{}-{}", self.line, self.column, end_column)
            } else {
                write!(f, "{}:{}-{}:{}", self.line, self.column, end_line, end_column)
            }
        } else {
            write!(f, "{}:{}", self.line, self.column)
        }
    }
}

/// エラー修正案
#[derive(Debug, Clone)]
pub struct ErrorFix {
    /// 修正の説明
    pub description: String,
    /// 修正するテキスト
    pub replacement: String,
    /// 修正を適用する位置
    pub location: SourceLocation,
    /// 修正の種類
    pub fix_type: FixType,
    /// 修正の信頼度（0.0〜1.0）
    pub confidence: f32,
}

/// 修正の種類
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FixType {
    /// 単純な置換
    Replace,
    /// 挿入
    Insert,
    /// 削除
    Delete,
    /// リファクタリング（複数ファイルに影響する可能性あり）
    Refactor,
    /// 型の修正
    TypeFix,
    /// インポートの追加
    AddImport,
    /// 構造的な修正
    Structural,
}

/// コンパイラエラー
#[derive(Debug, Clone)]
pub struct CompilerError {
    /// エラーの種類
    pub kind: ErrorKind,
    /// エラーメッセージ
    pub message: String,
    /// エラーの位置
    pub location: Option<SourceLocation>,
    /// ソースファイルのパス
    pub file_path: Option<PathBuf>,
    /// 追加の注釈
    pub notes: Vec<String>,
    /// エラー修正案
    pub fixes: Vec<ErrorFix>,
    /// 関連する位置情報
    pub related_locations: Vec<(SourceLocation, String)>,
    /// エラーの重大度
    pub severity: ErrorSeverity,
    /// エラーの原因となったエラー
    pub cause: Option<Box<CompilerError>>,
    /// エラーに関連するソースコード
    pub source_snippet: Option<String>,
    /// エラーの詳細なドキュメントへのリンク
    pub documentation_link: Option<String>,
    /// エラーのカテゴリ（複数可）
    pub categories: Vec<ErrorCategory>,
    /// エラーの一意識別子
    pub id: String,
}

/// Errorは単にCompilerErrorの型エイリアス
pub type Error = CompilerError;

/// エラーカテゴリ
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorCategory {
    /// 構文関連
    Syntax,
    /// 型システム関連
    TypeSystem,
    /// メモリ安全性関連
    MemorySafety,
    /// 並行処理関連
    Concurrency,
    /// パフォーマンス関連
    Performance,
    /// セキュリティ関連
    Security,
    /// 依存型関連
    DependentTypes,
    /// メタプログラミング関連
    MetaProgramming,
    /// コンパイル時計算関連
    CompileTimeComputation,
    /// リソース管理関連
    ResourceManagement,
    /// モジュールシステム関連
    ModuleSystem,
    /// 形式検証関連
    FormalVerification,
}

impl CompilerError {
    /// 新しいエラーを作成
    pub fn new(kind: ErrorKind, message: String, location: Option<SourceLocation>) -> Self {
        let id = format!("E{:04}", kind as usize);
        Self {
            kind,
            message,
            location,
            file_path: None,
            notes: Vec::new(),
            fixes: Vec::new(),
            related_locations: Vec::new(),
            severity: ErrorSeverity::Error,
            cause: None,
            source_snippet: None,
            documentation_link: None,
            categories: Vec::new(),
            id,
        }
    }

    /// ファイルパスを設定
    pub fn with_file_path<P: AsRef<Path>>(mut self, path: P) -> Self {
        self.file_path = Some(path.as_ref().to_path_buf());
        self
    }

    /// 注釈を追加
    pub fn add_note<S: Into<String>>(&mut self, note: S) -> &mut Self {
        self.notes.push(note.into());
        self
    }

    /// 修正案を追加
    pub fn add_fix(&mut self, description: String, replacement: String, location: SourceLocation, 
                  fix_type: FixType, confidence: f32) -> &mut Self {
        self.fixes.push(ErrorFix {
            description,
            replacement,
            location,
            fix_type,
            confidence,
        });
        self
    }

    /// 簡易的な修正案を追加（デフォルトは置換）
    pub fn add_simple_fix(&mut self, description: String, replacement: String, location: SourceLocation) -> &mut Self {
        self.add_fix(description, replacement, location, FixType::Replace, 0.8)
    }

    /// 関連する位置情報を追加
    pub fn add_related_location(&mut self, location: SourceLocation, description: String) -> &mut Self {
        self.related_locations.push((location, description));
        self
    }

    /// エラーコードを取得
    pub fn code(&self) -> &str {
        &self.id
    }

    /// エラーの重大度を設定
    pub fn with_severity(mut self, severity: ErrorSeverity) -> Self {
        self.severity = severity;
        self
    }

    /// エラーの重大度を取得
    pub fn severity(&self) -> ErrorSeverity {
        self.severity
    }

    /// エラーの原因を設定
    pub fn with_cause(mut self, cause: CompilerError) -> Self {
        self.cause = Some(Box::new(cause));
        self
    }

    /// ソースコードスニペットを設定
    pub fn with_source_snippet<S: Into<String>>(mut self, snippet: S) -> Self {
        self.source_snippet = Some(snippet.into());
        self
    }

    /// ドキュメントリンクを設定
    pub fn with_documentation_link<S: Into<String>>(mut self, link: S) -> Self {
        self.documentation_link = Some(link.into());
        self
    }

    /// エラーカテゴリを追加
    pub fn add_category(&mut self, category: ErrorCategory) -> &mut Self {
        self.categories.push(category);
        self
    }

    /// エラーカテゴリを設定
    pub fn with_categories(mut self, categories: Vec<ErrorCategory>) -> Self {
        self.categories = categories;
        self
    }

    /// エラーの詳細な説明を取得
    pub fn detailed_explanation(&self) -> String {
        let mut explanation = String::new();
        
        // エラーの種類に基づいた詳細な説明
        match self.kind {
            ErrorKind::Type => {
                explanation.push_str("型エラーは、式や変数の型が期待される型と一致しない場合に発生します。\n");
                explanation.push_str("型の互換性を確認し、必要に応じて明示的な型変換を行ってください。\n");
            },
            ErrorKind::Ownership => {
                explanation.push_str("所有権エラーは、値の所有権が移動された後にその値にアクセスしようとした場合に発生します。\n");
                explanation.push_str("値をクローンするか、参照を使用するか、所有権を返す関数を使用することを検討してください。\n");
            },
            ErrorKind::Borrow => {
                explanation.push_str("借用エラーは、値の可変借用と不変借用の規則に違反した場合に発生します。\n");
                explanation.push_str("同時に存在できるのは、複数の不変借用か、単一の可変借用のいずれかです。\n");
            },
            ErrorKind::DependentType => {
                explanation.push_str("依存型エラーは、型が値に依存する場合に発生する可能性があります。\n");
                explanation.push_str("依存型の制約を満たすように値を調整するか、型の定義を見直してください。\n");
            },
            _ => {
                // その他のエラー種類に対する説明
                explanation.push_str(&format!("{}に関する詳細情報については、ドキュメントを参照してください。\n", self.kind));
            }
        }
        
        // ドキュメントリンクがあれば追加
        if let Some(link) = &self.documentation_link {
            explanation.push_str(&format!("詳細なドキュメント: {}\n", link));
        }
        
        explanation
    }

    /// エラーメッセージを色付きで整形
    pub fn format_colored(&self) -> String {
        let mut result = String::new();
        
        // エラーヘッダー
        let severity_str = match self.severity {
            ErrorSeverity::Fatal => "致命的エラー".red().bold(),
            ErrorSeverity::Error => "エラー".red().bold(),
            ErrorSeverity::Warning => "警告".yellow().bold(),
            ErrorSeverity::Info => "情報".blue().bold(),
            ErrorSeverity::Hint => "ヒント".green().bold(),
        };
        
        let location_str = if let Some(loc) = &self.location {
            if let Some(path) = &self.file_path {
                format!("{}:{}", path.display().to_string().cyan(), loc.to_string().cyan().bold())
            } else {
                loc.to_string().cyan().bold().to_string()
            }
        } else if let Some(path) = &self.file_path {
            path.display().to_string().cyan()
        } else {
            String::new()
        };
        
        result.push_str(&format!("{}: [{}] {}\n", 
            severity_str,
            self.code().yellow(),
            self.message.white().bold()
        ));
        
        if !location_str.is_empty() {
            result.push_str(&format!("  --> {}\n", location_str));
        }
        
        // ソースコードスニペット
        if let Some(snippet) = &self.source_snippet {
            result.push_str("\n");
            
            let lines: Vec<&str> = snippet.lines().collect();
            let start_line = self.location.as_ref().map_or(1, |loc| loc.line);
            let line_num_width = start_line.to_string().len() + lines.len().to_string().len();
            
            for (i, line) in lines.iter().enumerate() {
                let line_num = start_line + i;
                result.push_str(&format!("{:>width$} | {}\n", 
                    line_num.to_string().blue(), 
                    line,
                    width = line_num_width
                ));
                
                // エラー位置を示す矢印
                if let Some(loc) = &self.location {
                    if loc.line == line_num {
                        let mut arrow = " ".repeat(line_num_width);
                        arrow.push_str(" | ");
                        
                        // 列位置に合わせてスペースを追加
                        let prefix_width = UnicodeWidthStr::width(&line[..loc.column.saturating_sub(1)]);
                        arrow.push_str(&" ".repeat(prefix_width));
                        
                        // エラー範囲の長さに合わせて矢印を表示
                        let error_length = if let Some(end_col) = loc.end_column {
                            if loc.end_line == Some(line_num) {
                                end_col - loc.column
                            } else {
                                line.len() - loc.column + 1
                            }
                        } else {
                            loc.length.min(line.len() - loc.column + 1)
                        };
                        
                        arrow.push_str(&"^".repeat(error_length.max(1)).red().bold());
                        result.push_str(&format!("{}\n", arrow));
                    }
                }
            }
            result.push_str("\n");
        }
        
        // 注釈
        for note in &self.notes {
            result.push_str(&format!("注: {}\n", note.blue()));
        }
        
        // 関連する位置情報
        for (loc, desc) in &self.related_locations {
            result.push_str(&format!("  --> {}: {}\n", 
                loc.to_string().cyan(), 
                desc.white()
            ));
        }
        
        // 修正案
        if !self.fixes.is_empty() {
            result.push_str(&format!("\n{}\n", "修正案:".green().bold()));
            for (i, fix) in self.fixes.iter().enumerate() {
                let confidence_str = match fix.confidence {
                    c if c >= 0.9 => "高確度".green(),
                    c if c >= 0.6 => "中確度".yellow(),
                    _ => "低確度".red(),
                };
                
                result.push_str(&format!("  {}: {} [{}] at {}\n", 
                    (i + 1).to_string().green().bold(),
                    fix.description.white(),
                    confidence_str,
                    fix.location.to_string().cyan()
                ));
                
                // 修正内容のプレビュー
                result.push_str(&format!("      {} {}\n", 
                    "→".green().bold(),
                    fix.replacement.bright_white().italic()
                ));
            }
        }
        
        // 詳細な説明へのリンク
        if self.documentation_link.is_some() {
            result.push_str(&format!("\n詳細なドキュメント: {}\n", 
                self.documentation_link.as_ref().unwrap().underline().cyan()
            ));
        }
        
        result
    }

    /// エラーを適用可能な修正案で自動修正
    pub fn auto_fix(&self, source_code: &str) -> Option<String> {
        if self.fixes.is_empty() {
            return None;
        }
        
        // 信頼度が最も高い修正案を選択
        let best_fix = self.fixes.iter()
            .max_by(|a, b| a.confidence.partial_cmp(&b.confidence).unwrap())?;
        
        // 信頼度が低すぎる場合は修正しない
        if best_fix.confidence < 0.7 {
            return None;
        }
        
        let mut result = source_code.to_string();
        
        match best_fix.fix_type {
            FixType::Replace => {
                if let Some(loc) = &self.location {
                    let start = loc.offset;
                    let end = start + loc.length;
                    if start <= end && end <= result.len() {
                        result.replace_range(start..end, &best_fix.replacement);
                        return Some(result);
                    }
                }
            },
            FixType::Insert => {
                if let Some(loc) = &self.location {
                    let pos = loc.offset;
                    if pos <= result.len() {
                        result.insert_str(pos, &best_fix.replacement);
                        return Some(result);
                    }
                }
            },
            FixType::Delete => {
                if let Some(loc) = &self.location {
                    let start = loc.offset;
                    let end = start + loc.length;
                    if start <= end && end <= result.len() {
                        result.replace_range(start..end, "");
                        return Some(result);
                    }
                }
            },
            _ => {
                // 複雑な修正は自動適用しない
                return None;
            }
        }
        
        None
    }
}

impl fmt::Display for CompilerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let location_str = if let Some(loc) = &self.location {
            if let Some(path) = &self.file_path {
                format!("{}:{}:{}: ", path.display(), loc.line, loc.column)
            } else {
                format!("{}:{}: ", loc.line, loc.column)
            }
        } else if let Some(path) = &self.file_path {
            format!("{}: ", path.display())
        } else {
            String::new()
        };

        writeln!(f, "{}[{}] {}: {}", location_str, self.code(), self.severity(), self.message)?;

        // 注釈を出力
        for note in &self.notes {
            writeln!(f, "注: {}", note)?;
        }

        // 関連する位置情報を出力
        for (loc, desc) in &self.related_locations {
            writeln!(f, "  --> {}:{}: {}", loc.line, loc.column, desc)?;
        }

        // 修正案を出力
        if !self.fixes.is_empty() {
            writeln!(f, "修正案:")?;
            for (i, fix) in self.fixes.iter().enumerate() {
                writeln!(f, "  {}: {} at {}:{}", i + 1, fix.description, fix.location.line, fix.location.column)?;
            }
        }

        // ドキュメントリンクがあれば出力
        if let Some(link) = &self.documentation_link {
            writeln!(f, "詳細なドキュメント: {}", link)?;
        }

        Ok(())
    }
}

impl StdError for CompilerError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        self.cause.as_ref().map(|e| e.as_ref() as &(dyn StdError + 'static))
    }
}

/// エラーの重大度
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ErrorSeverity {
    /// 致命的エラー（コンパイルを即座に中止）
    Fatal,
    /// エラー（コンパイルは続行するが、生成されたコードは実行不可）
    Error,
    /// 警告（コンパイルは続行し、生成されたコードは実行可能）
    Warning,
    /// 情報（単なる情報提供）
    Info,
    /// ヒント（改善提案）
    Hint,
}

impl fmt::Display for ErrorSeverity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorSeverity::Fatal => write!(f, "致命的エラー"),
            ErrorSeverity::Error => write!(f, "エラー"),
            ErrorSeverity::Warning => write!(f, "警告"),
            ErrorSeverity::Info => write!(f, "情報"),
            ErrorSeverity::Hint => write!(f, "ヒント"),
        }
    }
}

/// Result型のエイリアス
pub type Result<T> = std::result::Result<T, CompilerError>;

/// IOエラーをCompilerErrorに変換
impl From<io::Error> for CompilerError {
    fn from(error: io::Error) -> Self {
        Self::new(
            ErrorKind::IO,
            format!("I/Oエラー: {}", error),
            None,
        )
    }
}

/// エラーマネージャー
#[derive(Debug, Default)]
pub struct ErrorManager {
    /// 発生したエラーのリスト
    errors: Vec<CompilerError>,
    /// 発生した警告のリスト
    warnings: Vec<CompilerError>,
    /// 警告をエラーとして扱うかどうか
    warnings_as_errors: bool,
}

impl ErrorManager {
    /// 新しいエラーマネージャーを作成
    pub fn new(warnings_as_errors: bool) -> Self {
        Self {
            errors: Vec::new(),
            warnings: Vec::new(),
            warnings_as_errors,
        }
    }

    /// エラーを追加
    pub fn add_error(&mut self, error: CompilerError) {
        self.errors.push(error);
    }

    /// 警告を追加
    pub fn add_warning(&mut self, warning: CompilerError) {
        if self.warnings_as_errors {
            self.errors.push(warning);
        } else {
            self.warnings.push(warning);
        }
    }

    /// エラーがあるかどうかを確認
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// 警告があるかどうかを確認
    pub fn has_warnings(&self) -> bool {
        !self.warnings.is_empty()
    }

    /// エラーと警告の数を取得
    pub fn error_count(&self) -> (usize, usize) {
        (self.errors.len(), self.warnings.len())
    }

    /// すべてのエラーと警告を取得
    pub fn get_all(&self) -> (&[CompilerError], &[CompilerError]) {
        (&self.errors, &self.warnings)
    }

    /// すべてのエラーと警告を出力
    pub fn report_all(&self, writer: &mut dyn std::io::Write) -> io::Result<()> {
        for error in &self.errors {
            writeln!(writer, "{}", error)?;
        }

        for warning in &self.warnings {
            writeln!(writer, "{}", warning)?;
        }

        let (error_count, warning_count) = self.error_count();
        if error_count > 0 || warning_count > 0 {
            writeln!(
                writer,
                "エラー: {}個、警告: {}個",
                error_count,
                warning_count
            )?;
        }

        Ok(())
    }

    /// 警告をエラーとして扱うかどうかを設定
    pub fn set_warnings_as_errors(&mut self, value: bool) {
        self.warnings_as_errors = value;
        
        if value {
            // 既存の警告をエラーに変換
            self.errors.extend(self.warnings.drain(..));
        }
    }

    /// エラーと警告をクリア
    pub fn clear(&mut self) {
        self.errors.clear();
        self.warnings.clear();
    }
} 