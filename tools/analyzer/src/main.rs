// SwiftLight 静的解析ツール
// このツールはSwiftLightコードの静的解析を行い、問題を検出します

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process;

mod analyzer;
mod diagnostics;
mod rules;
mod config;

fn main() {
    println!("SwiftLight 静的解析ツール v0.1.0");
    
    let args: Vec<String> = env::args().collect();
    
    if args.len() < 2 {
        print_usage();
        process::exit(1);
    }
    
    let config = match config::parse_args(&args[1..]) {
        Ok(config) => config,
        Err(err) => {
            eprintln!("引数解析エラー: {}", err);
            print_usage();
            process::exit(1);
        }
    };
    
    if config.show_help {
        print_usage();
        return;
    }
    
    if config.show_version {
        println!("SwiftLight 静的解析ツール v0.1.0");
        return;
    }
    
    if config.files.is_empty() && config.directories.is_empty() {
        eprintln!("エラー: 分析するファイルまたはディレクトリを指定してください");
        print_usage();
        process::exit(1);
    }
    
    // 個別ファイルの分析
    for file_path in &config.files {
        analyze_file(file_path, &config);
    }
    
    // ディレクトリの分析
    for dir_path in &config.directories {
        analyze_directory(dir_path, &config);
    }
    
    println!("分析が完了しました。");
}

fn print_usage() {
    println!("使用方法: swiftlight-analyzer [オプション] <ファイル/ディレクトリ...>");
    println!();
    println!("オプション:");
    println!("  -h, --help                 このヘルプメッセージを表示");
    println!("  -v, --version              バージョン情報を表示");
    println!("  -c, --config <ファイル>     設定ファイルを指定");
    println!("  -r, --rules <ルール>        有効にするルールをカンマ区切りで指定");
    println!("  -i, --ignore <パターン>     無視するファイルパターンを指定");
    println!("  -f, --format <フォーマット> 出力フォーマットを指定 (text, json, xml)");
    println!("  -o, --output <ファイル>     出力ファイルを指定");
    println!("  -s, --severity <レベル>     最小の重大度レベルを指定 (info, warning, error)");
    println!();
    println!("例:");
    println!("  swiftlight-analyzer src/main.swl");
    println!("  swiftlight-analyzer --rules=performance,security src/");
    println!("  swiftlight-analyzer --format=json --output=report.json src/");
}

fn analyze_file(file_path: &Path, config: &config::Config) {
    println!("ファイル分析: {}", file_path.display());
    
    if !file_path.exists() {
        eprintln!("エラー: ファイルが存在しません: {}", file_path.display());
        return;
    }
    
    if !file_path.extension().map_or(false, |ext| ext == "swl") {
        eprintln!("警告: SwiftLightファイル(.swl)ではありません: {}", file_path.display());
        if !config.force {
            return;
        }
    }
    
    match fs::read_to_string(file_path) {
        Ok(content) => {
            let diagnostics = analyzer::analyze_code(&content, file_path, config);
            diagnostics::report_diagnostics(&diagnostics, config);
        },
        Err(err) => {
            eprintln!("エラー: ファイル読み込み失敗: {}: {}", file_path.display(), err);
        }
    }
}

fn analyze_directory(dir_path: &Path, config: &config::Config) {
    println!("ディレクトリ分析: {}", dir_path.display());
    
    if !dir_path.exists() || !dir_path.is_dir() {
        eprintln!("エラー: ディレクトリが存在しません: {}", dir_path.display());
        return;
    }
    
    match fs::read_dir(dir_path) {
        Ok(entries) => {
            for entry in entries {
                if let Ok(entry) = entry {
                    let path = entry.path();
                    if path.is_file() && path.extension().map_or(false, |ext| ext == "swl") {
                        analyze_file(&path, config);
                    } else if path.is_dir() && !is_ignored(&path, config) {
                        analyze_directory(&path, config);
                    }
                }
            }
        },
        Err(err) => {
            eprintln!("エラー: ディレクトリ読み込み失敗: {}: {}", dir_path.display(), err);
        }
    }
}

fn is_ignored(path: &Path, config: &config::Config) -> bool {
    // 無視パターンのチェック
    for pattern in &config.ignore_patterns {
        if path.to_string_lossy().contains(pattern) {
            return true;
        }
    }
    
    false
} 