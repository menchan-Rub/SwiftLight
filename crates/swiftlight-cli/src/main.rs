/*
 * SwiftLight CLI - メインエントリーポイント
 *
 * SwiftLight言語のコンパイラCLIツールのエントリーポイントです。
 * コマンドライン引数の解析と処理ロジックを呼び出します。
 */

use clap::Parser;

mod cli;

fn main() -> anyhow::Result<()> {
    // コマンドライン引数の解析
    let cli = cli::Cli::parse();
    
    // コンパイラ処理の実行
    cli::run_compiler(&cli)
}
