// 静的解析モジュール
// SwiftLight言語のコード分析を行います

use std::path::Path;
use crate::config::{Config, Severity};
use crate::rules::{Rule, RuleViolation};
use crate::diagnostics::Diagnostic;

// 利用可能なすべてのルールを登録・適用する
pub fn analyze_code(source: &str, file_path: &Path, config: &Config) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    
    // ソースコードの前処理
    let lines: Vec<&str> = source.lines().collect();
    
    // 適用するルールを確定
    let rules = determine_rules(&config.rules);
    
    // 各ルールを適用
    for rule in rules {
        let violations = rule.check(source, &lines, file_path);
        
        // 診断情報を変換
        for violation in violations {
            if violation.severity.ge(&config.min_severity) {
                diagnostics.push(Diagnostic {
                    file: file_path.to_path_buf(),
                    line: violation.line,
                    column: violation.column,
                    message: violation.message,
                    rule_id: violation.rule_id,
                    severity: violation.severity,
                    suggestions: violation.suggestions,
                });
            }
        }
    }
    
    // 全体的な問題の検出（プロジェクト全体の文脈に依存する問題）
    detect_global_issues(source, &lines, file_path, &mut diagnostics, config);
    
    diagnostics
}

// ルールの決定
fn determine_rules(rule_names: &[String]) -> Vec<Box<dyn Rule>> {
    let mut rules: Vec<Box<dyn Rule>> = Vec::new();
    
    // すべてのルールを適用するかどうか
    let apply_all = rule_names.contains(&"all".to_string());
    
    // 各ルールを確認
    if apply_all || rule_names.contains(&"style".to_string()) {
        // スタイルルール
        rules.push(Box::new(rules::StyleRule::new()));
    }
    
    if apply_all || rule_names.contains(&"naming".to_string()) {
        // 命名規則
        rules.push(Box::new(rules::NamingRule::new()));
    }
    
    if apply_all || rule_names.contains(&"performance".to_string()) {
        // パフォーマンス問題
        rules.push(Box::new(rules::PerformanceRule::new()));
    }
    
    if apply_all || rule_names.contains(&"security".to_string()) {
        // セキュリティ問題
        rules.push(Box::new(rules::SecurityRule::new()));
    }
    
    if apply_all || rule_names.contains(&"correctness".to_string()) {
        // 正確性の問題
        rules.push(Box::new(rules::CorrectnessRule::new()));
    }
    
    if apply_all || rule_names.contains(&"maintainability".to_string()) {
        // 保守性の問題
        rules.push(Box::new(rules::MaintainabilityRule::new()));
    }
    
    // 将来追加されるルール...
    
    rules
}

// プロジェクト全体の問題を検出
fn detect_global_issues(
    source: &str,
    lines: &[&str],
    file_path: &Path,
    diagnostics: &mut Vec<Diagnostic>,
    config: &Config
) {
    // ファイル長の確認
    if lines.len() > 1000 && config.min_severity <= Severity::Warning {
        diagnostics.push(Diagnostic {
            file: file_path.to_path_buf(),
            line: 1,
            column: 1,
            message: format!("ファイルが長すぎます（{}行）。1000行以下に分割することを検討してください。", lines.len()),
            rule_id: "global:file_length".to_string(),
            severity: Severity::Warning,
            suggestions: vec!["複数の小さなモジュールに分割することを検討してください。".to_string()],
        });
    }
    
    // インポート/使用宣言の重複チェック
    let mut imports = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        if line.trim().starts_with("use ") || line.trim().starts_with("import ") {
            let import = line.trim();
            if imports.contains(&import) {
                diagnostics.push(Diagnostic {
                    file: file_path.to_path_buf(),
                    line: i + 1,
                    column: 1,
                    message: format!("重複したインポート/使用宣言: {}", import),
                    rule_id: "global:duplicate_import".to_string(),
                    severity: Severity::Warning,
                    suggestions: vec!["重複したインポート/使用宣言を削除してください。".to_string()],
                });
            } else {
                imports.push(import);
            }
        }
    }
    
    // TODOコメントの検出
    for (i, line) in lines.iter().enumerate() {
        if line.contains("TODO") || line.contains("FIXME") {
            diagnostics.push(Diagnostic {
                file: file_path.to_path_buf(),
                line: i + 1,
                column: line.find("TODO").unwrap_or_else(|| line.find("FIXME").unwrap_or(0)) + 1,
                message: format!("未解決のタスク: {}", line.trim()),
                rule_id: "global:todo_comment".to_string(),
                severity: Severity::Info,
                suggestions: vec!["このTODO/FIXMEを解決するか、正式なタスク追跡システムに記録してください。".to_string()],
            });
        }
    }
    
    // その他のグローバル問題の検出...
} 