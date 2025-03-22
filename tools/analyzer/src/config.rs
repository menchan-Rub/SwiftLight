// 設定モジュール
// コマンドライン引数の解析と設定の管理を行います

use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct Config {
    pub show_help: bool,
    pub show_version: bool,
    pub config_file: Option<PathBuf>,
    pub rules: Vec<String>,
    pub ignore_patterns: Vec<String>,
    pub output_format: OutputFormat,
    pub output_file: Option<PathBuf>,
    pub min_severity: Severity,
    pub files: Vec<PathBuf>,
    pub directories: Vec<PathBuf>,
    pub force: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OutputFormat {
    Text,
    Json,
    Xml,
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub enum Severity {
    Info,
    Warning,
    Error,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            show_help: false,
            show_version: false,
            config_file: None,
            rules: vec!["all".to_string()],
            ignore_patterns: vec![],
            output_format: OutputFormat::Text,
            output_file: None,
            min_severity: Severity::Warning,
            files: vec![],
            directories: vec![],
            force: false,
        }
    }
}

pub fn parse_args(args: &[String]) -> Result<Config, String> {
    let mut config = Config::default();
    let mut i = 0;
    
    while i < args.len() {
        let arg = &args[i];
        
        match arg.as_str() {
            "-h" | "--help" => {
                config.show_help = true;
                return Ok(config);
            },
            "-v" | "--version" => {
                config.show_version = true;
                return Ok(config);
            },
            "-c" | "--config" => {
                i += 1;
                if i >= args.len() {
                    return Err("設定ファイルが指定されていません".to_string());
                }
                let config_path = PathBuf::from(&args[i]);
                if !config_path.exists() {
                    return Err(format!("指定された設定ファイルが存在しません: {}", config_path.display()));
                }
                config.config_file = Some(config_path);
                
                // 設定ファイルからの読み込み処理を実装する
                // ...
            },
            "-r" | "--rules" => {
                i += 1;
                if i >= args.len() {
                    return Err("ルールが指定されていません".to_string());
                }
                config.rules = args[i].split(',').map(|s| s.trim().to_string()).collect();
            },
            "-i" | "--ignore" => {
                i += 1;
                if i >= args.len() {
                    return Err("無視パターンが指定されていません".to_string());
                }
                config.ignore_patterns = args[i].split(',').map(|s| s.trim().to_string()).collect();
            },
            "-f" | "--format" => {
                i += 1;
                if i >= args.len() {
                    return Err("出力フォーマットが指定されていません".to_string());
                }
                config.output_format = match args[i].to_lowercase().as_str() {
                    "text" => OutputFormat::Text,
                    "json" => OutputFormat::Json,
                    "xml" => OutputFormat::Xml,
                    _ => return Err(format!("不明な出力フォーマット: {}", args[i])),
                };
            },
            "-o" | "--output" => {
                i += 1;
                if i >= args.len() {
                    return Err("出力ファイルが指定されていません".to_string());
                }
                config.output_file = Some(PathBuf::from(&args[i]));
            },
            "-s" | "--severity" => {
                i += 1;
                if i >= args.len() {
                    return Err("重大度レベルが指定されていません".to_string());
                }
                config.min_severity = match args[i].to_lowercase().as_str() {
                    "info" => Severity::Info,
                    "warning" => Severity::Warning,
                    "error" => Severity::Error,
                    _ => return Err(format!("不明な重大度レベル: {}", args[i])),
                };
            },
            "--force" => {
                config.force = true;
            },
            _ => {
                // ファイルまたはディレクトリと解釈
                let path = PathBuf::from(arg);
                if !path.exists() {
                    return Err(format!("指定されたパスが存在しません: {}", path.display()));
                }
                
                if path.is_file() {
                    config.files.push(path);
                } else if path.is_dir() {
                    config.directories.push(path);
                } else {
                    return Err(format!("指定されたパスはファイルでもディレクトリでもありません: {}", path.display()));
                }
            }
        }
        
        i += 1;
    }
    
    // 設定ファイルがある場合は読み込み
    if let Some(config_file) = &config.config_file {
        load_config_file(config_file, &mut config)?;
    }
    
    Ok(config)
}

fn load_config_file(path: &Path, config: &mut Config) -> Result<(), String> {
    // 設定ファイルの読み込み処理
    // 実際の実装ではJSONやTOMLなどのフォーマットを解析する
    
    use std::collections::HashMap;
    use std::str::FromStr;
    use toml::Value;
    
    match fs::read_to_string(path) {
        Ok(content) => {
            // TOMLとしてパース
            let toml_content = content.parse::<Value>().map_err(|e| {
                format!("TOMLの解析エラー: {}", e)
            })?;
            
            // 設定ファイルのデータを解析して設定を更新
            if let Some(rules) = toml_content.get("rules").and_then(|v| v.as_array()) {
                let rule_names: Vec<String> = rules
                    .iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect();
                
                if !rule_names.is_empty() {
                    config.rules = rule_names;
                }
            }
            
            if let Some(ignore) = toml_content.get("ignore").and_then(|v| v.as_array()) {
                let ignore_patterns: Vec<String> = ignore
                    .iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect();
                
                if !ignore_patterns.is_empty() {
                    config.ignore_patterns = ignore_patterns;
                }
            }
            
            if let Some(format) = toml_content.get("output_format").and_then(|v| v.as_str()) {
                match format.to_lowercase().as_str() {
                    "text" => config.output_format = OutputFormat::Text,
                    "json" => config.output_format = OutputFormat::Json,
                    "xml" => config.output_format = OutputFormat::Xml,
                    _ => return Err(format!("不明な出力フォーマット: {}", format)),
                }
            }
            
            if let Some(output_file) = toml_content.get("output_file").and_then(|v| v.as_str()) {
                config.output_file = Some(PathBuf::from(output_file));
            }
            
            if let Some(severity) = toml_content.get("min_severity").and_then(|v| v.as_str()) {
                match severity.to_lowercase().as_str() {
                    "info" => config.min_severity = Severity::Info,
                    "warning" => config.min_severity = Severity::Warning,
                    "error" => config.min_severity = Severity::Error,
                    _ => return Err(format!("不明な重大度レベル: {}", severity)),
                }
            }
            
            if let Some(force) = toml_content.get("force").and_then(|v| v.as_bool()) {
                config.force = force;
            }
            
            // ディレクトリとファイルの設定
            if let Some(dirs) = toml_content.get("directories").and_then(|v| v.as_array()) {
                let directories: Vec<PathBuf> = dirs
                    .iter()
                    .filter_map(|v| v.as_str().map(PathBuf::from))
                    .collect();
                
                if !directories.is_empty() {
                    config.directories = directories;
                }
            }
            
            if let Some(files) = toml_content.get("files").and_then(|v| v.as_array()) {
                let file_paths: Vec<PathBuf> = files
                    .iter()
                    .filter_map(|v| v.as_str().map(PathBuf::from))
                    .collect();
                
                if !file_paths.is_empty() {
                    config.files = file_paths;
                }
            }
            
            println!("設定ファイルを読み込みました: {}", path.display());
            Ok(())
        },
        Err(err) => {
            Err(format!("設定ファイルの読み込みに失敗しました: {}: {}", path.display(), err))
        }
    }
}