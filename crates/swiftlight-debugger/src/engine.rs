/*
 * SwiftLight デバッガー - デバッグエンジン
 *
 * このモジュールは、SwiftLight言語のデバッグエンジンを提供します。
 * デバッグ対象プログラムの実行制御、ブレークポイント管理、変数監視などの機能を実装します。
 */

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::process::{Command, Child, Stdio};
use std::io::{BufRead, BufReader};
use log::{debug, info, warn, error};
use anyhow::{Result, Context, anyhow};
use tokio::sync::mpsc::{Sender, Receiver};
use tokio::sync::oneshot;
use tokio::task;
use tokio::io::{AsyncBufReadExt, BufStream};
use tokio::process::Command as TokioCommand;
use tokio::time::{sleep, Duration};
use nix::sys::ptrace;
use nix::sys::wait::{waitpid, WaitStatus};
use nix::unistd::Pid;
use goblin::elf::Elf;
use addr2line::{Context as AddrContext, Location};
use gimli::{Dwarf, EndianSlice, RunTimeEndian};
use object::{Object, ObjectSection};
use std::time::{SystemTime, UNIX_EPOCH};
use std::fs::File;
use std::io::{BufWriter, Write};
use serde_json;

use crate::protocol::{
    DebugConfiguration, DebugSession, ProcessStatus, StopReason, 
    StackFrame, Variable, VariableKind, Breakpoint, Thread
};

/// デバッグエンジンの実行モード
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionMode {
    /// 継続実行
    Continue,
    
    /// ステップイン（関数内に入る）
    StepIn,
    
    /// ステップオーバー（関数をスキップ）
    StepOver,
    
    /// ステップアウト（関数から出る）
    StepOut,
}

/// デバッグエンジンイベント
#[derive(Debug)]
pub enum DebugEvent {
    /// 初期化が完了
    Initialized,
    
    /// 停止
    Stopped {
        thread_id: usize,
        reason: StopReason,
    },
    
    /// 実行再開
    Continued {
        thread_id: usize,
        all_threads: bool,
    },
    
    /// 終了
    Exited {
        exit_code: i32,
    },
    
    /// 出力
    Output {
        category: String,
        output: String,
    },
    
    /// スレッド作成
    ThreadStarted {
        thread_id: usize,
    },
    
    /// スレッド終了
    ThreadExited {
        thread_id: usize,
    },
    
    /// ブレークポイント変更
    BreakpointChanged {
        breakpoint: Breakpoint,
    },
    
    /// タイムトラベル状態変更
    TimeTravelStateChanged {
        enabled: bool,
        direction: TimeDirection,
        current_snapshot_id: Option<usize>,
    },
    
    /// スナップショットに移動
    SnapshotNavigated {
        snapshot_id: usize,
        timestamp: u64,
        source_location: (String, usize, Option<usize>),
        function_name: String,
    },
}

/// デバッグエンジンに対するコマンド
#[derive(Debug)]
pub enum DebugCommand {
    /// 初期化
    Initialize {
        response: oneshot::Sender<Result<()>>,
    },
    
    /// 起動
    Launch {
        config: DebugConfiguration,
        response: oneshot::Sender<Result<()>>,
    },
    
    /// アタッチ
    Attach {
        config: DebugConfiguration,
        pid: Option<u32>,
        response: oneshot::Sender<Result<()>>,
    },
    
    /// 実行制御
    ExecutionControl {
        mode: ExecutionMode,
        thread_id: usize,
        response: oneshot::Sender<Result<()>>,
    },
    
    /// 一時停止
    Pause {
        thread_id: Option<usize>,
        response: oneshot::Sender<Result<()>>,
    },
    
    /// ブレークポイント設定
    SetBreakpoints {
        source: PathBuf,
        breakpoints: Vec<Breakpoint>,
        response: oneshot::Sender<Result<Vec<Breakpoint>>>,
    },
    
    /// スタックトレース取得
    GetStackTrace {
        thread_id: usize,
        response: oneshot::Sender<Result<Vec<StackFrame>>>,
    },
    
    /// 変数取得
    GetVariables {
        variable_reference: usize,
        response: oneshot::Sender<Result<Vec<Variable>>>,
    },
    
    /// 式評価
    Evaluate {
        expression: String,
        frame_id: Option<usize>,
        context: String,
        response: oneshot::Sender<Result<Variable>>,
    },
    
    /// 終了
    Disconnect {
        terminate: bool,
        response: oneshot::Sender<Result<()>>,
    },
    
    /// タイムトラベル関連コマンド
    TimeTravel {
        command: TimeTravelCommand,
    },
}

/// ブレークポイント情報
struct BreakpointInfo {
    /// ブレークポイントID
    id: usize,
    /// ソースファイルパス
    source_path: PathBuf,
    /// 行番号
    line: usize,
    /// 列番号
    column: Option<usize>,
    /// アドレス
    address: Option<u64>,
    /// 元の命令
    original_instruction: Option<u64>,
    /// 有効かどうか
    enabled: bool,
}

/// デバッグシンボル情報
struct DebugSymbols {
    /// アドレスから行情報へのマッピング
    addr2line_ctx: Option<AddrContext<EndianSlice<RunTimeEndian>>>,
    /// シンボル名からアドレスへのマッピング
    symbols: HashMap<String, u64>,
    /// 行情報からアドレスへのマッピング
    line_to_addr: HashMap<(PathBuf, usize), u64>,
}

impl DebugSymbols {
    /// 新しいデバッグシンボル情報を作成
    fn new() -> Self {
        Self {
            addr2line_ctx: None,
            symbols: HashMap::new(),
            line_to_addr: HashMap::new(),
        }
    }

    /// バイナリファイルからシンボル情報を読み込む
    fn load_from_binary(&mut self, binary_path: &Path) -> Result<()> {
        // バイナリファイルを読み込む
        let file_data = std::fs::read(binary_path)
            .context(format!("Failed to read binary file: {}", binary_path.display()))?;

        // ELFファイルを解析
        let elf = Elf::parse(&file_data)
            .context("Failed to parse ELF file")?;

        // シンボルテーブルを解析
        for sym in elf.syms.iter() {
            if let Some(name) = elf.strtab.get_at(sym.st_name) {
                self.symbols.insert(name.to_string(), sym.st_value);
            }
        }

        // デバッグ情報を解析
        let object = object::File::parse(&*file_data)
            .context("Failed to parse object file")?;

        // DWARF情報を取得
        let endian = if object.is_little_endian() {
            RunTimeEndian::Little
        } else {
            RunTimeEndian::Big
        };

        if let Some(section) = object.section_by_name(".debug_info") {
            if let Ok(data) = section.uncompressed_data() {
                let debug_info = EndianSlice::new(&data, endian);
                
                // その他のDWARFセクションを取得
                let mut debug_sections = HashMap::new();
                for &section_name in &[".debug_abbrev", ".debug_str", ".debug_line", ".debug_ranges", ".debug_addr"] {
                    if let Some(section) = object.section_by_name(section_name) {
                        if let Ok(data) = section.uncompressed_data() {
                            debug_sections.insert(section_name, EndianSlice::new(&data, endian));
                        }
                    }
                }

                // addr2lineコンテキストを作成
                if let Some(abbrev_data) = debug_sections.get(".debug_abbrev") {
                    if let Some(line_data) = debug_sections.get(".debug_line") {
                        if let Some(str_data) = debug_sections.get(".debug_str") {
                            let ctx = AddrContext::from_sections(
                                debug_info,
                                *abbrev_data,
                                *str_data,
                                *line_data,
                                debug_sections.get(".debug_ranges").copied(),
                                debug_sections.get(".debug_addr").copied(),
                            ).context("Failed to create addr2line context")?;
                            
                            self.addr2line_ctx = Some(ctx);
                            
                            // 行情報からアドレスへのマッピングを構築
                            self.build_line_to_addr_mapping(&object, endian)?;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// 行情報からアドレスへのマッピングを構築
    fn build_line_to_addr_mapping(&mut self, object: &object::File, endian: RunTimeEndian) -> Result<()> {
        if let Some(section) = object.section_by_name(".debug_line") {
            if let Ok(data) = section.uncompressed_data() {
                let debug_line = EndianSlice::new(&data, endian);
                
                // .debug_lineセクションを解析して行情報を取得
                let dwarf = Dwarf {
                    debug_line,
                    // 他のセクションはダミー値で埋める
                    debug_info: EndianSlice::new(&[], endian),
                    debug_abbrev: EndianSlice::new(&[], endian),
                    debug_str: EndianSlice::new(&[], endian),
                    debug_str_sup: EndianSlice::new(&[], endian),
                    debug_line_str: EndianSlice::new(&[], endian),
                    debug_addr: EndianSlice::new(&[], endian),
                    debug_ranges: EndianSlice::new(&[], endian),
                    debug_rnglists: EndianSlice::new(&[], endian),
                    debug_loclists: EndianSlice::new(&[], endian),
                    debug_frame: EndianSlice::new(&[], endian),
                    eh_frame: EndianSlice::new(&[], endian),
                    debug_loc: EndianSlice::new(&[], endian),
                    debug_pubnames: EndianSlice::new(&[], endian),
                    debug_pubtypes: EndianSlice::new(&[], endian),
                    debug_aranges: EndianSlice::new(&[], endian),
                    debug_names: EndianSlice::new(&[], endian),
                    debug_types: EndianSlice::new(&[], endian),
                    debug_macro: EndianSlice::new(&[], endian),
                    debug_macinfo: EndianSlice::new(&[], endian),
                };

                // 行プログラムを解析
                let mut line_program_iter = dwarf.debug_line.programs();
                while let Ok(Some((header, line_program))) = line_program_iter.next() {
                    let mut rows = line_program.rows();
                    while let Ok(Some(row)) = rows.next() {
                        if row.end_sequence() {
                            continue;
                        }

                        if let Some(file) = row.file(header) {
                            if let Some(dir) = file.directory(header) {
                                let dir_str = dwarf.debug_str.read_str(dir)?;
                                let file_str = dwarf.debug_str.read_str(file.path_name())?;
                                
                                let mut path = PathBuf::from(dir_str.to_string());
                                path.push(file_str.to_string());
                                
                                let line = row.line().unwrap_or(0) as usize;
                                let addr = row.address();
                                
                                self.line_to_addr.insert((path, line), addr);
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// アドレスから行情報を取得
    fn get_location_from_addr(&self, addr: u64) -> Option<(PathBuf, usize)> {
        if let Some(ctx) = &self.addr2line_ctx {
            if let Ok(mut locations) = ctx.find_location(addr) {
                if let Some(loc) = locations.next() {
                    if let Some(file) = loc.file {
                        if let Some(line) = loc.line {
                            return Some((PathBuf::from(file), line as usize));
                        }
                    }
                }
            }
        }
        None
    }

    /// 行情報からアドレスを取得
    fn get_addr_from_location(&self, file: &Path, line: usize) -> Option<u64> {
        self.line_to_addr.get(&(file.to_path_buf(), line)).copied()
    }

    /// シンボル名からアドレスを取得
    fn get_addr_from_symbol(&self, symbol: &str) -> Option<u64> {
        self.symbols.get(symbol).copied()
    }
}

/// タイムトラベルデバッグの状態
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TimeDirection {
    /// 前方（通常実行）
    Forward,
    
    /// 後方（時間を逆行）
    Backward,
    
    /// 停止（実行時点で静止）
    Paused,
}

/// 実行履歴のスナップショット
#[derive(Debug, Clone)]
pub struct ExecutionSnapshot {
    /// スナップショットID
    id: usize,
    
    /// 記録時刻
    timestamp: u64,
    
    /// ソースコード位置
    source_location: (PathBuf, usize, Option<usize>),
    
    /// 実行中の関数名
    function_name: String,
    
    /// スタックトレース
    stack_frames: Vec<StackFrame>,
    
    /// 変数状態
    variables: HashMap<String, Variable>,
    
    /// メモリダンプ（アドレス→バイト）
    memory_dump: HashMap<u64, Vec<u8>>,
    
    /// レジスタ状態
    registers: HashMap<String, u64>,
    
    /// スレッド状態
    threads: HashMap<usize, Thread>,
    
    /// ユーザー注釈（オプション）
    annotation: Option<String>,
}

/// デバッグエンジン
pub struct DebugEngine {
    /// セッション
    session: Arc<Mutex<DebugSession>>,
    
    /// コマンド受信チャネル
    command_rx: Option<Receiver<DebugCommand>>,
    
    /// イベント送信チャネル
    event_tx: Option<Sender<DebugEvent>>,
    
    /// デバッグ対象プロセス
    child_process: Option<Child>,
    
    /// デバッグ対象プロセスID
    target_pid: Option<Pid>,
    
    /// 次のフレームID
    next_frame_id: usize,
    
    /// 次の変数参照ID
    next_var_ref: usize,
    
    /// ブレークポイント情報
    breakpoints: HashMap<usize, BreakpointInfo>,
    
    /// デバッグシンボル情報
    debug_symbols: DebugSymbols,
    
    /// 実行中タスクのハンドル
    execution_task: Option<task::JoinHandle<()>>,
    
    /// 停止中かどうか
    is_stopped: bool,
    
    /// 現在のスレッドID
    current_thread_id: usize,
    
    /// スレッド情報
    threads: HashMap<usize, Thread>,
    
    /// タイムトラベルデバッグが有効かどうか
    time_travel_enabled: bool,
    
    /// 実行履歴のスナップショット
    execution_history: Vec<ExecutionSnapshot>,
    
    /// 現在のスナップショットインデックス
    current_snapshot_index: usize,
    
    /// タイムトラベルの方向
    time_direction: TimeDirection,
    
    /// 状態記録間隔（ミリ秒）
    recording_interval_ms: u64,
    
    /// 記録ファイル
    recording_file: Option<BufWriter<File>>,
}

/// タイムトラベル関連のデバッグコマンド
#[derive(Debug)]
pub enum TimeTravelCommand {
    /// タイムトラベルデバッグを有効化
    EnableTimeTravel {
        response: oneshot::Sender<Result<()>>,
    },
    
    /// タイムトラベルデバッグを無効化
    DisableTimeTravel {
        response: oneshot::Sender<Result<()>>,
    },
    
    /// 状態を記録
    RecordState {
        response: oneshot::Sender<Result<usize>>,
    },
    
    /// 指定したスナップショットに移動
    GotoSnapshot {
        snapshot_id: usize,
        response: oneshot::Sender<Result<()>>,
    },
    
    /// 前のスナップショットに移動
    StepBack {
        response: oneshot::Sender<Result<()>>,
    },
    
    /// 次のスナップショットに移動
    StepForward {
        response: oneshot::Sender<Result<()>>,
    },
    
    /// スナップショットの一覧を取得
    ListSnapshots {
        response: oneshot::Sender<Result<Vec<(usize, String, u64)>>>,
    },
    
    /// スナップショットに注釈を追加
    AnnotateSnapshot {
        snapshot_id: usize,
        annotation: String,
        response: oneshot::Sender<Result<()>>,
    },
}

impl DebugEngine {
    /// 新しいデバッグエンジンを作成
    pub fn new(
        session: Arc<Mutex<DebugSession>>, 
        command_rx: Receiver<DebugCommand>, 
        event_tx: Sender<DebugEvent>
    ) -> Self {
        Self {
            session,
            command_rx: Some(command_rx),
            event_tx: Some(event_tx),
            child_process: None,
            target_pid: None,
            next_frame_id: 1,
            next_var_ref: 1000,
            breakpoints: HashMap::new(),
            debug_symbols: DebugSymbols::new(),
            execution_task: None,
            is_stopped: false,
            current_thread_id: 1,
            threads: HashMap::new(),
            time_travel_enabled: false,
            execution_history: Vec::new(),
            current_snapshot_index: 0,
            time_direction: TimeDirection::Forward,
            recording_interval_ms: 100, // デフォルトは100ms間隔
            recording_file: None,
        }
    }
    
    /// エンジンを実行
    pub async fn run(&mut self) -> Result<()> {
        let mut command_rx = self.command_rx.take()
            .ok_or_else(|| anyhow!("コマンド受信チャネルが初期化されていません"))?;
        
        let event_tx = self.event_tx.clone()
            .ok_or_else(|| anyhow!("イベント送信チャネルが初期化されていません"))?;
        
        // コマンド処理ループ
        while let Some(command) = command_rx.recv().await {
            match command {
                DebugCommand::Initialize { response } => {
                    let result = self.handle_initialize().await;
                    let _ = response.send(result);
                }
                
                DebugCommand::Launch { config, response } => {
                    let result = self.handle_launch(config).await;
                    let _ = response.send(result);
                }
                
                DebugCommand::Attach { config, pid, response } => {
                    let result = self.handle_attach(config, pid).await;
                    let _ = response.send(result);
                }
                
                DebugCommand::ExecutionControl { mode, thread_id, response } => {
                    let result = self.handle_execution_control(mode, thread_id).await;
                    let _ = response.send(result);
                }
                
                DebugCommand::Pause { thread_id, response } => {
                    let result = self.handle_pause(thread_id).await;
                    let _ = response.send(result);
                }
                
                DebugCommand::SetBreakpoints { source, breakpoints, response } => {
                    let result = self.handle_set_breakpoints(source, breakpoints).await;
                    let _ = response.send(result);
                }
                
                DebugCommand::GetStackTrace { thread_id, response } => {
                    let result = self.handle_get_stack_trace(thread_id).await;
                    let _ = response.send(result);
                }
                
                DebugCommand::GetVariables { variable_reference, response } => {
                    let result = self.handle_get_variables(variable_reference).await;
                    let _ = response.send(result);
                }
                
                DebugCommand::Evaluate { expression, frame_id, context, response } => {
                    let result = self.handle_evaluate(expression, frame_id, &context).await;
                    let _ = response.send(result);
                }
                
                DebugCommand::Disconnect { terminate, response } => {
                    let result = self.handle_disconnect(terminate).await;
                    let _ = response.send(result);
                    
                    // 終了コマンドを受け取ったらループを抜ける
                    break;
                }
                
                DebugCommand::TimeTravel { command } => {
                    match command {
                        TimeTravelCommand::EnableTimeTravel { response } => {
                            let result = self.handle_enable_time_travel().await;
                            let _ = response.send(result);
                        },
                        TimeTravelCommand::DisableTimeTravel { response } => {
                            let result = self.handle_disable_time_travel().await;
                            let _ = response.send(result);
                        },
                        TimeTravelCommand::RecordState { response } => {
                            let result = self.handle_record_state().await;
                            let _ = response.send(result);
                        },
                        TimeTravelCommand::GotoSnapshot { snapshot_id, response } => {
                            let result = self.handle_goto_snapshot(snapshot_id).await;
                            let _ = response.send(result);
                        },
                        TimeTravelCommand::StepBack { response } => {
                            let result = self.handle_step_back().await;
                            let _ = response.send(result);
                        },
                        TimeTravelCommand::StepForward { response } => {
                            let result = self.handle_step_forward().await;
                            let _ = response.send(result);
                        },
                        TimeTravelCommand::ListSnapshots { response } => {
                            let result = self.handle_list_snapshots().await;
                            let _ = response.send(result);
                        },
                        TimeTravelCommand::AnnotateSnapshot { snapshot_id, annotation, response } => {
                            let result = self.handle_annotate_snapshot(snapshot_id, annotation).await;
                            let _ = response.send(result);
                        },
                    }
                },
            }
        }
        
        // 残ったリソースを解放
        self.cleanup().await?;
        
        Ok(())
    }
    
    /// 初期化処理
    async fn handle_initialize(&mut self) -> Result<()> {
        debug!("デバッグエンジンを初期化しています...");
        
        {
            let mut session = self.session.lock().map_err(|_| anyhow!("セッションのロックに失敗"))?;
            session.initialize().map_err(|e| anyhow!("セッション初期化エラー: {}", e))?;
        }
        
        // メインスレッドを追加
        self.threads.insert(1, Thread {
            id: 1,
            name: "main".to_string(),
        });
        
        // 初期化完了イベントを送信
        if let Some(event_tx) = &self.event_tx {
            event_tx.send(DebugEvent::Initialized).await
                .map_err(|_| anyhow!("イベント送信に失敗"))?;
        }
        
        debug!("デバッグエンジンの初期化が完了しました");
        Ok(())
    }
    
    /// プログラム起動処理
    async fn handle_launch(&mut self, config: DebugConfiguration) -> Result<()> {
        debug!("プログラムを起動しています: {}", config.program.display());
        
        {
            let mut session = self.session.lock().map_err(|_| anyhow!("セッションのロックに失敗"))?;
            session.config = config.clone();
            session.status = ProcessStatus::Running;
        }
        
        // デバッグシンボルを読み込む
        self.debug_symbols.load_from_binary(&config.program)
            .context("デバッグシンボルの読み込みに失敗")?;
        
        // プログラムを起動
        let mut command = Command::new(&config.program);
        
        // 作業ディレクトリを設定
        if let Some(cwd) = &config.cwd {
            command.current_dir(cwd);
        }
        
        // 引数を設定
        command.args(&config.args);
        
        // 環境変数を設定
        for (key, value) in &config.env {
            command.env(key, value);
        }
        
        // 標準入出力をキャプチャ
        command.stdin(Stdio::piped())
               .stdout(Stdio::piped())
               .stderr(Stdio::piped());
        
        // PTRACEフラグを設定（子プロセスが起動時に停止するように）
        unsafe {
            command.pre_exec(|| {
                ptrace::traceme().map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
                Ok(())
            });
        }
        
        // プロセスを起動
        let child = command.spawn()
            .context(format!("プログラムの起動に失敗: {}", config.program.display()))?;
        
        let pid = Pid::from_raw(child.id() as i32);
        self.target_pid = Some(pid);
        self.child_process = Some(child);
        
        // 子プロセスが停止するのを待つ
        match waitpid(pid, None) {
            Ok(WaitStatus::Stopped(_, _)) => {
                debug!("子プロセスが停止しました: {:?}", pid);
                self.is_stopped = true;
                
                // ブレークポイントを設定
                self.apply_breakpoints()?;
                
                // 標準出力と標準エラー出力をキャプチャするタスクを起動
                self.start_output_capture_tasks();
                
                // ストップオンエントリーが有効な場合は最初の行で停止
                if config.stop_on_entry {
                    self.handle_stop_on_entry().await?;
                } else {
                    // そうでなければ実行を継続
                    self.continue_execution()?;
                    
                    // 実行状態を監視するタスクを起動
                    self.start_execution_monitor_task();
                }
            }
            Ok(status) => {
                return Err(anyhow!("子プロセスが予期せぬ状態になりました: {:?}", status));
            }
            Err(e) => {
                return Err(anyhow!("子プロセスの待機に失敗: {}", e));
            }
        }
        
        Ok(())
    }
    
    /// 既存プロセスにアタッチする処理
    async fn handle_attach(&mut self, config: DebugConfiguration, pid: Option<u32>) -> Result<()> {
        debug!("プロセスにアタッチしています...");
        
        {
            let mut session = self.session.lock().map_err(|_| anyhow!("セッションのロックに失敗"))?;
            session.config = config.clone();
            session.status = ProcessStatus::Running;
        }
        
        // デバッグシンボルを読み込む
        self.debug_symbols.load_from_binary(&config.program)
            .context("デバッグシンボルの読み込みに失敗")?;
        
        // PIDが指定されている場合、そのプロセスにアタッチ
        if let Some(pid_value) = pid {
            let pid = Pid::from_raw(pid_value as i32);
            self.target_pid = Some(pid);
            
            // プロセスにアタッチ
            ptrace::attach(pid).context(format!("プロセス {} へのアタッチに失敗", pid_value))?;
            
            // プロセスが停止するのを待つ
            match waitpid(pid, None) {
                Ok(WaitStatus::Stopped(_, _)) => {
                    debug!("プロセス {} にアタッチしました", pid_value);
                    self.is_stopped = true;
                    
                    // ブレークポイントを設定
                    self.apply_breakpoints()?;
                    
                    // 実行を継続
                    self.continue_execution()?;
                    
                    // 実行状態を監視するタスクを起動
                    self.start_execution_monitor_task();
                }
                Ok(status) => {
                    return Err(anyhow!("プロセスが予期せぬ状態になりました: {:?}", status));
                }
                Err(e) => {
                    return Err(anyhow!("プロセスの待機に失敗: {}", e));
                }
            }
        } else {
            // リモートデバッグの場合
            return Err(anyhow!("リモートデバッグはまだサポートされていません"));
        }
        
        Ok(())
    }
    
    /// 実行制御処理
    async fn handle_execution_control(&mut self, mode: ExecutionMode, thread_id: usize) -> Result<()> {
        debug!("実行制御: {:?}, thread_id={}", mode, thread_id);
        
        // 現在のスレッドIDを更新
        self.current_thread_id = thread_id;
        
        // セッションを実行状態に変更
        {
            let mut session = self.session.lock().map_err(|_| anyhow!("セッションのロックに失敗"))?;
            session.run();
            session.current_thread_id = Some(thread_id);
        }
        
        // 実行モードに応じた処理
        match mode {
            ExecutionMode::Continue => {
                // 継続実行
                self.continue_execution()?;
                
                // 継続イベントを送信
                if let Some(event_tx) = &self.event_tx {
                    event_tx.send(DebugEvent::Continued {
                        thread_id,
                        all_threads: true,
                    }).await.map_err(|_| anyhow!("イベント送信に失敗"))?;
                }
                
                // 実行状態を監視するタスクを起動
                self.start_execution_monitor_task();
            }
            ExecutionMode::StepIn => {
                // ステップイン実行
                self.step_in()?;
                
                // 継続イベントを送信
                if let Some(event_tx) = &self.event_tx {
                    event_tx.send(DebugEvent::Continued {
                        thread_id,
                        all_threads: false,
                    }).await.map_err(|_| anyhow!("イベント送信に失敗"))?;
                }
                
                // 実行状態を監視するタスクを起動
                self.start_execution_monitor_task();
            }
            ExecutionMode::StepOver => {
                // ステップオーバー実行
                self.step_over()?;
                
                // 継続イベントを送信
                if let Some(event_tx) = &self.event_tx {
                    event_tx.send(DebugEvent::Continued {
                        thread_id,
                        all_threads: false,
                    }).await.map_err(|_| anyhow!("イベント送信に失敗"))?;
                }
                
                // 実行状態を監視するタスクを起動
                self.start_execution_monitor_task();
            }
            ExecutionMode::StepOut => {
                // ステップアウト実行
                self.step_out()?;
                
                // 継続イベントを送信
                if let Some(event_tx) = &self.event_tx {
                    event_tx.send(DebugEvent::Continued {
                        thread_id,
                        all_threads: false,
                    }).await.map_err(|_| anyhow!("イベント送信に失敗"))?;
                }
                
                // 実行状態を監視するタスクを起動
                self.start_execution_monitor_task();
            }
        }
        
        Ok(())
    }
    
    /// 一時停止処理
    async fn handle_pause(&mut self, thread_id: Option<usize>) -> Result<()> {
        let thread_id = thread_id.unwrap_or(self.current_thread_id);
        debug!("一時停止: thread_id={}", thread_id);
        
        if let Some(pid) = self.target_pid {
            // プロセスに一時停止シグナルを送信
            nix::sys::signal::kill(pid, nix::sys::signal::Signal::SIGINT)
                .context("一時停止シグナルの送信に失敗")?;
            
            // プロセスが停止するのを待つ
            match waitpid(pid, None) {
                Ok(WaitStatus::Stopped(_, _)) => {
                    debug!("プロセスが一時停止しました");
                    self.is_stopped = true;
                    
                    // 現在の実行位置を取得
                    let (source_path, line, column) = self.get_current_location()?;
                    
                    // 関数名を取得
                    let function_name = self.get_current_function_name()
                        .unwrap_or_else(|| "unknown".to_string());

                    // スタックトレースを取得（エラーが発生しても続行）
                    let stack_frames = match self.get_stack_trace(thread_id) {
                        Ok(frames) => frames,
                        Err(e) => {
                            error!("スタックトレース取得エラー: {}", e);
                            Vec::new()
                        }
                    };

                    // 変数情報を取得（エラーが発生しても続行）
                    let variables = match tokio::task::block_in_place(|| {
                        tokio::runtime::Runtime::new()
                            .unwrap()
                            .block_on(async {
                                self.collect_all_variables().await
                            })
                    }) {
                        Ok(vars) => vars,
                        Err(e) => {
                            error!("変数情報取得エラー: {}", e);
                            HashMap::new()
                        }
                    };

                    // メモリダンプを取得（エラーが発生しても続行）
                    let memory_dump = match self.dump_memory_regions() {
                        Ok(dump) => dump,
                        Err(e) => {
                            error!("メモリダンプ取得エラー: {}", e);
                            HashMap::new()
                        }
                    };

                    // レジスタ状態を取得（エラーが発生しても続行）
                    let registers = match self.get_registers() {
                        Ok(regs) => regs,
                        Err(e) => {
                            error!("レジスタ取得エラー: {}", e);
                            HashMap::new()
                        }
                    };

                    // スレッド情報を作成
                    let mut threads = HashMap::new();
                    threads.insert(thread_id, Thread {
                        id: thread_id,
                        name: "main".to_string(),
                        status: "running".to_string(),
                    });

                    // スナップショットを作成
                    let snapshot = ExecutionSnapshot {
                        id: 0, // 一時的なID（実際は後で割り当て）
                        timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs(),
                        source_location: (source_path, line, column),
                        function_name,
                        stack_frames,
                        variables,
                        memory_dump,
                        registers,
                        threads,
                        annotation: None,
                    };

                    // スナップショット記録イベントを送信
                    let event = DebugEvent::SnapshotNavigated {
                        snapshot_id: 0, // 一時的なID
                        timestamp: snapshot.timestamp,
                        source_location: (
                            snapshot.source_location.0.to_string_lossy().to_string(),
                            snapshot.source_location.1,
                            snapshot.source_location.2,
                        ),
                        function_name: snapshot.function_name.clone(),
                    };

                    // イベントを送信（エラーは無視して続行）
                    let _ = self.event_tx.as_ref().unwrap().try_send(event);
                }
                Ok(status) => {
                    error!("プロセスが予期しない状態になりました: {:?}", status);
                }
                Err(e) => {
                    error!("プロセス待機エラー: {}", e);
                }
            }
        }
        
        Ok(())
    }

    /// タイムトラベルデバッグを有効化
    async fn handle_enable_time_travel(&mut self) -> Result<()> {
        debug!("タイムトラベルデバッグを有効化します");
        
        if self.time_travel_enabled {
            debug!("タイムトラベルデバッグは既に有効です");
            return Ok(());
        }
        
        // タイムトラベルデバッグを有効化
        self.time_travel_enabled = true;
        self.execution_history.clear();
        self.current_snapshot_index = 0;
        self.time_direction = TimeDirection::Forward;
        
        // 記録ファイルを準備
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
        let program_name = self.session.lock().map_err(|_| anyhow!("セッションのロックに失敗"))?.config.program
            .file_name().unwrap_or_default().to_string_lossy();
        let file_name = format!("swiftlight_debug_recording_{}_{}.trace", program_name, timestamp);
        
        let file = File::create(&file_name).context(format!("記録ファイルの作成に失敗: {}", file_name))?;
        self.recording_file = Some(BufWriter::new(file));
        
        // 初期状態を記録
        self.record_current_state().await?;
        
        // 状態記録タスクを開始
        self.start_state_recording_task();
        
        // 状態変更イベントを送信
        if let Some(event_tx) = &self.event_tx {
            event_tx.send(DebugEvent::TimeTravelStateChanged {
                enabled: true,
                direction: TimeDirection::Forward,
                current_snapshot_id: self.execution_history.first().map(|s| s.id),
            }).await.map_err(|_| anyhow!("イベント送信に失敗"))?;
        }
        
        debug!("タイムトラベルデバッグを有効化しました");
        Ok(())
    }

    /// タイムトラベルデバッグを無効化
    async fn handle_disable_time_travel(&mut self) -> Result<()> {
        debug!("タイムトラベルデバッグを無効化します");
        
        if !self.time_travel_enabled {
            debug!("タイムトラベルデバッグは既に無効です");
            return Ok(());
        }
        
        // タイムトラベルデバッグを無効化
        self.time_travel_enabled = false;
        
        // 記録ファイルを閉じる
        if let Some(file) = self.recording_file.take() {
            drop(file);
        }
        
        // 状態変更イベントを送信
        if let Some(event_tx) = &self.event_tx {
            event_tx.send(DebugEvent::TimeTravelStateChanged {
                enabled: false,
                direction: TimeDirection::Forward,
                current_snapshot_id: None,
            }).await.map_err(|_| anyhow!("イベント送信に失敗"))?;
        }
        
        debug!("タイムトラベルデバッグを無効化しました");
        Ok(())
    }

    /// 現在の状態を記録
    async fn handle_record_state(&mut self) -> Result<usize> {
        debug!("現在の状態を記録します");
        
        if !self.time_travel_enabled {
            return Err(anyhow!("タイムトラベルデバッグが有効ではありません"));
        }
        
        // 現在の状態を記録
        let snapshot_id = self.record_current_state().await?;
        
        debug!("スナップショットを記録しました: ID={}", snapshot_id);
        Ok(snapshot_id)
    }

    /// 現在の実行状態を記録
    async fn record_current_state(&mut self) -> Result<usize> {
        // 現在の状態を収集
        let id = self.execution_history.len();
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
        
        // 現在の実行位置を取得
        let (source_path, line, column) = self.get_current_location()?;
        let source_location = (source_path.clone(), line, column);
        
        // 現在の関数名を取得
        let function_name = self.get_current_function_name().unwrap_or_else(|| "unknown".to_string());
        
        // スタックトレースを取得
        let stack_frames = self.get_stack_trace(self.current_thread_id)?;
        
        // 変数状態を取得
        let variables = self.collect_all_variables().await?;
        
        // メモリダンプを取得
        let memory_dump = self.dump_memory_regions()?;
        
        // レジスタ状態を取得
        let registers = self.get_registers()?;
        
        // スレッド情報を取得
        let threads = self.threads.clone();
        
        // スナップショットを作成
        let snapshot = ExecutionSnapshot {
            id,
            timestamp,
            source_location,
            function_name,
            stack_frames,
            variables,
            memory_dump,
            registers,
            threads,
            annotation: None,
        };
        
        // スナップショットを記録
        self.execution_history.push(snapshot.clone());
        self.current_snapshot_index = id;
        
        // 記録ファイルに書き込み
        if let Some(file) = &mut self.recording_file {
            let snapshot_json = serde_json::to_string(&snapshot)?;
            writeln!(file, "{}", snapshot_json)?;
            file.flush()?;
        }
        
        // 状態変更イベントを送信
        if let Some(event_tx) = &self.event_tx {
            let event = DebugEvent::SnapshotNavigated {
                snapshot_id: id,
                timestamp,
                source_location: (source_path.to_string_lossy().to_string(), line, column),
                function_name: snapshot.function_name.clone(),
            };
            
            event_tx.send(event).await
                .map_err(|_| anyhow!("イベント送信に失敗"))?;
        }
        
        Ok(id)
    }

    /// 指定したスナップショットに移動
    async fn handle_goto_snapshot(&mut self, snapshot_id: usize) -> Result<()> {
        debug!("スナップショット{}に移動します", snapshot_id);
        
        if !self.time_travel_enabled {
            return Err(anyhow!("タイムトラベルデバッグが有効ではありません"));
        }
        
        // スナップショットが存在するか確認
        if snapshot_id >= self.execution_history.len() {
            return Err(anyhow!("スナップショットが存在しません: ID={}", snapshot_id));
        }
        
        // 現在と同じスナップショットの場合は何もしない
        if self.current_snapshot_index == snapshot_id {
            debug!("すでに指定されたスナップショットにいます: ID={}", snapshot_id);
            return Ok(());
        }
        
        // 時間移動の方向を設定
        self.time_direction = if snapshot_id > self.current_snapshot_index {
            TimeDirection::Forward
        } else {
            TimeDirection::Backward
        };
        
        // スナップショットに移動
        self.restore_snapshot(snapshot_id).await?;
        
        // 移動完了
        self.current_snapshot_index = snapshot_id;
        
        debug!("スナップショット{}に移動しました", snapshot_id);
        Ok(())
    }

    /// 前のスナップショットに移動
    async fn handle_step_back(&mut self) -> Result<()> {
        debug!("前のスナップショットに移動します");
        
        if !self.time_travel_enabled {
            return Err(anyhow!("タイムトラベルデバッグが有効ではありません"));
        }
        
        // 前のスナップショットが存在するか確認
        if self.current_snapshot_index == 0 {
            return Err(anyhow!("これ以上前のスナップショットはありません"));
        }
        
        // 時間移動の方向を設定
        self.time_direction = TimeDirection::Backward;
        
        // 前のスナップショットに移動
        let prev_snapshot_id = self.current_snapshot_index - 1;
        self.restore_snapshot(prev_snapshot_id).await?;
        
        // 移動完了
        self.current_snapshot_index = prev_snapshot_id;
        
        debug!("前のスナップショットに移動しました: ID={}", prev_snapshot_id);
        Ok(())
    }

    /// 次のスナップショットに移動
    async fn handle_step_forward(&mut self) -> Result<()> {
        debug!("次のスナップショットに移動します");
        
        if !self.time_travel_enabled {
            return Err(anyhow!("タイムトラベルデバッグが有効ではありません"));
        }
        
        // 次のスナップショットが存在するか確認
        if self.current_snapshot_index + 1 >= self.execution_history.len() {
            return Err(anyhow!("これ以上次のスナップショットはありません"));
        }
        
        // 時間移動の方向を設定
        self.time_direction = TimeDirection::Forward;
        
        // 次のスナップショットに移動
        let next_snapshot_id = self.current_snapshot_index + 1;
        self.restore_snapshot(next_snapshot_id).await?;
        
        // 移動完了
        self.current_snapshot_index = next_snapshot_id;
        
        debug!("次のスナップショットに移動しました: ID={}", next_snapshot_id);
        Ok(())
    }

    /// スナップショットを復元
    async fn restore_snapshot(&mut self, snapshot_id: usize) -> Result<()> {
        // スナップショットを取得
        let snapshot = &self.execution_history[snapshot_id];
        
        // デバッグセッションを更新
        let mut session = self.session.lock().map_err(|_| anyhow!("セッションのロックに失敗"))?;
        session.status = ProcessStatus::Stopped;
        session.stop_reason = Some(StopReason::Step);
        
        // 現在位置に該当するスタックフレームを設定
        if !snapshot.stack_frames.is_empty() {
            session.set_stack_frames(self.current_thread_id, snapshot.stack_frames.clone());
        }
        
        // 変数情報を設定
        let variables = snapshot.variables.values().cloned().collect::<Vec<_>>();
        if !variables.is_empty() {
            session.add_variables(variables);
        }
        
        // スレッド情報を更新
        session.threads.clear();
        for (id, thread) in &snapshot.threads {
            session.threads.insert(*id, thread.clone());
        }
        
        // 実行停止イベントを発行
        drop(session); // セッションのロックを解放
        
        if let Some(event_tx) = &self.event_tx {
            event_tx.send(DebugEvent::Stopped {
                thread_id: self.current_thread_id,
                reason: StopReason::Step,
            }).await.map_err(|_| anyhow!("イベント送信に失敗"))?;
            
            // スナップショット移動イベントを送信
            let (path, line, column) = &snapshot.source_location;
            event_tx.send(DebugEvent::SnapshotNavigated {
                snapshot_id: snapshot.id,
                timestamp: snapshot.timestamp,
                source_location: (path.to_string_lossy().to_string(), *line, *column),
                function_name: snapshot.function_name.clone(),
            }).await.map_err(|_| anyhow!("イベント送信に失敗"))?;
        }
        
        Ok(())
    }

    /// スナップショットの一覧を取得
    async fn handle_list_snapshots(&mut self) -> Result<Vec<(usize, String, u64)>> {
        debug!("スナップショットの一覧を取得します");
        
        if !self.time_travel_enabled {
            return Err(anyhow!("タイムトラベルデバッグが有効ではありません"));
        }
        
        // スナップショット情報を収集
        let mut snapshots = Vec::new();
        for snapshot in &self.execution_history {
            let (path, line, _) = &snapshot.source_location;
            let location = format!("{}:{} - {}", 
                path.file_name().unwrap_or_default().to_string_lossy(), 
                line, 
                snapshot.function_name);
                
            snapshots.push((snapshot.id, location, snapshot.timestamp));
        }
        
        debug!("スナップショット一覧: {} 件", snapshots.len());
        Ok(snapshots)
    }

    /// スナップショットに注釈を追加
    async fn handle_annotate_snapshot(&mut self, snapshot_id: usize, annotation: String) -> Result<()> {
        debug!("スナップショット{}に注釈を追加します: {}", snapshot_id, annotation);
        
        if !self.time_travel_enabled {
            return Err(anyhow!("タイムトラベルデバッグが有効ではありません"));
        }
        
        // スナップショットが存在するか確認
        if snapshot_id >= self.execution_history.len() {
            return Err(anyhow!("スナップショットが存在しません: ID={}", snapshot_id));
        }
        
        // 注釈を追加
        self.execution_history[snapshot_id].annotation = Some(annotation);
        
        debug!("スナップショット{}に注釈を追加しました", snapshot_id);
        Ok(())
    }

    /// プロセス内のメモリ領域をダンプ
    fn dump_memory_regions(&self) -> Result<HashMap<u64, Vec<u8>>> {
        let mut memory_dump = HashMap::new();
        
        if let Some(pid) = self.target_pid {
            // メモリマップファイルを読み込み
            let maps_path = format!("/proc/{}/maps", pid);
            let maps_content = fs::read_to_string(maps_path)?;
            
            // 各メモリ領域を解析
            for line in maps_content.lines() {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 5 {
                    let address_range = parts[0];
                    let permissions = parts[1];
                    
                    // 読み取り可能な領域のみを対象とする
                    if permissions.contains('r') {
                        let addresses: Vec<&str> = address_range.split('-').collect();
                        if addresses.len() == 2 {
                            let start_addr = u64::from_str_radix(addresses[0], 16)?;
                            let end_addr = u64::from_str_radix(addresses[1], 16)?;
                            
                            // サイズが大きすぎる場合はスキップ（最大1MB）
                            let size = end_addr - start_addr;
                            if size > 0 && size <= 1024 * 1024 {
                                // メモリを読み取り
                                let mem_path = format!("/proc/{}/mem", pid);
                                let mut mem_file = File::open(mem_path)?;
                                mem_file.seek(std::io::SeekFrom::Start(start_addr))?;
                                
                                let mut buffer = vec![0u8; size as usize];
                                // 読み取りエラーは無視（一部のメモリ領域は読み取れない場合がある）
                                if let Ok(_) = mem_file.read_exact(&mut buffer) {
                                    memory_dump.insert(start_addr, buffer);
                                }
                            }
                        }
                    }
                }
            }
        }
        
        Ok(memory_dump)
    }

    /// プロセスのレジスタ値を取得
    fn get_registers(&self) -> Result<HashMap<String, u64>> {
        let mut registers = HashMap::new();
        
        if let Some(pid) = self.target_pid {
            // レジスタを読み取り
            unsafe {
                let regs = ptrace::getregs(pid)?;
                
                // x86_64の場合の例
                registers.insert("rax".to_string(), regs.rax);
                registers.insert("rbx".to_string(), regs.rbx);
                registers.insert("rcx".to_string(), regs.rcx);
                registers.insert("rdx".to_string(), regs.rdx);
                registers.insert("rsi".to_string(), regs.rsi);
                registers.insert("rdi".to_string(), regs.rdi);
                registers.insert("rbp".to_string(), regs.rbp);
                registers.insert("rsp".to_string(), regs.rsp);
                registers.insert("rip".to_string(), regs.rip);
                registers.insert("r8".to_string(), regs.r8);
                registers.insert("r9".to_string(), regs.r9);
                registers.insert("r10".to_string(), regs.r10);
                registers.insert("r11".to_string(), regs.r11);
                registers.insert("r12".to_string(), regs.r12);
                registers.insert("r13".to_string(), regs.r13);
                registers.insert("r14".to_string(), regs.r14);
                registers.insert("r15".to_string(), regs.r15);
                registers.insert("eflags".to_string(), regs.eflags as u64);
            }
        }
        
        Ok(registers)
    }

    /// 現在の関数名を取得
    fn get_current_function_name(&self) -> Option<String> {
        if let Some(pid) = self.target_pid {
            // レジスタを読み取り
            if let Ok(regs) = unsafe { ptrace::getregs(pid) } {
                // プログラムカウンタの値を取得
                let pc = regs.rip;
                
                // シンボル情報から関数名を検索
                for (name, addr) in &self.debug_symbols.symbols {
                    // アドレスの範囲内に収まっているか確認
                    // 簡易的な実装のため、正確ではない場合がある
                    if pc >= *addr && pc < *addr + 1024 {
                        return Some(name.clone());
                    }
                }
            }
        }
        
        None
    }

    /// 全ての変数情報を収集
    async fn collect_all_variables(&self) -> Result<HashMap<String, Variable>> {
        let mut variables = HashMap::new();
        
        // グローバル変数を収集
        let globals = self.collect_global_variables().await?;
        for var in globals {
            variables.insert(var.name.clone(), var);
        }
        
        // ローカル変数を収集
        let locals = self.collect_local_variables().await?;
        for var in locals {
            variables.insert(var.name.clone(), var);
        }
        
        Ok(variables)
    }

    /// グローバル変数を収集
    async fn collect_global_variables(&self) -> Result<Vec<Variable>> {
        // 実際の実装では、シンボル情報からグローバル変数を抽出し、
        // メモリから値を取得する処理を実装
        
        // ここではダミーデータを返す
        let mut globals = Vec::new();
        
        globals.push(Variable {
            name: "VERSION".to_string(),
            type_name: "string".to_string(),
            value: "\"1.0.0\"".to_string(),
            kind: VariableKind::Primitive,
            variable_reference: None,
            memory_reference: None,
        });
        
        globals.push(Variable {
            name: "DEBUG_MODE".to_string(),
            type_name: "bool".to_string(),
            value: "true".to_string(),
            kind: VariableKind::Primitive,
            variable_reference: None,
            memory_reference: None,
        });
        
        Ok(globals)
    }

    /// ローカル変数を収集
    async fn collect_local_variables(&self) -> Result<Vec<Variable>> {
        // 実際の実装では、スタックフレーム情報とデバッグ情報を使用して
        // ローカル変数を特定し、値を取得する処理を実装
        
        // ここではダミーデータを返す
        let mut locals = Vec::new();
        
        locals.push(Variable {
            name: "count".to_string(),
            type_name: "int".to_string(),
            value: "42".to_string(),
            kind: VariableKind::Primitive,
            variable_reference: None,
            memory_reference: None,
        });
        
        locals.push(Variable {
            name: "message".to_string(),
            type_name: "string".to_string(),
            value: "\"Hello, World!\"".to_string(),
            kind: VariableKind::Primitive,
            variable_reference: None,
            memory_reference: None,
        });
        
        Ok(locals)
    }

    /// 状態記録タスクを開始
    fn start_state_recording_task(&mut self) {
        // タイムトラベルが有効でない場合は何もしない
        if !self.time_travel_enabled {
            return;
        }
        
        // 既存のタスクがあれば中止
        if let Some(task) = self.execution_task.take() {
            task.abort();
        }
        
        let event_tx = self.event_tx.clone();
        let pid = self.target_pid;
        let recording_interval = self.recording_interval_ms;
        let time_travel_enabled = Arc::new(Mutex::new(self.time_travel_enabled));
        let debug_symbols = self.debug_symbols.clone();
        let session_clone = self.session.clone();
        
        // 状態記録タスクを開始
        self.execution_task = Some(tokio::spawn(async move {
            let enabled = time_travel_enabled.clone();
            let mut snapshot_id_counter = 0;
            
            loop {
                // 一定間隔でスリープ
                sleep(Duration::from_millis(recording_interval)).await;
                
                // タイムトラベルが無効になったら終了
                if !*enabled.lock().unwrap() {
                    break;
                }
                
                // プロセスが実行中かチェック
                if let Some(pid) = pid {
                    // プロセスが存在するか確認
                    if let Ok(status) = waitpid(pid, Some(nix::sys::wait::WaitPidFlag::WNOHANG)) {
                        match status {
                            WaitStatus::StillAlive => {
                                // プロセスは実行中
                                // 一時的に実行を停止してスナップショットを取得
                                match nix::sys::signal::kill(pid, nix::sys::signal::Signal::SIGSTOP) {
                                    Ok(_) => {
                                        // プロセスが停止するのを待つ
                                        match waitpid(pid, None) {
                                            Ok(WaitStatus::Stopped(_, _)) => {
                                                // 現在の実行位置を取得
                                                let regs = match ptrace::getregs(pid) {
                                                    Ok(regs) => regs,
                                                    Err(e) => {
                                                        error!("レジスタ取得エラー: {}", e);
                                                        // プロセスを再開して次のループへ
                                                        let _ = ptrace::cont(pid, None);
                                                        continue;
                                                    }
                                                };
                                                
                                                // 現在のプログラムカウンタを取得
                                                let pc = regs.rip as u64;
                                                
                                                // プログラムカウンタからソース位置を取得
                                                let source_location = debug_symbols.get_location_from_addr(pc)
                                                    .unwrap_or_else(|| (PathBuf::from("unknown"), 0));
                                                
                                                // 関数名を取得（可能であれば）
                                                let function_name = {
                                                    let mut name = "unknown".to_string();
                                                    // 実際には逆アセンブルして関数名を取得する処理が必要
                                                    // ここでは簡易的に実装
                                                    name
                                                };
                                                
                                                // スタックトレースを取得
                                                let stack_frames = {
                                                    let mut frames = Vec::new();
                                                    let mut current_bp = regs.rbp;
                                                    
                                                    // スタックフレームを最大10個まで取得
                                                    for i in 0..10 {
                                                        if current_bp == 0 {
                                                            break;
                                                        }
                                                        
                                                        // 戻りアドレスを読み取り
                                                        let return_addr = match ptrace::read(pid, (current_bp + 8) as ptrace::AddressType) {
                                                            Ok(addr) => addr as u64,
                                                            Err(_) => break,
                                                        };
                                                        
                                                        // 戻りアドレスからソース位置を取得
                                                        if let Some((source_file, line)) = debug_symbols.get_location_from_addr(return_addr) {
                                                            frames.push(StackFrame {
                                                                id: i,
                                                                name: format!("frame_{}", i),
                                                                source: Some(source_file.to_string_lossy().to_string()),
                                                                line: Some(line),
                                                                column: None,
                                                                end_line: None,
                                                                end_column: None,
                                                                instruction_pointer_reference: Some(format!("0x{:x}", return_addr)),
                                                                module_id: None,
                                                                presentation_hint: None,
                                                            });
                                                        }
                                                        
                                                        // 次のベースポインタを読み取り
                                                        current_bp = match ptrace::read(pid, current_bp as ptrace::AddressType) {
                                                            Ok(bp) => bp as u64,
                                                            Err(_) => break,
                                                        };
                                                    }
                                                    
                                                    frames
                                                };
                                                
                                                // 変数情報を取得
                                                let variables = {
                                                    let mut vars = HashMap::new();
                                                    
                                                    // ローカル変数を取得（スタックフレームから）
                                                    if !stack_frames.is_empty() {
                                                        let frame_bp = regs.rbp;
                                                        
                                                        // デバッグ情報からローカル変数の位置を取得
                                                        // 実際には DWARF 情報を解析する必要がある
                                                        // ここでは簡易的に実装
                                                        
                                                        // 例: 第一引数 (RDI レジスタ)
                                                        vars.insert("arg1".to_string(), Variable {
                                                            name: "arg1".to_string(),
                                                            value: format!("{}", regs.rdi),
                                                            type_name: Some("int".to_string()),
                                                            variables: Vec::new(),
                                                            named_variables: None,
                                                            indexed_variables: None,
                                                            memory_reference: None,
                                                            evaluate_name: None,
                                                            variable_reference: 0,
                                                        });
                                                        
                                                        // 例: 第二引数 (RSI レジスタ)
                                                        vars.insert("arg2".to_string(), Variable {
                                                            name: "arg2".to_string(),
                                                            value: format!("{}", regs.rsi),
                                                            type_name: Some("int".to_string()),
                                                            variables: Vec::new(),
                                                            named_variables: None,
                                                            indexed_variables: None,
                                                            memory_reference: None,
                                                            evaluate_name: None,
                                                            variable_reference: 0,
                                                        });
                                                    }
                                                    
                                                    // グローバル変数も取得する場合はここに追加
                                                    
                                                    vars
                                                };
                                                
                                                // メモリダンプを取得
                                                let memory_dump = {
                                                    let mut dump = HashMap::new();
                                                    
                                                    // スタック領域を読み取り
                                                    let stack_start = regs.rsp;
                                                    let stack_size = 1024; // 1KB分のスタックを読み取り
                                                    
                                                    let mut stack_data = Vec::with_capacity(stack_size as usize);
                                                    for offset in 0..(stack_size / 8) {
                                                        if let Ok(data) = ptrace::read(pid, (stack_start + offset * 8) as ptrace::AddressType) {
                                                            let bytes = data.to_le_bytes();
                                                            stack_data.extend_from_slice(&bytes);
                                                        } else {
                                                            break;
                                                        }
                                                    }
                                                    
                                                    dump.insert(stack_start, stack_data);
                                                    
                                                    // 必要に応じて他のメモリ領域も取得
                                                    
                                                    dump
                                                };
                                                
                                                // レジスタ状態を取得
                                                let registers = {
                                                    let mut regs_map = HashMap::new();
                                                    regs_map.insert("rax".to_string(), regs.rax);
                                                    regs_map.insert("rbx".to_string(), regs.rbx);
                                                    regs_map.insert("rcx".to_string(), regs.rcx);
                                                    regs_map.insert("rdx".to_string(), regs.rdx);
                                                    regs_map.insert("rdi".to_string(), regs.rdi);
                                                    regs_map.insert("rsi".to_string(), regs.rsi);
                                                    regs_map.insert("rbp".to_string(), regs.rbp);
                                                    regs_map.insert("rsp".to_string(), regs.rsp);
                                                    regs_map.insert("r8".to_string(), regs.r8);
                                                    regs_map.insert("r9".to_string(), regs.r9);
                                                    regs_map.insert("r10".to_string(), regs.r10);
                                                    regs_map.insert("r11".to_string(), regs.r11);
                                                    regs_map.insert("r12".to_string(), regs.r12);
                                                    regs_map.insert("r13".to_string(), regs.r13);
                                                    regs_map.insert("r14".to_string(), regs.r14);
                                                    regs_map.insert("r15".to_string(), regs.r15);
                                                    regs_map.insert("rip".to_string(), regs.rip);
                                                    regs_map.insert("eflags".to_string(), regs.eflags as u64);
                                                    regs_map
                                                };
                                                
                                                // スレッド情報を取得
                                                let threads = {
                                                    let mut thread_map = HashMap::new();
                                                    thread_map.insert(1, Thread {
                                                        id: 1,
                                                        name: "main".to_string(),
                                                        status: "stopped".to_string(),
                                                    });
                                                    
                                                    // 実際にはプロセスのスレッド一覧を取得
                                                    
                                                    thread_map
                                                };
                                                
                                                // スナップショットを作成
                                                let snapshot_id = snapshot_id_counter;
                                                snapshot_id_counter += 1;
                                                
                                                let timestamp = SystemTime::now()
                                                    .duration_since(UNIX_EPOCH)
                                                    .unwrap_or_default()
                                                    .as_secs();
                                                
                                                let snapshot = ExecutionSnapshot {
                                                    id: snapshot_id,
                                                    timestamp,
                                                    source_location: (source_location.0.clone(), source_location.1, None),
                                                    function_name,
                                                    stack_frames,
                                                    variables,
                                                    memory_dump,
                                                    registers,
                                                    threads,
                                                    annotation: None,
                                                };
                                                
                                                // スナップショットをセッションに保存
                                                if let Ok(mut session) = session_clone.lock() {
                                                    session.add_snapshot(snapshot.clone());
                                                }
                                                
                                                // スナップショット記録イベントを送信
                                                if let Some(event_tx) = &event_tx {
                                                    let event = DebugEvent::SnapshotNavigated {
                                                        snapshot_id,
                                                        timestamp,
                                                        source_location: (
                                                            source_location.0.to_string_lossy().to_string(),
                                                            source_location.1,
                                                            None,
                                                        ),
                                                        function_name: snapshot.function_name.clone(),
                                                    };
                                                    
                                                    let _ = event_tx.try_send(event);
                                                }
                                                
                                                // プロセスを再開
                                                let _ = ptrace::cont(pid, None);
                                            }
                                            Ok(status) => {
                                                error!("プロセスが予期しない状態になりました: {:?}", status);
                                                // プロセスを再開
                                                let _ = ptrace::cont(pid, None);
                                            }
                                            Err(e) => {
                                                error!("プロセス待機エラー: {}", e);
                                                // プロセスを再開試行
                                                let _ = ptrace::cont(pid, None);
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        error!("プロセスの一時停止に失敗: {}", e);
                                    }
                                }
                            }
                            _ => {
                                // プロセスが終了または異常状態
                                break;
                            }
                        }
                    } else {
                        // PIDがない場合は終了
                        break;
                    }
                }
            }
        }));
    }
} 