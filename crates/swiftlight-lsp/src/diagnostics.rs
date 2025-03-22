// SwiftLight言語のLSP診断モジュール
//
// このモジュールはSwiftLightのLSPサーバーが提供する診断機能を実装します。
// ソースコードの問題（エラー、警告、ヒント）を検出し、エディタに通知します。

use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use anyhow::Result;
use lsp_types::{Diagnostic, DiagnosticSeverity, Position, Range, Url};

use swiftlight_compiler::{
    frontend::{
        lexer::tokenize,
        parser,
        source_map::SourceMap,
        error::CompilerError,
    },
    frontend::semantic::type_checker::TypeChecker,
};

use crate::ServerState;

/// ドキュメントの診断情報を取得
pub async fn get_diagnostics(
    state: Arc<Mutex<ServerState>>,
    uri: &Url,
) -> Result<Vec<Diagnostic>> {
    // ドキュメントの取得
    let content = {
        let state = state.lock().unwrap();
        match state.get_document(uri) {
            Some(doc) => doc.content.clone(),
            None => {
                log::warn!("診断を行うドキュメントが見つかりません: {}", uri);
                return Ok(vec![]);
            }
        }
    };

    // URIをファイルパスに変換
    let file_path = match uri.to_file_path() {
        Ok(path) => path,
        Err(_) => {
            log::warn!("URIをファイルパスに変換できません: {}", uri);
            return Ok(vec![]);
        }
    };

    // ファイル名の取得
    let file_name = file_path.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("unknown.swl");

    // 診断結果の収集
    let mut diagnostics = Vec::new();

    // ソースマップの作成
    let source_map = SourceMap::new();
    let file_id = source_map.add_file(file_name, content.clone());

    // レキサー（字句解析）のエラーを収集
    match tokenize(&content, file_name) {
        Ok(_tokens) => {
            // レキサーエラーなし
        },
        Err(error) => {
            // レキサーエラーを診断に変換
            diagnostics.push(compiler_error_to_diagnostic(&error, &content));
        }
    }

    // パーサー（構文解析）のエラーを収集
    match parser::parse(file_id, &content, &source_map) {
        Ok(ast) => {
            // 型チェッカーのエラーを収集
            let mut type_checker = TypeChecker::new(&source_map);
            if let Err(error) = type_checker.check_module(&ast) {
                // 型チェックエラーを診断に変換
                diagnostics.push(compiler_error_to_diagnostic(&error, &content));
            }

            // 型チェッカーの診断を変換
            for diagnostic in type_checker.diagnostics() {
                diagnostics.push(compiler_error_to_diagnostic(diagnostic, &content));
            }
        },
        Err(error) => {
            // パーサーエラーを診断に変換
            diagnostics.push(compiler_error_to_diagnostic(&error, &content));
        }
    }

    // 追加の静的解析による診断（将来的に拡張）

    Ok(diagnostics)
}

/// コンパイラエラーをLSP診断に変換
fn compiler_error_to_diagnostic(error: &CompilerError, source: &str) -> Diagnostic {
    // エラー位置の取得
    let location = error.location();
    let start_line = location.line.saturating_sub(1) as u32; // LSPは0始まり
    let start_column = location.column.saturating_sub(1) as u32; // LSPは0始まり
    
    // 行の終端を取得
    let end_column = if let Some(end_column) = error.end_column() {
        end_column.saturating_sub(1) as u32
    } else {
        // 行末まで
        let lines: Vec<&str> = source.lines().collect();
        if start_line as usize < lines.len() {
            lines[start_line as usize].len() as u32
        } else {
            start_column + 1 // 少なくとも1文字分
        }
    };

    // 終了位置の確定
    let end_line = if let Some(end_line) = error.end_line() {
        end_line.saturating_sub(1) as u32
    } else {
        start_line
    };

    // 診断の重大度
    let severity = match error.severity() {
        swiftlight_compiler::frontend::error::Severity::Error => Some(DiagnosticSeverity::ERROR),
        swiftlight_compiler::frontend::error::Severity::Warning => Some(DiagnosticSeverity::WARNING),
        swiftlight_compiler::frontend::error::Severity::Info => Some(DiagnosticSeverity::INFORMATION),
        swiftlight_compiler::frontend::error::Severity::Hint => Some(DiagnosticSeverity::HINT),
    };

    // 診断の作成
    Diagnostic {
        range: Range {
            start: Position {
                line: start_line,
                character: start_column,
            },
            end: Position {
                line: end_line,
                character: end_column,
            },
        },
        severity,
        code: None,
        code_description: None,
        source: Some("swiftlight".to_string()),
        message: error.message().to_string(),
        related_information: None,
        tags: None,
        data: None,
    }
}

/// 文法的に軽い問題をチェック
fn check_syntax_issues(source: &str) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let lines: Vec<&str> = source.lines().collect();

    for (line_idx, line) in lines.iter().enumerate() {
        // 行が長すぎる場合の警告
        if line.len() > 120 {
            diagnostics.push(Diagnostic {
                range: Range {
                    start: Position {
                        line: line_idx as u32,
                        character: 0,
                    },
                    end: Position {
                        line: line_idx as u32,
                        character: line.len() as u32,
                    },
                },
                severity: Some(DiagnosticSeverity::INFORMATION),
                code: None,
                code_description: None,
                source: Some("swiftlight-style".to_string()),
                message: "行が120文字を超えています。読みやすさのために短くすることを検討してください。".to_string(),
                related_information: None,
                tags: None,
                data: None,
            });
        }

        // トレイリングスペースの警告
        if line.ends_with(' ') || line.ends_with('\t') {
            let trailing_whitespace_start = line.len() - line.trim_end().len();
            diagnostics.push(Diagnostic {
                range: Range {
                    start: Position {
                        line: line_idx as u32,
                        character: trailing_whitespace_start as u32,
                    },
                    end: Position {
                        line: line_idx as u32,
                        character: line.len() as u32,
                    },
                },
                severity: Some(DiagnosticSeverity::HINT),
                code: None,
                code_description: None,
                source: Some("swiftlight-style".to_string()),
                message: "行末の空白文字は不要です".to_string(),
                related_information: None,
                tags: None,
                data: None,
            });
        }

        // TODO/FIXMEコメントの強調表示
        if line.contains("TODO") || line.contains("FIXME") {
            let start_idx = if line.contains("TODO") {
                line.find("TODO").unwrap()
            } else {
                line.find("FIXME").unwrap()
            };

            diagnostics.push(Diagnostic {
                range: Range {
                    start: Position {
                        line: line_idx as u32,
                        character: start_idx as u32,
                    },
                    end: Position {
                        line: line_idx as u32,
                        character: line.len() as u32,
                    },
                },
                severity: Some(DiagnosticSeverity::INFORMATION),
                code: None,
                code_description: None,
                source: Some("swiftlight-todo".to_string()),
                message: "未解決のタスクが見つかりました".to_string(),
                related_information: None,
                tags: None,
                data: None,
            });
        }
    }

    diagnostics
}
