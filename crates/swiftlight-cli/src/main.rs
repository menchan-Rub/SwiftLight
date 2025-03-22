/*
 * SwiftLight CLI - メインエントリーポイント
 *
 * SwiftLight言語のコンパイラCLIツールのエントリーポイントです。
 * コマンドライン引数の解析と処理ロジックを呼び出します。
 * 
 * SwiftLightは高性能で安全なシステムプログラミング言語であり、
 * 並列処理、型安全性、メモリ安全性を重視しています。
 */

use clap::Parser;
use std::time::Instant;
use std::process;
use log::{info, error, LevelFilter};
use env_logger::Builder;

mod cli;
mod diagnostics;
mod profiler;
mod config;
mod utils;

fn main() -> anyhow::Result<()> {
    // ロギングの初期化
    let mut builder = Builder::new();
    builder.filter_level(LevelFilter::Info);
    builder.init();
    
    info!("SwiftLight コンパイラ v0.1.0 を起動しています");
    
    // プロファイリング開始
    let start_time = Instant::now();
    let profiler = profiler::Profiler::new();
    profiler.start();
    
    // コマンドライン引数の解析
    let cli = cli::Cli::parse();
    
    // 設定ファイルの読み込み
    let config = match config::load_config(&cli.config_path) {
        Ok(cfg) => cfg,
        Err(e) => {
            error!("設定ファイルの読み込みに失敗しました: {}", e);
            if cli.strict_mode {
                process::exit(1);
            }
            config::Config::default()
        }
    };
    
    // 診断ハンドラの設定
    let diagnostics = diagnostics::DiagnosticsHandler::new(
        cli.color_output, 
        cli.verbose,
        cli.error_format.clone()
    );
    
    // コンパイラ処理の実行
    let result = cli::run_compiler(&cli, &config, &diagnostics);
    
    // プロファイリング終了と結果表示
    profiler.stop();
    let elapsed = start_time.elapsed();
    
    if cli.show_timing {
        info!("コンパイル完了: {:.2}秒", elapsed.as_secs_f64());
        if cli.verbose {
            profiler.print_report();
        }
    }
    
    // 結果の返却
    match result {
        Ok(_) => {
            info!("コンパイルが正常に完了しました");
            Ok(())
        },
        Err(e) => {
            error!("コンパイルエラー: {}", e);
            if cli.fail_fast {
                process::exit(1);
            }
            Err(e)
        }
    }
}
