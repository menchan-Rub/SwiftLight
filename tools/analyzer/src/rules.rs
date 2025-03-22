// 静的解析ルールモジュール
// SwiftLight言語用の静的解析ルールを定義します

use std::path::Path;
use crate::config::Severity;

/// ルール違反を表す構造体
#[derive(Debug, Clone)]
pub struct RuleViolation {
    pub rule_id: String,
    pub message: String,
    pub line: usize,
    pub column: usize,
    pub severity: Severity,
    pub suggestions: Vec<String>,
}

/// 静的解析ルールのトレイト
pub trait Rule {
    /// ソースコードをチェックし、違反を報告する
    fn check(&self, source: &str, lines: &[&str], file_path: &Path) -> Vec<RuleViolation>;
    
    /// ルールのIDを返す
    fn id(&self) -> &str;
    
    /// ルールの説明を返す
    fn description(&self) -> &str;
}

/// スタイルに関するルール
pub struct StyleRule {
    id: String,
    description: String,
}

impl StyleRule {
    pub fn new() -> Self {
        StyleRule {
            id: "style".to_string(),
            description: "コードスタイルに関するルール".to_string(),
        }
    }
}

impl Rule for StyleRule {
    fn check(&self, source: &str, lines: &[&str], file_path: &Path) -> Vec<RuleViolation> {
        let mut violations = Vec::new();
        
        // インデントのチェック
        for (i, line) in lines.iter().enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            
            // タブ文字の使用をチェック
            if line.contains('\t') {
                violations.push(RuleViolation {
                    rule_id: format!("{}:tabs", self.id()),
                    message: "タブ文字の代わりにスペースを使用してください".to_string(),
                    line: i + 1,
                    column: line.find('\t').unwrap_or(0) + 1,
                    severity: Severity::Warning,
                    suggestions: vec!["タブをスペース（4または2個）に置き換えてください".to_string()],
                });
            }
            
            // 行末の空白をチェック
            if line.ends_with(' ') || line.ends_with('\t') {
                violations.push(RuleViolation {
                    rule_id: format!("{}:trailing_whitespace", self.id()),
                    message: "行末の空白を削除してください".to_string(),
                    line: i + 1,
                    column: line.len(),
                    severity: Severity::Info,
                    suggestions: vec!["行末の空白を削除してください".to_string()],
                });
            }
            
            // 行の長さをチェック
            if line.len() > 100 {
                violations.push(RuleViolation {
                    rule_id: format!("{}:line_length", self.id()),
                    message: format!("行が長すぎます（{}文字）。100文字以下にしてください", line.len()),
                    line: i + 1,
                    column: 101,
                    severity: Severity::Info,
                    suggestions: vec!["長い行は複数行に分割することを検討してください".to_string()],
                });
            }
        }
        
        // その他のスタイルチェック...
        
        violations
    }
    
    fn id(&self) -> &str {
        &self.id
    }
    
    fn description(&self) -> &str {
        &self.description
    }
}

/// 命名規則に関するルール
pub struct NamingRule {
    id: String,
    description: String,
}

impl NamingRule {
    pub fn new() -> Self {
        NamingRule {
            id: "naming".to_string(),
            description: "命名規則に関するルール".to_string(),
        }
    }
}

impl Rule for NamingRule {
    fn check(&self, source: &str, lines: &[&str], file_path: &Path) -> Vec<RuleViolation> {
        let mut violations = Vec::new();
        
        // 変数宣言を解析するためのパーサーを使用
        let variable_declarations = parse_variable_declarations(source);
        
        // 変数名のチェック（パーサーを使用）
        for var_decl in variable_declarations {
            let name = &var_decl.name;
            let line_idx = var_decl.line - 1; // 0-indexedに変換
            
            // キャメルケースのチェック（変数名）
            if !name.is_empty() && name.chars().next().unwrap().is_uppercase() {
                violations.push(RuleViolation {
                    rule_id: format!("{}:variable_case", self.id()),
                    message: format!("変数名 '{}' はキャメルケース（先頭小文字）で定義してください", name),
                    line: var_decl.line,
                    column: var_decl.column,
                    severity: Severity::Warning,
                    suggestions: vec![format!("'{}' を '{}'に変更することを検討してください", 
                                      name, 
                                      name.chars().next().unwrap().to_lowercase().collect::<String>() + &name[1..])],
                });
            }
        }
        
        // 関数名のチェック
        for (i, line) in lines.iter().enumerate() {
            if let Some(pos) = line.find("fn ") {
                let rest = &line[pos + 3..];
                if let Some(name_end) = rest.find('(') {
                    let name = rest[..name_end].trim();
                    
                    // スネークケースのチェック（関数名）
                    if !name.is_empty() && name.contains(char::is_uppercase) {
                        violations.push(RuleViolation {
                            rule_id: format!("{}:function_case", self.id()),
                            message: format!("関数名 '{}' はスネークケース（アンダースコア区切り小文字）で定義してください", name),
                            line: i + 1,
                            column: pos + 3 + 1,
                            severity: Severity::Warning,
                            suggestions: vec![format!("'{}' を '{}'に変更することを検討してください", 
                                              name, 
                                              name.chars().map(|c| if c.is_uppercase() { 
                                                '_'.to_string() + &c.to_lowercase().to_string() 
                                              } else { 
                                                c.to_string() 
                                              }).collect::<String>())],
                        });
                    }
                }
            }
        }
        
        violations
    }
    
    fn id(&self) -> &str {
        &self.id
    }
    
    fn description(&self) -> &str {
        &self.description
    }
}

/// 変数宣言を表す構造体
#[derive(Debug)]
struct VariableDeclaration {
    name: String,
    line: usize,
    column: usize,
}

/// ソースコードから変数宣言を解析する
fn parse_variable_declarations(source: &str) -> Vec<VariableDeclaration> {
    let mut declarations = Vec::new();
    let lines: Vec<&str> = source.lines().collect();
    
    // 簡易パーサー
    for (line_idx, line) in lines.iter().enumerate() {
        let mut in_comment = false;
        let mut in_string = false;
        let mut char_idx = 0;
        
        while char_idx < line.len() {
            let substr = &line[char_idx..];
            
            // コメントのスキップ
            if substr.starts_with("//") {
                break;  // 行コメント
            } else if substr.starts_with("/*") {
                in_comment = true;
                char_idx += 2;
                continue;
            } else if in_comment && substr.starts_with("*/") {
                in_comment = false;
                char_idx += 2;
                continue;
            }
            
            if in_comment {
                char_idx += 1;
                continue;
            }
            
            // 文字列リテラルのスキップ
            if !in_string && substr.starts_with('"') {
                in_string = true;
                char_idx += 1;
                continue;
            } else if in_string && substr.starts_with('"') && !substr.starts_with("\\\"") {
                in_string = false;
                char_idx += 1;
                continue;
            }
            
            if in_string {
                char_idx += 1;
                continue;
            }
            
            // 変数宣言の検出
            if substr.starts_with("let ") {
                let var_start = char_idx + 4;  // "let "の長さ
                let var_substr = &line[var_start..];
                
                // 変数名の終わりを検出
                let mut name_end = 0;
                let mut in_name = false;
                let mut name_start = 0;
                
                for (i, c) in var_substr.char_indices() {
                    if !in_name && c.is_alphabetic() {
                        in_name = true;
                        name_start = i;
                    }
                    
                    if in_name && (c == ':' || c == '=' || c == ' ' || c == ';') {
                        name_end = i;
                        break;
                    }
                }
                
                if in_name && name_end > name_start {
                    let var_name = var_substr[name_start..name_end].trim();
                    declarations.push(VariableDeclaration {
                        name: var_name.to_string(),
                        line: line_idx + 1,  // 1-indexedに変換
                        column: var_start + name_start + 1,  // 1-indexedに変換
                    });
                }
            }
            
            char_idx += 1;
        }
    }
    
    declarations
}

/// パフォーマンスに関するルール
pub struct PerformanceRule {
    id: String,
    description: String,
}

impl PerformanceRule {
    pub fn new() -> Self {
        PerformanceRule {
            id: "performance".to_string(),
            description: "パフォーマンスに関するルール".to_string(),
        }
    }
}

impl Rule for PerformanceRule {
    fn check(&self, source: &str, lines: &[&str], file_path: &Path) -> Vec<RuleViolation> {
        let mut violations = Vec::new();
        
        // ループ内での不要な割り当てのチェック
        for (i, line) in lines.iter().enumerate() {
            // ループ内での不要な割り当てを検出（簡易版）
            if is_in_loop(lines, i) && line.contains("new") {
                violations.push(RuleViolation {
                    rule_id: format!("{}:allocation_in_loop", self.id()),
                    message: "ループ内でメモリ割り当てが行われています".to_string(),
                    line: i + 1,
                    column: line.find("new").unwrap_or(0) + 1,
                    severity: Severity::Warning,
                    suggestions: vec![
                        "可能な場合は、ループの外でメモリを事前に割り当てることを検討してください".to_string(),
                        "繰り返しの各反復で同じオブジェクトを再利用できるか検討してください".to_string(),
                    ],
                });
            }
        }
        
        violations
    }
    
    fn id(&self) -> &str {
        &self.id
    }
    
    fn description(&self) -> &str {
        &self.description
    }
}

/// セキュリティに関するルール
pub struct SecurityRule {
    id: String,
    description: String,
}

impl SecurityRule {
    pub fn new() -> Self {
        SecurityRule {
            id: "security".to_string(),
            description: "セキュリティに関するルール".to_string(),
        }
    }
}

impl Rule for SecurityRule {
    fn check(&self, source: &str, lines: &[&str], file_path: &Path) -> Vec<RuleViolation> {
        let mut violations = Vec::new();
        
        // 危険な関数の使用をチェック
        for (i, line) in lines.iter().enumerate() {
            // 安全でない関数の使用を検出
            if line.contains("unsafe") {
                violations.push(RuleViolation {
                    rule_id: format!("{}:unsafe_code", self.id()),
                    message: "unsafeコードが使用されています".to_string(),
                    line: i + 1,
                    column: line.find("unsafe").unwrap_or(0) + 1,
                    severity: Severity::Warning,
                    suggestions: vec![
                        "可能な場合は、安全な代替手段を使用してください".to_string(),
                        "unsafeブロックを最小限に保ち、十分にコメントを付けてください".to_string(),
                    ],
                });
            }
            
            // 未検証の入力を使用する関数をチェック
            if line.contains("exec(") || line.contains("system(") {
                violations.push(RuleViolation {
                    rule_id: format!("{}:command_injection", self.id()),
                    message: "コマンドインジェクションの可能性があります".to_string(),
                    line: i + 1,
                    column: line.find("exec(").or_else(|| line.find("system(")).unwrap_or(0) + 1,
                    severity: Severity::Error,
                    suggestions: vec![
                        "ユーザー入力を直接コマンドに渡す前に検証してください".to_string(),
                        "可能な場合は、より安全なAPIを使用してください".to_string(),
                    ],
                });
            }
        }
        
        violations
    }
    
    fn id(&self) -> &str {
        &self.id
    }
    
    fn description(&self) -> &str {
        &self.description
    }
}

/// 正確性に関するルール
pub struct CorrectnessRule {
    id: String,
    description: String,
}

impl CorrectnessRule {
    pub fn new() -> Self {
        CorrectnessRule {
            id: "correctness".to_string(),
            description: "コードの正確性に関するルール".to_string(),
        }
    }
}

impl Rule for CorrectnessRule {
    fn check(&self, source: &str, lines: &[&str], file_path: &Path) -> Vec<RuleViolation> {
        let mut violations = Vec::new();
        
        // 未使用の変数をチェック（簡易版）
        let mut variables = Vec::new();
        let mut used_vars = Vec::new();
        
        for (i, line) in lines.iter().enumerate() {
            // 変数宣言を検出（簡易版）
            if let Some(pos) = line.find("let ") {
                let rest = &line[pos + 4..];
                if let Some(name_end) = rest.find(':').or_else(|| rest.find('=')) {
                    let name = rest[..name_end].trim();
                    variables.push((name.to_string(), i));
                }
            }
            
            // 変数の使用を検出（非常に簡易的）
            for (var, _) in &variables {
                if line.contains(var) && !line.contains(&format!("let {}", var)) {
                    used_vars.push(var.clone());
                }
            }
        }
        
        // 未使用の変数を報告
        for (var, line_num) in variables {
            if !used_vars.contains(&var) && !var.starts_with('_') {
                violations.push(RuleViolation {
                    rule_id: format!("{}:unused_variable", self.id()),
                    message: format!("変数 '{}' は宣言されていますが使用されていません", var),
                    line: line_num + 1,
                    column: lines[line_num].find(&var).unwrap_or(0) + 1,
                    severity: Severity::Warning,
                    suggestions: vec![
                        format!("未使用の変数を削除するか、名前の先頭にアンダースコアを付けて(_{}とする)未使用であることを明示してください", var),
                    ],
                });
            }
        }
        
        violations
    }
    
    fn id(&self) -> &str {
        &self.id
    }
    
    fn description(&self) -> &str {
        &self.description
    }
}

/// 保守性に関するルール
pub struct MaintainabilityRule {
    id: String,
    description: String,
}

impl MaintainabilityRule {
    pub fn new() -> Self {
        MaintainabilityRule {
            id: "maintainability".to_string(),
            description: "コードの保守性に関するルール".to_string(),
        }
    }
}

impl Rule for MaintainabilityRule {
    fn check(&self, source: &str, lines: &[&str], file_path: &Path) -> Vec<RuleViolation> {
        let mut violations = Vec::new();
        
        // 関数の長さをチェック
        let mut in_function = false;
        let mut function_start_line = 0;
        let mut function_name = String::new();
        let mut brace_count = 0;
        
        for (i, line) in lines.iter().enumerate() {
            // 関数の開始を検出
            if !in_function && line.contains("fn ") && line.contains("(") {
                in_function = true;
                function_start_line = i;
                if let Some(name_start) = line.find("fn ") {
                    let rest = &line[name_start + 3..];
                    if let Some(name_end) = rest.find('(') {
                        function_name = rest[..name_end].trim().to_string();
                    }
                }
            }
            
            // 波括弧のカウント
            if in_function {
                brace_count += line.chars().filter(|&c| c == '{').count();
                brace_count -= line.chars().filter(|&c| c == '}').count();
                
                // 関数の終了を検出
                if brace_count == 0 && line.contains("}") {
                    let function_length = i - function_start_line + 1;
                    
                    // 長い関数を報告
                    if function_length > 50 {
                        violations.push(RuleViolation {
                            rule_id: format!("{}:function_length", self.id()),
                            message: format!("関数 '{}' が長すぎます（{}行）。50行以下にすることを検討してください", function_name, function_length),
                            line: function_start_line + 1,
                            column: 1,
                            severity: Severity::Warning,
                            suggestions: vec![
                                "関数を複数の小さな関数に分割することを検討してください".to_string(),
                                "関数の責任を単一の目的に制限してください".to_string(),
                            ],
                        });
                    }
                    
                    in_function = false;
                    function_name.clear();
                }
            }
        }
        
        // コードの複雑さをチェック（ネストの深さなど）
        for (i, line) in lines.iter().enumerate() {
            // インデントのレベルでネストの深さを推定（簡易版）
            let indent_level = line.len() - line.trim_start().len();
            if indent_level >= 24 { // 6レベル以上のネスト（4スペースインデント）
                violations.push(RuleViolation {
                    rule_id: format!("{}:nesting_depth", self.id()),
                    message: "コードのネストが深すぎます".to_string(),
                    line: i + 1,
                    column: 1,
                    severity: Severity::Warning,
                    suggestions: vec![
                        "早期リターンを使用してネストを減らすことを検討してください".to_string(),
                        "ネストされた条件を関数に抽出することを検討してください".to_string(),
                    ],
                });
            }
        }
        
        violations
    }
    
    fn id(&self) -> &str {
        &self.id
    }
    
    fn description(&self) -> &str {
        &self.description
    }
}

// ヘルパー関数

/// 指定された行がループ内にあるかどうかを判断する（簡易版）
fn is_in_loop(lines: &[&str], line_idx: usize) -> bool {
    // 現在の行の前にループキーワードがあるかをチェック
    for i in 0..=line_idx {
        if lines[i].contains("for ") || lines[i].contains("while ") {
            return true;
        }
    }
    false
} 