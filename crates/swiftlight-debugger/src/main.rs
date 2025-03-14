/*
 * SwiftLight デバッガー - メインエントリーポイント
 *
 * このモジュールは、SwiftLight言語のデバッガーのメインエントリーポイントを提供します。
 * コマンドライン引数の解析、デバッグサーバーの起動、クライアント接続の処理を行います。
 */

mod protocol;
mod engine;

use anyhow::{Context, Result, anyhow};
use clap::{Parser, Subcommand};
use colored::Colorize;
use log::{debug, info, warn, error, LevelFilter};
use std::path::{Path, PathBuf};
use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener as AsyncTcpListener;
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use tokio::task;
use env_logger::Builder;
use serde_json::json;
use protocol::{DebugConfiguration, DebugSession, DapProtocolHandler, StopReason};
use engine::{DebugEngine, DebugCommand, DebugEvent, ExecutionMode, TimeTravelCommand};
use std::collections::HashMap;

/// SwiftLight デバッガーのコマンドライン引数
#[derive(Parser, Debug)]
#[clap(author, version, about = "SwiftLight言語のデバッガー")]
struct Cli {
    /// 詳細なログ出力を有効にする
    #[clap(short, long, default_value = "false")]
    verbose: bool,

    /// ログ出力を抑制する
    #[clap(short, long, default_value = "false")]
    quiet: bool,

    /// デバッグサーバーのポート
    #[clap(short, long, default_value = "9229")]
    port: u16,

    /// デバッグサーバーのホスト
    #[clap(short, long, default_value = "127.0.0.1")]
    host: String,

    /// サブコマンド
    #[clap(subcommand)]
    command: Commands,
}

/// SwiftLightデバッガーのサブコマンド
#[derive(Subcommand, Debug)]
enum Commands {
    /// プログラムを起動してデバッグ
    Launch {
        /// デバッグ対象の実行ファイルパス
        #[clap(required = true)]
        program: PathBuf,

        /// 作業ディレクトリ
        #[clap(short, long)]
        cwd: Option<PathBuf>,

        /// プログラム引数
        #[clap(last = true)]
        args: Vec<String>,
    },

    /// 既存プロセスにアタッチしてデバッグ
    Attach {
        /// プロセスID
        #[clap(short, long)]
        pid: Option<u32>,

        /// リモートデバッグの場合のホスト
        #[clap(short, long)]
        host: Option<String>,

        /// リモートデバッグの場合のポート
        #[clap(short, long)]
        port: Option<u16>,
    },

    /// DAP（Debug Adapter Protocol）サーバーを起動
    Server {
        /// サーバーモード（stdio または tcp）
        #[clap(short, long, default_value = "stdio")]
        mode: String,
    },

    /// タイムトラベルデバッグ関連コマンド
    TimeTravel {
        /// タイムトラベルサブコマンド
        #[clap(subcommand)]
        command: TimeTravelCommands,
    },
}

#[derive(Subcommand, Debug)]
enum TimeTravelCommands {
    /// タイムトラベルデバッグを有効化
    Enable,
    
    /// タイムトラベルデバッグを無効化
    Disable,
    
    /// 現在の状態を記録
    Record {
        /// スナップショットの注釈（オプション）
        #[clap(short, long)]
        annotation: Option<String>,
    },
    
    /// 指定したスナップショットに移動
    Goto {
        /// スナップショットID
        #[clap(required = true)]
        snapshot_id: usize,
    },
    
    /// 前のスナップショットに移動
    Back,
    
    /// 次のスナップショットに移動
    Forward,
    
    /// スナップショットの一覧を表示
    List,
    
    /// スナップショットに注釈を追加
    Annotate {
        /// スナップショットID
        #[clap(required = true)]
        snapshot_id: usize,
        
        /// 注釈
        #[clap(required = true)]
        annotation: String,
    },
}

/// デバッガーのメイン関数
#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // ログレベルの設定
    setup_logging(cli.verbose, cli.quiet);

    info!("SwiftLightデバッガーを起動中...");

    // サブコマンドに基づいて処理を分岐
    match &cli.command {
        Commands::Launch { program, cwd, args } => {
            launch_program(program, cwd.as_deref(), args, cli.host, cli.port).await
        }
        Commands::Attach { pid, host, port } => {
            attach_to_process(*pid, host.clone(), *port, cli.host, cli.port).await
        }
        Commands::Server { mode } => {
            run_debug_server(mode, cli.host, cli.port).await
        }
        Commands::TimeTravel { command } => {
            handle_time_travel_command(command, cli.host, cli.port).await?;
        }
    }

    Ok(())
}

/// ロギングの設定
fn setup_logging(verbose: bool, quiet: bool) {
    let mut builder = Builder::new();

    // ログレベルを設定
    let level = if quiet {
        LevelFilter::Error
    } else if verbose {
        LevelFilter::Debug
    } else {
        LevelFilter::Info
    };

    builder.filter_level(level);
    builder.format_timestamp_secs();
    builder.init();
}

/// プログラムを起動してデバッグする
async fn launch_program(
    program: &Path,
    cwd: Option<&Path>,
    args: &[String],
    host: String,
    port: u16,
) -> Result<()> {
    info!("デバッグ対象プログラムを起動: {}", program.display());

    // プログラムが存在するか確認
    if !program.exists() {
        return Err(anyhow!("プログラムが見つかりません: {}", program.display()));
    }

    // デバッグ設定を構築
    let config = DebugConfiguration {
        name: format!("SwiftLight Debug: {}", program.file_name().unwrap_or_default().to_string_lossy()),
        type_name: "swiftlight".to_string(),
        request: "launch".to_string(),
        program: program.to_path_buf(),
        cwd: cwd.map(|p| p.to_path_buf()),
        args: args.to_vec(),
        env: std::env::vars().collect(),
        address: Some(host),
        port: Some(port),
        source_map: Default::default(),
        stop_on_entry: true,
    };

    // デバッグエンジンを使用して起動
    let session = Arc::new(Mutex::new(DebugSession::new(config.clone())));
    
    // チャネルを作成
    let (command_tx, command_rx) = mpsc::channel(100);
    let (event_tx, mut event_rx) = mpsc::channel(100);
    
    // エンジンを別タスクで実行
    let mut engine = DebugEngine::new(session.clone(), command_rx, event_tx.clone());
    let engine_task = tokio::spawn(async move {
        if let Err(e) = engine.run().await {
            error!("デバッグエンジンエラー: {}", e);
        }
    });
    
    // 初期化コマンドを送信
    let (resp_tx, resp_rx) = oneshot::channel();
    command_tx.send(DebugCommand::Initialize { response: resp_tx }).await
        .map_err(|_| anyhow!("コマンド送信失敗"))?;
    resp_rx.await.map_err(|_| anyhow!("レスポンス受信失敗"))??;
    
    // 起動コマンドを送信
    let (resp_tx, resp_rx) = oneshot::channel();
    command_tx.send(DebugCommand::Launch { config, response: resp_tx }).await
        .map_err(|_| anyhow!("コマンド送信失敗"))?;
    resp_rx.await.map_err(|_| anyhow!("レスポンス受信失敗"))??;
    
    println!("{}", "== デバッグセッション開始 ==".green());
    println!("プログラム: {}", program.display());
    if let Some(cwd) = cwd {
        println!("作業ディレクトリ: {}", cwd.display());
    }
    if !args.is_empty() {
        println!("引数: {:?}", args);
    }
    
    // イベントを処理するループ
    while let Some(event) = event_rx.recv().await {
        match event {
            DebugEvent::Initialized => {
                println!("デバッガーが初期化されました");
            }
            DebugEvent::Stopped { thread_id, reason } => {
                println!("プログラムが停止しました: スレッド={}, 理由={:?}", thread_id, reason);
                
                // スタックトレースを取得して表示
                let (resp_tx, resp_rx) = oneshot::channel();
                command_tx.send(DebugCommand::GetStackTrace { thread_id, response: resp_tx }).await
                    .map_err(|_| anyhow!("コマンド送信失敗"))?;
                
                if let Ok(Ok(frames)) = resp_rx.await {
                    println!("{}", "== スタックトレース ==".cyan());
                    for frame in &frames {
                        println!("#{} {} ({}:{})", frame.id, frame.name, frame.source_path.display(), frame.line);
                    }
                    
                    // 現在の位置のソースコードを表示
                    if let Some(frame) = frames.first() {
                        show_source_with_location(&frame.source_path, frame.line, 3)?;
                    }
                }
                
                // コマンドプロンプトを表示して次のコマンドを待機
                print!("{}", "\nコマンド (c=継続, n=次へ, s=ステップイン, o=ステップアウト, q=終了): ".yellow());
                std::io::stdout().flush().unwrap();
                
                // ユーザー入力を待機
                let mut input = String::new();
                std::io::stdin().read_line(&mut input)?;
                
                match input.trim() {
                    "c" => {
                        // 継続実行
                        let (resp_tx, resp_rx) = oneshot::channel();
                        command_tx.send(DebugCommand::ExecutionControl { 
                            mode: ExecutionMode::Continue, 
                            thread_id, 
                            response: resp_tx 
                        }).await.map_err(|_| anyhow!("コマンド送信失敗"))?;
                        let _ = resp_rx.await;
                    }
                    "n" => {
                        // ステップオーバー
                        let (resp_tx, resp_rx) = oneshot::channel();
                        command_tx.send(DebugCommand::ExecutionControl { 
                            mode: ExecutionMode::StepOver, 
                            thread_id, 
                            response: resp_tx 
                        }).await.map_err(|_| anyhow!("コマンド送信失敗"))?;
                        let _ = resp_rx.await;
                    }
                    "s" => {
                        // ステップイン
                        let (resp_tx, resp_rx) = oneshot::channel();
                        command_tx.send(DebugCommand::ExecutionControl { 
                            mode: ExecutionMode::StepIn, 
                            thread_id, 
                            response: resp_tx 
                        }).await.map_err(|_| anyhow!("コマンド送信失敗"))?;
                        let _ = resp_rx.await;
                    }
                    "o" => {
                        // ステップアウト
                        let (resp_tx, resp_rx) = oneshot::channel();
                        command_tx.send(DebugCommand::ExecutionControl { 
                            mode: ExecutionMode::StepOut, 
                            thread_id, 
                            response: resp_tx 
                        }).await.map_err(|_| anyhow!("コマンド送信失敗"))?;
                        let _ = resp_rx.await;
                    }
                    "q" => {
                        // 終了
                        break;
                    }
                    _ => {
                        println!("不明なコマンドです");
                    }
                }
            }
            DebugEvent::Continued { thread_id, all_threads } => {
                println!("プログラムが再開されました: スレッド={}, 全スレッド={}", thread_id, all_threads);
            }
            DebugEvent::Exited { exit_code } => {
                println!("プログラムが終了しました: 終了コード={}", exit_code);
                break;
            }
            DebugEvent::Output { category, output } => {
                match category.as_str() {
                    "stdout" => println!("{}", output),
                    "stderr" => eprintln!("{}", output.red()),
                    _ => println!("[{}] {}", category, output),
                }
            }
            DebugEvent::ThreadStarted { thread_id } => {
                println!("スレッドが開始されました: id={}", thread_id);
            }
            DebugEvent::ThreadExited { thread_id } => {
                println!("スレッドが終了しました: id={}", thread_id);
            }
            DebugEvent::BreakpointChanged { breakpoint } => {
                println!("ブレークポイントが変更されました: id={}, パス={}, 行={}", 
                       breakpoint.id, breakpoint.source_path.display(), breakpoint.line);
            }
        }
    }
    
    // 切断コマンドを送信して終了
    let (resp_tx, resp_rx) = oneshot::channel();
    command_tx.send(DebugCommand::Disconnect { terminate: true, response: resp_tx }).await
        .map_err(|_| anyhow!("コマンド送信失敗"))?;
    let _ = resp_rx.await;
    
    // エンジンタスクが終了するのを待つ
    let _ = engine_task.await;
    
    println!("{}", "== デバッグセッション終了 ==".green());
    
    Ok(())
}

/// 既存プロセスにアタッチしてデバッグする
async fn attach_to_process(
    pid: Option<u32>,
    host: Option<String>,
    port: Option<u16>,
    server_host: String,
    server_port: u16,
) -> Result<()> {
    if let Some(pid) = pid {
        info!("プロセスにアタッチ: PID={}", pid);
        
        // デバッグ設定を構築
        let config = DebugConfiguration {
            name: format!("SwiftLight Debug: PID {}", pid),
            type_name: "swiftlight".to_string(),
            request: "attach".to_string(),
            program: PathBuf::new(), // アタッチの場合はプログラムパスは不要
            cwd: None,
            args: Vec::new(),
            env: std::env::vars().collect(),
            address: Some(server_host),
            port: Some(server_port),
            source_map: Default::default(),
            stop_on_entry: false,
        };
        
        // デバッグエンジンを使用してアタッチ
        let session = Arc::new(Mutex::new(DebugSession::new(config.clone())));
        
        // チャネルを作成
        let (command_tx, command_rx) = mpsc::channel(100);
        let (event_tx, mut event_rx) = mpsc::channel(100);
        
        // エンジンを別タスクで実行
        let mut engine = DebugEngine::new(session.clone(), command_rx, event_tx.clone());
        let engine_task = tokio::spawn(async move {
            if let Err(e) = engine.run().await {
                error!("デバッグエンジンエラー: {}", e);
            }
        });
        
        // 初期化コマンドを送信
        let (resp_tx, resp_rx) = oneshot::channel();
        command_tx.send(DebugCommand::Initialize { response: resp_tx }).await
            .map_err(|_| anyhow!("コマンド送信失敗"))?;
        resp_rx.await.map_err(|_| anyhow!("レスポンス受信失敗"))??;
        
        // アタッチコマンドを送信
        let (resp_tx, resp_rx) = oneshot::channel();
        command_tx.send(DebugCommand::Attach { config, pid: Some(pid), response: resp_tx }).await
            .map_err(|_| anyhow!("コマンド送信失敗"))?;
        resp_rx.await.map_err(|_| anyhow!("レスポンス受信失敗"))??;
        
        println!("{}", "== デバッグセッション開始 (アタッチモード) ==".green());
        println!("PID: {}", pid);
        
        // イベントを処理するループ (launch_programと同様)
        // 簡略化のため、詳細なイベント処理は省略
        
        // 切断コマンドを送信して終了
        let (resp_tx, resp_rx) = oneshot::channel();
        command_tx.send(DebugCommand::Disconnect { terminate: false, response: resp_tx }).await
            .map_err(|_| anyhow!("コマンド送信失敗"))?;
        let _ = resp_rx.await;
        
        // エンジンタスクが終了するのを待つ
        let _ = engine_task.await;
        
        println!("{}", "== デバッグセッション終了 ==".green());
        
    } else if let (Some(host), Some(port)) = (host, port) {
        info!("リモートプロセスにアタッチ: {}:{}", host, port);
        println!("{}", "リモートアタッチ機能は現在開発中です。".yellow());
    } else {
        return Err(anyhow!("PIDまたはホスト/ポートを指定してください"));
    }

    Ok(())
}

/// DAP（Debug Adapter Protocol）サーバーを起動
async fn run_debug_server(mode: &str, host: String, port: u16) -> Result<()> {
    match mode {
        "stdio" => run_dap_stdio_server().await,
        "tcp" => run_dap_tcp_server(&host, port).await,
        _ => Err(anyhow!("不明なサーバーモード: {}", mode)),
    }
}

/// 標準入出力を使ったDAPサーバー
async fn run_dap_stdio_server() -> Result<()> {
    info!("標準入出力モードでDAPサーバーを起動");
    
    // セッションを作成
    let session = Arc::new(Mutex::new(DebugSession::default()));
    
    // チャネルを作成
    let (command_tx, command_rx) = mpsc::channel(100);
    let (event_tx, mut event_rx) = mpsc::channel(100);
    
    // エンジンを別タスクで実行
    let mut engine = DebugEngine::new(session.clone(), command_rx, event_tx.clone());
    let engine_task = tokio::spawn(async move {
        if let Err(e) = engine.run().await {
            error!("デバッグエンジンエラー: {}", e);
        }
    });

    // 非同期チャネルを作成
    let (msg_tx, mut msg_rx) = mpsc::channel(100);

    // 標準入力を読み取るタスク
    task::spawn(async move {
        let mut buffer = [0u8; 4096];
        let mut stdin = tokio::io::stdin();
        
        loop {
            match stdin.read(&mut buffer).await {
                Ok(n) if n > 0 => {
                    if let Err(e) = msg_tx.send(buffer[..n].to_vec()).await {
                        error!("メッセージ送信エラー: {}", e);
                        break;
                    }
                }
                Ok(_) => break, // EOF
                Err(e) => {
                    error!("標準入力の読み取りエラー: {}", e);
                    break;
                }
            }
        }
    });

    // DAP プロトコルハンドラーを作成
    let dap_handler = Arc::new(Mutex::new(DapProtocolHandler::new(session.lock().unwrap().clone())));
    
    // イベント処理タスク
    let dap_handler_clone = dap_handler.clone();
    let event_task = tokio::spawn(async move {
        while let Some(event) = event_rx.recv().await {
            // イベントをDAPイベントに変換して送信
            let dap_event = match event {
                DebugEvent::Initialized => {
                    Some(dap_handler_clone.lock().unwrap().create_event("initialized", json!({})))
                }
                DebugEvent::Stopped { thread_id, reason } => {
                    let reason_str = match &reason {
                        StopReason::Breakpoint { .. } => "breakpoint",
                        StopReason::Step => "step",
                        StopReason::Exception { .. } => "exception",
                        StopReason::Pause => "pause",
                        StopReason::Entry => "entry",
                        StopReason::Exit { .. } => "exit",
                    };
                    
                    Some(dap_handler_clone.lock().unwrap().create_event("stopped", json!({
                        "reason": reason_str,
                        "threadId": thread_id,
                        "allThreadsStopped": true
                    })))
                }
                DebugEvent::Continued { thread_id, all_threads } => {
                    Some(dap_handler_clone.lock().unwrap().create_event("continued", json!({
                        "threadId": thread_id,
                        "allThreadsContinued": all_threads
                    })))
                }
                DebugEvent::Exited { exit_code } => {
                    Some(dap_handler_clone.lock().unwrap().create_event("exited", json!({
                        "exitCode": exit_code
                    })))
                }
                DebugEvent::Output { category, output } => {
                    Some(dap_handler_clone.lock().unwrap().create_event("output", json!({
                        "category": category,
                        "output": output
                    })))
                }
                DebugEvent::ThreadStarted { thread_id } => {
                    Some(dap_handler_clone.lock().unwrap().create_event("thread", json!({
                        "reason": "started",
                        "threadId": thread_id
                    })))
                }
                DebugEvent::ThreadExited { thread_id } => {
                    Some(dap_handler_clone.lock().unwrap().create_event("thread", json!({
                        "reason": "exited",
                        "threadId": thread_id
                    })))
                }
                DebugEvent::BreakpointChanged { breakpoint } => {
                    Some(dap_handler_clone.lock().unwrap().create_event("breakpoint", json!({
                        "reason": "changed",
                        "breakpoint": {
                            "id": breakpoint.id,
                            "verified": breakpoint.verified,
                            "source": {
                                "path": breakpoint.source_path.to_string_lossy()
                            },
                            "line": breakpoint.line,
                            "column": breakpoint.column
                        }
                    })))
                }
                DebugEvent::TimeTravelStateChanged { enabled, direction, current_snapshot_id } => {
                    let direction_str = match direction {
                        engine::TimeDirection::Forward => "forward",
                        engine::TimeDirection::Backward => "backward",
                        engine::TimeDirection::Paused => "paused",
                    };
                    
                    dap_handler_clone.lock().unwrap().create_event("timeTravelStateChanged", json!({
                        "enabled": enabled,
                        "direction": direction_str,
                        "currentSnapshotId": current_snapshot_id
                    }))
                }
                DebugEvent::SnapshotNavigated { snapshot_id, timestamp, source_location, function_name } => {
                    let (source_path, line, column) = source_location;
                    
                    dap_handler_clone.lock().unwrap().create_event("snapshotNavigated", json!({
                        "snapshotId": snapshot_id,
                        "timestamp": timestamp,
                        "source": {
                            "path": source_path,
                            "line": line,
                            "column": column
                        },
                        "functionName": function_name
                    }))
                }
            };
            
            if let Some(event) = dap_event {
                let response_data = serde_json::to_vec(&event).unwrap();
                let mut stdout = tokio::io::stdout();
                stdout.write_all(&response_data).await.unwrap();
                stdout.flush().await.unwrap();
            }
        }
    });

    // メインループ
    let mut stdout = tokio::io::stdout();
    while let Some(data) = msg_rx.recv().await {
        // 受信データをJSONとしてパース
        if let Ok(request) = serde_json::from_slice::<serde_json::Value>(&data) {
            debug!("受信したリクエスト: {:?}", request);

            // リクエストを処理
            match dap_handler.lock().unwrap().handle_request(request) {
                Ok(response) => {
                    let response_data = serde_json::to_vec(&response)?;
                    stdout.write_all(&response_data).await?;
                    stdout.flush().await?;
                }
                Err(e) => {
                    error!("リクエスト処理エラー: {}", e);
                    // エラーレスポンスを送信
                    let error_response = serde_json::json!({
                        "type": "response",
                        "success": false,
                        "message": format!("エラー: {}", e)
                    });
                    let response_data = serde_json::to_vec(&error_response)?;
                    stdout.write_all(&response_data).await?;
                    stdout.flush().await?;
                }
            }
        } else {
            error!("不正なJSON形式のリクエスト");
        }
    }
    
    // 切断コマンドを送信して終了
    let (resp_tx, resp_rx) = oneshot::channel();
    command_tx.send(DebugCommand::Disconnect { terminate: true, response: resp_tx }).await
        .map_err(|_| anyhow!("コマンド送信失敗"))?;
    let _ = resp_rx.await;
    
    // タスクが終了するのを待つ
    let _ = engine_task.await;
    let _ = event_task.await;

    Ok(())
}

/// TCPソケットを使ったDAPサーバー
async fn run_dap_tcp_server(host: &str, port: u16) -> Result<()> {
    let addr = format!("{}:{}", host, port);
    info!("TCPソケットモードでDAPサーバーを起動: {}", addr);

    let listener = AsyncTcpListener::bind(&addr).await
        .context(format!("{}:{} でのリッスンに失敗", host, port))?;

    println!("{}", format!("デバッグサーバーがポート {} で待機中...", port).green());

    loop {
        match listener.accept().await {
            Ok((stream, addr)) => {
                info!("クライアント接続: {}", addr);
                
                // 接続ごとに新しいタスクを作成
                tokio::spawn(async move {
                    if let Err(e) = handle_dap_client(stream).await {
                        error!("クライアント処理エラー: {}", e);
                    }
                });
            }
            Err(e) => {
                error!("接続の受け入れに失敗: {}", e);
            }
        }
    }
}

/// クライアント接続を処理（DAP）
async fn handle_dap_client(mut stream: tokio::net::TcpStream) -> Result<()> {
    debug!("クライアント接続を処理します");
    
    // 設定とセッションの初期化
    let session = DebugSession::default();
    let mut dap_handler = DapProtocolHandler::new(session);
    
    // 初期設定
    let config = DebugConfiguration {
        name: "SwiftLight Debug".to_string(),
        type_name: "swiftlight".to_string(),
        request: "launch".to_string(),
        program: PathBuf::new(), // 後で設定される
        cwd: None,
        args: Vec::new(),
        env: HashMap::new(),
        address: None,
        port: None,
        source_map: HashMap::new(),
        stop_on_entry: true,
    };
    
    // デバッグエンジンとセッションの作成
    let session = Arc::new(Mutex::new(dap_handler.session.clone()));
    let (command_tx, command_rx) = tokio::sync::mpsc::channel::<DebugCommand>(100);
    let (event_tx, mut event_rx) = tokio::sync::mpsc::channel::<DebugEvent>(100);
    
    let mut engine = engine::DebugEngine::new(session.clone(), command_rx, event_tx.clone());
    
    // デバッグエンジンを非同期で実行
    let engine_task = tokio::spawn(async move {
        if let Err(e) = engine.run().await {
            error!("デバッグエンジンの実行エラー: {}", e);
        }
    });
    
    // セッションの初期化
    let (resp_tx, resp_rx) = oneshot::channel();
    command_tx.send(DebugCommand::Initialize {
        response: resp_tx,
    }).await.map_err(|_| anyhow!("コマンド送信に失敗"))?;
    
    resp_rx.await.map_err(|_| anyhow!("応答の受信に失敗"))??;
    
    // バッファを使用して、メッセージの境界を正確に処理
    let (mut reader, mut writer) = stream.split();
    let mut buffer = Vec::new();
    let mut content_length = 0;
    let mut reading_headers = true;
    
    // DAP通信を非同期で処理
    loop {
        // イベント処理（イベントがあれば送信）
        tokio::select! {
            event = event_rx.recv() => {
                if let Some(event) = event {
                    let event_json = match event {
                        DebugEvent::Initialized => {
                            dap_handler.create_event("initialized", json!({}))
                        }
                        DebugEvent::Stopped { thread_id, reason } => {
                            let reason_str = match &reason {
                                StopReason::Breakpoint { id } => "breakpoint",
                                StopReason::Step => "step",
                                StopReason::Exception { description: _ } => "exception",
                                StopReason::Pause => "pause",
                                StopReason::Entry => "entry",
                                StopReason::Exit { exit_code: _ } => "exit",
                            };
                            
                            let mut body = json!({
                                "reason": reason_str,
                                "threadId": thread_id,
                                "allThreadsStopped": true
                            });
                            
                            if let StopReason::Exception { description } = &reason {
                                body["description"] = json!(description);
                            }
                            
                            dap_handler.create_event("stopped", body)
                        }
                        DebugEvent::Continued { thread_id, all_threads } => {
                            dap_handler.create_event("continued", json!({
                                "threadId": thread_id,
                                "allThreadsContinued": all_threads
                            }))
                        }
                        DebugEvent::Exited { exit_code } => {
                            dap_handler.create_event("exited", json!({
                                "exitCode": exit_code
                            }))
                        }
                        DebugEvent::Output { category, output } => {
                            dap_handler.create_event("output", json!({
                                "category": category,
                                "output": output
                            }))
                        }
                        DebugEvent::ThreadStarted { thread_id } => {
                            dap_handler.create_event("thread", json!({
                                "reason": "started",
                                "threadId": thread_id
                            }))
                        }
                        DebugEvent::ThreadExited { thread_id } => {
                            dap_handler.create_event("thread", json!({
                                "reason": "exited",
                                "threadId": thread_id
                            }))
                        }
                        DebugEvent::BreakpointChanged { breakpoint } => {
                            dap_handler.create_event("breakpoint", json!({
                                "reason": "changed",
                                "breakpoint": {
                                    "id": breakpoint.id,
                                    "verified": breakpoint.verified,
                                    "source": {
                                        "path": breakpoint.source_path.to_string_lossy()
                                    },
                                    "line": breakpoint.line,
                                    "column": breakpoint.column
                                }
                            }))
                        }
                        DebugEvent::TimeTravelStateChanged { enabled, direction, current_snapshot_id } => {
                            let direction_str = match direction {
                                engine::TimeDirection::Forward => "forward",
                                engine::TimeDirection::Backward => "backward",
                                engine::TimeDirection::Paused => "paused",
                            };
                            
                            dap_handler.create_event("timeTravelStateChanged", json!({
                                "enabled": enabled,
                                "direction": direction_str,
                                "currentSnapshotId": current_snapshot_id
                            }))
                        }
                        DebugEvent::SnapshotNavigated { snapshot_id, timestamp, source_location, function_name } => {
                            let (source_path, line, column) = source_location;
                            
                            dap_handler.create_event("snapshotNavigated", json!({
                                "snapshotId": snapshot_id,
                                "timestamp": timestamp,
                                "source": {
                                    "path": source_path,
                                    "line": line,
                                    "column": column
                                },
                                "functionName": function_name
                            }))
                        }
                    };
                    
                    let message = format!("Content-Length: {}\r\n\r\n{}", 
                                        event_json.to_string().len(), 
                                        event_json.to_string());
                    
                    if let Err(e) = writer.write_all(message.as_bytes()).await {
                        error!("イベント送信エラー: {}", e);
                        break;
                    }
                }
            },
            
            read_result = reader.read_u8() => {
                match read_result {
                    Ok(byte) => {
                        if reading_headers {
                            buffer.push(byte);
                            
                            // ヘッダーの終了を検出（空行）
                            if buffer.len() >= 4 && 
                               buffer[buffer.len()-4..] == [13, 10, 13, 10] {
                                
                                // Content-Lengthヘッダーを解析
                                let headers = String::from_utf8_lossy(&buffer);
                                for line in headers.lines() {
                                    if line.to_lowercase().starts_with("content-length:") {
                                        if let Some(len_str) = line.split(':').nth(1) {
                                            if let Ok(len) = len_str.trim().parse::<usize>() {
                                                content_length = len;
                                            }
                                        }
                                    }
                                }
                                
                                buffer.clear();
                                reading_headers = false;
                            }
                        } else {
                            buffer.push(byte);
                            
                            // メッセージ本文を完全に読み込んだかチェック
                            if buffer.len() >= content_length {
                                // JSONメッセージを解析
                                let message = String::from_utf8_lossy(&buffer);
                                match serde_json::from_str::<serde_json::Value>(&message) {
                                    Ok(request) => {
                                        // リクエストを処理
                                        match dap_handler.handle_request(request) {
                                            Ok(response) => {
                                                // DAP応答の処理（特別なコマンドの場合）
                                                if let Some(command) = response["command"].as_str() {
                                                    match command {
                                                        "launch" => {
                                                            if let Some(args) = response["arguments"].as_object() {
                                                                // プログラムパスを取得
                                                                let program = args.get("program")
                                                                    .and_then(|p| p.as_str())
                                                                    .map(PathBuf::from);
                                                                
                                                                if let Some(program) = program {
                                                                    // 新しい設定を構築
                                                                    let mut config = DebugConfiguration {
                                                                        name: "SwiftLight Debug".to_string(),
                                                                        type_name: "swiftlight".to_string(),
                                                                        request: "launch".to_string(),
                                                                        program,
                                                                        cwd: None,
                                                                        args: Vec::new(),
                                                                        env: HashMap::new(),
                                                                        address: None,
                                                                        port: None,
                                                                        source_map: HashMap::new(),
                                                                        stop_on_entry: true,
                                                                    };
                                                                    
                                                                    // 追加引数があれば設定
                                                                    if let Some(args) = args.get("args")
                                                                        .and_then(|a| a.as_array()) {
                                                                        config.args = args.iter()
                                                                            .filter_map(|a| a.as_str().map(String::from))
                                                                            .collect();
                                                                    }
                                                                    
                                                                    // 作業ディレクトリがあれば設定
                                                                    if let Some(cwd) = args.get("cwd")
                                                                        .and_then(|c| c.as_str()) {
                                                                        config.cwd = Some(PathBuf::from(cwd));
                                                                    }
                                                                    
                                                                    // 環境変数があれば設定
                                                                    if let Some(env) = args.get("env")
                                                                        .and_then(|e| e.as_object()) {
                                                                        for (key, value) in env {
                                                                            if let Some(value_str) = value.as_str() {
                                                                                config.env.insert(key.clone(), value_str.to_string());
                                                                            }
                                                                        }
                                                                    }
                                                                    
                                                                    // stopOnEntryフラグがあれば設定
                                                                    if let Some(stop_on_entry) = args.get("stopOnEntry")
                                                                        .and_then(|s| s.as_bool()) {
                                                                        config.stop_on_entry = stop_on_entry;
                                                                    }
                                                                    
                                                                    // ソースマップがあれば設定
                                                                    if let Some(source_map) = args.get("sourceMap")
                                                                        .and_then(|s| s.as_object()) {
                                                                        for (key, value) in source_map {
                                                                            if let Some(value_str) = value.as_str() {
                                                                                config.source_map.insert(key.clone(), value_str.to_string());
                                                                            }
                                                                        }
                                                                    }
                                                                    
                                                                    // デバッグエンジンに起動コマンドを送信
                                                                    let (resp_tx, resp_rx) = oneshot::channel();
                                                                    command_tx.send(DebugCommand::Launch {
                                                                        config: config.clone(),
                                                                        response: resp_tx,
                                                                    }).await.map_err(|_| anyhow!("起動コマンド送信に失敗"))?;
                                                                    
                                                                    // 応答を待機
                                                                    match resp_rx.await {
                                                                        Ok(launch_result) => {
                                                                            if let Err(e) = launch_result {
                                                                                error!("プログラム起動に失敗: {}", e);
                                                                            }
                                                                        },
                                                                        Err(e) => {
                                                                            error!("起動応答の受信に失敗: {}", e);
                                                                        }
                                                                    }
                                                                }
                                                            },
                                                        "setBreakpoints" => {
                                                            if let Some(args) = response["arguments"].as_object() {
                                                                // ソースファイルを取得
                                                                let source_path = args.get("source")
                                                                    .and_then(|s| s.get("path"))
                                                                    .and_then(|p| p.as_str())
                                                                    .map(PathBuf::from);
                                                                
                                                                if let Some(source_path) = source_path {
                                                                    // ブレークポイントのリストを構築
                                                                    let breakpoints = args.get("breakpoints")
                                                                        .and_then(|b| b.as_array())
                                                                        .map(|bps| {
                                                                            bps.iter().enumerate()
                                                                                .filter_map(|(id, bp)| {
                                                                                    bp.as_object().map(|bp_obj| {
                                                                                        let line = bp_obj.get("line")
                                                                                            .and_then(|l| l.as_u64())
                                                                                            .unwrap_or(1) as usize;
                                                                                        
                                                                                        let column = bp_obj.get("column")
                                                                                            .and_then(|c| c.as_u64())
                                                                                            .map(|c| c as usize);
                                                                                        
                                                                                        let condition = bp_obj.get("condition")
                                                                                            .and_then(|c| c.as_str())
                                                                                            .map(String::from);
                                                                                        
                                                                                        let log_message = bp_obj.get("logMessage")
                                                                                            .and_then(|m| m.as_str())
                                                                                            .map(String::from);
                                                                                        
                                                                                        Breakpoint {
                                                                                            id: id + 1, // 1-indexed
                                                                                            verified: true,
                                                                                            source_path: source_path.clone(),
                                                                                            line,
                                                                                            column,
                                                                                            condition,
                                                                                            hit_condition: None,
                                                                                            log_message,
                                                                                        }
                                                                                    })
                                                                                })
                                                                                .collect::<Vec<_>>()
                                                                        })
                                                                        .unwrap_or_default();
                                                                    
                                                                    // デバッグエンジンにブレークポイント設定コマンドを送信
                                                                    let (resp_tx, resp_rx) = oneshot::channel();
                                                                    command_tx.send(DebugCommand::SetBreakpoints {
                                                                        source: source_path,
                                                                        breakpoints,
                                                                        response: resp_tx,
                                                                    }).await.map_err(|_| anyhow!("ブレークポイント設定コマンド送信に失敗"))?;
                                                                    
                                                                    // 応答を待機
                                                                    match resp_rx.await {
                                                                        Ok(bp_result) => {
                                                                            if let Err(e) = bp_result {
                                                                                error!("ブレークポイント設定に失敗: {}", e);
                                                                            }
                                                                        },
                                                                        Err(e) => {
                                                                            error!("ブレークポイント応答の受信に失敗: {}", e);
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                        },
                                                        "continue" => {
                                                            if let Some(args) = response["arguments"].as_object() {
                                                                let thread_id = args.get("threadId")
                                                                    .and_then(|t| t.as_u64())
                                                                    .unwrap_or(1) as usize;
                                                                
                                                                // デバッグエンジンに継続実行コマンドを送信
                                                                let (resp_tx, resp_rx) = oneshot::channel();
                                                                command_tx.send(DebugCommand::ExecutionControl {
                                                                    mode: ExecutionMode::Continue,
                                                                    thread_id,
                                                                    response: resp_tx,
                                                                }).await.map_err(|_| anyhow!("継続実行コマンド送信に失敗"))?;
                                                                
                                                                // 応答を待機
                                                                match resp_rx.await {
                                                                    Ok(continue_result) => {
                                                                        if let Err(e) = continue_result {
                                                                            error!("継続実行に失敗: {}", e);
                                                                        }
                                                                    },
                                                                    Err(e) => {
                                                                        error!("継続実行応答の受信に失敗: {}", e);
                                                                    }
                                                                }
                                                            }
                                                        },
                                                        "next" => {
                                                            if let Some(args) = response["arguments"].as_object() {
                                                                let thread_id = args.get("threadId")
                                                                    .and_then(|t| t.as_u64())
                                                                    .unwrap_or(1) as usize;
                                                                
                                                                // デバッグエンジンにステップオーバーコマンドを送信
                                                                let (resp_tx, resp_rx) = oneshot::channel();
                                                                command_tx.send(DebugCommand::ExecutionControl {
                                                                    mode: ExecutionMode::StepOver,
                                                                    thread_id,
                                                                    response: resp_tx,
                                                                }).await.map_err(|_| anyhow!("ステップオーバーコマンド送信に失敗"))?;
                                                                
                                                                // 応答を待機
                                                                match resp_rx.await {
                                                                    Ok(step_result) => {
                                                                        if let Err(e) = step_result {
                                                                            error!("ステップオーバーに失敗: {}", e);
                                                                        }
                                                                    },
                                                                    Err(e) => {
                                                                        error!("ステップオーバー応答の受信に失敗: {}", e);
                                                                    }
                                                                }
                                                            }
                                                        },
                                                        "stepIn" => {
                                                            if let Some(args) = response["arguments"].as_object() {
                                                                let thread_id = args.get("threadId")
                                                                    .and_then(|t| t.as_u64())
                                                                    .unwrap_or(1) as usize;
                                                                
                                                                // デバッグエンジンにステップインコマンドを送信
                                                                let (resp_tx, resp_rx) = oneshot::channel();
                                                                command_tx.send(DebugCommand::ExecutionControl {
                                                                    mode: ExecutionMode::StepIn,
                                                                    thread_id,
                                                                    response: resp_tx,
                                                                }).await.map_err(|_| anyhow!("ステップインコマンド送信に失敗"))?;
                                                                
                                                                // 応答を待機
                                                                match resp_rx.await {
                                                                    Ok(step_result) => {
                                                                        if let Err(e) = step_result {
                                                                            error!("ステップインに失敗: {}", e);
                                                                        }
                                                                    },
                                                                    Err(e) => {
                                                                        error!("ステップイン応答の受信に失敗: {}", e);
                                                                    }
                                                                }
                                                            }
                                                        },
                                                        "stepOut" => {
                                                            if let Some(args) = response["arguments"].as_object() {
                                                                let thread_id = args.get("threadId")
                                                                    .and_then(|t| t.as_u64())
                                                                    .unwrap_or(1) as usize;
                                                                
                                                                // デバッグエンジンにステップアウトコマンドを送信
                                                                let (resp_tx, resp_rx) = oneshot::channel();
                                                                command_tx.send(DebugCommand::ExecutionControl {
                                                                    mode: ExecutionMode::StepOut,
                                                                    thread_id,
                                                                    response: resp_tx,
                                                                }).await.map_err(|_| anyhow!("ステップアウトコマンド送信に失敗"))?;
                                                                
                                                                // 応答を待機
                                                                match resp_rx.await {
                                                                    Ok(step_result) => {
                                                                        if let Err(e) = step_result {
                                                                            error!("ステップアウトに失敗: {}", e);
                                                                        }
                                                                    },
                                                                    Err(e) => {
                                                                        error!("ステップアウト応答の受信に失敗: {}", e);
                                                                    }
                                                                }
                                                            }
                                                        },
                                                        "pause" => {
                                                            if let Some(args) = response["arguments"].as_object() {
                                                                let thread_id = args.get("threadId")
                                                                    .and_then(|t| t.as_u64())
                                                                    .map(|t| t as usize);
                                                                
                                                                // デバッグエンジンに一時停止コマンドを送信
                                                                let (resp_tx, resp_rx) = oneshot::channel();
                                                                command_tx.send(DebugCommand::Pause {
                                                                    thread_id,
                                                                    response: resp_tx,
                                                                }).await.map_err(|_| anyhow!("一時停止コマンド送信に失敗"))?;
                                                                
                                                                // 応答を待機
                                                                match resp_rx.await {
                                                                    Ok(pause_result) => {
                                                                        if let Err(e) = pause_result {
                                                                            error!("一時停止に失敗: {}", e);
                                                                        }
                                                                    },
                                                                    Err(e) => {
                                                                        error!("一時停止応答の受信に失敗: {}", e);
                                                                    }
                                                                }
                                                            }
                                                        },
                                                        "evaluate" => {
                                                            if let Some(args) = response["arguments"].as_object() {
                                                                let expression = args.get("expression")
                                                                    .and_then(|e| e.as_str())
                                                                    .unwrap_or("").to_string();
                                                                
                                                                let frame_id = args.get("frameId")
                                                                    .and_then(|f| f.as_u64())
                                                                    .map(|f| f as usize);
                                                                
                                                                let context = args.get("context")
                                                                    .and_then(|c| c.as_str())
                                                                    .unwrap_or("").to_string();
                                                                
                                                                // デバッグエンジンに式評価コマンドを送信
                                                                let (resp_tx, resp_rx) = oneshot::channel();
                                                                command_tx.send(DebugCommand::Evaluate {
                                                                    expression,
                                                                    frame_id,
                                                                    context,
                                                                    response: resp_tx,
                                                                }).await.map_err(|_| anyhow!("式評価コマンド送信に失敗"))?;
                                                                
                                                                // 応答を待機
                                                                match resp_rx.await {
                                                                    Ok(eval_result) => {
                                                                        if let Err(e) = eval_result {
                                                                            error!("式評価に失敗: {}", e);
                                                                        }
                                                                    },
                                                                    Err(e) => {
                                                                        error!("式評価応答の受信に失敗: {}", e);
                                                                    }
                                                                }
                                                            }
                                                        },
                                                        "disconnect" => {
                                                            if let Some(args) = response["arguments"].as_object() {
                                                                let terminate = args.get("terminateDebuggee")
                                                                    .and_then(|t| t.as_bool())
                                                                    .unwrap_or(false);
                                                                
                                                                // デバッグエンジンに切断コマンドを送信
                                                                let (resp_tx, resp_rx) = oneshot::channel();
                                                                command_tx.send(DebugCommand::Disconnect {
                                                                    terminate,
                                                                    response: resp_tx,
                                                                }).await.map_err(|_| anyhow!("切断コマンド送信に失敗"))?;
                                                                
                                                                // 応答を待機
                                                                match resp_rx.await {
                                                                    Ok(disconnect_result) => {
                                                                        if let Err(e) = disconnect_result {
                                                                            error!("切断に失敗: {}", e);
                                                                        }
                                                                    },
                                                                    Err(e) => {
                                                                        error!("切断応答の受信に失敗: {}", e);
                                                                    }
                                                                }
                                                                
                                                                // 接続を閉じる
                                                                return Ok(());
                                                            }
                                                        },
                                                        "enableTimeTravel" => {
                                                            // タイムトラベルデバッグを有効化
                                                            let (resp_tx, resp_rx) = oneshot::channel();
                                                            command_tx.send(DebugCommand::TimeTravel { 
                                                                command: TimeTravelCommand::EnableTimeTravel { response: resp_tx } 
                                                            }).await.map_err(|_| anyhow!("コマンド送信失敗"))?;
                                                            
                                                            // 応答を待機
                                                            match resp_rx.await {
                                                                Ok(result) => {
                                                                    if let Err(e) = result {
                                                                        error!("タイムトラベルデバッグの有効化に失敗: {}", e);
                                                                    }
                                                                },
                                                                Err(e) => {
                                                                    error!("タイムトラベル応答の受信に失敗: {}", e);
                                                                }
                                                            }
                                                        },
                                                        "disableTimeTravel" => {
                                                            // タイムトラベルデバッグを無効化
                                                            let (resp_tx, resp_rx) = oneshot::channel();
                                                            command_tx.send(DebugCommand::TimeTravel { 
                                                                command: TimeTravelCommand::DisableTimeTravel { response: resp_tx } 
                                                            }).await.map_err(|_| anyhow!("コマンド送信失敗"))?;
                                                            
                                                            // 応答を待機
                                                            match resp_rx.await {
                                                                Ok(result) => {
                                                                    if let Err(e) = result {
                                                                        error!("タイムトラベルデバッグの無効化に失敗: {}", e);
                                                                    }
                                                                },
                                                                Err(e) => {
                                                                    error!("タイムトラベル応答の受信に失敗: {}", e);
                                                                }
                                                            }
                                                        },
                                                        "recordState" => {
                                                            // 現在の状態を記録
                                                            let (resp_tx, resp_rx) = oneshot::channel();
                                                            command_tx.send(DebugCommand::TimeTravel { 
                                                                command: TimeTravelCommand::RecordState { response: resp_tx } 
                                                            }).await.map_err(|_| anyhow!("コマンド送信失敗"))?;
                                                            
                                                            // 応答を待機
                                                            match resp_rx.await {
                                                                Ok(result) => {
                                                                    if let Err(e) = result {
                                                                        error!("スナップショットの記録に失敗: {}", e);
                                                                    }
                                                                },
                                                                Err(e) => {
                                                                    error!("スナップショット応答の受信に失敗: {}", e);
                                                                }
                                                            }
                                                            
                                                            // 注釈があれば追加
                                                            if let Some(args) = response["arguments"].as_object() {
                                                                if let Some(annotation) = args.get("annotation").and_then(|a| a.as_str()) {
                                                                    if let Some(snapshot_id) = response["body"]["snapshotId"].as_u64() {
                                                                        let (resp_tx, resp_rx) = oneshot::channel();
                                                                        command_tx.send(DebugCommand::TimeTravel { 
                                                                            command: TimeTravelCommand::AnnotateSnapshot { 
                                                                                snapshot_id: snapshot_id as usize, 
                                                                                annotation: annotation.to_string(), 
                                                                                response: resp_tx 
                                                                            } 
                                                                        }).await.map_err(|_| anyhow!("コマンド送信失敗"))?;
                                                                        
                                                                        // 応答を待機
                                                                        match resp_rx.await {
                                                                            Ok(result) => {
                                                                                if let Err(e) = result {
                                                                                    error!("注釈の追加に失敗: {}", e);
                                                                                }
                                                                            },
                                                                            Err(e) => {
                                                                                error!("注釈応答の受信に失敗: {}", e);
                                                                            }
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                        },
                                                        "gotoSnapshot" => {
                                                            // 引数を取得
                                                            if let Some(args) = response["arguments"].as_object() {
                                                                if let Some(snapshot_id) = args.get("snapshotId").and_then(|id| id.as_u64()) {
                                                                    // 指定したスナップショットに移動
                                                                    let (resp_tx, resp_rx) = oneshot::channel();
                                                                    command_tx.send(DebugCommand::TimeTravel { 
                                                                        command: TimeTravelCommand::GotoSnapshot { 
                                                                            snapshot_id: snapshot_id as usize, 
                                                                            response: resp_tx 
                                                                        } 
                                                                    }).await.map_err(|_| anyhow!("コマンド送信失敗"))?;
                                                                    
                                                                    // 応答を待機
                                                                    match resp_rx.await {
                                                                        Ok(result) => {
                                                                            if let Err(e) = result {
                                                                                error!("スナップショットへの移動に失敗: {}", e);
                                                                            }
                                                                        },
                                                                        Err(e) => {
                                                                            error!("スナップショット応答の受信に失敗: {}", e);
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                        },
                                                        "stepBack" => {
                                                            // 前のスナップショットに移動
                                                            let (resp_tx, resp_rx) = oneshot::channel();
                                                            command_tx.send(DebugCommand::TimeTravel { 
                                                                command: TimeTravelCommand::StepBack { response: resp_tx } 
                                                            }).await.map_err(|_| anyhow!("コマンド送信失敗"))?;
                                                            
                                                            // 応答を待機
                                                            match resp_rx.await {
                                                                Ok(result) => {
                                                                    if let Err(e) = result {
                                                                        error!("前のスナップショットへの移動に失敗: {}", e);
                                                                    }
                                                                },
                                                                Err(e) => {
                                                                    error!("スナップショット応答の受信に失敗: {}", e);
                                                                }
                                                            }
                                                        },
                                                        "stepForward" => {
                                                            // 次のスナップショットに移動
                                                            let (resp_tx, resp_rx) = oneshot::channel();
                                                            command_tx.send(DebugCommand::TimeTravel { 
                                                                command: TimeTravelCommand::StepForward { response: resp_tx } 
                                                            }).await.map_err(|_| anyhow!("コマンド送信失敗"))?;
                                                            
                                                            // 応答を待機
                                                            match resp_rx.await {
                                                                Ok(result) => {
                                                                    if let Err(e) = result {
                                                                        error!("次のスナップショットへの移動に失敗: {}", e);
                                                                    }
                                                                },
                                                                Err(e) => {
                                                                    error!("スナップショット応答の受信に失敗: {}", e);
                                                                }
                                                            }
                                                        },
                                                        "listSnapshots" => {
                                                            // スナップショットの一覧を取得
                                                            let (resp_tx, resp_rx) = oneshot::channel();
                                                            command_tx.send(DebugCommand::TimeTravel { 
                                                                command: TimeTravelCommand::ListSnapshots { response: resp_tx } 
                                                            }).await.map_err(|_| anyhow!("コマンド送信失敗"))?;
                                                            
                                                            // 応答を待機
                                                            match resp_rx.await {
                                                                Ok(result) => {
                                                                    if let Err(e) = result {
                                                                        error!("スナップショット一覧の取得に失敗: {}", e);
                                                                    }
                                                                },
                                                                Err(e) => {
                                                                    error!("スナップショット応答の受信に失敗: {}", e);
                                                                }
                                                            }
                                                        },
                                                        "annotateSnapshot" => {
                                                            // 引数を取得
                                                            if let Some(args) = response["arguments"].as_object() {
                                                                if let Some(snapshot_id) = args.get("snapshotId").and_then(|id| id.as_u64()) {
                                                                    if let Some(annotation) = args.get("annotation").and_then(|a| a.as_str()) {
                                                                        // スナップショットに注釈を追加
                                                                        let (resp_tx, resp_rx) = oneshot::channel();
                                                                        command_tx.send(DebugCommand::TimeTravel { 
                                                                            command: TimeTravelCommand::AnnotateSnapshot { 
                                                                                snapshot_id: snapshot_id as usize, 
                                                                                annotation: annotation.to_string(), 
                                                                                response: resp_tx 
                                                                            } 
                                                                        }).await.map_err(|_| anyhow!("コマンド送信失敗"))?;
                                                                        
                                                                        // 応答を待機
                                                                        match resp_rx.await {
                                                                            Ok(result) => {
                                                                                if let Err(e) = result {
                                                                                    error!("注釈の追加に失敗: {}", e);
                                                                                }
                                                                            },
                                                                            Err(e) => {
                                                                                error!("注釈応答の受信に失敗: {}", e);
                                                                            }
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                        },
                                                        _ => {}
                                                    }
                                                },
                                                Err(e) => {
                                                    error!("リクエスト処理エラー: {}", e);
                                                }
                                            }
                                        },
                                        Err(e) => {
                                            error!("JSONメッセージの解析エラー: {}", e);
                                        }
                                    }
                                    
                                    buffer.clear();
                                    reading_headers = true;
                                    content_length = 0;
                                }
                            }
                        },
                        Err(e) => {
                            debug!("クライアント接続が閉じられました: {}", e);
                            break;
                        }
                    }
                }
            }
        }
    }
    
    // エンジンタスクを終了
    engine_task.abort();
    
    debug!("クライアント接続を閉じました");
    Ok(())
}

/// SwiftLightプログラムのデバッグシンボル情報を表示
fn print_debug_symbols(program: &Path) -> Result<()> {
    println!("{}", "== デバッグシンボル情報 ==".cyan());
    
    // 実際の実装では、SwiftLightのデバッグシンボル情報を解析して表示
    println!("この機能は現在開発中です。");
    
    Ok(())
}

/// SwiftLightプログラムのブレークポイントの一覧を表示
fn list_breakpoints(session: &DebugSession) {
    println!("{}", "== ブレークポイント一覧 ==".cyan());
    
    if session.breakpoints.is_empty() {
        println!("ブレークポイントが設定されていません。");
        return;
    }
    
    for (id, bp) in &session.breakpoints {
        let location = format!("{}:{}", bp.source_path.display(), bp.line);
        
        let condition = match &bp.condition {
            Some(cond) => format!(" 条件: {}", cond),
            None => String::new(),
        };
        
        println!("BP #{}: {}{} [{}]", 
                id, 
                location, 
                condition,
                if bp.verified { "有効" } else { "無効" });
    }
}

/// SwiftLightプログラムのソースコード表示（現在位置を強調表示）
fn show_source_with_location(source_path: &Path, current_line: usize, context_lines: usize) -> Result<()> {
    if !source_path.exists() {
        return Err(anyhow!("ソースファイルが見つかりません: {}", source_path.display()));
    }
    
    let source = fs::read_to_string(source_path)
        .context(format!("ソースファイルの読み込みに失敗: {}", source_path.display()))?;
    
    let lines: Vec<&str> = source.lines().collect();
    let start_line = current_line.saturating_sub(context_lines);
    let end_line = std::cmp::min(current_line + context_lines, lines.len());
    
    println!("{}", format!("== ソースコード: {} ==", source_path.display()).cyan());
    
    for (i, line) in lines[start_line..end_line].iter().enumerate() {
        let line_num = start_line + i + 1; // 1-indexedにする
        
        if line_num == current_line {
            println!("{} {}", format!("{:4}▶", line_num).green().bold(), line);
        } else {
            println!("{} {}", format!("{:4} ", line_num).dimmed(), line);
        }
    }
    
    Ok(())
}

/// タイムトラベルデバッグコマンドを処理
async fn handle_time_travel_command(command: TimeTravelCommands, host: String, port: u16) -> Result<()> {
    info!("タイムトラベルデバッグコマンドを実行: {:?}", command);
    
    // デバッグセッションを作成
    let session = Arc::new(Mutex::new(DebugSession::default()));
    
    // チャネルを作成
    let (command_tx, command_rx) = mpsc::channel(100);
    let (event_tx, mut event_rx) = mpsc::channel(100);
    
    // エンジンを別タスクで実行
    let mut engine = DebugEngine::new(session.clone(), command_rx, event_tx.clone());
    let engine_task = tokio::spawn(async move {
        if let Err(e) = engine.run().await {
            error!("デバッグエンジンエラー: {}", e);
        }
    });
    
    // 初期化コマンドを送信
    let (resp_tx, resp_rx) = oneshot::channel();
    command_tx.send(DebugCommand::Initialize { response: resp_tx }).await
        .map_err(|_| anyhow!("コマンド送信失敗"))?;
    resp_rx.await.map_err(|_| anyhow!("レスポンス受信失敗"))??;
    
    // タイムトラベルコマンドを処理
    match command {
        TimeTravelCommands::Enable => {
            // タイムトラベルデバッグを有効化
            let (resp_tx, resp_rx) = oneshot::channel();
            command_tx.send(DebugCommand::TimeTravel { 
                command: TimeTravelCommand::EnableTimeTravel { response: resp_tx } 
            }).await.map_err(|_| anyhow!("コマンド送信失敗"))?;
            
            match resp_rx.await {
                Ok(result) => {
                    if let Err(e) = result {
                        error!("タイムトラベルデバッグの有効化に失敗: {}", e);
                    }
                },
                Err(e) => {
                    error!("タイムトラベル応答の受信に失敗: {}", e);
                }
            }
        }
        TimeTravelCommands::Disable => {
            // タイムトラベルデバッグを無効化
            let (resp_tx, resp_rx) = oneshot::channel();
            command_tx.send(DebugCommand::TimeTravel { 
                command: TimeTravelCommand::DisableTimeTravel { response: resp_tx } 
            }).await.map_err(|_| anyhow!("コマンド送信失敗"))?;
            
            match resp_rx.await {
                Ok(result) => {
                    if let Err(e) = result {
                        error!("タイムトラベルデバッグの無効化に失敗: {}", e);
                    }
                },
                Err(e) => {
                    error!("タイムトラベル応答の受信に失敗: {}", e);
                }
            }
        }
        TimeTravelCommands::Record { annotation } => {
            // 現在の状態を記録
            let (resp_tx, resp_rx) = oneshot::channel();
            command_tx.send(DebugCommand::TimeTravel { 
                command: TimeTravelCommand::RecordState { response: resp_tx } 
            }).await.map_err(|_| anyhow!("コマンド送信失敗"))?;
            
            match resp_rx.await {
                Ok(result) => {
                    match result {
                        Ok(snapshot_id) => {
                            println!("スナップショットを記録しました: ID={}", snapshot_id);
                            
                            // 注釈があれば追加
                            if let Some(annotation) = annotation {
                                let (resp_tx, resp_rx) = oneshot::channel();
                                command_tx.send(DebugCommand::TimeTravel { 
                                    command: TimeTravelCommand::AnnotateSnapshot { 
                                        snapshot_id, 
                                        annotation, 
                                        response: resp_tx 
                                    } 
                                }).await.map_err(|_| anyhow!("コマンド送信失敗"))?;
                                
                                // 応答を待機
                                match resp_rx.await {
                                    Ok(result) => {
                                        if let Err(e) = result {
                                            error!("注釈の追加に失敗: {}", e);
                                        } else {
                                            println!("スナップショットに注釈を追加しました");
                                        }
                                    },
                                    Err(e) => {
                                        error!("注釈応答の受信に失敗: {}", e);
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            error!("スナップショットの記録に失敗: {}", e);
                        }
                    }
                },
                Err(e) => {
                    error!("スナップショット応答の受信に失敗: {}", e);
                }
            }
        }
        TimeTravelCommands::Goto { snapshot_id } => {
            // 指定したスナップショットに移動
            let (resp_tx, resp_rx) = oneshot::channel();
            command_tx.send(DebugCommand::TimeTravel { 
                command: TimeTravelCommand::GotoSnapshot { 
                    snapshot_id, 
                    response: resp_tx 
                } 
            }).await.map_err(|_| anyhow!("コマンド送信失敗"))?;
            
            match resp_rx.await {
                Ok(result) => {
                    if let Err(e) = result {
                        error!("スナップショットへの移動に失敗: {}", e);
                    }
                },
                Err(e) => {
                    error!("スナップショット応答の受信に失敗: {}", e);
                }
            }
        }
        TimeTravelCommands::Back => {
            // 前のスナップショットに移動
            let (resp_tx, resp_rx) = oneshot::channel();
            command_tx.send(DebugCommand::TimeTravel { 
                command: TimeTravelCommand::StepBack { response: resp_tx } 
            }).await.map_err(|_| anyhow!("コマンド送信失敗"))?;
            
            match resp_rx.await {
                Ok(result) => {
                    if let Err(e) = result {
                        error!("前のスナップショットへの移動に失敗: {}", e);
                    }
                },
                Err(e) => {
                    error!("スナップショット応答の受信に失敗: {}", e);
                }
            }
        }
        TimeTravelCommands::Forward => {
            // 次のスナップショットに移動
            let (resp_tx, resp_rx) = oneshot::channel();
            command_tx.send(DebugCommand::TimeTravel { 
                command: TimeTravelCommand::StepForward { response: resp_tx } 
            }).await.map_err(|_| anyhow!("コマンド送信失敗"))?;
            
            match resp_rx.await {
                Ok(result) => {
                    if let Err(e) = result {
                        error!("次のスナップショットへの移動に失敗: {}", e);
                    }
                },
                Err(e) => {
                    error!("スナップショット応答の受信に失敗: {}", e);
                }
            }
        }
        TimeTravelCommands::List => {
            // スナップショットの一覧を取得
            let (resp_tx, resp_rx) = oneshot::channel();
            command_tx.send(DebugCommand::TimeTravel { 
                command: TimeTravelCommand::ListSnapshots { response: resp_tx } 
            }).await.map_err(|_| anyhow!("コマンド送信失敗"))?;
            
            match resp_rx.await {
                Ok(result) => {
                    match result {
                        Ok(snapshots) => {
                            if snapshots.is_empty() {
                                println!("スナップショットはありません");
                            } else {
                                println!("{}", "== スナップショット一覧 ==".cyan());
                                println!("{:<5} {:<30} {:<20} {}", 
                                    "ID", "説明", "タイムスタンプ", "現在位置");
                                
                                // セッションから現在のスナップショットIDを取得
                                let current_id = session.lock().unwrap().current_snapshot_id;
                                
                                for (id, description, timestamp) in snapshots {
                                    let dt = chrono::DateTime::from_timestamp(timestamp as i64, 0)
                                        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                                        .unwrap_or_else(|| timestamp.to_string());
                                    
                                    let current_marker = if current_id == Some(id) {
                                        "◀".green().bold().to_string()
                                    } else {
                                        "".to_string()
                                    };
                                    
                                    println!("{:<5} {:<30} {:<20} {}", 
                                        id, description, dt, current_marker);
                                }
                            }
                        }
                        Err(e) => {
                            error!("スナップショット一覧の取得に失敗: {}", e);
                        }
                    }
                },
                Err(e) => {
                    error!("スナップショット応答の受信に失敗: {}", e);
                }
            }
        }
        TimeTravelCommands::Annotate { snapshot_id, annotation } => {
            // スナップショットに注釈を追加
            let (resp_tx, resp_rx) = oneshot::channel();
            command_tx.send(DebugCommand::TimeTravel { 
                command: TimeTravelCommand::AnnotateSnapshot { 
                    snapshot_id, 
                    annotation, 
                    response: resp_tx 
                } 
            }).await.map_err(|_| anyhow!("コマンド送信失敗"))?;
            
            match resp_rx.await {
                Ok(result) => {
                    if let Err(e) = result {
                        error!("注釈の追加に失敗: {}", e);
                    }
                },
                Err(e) => {
                    error!("注釈応答の受信に失敗: {}", e);
                }
            }
        }
    }
    
    // エンジンタスクを終了
    engine_task.abort();
    
    Ok(())
}

/// テスト
#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::io::Cursor;
    
    #[test]
    fn test_cli_parsing() {
        let args = vec![
            "swiftlight-debug",
            "launch",
            "/path/to/program",
            "--cwd",
            "/path/to/dir",
            "arg1",
            "arg2",
        ];
        
        let cli = Cli::try_parse_from(args).unwrap();
        
        match cli.command {
            Commands::Launch { program, cwd, args } => {
                assert_eq!(program, PathBuf::from("/path/to/program"));
                assert_eq!(cwd, Some(PathBuf::from("/path/to/dir")));
                assert_eq!(args, vec!["arg1", "arg2"]);
            }
            _ => panic!("Expected Launch command"),
        }
    }
    
    #[test]
    fn test_display_source() {
        // テスト用の一時ファイルを作成
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("test.sl");
        
        let source = "fn main() {\n    println(\"Hello\");\n    let x = 42;\n    return 0;\n}\n";
        fs::write(&file_path, source).unwrap();
        
        // 現在の行を指定してソースを表示
        let result = show_source_with_location(&file_path, 3, 1);
        assert!(result.is_ok());
        
        // 一時ファイルは自動的に削除される
    }
}
