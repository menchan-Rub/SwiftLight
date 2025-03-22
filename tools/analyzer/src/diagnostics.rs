// 診断情報モジュール
// 解析結果の表示と出力を担当します

use std::fs::File;
use std::io::{self, Write};
use std::path::PathBuf;
use crate::config::{Config, OutputFormat, Severity};

/// 診断情報を表す構造体
#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub file: PathBuf,
    pub line: usize,
    pub column: usize,
    pub message: String,
    pub rule_id: String,
    pub severity: Severity,
    pub suggestions: Vec<String>,
}

/// 診断情報をレポートする
pub fn report_diagnostics(diagnostics: &[Diagnostic], config: &Config) {
    if diagnostics.is_empty() {
        println!("問題は検出されませんでした。");
        return;
    }
    
    // 出力先を決定
    match &config.output_file {
        Some(path) => {
            match File::create(path) {
                Ok(mut file) => {
                    write_diagnostics(&mut file, diagnostics, config);
                    println!("診断結果を {} に出力しました。", path.display());
                },
                Err(err) => {
                    eprintln!("ファイルへの書き込みに失敗しました: {}: {}", path.display(), err);
                    // 標準出力にフォールバック
                    write_diagnostics(&mut io::stdout(), diagnostics, config);
                }
            }
        },
        None => {
            // 標準出力に出力
            write_diagnostics(&mut io::stdout(), diagnostics, config);
        }
    }
    
    // 統計情報を表示
    print_statistics(diagnostics);
}

/// 診断情報を書き込む
fn write_diagnostics<W: Write>(writer: &mut W, diagnostics: &[Diagnostic], config: &Config) {
    match config.output_format {
        OutputFormat::Text => write_text_format(writer, diagnostics),
        OutputFormat::Json => write_json_format(writer, diagnostics),
        OutputFormat::Xml => write_xml_format(writer, diagnostics),
    }
}

/// テキスト形式で診断情報を出力
fn write_text_format<W: Write>(writer: &mut W, diagnostics: &[Diagnostic]) {
    writeln!(writer, "SwiftLight 静的解析の結果:").unwrap();
    writeln!(writer, "============================").unwrap();
    
    for (i, diag) in diagnostics.iter().enumerate() {
        writeln!(writer, "[{}] {} ({}:{}:{})", 
            severity_to_string(diag.severity),
            diag.message,
            diag.file.display(),
            diag.line,
            diag.column
        ).unwrap();
        
        writeln!(writer, "    ルールID: {}", diag.rule_id).unwrap();
        
        if !diag.suggestions.is_empty() {
            writeln!(writer, "    提案:").unwrap();
            for suggestion in &diag.suggestions {
                writeln!(writer, "      - {}", suggestion).unwrap();
            }
        }
        
        if i < diagnostics.len() - 1 {
            writeln!(writer, "----------------------------").unwrap();
        }
    }
}

/// JSON形式で診断情報を出力
fn write_json_format<W: Write>(writer: &mut W, diagnostics: &[Diagnostic]) {
    // 実際の実装ではserdeなどを使ってJSON形式に変換する
    use serde::Serialize;
    use serde_json::json;

    #[derive(Serialize)]
    struct DiagnosticOutput<'a> {
        severity: &'static str,
        message: &'a str,
        file: String,
        line: usize,
        column: usize,
        rule_id: &'a str,
        suggestions: &'a [String],
    }

    let diagnostics_json: Vec<DiagnosticOutput> = diagnostics
        .iter()
        .map(|diag| DiagnosticOutput {
            severity: severity_to_string(diag.severity),
            message: &diag.message,
            file: diag.file.display().to_string(),
            line: diag.line,
            column: diag.column,
            rule_id: &diag.rule_id,
            suggestions: &diag.suggestions,
        })
        .collect();

    let output = json!({
        "diagnostics": diagnostics_json
    });

    serde_json::to_writer_pretty(writer, &output).unwrap_or_else(|e| {
        eprintln!("JSONの書き込みエラー: {}", e);
    });
}

/// XML形式で診断情報を出力
fn write_xml_format<W: Write>(writer: &mut W, diagnostics: &[Diagnostic]) {
    writeln!(writer, "<?xml version=\"1.0\" encoding=\"UTF-8\"?>").unwrap();
    writeln!(writer, "<diagnostics>").unwrap();
    
    for diag in diagnostics {
        writeln!(writer, "  <diagnostic>").unwrap();
        writeln!(writer, "    <severity>{}</severity>", severity_to_string(diag.severity)).unwrap();
        writeln!(writer, "    <message>{}</message>", escape_xml(&diag.message)).unwrap();
        writeln!(writer, "    <file>{}</file>", diag.file.display()).unwrap();
        writeln!(writer, "    <line>{}</line>", diag.line).unwrap();
        writeln!(writer, "    <column>{}</column>", diag.column).unwrap();
        writeln!(writer, "    <rule_id>{}</rule_id>", diag.rule_id).unwrap();
        
        writeln!(writer, "    <suggestions>").unwrap();
        for suggestion in &diag.suggestions {
            writeln!(writer, "      <suggestion>{}</suggestion>", escape_xml(suggestion)).unwrap();
        }
        writeln!(writer, "    </suggestions>").unwrap();
        
        writeln!(writer, "  </diagnostic>").unwrap();
    }
    
    writeln!(writer, "</diagnostics>").unwrap();
}

/// 統計情報を表示
fn print_statistics(diagnostics: &[Diagnostic]) {
    let total = diagnostics.len();
    let errors = diagnostics.iter().filter(|d| d.severity == Severity::Error).count();
    let warnings = diagnostics.iter().filter(|d| d.severity == Severity::Warning).count();
    let infos = diagnostics.iter().filter(|d| d.severity == Severity::Info).count();
    
    println!("\n統計情報:");
    println!("  合計: {}", total);
    println!("  エラー: {}", errors);
    println!("  警告: {}", warnings);
    println!("  情報: {}", infos);
}

/// 重大度を文字列に変換
fn severity_to_string(severity: Severity) -> &'static str {
    match severity {
        Severity::Error => "エラー",
        Severity::Warning => "警告",
        Severity::Info => "情報",
    }
}

/// JSON文字列のエスケープ処理
fn escape_json(s: &str) -> String {
    s.replace('\\', "\\\\")
     .replace('"', "\\\"")
     .replace('\n', "\\n")
     .replace('\r', "\\r")
     .replace('\t', "\\t")
}

/// XML文字列のエスケープ処理
fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
     .replace('<', "&lt;")
     .replace('>', "&gt;")
     .replace('"', "&quot;")
     .replace('\'', "&apos;")
} 