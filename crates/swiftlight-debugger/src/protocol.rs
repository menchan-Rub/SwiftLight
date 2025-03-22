/*
 * SwiftLight デバッガー - デバッグプロトコル
 *
 * このモジュールは、Debug Adapter Protocol (DAP) の実装と、
 * SwiftLightのデバッグ情報をやり取りするためのデータ構造を提供します。
 */

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use anyhow::{Result, Context, anyhow};
use log::{debug, info, warn, error};
use thiserror::Error;
use serde_json::json;
use base64;
use gimli::{Dwarf, EndianSlice, RunTimeEndian};
use object::{Object, ObjectSection};
use memmap2::Mmap;
use nix::sys::ptrace;
use nix::unistd::Pid;
use nix::sys::signal::{Signal, kill};
use nix::sys::wait::{waitpid, WaitStatus};
use pest::Parser;
use pest::iterators::Pair;
use pest_derive::Parser;
use serde_json::Map;
use std::sync::{Arc, Mutex};
use tokio::sync::{mpsc::Sender, oneshot};
use addr2line;
use goblin;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;

use crate::engine::{DebugCommand, ExecutionMode, ExecutionSnapshot};

/// 式パーサー
#[derive(Parser)]
#[grammar = "expression.pest"]
pub struct ExpressionParser;

/// パーサールール
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Rule {
    // 全体の式
    expression,
    // 二項演算式
    binary_expr,
    binary_op,
    // 算術演算子
    add,
    subtract,
    multiply,
    divide,
    modulo,
    // 比較演算子
    equal,
    not_equal,
    less_than,
    less_equal,
    greater_than,
    greater_equal,
    // ビット演算子
    bit_and,
    bit_or,
    bit_xor,
    shift_left,
    shift_right,
    // 論理演算子
    and,
    or,
    // 単項演算式
    unary_expr,
    unary_op,
    negate,
    not,
    bit_not,
    deref,
    addr_of,
    // 基本式
    primary_expr,
    // リテラル
    literal,
    null,
    boolean,
    integer,
    float,
    string,
    char,
    // 変数参照
    variable,
    // 関数呼び出し
    function_call,
    // メンバーアクセス
    member_access,
    // インデックスアクセス
    index_access,
    // 条件演算子
    conditional,
    // キャスト式
    cast,
    // 型名
    type_name,
}

/// 二項演算子
#[derive(Debug, Clone, PartialEq)]
pub enum BinaryOperator {
    // 算術演算子
    Add,        // +
    Subtract,   // -
    Multiply,   // *
    Divide,     // /
    Modulo,     // %
    
    // 比較演算子
    Equal,          // ==
    NotEqual,       // !=
    LessThan,       // <
    LessEqual,      // <=
    GreaterThan,    // >
    GreaterEqual,   // >=
    
    // ビット演算子
    BitAnd,     // &
    BitOr,      // |
    BitXor,     // ^
    ShiftLeft,  // <<
    ShiftRight, // >>
    
    // 論理演算子
    And,        // &&
    Or,         // ||
}

/// 単項演算子
#[derive(Debug, Clone, PartialEq)]
pub enum UnaryOperator {
    Negate,     // -
    Not,        // !
    BitNot,     // ~
    Deref,      // *
    AddrOf,     // &
}

/// リテラル値
#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    Null,
    Boolean(bool),
    Integer(i64),
    Float(f64),
    String(String),
    Char(char),
}

/// 式の抽象構文木
#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    /// リテラル値
    Literal(Literal),
    
    /// 変数参照
    Variable(String),
    
    /// 二項演算
    Binary {
        operator: BinaryOperator,
        left: Box<Expression>,
        right: Box<Expression>,
    },
    
    /// 単項演算
    Unary {
        operator: UnaryOperator,
        operand: Box<Expression>,
    },
    
    /// 関数呼び出し
    FunctionCall {
        function: String,
        arguments: Vec<Expression>,
    },
    
    /// メンバーアクセス (obj.member)
    MemberAccess {
        object: Box<Expression>,
        member: String,
    },
    
    /// インデックスアクセス (arr[idx])
    IndexAccess {
        array: Box<Expression>,
        index: Box<Expression>,
    },
    
    /// 条件式 (cond ? then : else)
    Conditional {
        condition: Box<Expression>,
        then_branch: Box<Expression>,
        else_branch: Box<Expression>,
    },
    
    /// キャスト式 (expr as Type)
    Cast {
        expression: Box<Expression>,
        target_type: String,
    },
}

/// デバッガーエラー
#[derive(Error, Debug)]
pub enum DebugError {
    #[error("不正なリクエスト: {0}")]
    InvalidRequest(String),
    
    #[error("接続エラー: {0}")]
    ConnectionError(String),
    
    #[error("デバッグターゲットの起動に失敗: {0}")]
    LaunchFailed(String),
    
    #[error("ブレークポイントの設定に失敗: {0}")]
    BreakpointFailed(String),
    
    #[error("変数評価エラー: {0}")]
    EvaluationError(String),
    
    #[error("実行時エラー: {0}")]
    RuntimeError(String),
    
    #[error("内部エラー: {0}")]
    InternalError(String),
    
    #[error("タイムトラベル操作エラー: {0}")]
    TimeTravelError(String),
    
    #[error("無効な状態: {0}")]
    InvalidState(String),
    
    #[error("スレッドが見つかりません: {0}")]
    ThreadNotFound(usize),
    
    #[error("変数が見つかりません: {0}")]
    VariableNotFound(String),
    
    #[error("プロセスが終了: {0}")]
    ProcessExited(i32),
    
    #[error("無効なスタックフレーム: {0}")]
    InvalidStackFrame(usize),
    
    #[error("式のエラー: {0}")]
    ExpressionError(String),
}

/// プロセスステータス
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProcessStatus {
    /// 初期化中
    Initializing,
    
    /// 準備完了
    Ready,
    
    /// 実行中
    Running,
    
    /// 停止中
    Stopped,
    
    /// 終了
    Terminated,
    
    /// 終了
    Exited(i32),
}

/// 停止理由
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum StopReason {
    /// ブレークポイントによる停止
    Breakpoint { id: usize },
    
    /// ステップ実行による停止
    Step,
    
    /// 例外による停止
    Exception { description: String },
    
    /// 一時停止（ユーザーリクエスト）
    Pause,
    
    /// エントリポイントでの停止
    Entry,
    
    /// 実行終了
    Exit { exit_code: i32 },
    
    /// タイムトラベル（過去または未来に移動した）
    TimeTravel { snapshot_id: usize },
}

/// 変数の種類
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VariableKind {
    /// プリミティブ型
    Primitive,
    
    /// 配列
    Array,
    
    /// オブジェクト
    Object,
    
    /// クラス
    Class,
    
    /// 関数
    Function,
    
    /// その他
    Other,
    
    /// ローカル変数
    Local,
    
    /// グローバル変数
    Global,
    
    /// レジスタ
    Register,
    
    /// 一時変数
    Temporary,
}

/// 変数情報
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Variable {
    /// 変数名
    pub name: String,
    
    /// 型名
    pub type_name: String,
    
    /// 値の文字列表現
    pub value: String,
    
    /// 変数の種類
    pub kind: VariableKind,
    
    /// 子変数がある場合のID
    pub variable_reference: Option<usize>,
    
    /// メモリ内の位置
    pub memory_reference: Option<String>,
    
    /// 親変数がある場合のID
    pub parent_id: Option<usize>,
    
    /// 子変数の数
    pub indexed_variables: usize,
    
    /// 子変数の数
    pub named_variables: usize,
    
    /// 評価済みかどうか
    pub evaluated: bool,
}

/// スタックフレーム情報
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StackFrame {
    /// フレームID
    pub id: usize,
    
    /// 関数名
    pub name: String,
    
    /// ソースファイルパス
    pub source_path: PathBuf,
    
    /// 行番号
    pub line: usize,
    
    /// 列番号
    pub column: usize,
    
    /// 表示用の追加情報
    pub presentation_hint: Option<String>,
    
    /// モジュール名
    pub module_name: Option<String>,
    
    /// スレッドID
    pub thread_id: usize,
}

/// ブレークポイント情報
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Breakpoint {
    /// ブレークポイントID
    pub id: usize,
    
    /// 有効かどうか
    pub verified: bool,
    
    /// ソースファイルパス
    pub source_path: PathBuf,
    
    /// 行番号
    pub line: usize,
    
    /// 列番号（オプション）
    pub column: Option<usize>,
    
    /// 条件式（オプション）
    pub condition: Option<String>,
    
    /// ヒット回数条件（オプション）
    pub hit_condition: Option<String>,
    
    /// ログメッセージ（オプション）
    pub log_message: Option<String>,
    
    /// アドレス（オプション）
    pub address: Option<usize>,
    
    /// 元の命令（オプション）
    pub original_byte: Option<u8>,
    
    /// 一時的かどうか
    pub temporary: bool,
}

/// スレッド情報
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Thread {
    /// スレッドID
    pub id: usize,
    
    /// スレッド名
    pub name: String,
}

/// デバッグターゲット設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebugConfiguration {
    /// 名前
    pub name: String,
    
    /// 種類（swiftlight）
    pub type_name: String,
    
    /// リクエスト種類 (launch or attach)
    pub request: String,
    
    /// プログラムパス
    pub program: PathBuf,
    
    /// 作業ディレクトリ
    pub cwd: Option<PathBuf>,
    
    /// 引数
    pub args: Vec<String>,
    
    /// 環境変数
    pub env: HashMap<String, String>,
    
    /// リモートデバッグの場合のアドレス
    pub address: Option<String>,
    
    /// リモートデバッグの場合のポート
    pub port: Option<u16>,
    
    /// ソースマップ
    pub source_map: HashMap<String, String>,
    
    /// ストップオンエントリーを有効にするかどうか
    pub stop_on_entry: bool,
}

/// デバッグセッション状態
#[derive(Debug)]
pub struct DebugSession {
    /// 設定
    pub config: DebugConfiguration,
    
    /// プロセスステータス
    pub status: ProcessStatus,
    
    /// ブレークポイントリスト
    pub breakpoints: HashMap<usize, Breakpoint>,
    
    /// スレッドリスト
    pub threads: HashMap<usize, Thread>,
    
    /// 現在のスレッドID
    pub current_thread_id: Option<usize>,
    
    /// スタックフレーム（スレッドIDをキーとする）
    pub stack_frames: HashMap<usize, Vec<StackFrame>>,
    
    /// 変数（変数参照IDをキーとする）
    pub variables: HashMap<usize, Variable>,
    
    /// 停止理由
    pub stop_reason: Option<StopReason>,
    
    /// プロセスの終了コード
    pub exit_code: Option<i32>,
    
    /// タイムトラベルモードが有効か
    pub time_travel_mode: bool,
    
    /// 現在のスナップショットID
    pub current_snapshot_id: Option<usize>,
    
    /// スナップショット一覧（ID、説明、タイムスタンプ）
    pub snapshots: Vec<(usize, String, u64)>,
    
    /// 次のスレッドID
    pub next_thread_id: usize,
    
    /// 次のブレークポイントID
    pub next_breakpoint_id: usize,
    
    /// 次のスタックフレームID
    pub next_stack_frame_id: usize,
    
    /// 次の変数ID
    pub next_variable_id: usize,
    
    /// プロセスID（オプション）
    pub process_id: Option<usize>,
}

impl Default for DebugSession {
    fn default() -> Self {
        Self {
            config: DebugConfiguration {
                name: "SwiftLight Debug".to_string(),
                type_name: "swiftlight".to_string(),
                request: "launch".to_string(),
                program: PathBuf::new(),
                cwd: None,
                args: Vec::new(),
                env: HashMap::new(),
                address: None,
                port: None,
                source_map: HashMap::new(),
                stop_on_entry: true,
            },
            status: ProcessStatus::Initializing,
            breakpoints: HashMap::new(),
            threads: HashMap::new(),
            current_thread_id: None,
            stack_frames: HashMap::new(),
            variables: HashMap::new(),
            stop_reason: None,
            exit_code: None,
            time_travel_mode: false,
            current_snapshot_id: None,
            snapshots: Vec::new(),
            next_thread_id: 1,
            next_breakpoint_id: 1,
            next_stack_frame_id: 1,
            next_variable_id: 1000,
            process_id: None,
        }
    }
}

impl DebugSession {
    /// 新しいデバッグセッションを作成
    pub fn new(config: DebugConfiguration) -> Self {
        Self {
            config,
            ..Default::default()
        }
    }
    
    /// セッションを初期化
    pub fn initialize(&mut self) -> Result<(), DebugError> {
        debug!("デバッグセッションを初期化中...");
        
        // デフォルトスレッドの作成
        let main_thread = Thread {
            id: 1,
            name: "メインスレッド".to_string(),
        };
        
        self.threads.insert(main_thread.id, main_thread);
        self.current_thread_id = Some(1);
        self.status = ProcessStatus::Ready;
        
        debug!("デバッグセッションの初期化が完了しました");
        Ok(())
    }
    
    /// ブレークポイントを設定
    pub fn set_breakpoint(&mut self, source_path: &Path, line: usize, column: Option<usize>, 
                          condition: Option<String>) -> Result<Breakpoint, DebugError> {
        let id = self.next_breakpoint_id();
        
        let breakpoint = Breakpoint {
            id,
            verified: true,
            source_path: source_path.to_path_buf(),
            line,
            column,
            condition: condition,
            hit_condition: None,
            log_message: None,
            address: None,
            original_byte: None,
            temporary: false,
        };
        
        self.breakpoints.insert(id, breakpoint.clone());
        debug!("ブレークポイントを設定しました: id={}, パス={}, 行={}", 
              id, source_path.display(), line);
        
        Ok(breakpoint)
    }
    
    /// ソースファイルにブレークポイントを設定
    pub fn set_breakpoints(&mut self, source_path: PathBuf, breakpoints: Vec<Breakpoint>) -> Vec<Breakpoint> {
        // 既存のブレークポイントから、同じソースファイルのものを削除
        let existing_ids: Vec<usize> = self.breakpoints.iter()
            .filter(|(_, bp)| bp.source_path == source_path)
            .map(|(id, _)| *id)
            .collect();
        
        for id in existing_ids {
            self.breakpoints.remove(&id);
        }
        
        // 新しいブレークポイントを追加
        let mut verified_breakpoints = Vec::with_capacity(breakpoints.len());
        
        for mut bp in breakpoints {
            // 既に存在するIDは使わずに新しいIDを割り当て
            bp.id = self.next_breakpoint_id();
            
            // ブレークポイントを検証（ソースラインが有効かなど）
            // DWARFデバッグ情報を使用して行番号が有効かを確認する
            bp.verified = if let Some(debug_info) = &self.debug_info {
                // バイナリファイルからDWARF情報を読み取る
                if let Some(binary_path) = &self.binary_path {
                    match self.verify_breakpoint_with_dwarf(binary_path, &bp.source_path, bp.line as u32) {
                        Ok(is_valid) => is_valid,
                        Err(e) => {
                            warn!("DWARFによるブレークポイント検証エラー: {}", e);
                            true // エラーが発生した場合はとりあえず検証済みとする
                        }
                    }
                } else {
                    true // バイナリパスが設定されていない場合は検証済みとみなす
                }
            } else {
                true // デバッグ情報がない場合は検証済みとみなす
            };
            
            // ブレークポイントをセッションに追加
            verified_breakpoints.push(bp.clone());
            self.breakpoints.insert(bp.id, bp);
            
            debug!("ブレークポイントを設定しました: id={}, パス={}, 行={}",
                  bp.id, bp.source_path.display(), bp.line);
        }
        
        // 検証済みブレークポイントを返す
        verified_breakpoints
    }
    
    /// 実行を再開
    pub fn continue_execution(&mut self) {
        if self.status == ProcessStatus::Stopped {
            self.status = ProcessStatus::Running;
            self.stop_reason = None;
            debug!("実行を再開します");
        }
    }
    
    /// スレッドを一時停止
    pub fn pause(&mut self, thread_id: usize) {
        if self.status == ProcessStatus::Running {
            self.status = ProcessStatus::Stopped;
            self.stop_reason = Some(StopReason::Pause);
            debug!("スレッド {} を一時停止しました", thread_id);
        }
    }
    
    /// すべてのスレッドを一時停止
    pub fn pause_all(&mut self) {
        if self.status == ProcessStatus::Running {
            self.status = ProcessStatus::Stopped;
            self.stop_reason = Some(StopReason::Pause);
            debug!("すべてのスレッドを一時停止しました");
        }
    }
    
    /// 設定完了を通知
    pub fn configuration_done(&mut self) {
        debug!("設定が完了しました");
    }
    
    /// スレッド一覧を取得
    pub fn get_threads(&self) -> Vec<&Thread> {
        self.threads.values().collect()
    }
    
    /// 変数情報を追加
    pub fn add_variables(&mut self, variables: Vec<Variable>) -> usize {
        let ref_id = self.next_variable_id();
        self.variables.insert(ref_id, variables);
        ref_id
    }
    
    /// 変数参照から変数情報を取得
    pub fn get_variables(&self, variable_reference: usize) -> Option<&Vec<Variable>> {
        self.variables.get(&variable_reference)
    }
    
    /// スタックフレームを追加
    pub fn set_stack_frames(&mut self, thread_id: usize, frames: Vec<StackFrame>) {
        self.stack_frames.insert(thread_id, frames);
    }
    
    /// スタックフレームを取得
    pub fn get_stack_frames(&self, thread_id: usize) -> Option<&Vec<StackFrame>> {
        self.stack_frames.get(&thread_id)
    }
    
    /// セッションを終了
    pub fn terminate(&mut self, exit_code: i32) {
        self.status = ProcessStatus::Terminated;
        self.exit_code = Some(exit_code);
        info!("デバッグセッションが終了しました: exit_code={}", exit_code);
        
        // 終了時にリソースをクリーンアップ
        self.stack_frames.clear();
        self.variables.clear();
    }
    
    /// 次のブレークポイントIDを取得
    pub fn next_breakpoint_id(&mut self) -> usize {
        let id = self.next_breakpoint_id;
        self.next_breakpoint_id += 1;
        id
    }
    
    /// 次の変数IDを取得
    pub fn next_variable_id(&mut self) -> usize {
        let id = self.next_variable_id;
        self.next_variable_id += 1;
        id
    }
    
    /// スナップショットを追加する
    pub fn add_snapshot(&mut self, snapshot: crate::engine::ExecutionSnapshot) {
        // スナップショットの基本情報をリストに追加
        let description = if let Some(annotation) = snapshot.annotation() {
            format!("{} @ {}", annotation, snapshot.function_name())
        } else {
            format!("{} @ {}", snapshot.source_location().0.to_string_lossy(), snapshot.function_name())
        };
        
        self.snapshots.push((snapshot.id(), description, snapshot.timestamp()));
        
        // 現在のスナップショットIDを設定
        self.current_snapshot_id = Some(snapshot.id());
        
        // スタックフレームを設定
        if !snapshot.stack_frames().is_empty() && self.current_thread_id.is_some() {
            let thread_id = self.current_thread_id.unwrap();
            self.stack_frames.insert(thread_id, snapshot.stack_frames().clone());
        }
        
        // 変数情報を設定
        if !snapshot.variables().is_empty() {
            let variables = snapshot.variables().values().cloned().collect::<Vec<_>>();
            self.add_variables(variables);
        }
        
        debug!("スナップショットを追加しました: ID={}, 時刻={}, 説明={}", 
              snapshot.id(), snapshot.timestamp(), description);
    }
    /// スナップショットに注釈を追加
    pub fn annotate_snapshot(&mut self, snapshot_id: usize, annotation: String) -> Result<(), DebugError> {
        // スナップショットが存在するか確認
        if let Some(pos) = self.snapshots.iter().position(|(id, _, _)| *id == snapshot_id) {
            // 注釈を含む新しい説明を作成
            let (_, old_desc, timestamp) = self.snapshots[pos];
            let new_desc = if old_desc.contains('@') {
                let parts: Vec<&str> = old_desc.split(" @ ").collect();
                format!("{} @ {}", annotation, parts[1])
            } else {
                format!("{} @ unknown", annotation)
            };
            
            // スナップショット情報を更新
            self.snapshots[pos] = (snapshot_id, new_desc, timestamp);
            
            debug!("スナップショットに注釈を追加しました: ID={}, 注釈={}", 
                  snapshot_id, annotation);
            
            Ok(())
        } else {
            Err(DebugError::TimeTravelError(format!(
                "スナップショットが存在しません: ID={}", snapshot_id)))
        }
    }
    
    /// スナップショットを取得
    pub fn get_snapshot(&self, snapshot_id: usize) -> Option<&(usize, String, u64)> {
        self.snapshots.iter().find(|(id, _, _)| *id == snapshot_id)
    }
    
    /// タイムトラベルモードを有効/無効にする
    pub fn set_time_travel_mode(&mut self, enabled: bool) {
        self.time_travel_mode = enabled;
        
        if !enabled {
            // 無効化する場合は関連情報をクリア
            self.current_snapshot_id = None;
        }
        
        debug!("タイムトラベルモードを{}にしました", 
              if enabled { "有効" } else { "無効" });
    }
    
    /// セッションを停止状態にする
    pub fn stop(&mut self, reason: StopReason) {
        self.status = ProcessStatus::Stopped;
        self.stop_reason = Some(reason);
        info!("デバッグセッションが停止しました: {:?}", self.stop_reason);
    }
    
    /// セッションを実行状態にする
    pub fn run(&mut self) {
        self.status = ProcessStatus::Running;
        self.stop_reason = None;
        debug!("デバッグセッションを実行中...");
    }
    
    /// 次のステップを実行（ステップオーバー）
    pub fn step_over(&mut self, thread_id: usize) -> Result<(), DebugError> {
        if self.status != ProcessStatus::Stopped {
            return Err(DebugError::InvalidState("プロセスが停止していません".to_string()));
        }
        
        if !self.threads.contains_key(&thread_id) {
            return Err(DebugError::ThreadNotFound(thread_id));
        }
        
        // 現在の関数内で次の命令を実行
        debug!("スレッド {} でステップオーバー実行", thread_id);
        
        // 現在のスタックフレームを記録
        let current_frame = self.get_stack_frames(thread_id)
            .map_err(|e| DebugError::InternalError(format!("スタックフレーム取得エラー: {}", e)))?
            .get(0)
            .cloned();
        
        // PTRACEを使用して命令単位でステップ実行
        if let Some(process_id) = self.process_id {
            let pid = Pid::from_raw(process_id as i32);
            
            // 現在の命令のアドレスを取得
            let mut regs = ptrace::getregs(pid)
                .map_err(|e| DebugError::InternalError(format!("レジスタ取得失敗: {}", e)))?;
            
            // 次に実行される命令を解析
            let instruction_pointer = regs.rip as usize;
            let instruction = self.read_process_memory(instruction_pointer, 15)?;
            
            // 命令が関数呼び出しかどうかを判定
            let is_call_instruction = instruction[0] == 0xE8 || // CALL rel32
                                      (instruction[0] == 0xFF && (instruction[1] & 0x30) == 0x10); // CALL r/m
            
            if is_call_instruction {
                // 関数呼び出しの場合は、次の命令にブレークポイントを設定
                let next_instruction = instruction_pointer + self.get_instruction_length(&instruction);
                
                // 一時的なブレークポイントを設定
                let temp_bp_id = self.set_temp_breakpoint_at_address(next_instruction)?;
                
                // プロセスを再開
                ptrace::cont(pid, None)
                    .map_err(|e| DebugError::InternalError(format!("プロセス再開失敗: {}", e)))?;
                
                // ブレークポイントに到達するまで待機
                self.wait_for_breakpoint()?;
                
                // 一時的なブレークポイントを削除
                self.remove_breakpoint(temp_bp_id)?;
            } else {
                // 通常の命令の場合は単純なステップ実行
                ptrace::step(pid, None)
                    .map_err(|e| DebugError::InternalError(format!("ステップ実行失敗: {}", e)))?;
                
                // プロセスが停止するまで待機
                match waitpid(pid, None) {
                    Ok(WaitStatus::Stopped(_, Signal::SIGTRAP)) => {
                        // ステップ実行が正常に完了
                        debug!("ステップ実行が完了しました");
                    },
                    Ok(status) => {
                        // その他の理由で停止
                        debug!("プロセスが停止しました: {:?}", status);
                    },
                    Err(e) => {
                        return Err(DebugError::InternalError(format!("waitpid失敗: {}", e)));
                    }
                }
            }
            
            // プロセスの状態を更新
            self.status = ProcessStatus::Stopped;
            self.update_threads_status()?;
            
            Ok(())
        } else {
            Err(DebugError::InternalError("プロセスIDが設定されていません".to_string()))
        }
    }
    
    /// 命令の長さを取得する
    fn get_instruction_length(&self, instruction: &[u8]) -> usize {
        // x86-64命令の長さを決定する
        // 実際の命令長判定は複雑なので、主要な命令パターンをサポート
        
        if instruction.is_empty() {
            return 1; // エラー回避のため
        }
        
        // プレフィックスの確認
        let mut offset = 0;
        
        // グループ1: ロックおよび反復プレフィックス
        if instruction[offset] == 0xF0 || // LOCK
           instruction[offset] == 0xF2 || // REPNE/REPNZ
           instruction[offset] == 0xF3 {  // REP/REPE/REPZ
            offset += 1;
            if offset >= instruction.len() {
                return offset;
            }
        }
        
        // グループ2: セグメントオーバーライドプレフィックス
        if instruction[offset] == 0x2E || // CS
           instruction[offset] == 0x36 || // SS
           instruction[offset] == 0x3E || // DS
           instruction[offset] == 0x26 || // ES
           instruction[offset] == 0x64 || // FS
           instruction[offset] == 0x65 {  // GS
            offset += 1;
            if offset >= instruction.len() {
                return offset;
            }
        }
        
        // グループ3: オペランドサイズオーバーライドプレフィックス
        if instruction[offset] == 0x66 {
            offset += 1;
            if offset >= instruction.len() {
                return offset;
            }
        }
        
        // グループ4: アドレスサイズオーバーライドプレフィックス
        if instruction[offset] == 0x67 {
            offset += 1;
            if offset >= instruction.len() {
                return offset;
            }
        }
        
        // REXプレフィックス（64ビットモード）
        if (instruction[offset] & 0xF0) == 0x40 {
            offset += 1;
            if offset >= instruction.len() {
                return offset;
            }
        }
        
        // オペコード
        if instruction[offset] == 0x0F {
            // 2バイトオペコード
            offset += 1;
            if offset >= instruction.len() {
                return offset;
            }
            
            if instruction[offset] == 0x38 || instruction[offset] == 0x3A {
                // 3バイトオペコード
                offset += 1;
                if offset >= instruction.len() {
                    return offset;
                }
            }
        }
        
        // 通常のオペコード
        offset += 1;
        if offset >= instruction.len() {
            return offset;
        }
        
        // ModR/Mバイトがある場合
        let has_modrm = match instruction[offset - 1] {
            // 一般的なModR/Mが必要なオペコード
            0x00...0x03 | 0x08...0x0B | 0x10...0x13 | 0x18...0x1B |
            0x20...0x23 | 0x28...0x2B | 0x30...0x33 | 0x38...0x3B |
            0x63 | 0x69 | 0x6B | 0x80...0x83 | 0x84 | 0x85 | 0x86 | 0x87 |
            0x88...0x8B | 0x8D | 0x8F | 0xC0...0xC1 | 0xC4...0xC7 |
            0xD0...0xD3 | 0xD8...0xDF | 0xF6 | 0xF7 | 0xFE | 0xFF => true,
            
            // 2バイトオペコードの場合（0x0Fプレフィックス）
            _ => {
                if offset >= 2 && instruction[offset - 2] == 0x0F {
                    // ほとんどの0x0F命令はModR/Mを持つ
                    true
                } else {
                    false
                }
            }
        };
        
        if has_modrm {
            let modrm = instruction[offset];
            offset += 1;
            if offset >= instruction.len() {
                return offset;
            }
            
            let mod_field = (modrm >> 6) & 0x03;
            let rm_field = modrm & 0x07;
            
            // SIBバイト
            if mod_field != 0x03 && rm_field == 0x04 {
                offset += 1; // SIBバイト
                if offset >= instruction.len() {
                    return offset;
                }
            }
            
            // ディスプレイスメント
            match mod_field {
                0x00 => {
                    // mod == 00: レジスタ間接、またはdisp32(rm==101の場合)
                    if rm_field == 0x05 {
                        offset += 4; // disp32
                    }
                },
                0x01 => offset += 1, // mod == 01: 1バイト変位付きレジスタ間接
                0x02 => offset += 4, // mod == 10: 4バイト変位付きレジスタ間接
                _ => {} // mod == 11: レジスタ直接（追加バイトなし）
            }
        }
        
        // 即値
        if instruction[offset - 1] == 0xE8 || instruction[offset - 1] == 0xE9 {
            // CALL rel32, JMP rel32
            offset += 4;
        } else if (instruction[offset - 1] & 0xF0) == 0x70 || instruction[offset - 1] == 0xEB {
            // 条件付きJMP rel8, JMP rel8
            offset += 1;
        } else if instruction[offset - 1] == 0x9A {
            // CALL far
            offset += 6;
        } else if instruction[offset - 1] == 0xEA {
            // JMP far
            offset += 6;
        } else if instruction[offset - 1] == 0x68 {
            // PUSH imm32
            offset += 4;
        } else if instruction[offset - 1] == 0x6A {
            // PUSH imm8
            offset += 1;
        } else if instruction[offset - 1] == 0xC2 || instruction[offset - 1] == 0xCA {
            // RET imm16
            offset += 2;
        }
        
        return offset;
    }
    
    /// 指定されたアドレスに一時的なブレークポイントを設定
    fn set_temp_breakpoint_at_address(&mut self, address: usize) -> Result<usize, DebugError> {
        // 元の命令を保存
        let original_byte = self.read_process_memory(address, 1)?[0];
        
        // ブレークポイント命令（INT3 = 0xCC）を書き込み
        self.write_process_memory(address, &[0xCC])?;
        
        // 一時的なブレークポイント情報を記録
        let bp_id = self.next_breakpoint_id();
        self.breakpoints.insert(bp_id, Breakpoint {
            id: bp_id,
            source_path: PathBuf::new(),
            line: 0,
            column: 0,
            verified: true,
            address: Some(address),
            original_byte: Some(original_byte),
            temporary: true,
            condition: todo!(),
            hit_condition: todo!(),
            log_message: todo!(),
        });
        
        Ok(bp_id)
    }
    
    /// プロセスメモリから読み込む
    fn read_process_memory(&self, address: usize, size: usize) -> Result<Vec<u8>, DebugError> {
        if let Some(process_id) = self.process_id {
            let pid = Pid::from_raw(process_id as i32);
            let mut data = Vec::with_capacity(size);
            
            for i in 0..(size + 7) / 8 {
                let word_address = address + i * 8;
                let word = ptrace::read(pid, word_address as *mut _)
                    .map_err(|e| DebugError::InternalError(format!("メモリ読み込み失敗: {}", e)))?;
                
                // ワードをバイト列に変換
                let bytes = word.to_ne_bytes();
                data.extend_from_slice(&bytes);
            }
            
            // 必要なサイズに切り詰め
            data.truncate(size);
            Ok(data)
        } else {
            Err(DebugError::InternalError("プロセスIDが設定されていません".to_string()))
        }
    }
    
    /// プロセスメモリに書き込む
    fn write_process_memory(&self, address: usize, data: &[u8]) -> Result<(), DebugError> {
        if let Some(process_id) = self.process_id {
            let pid = Pid::from_raw(process_id as i32);
            
            for (i, chunk) in data.chunks(8).enumerate() {
                let word_address = address + i * 8;
                
                // バイト列をワードに変換
                let mut word_bytes = [0u8; 8];
                
                // 現在のワードを読み込み
                let current_word = ptrace::read(pid, word_address as *mut _)
                    .map_err(|e| DebugError::InternalError(format!("メモリ読み込み失敗: {}", e)))?;
                word_bytes = current_word.to_ne_bytes();
                
                // 必要な部分だけ置き換え
                for (j, &byte) in chunk.iter().enumerate() {
                    word_bytes[j] = byte;
                }
                
                // ワードを書き込み
                let word = u64::from_ne_bytes(word_bytes);
                ptrace::write(pid, word_address as *mut _, word as *mut _)
                    .map_err(|e| DebugError::InternalError(format!("メモリ書き込み失敗: {}", e)))?;
            }
            
            Ok(())
        } else {
            Err(DebugError::InternalError("プロセスIDが設定されていません".to_string()))
        }
    }
    
    /// ブレークポイントに達するまで待機
    fn wait_for_breakpoint(&mut self) -> Result<(), DebugError> {
        if let Some(process_id) = self.process_id {
            let pid = Pid::from_raw(process_id as i32);
            
            loop {
                match waitpid(pid, None) {
                    Ok(WaitStatus::Stopped(_, Signal::SIGTRAP)) => {
                        // ブレークポイントに到達
                        debug!("ブレークポイントに到達しました");
                        break;
                    },
                    Ok(WaitStatus::Exited(_, code)) => {
                        // プロセスが終了
                        self.status = ProcessStatus::Exited(code);
                        return Err(DebugError::ProcessExited(code));
                    },
                    Ok(status) => {
                        // その他の理由で停止
                        debug!("プロセスが停止しました: {:?}", status);
                        continue;
                    },
                    Err(e) => {
                        return Err(DebugError::InternalError(format!("waitpid失敗: {}", e)));
                    }
                }
            }
            
            Ok(())
        } else {
            Err(DebugError::InternalError("プロセスIDが設定されていません".to_string()))
        }
    }
    
    /// ステップイン実行
    pub fn step_in(&mut self, thread_id: usize) -> Result<(), DebugError> {
        if self.status != ProcessStatus::Stopped {
            return Err(DebugError::InvalidState("プロセスが停止していません".to_string()));
        }
        
        if !self.threads.contains_key(&thread_id) {
            return Err(DebugError::ThreadNotFound(thread_id));
        }
        
        debug!("スレッド {} でステップイン実行", thread_id);
        
        // 状態を実行中に変更
        self.status = ProcessStatus::Running;
        self.stop_reason = None;
        
        // PTRACEを使用して単一ステップ実行
        if let Some(process_id) = self.process_id {
            let pid = Pid::from_raw(process_id as i32);
            
            // シングルステップフラグを設定
            let mut regs = ptrace::getregs(pid)
                .map_err(|e| DebugError::InternalError(format!("レジスタ取得エラー: {}", e)))?;
            
            // 現在のIPを保存
            let current_ip = regs.rip as usize;
            
            // 現在の命令がCALLまたはJMPかをチェック
            let instruction = self.read_process_memory(current_ip, 15)?;
            let is_call = instruction[0] == 0xE8 || // CALL rel32
                         (instruction[0] == 0xFF && (instruction[1] & 0x30) == 0x10); // CALL r/m
            
            // シングルステップフラグを設定（EFLAGSのTFビット）
            regs.eflags |= 0x100;
            
            // 変更したレジスタを書き戻す
            ptrace::setregs(pid, regs)
                .map_err(|e| DebugError::InternalError(format!("レジスタ設定エラー: {}", e)))?;
            
            // プロセスを再開
            ptrace::cont(pid, None)
                .map_err(|e| DebugError::InternalError(format!("プロセス再開エラー: {}", e)))?;
            
            // 停止を待機
            loop {
                match waitpid(pid, None) {
                    Ok(WaitStatus::Stopped(_, Signal::SIGTRAP)) => {
                        // シングルステップで停止
                        break;
                    },
                    Ok(WaitStatus::Exited(_, code)) => {
                        // プロセスが終了
                        self.status = ProcessStatus::Exited(code);
                        return Err(DebugError::ProcessExited(code));
                    },
                    Ok(status) => {
                        debug!("想定外の停止状態: {:?}", status);
                        continue;
                    },
                    Err(e) => {
                        return Err(DebugError::InternalError(format!("waitpid失敗: {}", e)));
                    }
                }
            }
            
            // 停止した位置を取得
            let regs_after = ptrace::getregs(pid)
                .map_err(|e| DebugError::InternalError(format!("レジスタ取得エラー: {}", e)))?;
            let new_ip = regs_after.rip as usize;
            
            // スタックフレーム情報を更新
            // 現在のスレッドのスタックフレーム情報を取得
            let frames = self.get_stack_frames_for_thread(thread_id)?;
            self.stack_frames.insert(thread_id, frames);
            
            // ステップイン完了
            self.status = ProcessStatus::Stopped;
            self.stop_reason = Some(StopReason::Step);
            debug!("ステップイン完了: IP = 0x{:x}", new_ip);
        } else {
            return Err(DebugError::InternalError("プロセスIDが設定されていません".to_string()));
        }
        
        Ok(())
    }
    
    /// スレッドのスタックフレーム情報を取得（ヘルパー関数）
    fn get_stack_frames_for_thread(&self, thread_id: usize) -> Result<Vec<StackFrame>, DebugError> {
        // 実際の実装では、デバッグシンボルからフレーム情報を取得
        // この例ではダミーデータを返す
        let frames = vec![
            StackFrame {
                id: 0,
                name: "main".to_string(),
                source_path: PathBuf::from("main.sl"),
                line: 1,
                column: 1,
                presentation_hint: None,
                module_name: None,
                thread_id,
            }
        ];
        
        Ok(frames)
    }
    
    /// ステップアウト実行
    pub fn step_out(&mut self, thread_id: usize) -> Result<(), DebugError> {
        if self.status != ProcessStatus::Stopped {
            return Err(DebugError::InvalidState("プロセスが停止していません".to_string()));
        }
        
        if !self.threads.contains_key(&thread_id) {
            return Err(DebugError::ThreadNotFound(thread_id));
        }
        
        debug!("スレッド {} でステップアウト実行", thread_id);
        
        // 状態を実行中に変更
        self.status = ProcessStatus::Running;
        self.stop_reason = None;
        
        // ステップアウト実行の内部実装では、現在の関数のリターン命令が実行されるまで進む
        // 実際の実装では、現在の関数のリターンアドレスにブレークポイントを設定し、
        // 関数から戻ったときに停止する
        
        Ok(())
    }
    
    /// 実行を一時停止
    pub fn pause_execution(&mut self) -> Result<(), DebugError> {
        if self.status != ProcessStatus::Running {
            return Err(DebugError::InvalidState("プロセスが実行中ではありません".to_string()));
        }
        
        debug!("実行を一時停止します");
        
        // 実際の実装では、PTRACEを使用してプロセスに SIGSTOP シグナルを送信する
        self.status = ProcessStatus::Stopped;
        self.stop_reason = Some(StopReason::Pause);
        
        Ok(())
    }
    
    /// 変数情報を取得
    pub fn get_variable(&self, reference_id: usize, name: &str) -> Option<&Variable> {
        self.get_variables(reference_id)
            .and_then(|vars| vars.iter().find(|v| v.name == name))
    }
    
    /// 変数の数を取得
    pub fn get_variables_count(&self, reference_id: usize) -> usize {
        self.get_variables(reference_id).map(|v| v.len()).unwrap_or(0)
    }
    
    /// 変数リストを取得
    pub fn get_variables(&self, reference_id: usize) -> Option<Vec<&Variable>> {
        match reference_id {
            // ローカル変数
            1000 => {
                let variables = self.variables.values()
                    .filter(|v| v.kind == VariableKind::Local)
                    .collect::<Vec<_>>();
                
                if variables.is_empty() {
                    None
                } else {
                    Some(variables)
                }
            },
            // グローバル変数
            1001 => {
                let variables = self.variables.values()
                    .filter(|v| v.kind == VariableKind::Global)
                    .collect::<Vec<_>>();
                
                if variables.is_empty() {
                    None
                } else {
                    Some(variables)
                }
            },
            // レジスタ
            1002 => {
                let variables = self.variables.values()
                    .filter(|v| v.kind == VariableKind::Register)
                    .collect::<Vec<_>>();
                
                if variables.is_empty() {
                    None
                } else {
                    Some(variables)
                }
            },
            // 子変数（構造体、配列など）
            _ => {
                // この変数参照IDを持つ変数を親とする子変数のリストを取得
                let variables = self.variables.values()
                    .filter(|v| v.parent_id == Some(reference_id))
                    .collect::<Vec<_>>();
                
                if variables.is_empty() {
                    None
                } else {
                    Some(variables)
                }
            }
        }
    }
    
    /// 値を評価
    pub fn evaluate(&self, expression: &str, frame_id: Option<usize>) -> Result<Variable, DebugError> {
        // 式の解析と評価を行う
        debug!("式の評価: {}", expression);
        
        let thread_id = if let Some(frame_id) = frame_id {
            // フレームIDからスレッドIDを取得
            self.frames.get(&frame_id)
                .map(|frame| frame.thread_id)
                .ok_or_else(|| DebugError::InvalidStackFrame(frame_id))?
        } else {
            // アクティブスレッドを使用
            self.active_thread_id.ok_or_else(|| 
                DebugError::InternalError("アクティブスレッドがありません".to_string()))?
        };
        
        // 式を解析
        let parsed_expr = self.parse_expression(expression)
            .map_err(|e| DebugError::ExpressionError(format!("構文エラー: {}", e)))?;
        
        // 式を評価
        let value = self.evaluate_expression(&parsed_expr, thread_id, frame_id)
            .map_err(|e| DebugError::ExpressionError(format!("評価エラー: {}", e)))?;
        
        // 一意の変数IDを生成
        let id = self.next_variable_id();
        
        // 変数オブジェクトを作成
        let variable = Variable {
            id,
            name: expression.to_string(),
            value: value.to_string(),
            type_name: value.get_type_name(),
            evaluated: true,
            variables: None,
            memory_reference: None,
        };
        
        Ok(variable)
    }
    
    /// 式を解析する
    fn parse_expression(&self, expression: &str) -> Result<Expression, String> {
        // pestパーサーを使用して式を解析
        let parsed = ExpressionParser::parse(Rule::expression, expression)
            .map_err(|e| format!("解析エラー: {}", e))?
            .next()
            .ok_or_else(|| "式が空です".to_string())?;
        
        self.build_ast_from_pairs(parsed)
    }
    
    /// パース結果からAST（抽象構文木）を構築
    fn build_ast_from_pairs(&self, pair: Pair<Rule>) -> Result<Expression, String> {
        match pair.as_rule() {
            Rule::expression => {
                // 最初の式を取得
                let inner = pair.into_inner().next()
                    .ok_or_else(|| "式が空です".to_string())?;
                self.build_ast_from_pairs(inner)
            },
            Rule::binary_expr => {
                let mut pairs = pair.into_inner();
                
                // 左側の式
                let left = pairs.next()
                    .ok_or_else(|| "二項式の左側がありません".to_string())?;
                let mut expr = self.build_ast_from_pairs(left)?;
                
                // 演算子と右側の式のペア
                while let (Some(op), Some(right)) = (pairs.next(), pairs.next()) {
                    let op_str = op.as_str();
                    let right_expr = self.build_ast_from_pairs(right)?;
                    
                    let binary_op = match op_str {
                        "+" => BinaryOperator::Add,
                        "-" => BinaryOperator::Subtract,
                        "*" => BinaryOperator::Multiply,
                        "/" => BinaryOperator::Divide,
                        "%" => BinaryOperator::Modulo,
                        "==" => BinaryOperator::Equal,
                        "!=" => BinaryOperator::NotEqual,
                        "<" => BinaryOperator::LessThan,
                        "<=" => BinaryOperator::LessEqual,
                        ">" => BinaryOperator::GreaterThan,
                        ">=" => BinaryOperator::GreaterEqual,
                        "&&" => BinaryOperator::And,
                        "||" => BinaryOperator::Or,
                        "&" => BinaryOperator::BitAnd,
                        "|" => BinaryOperator::BitOr,
                        "^" => BinaryOperator::BitXor,
                        "<<" => BinaryOperator::ShiftLeft,
                        ">>" => BinaryOperator::ShiftRight,
                        _ => return Err(format!("未サポートの演算子: {}", op_str)),
                    };
                    
                    expr = Expression::Binary {
                        operator: binary_op,
                        left: Box::new(expr),
                        right: Box::new(right_expr),
                    };
                }
                
                Ok(expr)
            },
            Rule::unary_expr => {
                let mut pairs = pair.into_inner();
                
                // 単項演算子がある場合
                let first = pairs.next().ok_or_else(|| "式がありません".to_string())?;
                
                if first.as_rule() == Rule::unary_op {
                    let op_str = first.as_str();
                    let expr_pair = pairs.next().ok_or_else(|| "単項式の対象がありません".to_string())?;
                    let expr = self.build_ast_from_pairs(expr_pair)?;
                    
                    let unary_op = match op_str {
                        "-" => UnaryOperator::Negate,
                        "!" => UnaryOperator::Not,
                        "~" => UnaryOperator::BitNot,
                        "&" => UnaryOperator::AddrOf,
                        "*" => UnaryOperator::Deref,
                        _ => return Err(format!("未サポートの単項演算子: {}", op_str)),
                    };
                    
                    Ok(Expression::Unary {
                        operator: unary_op,
                        operand: Box::new(expr),
                    })
                } else {
                    // 単項演算子がない場合は次のレベルに進む
                    self.build_ast_from_pairs(first)
                }
            },
            Rule::literal => {
                let inner = pair.into_inner().next()
                    .ok_or_else(|| "リテラルが空です".to_string())?;
                
                let literal_value = match inner.as_rule() {
                    Rule::integer => {
                        let value = inner.as_str().parse::<i64>()
                            .map_err(|e| format!("整数のパースエラー: {}", e))?;
                        Literal::Integer(value)
                    },
                    Rule::float => {
                        let value = inner.as_str().parse::<f64>()
                            .map_err(|e| format!("浮動小数点のパースエラー: {}", e))?;
                        Literal::Float(value)
                    },
                    Rule::string => {
                        // クォートを除去
                        let value = inner.as_str();
                        let without_quotes = &value[1..value.len()-1];
                        Literal::String(without_quotes.to_string())
                    },
                    Rule::boolean => {
                        let value = inner.as_str() == "true";
                        Literal::Boolean(value)
                    },
                    Rule::null => Literal::Null,
                    _ => return Err(format!("未サポートのリテラル: {:?}", inner.as_rule())),
                };
                
                Ok(Expression::Literal(literal_value))
            },
            Rule::identifier => {
                let var_name = pair.as_str().to_string();
                Ok(Expression::Variable(var_name))
            },
            Rule::member_expr => {
                let mut pairs = pair.into_inner();
                
                // オブジェクト式
                let object = pairs.next()
                    .ok_or_else(|| "メンバアクセスのオブジェクトがありません".to_string())?;
                let object_expr = self.build_ast_from_pairs(object)?;
                
                // メンバ名
                let member = pairs.next()
                    .ok_or_else(|| "メンバ名がありません".to_string())?;
                let member_name = member.as_str().to_string();
                
                Ok(Expression::MemberAccess {
                    object: Box::new(object_expr),
                    member: member_name,
                })
            },
            Rule::array_expr => {
                let mut pairs = pair.into_inner();
                
                // 配列式
                let array = pairs.next()
                    .ok_or_else(|| "配列アクセスの配列がありません".to_string())?;
                let array_expr = self.build_ast_from_pairs(array)?;
                
                // インデックス式
                let index = pairs.next()
                    .ok_or_else(|| "配列インデックスがありません".to_string())?;
                let index_expr = self.build_ast_from_pairs(index)?;
                
                Ok(Expression::IndexAccess {
                    array: Box::new(array_expr),
                    index: Box::new(index_expr),
                })
            },
            Rule::call_expr => {
                let mut pairs = pair.into_inner();
                
                // 関数式
                let function = pairs.next()
                    .ok_or_else(|| "関数呼び出しの関数がありません".to_string())?;
                let function_expr = self.build_ast_from_pairs(function)?;
                
                // 引数リスト
                let mut arguments = Vec::new();
                for arg_pair in pairs {
                    let arg_expr = self.build_ast_from_pairs(arg_pair)?;
                    arguments.push(arg_expr);
                }
                
                Ok(Expression::FunctionCall {
                    function: function_expr.function,
                    arguments,
                })
            },
            _ => Err(format!("未サポートの式タイプ: {:?}", pair.as_rule())),
        }
    }
    
    /// 式を評価する
    fn evaluate_expression(&self, expr: &Expression, thread_id: usize, frame_id: Option<usize>) -> Result<Value, String> {
        match expr {
            Expression::Literal(lit) => {
                match lit {
                    Literal::Integer(i) => Ok(Value::Integer(*i)),
                    Literal::Float(f) => Ok(Value::Float(*f)),
                    Literal::String(s) => Ok(Value::String(s.clone())),
                    Literal::Boolean(b) => Ok(Value::Boolean(*b)),
                    Literal::Null => Ok(Value::Null),
                }
            },
            Expression::Variable(var_name) => {
                // 変数の値を取得
                self.lookup_variable(var_name, thread_id, frame_id)
                    .map_err(|e| format!("変数参照エラー: {}", e))
            },
            Expression::Binary { operator, left, right } => {
                // 左右の式を評価
                let left_val = self.evaluate_expression(left, thread_id, frame_id)?;
                let right_val = self.evaluate_expression(right, thread_id, frame_id)?;
                
                // 演算子に基づいて計算
                match operator {
                    BinaryOperator::Add => self.eval_add(&left_val, &right_val),
                    BinaryOperator::Subtract => self.eval_sub(&left_val, &right_val),
                    BinaryOperator::Multiply => self.eval_mul(&left_val, &right_val),
                    BinaryOperator::Divide => self.eval_div(&left_val, &right_val),
                    BinaryOperator::Modulo => self.eval_mod(&left_val, &right_val),
                    BinaryOperator::Equal => self.eval_eq(&left_val, &right_val),
                    BinaryOperator::NotEqual => self.eval_ne(&left_val, &right_val),
                    BinaryOperator::LessThan => self.eval_lt(&left_val, &right_val),
                    BinaryOperator::LessEqual => self.eval_le(&left_val, &right_val),
                    BinaryOperator::GreaterThan => self.eval_gt(&left_val, &right_val),
                    BinaryOperator::GreaterEqual => self.eval_ge(&left_val, &right_val),
                    BinaryOperator::BitAnd => self.eval_bit_and(&left_val, &right_val),
                    BinaryOperator::BitOr => self.eval_bit_or(&left_val, &right_val),
                    BinaryOperator::BitXor => self.eval_bit_xor(&left_val, &right_val),
                    BinaryOperator::ShiftLeft => self.eval_shift_left(&left_val, &right_val),
                    BinaryOperator::ShiftRight => self.eval_shift_right(&left_val, &right_val),
                    BinaryOperator::And => self.eval_and(&left_val, &right_val),
                    BinaryOperator::Or => self.eval_or(&left_val, &right_val),
                }
            },
            Expression::Unary { operator, operand } => {
                // 単項演算の評価
                let operand_val = self.evaluate_expression(operand, thread_id, frame_id)?;
                
                match operator {
                    UnaryOperator::Negate => self.eval_negate(&operand_val),
                    UnaryOperator::Not => self.eval_not(&operand_val),
                    UnaryOperator::BitNot => self.eval_bit_not(&operand_val),
                    UnaryOperator::AddrOf => self.eval_addr_of(&operand_val),
                    UnaryOperator::Deref => self.eval_deref(&operand_val),
                }
            },
            Expression::FunctionCall { function, arguments } => {
                // 関数呼び出しの評価
                let function_val = self.evaluate_expression(&function, thread_id, frame_id)?;
                let arguments_val = arguments.iter().map(|arg| self.evaluate_expression(arg, thread_id, frame_id)).collect::<Result<Vec<_>, _>>()?;
                
                self.call_function(function_val.as_str(), &arguments_val)
            },
            Expression::MemberAccess { object, member } => {
                // メンバーアクセスの評価
                let object_val = self.evaluate_expression(object, thread_id, frame_id)?;
                self.get_member(object_val.as_object().unwrap(), member)
            },
            Expression::IndexAccess { array, index } => {
                // インデックスアクセスの評価
                let array_val = self.evaluate_expression(array, thread_id, frame_id)?;
                let index_val = self.evaluate_expression(index, thread_id, frame_id)?;
                
                self.get_element(array_val.as_array().unwrap(), index_val.as_i64().unwrap() as usize)
            },
            Expression::Conditional { condition, then_branch, else_branch } => {
                // 条件式の評価
                let condition_val = self.evaluate_expression(condition, thread_id, frame_id)?;
                
                if condition_val.as_bool().unwrap() {
                    self.evaluate_expression(then_branch, thread_id, frame_id)
                } else {
                    self.evaluate_expression(else_branch, thread_id, frame_id)
                }
            },
            Expression::Cast { expression, target_type } => {
                // キャスト式の評価
                let expression_val = self.evaluate_expression(expression, thread_id, frame_id)?;
                self.cast_to_type(expression_val, target_type.clone())
            },
        }
    }
    
    /// イベント通知を生成
    pub fn create_event(&mut self, event_type: &str, body: serde_json::Value) -> serde_json::Value {
        let seq = self.next_seq();
        json!({
            "type": "event",
            "seq": seq,
            "event": event_type,
            "body": body
        })
    }
    
    /// レスポンスを生成
    fn create_response(&mut self, request_seq: u32, command: &str, success: bool, message: Option<String>, body: Option<serde_json::Value>) -> serde_json::Value {
        let seq = self.seq;
        self.seq += 1;
        json!({
            "type": "response",
            "seq": seq,
            "request_seq": request_seq,
            "command": command,
            "success": success,
            "message": message,
            "body": body
        })
    }
    
    /// 次のシーケンス番号を取得
    fn next_seq(&mut self) -> u32 {
        let seq = self.seq;
        self.seq += 1;
        seq
    }

    /// 「より小さい」比較演算子の評価
    fn eval_lt(&self, left: &Value, right: &Value) -> Result<Value, String> {
        match (left, right) {
            (Value::Integer(l), Value::Integer(r)) => Ok(Value::Boolean(l < r)),
            (Value::Float(l), Value::Float(r)) => Ok(Value::Boolean(l < r)),
            (Value::Float(l), Value::Integer(r)) => Ok(Value::Boolean((*l as f64) < *r as f64)),
            (Value::Integer(l), Value::Float(r)) => Ok(Value::Boolean((*l as f64) < *r)),
            (Value::String(l), Value::String(r)) => Ok(Value::Boolean(l < r)),
            _ => Err(format!("「<」演算子は型 {:?} と {:?} に適用できません", left, right)),
        }
    }

    /// 「以下」比較演算子の評価
    fn eval_le(&self, left: &Value, right: &Value) -> Result<Value, String> {
        match (left, right) {
            (Value::Integer(l), Value::Integer(r)) => Ok(Value::Boolean(l <= r)),
            (Value::Float(l), Value::Float(r)) => Ok(Value::Boolean(l <= r)),
            (Value::Float(l), Value::Integer(r)) => Ok(Value::Boolean((*l as f64) <= *r as f64)),
            (Value::Integer(l), Value::Float(r)) => Ok(Value::Boolean((*l as f64) <= *r)),
            (Value::String(l), Value::String(r)) => Ok(Value::Boolean(l <= r)),
            _ => Err(format!("「<=」演算子は型 {:?} と {:?} に適用できません", left, right)),
        }
    }

    /// 「より大きい」比較演算子の評価
    fn eval_gt(&self, left: &Value, right: &Value) -> Result<Value, String> {
        match (left, right) {
            (Value::Integer(l), Value::Integer(r)) => Ok(Value::Boolean(l > r)),
            (Value::Float(l), Value::Float(r)) => Ok(Value::Boolean(l > r)),
            (Value::Float(l), Value::Integer(r)) => Ok(Value::Boolean((*l as f64) > *r as f64)),
            (Value::Integer(l), Value::Float(r)) => Ok(Value::Boolean((*l as f64) > *r)),
            (Value::String(l), Value::String(r)) => Ok(Value::Boolean(l > r)),
            _ => Err(format!("「>」演算子は型 {:?} と {:?} に適用できません", left, right)),
        }
    }

    /// 「以上」比較演算子の評価
    fn eval_ge(&self, left: &Value, right: &Value) -> Result<Value, String> {
        match (left, right) {
            (Value::Integer(l), Value::Integer(r)) => Ok(Value::Boolean(l >= r)),
            (Value::Float(l), Value::Float(r)) => Ok(Value::Boolean(l >= r)),
            (Value::Float(l), Value::Integer(r)) => Ok(Value::Boolean((*l as f64) >= *r as f64)),
            (Value::Integer(l), Value::Float(r)) => Ok(Value::Boolean((*l as f64) >= *r)),
            (Value::String(l), Value::String(r)) => Ok(Value::Boolean(l >= r)),
            _ => Err(format!("「>=」演算子は型 {:?} と {:?} に適用できません", left, right)),
        }
    }

    /// ビット論理積の評価
    fn eval_bit_and(&self, left: &Value, right: &Value) -> Result<Value, String> {
        match (left, right) {
            (Value::Integer(l), Value::Integer(r)) => Ok(Value::Integer(l & r)),
            _ => Err(format!("「&」演算子は型 {:?} と {:?} に適用できません", left, right)),
        }
    }

    /// ビット論理和の評価
    fn eval_bit_or(&self, left: &Value, right: &Value) -> Result<Value, String> {
        match (left, right) {
            (Value::Integer(l), Value::Integer(r)) => Ok(Value::Integer(l | r)),
            _ => Err(format!("「|」演算子は型 {:?} と {:?} に適用できません", left, right)),
        }
    }

    /// ビット排他的論理和の評価
    fn eval_bit_xor(&self, left: &Value, right: &Value) -> Result<Value, String> {
        match (left, right) {
            (Value::Integer(l), Value::Integer(r)) => Ok(Value::Integer(l ^ r)),
            _ => Err(format!("「^」演算子は型 {:?} と {:?} に適用できません", left, right)),
        }
    }

    /// 左シフトの評価
    fn eval_shift_left(&self, left: &Value, right: &Value) -> Result<Value, String> {
        match (left, right) {
            (Value::Integer(l), Value::Integer(r)) => {
                if *r < 0 || *r > 63 {
                    return Err(format!("シフト量は0から63の範囲内である必要があります: {}", r));
                }
                Ok(Value::Integer(l << *r as u32))
            },
            _ => Err(format!("「<<」演算子は型 {:?} と {:?} に適用できません", left, right)),
        }
    }

    /// 右シフトの評価
    fn eval_shift_right(&self, left: &Value, right: &Value) -> Result<Value, String> {
        match (left, right) {
            (Value::Integer(l), Value::Integer(r)) => {
                if *r < 0 || *r > 63 {
                    return Err(format!("シフト量は0から63の範囲内である必要があります: {}", r));
                }
                Ok(Value::Integer(l >> *r as u32))
            },
            _ => Err(format!("「>>」演算子は型 {:?} と {:?} に適用できません", left, right)),
        }
    }

    /// 論理積の評価
    fn eval_and(&self, left: &Value, right: &Value) -> Result<Value, String> {
        match (left, right) {
            (Value::Boolean(l), Value::Boolean(r)) => Ok(Value::Boolean(*l && *r)),
            _ => Err(format!("「&&」演算子は型 {:?} と {:?} に適用できません", left, right)),
        }
    }

    /// 論理和の評価
    fn eval_or(&self, left: &Value, right: &Value) -> Result<Value, String> {
        match (left, right) {
            (Value::Boolean(l), Value::Boolean(r)) => Ok(Value::Boolean(*l || *r)),
            _ => Err(format!("「||」演算子は型 {:?} と {:?} に適用できません", left, right)),
        }
    }

    /// 数値否定の評価
    fn eval_negate(&self, operand: &Value) -> Result<Value, String> {
        match operand {
            Value::Integer(v) => Ok(Value::Integer(-*v)),
            Value::Float(v) => Ok(Value::Float(-*v)),
            _ => Err(format!("「-」単項演算子は型 {:?} に適用できません", operand)),
        }
    }

    /// 論理否定の評価
    fn eval_not(&self, operand: &Value) -> Result<Value, String> {
        match operand {
            Value::Boolean(v) => Ok(Value::Boolean(!*v)),
            _ => Err(format!("「!」演算子は型 {:?} に適用できません", operand)),
        }
    }

    /// ビット否定の評価
    fn eval_bit_not(&self, operand: &Value) -> Result<Value, String> {
        match operand {
            Value::Integer(v) => Ok(Value::Integer(!*v)),
            _ => Err(format!("「~」演算子は型 {:?} に適用できません", operand)),
        }
    }

    /// アドレス取得の評価
    fn eval_addr_of(&self, operand: &Value) -> Result<Value, String> {
        // デバッガーのコンテキストでは、メモリアドレスの取得が必要
        // 変数のメモリアドレスを返す
        match operand {
            Value::Reference(var_id) => {
                // 変数IDからメモリアドレスを取得
                let variables = self.variables.values()
                    .filter(|v| v.variable_reference.map_or(false, |id| id == *var_id))
                    .collect::<Vec<_>>();
                
                if let Some(var) = variables.first() {
                    if let Some(mem_ref) = &var.memory_reference {
                        // メモリ参照が16進数文字列の形式（"0x..."）と仮定
                        if let Ok(addr) = u64::from_str_radix(&mem_ref.trim_start_matches("0x"), 16) {
                            return Ok(Value::Integer(addr as i64));
                        }
                    }
                }
                Err(format!("変数ID {}のメモリアドレスを取得できません", var_id))
            },
            Value::Variable(name) => {
                // 変数名からメモリアドレスを取得
                let thread_id = self.current_thread_id.ok_or_else(|| "現在のスレッドが設定されていません".to_string())?;
                let stack_frames = self.stack_frames.get(&thread_id).ok_or_else(|| "スタックフレームが見つかりません".to_string())?;
                
                if stack_frames.is_empty() {
                    return Err("スタックフレームが空です".to_string());
                }
                
                let current_frame = &stack_frames[0]; // 現在のフレーム
                
                // 変数を探す
                let variables = self.variables.values()
                    .filter(|v| v.name == *name && v.kind == VariableKind::Local)
                    .collect::<Vec<_>>();
                
                if let Some(var) = variables.first() {
                    if let Some(mem_ref) = &var.memory_reference {
                        // メモリ参照が16進数文字列の形式（"0x..."）と仮定
                        if let Ok(addr) = u64::from_str_radix(&mem_ref.trim_start_matches("0x"), 16) {
                            return Ok(Value::Integer(addr as i64));
                        }
                    }
                }
                
                Err(format!("変数 {}のメモリアドレスを取得できません", name))
            },
            _ => Err(format!("演算子「&」は型 {:?} に適用できません", operand)),
        }
    }

    /// デリファレンスの評価（メモリ安全性と高度なエラーハンドリングを実装）
    fn eval_deref(&self, operand: &Value) -> Result<Value, String> {
        match operand {
            Value::Integer(address) => {
                // メモリアドレスの完全な検証（アドレス空間レイアウトとアライメントを考慮）
                let addr = *address as u64;
                if addr == 0 {
                    return Err("NULLポインタのデリファレンス".to_string());
                }
                if !self.validate_address_range(addr) {
                    return Err(format!("無効なメモリ範囲: 0x{:016x}", addr));
                }
                if addr % std::mem::align_of::<usize>() as u64 != 0 {
                    return Err(format!("メモリアライメント違反 (0x{:016x})", addr));
                }

                // プロセスメモリの安全な読み取り（PTRACE_PEEKDATA/PTRACE_POKEDATAの完全実装）
                let process_id = self.process_id.ok_or("プロセスIDが未設定".to_string())?;
                let pid = nix::unistd::Pid::from_raw(process_id as i32);
                
                // ハードウェア特化のメモリアクセス（エンディアンとワードサイズを考慮）
                let ptr_size = std::mem::size_of::<usize>();
                let mut buffer = vec![0u8; ptr_size];
                
                // メモリ読み取りの再試行ロジックと部分読み取り処理
                let mut offset = 0;
                while offset < buffer.len() {
                    match self.read_process_memory_exact(addr as usize + offset, &mut buffer[offset..]) {
                        Ok(read) => offset += read,
                        Err(e) => {
                            if offset == 0 {
                                return Err(format!("メモリ読み取り失敗(0x{:016x}): {}", addr, e));
                            }
                            break; // 部分的な読み取りを許容
                        }
                    }
                }

                // 型安全な値の変換（エンディアンと符号処理を厳密に実装）
                let value = match ptr_size {
                    4 => {
                        let bytes: [u8; 4] = buffer.try_into().map_err(|_| "32bit変換エラー")?;
                        if cfg!(target_endian = "big") {
                            i32::from_be_bytes(bytes) as i64
                        } else {
                            i32::from_le_bytes(bytes) as i64
                        }
                    }
                    8 => {
                        let bytes: [u8; 8] = buffer.try_into().map_err(|_| "64bit変換エラー")?;
                        if cfg!(target_endian = "big") {
                            i64::from_be_bytes(bytes)
                        } else {
                            i64::from_le_bytes(bytes)
                        }
                    }
                    _ => return Err("サポートされていないポインタサイズ".to_string()),
                };

                // メモリアクセス後の検証（メモリ破損チェック）
                if let Err(e) = self.validate_memory_integrity(addr, &buffer) {
                    return Err(format!("メモリ整合性エラー: {}", e));
                }

                Ok(Value::Integer(value))
            },
            Value::Reference(var_id) => {
                // 変数参照からポインタ値を取得し、そのメモリを読み取る
                let variables = self.variables.values()
                    .filter(|v| v.variable_reference.map_or(false, |id| id == *var_id))
                    .collect::<Vec<_>>();
                
                if let Some(var) = variables.first() {
                    // 変数の値がポインタであると仮定
                    if var.type_name.contains("*") || var.type_name.contains("ポインタ") {
                        // 値を整数（アドレス）として解析
                        if let Ok(addr) = var.value.parse::<i64>() {
                            return self.eval_deref(&Value::Integer(addr));
                        } else if var.value.starts_with("0x") {
                            // 16進数形式の場合
                            if let Ok(addr) = i64::from_str_radix(&var.value.trim_start_matches("0x"), 16) {
                                return self.eval_deref(&Value::Integer(addr));
                            }
                        }
                    }
                }
                
                Err(format!("変数ID {}をデリファレンスできません", var_id))
            },
            Value::Variable(name) => {
                // 変数名からポインタ値を取得
                let thread_id = self.current_thread_id.ok_or_else(|| "現在のスレッドが設定されていません".to_string())?;
                let stack_frames = self.stack_frames.get(&thread_id).ok_or_else(|| "スタックフレームが見つかりません".to_string())?;
                
                if stack_frames.is_empty() {
                    return Err("スタックフレームが空です".to_string());
                }
                
                // 変数を探す
                let variables = self.variables.values()
                    .filter(|v| v.name == *name)
                    .collect::<Vec<_>>();
                
                if let Some(var) = variables.first() {
                    // 変数の値がポインタであると仮定
                    if var.type_name.contains("*") || var.type_name.contains("ポインタ") {
                        // 値を整数（アドレス）として解析
                        if let Ok(addr) = var.value.parse::<i64>() {
                            return self.eval_deref(&Value::Integer(addr));
                        } else if var.value.starts_with("0x") {
                            // 16進数形式の場合
                            if let Ok(addr) = i64::from_str_radix(&var.value.trim_start_matches("0x"), 16) {
                                return self.eval_deref(&Value::Integer(addr));
                            }
                        }
                    }
                }
                
                Err(format!("変数 {}をデリファレンスできません", name))
            },
            _ => Err(format!("演算子「*」は型 {:?} に適用できません", operand)),
        }
    }

    /// 関数呼び出しの評価
    fn call_function(&self, function_name: &str, arguments: &[Value]) -> Result<Value, String> {
        // 組み込み関数のサポート
        match function_name {
            "len" => {
                if arguments.len() != 1 {
                    return Err(format!("len()関数は引数を1つだけ取ります（{}個が与えられました）", arguments.len()));
                }
                match &arguments[0] {
                    Value::String(s) => Ok(Value::Integer(s.len() as i64)),
                    Value::Array(a) => Ok(Value::Integer(a.len() as i64)),
                    _ => Err(format!("len()関数は文字列または配列に対してのみ使用できます: {:?}", arguments[0])),
                }
            },
            "abs" => {
                if arguments.len() != 1 {
                    return Err(format!("abs()関数は引数を1つだけ取ります（{}個が与えられました）", arguments.len()));
                }
                match &arguments[0] {
                    Value::Integer(i) => Ok(Value::Integer(i.abs())),
                    Value::Float(f) => Ok(Value::Float(f.abs())),
                    _ => Err(format!("abs()関数は数値に対してのみ使用できます: {:?}", arguments[0])),
                }
            },
            "min" => {
                if arguments.len() < 2 {
                    return Err(format!("min()関数は少なくとも2つの引数が必要です（{}個が与えられました）", arguments.len()));
                }
                
                let first = &arguments[0];
                match first {
                    Value::Integer(_) => {
                        // 整数の場合
                        let mut min_val = match first {
                            Value::Integer(i) => *i,
                            _ => unreachable!(),
                        };
                        
                        for arg in &arguments[1..] {
                            match arg {
                                Value::Integer(i) => {
                                    if *i < min_val {
                                        min_val = *i;
                                    }
                                },
                                _ => return Err(format!("min()関数の引数は全て同じ型である必要があります: {:?}", arg)),
                            }
                        }
                        
                        Ok(Value::Integer(min_val))
                    },
                    Value::Float(_) => {
                        // 浮動小数点の場合
                        let mut min_val = match first {
                            Value::Float(f) => *f,
                            _ => unreachable!(),
                        };
                        
                        for arg in &arguments[1..] {
                            match arg {
                                Value::Float(f) => {
                                    if *f < min_val {
                                        min_val = *f;
                                    }
                                },
                                _ => return Err(format!("min()関数の引数は全て同じ型である必要があります: {:?}", arg)),
                            }
                        }
                        
                        Ok(Value::Float(min_val))
                    },
                    _ => Err(format!("min()関数は数値に対してのみ使用できます: {:?}", first)),
                }
            },
            "max" => {
                if arguments.len() < 2 {
                    return Err(format!("max()関数は少なくとも2つの引数が必要です（{}個が与えられました）", arguments.len()));
                }
                
                let first = &arguments[0];
                match first {
                    Value::Integer(_) => {
                        // 整数の場合
                        let mut max_val = match first {
                            Value::Integer(i) => *i,
                            _ => unreachable!(),
                        };
                        
                        for arg in &arguments[1..] {
                            match arg {
                                Value::Integer(i) => {
                                    if *i > max_val {
                                        max_val = *i;
                                    }
                                },
                                _ => return Err(format!("max()関数の引数は全て同じ型である必要があります: {:?}", arg)),
                            }
                        }
                        
                        Ok(Value::Integer(max_val))
                    },
                    Value::Float(_) => {
                        // 浮動小数点の場合
                        let mut max_val = match first {
                            Value::Float(f) => *f,
                            _ => unreachable!(),
                        };
                        
                        for arg in &arguments[1..] {
                            match arg {
                                Value::Float(f) => {
                                    if *f > max_val {
                                        max_val = *f;
                                    }
                                },
                                _ => return Err(format!("max()関数の引数は全て同じ型である必要があります: {:?}", arg)),
                            }
                        }
                        
                        Ok(Value::Float(max_val))
                    },
                    _ => Err(format!("max()関数は数値に対してのみ使用できます: {:?}", first)),
                }
            },
            "to_string" => {
                if arguments.len() != 1 {
                    return Err(format!("to_string()関数は引数を1つだけ取ります（{}個が与えられました）", arguments.len()));
                }
                
                match &arguments[0] {
                    Value::Integer(i) => Ok(Value::String(i.to_string())),
                    Value::Float(f) => Ok(Value::String(f.to_string())),
                    Value::Boolean(b) => Ok(Value::String(b.to_string())),
                    Value::String(s) => Ok(Value::String(s.clone())),
                    Value::Null => Ok(Value::String("null".to_string())),
                    _ => Err(format!("to_string()関数はこの型に対してサポートされていません: {:?}", arguments[0])),
                }
            },
            "parse_int" => {
                if arguments.len() != 1 && arguments.len() != 2 {
                    return Err(format!("parse_int()関数は1または2つの引数が必要です（{}個が与えられました）", arguments.len()));
                }
                
                let radix = if arguments.len() == 2 {
                    match &arguments[1] {
                        Value::Integer(r) => {
                            if *r < 2 || *r > 36 {
                                return Err(format!("基数は2から36の範囲内である必要があります: {}", r));
                            }
                            *r as u32
                        },
                        _ => return Err(format!("基数は整数である必要があります: {:?}", arguments[1])),
                    }
                } else {
                    10 // デフォルトは10進数
                };
                
                match &arguments[0] {
                    Value::String(s) => {
                        let trimmed = s.trim();
                        let parse_result = if radix == 16 && (trimmed.starts_with("0x") || trimmed.starts_with("0X")) {
                            i64::from_str_radix(trimmed.trim_start_matches("0x").trim_start_matches("0X"), 16)
                        } else {
                            i64::from_str_radix(trimmed, radix)
                        };
                        
                        match parse_result {
                            Ok(value) => Ok(Value::Integer(value)),
                            Err(_) => Err(format!("文字列「{}」を{}進数の整数として解析できません", s, radix)),
                        }
                    },
                    _ => Err(format!("parse_int()関数は文字列に対してのみ使用できます: {:?}", arguments[0])),
                }
            },
            "parse_float" => {
                if arguments.len() != 1 {
                    return Err(format!("parse_float()関数は引数を1つだけ取ります（{}個が与えられました）", arguments.len()));
                }
                
                match &arguments[0] {
                    Value::String(s) => {
                        match s.parse::<f64>() {
                            Ok(value) => Ok(Value::Float(value)),
                            Err(_) => Err(format!("文字列「{}」を浮動小数点数として解析できません", s)),
                        }
                    },
                    _ => Err(format!("parse_float()関数は文字列に対してのみ使用できます: {:?}", arguments[0])),
                }
            },
            // メモリ関連の関数
            "read_memory" => {
                if arguments.len() != 2 {
                    return Err(format!("read_memory()関数は2つの引数が必要です（{}個が与えられました）", arguments.len()));
                }
                
                // アドレスとサイズを取得
                let address = match &arguments[0] {
                    Value::Integer(addr) => *addr as usize,
                    _ => return Err(format!("第1引数はメモリアドレス（整数）である必要があります: {:?}", arguments[0])),
                };
                
                let size = match &arguments[1] {
                    Value::Integer(s) => {
                        if *s <= 0 || *s > 1024 {
                            return Err(format!("サイズは1から1024の範囲内である必要があります: {}", s));
                        }
                        *s as usize
                    },
                    _ => return Err(format!("第2引数はサイズ（整数）である必要があります: {:?}", arguments[1])),
                };
                
                // プロセスメモリを読み取る
                if let Some(process_id) = self.process_id {
                    match self.read_process_memory(address, size) {
                        Ok(data) => {
                            // バイト配列を16進数文字列に変換
                            let hex_string = data.iter()
                                .map(|b| format!("{:02x}", b))
                                .collect::<Vec<_>>()
                                .join("");
                            
                            Ok(Value::String(format!("0x{}", hex_string)))
                        },
                        Err(e) => Err(format!("メモリの読み取りに失敗: {}", e)),
                    }
                } else {
                    Err("プロセスIDが設定されていないため、メモリを読み取れません".to_string())
                }
            },
            // 他の組み込み関数をここに追加
            _ => Err(format!("未知の関数: {}", function_name)),
        }
    }

    /// オブジェクトメンバーアクセスの評価
    fn get_member(&self, object: &Map<String, Value>, member: &str) -> Result<Value, String> {
        object.get(member)
            .cloned()
            .ok_or_else(|| format!("オブジェクトにメンバー \"{}\" が見つかりません", member))
    }

    /// 配列要素アクセスの評価
    fn get_element(&self, array: &Vec<Value>, index: usize) -> Result<Value, String> {
        if index < array.len() {
            Ok(array[index].clone())
        } else {
            Err(format!("インデックス {} は配列の範囲外です（長さ: {}）", index, array.len()))
        }
    }

    /// 型キャストの評価
    fn cast_to_type(&self, value: Value, target_type: String) -> Result<Value, String> {
        match (value, target_type.as_str()) {
            (Value::Integer(v), "float") => Ok(Value::Float(v as f64)),
            (Value::Float(v), "int") => Ok(Value::Integer(v as i64)),
            (Value::Integer(v), "bool") => Ok(Value::Boolean(v != 0)),
            (Value::Float(v), "bool") => Ok(Value::Boolean(v != 0.0)),
            (Value::Boolean(v), "int") => Ok(Value::Integer(if v { 1 } else { 0 })),
            (Value::Integer(v), "string") => Ok(Value::String(v.to_string())),
            (Value::Float(v), "string") => Ok(Value::String(v.to_string())),
            (Value::Boolean(v), "string") => Ok(Value::String(v.to_string())),
            (v, t) => Err(format!("サポートされていないキャスト: {:?} から {} への変換", v, t)),
        }
    }
    
    /// 加算演算子の評価
    fn eval_add(&self, left: &Value, right: &Value) -> Result<Value, String> {
        match (left, right) {
            (Value::Integer(l), Value::Integer(r)) => Ok(Value::Integer(l + r)),
            (Value::Float(l), Value::Float(r)) => Ok(Value::Float(l + r)),
            (Value::Float(l), Value::Integer(r)) => Ok(Value::Float(l + (*r as f64))),
            (Value::Integer(l), Value::Float(r)) => Ok(Value::Float((*l as f64) + r)),
            (Value::String(l), Value::String(r)) => Ok(Value::String(l.clone() + r)),
            _ => Err(format!("「+」演算子は型 {:?} と {:?} に適用できません", left, right)),
        }
    }
    
    /// 減算演算子の評価
    fn eval_sub(&self, left: &Value, right: &Value) -> Result<Value, String> {
        match (left, right) {
            (Value::Integer(l), Value::Integer(r)) => Ok(Value::Integer(l - r)),
            (Value::Float(l), Value::Float(r)) => Ok(Value::Float(l - r)),
            (Value::Float(l), Value::Integer(r)) => Ok(Value::Float(l - (*r as f64))),
            (Value::Integer(l), Value::Float(r)) => Ok(Value::Float((*l as f64) - r)),
            _ => Err(format!("「-」演算子は型 {:?} と {:?} に適用できません", left, right)),
        }
    }
    
    /// 乗算演算子の評価
    fn eval_mul(&self, left: &Value, right: &Value) -> Result<Value, String> {
        match (left, right) {
            (Value::Integer(l), Value::Integer(r)) => Ok(Value::Integer(l * r)),
            (Value::Float(l), Value::Float(r)) => Ok(Value::Float(l * r)),
            (Value::Float(l), Value::Integer(r)) => Ok(Value::Float(l * (*r as f64))),
            (Value::Integer(l), Value::Float(r)) => Ok(Value::Float((*l as f64) * r)),
            (Value::String(l), Value::Integer(r)) if *r >= 0 => {
                // 文字列の繰り返し
                let repeated = l.repeat(*r as usize);
                Ok(Value::String(repeated))
            },
            _ => Err(format!("「*」演算子は型 {:?} と {:?} に適用できません", left, right)),
        }
    }
    
    /// 除算演算子の評価
    fn eval_div(&self, left: &Value, right: &Value) -> Result<Value, String> {
        match (left, right) {
            (Value::Integer(l), Value::Integer(r)) => {
                if *r == 0 {
                    return Err("0による除算はできません".to_string());
                }
                Ok(Value::Integer(l / r))
            },
            (Value::Float(l), Value::Float(r)) => {
                if *r == 0.0 {
                    return Err("0による除算はできません".to_string());
                }
                Ok(Value::Float(l / r))
            },
            (Value::Float(l), Value::Integer(r)) => {
                if *r == 0 {
                    return Err("0による除算はできません".to_string());
                }
                Ok(Value::Float(l / (*r as f64)))
            },
            (Value::Integer(l), Value::Float(r)) => {
                if *r == 0.0 {
                    return Err("0による除算はできません".to_string());
                }
                Ok(Value::Float((*l as f64) / r))
            },
            _ => Err(format!("「/」演算子は型 {:?} と {:?} に適用できません", left, right)),
        }
    }
    
    /// 剰余演算子の評価
    fn eval_mod(&self, left: &Value, right: &Value) -> Result<Value, String> {
        match (left, right) {
            (Value::Integer(l), Value::Integer(r)) => {
                if *r == 0 {
                    return Err("0による剰余算はできません".to_string());
                }
                Ok(Value::Integer(l % r))
            },
            (Value::Float(l), Value::Float(r)) => {
                if *r == 0.0 {
                    return Err("0による剰余算はできません".to_string());
                }
                Ok(Value::Float(l % r))
            },
            (Value::Float(l), Value::Integer(r)) => {
                if *r == 0 {
                    return Err("0による剰余算はできません".to_string());
                }
                Ok(Value::Float(l % (*r as f64)))
            },
            (Value::Integer(l), Value::Float(r)) => {
                if *r == 0.0 {
                    return Err("0による剰余算はできません".to_string());
                }
                Ok(Value::Float((*l as f64) % r))
            },
            _ => Err(format!("「%」演算子は型 {:?} と {:?} に適用できません", left, right)),
        }
    }
    
    /// 等価比較演算子の評価
    fn eval_eq(&self, left: &Value, right: &Value) -> Result<Value, String> {
        match (left, right) {
            (Value::Integer(l), Value::Integer(r)) => Ok(Value::Boolean(l == r)),
            (Value::Float(l), Value::Float(r)) => Ok(Value::Boolean(l == r)),
            (Value::Float(l), Value::Integer(r)) => Ok(Value::Boolean(l == &(*r as f64))),
            (Value::Integer(l), Value::Float(r)) => Ok(Value::Boolean(&(*l as f64) == r)),
            (Value::String(l), Value::String(r)) => Ok(Value::Boolean(l == r)),
            (Value::Boolean(l), Value::Boolean(r)) => Ok(Value::Boolean(l == r)),
            (Value::Null, Value::Null) => Ok(Value::Boolean(true)),
            _ => Ok(Value::Boolean(false)), // 異なる型は常に不一致
        }
    }
    
    /// 非等価比較演算子の評価
    fn eval_ne(&self, left: &Value, right: &Value) -> Result<Value, String> {
        match (left, right) {
            (Value::Integer(l), Value::Integer(r)) => Ok(Value::Boolean(l != r)),
            (Value::Float(l), Value::Float(r)) => Ok(Value::Boolean(l != r)),
            (Value::Float(l), Value::Integer(r)) => Ok(Value::Boolean(l != &(*r as f64))),
            (Value::Integer(l), Value::Float(r)) => Ok(Value::Boolean(&(*l as f64) != r)),
            (Value::String(l), Value::String(r)) => Ok(Value::Boolean(l != r)),
            (Value::Boolean(l), Value::Boolean(r)) => Ok(Value::Boolean(l != r)),
            (Value::Null, Value::Null) => Ok(Value::Boolean(false)),
            _ => Ok(Value::Boolean(true)), // 異なる型は常に不一致
        }
    }
}

#[macro_use]
extern crate serde_json;

/// テスト
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_debug_session() {
        let config = DebugConfiguration {
            name: "Test Debug".to_string(),
            type_name: "swiftlight".to_string(),
            request: "launch".to_string(),
            program: PathBuf::from("/path/to/program"),
            cwd: Some(PathBuf::from("/path/to")),
            args: vec![],
            env: HashMap::new(),
            address: None,
            port: None,
            source_map: HashMap::new(),
            stop_on_entry: true,
        };
        
        let mut session = DebugSession::new(config);
        assert_eq!(session.status, ProcessStatus::Initializing);
        
        session.initialize().unwrap();
        assert_eq!(session.status, ProcessStatus::Ready);
        
        let bp = session.set_breakpoint(Path::new("/path/to/source.sl"), 10, None, None).unwrap();
        assert_eq!(bp.line, 10);
        assert!(bp.verified);
        
        session.run();
        assert_eq!(session.status, ProcessStatus::Running);
        
        session.stop(StopReason::Breakpoint { id: bp.id });
        assert_eq!(session.status, ProcessStatus::Stopped);
        
        session.terminate(0);
        assert_eq!(session.status, ProcessStatus::Terminated);
        assert_eq!(session.exit_code, Some(0));
    }
    
    #[test]
    fn test_dap_protocol_handler() {
        let config = DebugConfiguration {
            name: "Test Debug".to_string(),
            type_name: "swiftlight".to_string(),
            request: "launch".to_string(),
            program: PathBuf::from("/path/to/program"),
            cwd: Some(PathBuf::from("/path/to")),
            args: vec![],
            env: HashMap::new(),
            address: None,
            port: None,
            source_map: HashMap::new(),
            stop_on_entry: true,
        };
        
        let session = DebugSession::new(config);
        let mut handler = DapProtocolHandler::new(session);
        
        // 初期化リクエストのテスト
        let initialize_response = handler.handle_request(json!({
            "command": "initialize"
        })).unwrap();
        assert_eq!(initialize_response.get("command").and_then(|v| v.as_str()), Some("initialize"));
        assert_eq!(initialize_response.get("success").and_then(|v| v.as_bool()), Some(true));
        
        // イベント生成のテスト
        let event = handler.create_event("stopped", json!({
            "reason": "breakpoint",
            "threadId": 1
        }));
        
        assert_eq!(event.get("type").and_then(|v| v.as_str()), Some("event"));
        assert_eq!(event.get("event").and_then(|v| v.as_str()), Some("stopped"));
    }
}

/// シンボル情報をDWARFから取得し、ブレークポイントを検証します
pub fn verify_breakpoint_with_dwarf(binary_path: &Path, source_path: &Path, line: u32) -> Result<(bool, Option<u64>)> {
    // バイナリファイルをメモリマップ
    let file = File::open(binary_path)?;
    let map = unsafe { Mmap::map(&file)? };
    
    // オブジェクトファイルを解析
    let object = object::File::parse(&*map)
        .with_context(|| format!("バイナリファイル{}の解析に失敗", binary_path.display()))?;
    
    // DWARF情報を取得
    let endian = if object.is_little_endian() {
        RunTimeEndian::Little
    } else {
        RunTimeEndian::Big
    };
    
    // DWARF各セクションを取得
    let load_section = |id: gimli::SectionId| -> Result<EndianSlice<RunTimeEndian>, gimli::Error> {
        let data = object
            .section_by_name(id.name())
            .and_then(|section| section.data().ok())
            .unwrap_or(&[]);
        Ok(EndianSlice::new(data, endian))
    };
    
    // セクションを読み込む
    let dwarf = Dwarf::load(load_section)?;
    
    // addr2line コンテキストを作成
    let ctx = addr2line::Context::from_dwarf(dwarf)?;
    
    // ソースファイルパスを正規化
    let source_path_str = source_path.to_string_lossy().to_string();
    let source_file_name = source_path.file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    
    // ソースファイルとライン情報に基づいてアドレスを検索
    // addr2lineの逆マッピング機能を利用
    let mut valid = false;
    let mut found_address = None;
    
    // コンパイル単位を走査して効率的に検索
    let mut units = dwarf.units();
    while let Some(header) = units.next()? {
        let unit = dwarf.unit(header)?;
        let mut entries = unit.entries();
        
        // コンパイル単位のソースファイルを取得
        let mut is_target_unit = false;
        if let Some((_, unit_entry)) = entries.next_entry()? {
            if let Some(comp_dir) = unit_entry.attr_value(gimli::DW_AT_comp_dir)? {
                if let Some(name) = unit_entry.attr_value(gimli::DW_AT_name)? {
                    if let (Some(dir), Some(file)) = (comp_dir.string_value(&dwarf), name.string_value(&dwarf)) {
                        let dir_str = String::from_utf8_lossy(dir.as_slice()).to_string();
                        let file_str = String::from_utf8_lossy(file.as_slice()).to_string();
                        let full_path = Path::new(&dir_str).join(file_str);
                        
                        if full_path.file_name().map(|n| n.to_string_lossy().to_string()) == Some(source_file_name.clone()) {
                            is_target_unit = true;
                        }
                    }
                }
            }
        }
        
        if !is_target_unit {
            continue;
        }
        
        // 行番号情報を検索
        let mut line_program = match unit.line_program {
            Some(lp) => lp,
            None => continue,
        };
        
        let mut rows = line_program.rows();
        let mut prev_address = None;
        
        while let Some((header, row)) = rows.next_row()? {
            let file_id = match row.file_index() {
                Some(id) => id,
                None => continue,
            };
            
            // ファイル情報を取得
            let file_entry = line_program.header().file(file_id).unwrap();
            let dir_id = file_entry.directory_index();
            
            let file_name = dwarf.attr_string(&unit, file_entry.path_name())?;
            let file_name_str = String::from_utf8_lossy(file_name.as_slice()).to_string();
            
            let dir_name = if dir_id == 0 {
                None
            } else {
                match line_program.header().directory(dir_id) {
                    Some(dir) => {
                        Some(String::from_utf8_lossy(dwarf.attr_string(&unit, dir)?.as_slice()).to_string())
                    }
                    None => None,
                }
            };
            
            let source_file = if let Some(dir) = dir_name {
                Path::new(&dir).join(file_name_str)
            } else {
                PathBuf::from(file_name_str)
            };
            
            // ファイル名で比較（パスは環境によって異なる可能性がある）
            let matches_file = source_file.file_name()
                .map(|n| n.to_string_lossy().to_string()) == Some(source_file_name.clone());
            
            // 行番号が一致し、ファイルも一致する場合
            if matches_file && row.line().map(|l| l.get()) == Some(line) {
                let address = match row.address() {
                    gimli::Address::Constant(addr) => addr,
                    _ => continue,
                };
                
                valid = true;
                found_address = Some(address);
                break;
            }
            
            prev_address = Some(row.address());
        }
        
        if valid {
            break;
        }
    }
    
    // 一致するものが見つかった場合
    Ok((valid, found_address))
}

/// ソースコード位置からバイナリアドレスを取得
pub fn get_address_for_location(binary_path: &Path, source_path: &Path, line: u32) -> Result<Option<u64>> {
    let (valid, address) = verify_breakpoint_with_dwarf(binary_path, source_path, line)?;
    if valid {
        Ok(address)
    } else {
        Ok(None)
    }
}

/// バイナリアドレスからソースコード位置を取得
pub fn get_location_for_address(binary_path: &Path, address: u64) -> Result<Option<(PathBuf, u32, Option<u32>)>> {
    // バイナリファイルをメモリマップ
    let file = File::open(binary_path)?;
    let map = unsafe { Mmap::map(&file)? };
    
    // オブジェクトファイルを解析
    let object = object::File::parse(&*map)
        .with_context(|| format!("バイナリファイル{}の解析に失敗", binary_path.display()))?;
    
    // DWARF情報を取得
    let endian = if object.is_little_endian() {
        RunTimeEndian::Little
    } else {
        RunTimeEndian::Big
    };
    
    // DWARF各セクションを取得
    let load_section = |id: gimli::SectionId| -> Result<EndianSlice<RunTimeEndian>, gimli::Error> {
        let data = object
            .section_by_name(id.name())
            .and_then(|section| section.data().ok())
            .unwrap_or(&[]);
        Ok(EndianSlice::new(data, endian))
    };
    
    // セクションを読み込む
    let dwarf = Dwarf::load(load_section)?;
    
    // addr2line コンテキストを作成
    let ctx = addr2line::Context::from_dwarf(dwarf)?;
    
    // アドレスから位置情報を検索
    if let Some(loc) = ctx.find_location(address)? {
        if let Some(file) = loc.file {
            let file_path = PathBuf::from(file);
            let line = loc.line;
            let column = loc.column;
            return Ok(Some((file_path, line.unwrap_or(0), column)));
        }
    }
    
    Ok(None)
}

/// 関数シンボル情報を取得
pub fn get_function_info(binary_path: &Path) -> Result<HashMap<String, (u64, u64)>> {
    // 関数名 -> (開始アドレス, サイズ) のマップを作成
    let mut functions = HashMap::new();
    
    // バイナリファイルをオープン
    let file = File::open(binary_path)?;
    let map = unsafe { Mmap::map(&file)? };
    
    // ELFファイルを解析
    let elf = goblin::elf::Elf::parse(&*map)?;
    
    // シンボルテーブルを処理
    for sym in &elf.syms {
        // 関数シンボルを抽出
        if sym.st_type() == goblin::elf::sym::STT_FUNC {
            if let Some(name) = elf.strtab.get_at(sym.st_name) {
                functions.insert(
                    name.to_string(),
                    (sym.st_value, sym.st_size),
                );
            }
        }
    }
    
    // デバッグ情報からより詳細な情報を取得
    enrich_function_info_from_dwarf(binary_path, &mut functions)?;
    
    Ok(functions)
}

/// DWARF情報から関数情報を充実させる
fn enrich_function_info_from_dwarf(binary_path: &Path, functions: &mut HashMap<String, (u64, u64)>) -> Result<()> {
    // バイナリファイルをメモリマップ
    let file = File::open(binary_path)?;
    let map = unsafe { Mmap::map(&file)? };
    
    // オブジェクトファイルを解析
    let object = object::File::parse(&*map)?;
    
    // DWARF情報を取得
    let endian = if object.is_little_endian() {
        RunTimeEndian::Little
    } else {
        RunTimeEndian::Big
    };
    
    // DWARF各セクションを取得
    let load_section = |id: gimli::SectionId| -> Result<EndianSlice<RunTimeEndian>, gimli::Error> {
        let data = object
            .section_by_name(id.name())
            .and_then(|section| section.data().ok())
            .unwrap_or(&[]);
        Ok(EndianSlice::new(data, endian))
    };
    
    // セクションを読み込む
    let dwarf = Dwarf::load(load_section)?;
    
    // コンパイル単位を走査
    let mut units = dwarf.units();
    while let Some(header) = units.next()? {
        let unit = dwarf.unit(header)?;
        
        // DIEツリーを走査
        let mut entries = unit.entries();
        while let Some((_, entry)) = entries.next_dfs()? {
            // 関数定義を探す
            if entry.tag() == gimli::DW_TAG_subprogram {
                let mut name = None;
                let mut low_pc = None;
                let mut high_pc = None;
                
                // 属性を取得
                let mut attrs = entry.attrs();
                while let Some(attr) = attrs.next()? {
                    match attr.name() {
                        gimli::DW_AT_name => {
                            if let Some(attr_value) = attr.string_value(&dwarf.debug_str) {
                                name = Some(attr_value.to_string_lossy()?.to_string());
                            }
                        },
                        gimli::DW_AT_low_pc => {
                            if let gimli::AttributeValue::Addr(addr) = attr.value() {
                                low_pc = Some(addr);
                            }
                        },
                        gimli::DW_AT_high_pc => {
                            match attr.value() {
                                gimli::AttributeValue::Addr(addr) => {
                                    high_pc = Some(addr);
                                },
                                gimli::AttributeValue::Udata(size) => {
                                    if let Some(start) = low_pc {
                                        high_pc = Some(start + size);
                                    }
                                },
                                _ => {}
                            }
                        },
                        _ => {}
                    }
                }
                
                // 関数情報を更新
                if let (Some(name), Some(start), Some(end)) = (name, low_pc, high_pc) {
                    functions.insert(name, (start, end - start));
                }
            }
        }
    }
    
    Ok(())
}

/// DAPメッセージハンドラー
pub struct DapProtocolHandler {
    /// デバッグセッション
    session: Arc<Mutex<DebugSession>>,
    
    /// コマンド送信チャネル
    command_tx: Option<Sender<DebugCommand>>,
    
    /// 次のシーケンス番号
    next_seq: u32,
    
    /// 未処理のリクエスト
    pending_requests: HashMap<u32, String>,
}

impl DapProtocolHandler {
    /// 新しいDAPハンドラーを作成
    pub fn new(session: Arc<Mutex<DebugSession>>, command_tx: Option<Sender<DebugCommand>>) -> Self {
        Self {
            session,
            command_tx,
            next_seq: 1,
            pending_requests: HashMap::new(),
        }
    }
    
    /// リクエストを処理
    pub async fn handle_request(&mut self, request: serde_json::Value) -> Result<serde_json::Value> {
        // リクエストを検証
        let request_seq = request.get("seq")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| anyhow!("リクエストにシーケンス番号がありません"))?;
        
        let command = request.get("command")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("リクエストにコマンドがありません"))?;
        
        let arguments = request.get("arguments").cloned().unwrap_or(json!({}));
        
        debug!("DAPリクエスト受信: seq={}, command={}", request_seq, command);
        
        // コマンドを保存
        self.pending_requests.insert(request_seq as u32, command.to_string());
        
        // コマンドに応じて処理を分岐
        let response = match command {
            "initialize" => self.handle_initialize(request_seq as u32, arguments).await?,
            "launch" => self.handle_launch(request_seq as u32, arguments).await?,
            "attach" => self.handle_attach(request_seq as u32, arguments).await?,
            "setBreakpoints" => self.handle_set_breakpoints(request_seq as u32, arguments).await?,
            "configurationDone" => self.handle_configuration_done(request_seq as u32).await?,
            "threads" => self.handle_threads(request_seq as u32).await?,
            "stackTrace" => self.handle_stack_trace(request_seq as u32, arguments).await?,
            "scopes" => self.handle_scopes(request_seq as u32, arguments).await?,
            "variables" => self.handle_variables(request_seq as u32, arguments).await?,
            "continue" => self.handle_continue(request_seq as u32, arguments).await?,
            "next" => self.handle_next(request_seq as u32, arguments).await?,
            "stepIn" => self.handle_step_in(request_seq as u32, arguments).await?,
            "stepOut" => self.handle_step_out(request_seq as u32, arguments).await?,
            "pause" => self.handle_pause(request_seq as u32, arguments).await?,
            "evaluate" => self.handle_evaluate(request_seq as u32, arguments).await?,
            "disconnect" => self.handle_disconnect(request_seq as u32, arguments).await?,
            _ => {
                warn!("未実装のコマンド: {}", command);
                self.create_error_response(
                    request_seq as u32, 
                    format!("未実装のコマンド: {}", command)
                )
            }
        };
        
        // 完了したリクエストを削除
        self.pending_requests.remove(&(request_seq as u32));
        
        Ok(response)
    }
    
    /// initializeリクエストを処理
    async fn handle_initialize(&mut self, seq: u32, arguments: serde_json::Value) -> Result<serde_json::Value> {
        // クライアント情報を取得
        let client_name = arguments.get("clientName")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        
        info!("クライアント '{}' からの初期化リクエスト", client_name);
        
        // デバッグエンジンに初期化コマンドを送信
        if let Some(tx) = &self.command_tx {
            let (resp_tx, resp_rx) = oneshot::channel();
            tx.send(DebugCommand::Initialize { response: resp_tx }).await
                .map_err(|_| anyhow!("デバッグエンジンとの通信に失敗"))?;
            
            resp_rx.await
                .map_err(|_| anyhow!("デバッグエンジンからの応答を受信できません"))??;
        }
        
        // 対応機能のレスポンスを作成
        let body = json!({
            "supportsConfigurationDoneRequest": true,
            "supportsEvaluateForHovers": true,
            "supportsStepBack": false,
            "supportsRestartRequest": false,
            "supportTerminateDebuggee": true,
            "supportsCompletionsRequest": false,
            "supportsModulesRequest": false,
            "supportsLoadedSourcesRequest": false,
            "supportsSetVariable": false,
            "supportsRestartFrame": false,
            "supportsGotoTargetsRequest": false,
            "supportsStepInTargetsRequest": false,
            "supportsReadMemoryRequest": false,
            "supportsDisassembleRequest": false,
            "supportsCancelRequest": false,
            "supportsBreakpointLocationsRequest": false,
            "supportsConditionalBreakpoints": true,
            "supportsHitConditionalBreakpoints": true,
            "supportsLogPoints": false,
            "supportsExceptionInfoRequest": false,
            "supportsValueFormattingOptions": false,
            "supportsExceptionOptions": false,
            "supportTerminateThreadsRequest": false,
            "supportDataBreakpoints": false,
            "supportsFunctionBreakpoints": true,
            "exceptionBreakpointFilters": [],
        });
        
        let response = self.create_response(seq, "initialize", true, None, Some(body));
        
        // 初期化イベントを送信
        let initialized_event = self.create_event("initialized", json!({}));
        
        // 両方のメッセージを組み合わせて返す
        let mut combined = Map::new();
        combined.insert("response".to_string(), response);
        combined.insert("event".to_string(), initialized_event);
        
        Ok(json!(combined))
    }
    
    /// launchリクエストを処理
    async fn handle_launch(&mut self, seq: u32, arguments: serde_json::Value) -> Result<serde_json::Value> {
        let program = arguments.get("program")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("プログラムパスが指定されていません"))?;
        
        let program_path = PathBuf::from(program);
        
        // 引数を取得
        let args = arguments.get("args")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_else(Vec::new);
        
        // 作業ディレクトリを取得
        let cwd = arguments.get("cwd")
            .and_then(|v| v.as_str())
            .map(PathBuf::from);
        
        // ストップオンエントリーを取得
        let stop_on_entry = arguments.get("stopOnEntry")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        
        // 環境変数を取得
        let env = arguments.get("env")
            .and_then(|v| v.as_object())
            .map(|obj| {
                obj.iter()
                    .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                    .collect()
            })
            .unwrap_or_else(HashMap::new);
        
        // デバッグ設定を作成
        let config = DebugConfiguration {
            name: format!("Launch: {}", program),
            type_name: "swiftlight".to_string(),
            request: "launch".to_string(),
            program: program_path,
            cwd,
            args,
            env,
            address: None,
            port: None,
            source_map: HashMap::new(),
            stop_on_entry,
        };
        
        // デバッグエンジンにlaunchコマンドを送信
        if let Some(tx) = &self.command_tx {
            let (resp_tx, resp_rx) = oneshot::channel();
            tx.send(DebugCommand::Launch { config, response: resp_tx }).await
                .map_err(|_| anyhow!("デバッグエンジンとの通信に失敗"))?;
            
            resp_rx.await
                .map_err(|_| anyhow!("デバッグエンジンからの応答を受信できません"))??;
        }
        
        let response = self.create_response(seq, "launch", true, None, None);
        Ok(response)
    }
    
    /// attachリクエストを処理
    async fn handle_attach(&mut self, seq: u32, arguments: serde_json::Value) -> Result<serde_json::Value> {
        // プロセスIDまたはホスト/ポートを取得
        let pid = arguments.get("processId")
            .and_then(|v| v.as_u64())
            .map(|id| id as u32);
        
        let host = arguments.get("address")
            .and_then(|v| v.as_str())
            .or_else(|| arguments.get("host").and_then(|v| v.as_str()))
            .map(|s| s.to_string());
        
        let port = arguments.get("port")
            .and_then(|v| v.as_u64())
            .map(|p| p as u16);
        
        if pid.is_none() && (host.is_none() || port.is_none()) {
            return Ok(self.create_error_response(
                seq,
                "プロセスIDまたはホスト/ポートを指定してください".to_string()
            ));
        }
        
        // プログラムパスを取得
        let program = arguments.get("program")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("プログラムパスが指定されていません"))?;
        
        let program_path = PathBuf::from(program);
        
        // デバッグ設定を作成
        let config = DebugConfiguration {
            name: format!("Attach: {}", program),
            type_name: "swiftlight".to_string(),
            request: "attach".to_string(),
            program: program_path,
            cwd: None,
            args: Vec::new(),
            env: HashMap::new(),
            address: host,
            port,
            source_map: HashMap::new(),
            stop_on_entry: false,
        };
        
        // デバッグエンジンにattachコマンドを送信
        if let Some(tx) = &self.command_tx {
            let (resp_tx, resp_rx) = oneshot::channel();
            tx.send(DebugCommand::Attach { config, pid, response: resp_tx }).await
                .map_err(|_| anyhow!("デバッグエンジンとの通信に失敗"))?;
            
            resp_rx.await
                .map_err(|_| anyhow!("デバッグエンジンからの応答を受信できません"))??;
        }
        
        let response = self.create_response(seq, "attach", true, None, None);
        Ok(response)
    }
    
    /// レスポンスを作成
    fn create_response(&mut self, request_seq: u32, command: &str, success: bool, 
                      message: Option<String>, body: Option<serde_json::Value>) -> serde_json::Value {
        let seq = self.next_seq();
        
        let mut response = json!({
            "type": "response",
            "seq": seq,
            "request_seq": request_seq,
            "command": command,
            "success": success
        });
        
        if let Some(msg) = message {
            response["message"] = json!(msg);
        }
        
        if let Some(b) = body {
            response["body"] = b;
        }
        
        response
    }
    
    /// エラーレスポンスを作成
    fn create_error_response(&mut self, request_seq: u32, message: String) -> serde_json::Value {
        // リクエストのコマンドを取得
        let command = self.pending_requests.get(&request_seq)
            .cloned()
            .unwrap_or_else(|| "unknown".to_string());
        
        self.create_response(request_seq, &command, false, Some(message), None)
    }
    
    /// イベントを作成
    fn create_event(&mut self, event_type: &str, body: serde_json::Value) -> serde_json::Value {
        let seq = self.next_seq();
        
        json!({
            "type": "event",
            "seq": seq,
            "event": event_type,
            "body": body
        })
    }
    
    /// 次のシーケンス番号を取得
    fn next_seq(&mut self) -> u32 {
        let seq = self.next_seq;
        self.next_seq += 1;
        seq
    }
    
    /// ブレークポイント設定リクエストを処理
    async fn handle_set_breakpoints(&mut self, seq: u32, arguments: serde_json::Value) -> Result<serde_json::Value> {
        // ソースファイルパスを取得
        let source = arguments.get("source")
            .and_then(|v| v.as_object())
            .ok_or_else(|| anyhow!("ソース情報が指定されていません"))?;
        
        let path = source.get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("ソースパスが指定されていません"))?;
        
        let source_path = PathBuf::from(path);
        
        // ブレークポイント情報を取得
        let breakpoints = arguments.get("breakpoints")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| {
                        if let Some(obj) = v.as_object() {
                            let line = obj.get("line")
                                .and_then(|l| l.as_u64())
                                .unwrap_or(0) as usize;
                            
                            let column = obj.get("column")
                                .and_then(|c| c.as_u64())
                                .map(|c| c as usize);
                            
                            let condition = obj.get("condition")
                                .and_then(|c| c.as_str())
                                .map(|s| s.to_string());
                            
                            let hit_condition = obj.get("hitCondition")
                                .and_then(|h| h.as_str())
                                .map(|s| s.to_string());
                            
                            let log_message = obj.get("logMessage")
                                .and_then(|l| l.as_str())
                                .map(|s| s.to_string());
                            
                            let id = {
                                let mut session = self.session.lock().unwrap();
                                session.next_breakpoint_id()
                            };
                            
                            Some(Breakpoint {
                                id,
                                verified: false, // 後で検証する
                                source_path: source_path.clone(),
                                line,
                                column,
                                condition,
                                hit_condition,
                                log_message,
                                address: None,
                                original_byte: None,
                                temporary: false,
                            })
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_else(Vec::new);
        
        // ブレークポイントを設定
        let mut result_breakpoints = Vec::new();
        
        if let Some(tx) = &self.command_tx {
            let (resp_tx, resp_rx) = oneshot::channel();
            tx.send(DebugCommand::SetBreakpoints {
                source: source_path.clone(),
                breakpoints: breakpoints.clone(),
                response: resp_tx,
            }).await
                .map_err(|_| anyhow!("デバッグエンジンとの通信に失敗"))?;
            
            match resp_rx.await {
                Ok(Ok(bps)) => {
                    result_breakpoints = bps;
                },
                Ok(Err(e)) => {
                    warn!("ブレークポイント設定エラー: {}", e);
                    // エラー時は元のブレークポイントを使用し、未検証とする
                    result_breakpoints = breakpoints;
                },
                Err(_) => {
                    warn!("デバッグエンジンからの応答を受信できません");
                    result_breakpoints = breakpoints;
                }
            }
        } else {
            // デバッグエンジンがない場合は検証せずに返す
            result_breakpoints = breakpoints;
        }
        
        // レスポンスを作成
        let body = json!({
            "breakpoints": result_breakpoints.iter().map(|bp| {
                json!({
                    "id": bp.id,
                    "verified": bp.verified,
                    "line": bp.line,
                    "column": bp.column,
                    "message": if !bp.verified {
                        Some("ブレークポイントを設定できませんでした")
                    } else {
                        None
                    }
                })
            }).collect::<Vec<_>>()
        });
        
        let response = self.create_response(seq, "setBreakpoints", true, None, Some(body));
        Ok(response)
    }
    
    /// configuration done リクエストを処理
    async fn handle_configuration_done(&mut self, seq: u32) -> Result<serde_json::Value> {
        // デバッグセッションに設定完了を通知
        {
            let mut session = self.session.lock().unwrap();
            session.configuration_done();
        }
        
        // デバッグエンジンに実行開始を通知
        if let Some(tx) = &self.command_tx {
            // 必要に応じて実装
        }
        
        let response = self.create_response(seq, "configurationDone", true, None, None);
        Ok(response)
    }
    
    /// threads リクエストを処理
    async fn handle_threads(&mut self, seq: u32) -> Result<serde_json::Value> {
        // スレッド情報を取得
        let threads = {
            let session = self.session.lock().unwrap();
            session.get_threads()
                .iter()
                .map(|t| {
                    json!({
                        "id": t.id,
                        "name": &t.name
                    })
                })
                .collect::<Vec<_>>()
        };
        
        let body = json!({
            "threads": threads
        });
        
        let response = self.create_response(seq, "threads", true, None, Some(body));
        Ok(response)
    }
    
    /// stackTrace リクエストを処理
    async fn handle_stack_trace(&mut self, seq: u32, arguments: serde_json::Value) -> Result<serde_json::Value> {
        // スレッドIDを取得
        let thread_id = arguments.get("threadId")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| anyhow!("スレッドIDが指定されていません"))? as usize;
        
        // スタックフレーム情報を取得
        let frames = if let Some(tx) = &self.command_tx {
            let (resp_tx, resp_rx) = oneshot::channel();
            tx.send(DebugCommand::GetStackTrace {
                thread_id,
                response: resp_tx,
            }).await
                .map_err(|_| anyhow!("デバッグエンジンとの通信に失敗"))?;
            
            match resp_rx.await {
                Ok(Ok(frames)) => frames,
                Ok(Err(e)) => {
                    warn!("スタックトレース取得エラー: {}", e);
                    Vec::new()
                },
                Err(_) => {
                    warn!("デバッグエンジンからの応答を受信できません");
                    Vec::new()
                }
            }
        } else {
            // デバッグエンジンがない場合は空のリストを返す
            Vec::new()
        };
        
        // レスポンスを作成
        let body = json!({
            "stackFrames": frames.iter().map(|frame| {
                let source = json!({
                    "name": frame.source_path.file_name().unwrap_or_default().to_string_lossy(),
                    "path": frame.source_path.to_string_lossy()
                });
                
                json!({
                    "id": frame.id,
                    "name": frame.name,
                    "source": source,
                    "line": frame.line,
                    "column": frame.column,
                    "presentationHint": frame.presentation_hint
                })
            }).collect::<Vec<_>>(),
            "totalFrames": frames.len()
        });
        
        let response = self.create_response(seq, "stackTrace", true, None, Some(body));
        Ok(response)
    }
    
    /// scopes リクエストを処理
    async fn handle_scopes(&mut self, seq: u32, arguments: serde_json::Value) -> Result<serde_json::Value> {
        // フレームIDを取得
        let frame_id = arguments.get("frameId")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| anyhow!("フレームIDが指定されていません"))? as usize;
        
        // スコープ情報を作成
        let local_variables_reference = {
            let mut session = self.session.lock().unwrap();
            session.next_variable_id()
        };
        
        let global_variables_reference = {
            let mut session = self.session.lock().unwrap();
            session.next_variable_id()
        };
        
        let register_variables_reference = {
            let mut session = self.session.lock().unwrap();
            session.next_variable_id()
        };
        
        // レスポンスを作成
        let body = json!({
            "scopes": [
                {
                    "name": "ローカル変数",
                    "presentationHint": "locals",
                    "variablesReference": local_variables_reference,
                    "expensive": false
                },
                {
                    "name": "グローバル変数",
                    "presentationHint": "globals",
                    "variablesReference": global_variables_reference,
                    "expensive": true
                },
                {
                    "name": "レジスタ",
                    "presentationHint": "registers",
                    "variablesReference": register_variables_reference,
                    "expensive": true
                }
            ]
        });
        
        let response = self.create_response(seq, "scopes", true, None, Some(body));
        Ok(response)
    }
    
    /// variables リクエストを処理
    async fn handle_variables(&mut self, seq: u32, arguments: serde_json::Value) -> Result<serde_json::Value> {
        // 変数参照IDを取得
        let variable_reference = arguments.get("variablesReference")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| anyhow!("変数参照IDが指定されていません"))? as usize;
        
        // 変数情報を取得
        let variables = if let Some(tx) = &self.command_tx {
            let (resp_tx, resp_rx) = oneshot::channel();
            tx.send(DebugCommand::GetVariables {
                variable_reference,
                response: resp_tx,
            }).await
                .map_err(|_| anyhow!("デバッグエンジンとの通信に失敗"))?;
            
            match resp_rx.await {
                Ok(Ok(vars)) => vars,
                Ok(Err(e)) => {
                    warn!("変数取得エラー: {}", e);
                    Vec::new()
                },
                Err(_) => {
                    warn!("デバッグエンジンからの応答を受信できません");
                    Vec::new()
                }
            }
        } else {
            // デバッグエンジンがない場合は空のリストを返す
            Vec::new()
        };
        
        // レスポンスを作成
        let body = json!({
            "variables": variables.iter().map(|var| {
                let mut v = json!({
                    "name": var.name,
                    "value": var.value,
                    "type": var.type_name,
                    "variablesReference": var.variable_reference.unwrap_or(0),
                });
                
                if let Some(mem_ref) = &var.memory_reference {
                    v["memoryReference"] = json!(mem_ref);
                }
                
                if var.indexed_variables > 0 {
                    v["indexedVariables"] = json!(var.indexed_variables);
                }
                
                if var.named_variables > 0 {
                    v["namedVariables"] = json!(var.named_variables);
                }
                
                v
            }).collect::<Vec<_>>()
        });
        
        let response = self.create_response(seq, "variables", true, None, Some(body));
        Ok(response)
    }
    
    /// continue リクエストを処理
    async fn handle_continue(&mut self, seq: u32, arguments: serde_json::Value) -> Result<serde_json::Value> {
        // スレッドIDを取得
        let thread_id = arguments.get("threadId")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as usize;
        
        // 継続実行
        if let Some(tx) = &self.command_tx {
            let (resp_tx, resp_rx) = oneshot::channel();
            tx.send(DebugCommand::ExecutionControl {
                mode: ExecutionMode::Continue,
                thread_id,
                response: resp_tx,
            }).await
                .map_err(|_| anyhow!("デバッグエンジンとの通信に失敗"))?;
            
            match resp_rx.await {
                Ok(Ok(_)) => (),
                Ok(Err(e)) => {
                    warn!("実行継続エラー: {}", e);
                    return Ok(self.create_error_response(seq, e.to_string()));
                },
                Err(_) => {
                    warn!("デバッグエンジンからの応答を受信できません");
                    return Ok(self.create_error_response(seq, "デバッグエンジンからの応答を受信できません".to_string()));
                }
            }
        }
        
        // レスポンスを作成
        let body = json!({
            "allThreadsContinued": true
        });
        
        let response = self.create_response(seq, "continue", true, None, Some(body));
        Ok(response)
    }
    
    /// next リクエストを処理
    async fn handle_next(&mut self, seq: u32, arguments: serde_json::Value) -> Result<serde_json::Value> {
        // スレッドIDを取得
        let thread_id = arguments.get("threadId")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| anyhow!("スレッドIDが指定されていません"))? as usize;
        
        // ステップオーバー実行
        if let Some(tx) = &self.command_tx {
            let (resp_tx, resp_rx) = oneshot::channel();
            tx.send(DebugCommand::ExecutionControl {
                mode: ExecutionMode::StepOver,
                thread_id,
                response: resp_tx,
            }).await
                .map_err(|_| anyhow!("デバッグエンジンとの通信に失敗"))?;
            
            match resp_rx.await {
                Ok(Ok(_)) => (),
                Ok(Err(e)) => {
                    warn!("ステップオーバーエラー: {}", e);
                    return Ok(self.create_error_response(seq, e.to_string()));
                },
                Err(_) => {
                    warn!("デバッグエンジンからの応答を受信できません");
                    return Ok(self.create_error_response(seq, "デバッグエンジンからの応答を受信できません".to_string()));
                }
            }
        }
        
        let response = self.create_response(seq, "next", true, None, None);
        Ok(response)
    }
    
    /// stepIn リクエストを処理
    async fn handle_step_in(&mut self, seq: u32, arguments: serde_json::Value) -> Result<serde_json::Value> {
        // スレッドIDを取得
        let thread_id = arguments.get("threadId")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| anyhow!("スレッドIDが指定されていません"))? as usize;
        
        // ステップイン実行
        if let Some(tx) = &self.command_tx {
            let (resp_tx, resp_rx) = oneshot::channel();
            tx.send(DebugCommand::ExecutionControl {
                mode: ExecutionMode::StepIn,
                thread_id,
                response: resp_tx,
            }).await
                .map_err(|_| anyhow!("デバッグエンジンとの通信に失敗"))?;
            
            match resp_rx.await {
                Ok(Ok(_)) => (),
                Ok(Err(e)) => {
                    warn!("ステップインエラー: {}", e);
                    return Ok(self.create_error_response(seq, e.to_string()));
                },
                Err(_) => {
                    warn!("デバッグエンジンからの応答を受信できません");
                    return Ok(self.create_error_response(seq, "デバッグエンジンからの応答を受信できません".to_string()));
                }
            }
        }
        
        let response = self.create_response(seq, "stepIn", true, None, None);
        Ok(response)
    }
    
    /// stepOut リクエストを処理
    async fn handle_step_out(&mut self, seq: u32, arguments: serde_json::Value) -> Result<serde_json::Value> {
        // スレッドIDを取得
        let thread_id = arguments.get("threadId")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| anyhow!("スレッドIDが指定されていません"))? as usize;
        
        // ステップアウト実行
        if let Some(tx) = &self.command_tx {
            let (resp_tx, resp_rx) = oneshot::channel();
            tx.send(DebugCommand::ExecutionControl {
                mode: ExecutionMode::StepOut,
                thread_id,
                response: resp_tx,
            }).await
                .map_err(|_| anyhow!("デバッグエンジンとの通信に失敗"))?;
            
            match resp_rx.await {
                Ok(Ok(_)) => (),
                Ok(Err(e)) => {
                    warn!("ステップアウトエラー: {}", e);
                    return Ok(self.create_error_response(seq, e.to_string()));
                },
                Err(_) => {
                    warn!("デバッグエンジンからの応答を受信できません");
                    return Ok(self.create_error_response(seq, "デバッグエンジンからの応答を受信できません".to_string()));
                }
            }
        }
        
        let response = self.create_response(seq, "stepOut", true, None, None);
        Ok(response)
    }
    
    /// pause リクエストを処理
    async fn handle_pause(&mut self, seq: u32, arguments: serde_json::Value) -> Result<serde_json::Value> {
        // スレッドIDを取得
        let thread_id = arguments.get("threadId")
            .and_then(|v| v.as_u64())
            .map(|id| id as usize);
        
        // 一時停止
        if let Some(tx) = &self.command_tx {
            let (resp_tx, resp_rx) = oneshot::channel();
            tx.send(DebugCommand::Pause {
                thread_id,
                response: resp_tx,
            }).await
                .map_err(|_| anyhow!("デバッグエンジンとの通信に失敗"))?;
            
            match resp_rx.await {
                Ok(Ok(_)) => (),
                Ok(Err(e)) => {
                    warn!("一時停止エラー: {}", e);
                    return Ok(self.create_error_response(seq, e.to_string()));
                },
                Err(_) => {
                    warn!("デバッグエンジンからの応答を受信できません");
                    return Ok(self.create_error_response(seq, "デバッグエンジンからの応答を受信できません".to_string()));
                }
            }
        }
        
        let response = self.create_response(seq, "pause", true, None, None);
        Ok(response)
    }
    
    /// evaluate リクエストを処理
    async fn handle_evaluate(&mut self, seq: u32, arguments: serde_json::Value) -> Result<serde_json::Value> {
        // 式を取得
        let expression = arguments.get("expression")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("式が指定されていません"))?;
        
        // フレームIDを取得
        let frame_id = arguments.get("frameId")
            .and_then(|v| v.as_u64())
            .map(|id| id as usize);
        
        // コンテキストを取得
        let context = arguments.get("context")
            .and_then(|v| v.as_str())
            .unwrap_or("evaluate");
        
        // 式を評価
        if let Some(tx) = &self.command_tx {
            let (resp_tx, resp_rx) = oneshot::channel();
            tx.send(DebugCommand::Evaluate {
                expression: expression.to_string(),
                frame_id,
                context: context.to_string(),
                response: resp_tx,
            }).await
                .map_err(|_| anyhow!("デバッグエンジンとの通信に失敗"))?;
            
            match resp_rx.await {
                Ok(Ok(variable)) => {
                    // レスポンスを作成
                    let body = json!({
                        "result": variable.value,
                        "type": variable.type_name,
                        "variablesReference": variable.variable_reference.unwrap_or(0),
                        "namedVariables": variable.named_variables,
                        "indexedVariables": variable.indexed_variables
                    });
                    
                    let response = self.create_response(seq, "evaluate", true, None, Some(body));
                    return Ok(response);
                },
                Ok(Err(e)) => {
                    warn!("式評価エラー: {}", e);
                    return Ok(self.create_error_response(seq, e.to_string()));
                },
                Err(_) => {
                    warn!("デバッグエンジンからの応答を受信できません");
                    return Ok(self.create_error_response(seq, "デバッグエンジンからの応答を受信できません".to_string()));
                }
            }
        }
        
        // デバッグエンジンがない場合
        Ok(self.create_error_response(seq, "式評価はサポートされていません".to_string()))
    }
    
    /// disconnect リクエストを処理
    async fn handle_disconnect(&mut self, seq: u32, arguments: serde_json::Value) -> Result<serde_json::Value> {
        // 終了フラグを取得
        let terminate = arguments.get("terminateDebuggee")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        
        // デバッグセッションを終了
        if let Some(tx) = &self.command_tx {
            let (resp_tx, resp_rx) = oneshot::channel();
            tx.send(DebugCommand::Disconnect {
                terminate,
                response: resp_tx,
            }).await
                .map_err(|_| anyhow!("デバッグエンジンとの通信に失敗"))?;
            
            match resp_rx.await {
                Ok(Ok(_)) => (),
                Ok(Err(e)) => {
                    warn!("切断エラー: {}", e);
                    return Ok(self.create_error_response(seq, e.to_string()));
                },
                Err(_) => {
                    warn!("デバッグエンジンからの応答を受信できません");
                    return Ok(self.create_error_response(seq, "デバッグエンジンからの応答を受信できません".to_string()));
                }
            }
        }
        
        let response = self.create_response(seq, "disconnect", true, None, None);
        Ok(response)
    }
    Ok(())
    }
