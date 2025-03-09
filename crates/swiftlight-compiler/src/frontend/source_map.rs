//! # ソースマップ
//! 
//! ソースコードの位置情報を管理するモジュールです。
//! ファイル名、行番号、列番号などのマッピングを提供します。

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::fmt;
use std::fs;
use std::sync::Arc;

use crate::frontend::error::{SourceLocation, CompilerError, ErrorKind, Result};

/// ソースファイルの内容と位置情報を表す構造体
#[derive(Debug, Clone)]
pub struct SourceFile {
    /// ファイル名（パス）
    pub file_name: String,
    /// 実際のファイルパス
    pub path: PathBuf,
    /// ファイルの内容
    pub content: String,
    /// 各行の開始位置（バイトオフセット）
    line_starts: Vec<usize>,
}

impl SourceFile {
    /// 新しいソースファイルを作成
    pub fn new(file_name: impl Into<String>, path: impl Into<PathBuf>, content: impl Into<String>) -> Self {
        let content = content.into();
        let mut line_starts = Vec::new();
        
        // 行の開始位置を計算
        line_starts.push(0); // 最初の行は0から始まる
        for (i, c) in content.char_indices() {
            if c == '\n' {
                line_starts.push(i + 1);
            }
        }
        
        Self {
            file_name: file_name.into(),
            path: path.into(),
            content,
            line_starts,
        }
    }
    
    /// ファイルパスから新しいソースファイルを読み込む
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let file_name = path.file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "<unknown>".to_string());
        
        let content = fs::read_to_string(path)
            .map_err(|e| CompilerError::new(
                ErrorKind::IO,
                format!("ファイル読み込みエラー: {}", e),
                None
            ))?;
        
        Ok(Self::new(file_name, path, content))
    }
    
    /// バイトオフセットから行番号と列番号を取得
    pub fn location_info(&self, offset: usize) -> (usize, usize) {
        // オフセットを含む行を二分探索で検索
        let line_index = match self.line_starts.binary_search(&offset) {
            Ok(i) => i,
            Err(i) => i.saturating_sub(1),
        };
        
        let line = line_index + 1; // 1-indexed
        let column = offset - self.line_starts[line_index] + 1; // 1-indexed
        
        (line, column)
    }
    
    /// SourceLocationオブジェクトを作成
    pub fn get_location(&self, start: usize, end: usize) -> SourceLocation {
        let (line, column) = self.location_info(start);
        SourceLocation::new(&self.file_name, line, column, start, end)
    }
    
    /// 指定された行のコードスニペットを取得
    pub fn get_line(&self, line: usize) -> Option<&str> {
        if line == 0 || line > self.line_starts.len() {
            return None;
        }
        
        let start = self.line_starts[line - 1];
        let end = if line < self.line_starts.len() {
            self.line_starts[line]
        } else {
            self.content.len()
        };
        
        Some(&self.content[start..end])
    }
    
    /// エラー箇所を含むコードスニペットを生成
    pub fn get_snippet(&self, location: &SourceLocation, context_lines: usize) -> String {
        let line = location.line;
        if line == 0 {
            return String::new();
        }
        
        let start_line = line.saturating_sub(context_lines);
        let end_line = std::cmp::min(line + context_lines, self.line_starts.len());
        
        let mut result = String::new();
        for l in start_line..=end_line {
            let line_content = self.get_line(l).unwrap_or("");
            let prefix = format!("{:>4} | ", l);
            
            result.push_str(&prefix);
            result.push_str(line_content);
            
            if !line_content.ends_with('\n') {
                result.push('\n');
            }
            
            // エラー位置にマーカーを追加
            if l == line {
                result.push_str(&" ".repeat(prefix.len()));
                
                // エラー位置までのスペース
                let offset = location.column - 1;
                result.push_str(&" ".repeat(offset));
                
                // エラーの範囲を示すキャレット (^)
                let error_len = location.end - location.start;
                let marker_len = if error_len > 0 { error_len } else { 1 };
                result.push_str(&"^".repeat(marker_len));
                
                result.push('\n');
            }
        }
        
        result
    }
}

/// 複数のソースファイルを管理するマップ
#[derive(Debug, Clone, Default)]
pub struct SourceMap {
    /// ファイル名とソースファイルのマップ
    files: HashMap<String, Arc<SourceFile>>,
}

impl SourceMap {
    /// 新しいソースマップを作成
    pub fn new() -> Self {
        Self {
            files: HashMap::new(),
        }
    }
    
    /// ソースファイルを追加
    pub fn add_file(&mut self, file: SourceFile) -> Arc<SourceFile> {
        let file_name = file.file_name.clone();
        let file_arc = Arc::new(file);
        self.files.insert(file_name, Arc::clone(&file_arc));
        file_arc
    }
    
    /// ファイルパスからソースファイルを読み込んで追加
    pub fn load_file(&mut self, path: impl AsRef<Path>) -> Result<Arc<SourceFile>> {
        let file = SourceFile::from_path(path)?;
        Ok(self.add_file(file))
    }
    
    /// ファイル名からソースファイルを取得
    pub fn get_file(&self, file_name: &str) -> Option<Arc<SourceFile>> {
        self.files.get(file_name).cloned()
    }
    
    /// 位置情報からソースファイルとコードスニペットを取得
    pub fn get_snippet(&self, location: &SourceLocation, context_lines: usize) -> Option<String> {
        self.get_file(&location.file_name)
            .map(|file| file.get_snippet(location, context_lines))
    }
}

/// テスト
#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::fs::File;
    use tempfile::tempdir;
    
    #[test]
    fn test_source_file_location() {
        let content = "line1\nline2\nline3\n";
        let source_file = SourceFile::new("test.swl", PathBuf::from("test.swl"), content);
        
        // 各行の開始位置を確認
        assert_eq!(source_file.line_starts, vec![0, 6, 12, 18]);
        
        // オフセットから行と列を取得
        assert_eq!(source_file.location_info(0), (1, 1)); // 最初の文字
        assert_eq!(source_file.location_info(7), (2, 2)); // 2行目の2文字目
        assert_eq!(source_file.location_info(17), (3, 6)); // 3行目の最後
    }
    
    #[test]
    fn test_get_snippet() {
        let content = "function test() {\n    let x = 10;\n    let y = 20;\n}";
        let source_file = SourceFile::new("test.swl", PathBuf::from("test.swl"), content);
        
        let location = source_file.get_location(27, 33); // "let y = 20" 行
        assert_eq!(location.line, 3);
        assert_eq!(location.column, 5);
        
        let snippet = source_file.get_snippet(&location, 1);
        let expected = "   2 |     let x = 10;\n   3 |     let y = 20;\n       ^^^^^\n   4 | }\n";
        assert_eq!(snippet, expected);
    }
    
    #[test]
    fn test_source_map() -> Result<()> {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.swl");
        
        {
            let mut file = File::create(&file_path).unwrap();
            writeln!(file, "function test() {{").unwrap();
            writeln!(file, "    let x = 10;").unwrap();
            writeln!(file, "    let y = 20;").unwrap();
            write!(file, "}}").unwrap();
        }
        
        let mut source_map = SourceMap::new();
        let file = source_map.load_file(&file_path)?;
        
        assert_eq!(file.file_name, "test.swl");
        assert_eq!(file.content, "function test() {\n    let x = 10;\n    let y = 20;\n}");
        
        Ok(())
    }
} 