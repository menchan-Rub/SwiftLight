//! 診断情報モジュール
//! 
//! コンパイラの診断情報を管理し、整形して出力するためのユーティリティを提供します。
//! このモジュールは、コンパイル時のエラー、警告、ヒントなどを高度に構造化し、
//! 開発者に対して最大限の情報と修正案を提供することを目的としています。

use std::io::{self, Write};
use std::fmt;
use std::collections::{HashMap, BTreeMap};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use std::borrow::Cow;
use colored::{Colorize, ColoredString};
use serde::{Serialize, Deserialize};
use unicode_width::UnicodeWidthStr;

/// ソースコード上の位置情報
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SourcePosition {
    /// 行番号（0ベース）
    pub line: usize,
    /// 列番号（0ベース）
    pub column: usize,
    /// バイトオフセット
    pub offset: usize,
}

impl SourcePosition {
    /// 新しい位置情報を作成
    pub fn new(line: usize, column: usize, offset: usize) -> Self {
        Self { line, column, offset }
    }
    
    /// 人間が読みやすい形式に変換（1ベース）
    pub fn to_human_readable(&self) -> (usize, usize) {
        (self.line + 1, self.column + 1)
    }
}

/// ソースコード上の範囲
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SourceRange {
    /// 開始位置
    pub start: SourcePosition,
    /// 終了位置
    pub end: SourcePosition,
    /// ファイルID
    pub file_id: usize,
}

impl SourceRange {
    /// 新しい範囲を作成
    pub fn new(start: SourcePosition, end: SourcePosition, file_id: usize) -> Self {
        Self { start, end, file_id }
    }
    
    /// 範囲が単一行に収まるかどうか
    pub fn is_single_line(&self) -> bool {
        self.start.line == self.end.line
    }
    
    /// 範囲の行数を取得
    pub fn line_count(&self) -> usize {
        self.end.line - self.start.line + 1
    }
}

/// 診断情報のレベル
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum DiagnosticLevel {
    /// エラー - プログラムのコンパイルを妨げる重大な問題
    Error,
    /// 警告 - 潜在的な問題や非推奨の使用法
    Warning,
    /// 情報 - 一般的な情報提供
    Info,
    /// ヒント - コード改善のための提案
    Hint,
    /// 注意 - 追加の文脈情報
    Note,
    /// 内部エラー - コンパイラ自体の問題
    InternalError,
}

impl DiagnosticLevel {
    /// レベルに応じた色付き文字列を取得
    pub fn colored(&self) -> ColoredString {
        match self {
            DiagnosticLevel::Error => "エラー".red().bold(),
            DiagnosticLevel::Warning => "警告".yellow().bold(),
            DiagnosticLevel::Info => "情報".blue().bold(),
            DiagnosticLevel::Hint => "ヒント".green().bold(),
            DiagnosticLevel::Note => "注意".cyan().bold(),
            DiagnosticLevel::InternalError => "内部エラー".magenta().bold(),
        }
    }
    
    /// レベルに応じた記号を取得
    pub fn symbol(&self) -> &'static str {
        match self {
            DiagnosticLevel::Error => "✘",
            DiagnosticLevel::Warning => "⚠",
            DiagnosticLevel::Info => "ℹ",
            DiagnosticLevel::Hint => "💡",
            DiagnosticLevel::Note => "✎",
            DiagnosticLevel::InternalError => "⚙",
        }
    }
}

impl fmt::Display for DiagnosticLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DiagnosticLevel::Error => write!(f, "エラー"),
            DiagnosticLevel::Warning => write!(f, "警告"),
            DiagnosticLevel::Info => write!(f, "情報"),
            DiagnosticLevel::Hint => write!(f, "ヒント"),
            DiagnosticLevel::Note => write!(f, "注意"),
            DiagnosticLevel::InternalError => write!(f, "内部エラー"),
        }
    }
}

/// 診断情報のカテゴリ
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DiagnosticCategory {
    /// 構文エラー
    Syntax,
    /// 型エラー
    Type,
    /// 名前解決エラー
    Name,
    /// メモリ安全性エラー
    Memory,
    /// 並行処理エラー
    Concurrency,
    /// パフォーマンス問題
    Performance,
    /// スタイル問題
    Style,
    /// セキュリティ問題
    Security,
    /// 内部エラー
    Internal,
    /// その他
    Other,
}

impl fmt::Display for DiagnosticCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DiagnosticCategory::Syntax => write!(f, "構文"),
            DiagnosticCategory::Type => write!(f, "型"),
            DiagnosticCategory::Name => write!(f, "名前解決"),
            DiagnosticCategory::Memory => write!(f, "メモリ安全性"),
            DiagnosticCategory::Concurrency => write!(f, "並行処理"),
            DiagnosticCategory::Performance => write!(f, "パフォーマンス"),
            DiagnosticCategory::Style => write!(f, "スタイル"),
            DiagnosticCategory::Security => write!(f, "セキュリティ"),
            DiagnosticCategory::Internal => write!(f, "内部"),
            DiagnosticCategory::Other => write!(f, "その他"),
        }
    }
}

/// 修正案の種類
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FixKind {
    /// コードの置換
    Replace {
        /// 置換範囲
        range: SourceRange,
        /// 置換テキスト
        replacement: String,
    },
    /// コードの挿入
    Insert {
        /// 挿入位置
        position: SourcePosition,
        /// 挿入テキスト
        text: String,
    },
    /// コードの削除
    Delete {
        /// 削除範囲
        range: SourceRange,
    },
    /// 複数の修正を一括で適用
    Composite {
        /// 修正リスト
        fixes: Vec<Fix>,
    },
}

/// 修正案
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Fix {
    /// 修正の種類
    pub kind: FixKind,
    /// 修正の説明
    pub description: String,
    /// 修正の優先度（低いほど優先）
    pub priority: u8,
}

impl Fix {
    /// 新しい置換修正を作成
    pub fn replace(range: SourceRange, replacement: String, description: String) -> Self {
        Self {
            kind: FixKind::Replace { range, replacement },
            description,
            priority: 0,
        }
    }
    
    /// 新しい挿入修正を作成
    pub fn insert(position: SourcePosition, text: String, description: String) -> Self {
        Self {
            kind: FixKind::Insert { position, text },
            description,
            priority: 0,
        }
    }
    
    /// 新しい削除修正を作成
    pub fn delete(range: SourceRange, description: String) -> Self {
        Self {
            kind: FixKind::Delete { range },
            description,
            priority: 0,
        }
    }
    
    /// 優先度を設定
    pub fn with_priority(mut self, priority: u8) -> Self {
        self.priority = priority;
        self
    }
}

/// 診断情報の関連コード
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiagnosticCode {
    /// コード（例: E0001）
    pub code: String,
    /// ドキュメントURL
    pub url: Option<String>,
}

impl DiagnosticCode {
    /// 新しい診断コードを作成
    pub fn new<S: Into<String>>(code: S) -> Self {
        Self {
            code: code.into(),
            url: None,
        }
    }
    
    /// ドキュメントURLを設定
    pub fn with_url<S: Into<String>>(mut self, url: S) -> Self {
        self.url = Some(url.into());
        self
    }
}

impl fmt::Display for DiagnosticCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.code)
    }
}

/// 診断情報のラベル
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiagnosticLabel {
    /// ラベルのメッセージ
    pub message: String,
    /// ラベルの範囲
    pub range: SourceRange,
    /// ラベルのスタイル
    pub style: LabelStyle,
}

/// ラベルのスタイル
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LabelStyle {
    /// 主要な問題箇所
    Primary,
    /// 関連する箇所
    Secondary,
}

impl DiagnosticLabel {
    /// 新しい主要ラベルを作成
    pub fn primary<S: Into<String>>(range: SourceRange, message: S) -> Self {
        Self {
            message: message.into(),
            range,
            style: LabelStyle::Primary,
        }
    }
    
    /// 新しい関連ラベルを作成
    pub fn secondary<S: Into<String>>(range: SourceRange, message: S) -> Self {
        Self {
            message: message.into(),
            range,
            style: LabelStyle::Secondary,
        }
    }
}

/// 診断情報
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Diagnostic {
    /// レベル
    pub level: DiagnosticLevel,
    /// カテゴリ
    pub category: DiagnosticCategory,
    /// メッセージ
    pub message: String,
    /// コード
    pub code: Option<DiagnosticCode>,
    /// ラベル
    pub labels: Vec<DiagnosticLabel>,
    /// 注釈
    pub notes: Vec<String>,
    /// 修正案
    pub fixes: Vec<Fix>,
    /// 関連する診断情報
    pub related: Vec<Diagnostic>,
    /// 発生時刻
    pub timestamp: u64,
    /// 追加のメタデータ
    pub metadata: HashMap<String, String>,
}

impl Diagnostic {
    /// 新しい診断情報を作成
    pub fn new<S: Into<String>>(level: DiagnosticLevel, category: DiagnosticCategory, message: S) -> Self {
        Self {
            level,
            category,
            message: message.into(),
            code: None,
            labels: Vec::new(),
            notes: Vec::new(),
            fixes: Vec::new(),
            related: Vec::new(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            metadata: HashMap::new(),
        }
    }
    
    /// エラー診断を作成
    pub fn error<S: Into<String>>(category: DiagnosticCategory, message: S) -> Self {
        Self::new(DiagnosticLevel::Error, category, message)
    }
    
    /// 警告診断を作成
    pub fn warning<S: Into<String>>(category: DiagnosticCategory, message: S) -> Self {
        Self::new(DiagnosticLevel::Warning, category, message)
    }
    
    /// 情報診断を作成
    pub fn info<S: Into<String>>(category: DiagnosticCategory, message: S) -> Self {
        Self::new(DiagnosticLevel::Info, category, message)
    }
    
    /// ヒント診断を作成
    pub fn hint<S: Into<String>>(category: DiagnosticCategory, message: S) -> Self {
        Self::new(DiagnosticLevel::Hint, category, message)
    }
    
    /// 注意診断を作成
    pub fn note<S: Into<String>>(category: DiagnosticCategory, message: S) -> Self {
        Self::new(DiagnosticLevel::Note, category, message)
    }
    
    /// 内部エラー診断を作成
    pub fn internal_error<S: Into<String>>(message: S) -> Self {
        Self::new(DiagnosticLevel::InternalError, DiagnosticCategory::Internal, message)
    }
    
    /// コードを設定
    pub fn with_code<S: Into<String>>(mut self, code: S) -> Self {
        self.code = Some(DiagnosticCode::new(code));
        self
    }
    
    /// コードとURLを設定
    pub fn with_code_and_url<S1: Into<String>, S2: Into<String>>(mut self, code: S1, url: S2) -> Self {
        self.code = Some(DiagnosticCode::new(code).with_url(url));
        self
    }
    
    /// ラベルを追加
    pub fn with_label(mut self, label: DiagnosticLabel) -> Self {
        self.labels.push(label);
        self
    }
    
    /// 主要ラベルを追加
    pub fn with_primary_label<S: Into<String>>(self, range: SourceRange, message: S) -> Self {
        self.with_label(DiagnosticLabel::primary(range, message))
    }
    
    /// 関連ラベルを追加
    pub fn with_secondary_label<S: Into<String>>(self, range: SourceRange, message: S) -> Self {
        self.with_label(DiagnosticLabel::secondary(range, message))
    }
    
    /// 注釈を追加
    pub fn with_note<S: Into<String>>(mut self, note: S) -> Self {
        self.notes.push(note.into());
        self
    }
    
    /// 修正案を追加
    pub fn with_fix(mut self, fix: Fix) -> Self {
        self.fixes.push(fix);
        self
    }
    
    /// 関連診断を追加
    pub fn with_related(mut self, related: Diagnostic) -> Self {
        self.related.push(related);
        self
    }
    
    /// メタデータを追加
    pub fn with_metadata<K: Into<String>, V: Into<String>>(mut self, key: K, value: V) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
    
    /// 主要ラベルの範囲を取得
    pub fn primary_range(&self) -> Option<SourceRange> {
        self.labels.iter()
            .find(|label| label.style == LabelStyle::Primary)
            .map(|label| label.range)
    }
    
    /// 修正案を適用したコードを生成
    pub fn apply_fixes(&self, source_code: &str, file_id: usize) -> Result<String, String> {
        if self.fixes.is_empty() {
            return Ok(source_code.to_string());
        }
        
        // 修正を優先度順にソート
        let mut fixes = self.fixes.clone();
        fixes.sort_by_key(|fix| fix.priority);
        
        let mut result = source_code.to_string();
        
        // 修正を適用（後ろから適用して位置ずれを防ぐ）
        for fix in fixes.iter().rev() {
            match &fix.kind {
                FixKind::Replace { range, replacement } => {
                    if range.file_id != file_id {
                        continue;
                    }
                    
                    let start_offset = range.start.offset;
                    let end_offset = range.end.offset;
                    
                    if start_offset > result.len() || end_offset > result.len() {
                        return Err(format!("修正範囲がソースコードの範囲外です: {}..{}", start_offset, end_offset));
                    }
                    
                    result.replace_range(start_offset..end_offset, replacement);
                },
                FixKind::Insert { position, text } => {
                    if position.offset > result.len() {
                        return Err(format!("挿入位置がソースコードの範囲外です: {}", position.offset));
                    }
                    
                    result.insert_str(position.offset, text);
                },
                FixKind::Delete { range } => {
                    if range.file_id != file_id {
                        continue;
                    }
                    
                    let start_offset = range.start.offset;
                    let end_offset = range.end.offset;
                    
                    if start_offset > result.len() || end_offset > result.len() {
                        return Err(format!("削除範囲がソースコードの範囲外です: {}..{}", start_offset, end_offset));
                    }
                    
                    result.replace_range(start_offset..end_offset, "");
                },
                FixKind::Composite { fixes } => {
                    // 複合修正は再帰的に処理
                    let mut composite_diagnostic = self.clone();
                    composite_diagnostic.fixes = fixes.clone();
                    result = composite_diagnostic.apply_fixes(&result, file_id)?;
                },
            }
        }
        
        Ok(result)
    }
    
    /// 診断情報の重大度を判定
    pub fn is_error(&self) -> bool {
        self.level == DiagnosticLevel::Error || self.level == DiagnosticLevel::InternalError
    }
    
    /// 診断情報が警告かどうか
    pub fn is_warning(&self) -> bool {
        self.level == DiagnosticLevel::Warning
    }
}

/// ソースファイル情報
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceFile {
    /// ファイルID
    pub id: usize,
    /// ファイルパス
    pub path: PathBuf,
    /// ソースコード
    pub content: String,
    /// 行の開始位置（バイトオフセット）
    pub line_starts: Vec<usize>,
}

impl SourceFile {
    /// 新しいソースファイルを作成
    pub fn new<P: AsRef<Path>>(id: usize, path: P, content: String) -> Self {
        let line_starts = Self::compute_line_starts(&content);
        Self {
            id,
            path: path.as_ref().to_path_buf(),
            content,
            line_starts,
        }
    }
    
    /// 行の開始位置を計算
    fn compute_line_starts(content: &str) -> Vec<usize> {
        let mut starts = vec![0];
        let mut pos = 0;
        
        for c in content.chars() {
            pos += c.len_utf8();
            if c == '\n' {
                starts.push(pos);
            }
        }
        
        starts
    }
    
    /// 位置情報から行と列を取得
    pub fn position_to_line_column(&self, offset: usize) -> SourcePosition {
        let line_idx = match self.line_starts.binary_search(&offset) {
            Ok(idx) => idx,
            Err(idx) => idx - 1,
        };
        
        let line_start = self.line_starts[line_idx];
        let column = offset - line_start;
        
        SourcePosition::new(line_idx, column, offset)
    }
    
    /// 行と列から位置情報を取得
    pub fn line_column_to_position(&self, line: usize, column: usize) -> Option<SourcePosition> {
        if line >= self.line_starts.len() {
            return None;
        }
        
        let line_start = self.line_starts[line];
        let offset = line_start + column;
        
        // 列が行の長さを超えていないか確認
        if line + 1 < self.line_starts.len() {
            let next_line_start = self.line_starts[line + 1];
            if offset >= next_line_start {
                return None;
            }
        } else if offset > self.content.len() {
            return None;
        }
        
        Some(SourcePosition::new(line, column, offset))
    }
    
    /// 指定された行を取得
    pub fn get_line(&self, line: usize) -> Option<&str> {
        if line >= self.line_starts.len() {
            return None;
        }
        
        let start = self.line_starts[line];
        let end = if line + 1 < self.line_starts.len() {
            self.line_starts[line + 1]
        } else {
            self.content.len()
        };
        
        Some(&self.content[start..end])
    }
    
    /// 指定された範囲のテキストを取得
    pub fn get_text(&self, range: SourceRange) -> Option<&str> {
        if range.file_id != self.id {
            return None;
        }
        
        if range.start.offset > self.content.len() || range.end.offset > self.content.len() {
            return None;
        }
        
        Some(&self.content[range.start.offset..range.end.offset])
    }
}

/// ソースファイルデータベース
#[derive(Debug, Clone)]
pub struct SourceDatabase {
    /// ファイルマップ（ID -> ファイル）
    files: BTreeMap<usize, SourceFile>,
    /// パスマップ（パス -> ID）
    path_to_id: HashMap<PathBuf, usize>,
    /// 次のファイルID
    next_id: usize,
}

impl SourceDatabase {
    /// 新しいソースデータベースを作成
    pub fn new() -> Self {
        Self {
            files: BTreeMap::new(),
            path_to_id: HashMap::new(),
            next_id: 0,
        }
    }
    
    /// ファイルを追加
    pub fn add_file<P: AsRef<Path>>(&mut self, path: P, content: String) -> usize {
        let path_buf = path.as_ref().to_path_buf();
        
        // 既に存在する場合はIDを返す
        if let Some(&id) = self.path_to_id.get(&path_buf) {
            return id;
        }
        
        let id = self.next_id;
        self.next_id += 1;
        
        let file = SourceFile::new(id, &path_buf, content);
        self.files.insert(id, file);
        self.path_to_id.insert(path_buf, id);
        
        id
    }
    
    /// ファイルを取得
    pub fn get_file(&self, id: usize) -> Option<&SourceFile> {
        self.files.get(&id)
    }
    
    /// パスからファイルを取得
    pub fn get_file_by_path<P: AsRef<Path>>(&self, path: P) -> Option<&SourceFile> {
        let path_buf = path.as_ref().to_path_buf();
        self.path_to_id.get(&path_buf).and_then(|&id| self.get_file(id))
    }
    
    /// パスからファイルIDを取得
    pub fn get_file_id<P: AsRef<Path>>(&self, path: P) -> Option<usize> {
        let path_buf = path.as_ref().to_path_buf();
        self.path_to_id.get(&path_buf).copied()
    }
    
    /// 全てのファイルを取得
    pub fn get_all_files(&self) -> impl Iterator<Item = &SourceFile> {
        self.files.values()
    }
    
    /// ファイル数を取得
    pub fn file_count(&self) -> usize {
        self.files.len()
    }
}

impl Default for SourceDatabase {
    fn default() -> Self {
        Self::new()
    }
}

/// 診断情報レンダラー
#[derive(Debug, Clone)]
pub struct DiagnosticRenderer {
    /// ソースデータベース
    source_db: Arc<SourceDatabase>,
    /// 色付き出力を使用するかどうか
    colored: bool,
    /// 行番号を表示するかどうか
    show_line_numbers: bool,
    /// コンテキスト行数
    context_lines: usize,
    /// 修正案を表示するかどうか
    show_fixes: bool,
    /// 関連診断を表示するかどうか
    show_related: bool,
    /// 最大行幅
    max_width: Option<usize>,
}

impl DiagnosticRenderer {
    /// 新しい診断レンダラーを作成
    pub fn new(source_db: Arc<SourceDatabase>) -> Self {
        Self {
            source_db,
            colored: true,
            show_line_numbers: true,
            context_lines: 2,
            show_fixes: true,
            show_related: true,
            max_width: None,
        }
    }
    
    /// 色付き出力を設定
    pub fn with_colored(mut self, colored: bool) -> Self {
        self.colored = colored;
        self
    }
    
    /// 行番号表示を設定
    pub fn with_line_numbers(mut self, show: bool) -> Self {
        self.show_line_numbers = show;
        self
    }
    
    /// コンテキスト行数を設定
    pub fn with_context_lines(mut self, lines: usize) -> Self {
        self.context_lines = lines;
        self
    }
    
    /// 修正案表示を設定
    pub fn with_fixes(mut self, show: bool) -> Self {
        self.show_fixes = show;
        self
    }
    
    /// 関連診断表示を設定
    pub fn with_related(mut self, show: bool) -> Self {
        self.show_related = show;
        self
    }
    
    /// 最大行幅を設定
    pub fn with_max_width(mut self, width: Option<usize>) -> Self {
        self.max_width = width;
        self
    }
    
    /// 診断情報をレンダリング
    pub fn render(&self, diagnostic: &Diagnostic, writer: &mut dyn Write) -> io::Result<()> {
        // ヘッダー
        self.render_header(diagnostic, writer)?;
        
        // ラベル
        for label in &diagnostic.labels {
            self.render_label(diagnostic, label, writer)?;
        }
        
        // 注釈
        for note in &diagnostic.notes {
            writeln!(writer, "注意: {}", note)?;
        }
        
        // 修正案
        if self.show_fixes && !diagnostic.fixes.is_empty() {
            self.render_fixes(diagnostic, writer)?;
        }
        
        // 関連診断
        if self.show_related {
            for related in &diagnostic.related {
                writeln!(writer)?;
                self.render(related, writer)?;
            }
        }
        
        Ok(())
    }
    
    /// ヘッダーをレンダリング
    fn render_header(&self, diagnostic: &Diagnostic, writer: &mut dyn Write) -> io::Result<()> {
        let level_str = if self.colored {
            diagnostic.level.colored().to_string()
        } else {
            format!("{}", diagnostic.level)
        };
        
        let code_str = diagnostic.code.as_ref().map_or(String::new(), |code| {
            format!("[{}] ", code)
        });
        
        writeln!(
            writer,
            "{} {}{}: {}",
            diagnostic.level.symbol(),
            code_str,
} 