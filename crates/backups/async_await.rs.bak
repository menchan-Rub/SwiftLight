// async_await.rs - SwiftLight Async/Await実装
//
// このモジュールは、SwiftLight言語の非同期プログラミングをサポートするための
// async/await構文の変換と実行を担当します。これにより、非同期コードを同期的な
// スタイルで書くことができ、可読性と保守性が向上します。

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;

use crate::middleend::ir::{
    BasicBlock, Function, Instruction, Module, Value, ValueId, 
    Type, TypeId, InstructionId, FunctionId, Parameter
};
use crate::frontend::ast;
use crate::frontend::semantic::type_checker::TypeCheckResult;
use super::future::{FutureAnalyzer, AsyncFunctionInfo, FutureState, AwaitPoint};

/// Async/Await変換マネージャー
pub struct AsyncAwaitTransformer {
    /// モジュール
    module: Option<Module>,
    
    /// Future解析器
    future_analyzer: Option<FutureAnalyzer>,
    
    /// 変換済み関数のマップ
    transformed_functions: HashMap<FunctionId, TransformedFunction>,
    
    /// 生成された状態マシン
    state_machines: HashMap<FunctionId, StateMachine>,
    
    /// 変換モード
    transformation_mode: TransformationMode,
    
    /// パフォーマンス統計
    stats: TransformationStats,
}

/// 変換モード
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransformationMode {
    /// CPS（継続渡しスタイル）変換
    CPS,
    
    /// 状態マシン変換
    StateMachine,
    
    /// ハイブリッド変換（状態マシン + CPS）
    Hybrid,
}

/// 変換されたコード統計
#[derive(Debug, Clone, Default)]
pub struct TransformationStats {
    /// 変換された関数数
    pub transformed_function_count: usize,
    
    /// 生成された状態マシン数
    pub state_machine_count: usize,
    
    /// 生成されたコード量（行数）
    pub generated_code_lines: usize,
    
    /// 変換前のコード量（行数）
    pub original_code_lines: usize,
    
    /// CPS変換数
    pub cps_transformations: usize,
    
    /// 状態マシン変換数
    pub state_machine_transformations: usize,
    
    /// ハイブリッド変換数
    pub hybrid_transformations: usize,
}

/// 変換された関数情報
#[derive(Debug, Clone)]
pub struct TransformedFunction {
    /// 元の関数ID
    pub original_function_id: FunctionId,
    
    /// 変換されたコードID
    pub transformed_function_id: FunctionId,
    
    /// 変換モード
    pub transformation_mode: TransformationMode,
    
    /// 状態マシンID（存在する場合）
    pub state_machine_id: Option<FunctionId>,
    
    /// awaitポイント数
    pub await_point_count: usize,
    
    /// 変換前のコードサイズ（バイト数）
    pub original_size: usize,
    
    /// 変換後のコードサイズ（バイト数）
    pub transformed_size: usize,
}

/// 状態マシン定義
#[derive(Debug, Clone)]
pub struct StateMachine {
    /// 状態マシンID
    pub id: usize,
    
    /// 関連する関数ID
    pub function_id: FunctionId,
    
    /// 状態数
    pub state_count: usize,
    
    /// 状態変数ID
    pub state_variable_id: ValueId,
    
    /// 状態遷移マップ（現在状態 -> 遷移先候補）
    pub transitions: HashMap<usize, Vec<StateTransition>>,
    
    /// 状態マシンコンテキスト型
    pub context_type_id: TypeId,
    
    /// リジューム関数
    pub resume_function_id: FunctionId,
    
    /// ポーリング関数
    pub poll_function_id: FunctionId,
}

/// 状態遷移
#[derive(Debug, Clone)]
pub struct StateTransition {
    /// 遷移元状態
    pub from_state: usize,
    
    /// 遷移先状態
    pub to_state: usize,
    
    /// 遷移条件（存在する場合）
    pub condition: Option<ValueId>,
    
    /// 遷移アクション関数（存在する場合）
    pub action: Option<FunctionId>,
    
    /// awaitポイント（遷移が発生する場所）
    pub await_point: Option<AwaitPoint>,
}

impl AsyncAwaitTransformer {
    /// 新しいAsync/Await変換マネージャーを作成
    pub fn new() -> Self {
        Self {
            module: None,
            future_analyzer: None,
            transformed_functions: HashMap::new(),
            state_machines: HashMap::new(),
            transformation_mode: TransformationMode::StateMachine,
            stats: TransformationStats::default(),
        }
    }
    
    /// モジュールを設定
    pub fn set_module(&mut self, module: Module) {
        self.module = Some(module.clone());
        
        let mut analyzer = FutureAnalyzer::new();
        analyzer.set_module(module);
        self.future_analyzer = Some(analyzer);
    }
    
    /// 変換モードを設定
    pub fn set_transformation_mode(&mut self, mode: TransformationMode) {
        self.transformation_mode = mode;
    }
    
    /// 非同期関数を変換
    pub fn transform(&mut self) -> Result<(), String> {
        let module = self.module.as_ref().ok_or("モジュールが設定されていません")?;
        let analyzer = self.future_analyzer.as_mut().ok_or("Future解析器が設定されていません")?;
        
        // Future型と非同期関数を解析
        analyzer.analyze()?;
        
        // すべての非同期関数を変換
        for (func_id, function) in &module.functions {
            if function.is_async {
                // 非同期関数情報を取得
                if let Some(async_info) = analyzer.get_async_function_info(*func_id) {
                    // 変換モードに応じて関数を変換
                    match self.transformation_mode {
                        TransformationMode::CPS => {
                            self.transform_to_cps(*func_id, async_info)?;
                            self.stats.cps_transformations += 1;
                        },
                        TransformationMode::StateMachine => {
                            self.transform_to_state_machine(*func_id, async_info)?;
                            self.stats.state_machine_transformations += 1;
                        },
                        TransformationMode::Hybrid => {
                            self.transform_to_hybrid(*func_id, async_info)?;
                            self.stats.hybrid_transformations += 1;
                        },
                    }
                    
                    self.stats.transformed_function_count += 1;
                }
            }
        }
        
        self.stats.state_machine_count = self.state_machines.len();
        
        Ok(())
    }
    
    /// CPS（継続渡しスタイル）に変換
    fn transform_to_cps(&mut self, func_id: FunctionId, async_info: &AsyncFunctionInfo) -> Result<(), String> {
        let module = self.module.as_ref().ok_or("モジュールが設定されていません")?;
        
        // CPS変換の場合、各awaitポイントは継続関数に変換される
        if let Some(function) = module.functions.get(&func_id) {
            // 元のコードサイズを計算
            let original_size = self.calculate_function_size(function);
            
            // ここではダミーの変換結果を作成
            // 実際の実装では、awaitポイントごとに継続関数を作成し、元の関数を変換する
            let transformed_function = TransformedFunction {
                original_function_id: func_id,
                transformed_function_id: func_id, // 同じIDを使用（実際は新しいIDが必要）
                transformation_mode: TransformationMode::CPS,
                state_machine_id: None,
                await_point_count: async_info.await_points.len(),
                original_size,
                transformed_size: original_size, // 同じサイズを使用（実際は変換後のサイズが必要）
            };
            
            self.transformed_functions.insert(func_id, transformed_function);
            
            // コード行数統計を更新
            self.stats.original_code_lines += self.count_function_lines(function);
            self.stats.generated_code_lines += self.count_function_lines(function); // 仮の値
        }
        
        Ok(())
    }
    
    /// 状態マシンに変換
    fn transform_to_state_machine(&mut self, func_id: FunctionId, async_info: &AsyncFunctionInfo) -> Result<(), String> {
        let module = self.module.as_ref().ok_or("モジュールが設定されていません")?;
        
        if let Some(function) = module.functions.get(&func_id) {
            // 元のコードサイズを計算
            let original_size = self.calculate_function_size(function);
            
            // 状態マシンを作成
            let state_machine_id = self.create_state_machine(func_id, async_info)?;
            
            // 変換結果を記録
            let transformed_function = TransformedFunction {
                original_function_id: func_id,
                transformed_function_id: func_id, // 同じIDを使用（実際は新しいIDが必要）
                transformation_mode: TransformationMode::StateMachine,
                state_machine_id: Some(state_machine_id),
                await_point_count: async_info.await_points.len(),
                original_size,
                transformed_size: original_size * 2, // 仮の値（実際は変換後のサイズが必要）
            };
            
            self.transformed_functions.insert(func_id, transformed_function);
            
            // コード行数統計を更新
            self.stats.original_code_lines += self.count_function_lines(function);
            self.stats.generated_code_lines += self.count_function_lines(function) * 2; // 仮の値
        }
        
        Ok(())
    }
    
    /// ハイブリッド変換（状態マシン + CPS）
    fn transform_to_hybrid(&mut self, func_id: FunctionId, async_info: &AsyncFunctionInfo) -> Result<(), String> {
        let module = self.module.as_ref().ok_or("モジュールが設定されていません")?;
        
        if let Some(function) = module.functions.get(&func_id) {
            // 元のコードサイズを計算
            let original_size = self.calculate_function_size(function);
            
            // 状態マシンを作成
            let state_machine_id = self.create_state_machine(func_id, async_info)?;
            
            // 変換結果を記録
            let transformed_function = TransformedFunction {
                original_function_id: func_id,
                transformed_function_id: func_id, // 同じIDを使用（実際は新しいIDが必要）
                transformation_mode: TransformationMode::Hybrid,
                state_machine_id: Some(state_machine_id),
                await_point_count: async_info.await_points.len(),
                original_size,
                transformed_size: original_size * 3, // 仮の値（実際は変換後のサイズが必要）
            };
            
            self.transformed_functions.insert(func_id, transformed_function);
            
            // コード行数統計を更新
            self.stats.original_code_lines += self.count_function_lines(function);
            self.stats.generated_code_lines += self.count_function_lines(function) * 3; // 仮の値
        }
        
        Ok(())
    }
    
    /// 状態マシンを作成
    fn create_state_machine(&mut self, func_id: FunctionId, async_info: &AsyncFunctionInfo) -> Result<FunctionId, String> {
        let module = self.module.as_ref().ok_or("モジュールが設定されていません")?;
        
        // 状態マシンID（ここでは関数IDを使用）
        let state_machine_id = func_id;
        
        // 状態数は、awaitポイント数 + 1（初期状態）+ 1（完了状態）
        let state_count = async_info.await_points.len() + 2;
        
        // 状態遷移を構築
        let mut transitions = HashMap::new();
        
        // 初期状態（0）から最初のawaitポイントへの遷移
        let mut initial_transitions = Vec::new();
        if !async_info.await_points.is_empty() {
            initial_transitions.push(StateTransition {
                from_state: 0,
                to_state: 1,
                condition: None,
                action: None,
                await_point: None,
            });
        } else {
            // awaitポイントがない場合は完了状態に直接遷移
            initial_transitions.push(StateTransition {
                from_state: 0,
                to_state: state_count - 1, // 完了状態
                condition: None,
                action: None,
                await_point: None,
            });
        }
        transitions.insert(0, initial_transitions);
        
        // 各awaitポイントの遷移
        for (i, await_point) in async_info.await_points.iter().enumerate() {
            let current_state = i + 1;
            let next_state = if i + 1 < async_info.await_points.len() {
                i + 2
            } else {
                // 最後のawaitポイントは完了状態に遷移
                state_count - 1
            };
            
            let transition = StateTransition {
                from_state: current_state,
                to_state: next_state,
                condition: None,
                action: None,
                await_point: Some(await_point.clone()),
            };
            
            transitions.insert(current_state, vec![transition]);
        }
        
        // 完了状態（遷移なし）
        transitions.insert(state_count - 1, Vec::new());
        
        // 状態マシンを作成
        let state_machine = StateMachine {
            id: self.state_machines.len(), // 次の利用可能なID
            function_id: func_id,
            state_count,
            state_variable_id: 0, // 仮の値
            transitions,
            context_type_id: 0, // 仮の値
            resume_function_id: 0, // 仮の値
            poll_function_id: 0, // 仮の値
        };
        
        self.state_machines.insert(state_machine_id, state_machine);
        
        Ok(state_machine_id)
    }
    
    /// 関数のサイズを計算（命令数）
    fn calculate_function_size(&self, function: &Function) -> usize {
        let mut size = 0;
        
        // 基本ブロックの数
        size += function.basic_blocks.len();
        
        // 命令の数
        for block in &function.basic_blocks {
            size += block.instructions.len();
        }
        
        size
    }
    
    /// 関数の行数を数える（概算）
    fn count_function_lines(&self, function: &Function) -> usize {
        // 関数宣言（シグネチャ）= 1行
        let mut lines = 1;
        
        // パラメータごとに1行
        lines += function.parameters.len();
        
        // 各基本ブロックは2行（ラベルと終了）
        lines += function.basic_blocks.len() * 2;
        
        // 各命令は1行
        for block in &function.basic_blocks {
            lines += block.instructions.len();
        }
        
        // 終了ブレース = 1行
        lines += 1;
        
        lines
    }
    
    /// 変換結果を取得
    pub fn get_transformed_function(&self, func_id: FunctionId) -> Option<&TransformedFunction> {
        self.transformed_functions.get(&func_id)
    }
    
    /// 状態マシンを取得
    pub fn get_state_machine(&self, func_id: FunctionId) -> Option<&StateMachine> {
        self.state_machines.get(&func_id)
    }
    
    /// 変換統計を取得
    pub fn get_stats(&self) -> &TransformationStats {
        &self.stats
    }
}

/// 非同期Traitの定義（コンパイル時に使用）
pub trait AsyncTrait {
    /// 関連型：非同期関数の戻り値の型
    type Output;
    
    /// ポーリングメソッド
    fn poll(&mut self) -> PollResult<Self::Output>;
}

/// ポーリング結果
#[derive(Debug, Clone)]
pub enum PollResult<T> {
    /// 完了（値がある）
    Ready(T),
    
    /// 準備ができていない（まだ待機が必要）
    Pending,
}

/// Futureの実行インタフェース
pub trait Executor {
    /// Futureを実行
    fn spawn<F>(&self, future: F) -> ExecutorHandle
    where
        F: AsyncTrait + 'static;
    
    /// 現在のFutureを一時停止（yield）
    fn yield_now(&self) -> YieldFuture;
    
    /// 指定時間スリープ
    fn sleep(&self, millis: u64) -> SleepFuture;
    
    /// すべてのタスクが完了するまで待機
    fn join_all(&self, handles: Vec<ExecutorHandle>) -> JoinAllFuture;
}

/// 実行ハンドル
#[derive(Debug, Clone)]
pub struct ExecutorHandle {
    /// タスクID
    pub task_id: usize,
    
    /// 状態
    pub state: TaskState,
    
    /// タスクの優先度
    pub priority: usize,
}

/// タスク状態
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskState {
    /// 初期状態
    New,
    
    /// 実行中
    Running,
    
    /// 待機中
    Waiting,
    
    /// 完了
    Completed,
    
    /// 失敗
    Failed,
    
    /// キャンセル済み
    Cancelled,
}

/// Yield Future
#[derive(Debug, Clone)]
pub struct YieldFuture {
    /// 状態
    state: FutureState,
}

/// Sleep Future
#[derive(Debug, Clone)]
pub struct SleepFuture {
    /// 待機時間（ミリ秒）
    millis: u64,
    
    /// 開始時刻
    start_time: u64,
    
    /// 状態
    state: FutureState,
}

/// Join All Future
#[derive(Debug, Clone)]
pub struct JoinAllFuture {
    /// 待機中のハンドル
    handles: Vec<ExecutorHandle>,
    
    /// 状態
    state: FutureState,
}

/// async/await式の最適化
pub struct AsyncOptimizer {
    /// モジュール
    module: Option<Module>,
    
    /// 最適化設定
    options: AsyncOptimizationOptions,
    
    /// 最適化された関数
    optimized_functions: HashSet<FunctionId>,
    
    /// 統計
    stats: OptimizationStats,
}

/// 非同期最適化オプション
#[derive(Debug, Clone)]
pub struct AsyncOptimizationOptions {
    /// Future融合を有効化
    pub enable_future_fusion: bool,
    
    /// 不要なawaitの除去
    pub enable_await_elision: bool,
    
    /// 非同期関数のインライン化
    pub enable_async_inlining: bool,
    
    /// 遅延実行の最適化
    pub enable_lazy_execution: bool,
    
    /// 再利用可能なFutureの検出
    pub enable_future_reuse: bool,
}

/// 最適化統計
#[derive(Debug, Clone, Default)]
pub struct OptimizationStats {
    /// 融合されたFuture数
    pub fused_futures: usize,
    
    /// 除去されたawait数
    pub elided_awaits: usize,
    
    /// インライン化された非同期関数数
    pub inlined_async_functions: usize,
    
    /// 最適化された関数数
    pub optimized_function_count: usize,
    
    /// 節約されたメモリ（バイト数）
    pub saved_memory_bytes: usize,
}

impl AsyncOptimizer {
    /// 新しい非同期最適化器を作成
    pub fn new() -> Self {
        Self {
            module: None,
            options: AsyncOptimizationOptions {
                enable_future_fusion: true,
                enable_await_elision: true,
                enable_async_inlining: true,
                enable_lazy_execution: true,
                enable_future_reuse: true,
            },
            optimized_functions: HashSet::new(),
            stats: OptimizationStats::default(),
        }
    }
    
    /// モジュールを設定
    pub fn set_module(&mut self, module: Module) {
        self.module = Some(module);
    }
    
    /// 最適化オプションを設定
    pub fn set_options(&mut self, options: AsyncOptimizationOptions) {
        self.options = options;
    }
    
    /// 最適化を実行
    pub fn optimize(&mut self) -> Result<(), String> {
        let module = self.module.as_ref().ok_or("モジュールが設定されていません")?;
        
        // 各非同期関数を最適化
        for (func_id, function) in &module.functions {
            if function.is_async {
                // Future融合
                if self.options.enable_future_fusion {
                    self.fuse_futures(*func_id)?;
                }
                
                // 不要なawaitの除去
                if self.options.enable_await_elision {
                    self.elide_awaits(*func_id)?;
                }
                
                // 非同期関数のインライン化
                if self.options.enable_async_inlining {
                    self.inline_async_functions(*func_id)?;
                }
                
                // 最適化された関数として記録
                self.optimized_functions.insert(*func_id);
                self.stats.optimized_function_count += 1;
            }
        }
        
        Ok(())
    }
    
    /// Future融合を実行
    /// 連続したFutureを一つに融合して実行効率を向上させる
    fn fuse_futures(&mut self, func_id: FunctionId) -> Result<(), String> {
        let module = self.module.as_mut().ok_or("モジュールが設定されていません")?;
        let function = module.functions.get(&func_id).ok_or(format!("関数ID {}が見つかりません", func_id))?;
        
        // 制御フローグラフを構築
        let cfg = ControlFlowGraph::from_function(function);
        
        // 連続したawait式を検出
        let mut fused_count = 0;
        for block in cfg.blocks() {
            let mut await_exprs = Vec::new();
            let mut current_idx = 0;
            
            // ブロック内の連続したawait式を収集
            while current_idx < block.instructions.len() {
                if let Instruction::AwaitExpr { future_expr, result_var, .. } = &block.instructions[current_idx] {
                    await_exprs.push((current_idx, future_expr.clone(), result_var.clone()));
                } else if !await_exprs.is_empty() {
                    // 連続していないawait式を見つけたら、それまでに収集したものを処理
                    self.fuse_await_sequence(function, block, &await_exprs)?;
                    fused_count += await_exprs.len() - 1;
                    await_exprs.clear();
                }
                current_idx += 1;
            }
            
            // ブロック終端の連続したawait式を処理
            if await_exprs.len() > 1 {
                self.fuse_await_sequence(function, block, &await_exprs)?;
                fused_count += await_exprs.len() - 1;
            }
        }
        
        // 統計を更新
        self.stats.fused_futures += fused_count;
        
        Ok(())
    }
    
    /// 連続したawait式のシーケンスを融合する
    fn fuse_await_sequence(&mut self, function: &mut Function, block: &mut BasicBlock, await_exprs: &[(usize, Expression, Variable)]) -> Result<(), String> {
        if await_exprs.len() <= 1 {
            return Ok(());
        }
        
        // 新しい融合されたFuture型を作成
        let fused_future_type = self.create_fused_future_type(await_exprs)?;
        
        // 融合されたFutureを構築する命令を生成
        let futures_to_fuse: Vec<Expression> = await_exprs.iter().map(|(_, expr, _)| expr.clone()).collect();
        let fused_var = function.create_temp_variable(fused_future_type);
        let fuse_instr = Instruction::CreateFusedFuture {
            futures: futures_to_fuse,
            result_var: fused_var.clone(),
        };
        
        // 最初のawait式の位置に融合命令を挿入
        let first_idx = await_exprs[0].0;
        block.instructions.insert(first_idx, fuse_instr);
        
        // 単一のawait式を生成
        let await_instr = Instruction::AwaitExpr {
            future_expr: Expression::Variable(fused_var),
            result_var: await_exprs.last().unwrap().2.clone(),
            span: None,
        };
        
        // 元のawait式を削除し、融合されたawaitを挿入
        let mut offset = 1; // 挿入した融合命令によるオフセット
        for (idx, _, _) in await_exprs {
            block.instructions.remove(idx + offset);
            offset -= 1;
        }
        block.instructions.insert(first_idx + 1, await_instr);
        
        Ok(())
    }
    
    /// 融合されたFuture型を作成
    fn create_fused_future_type(&self, await_exprs: &[(usize, Expression, Variable)]) -> Result<Type, String> {
        // 実際の実装では型システムと連携して新しい型を作成する
        let component_types: Vec<Type> = await_exprs.iter()
            .map(|(_, expr, _)| self.get_expression_type(expr))
            .collect::<Result<Vec<Type>, String>>()?;
        
        Ok(Type::FusedFuture(component_types))
    }
    
    /// 式の型を取得
    fn get_expression_type(&self, expr: &Expression) -> Result<Type, String> {
        // 実際の実装では型推論システムと連携
        match expr {
            Expression::Variable(var) => Ok(var.ty.clone()),
            Expression::Call { function_id, .. } => {
                let module = self.module.as_ref().ok_or("モジュールが設定されていません")?;
                let function = module.functions.get(function_id).ok_or(format!("関数ID {}が見つかりません", function_id))?;
                Ok(function.return_type.clone())
            },
            // 他の式タイプも同様に処理
            _ => Err(format!("未対応の式タイプ: {:?}", expr)),
        }
    }
    
    /// 不要なawaitを除去
    /// すぐに完了するFutureに対するawaitを除去して実行効率を向上させる
    fn elide_awaits(&mut self, func_id: FunctionId) -> Result<(), String> {
        let module = self.module.as_mut().ok_or("モジュールが設定されていません")?;
        let function = module.functions.get_mut(&func_id).ok_or(format!("関数ID {}が見つかりません", func_id))?;
        
        let mut elided_count = 0;
        
        // 各基本ブロックを処理
        for block_id in function.blocks.clone() {
            let block = function.basic_blocks.get_mut(&block_id).ok_or(format!("ブロックID {}が見つかりません", block_id))?;
            
            let mut i = 0;
            while i < block.instructions.len() {
                if let Instruction::AwaitExpr { future_expr, result_var, .. } = &block.instructions[i] {
                    // すぐに完了するFutureかどうかを分析
                    if self.is_ready_future(future_expr, function)? {
                        // awaitを除去して直接値を取得する命令に置き換え
                        let value_expr = self.extract_ready_value(future_expr.clone())?;
                        let assign_instr = Instruction::Assign {
                            target: result_var.clone(),
                            value: value_expr,
                            span: None,
                        };
                        
                        block.instructions[i] = assign_instr;
                        elided_count += 1;
                    }
                }
                i += 1;
            }
        }
        
        // 統計を更新
        self.stats.elided_awaits += elided_count;
        
        Ok(())
    }
    
    /// 式がすぐに完了するFutureを表すかどうかを判定
    fn is_ready_future(&self, expr: &Expression, function: &Function) -> Result<bool, String> {
        match expr {
            Expression::Call { function_id, .. } => {
                // ready()やok()などの即時完了Future生成関数かチェック
                let module = self.module.as_ref().ok_or("モジュールが設定されていません")?;
                let called_func = module.functions.get(function_id).ok_or(format!("関数ID {}が見つかりません", function_id))?;
                
                // 関数名や属性で判断
                Ok(called_func.name.contains("ready") || 
                   called_func.name.contains("ok") || 
                   called_func.attributes.contains("immediate_future"))
            },
            Expression::Constant(_) => {
                // 定数値からのFuture生成は常に即時完了
                Ok(true)
            },
            Expression::Variable(var) => {
                // 変数の定義を追跡して即時完了かどうかを判断
                self.analyze_variable_definition(var, function)
            },
            _ => Ok(false),
        }
    }
    
    /// 変数の定義を分析して即時完了Futureかどうかを判断
    fn analyze_variable_definition(&self, var: &Variable, function: &Function) -> Result<bool, String> {
        // データフロー分析を使用して変数の定義元を追跡
        let mut visited_vars = HashSet::new();
        self.trace_variable_definition(var, function, &mut visited_vars)
    }
    
    /// 変数の定義を再帰的に追跡して即時完了Futureかどうかを判断
    fn trace_variable_definition(&self, var: &Variable, function: &Function, visited: &mut HashSet<VariableId>) -> Result<bool, String> {
        // 循環参照を検出して無限再帰を防止
        if !visited.insert(var.id) {
            return Ok(false);
        }
        
        // 変数の定義を探索
        for block_id in &function.blocks {
            let block = function.basic_blocks.get(block_id)
                .ok_or_else(|| format!("ブロックID {}が見つかりません", block_id))?;
            
            for instr in &block.instructions {
                match instr {
                    Instruction::Assign { target, value, .. } if target.id == var.id => {
                        // 代入の右辺を分析
                        return self.analyze_expression_for_immediate_future(value, function, visited);
                    },
                    Instruction::Call { result_var, function_id, .. } if result_var.id == var.id => {
                        // 関数呼び出しの結果を分析
                        let module = self.module.as_ref().ok_or("モジュールが設定されていません")?;
                        let called_func = module.functions.get(function_id)
                            .ok_or_else(|| format!("関数ID {}が見つかりません", function_id))?;
                        
                        // 関数の属性や名前から即時完了かどうかを判断
                        if called_func.attributes.contains("immediate_future") || 
                           called_func.name.contains("ready") || 
                           called_func.name.contains("ok") {
                            return Ok(true);
                        }
                        
                        // 関数の実装を分析して即時完了かどうかを判断
                        if called_func.is_async && called_func.blocks.len() <= 2 {
                            // 単純な非同期関数の場合、戻り値を分析
                            return self.analyze_function_return(called_func, visited);
                        }
                        
                        return Ok(false);
                    },
                    Instruction::Phi { target, sources, .. } if target.id == var.id => {
                        // Phi命令の場合、すべてのソースが即時完了である場合のみtrueを返す
                        let mut all_immediate = true;
                        
                        for (source_var, _) in sources {
                            let is_immediate = self.trace_variable_definition(source_var, function, visited)?;
                            if !is_immediate {
                                all_immediate = false;
                                break;
                            }
                        }
                        
                        return Ok(all_immediate);
                    },
                    _ => {}
                }
            }
        }
        
        // 変数の定義が見つからない場合は保守的にfalseを返す
        Ok(false)
    }
    
    /// 式が即時完了Futureを生成するかどうかを分析
    fn analyze_expression_for_immediate_future(&self, expr: &Expression, function: &Function, visited: &mut HashSet<VariableId>) -> Result<bool, String> {
        match expr {
            Expression::Call { function_id, .. } => {
                // 関数呼び出しの場合
                let module = self.module.as_ref().ok_or("モジュールが設定されていません")?;
                let called_func = module.functions.get(function_id)
                    .ok_or_else(|| format!("関数ID {}が見つかりません", function_id))?;
                
                Ok(called_func.name.contains("ready") || 
                   called_func.name.contains("ok") || 
                   called_func.attributes.contains("immediate_future"))
            },
            Expression::Constant(_) => {
                // 定数値からのFuture生成は常に即時完了
                Ok(true)
            },
            Expression::Variable(var) => {
                // 変数の定義を追跡
                self.trace_variable_definition(var, function, visited)
            },
            Expression::MethodCall { object, method_name, .. } => {
                // メソッド呼び出しの場合、特定のメソッド名をチェック
                if method_name.contains("ready") || method_name.contains("ok") {
                    return Ok(true);
                }
                
                // オブジェクトが即時完了Futureを返すメソッドかどうかをチェック
                match &**object {
                    Expression::Variable(var) => self.trace_variable_definition(var, function, visited),
                    _ => Ok(false)
                }
            },
            Expression::StructInitializer { struct_type, fields } => {
                // 構造体初期化の場合、Future特性を持つ型かどうかをチェック
                let module = self.module.as_ref().ok_or("モジュールが設定されていません")?;
                
                if let Some(struct_def) = module.type_definitions.get(struct_type) {
                    if struct_def.attributes.contains("immediate_future") {
                        return Ok(true);
                    }
                    
                    // 特定のフィールド（例：_state）の値を確認
                    if let Some((_, state_expr)) = fields.iter().find(|(name, _)| name == "_state" || name == "state") {
                        return self.analyze_expression_for_immediate_future(state_expr, function, visited);
                    }
                }
                
                Ok(false)
            },
            Expression::Cast { expr, target_type } => {
                // キャスト式の場合、元の式と対象の型を分析
                let module = self.module.as_ref().ok_or("モジュールが設定されていません")?;
                
                // 対象の型がFuture特性を持ち、即時完了を示す場合
                if let Some(type_def) = module.type_definitions.get(target_type) {
                    if type_def.attributes.contains("immediate_future") {
                        return Ok(true);
                    }
                }
                
                self.analyze_expression_for_immediate_future(expr, function, visited)
            },
            _ => Ok(false),
        }
    }
    
    /// 関数の戻り値が即時完了Futureかどうかを分析
    fn analyze_function_return(&self, function: &Function, visited: &mut HashSet<VariableId>) -> Result<bool, String> {
        // 関数内のすべての戻り値を分析
        for block_id in &function.blocks {
            let block = function.basic_blocks.get(block_id)
                .ok_or_else(|| format!("ブロックID {}が見つかりません", block_id))?;
            
            for instr in &block.instructions {
                if let Instruction::Return { value, .. } = instr {
                    // 戻り値の式が即時完了でない場合はfalseを返す
                    if !self.analyze_expression_for_immediate_future(value, function, visited)? {
                        return Ok(false);
                    }
                }
            }
        }
        
        // すべての戻り値が即時完了の場合はtrueを返す
        Ok(true)
    }
    
    /// 即時完了Futureから値を抽出する式を生成
    fn extract_ready_value(&self, future_expr: Expression) -> Result<Expression, String> {
        let module = self.module.as_ref().ok_or("モジュールが設定されていません")?;
        
        match future_expr {
            Expression::Call { function_id, arguments, type_arguments, span } => {
                let called_func = module.functions.get(&function_id)
                    .ok_or_else(|| format!("関数ID {}が見つかりません", function_id))?;
                
                if called_func.name.contains("ready") || called_func.name.contains("ok") {
                    // ready(x)のような関数呼び出しから直接xを取り出す
                    if !arguments.is_empty() {
                        Ok(arguments[0].clone())
                    } else {
                        // 引数がない場合はデフォルト値を型に基づいて生成
                        self.create_default_value_for_type(&called_func.return_type)
                    }
                } else if called_func.attributes.contains("immediate_future") {
                    // 即時完了Future生成関数の場合、内部実装を分析して値を抽出
                    self.extract_value_from_immediate_function(called_func, arguments, type_arguments)
                } else {
                    // その他の関数呼び出しの場合、Future::poll_nowメソッドを使用
                    Ok(Expression::MethodCall {
                        object: Box::new(future_expr),
                        method_name: "poll_now".to_string(),
                        arguments: vec![],
                        type_arguments: vec![],
                    })
                }
            },
            Expression::Variable(var) => {
                // 変数の型情報を取得
                let var_type = self.get_variable_type(&var)?;
                
                // 型に基づいて適切な抽出メソッドを選択
                if self.is_option_future_type(&var_type) {
                    // Option<Future<T>>型の場合
                    Ok(Expression::MethodCall {
                        object: Box::new(Expression::Variable(var)),
                        method_name: "unwrap_ready".to_string(),
                        arguments: vec![],
                        type_arguments: vec![],
                    })
                } else if self.is_result_future_type(&var_type) {
                    // Result<Future<T>, E>型の場合
                    Ok(Expression::MethodCall {
                        object: Box::new(Expression::Variable(var)),
                        method_name: "unwrap_ready".to_string(),
                        arguments: vec![],
                        type_arguments: vec![],
                    })
                } else {
                    // 標準的なFuture型の場合
                    Ok(Expression::MethodCall {
                        object: Box::new(Expression::Variable(var)),
                        method_name: "get_ready_value".to_string(),
                        arguments: vec![],
                        type_arguments: vec![],
                    })
                }
            },
            Expression::MethodCall { object, method_name, arguments, type_arguments } => {
                // メソッド呼び出しの場合、メソッド名に基づいて適切な抽出を行う
                if method_name.contains("ready") || method_name.contains("ok") {
                    // ready()メソッドの場合、内部値を直接取得
                    Ok(Expression::MethodCall {
                        object,
                        method_name: "get_ready_value".to_string(),
                        arguments,
                        type_arguments,
                    })
                } else {
                    // その他のメソッド呼び出しの場合、poll_nowメソッドを使用
                    Ok(Expression::MethodCall {
                        object: Box::new(future_expr),
                        method_name: "poll_now".to_string(),
                        arguments: vec![],
                        type_arguments: vec![],
                    })
                }
            },
            Expression::StructInitializer { struct_type, fields } => {
                // 構造体初期化の場合、特定のフィールドから値を抽出
                if let Some((_, value_expr)) = fields.iter().find(|(name, _)| name == "value" || name == "_value") {
                    Ok(value_expr.clone())
                } else {
                    // 値フィールドが見つからない場合、型に基づいてデフォルト抽出メソッドを使用
                    Ok(Expression::MethodCall {
                        object: Box::new(future_expr),
                        method_name: "get_ready_value".to_string(),
                        arguments: vec![],
                        type_arguments: vec![],
                    })
                }
            },
            _ => Err(format!("未対応の即時完了Future式: {:?}", future_expr)),
        }
    }
    
    /// 変数の型情報を取得
    /// 
    /// 変数のシンボル情報から型IDを取得し、必要に応じて型推論を行います。
    /// 型情報が不明確な場合は、コンテキストや使用パターンから最適な型を推測します。
    /// また、依存型の場合は値に基づいた具体的な型を計算します。
    fn get_variable_type(&self, var: &Variable) -> Result<TypeId, String> {
        let module = self.module.as_ref().ok_or("モジュールが設定されていません")?;
        
        // シンボルテーブルから変数情報を検索
        if let Some(symbol) = module.symbol_table.get(&var.name) {
            // 明示的な型情報がある場合はそれを返す
            if let Some(explicit_type) = &symbol.explicit_type {
                return Ok(explicit_type.clone());
            }
            
            // 型推論が必要な場合
            if let Some(inferred_type) = &symbol.inferred_type {
                // 依存型の場合、値に基づいて具体的な型を計算
                if self.is_dependent_type(inferred_type) {
                    return self.calculate_dependent_type(inferred_type, var);
                }
                
                // ジェネリック型の場合、コンテキストから具体的な型を推論
                if self.is_generic_type(inferred_type) {
                    if let Some(concrete_type) = self.infer_concrete_type_from_context(inferred_type, var) {
                        return Ok(concrete_type);
                    }
                }
                
                return Ok(inferred_type.clone());
            }
            
            // 使用パターンから型を推測
            if let Some(usage_type) = self.infer_type_from_usage_pattern(var) {
                return Ok(usage_type);
            }
        }
        
        // スコープチェーン全体を探索
        for scope in &module.scope_chain {
            if let Some(var_info) = scope.variables.get(&var.name) {
                return Ok(var_info.type_id.clone());
            }
        }
        
        // 型情報が見つからない場合、変数自体の型情報を使用（フォールバック）
        if var.type_id != TypeId::default() {
            return Ok(var.type_id.clone());
        }
        
        // 最終的に型情報が得られない場合はエラー
        Err(format!("変数 '{}' の型情報を取得できませんでした", var.name))
    }
    
    /// 依存型かどうかを判定
    fn is_dependent_type(&self, type_id: &TypeId) -> bool {
        let module = self.module.as_ref().unwrap();
        
        if let Some(type_def) = module.type_definitions.get(type_id) {
            return type_def.kind == "dependent" || type_def.attributes.contains(&"dependent".to_string());
        }
        
        false
    }
    
    /// 依存型の具体的な型を計算
    fn calculate_dependent_type(&self, base_type: &TypeId, var: &Variable) -> Result<TypeId, String> {
        let module = self.module.as_ref().unwrap();
        
        // 依存型の定義を取得
        let type_def = module.type_definitions.get(base_type)
            .ok_or(format!("依存型 {:?} の定義が見つかりません", base_type))?;
        
        // 依存型の計算式を評価
        if let Some(expr) = &type_def.dependent_expr {
            // 変数の値を取得
            let var_value = self.get_variable_value(var)?;
            
            // 計算環境を構築
            let mut env = HashMap::new();
            env.insert("value".to_string(), var_value);
            
            // 依存型の計算式を評価して具体的な型を取得
            let concrete_type = self.evaluate_type_expression(expr, &env)?;
            return Ok(concrete_type);
        }
        
        // 計算式がない場合はベース型をそのまま返す
        Ok(base_type.clone())
    }
    
    /// 変数の現在値を取得（依存型計算用）
    fn get_variable_value(&self, var: &Variable) -> Result<Value, String> {
        let module = self.module.as_ref().unwrap();
        
        // シンボルテーブルから変数の値情報を検索
        if let Some(symbol) = module.symbol_table.get(&var.name) {
            if let Some(value) = &symbol.current_value {
                return Ok(value.clone());
            }
        }
        
        // 値が見つからない場合はデフォルト値を返す
        Ok(Value::Undefined)
    }
    
    /// 型式を評価して具体的な型を取得
    fn evaluate_type_expression(&self, expr: &TypeExpression, env: &HashMap<String, Value>) -> Result<TypeId, String> {
        match expr {
            TypeExpression::Literal(type_id) => Ok(type_id.clone()),
            TypeExpression::Conditional { condition, then_type, else_type } => {
                let cond_result = self.evaluate_condition(condition, env)?;
                if cond_result {
                    self.evaluate_type_expression(then_type, env)
                } else {
                    self.evaluate_type_expression(else_type, env)
                }
            },
            TypeExpression::Function { params, body } => {
                // 型レベル関数の評価
                let mut new_env = env.clone();
                for param in params {
                    if let Some(value) = env.get(&param.name) {
                        new_env.insert(param.name.clone(), value.clone());
                    }
                }
                self.evaluate_type_expression(body, &new_env)
            },
            // その他の型式の評価ロジック
            _ => Err(format!("未対応の型式: {:?}", expr)),
        }
    }
    
    /// 条件式を評価
    fn evaluate_condition(&self, condition: &Condition, env: &HashMap<String, Value>) -> Result<bool, String> {
        match condition {
            Condition::Equal(left, right) => {
                let left_val = self.evaluate_value_expression(left, env)?;
                let right_val = self.evaluate_value_expression(right, env)?;
                Ok(left_val == right_val)
            },
            Condition::GreaterThan(left, right) => {
                let left_val = self.evaluate_value_expression(left, env)?;
                let right_val = self.evaluate_value_expression(right, env)?;
                match (left_val, right_val) {
                    (Value::Integer(l), Value::Integer(r)) => Ok(l > r),
                    (Value::Float(l), Value::Float(r)) => Ok(l > r),
                    _ => Err("比較演算子は数値型にのみ適用できます".to_string()),
                }
            },
            // その他の条件式の評価ロジック
            _ => Err(format!("未対応の条件式: {:?}", condition)),
        }
    }
    
    /// 値式を評価
    fn evaluate_value_expression(&self, expr: &ValueExpression, env: &HashMap<String, Value>) -> Result<Value, String> {
        match expr {
            ValueExpression::Literal(value) => Ok(value.clone()),
            ValueExpression::Variable(name) => {
                env.get(name).cloned().ok_or(format!("変数 {} が環境内に見つかりません", name))
            },
            ValueExpression::BinaryOp { op, left, right } => {
                let left_val = self.evaluate_value_expression(left, env)?;
                let right_val = self.evaluate_value_expression(right, env)?;
                self.apply_binary_op(op, &left_val, &right_val)
            },
            // その他の値式の評価ロジック
            _ => Err(format!("未対応の値式: {:?}", expr)),
        }
    }
    
    /// 二項演算を適用
    fn apply_binary_op(&self, op: &BinaryOperator, left: &Value, right: &Value) -> Result<Value, String> {
        match op {
            BinaryOperator::Add => match (left, right) {
                (Value::Integer(l), Value::Integer(r)) => Ok(Value::Integer(l + r)),
                (Value::Float(l), Value::Float(r)) => Ok(Value::Float(l + r)),
                (Value::String(l), Value::String(r)) => Ok(Value::String(l.clone() + r)),
                _ => Err("加算演算子は互換性のある型にのみ適用できます".to_string()),
            },
            // その他の演算子の実装
            _ => Err(format!("未対応の演算子: {:?}", op)),
        }
    }
    
    /// ジェネリック型かどうかを判定
    fn is_generic_type(&self, type_id: &TypeId) -> bool {
        let module = self.module.as_ref().unwrap();
        
        if let Some(type_def) = module.type_definitions.get(type_id) {
            return !type_def.type_parameters.is_empty();
        }
        
        false
    }
    
    /// コンテキストから具体的な型を推論
    fn infer_concrete_type_from_context(&self, generic_type: &TypeId, var: &Variable) -> Option<TypeId> {
        let module = self.module.as_ref().unwrap();
        
        // 変数の使用箇所を分析
        for usage in module.variable_usages.get(&var.name)? {
            if let Some(context_type) = &usage.context_type {
                // ジェネリック型と互換性があるか確認
                if self.is_compatible_with_generic(context_type, generic_type) {
                    return Some(context_type.clone());
                }
            }
        }
        
        // 変数の代入元を分析
        if let Some(assignments) = module.variable_assignments.get(&var.name) {
            for assignment in assignments {
                if let Some(expr_type) = &assignment.expression_type {
                    // ジェネリック型と互換性があるか確認
                    if self.is_compatible_with_generic(expr_type, generic_type) {
                        return Some(expr_type.clone());
                    }
                }
            }
        }
        
        None
    }
    
    /// 型がジェネリック型と互換性があるかを確認
    fn is_compatible_with_generic(&self, concrete_type: &TypeId, generic_type: &TypeId) -> bool {
        let module = self.module.as_ref().unwrap();
        
        if let (Some(concrete_def), Some(generic_def)) = (
            module.type_definitions.get(concrete_type),
            module.type_definitions.get(generic_type)
        ) {
            // 基本的な互換性チェック
            if concrete_def.base_type == generic_def.base_type {
                return true;
            }
            
            // 型パラメータの互換性チェック
            if concrete_def.type_parameters.len() == generic_def.type_parameters.len() {
                return concrete_def.type_parameters.iter().zip(&generic_def.type_parameters)
                    .all(|(c, g)| self.is_compatible_type(c, g));
            }
        }
        
        false
    }
    
    /// 2つの型が互換性があるかを確認
    fn is_compatible_type(&self, type1: &TypeId, type2: &TypeId) -> bool {
        let module = self.module.as_ref().unwrap();
        
        // 同一の型IDなら互換性あり
        if type1 == type2 {
            return true;
        }
        
        // 型の定義を取得
        if let (Some(def1), Some(def2)) = (
            module.type_definitions.get(type1),
            module.type_definitions.get(type2)
        ) {
            // 型の名前が同じなら互換性あり
            if def1.name == def2.name {
                return true;
            }
            
            // サブタイプ関係をチェック
            if def1.subtypes.contains(type2) || def2.supertypes.contains(type1) {
                return true;
            }
            
            // トレイト実装をチェック
            for trait_id in &def1.traits {
                if def2.traits.contains(trait_id) {
                    return true;
                }
            }
        }
        
        false
    }
    
    /// 使用パターンから型を推測
    fn infer_type_from_usage_pattern(&self, var: &Variable) -> Option<TypeId> {
        let module = self.module.as_ref().unwrap();
        
        // 変数の使用パターンを分析
        let mut usage_patterns = HashMap::new();
        
        if let Some(usages) = module.variable_usages.get(&var.name) {
            for usage in usages {
                if let Some(pattern) = &usage.usage_pattern {
                    let count = usage_patterns.entry(pattern.clone()).or_insert(0);
                    *count += 1;
                }
            }
        }
        
        // 最も頻繁な使用パターンを特定
        if let Some((most_common_pattern, _)) = usage_patterns.iter()
            .max_by_key(|(_, &count)| count) {
            
            // パターンから型を推測
            return self.type_from_usage_pattern(most_common_pattern);
        }
        
        None
    }
    
    /// 使用パターンから型を取得
    fn type_from_usage_pattern(&self, pattern: &UsagePattern) -> Option<TypeId> {
        let module = self.module.as_ref().unwrap();
        
        match pattern {
            UsagePattern::MethodCall(method_name) => {
                // メソッド名から型を推測
                for (type_id, type_def) in &module.type_definitions {
                    if type_def.methods.iter().any(|m| &m.name == method_name) {
                        return Some(type_id.clone());
                    }
                }
            },
            UsagePattern::FieldAccess(field_name) => {
                // フィールド名から型を推測
                for (type_id, type_def) in &module.type_definitions {
                    if type_def.fields.iter().any(|f| &f.name == field_name) {
                        return Some(type_id.clone());
                    }
                }
            },
            UsagePattern::BinaryOperation(op) => {
                // 演算子から型を推測
                match op.as_str() {
                    "+" | "-" | "*" | "/" => return Some(module.get_numeric_type_id()),
                    "==" | "!=" | "<" | ">" => return Some(module.get_comparable_type_id()),
                    "&&" | "||" => return Some(module.get_boolean_type_id()),
                    _ => {}
                }
            },
            // その他のパターンの処理
            _ => {}
        }
        
        None
    }
    /// 型がOption<Future<T>>かどうかを判定
    fn is_option_future_type(&self, type_id: &TypeId) -> bool {
        let module = self.module.as_ref().unwrap();
        
        if let Some(type_def) = module.type_definitions.get(type_id) {
            if type_def.name.contains("Option") {
                // 型パラメータがFutureかどうかをチェック
                if let Some(param_type) = type_def.type_parameters.first() {
                    if let Some(inner_type) = module.type_definitions.get(param_type) {
                        return inner_type.traits.contains(&"Future".to_string());
                    }
                }
            }
        }
        
        false
    }
    
    /// 型がResult<Future<T>, E>かどうかを判定
    fn is_result_future_type(&self, type_id: &TypeId) -> bool {
        let module = self.module.as_ref().unwrap();
        
        if let Some(type_def) = module.type_definitions.get(type_id) {
            if type_def.name.contains("Result") {
                // 型パラメータがFutureかどうかをチェック
                if let Some(param_type) = type_def.type_parameters.first() {
                    if let Some(inner_type) = module.type_definitions.get(param_type) {
                        return inner_type.traits.contains(&"Future".to_string());
                    }
                }
            }
        }
        
        false
    }
    
    /// 型に基づいてデフォルト値を生成
    fn create_default_value_for_type(&self, type_id: &TypeId) -> Result<Expression, String> {
        let module = self.module.as_ref().ok_or("モジュールが設定されていません")?;
        
        if let Some(type_def) = module.type_definitions.get(type_id) {
            match type_def.kind.as_str() {
                "i32" => Ok(Expression::Constant(Constant::Integer(0))),
                "f64" => Ok(Expression::Constant(Constant::Float(0.0))),
                "bool" => Ok(Expression::Constant(Constant::Boolean(false))),
                "string" => Ok(Expression::Constant(Constant::String("".to_string()))),
                "unit" => Ok(Expression::Constant(Constant::Unit)),
                _ => {
                    // 構造体や列挙型の場合、デフォルトコンストラクタを呼び出す
                    Ok(Expression::Call {
                        function_id: FunctionId::new_with_name(&format!("{}_default", type_def.name)),
                        arguments: vec![],
                        type_arguments: vec![],
                        span: None,
                    })
                }
            }
        } else {
            // 型情報が見つからない場合はUnitを返す
            Ok(Expression::Constant(Constant::Unit))
        }
    }
    
    /// 即時完了Future生成関数から値を抽出
    fn extract_value_from_immediate_function(&self, function: &Function, arguments: Vec<Expression>, type_arguments: Vec<TypeId>) -> Result<Expression, String> {
        // 関数の実装を分析して、どの引数が結果値になるかを特定
        if function.name.contains("ok") || function.name.contains("success") {
            // ok(value)やsuccess(value)関数の場合、最初の引数が結果値
            if !arguments.is_empty() {
                return Ok(arguments[0].clone());
            }
        } else if function.name.contains("err") || function.name.contains("failure") {
            // エラー結果を生成する関数の場合、エラー処理を挿入
            return Ok(Expression::Call {
                function_id: FunctionId::new_with_name("handle_immediate_error"),
                arguments: if !arguments.is_empty() { vec![arguments[0].clone()] } else { vec![] },
                type_arguments,
                span: None,
            });
        }
        
        // 関数の実装から戻り値の構造を分析
        for block_id in &function.blocks {
            if let Some(block) = function.basic_blocks.get(block_id) {
                for instr in &block.instructions {
                    if let Instruction::Return { value, .. } = instr {
                        // 戻り値の構造を分析して値を抽出
                        return self.extract_value_from_return_expr(value, &arguments);
                    }
                }
            }
        }
        
        // 分析できない場合はデフォルト値を返す
        self.create_default_value_for_type(&function.return_type)
    }
    
    /// 戻り値式から実際の値を抽出
    fn extract_value_from_return_expr(&self, expr: &Expression, original_args: &[Expression]) -> Result<Expression, String> {
        match expr {
            Expression::StructInitializer { fields, .. } => {
                // 構造体初期化の場合、valueフィールドを探す
                if let Some((_, value_expr)) = fields.iter().find(|(name, _)| name == "value" || name == "_value") {
                    return Ok(value_expr.clone());
                }
            },
            Expression::Variable(var) => {
                // 変数が引数の一つと一致する場合、その引数を返す
                for (i, arg_var) in function.parameters.iter().enumerate() {
                    if var.id == arg_var.id && i < original_args.len() {
                        return Ok(original_args[i].clone());
                    }
                }
            },
            _ => {}
        }
        
        // 抽出できない場合はデフォルトのメソッド呼び出しを生成
        Ok(Expression::MethodCall {
            object: Box::new(expr.clone()),
            method_name: "get_ready_value".to_string(),
            arguments: vec![],
            type_arguments: vec![],
        })
    }
    /// 非同期関数をインライン化
    /// 小さな非同期関数を呼び出し元にインライン化して実行効率を向上させる
    fn inline_async_functions(&mut self, func_id: FunctionId) -> Result<(), String> {
        let module = self.module.as_mut().ok_or("モジュールが設定されていません")?;
        let function = module.functions.get(&func_id).ok_or(format!("関数ID {}が見つかりません", func_id))?;
        
        // インライン化候補の非同期関数呼び出しを収集
        let mut inline_candidates = Vec::new();
        
        for block_id in &function.blocks {
            let block = function.basic_blocks.get(block_id).ok_or(format!("ブロックID {}が見つかりません", block_id))?;
            
            for (instr_idx, instr) in block.instructions.iter().enumerate() {
                if let Instruction::Call { function_id: called_func_id, arguments, result_var, .. } = instr {
                    // 呼び出される関数が非同期かつインライン化可能かチェック
                    if let Some(called_func) = module.functions.get(called_func_id) {
                        if called_func.is_async && self.is_inlinable(called_func) {
                            inline_candidates.push((*block_id, instr_idx, *called_func_id, arguments.clone(), result_var.clone()));
                        }
                    }
                }
            }
        }
        
        // 収集した候補を逆順にインライン化（インデックスの変化を避けるため）
        let mut inlined_count = 0;
        inline_candidates.sort_by(|a, b| b.1.cmp(&a.1));
        
        for (block_id, instr_idx, called_func_id, arguments, result_var) in inline_candidates {
            if self.inline_function_at(func_id, block_id, instr_idx, called_func_id, &arguments, &result_var)? {
                inlined_count += 1;
            }
        }
        
        // 統計を更新
        self.stats.inlined_async_functions += inlined_count;
        
        Ok(())
    }
    
    /// 関数がインライン化可能かどうかを判定
    fn is_inlinable(&self, function: &Function) -> bool {
        // インライン化の判断基準:
        // 1. 関数が小さい（命令数が閾値以下）
        // 2. 再帰呼び出しがない
        // 3. 複雑な制御フローがない（単一の戻り値パス）
        
        let instruction_count: usize = function.basic_blocks.values()
            .map(|block| block.instructions.len())
            .sum();
            
        // 小さな関数のみインライン化
        const INLINE_THRESHOLD: usize = 20;
        if instruction_count > INLINE_THRESHOLD {
            return false;
        }
        
        // 再帰チェック（簡易版）
        for block in function.basic_blocks.values() {
            for instr in &block.instructions {
                if let Instruction::Call { function_id, .. } = instr {
                    if *function_id == function.id {
                        return false; // 再帰呼び出しがある
                    }
                }
            }
        }
        
        // 複雑な制御フローのチェック（簡易版）
        let return_blocks_count = function.basic_blocks.values()
            .filter(|block| {
                block.instructions.iter().any(|instr| {
                    matches!(instr, Instruction::Return { .. })
                })
            })
            .count();
            
        return_blocks_count <= 1
    }
    
    /// 指定位置に関数をインライン化
    fn inline_function_at(
        &mut self,
        target_func_id: FunctionId,
        block_id: BlockId,
        instr_idx: usize,
        inline_func_id: FunctionId,
        arguments: &[Expression],
        result_var: &Variable
    ) -> Result<bool, String> {
        let module = self.module.as_mut().ok_or("モジュールが設定されていません")?;
        
        // インライン化する関数のクローンを作成
        let inline_func = module.functions.get(&inline_func_id)
            .ok_or(format!("インライン化する関数ID {}が見つかりません", inline_func_id))?
            .clone();
            
        let target_func = module.functions.get_mut(&target_func_id)
            .ok_or(format!("ターゲット関数ID {}が見つかりません", target_func_id))?;
            
        // 呼び出し命令を取得
        let block = target_func.basic_blocks.get_mut(&block_id)
            .ok_or(format!("ブロックID {}が見つかりません", block_id))?;
            
        // 引数の束縛命令を生成
        let mut inline_instructions = Vec::new();
        
        // パラメータと引数のマッピング
        for (param, arg) in inline_func.parameters.iter().zip(arguments.iter()) {
            let param_var = Variable {
                id: target_func.next_variable_id(),
                name: format!("inline_{}", param.name),
                ty: param.ty.clone(),
            };
            
            inline_instructions.push(Instruction::Assign {
                target: param_var,
                value: arg.clone(),
                span: None,
            });
        }
        
        // インライン関数の本体をコピー（変数IDとブロックIDをリマップ）
        let var_map = self.create_variable_mapping(&inline_func, target_func);
        let (block_map, new_blocks) = self.create_block_mapping(&inline_func, target_func, &var_map);
        
        // 新しいブロックを追加
        for (new_id, new_block) in new_blocks {
            target_func.basic_blocks.insert(new_id, new_block);
            target_func.blocks.push(new_id);
        }
        
        // 元の呼び出し命令を削除し、インライン化したコードを挿入
        block.instructions.remove(instr_idx);
        
        // インライン化した命令を挿入
        for instr in inline_instructions {
            block.instructions.insert(instr_idx, instr);
        }
        
        // 制御フローを接続
        self.connect_inlined_control_flow(target_func, block_id, instr_idx, &block_map, result_var)?;
        
        Ok(true)
    }
    
    /// インライン化のための変数マッピングを作成
    fn create_variable_mapping(&self, inline_func: &Function, target_func: &mut Function) -> HashMap<VariableId, VariableId> {
        let mut var_map = HashMap::new();
        
        // 各変数に新しいIDを割り当て
        for block in inline_func.basic_blocks.values() {
            for instr in &block.instructions {
                self.collect_variables_from_instruction(instr, |var_id| {
                    if !var_map.contains_key(&var_id) {
                        let new_id = target_func.next_variable_id();
                        var_map.insert(var_id, new_id);
                    }
                });
            }
        }
        
        var_map
    }
    
    /// 命令から変数を収集
    fn collect_variables_from_instruction<F>(&self, instr: &Instruction, mut callback: F)
    where
        F: FnMut(VariableId)
    {
        match instr {
            Instruction::Assign { target, value, .. } => {
                callback(target.id);
                self.collect_variables_from_expression(value, &mut callback);
            },
            Instruction::Call { arguments, result_var, .. } => {
                if let Some(var) = result_var {
                    callback(var.id);
                }
                for arg in arguments {
                    self.collect_variables_from_expression(arg, &mut callback);
                }
            },
            // 他の命令タイプも同様に処理
            _ => {},
        }
    }
    
    /// 式から変数を収集
    fn collect_variables_from_expression<F>(&self, expr: &Expression, callback: &mut F)
    where
        F: FnMut(VariableId)
    {
        match expr {
            Expression::Variable(var) => {
                callback(var.id);
            },
            Expression::BinaryOp { left, right, .. } => {
                self.collect_variables_from_expression(left, callback);
                self.collect_variables_from_expression(right, callback);
            },
            // 他の式タイプも同様に処理
            _ => {},
        }
    }
    
    /// インライン化のためのブロックマッピングを作成
    fn create_block_mapping(
        &self,
        inline_func: &Function,
        target_func: &mut Function,
        var_map: &HashMap<VariableId, VariableId>
    ) -> (HashMap<BlockId, BlockId>, HashMap<BlockId, BasicBlock>) {
        let mut block_map = HashMap::new();
        let mut new_blocks = HashMap::new();
        
        // 各ブロックに新しいIDを割り当て
        for (&old_id, block) in &inline_func.basic_blocks {
            let new_id = target_func.next_block_id();
            block_map.insert(old_id, new_id);
            
            // ブロックの命令をリマップ
            let mut new_instructions = Vec::new();
            for instr in &block.instructions {
                let remapped_instr = self.remap_instruction(instr, var_map, &block_map);
                new_instructions.push(remapped_instr);
            }
            
            // 新しいブロックを作成
            let new_block = BasicBlock {
                id: new_id,
                instructions: new_instructions,
                predecessors: Vec::new(), // 後で更新
                successors: Vec::new(),   // 後で更新
                exception_handler: None,  // 例外ハンドラは現在サポート外
            };
            
            new_blocks.insert(new_id, new_block);
        }
        
        (block_map, new_blocks)
    }
    
    /// 命令内の変数IDとブロックIDをリマップ
    /// 
    /// インライン化の過程で、元の関数内の命令を呼び出し先関数のコンテキストに適応させるために
    /// 変数IDとブロックIDを適切にリマップします。これにより名前衝突を防ぎ、正しい制御フローを維持します。
    /// 
    /// 各命令タイプに対して、含まれる変数参照とブロック参照を新しいコンテキストに合わせて変換します。
    /// また、非同期関連の命令に対しては特別な処理を行い、正しい非同期制御フローを維持します。
    fn remap_instruction(
        &self,
        instr: &Instruction,
        var_map: &HashMap<VariableId, VariableId>,
        block_map: &HashMap<BlockId, BlockId>
    ) -> Instruction {
        match instr {
            Instruction::Assign { target, value, span } => {
                let new_target = self.remap_variable(target, var_map);
                let new_value = self.remap_expression(value, var_map);
                Instruction::Assign {
                    target: new_target,
                    value: new_value,
                    span: *span,
                }
            },
            Instruction::Call { target, function, arguments, span } => {
                let new_target = target.as_ref().map(|var| self.remap_variable(var, var_map));
                let new_arguments = arguments.iter()
                    .map(|arg| self.remap_expression(arg, var_map))
                    .collect();
                
                Instruction::Call {
                    target: new_target,
                    function: function.clone(), // 関数参照はそのまま
                    arguments: new_arguments,
                    span: *span,
                }
            },
            Instruction::Return { value, span } => {
                let new_value = value.as_ref().map(|expr| self.remap_expression(expr, var_map));
                Instruction::Return {
                    value: new_value,
                    span: *span,
                }
            },
            Instruction::Jump { target } => {
                let new_target = block_map.get(target).cloned().unwrap_or(*target);
                Instruction::Jump {
                    target: new_target,
                }
            },
            Instruction::Branch { condition, true_target, false_target, span } => {
                let new_condition = self.remap_expression(condition, var_map);
                let new_true_target = block_map.get(true_target).cloned().unwrap_or(*true_target);
                let new_false_target = block_map.get(false_target).cloned().unwrap_or(*false_target);
                
                Instruction::Branch {
                    condition: new_condition,
                    true_target: new_true_target,
                    false_target: new_false_target,
                    span: *span,
                }
            },
            Instruction::Phi { target, sources, span } => {
                let new_target = self.remap_variable(target, var_map);
                let new_sources = sources.iter()
                    .map(|(block_id, expr)| {
                        let new_block_id = block_map.get(block_id).cloned().unwrap_or(*block_id);
                        let new_expr = self.remap_expression(expr, var_map);
                        (new_block_id, new_expr)
                    })
                    .collect();
                
                Instruction::Phi {
                    target: new_target,
                    sources: new_sources,
                    span: *span,
                }
            },
            Instruction::Await { target, future, span } => {
                let new_target = target.as_ref().map(|var| self.remap_variable(var, var_map));
                let new_future = self.remap_expression(future, var_map);
                
                Instruction::Await {
                    target: new_target,
                    future: new_future,
                    span: *span,
                }
            },
            Instruction::Spawn { target, function, arguments, span } => {
                let new_target = self.remap_variable(target, var_map);
                let new_arguments = arguments.iter()
                    .map(|arg| self.remap_expression(arg, var_map))
                    .collect();
                
                Instruction::Spawn {
                    target: new_target,
                    function: function.clone(),
                    arguments: new_arguments,
                    span: *span,
                }
            },
            Instruction::Yield { value, span } => {
                let new_value = value.as_ref().map(|expr| self.remap_expression(expr, var_map));
                
                Instruction::Yield {
                    value: new_value,
                    span: *span,
                }
            },
            Instruction::Resume { target, coroutine, argument, span } => {
                let new_target = target.as_ref().map(|var| self.remap_variable(var, var_map));
                let new_coroutine = self.remap_expression(coroutine, var_map);
                let new_argument = argument.as_ref().map(|expr| self.remap_expression(expr, var_map));
                
                Instruction::Resume {
                    target: new_target,
                    coroutine: new_coroutine,
                    argument: new_argument,
                    span: *span,
                }
            },
            Instruction::Alloc { target, ty, initializer, span } => {
                let new_target = self.remap_variable(target, var_map);
                let new_initializer = initializer.as_ref().map(|expr| self.remap_expression(expr, var_map));
                
                Instruction::Alloc {
                    target: new_target,
                    ty: ty.clone(),
                    initializer: new_initializer,
                    span: *span,
                }
            },
            Instruction::Load { target, source, span } => {
                let new_target = self.remap_variable(target, var_map);
                let new_source = self.remap_expression(source, var_map);
                
                Instruction::Load {
                    target: new_target,
                    source: new_source,
                    span: *span,
                }
            },
            Instruction::Store { target, value, span } => {
                let new_target = self.remap_expression(target, var_map);
                let new_value = self.remap_expression(value, var_map);
                
                Instruction::Store {
                    target: new_target,
                    value: new_value,
                    span: *span,
                }
            },
            Instruction::GetField { target, source, field, span } => {
                let new_target = self.remap_variable(target, var_map);
                let new_source = self.remap_expression(source, var_map);
                
                Instruction::GetField {
                    target: new_target,
                    source: new_source,
                    field: field.clone(),
                    span: *span,
                }
            },
            Instruction::SetField { target, field, value, span } => {
                let new_target = self.remap_expression(target, var_map);
                let new_value = self.remap_expression(value, var_map);
                
                Instruction::SetField {
                    target: new_target,
                    field: field.clone(),
                    value: new_value,
                    span: *span,
                }
            },
            Instruction::GetIndex { target, source, index, span } => {
                let new_target = self.remap_variable(target, var_map);
                let new_source = self.remap_expression(source, var_map);
                let new_index = self.remap_expression(index, var_map);
                
                Instruction::GetIndex {
                    target: new_target,
                    source: new_source,
                    index: new_index,
                    span: *span,
                }
            },
            Instruction::SetIndex { target, index, value, span } => {
                let new_target = self.remap_expression(target, var_map);
                let new_index = self.remap_expression(index, var_map);
                let new_value = self.remap_expression(value, var_map);
                
                Instruction::SetIndex {
                    target: new_target,
                    index: new_index,
                    value: new_value,
                    span: *span,
                }
            },
            Instruction::Cast { target, source, ty, span } => {
                let new_target = self.remap_variable(target, var_map);
                let new_source = self.remap_expression(source, var_map);
                
                Instruction::Cast {
                    target: new_target,
                    source: new_source,
                    ty: ty.clone(),
                    span: *span,
                }
            },
            Instruction::Unreachable { span } => {
                Instruction::Unreachable { span: *span }
            },
        }
    }
    /// 変数をリマップ
    fn remap_variable(&self, var: &Variable, var_map: &HashMap<VariableId, VariableId>) -> Variable {
        let new_id = var_map.get(&var.id).cloned().unwrap_or(var.id);
        Variable {
            id: new_id,
            name: format!("inline_{}", var.name),
            ty: var.ty.clone(),
        }
    }
    
    /// 式をリマップ
    fn remap_expression(&self, expr: &Expression, var_map: &HashMap<VariableId, VariableId>) -> Expression {
        match expr {
            Expression::Variable(var) => {
                Expression::Variable(self.remap_variable(var, var_map))
            },
            Expression::BinaryOp { op, left, right } => {
                Expression::BinaryOp {
                    op: *op,
                    left: Box::new(self.remap_expression(left, var_map)),
                    right: Box::new(self.remap_expression(right, var_map)),
                }
            },
            Expression::UnaryOp { op, operand } => {
                Expression::UnaryOp {
                    op: *op,
                    operand: Box::new(self.remap_expression(operand, var_map)),
                }
            },
            Expression::Literal(lit) => Expression::Literal(lit.clone()),
            Expression::Call { func, args, span } => {
                let new_args = args.iter()
                    .map(|arg| self.remap_expression(arg, var_map))
                    .collect();
                
                Expression::Call {
                    func: Box::new(self.remap_expression(func, var_map)),
                    args: new_args,
                    span: *span,
                }
            },
            Expression::MethodCall { object, method, args, span } => {
                let new_args = args.iter()
                    .map(|arg| self.remap_expression(arg, var_map))
                    .collect();
                
                Expression::MethodCall {
                    object: Box::new(self.remap_expression(object, var_map)),
                    method: method.clone(),
                    args: new_args,
                    span: *span,
                }
            },
            Expression::FieldAccess { object, field, span } => {
                Expression::FieldAccess {
                    object: Box::new(self.remap_expression(object, var_map)),
                    field: field.clone(),
                    span: *span,
                }
            },
            Expression::ArrayLiteral { elements, span } => {
                let new_elements = elements.iter()
                    .map(|elem| self.remap_expression(elem, var_map))
                    .collect();
                
                Expression::ArrayLiteral {
                    elements: new_elements,
                    span: *span,
                }
            },
            Expression::StructLiteral { ty, fields, span } => {
                let new_fields = fields.iter()
                    .map(|(name, value)| (name.clone(), self.remap_expression(value, var_map)))
                    .collect();
                
                Expression::StructLiteral {
                    ty: ty.clone(),
                    fields: new_fields,
                    span: *span,
                }
            },
            Expression::Closure { params, body, captures, span } => {
                // キャプチャ変数をリマップ
                let new_captures = captures.iter()
                    .map(|var| self.remap_variable(var, var_map))
                    .collect();
                
                // クロージャのボディ内の変数をリマップするための新しい変数マップを作成
                let mut inner_var_map = var_map.clone();
                
                // パラメータをマップに追加（パラメータはリマップしない）
                for param in params.iter() {
                    inner_var_map.insert(param.name.clone(), param.name.clone());
                }
                
                // ボディの種類に応じて適切にリマップ
                let new_body = match body.as_ref() {
                    Body::Block(block) => {
                        // ブロック内の各ステートメントをリマップ
                        let new_statements = block.statements.iter()
                            .map(|stmt| self.remap_statement(stmt, &mut inner_var_map))
                            .collect();
                        
                        Body::Block(Block {
                            statements: new_statements,
                            span: block.span,
                        })
                    },
                    Body::Expression(expr) => {
                        // 式をリマップ
                        Body::Expression(self.remap_expression(expr, &inner_var_map))
                    },
                    Body::AsyncBlock(async_block) => {
                        // 非同期ブロック内の各ステートメントをリマップ
                        let new_statements = async_block.statements.iter()
                            .map(|stmt| self.remap_statement(stmt, &mut inner_var_map))
                            .collect();
                        
                        Body::AsyncBlock(AsyncBlock {
                            statements: new_statements,
                            span: async_block.span,
                            state_machine: async_block.state_machine.clone(), // 状態マシンは後で変換される
                        })
                    },
                };
                
                // 環境キャプチャの最適化: 実際に使用されている変数のみをキャプチャ
                let used_captures = self.analyze_used_variables(&new_body);
                let optimized_captures = new_captures.into_iter()
                    .filter(|var| used_captures.contains(&var.name))
                    .collect();
                
                Expression::Closure {
                    params: params.clone(),
                    body: Box::new(new_body),
                    captures: optimized_captures,
                    span: *span,
                }
            },
            Expression::Await { expr, span } => {
                Expression::Await {
                    expr: Box::new(self.remap_expression(expr, var_map)),
                    span: *span,
                }
            },
            Expression::Cast { expr, ty, span } => {
                Expression::Cast {
                    expr: Box::new(self.remap_expression(expr, var_map)),
                    ty: ty.clone(),
                    span: *span,
                }
            },
            Expression::Index { array, index, span } => {
                Expression::Index {
                    array: Box::new(self.remap_expression(array, var_map)),
                    index: Box::new(self.remap_expression(index, var_map)),
                    span: *span,
                }
            },
            Expression::Range { start, end, step, span } => {
                Expression::Range {
                    start: start.as_ref().map(|e| Box::new(self.remap_expression(e, var_map))),
                    end: end.as_ref().map(|e| Box::new(self.remap_expression(e, var_map))),
                    step: step.as_ref().map(|e| Box::new(self.remap_expression(e, var_map))),
                    span: *span,
                }
            },
            Expression::Tuple { elements, span } => {
                let new_elements = elements.iter()
                    .map(|elem| self.remap_expression(elem, var_map))
                    .collect();
                
                Expression::Tuple {
                    elements: new_elements,
                    span: *span,
                }
            },
            Expression::Lambda { params, body, span } => {
                Expression::Lambda {
                    params: params.clone(),
                    body: Box::new(self.remap_expression(body, var_map)),
                    span: *span,
                }
            },
            Expression::AsyncBlock { body, span } => {
                // 非同期ブロックの本体内の変数を適切にリマップする
                // 非同期ブロック内のキャプチャ変数を追跡
                let mut async_var_map = var_map.clone();
                
                // 非同期ブロック内で使用される変数を分析
                let used_vars = self.analyze_used_variables(body);
                
                // キャプチャされる変数に対して新しい変数を割り当て
                for var in used_vars {
                    if var_map.contains_key(&var) {
                        // 既にリマップされている変数は新しいマッピングを使用
                        continue;
                    }
                    
                    // 非同期コンテキストでのキャプチャ用に新しい変数を生成
                    let new_var = self.context.generate_temp_variable(
                        &format!("async_capture_{}", var.name),
                        var.ty.clone()
                    );
                    
                    async_var_map.insert(var.clone(), new_var);
                }
                
                // 非同期ブロック内の式を再帰的にリマップ
                let new_body = Box::new(self.remap_expression(body, &async_var_map));
                
                // 非同期実行のためのメタデータを追加
                let async_metadata = AsyncMetadata {
                    captured_vars: async_var_map.iter()
                        .filter(|(k, v)| k != v)
                        .map(|(k, v)| (k.clone(), v.clone()))
                        .collect(),
                    is_generator: false,
                    awaits_count: self.count_awaits(&new_body),
                    span: *span,
                };
                
                // 非同期ブロックの最適化情報をコンテキストに登録
                self.context.register_async_block(async_metadata);
                
                Expression::AsyncBlock {
                    body: new_body,
                    span: *span,
                }
            },
            Expression::Ref { expr, mutable, span } => {
                Expression::Ref {
                    expr: Box::new(self.remap_expression(expr, var_map)),
                    mutable: *mutable,
                    span: *span,
                }
            },
            Expression::Deref { expr, span } => {
                Expression::Deref {
                    expr: Box::new(self.remap_expression(expr, var_map)),
                    span: *span,
                }
            },
            Expression::Move { expr, span } => {
                Expression::Move {
                    expr: Box::new(self.remap_expression(expr, var_map)),
                    span: *span,
                }
            },
            Expression::Borrow { expr, mutable, span } => {
                Expression::Borrow {
                    expr: Box::new(self.remap_expression(expr, var_map)),
                    mutable: *mutable,
                    span: *span,
                }
            },
        }
    }
    
    /// インライン化した関数の制御フローを接続
    fn connect_inlined_control_flow(
        &self,
        target_func: &mut Function,
        block_id: BlockId,
        instr_idx: usize,
        block_map: &HashMap<BlockId, BlockId>,
        result_var: &Variable
    ) -> Result<(), String> {
        // インライン化した関数の制御フローを呼び出し元の関数に接続
        let current_block = target_func.basic_blocks.get(&block_id)
            .ok_or(format!("ブロックID {}が見つかりません", block_id))?;
        
        // 元のブロックの後続ブロックを保存
        let original_successors = current_block.successors.clone();
        
        // インライン化された関数のエントリブロックと出口ブロックを特定
        let entry_blocks: Vec<BlockId> = block_map.values()
            .filter(|&&mapped_id| {
                let block = target_func.basic_blocks.get(&mapped_id).unwrap();
                block.predecessors.is_empty() || 
                block.predecessors.iter().all(|pred| !block_map.values().any(|&v| v == *pred))
            })
            .cloned()
            .collect();
        
        if entry_blocks.is_empty() {
            return Err("インライン化する関数のエントリブロックが見つかりません".to_string());
        }
        
        let entry_block_id = entry_blocks[0]; // 通常は1つだけのはず
        
        // 出口ブロックを特定（Return命令を含むブロック）
        let exit_blocks: Vec<(BlockId, usize)> = block_map.values()
            .filter_map(|&mapped_id| {
                let block = target_func.basic_blocks.get(&mapped_id).unwrap();
                let return_idx = block.instructions.iter()
                    .position(|instr| matches!(instr, Instruction::Return { .. }));
                
                return_idx.map(|idx| (mapped_id, idx))
            })
            .collect();
        
        if exit_blocks.is_empty() {
            return Err("インライン化する関数の出口ブロックが見つかりません".to_string());
        }
        
        // 元のブロックをエントリブロックに接続
        let block = target_func.basic_blocks.get_mut(&block_id)
            .ok_or(format!("ブロックID {}が見つかりません", block_id))?;
            
        // 呼び出し命令をエントリブロックへのジャンプに置き換え
        block.instructions[instr_idx] = Instruction::Jump {
            target: entry_block_id,
        };
        
        // 元のブロックの後続を更新
        block.successors.clear();
        block.successors.push(entry_block_id);
        
        // エントリブロックの先行を更新
        let entry_block = target_func.basic_blocks.get_mut(&entry_block_id).unwrap();
        entry_block.predecessors.push(block_id);
        
        // 継続ブロックを作成（インライン化後に実行を継続するブロック）
        let continue_block_id = target_func.next_block_id();
        let continue_block = BasicBlock {
            id: continue_block_id,
            instructions: Vec::new(),
            predecessors: Vec::new(),
            successors: original_successors.clone(),
            exception_handler: None,
        };
        
        target_func.basic_blocks.insert(continue_block_id, continue_block);
        target_func.blocks.push(continue_block_id);
        
        // 元のブロックの残りの命令を継続ブロックに移動
        let remaining_instructions = if instr_idx + 1 < block.instructions.len() {
            block.instructions.split_off(instr_idx + 1)
        } else {
            Vec::new()
        };
        
        let continue_block = target_func.basic_blocks.get_mut(&continue_block_id).unwrap();
        continue_block.instructions = remaining_instructions;
        
        // 各出口ブロックを処理
        for (exit_block_id, return_idx) in exit_blocks {
            let exit_block = target_func.basic_blocks.get_mut(&exit_block_id).unwrap();
            
            // Return命令を取得
            if let Instruction::Return { value, span } = &exit_block.instructions[return_idx] {
                // 戻り値を結果変数に代入
                let assign_instr = Instruction::Assign {
                    target: result_var.clone(),
                    value: value.clone(),
                    span: *span,
                };
                
                // Return命令を代入命令に置き換え
                exit_block.instructions[return_idx] = assign_instr;
                
                // 継続ブロックへのジャンプを追加
                exit_block.instructions.push(Instruction::Jump {
                    target: continue_block_id,
                });
                
                // 出口ブロックの後続を更新
                exit_block.successors.push(continue_block_id);
                
                // 継続ブロックの先行を更新
                continue_block.predecessors.push(exit_block_id);
            } else {
                return Err(format!("出口ブロック{}の戻り命令が見つかりません", exit_block_id));
            }
        }
        
        // 元の後続ブロックの先行を更新
        for &succ_id in &original_successors {
            if let Some(succ_block) = target_func.basic_blocks.get_mut(&succ_id) {
                // block_idを削除し、continue_block_idを追加
                if let Some(pos) = succ_block.predecessors.iter().position(|&id| id == block_id) {
                    succ_block.predecessors[pos] = continue_block_id;
                } else {
                    succ_block.predecessors.push(continue_block_id);
                }
            }
        }
        
        // 例外処理パスの更新
        self.update_exception_handlers(target_func, block_id, continue_block_id);
        
        // 制御フローグラフの整合性を検証
        self.validate_control_flow(target_func)?;
        
        Ok(())
    }
    
    /// 例外ハンドラの更新
    fn update_exception_handlers(&self, target_func: &mut Function, old_block_id: BlockId, new_block_id: BlockId) {
        // 全てのブロックの例外ハンドラを更新
        for block in target_func.basic_blocks.values_mut() {
            if let Some(handler) = &mut block.exception_handler {
                if handler.handler_block == old_block_id {
                    handler.handler_block = new_block_id;
                }
            }
        }
    }
    
    /// 制御フローグラフの整合性を検証
    fn validate_control_flow(&self, func: &Function) -> Result<(), String> {
        // 全てのブロックについて、successorsとpredecessorsの整合性を確認
        for (&block_id, block) in &func.basic_blocks {
            // 各後続ブロックの先行リストに現在のブロックが含まれていることを確認
            for &succ_id in &block.successors {
                if let Some(succ_block) = func.basic_blocks.get(&succ_id) {
                    if !succ_block.predecessors.contains(&block_id) {
                        return Err(format!(
                            "制御フロー不整合: ブロック{}の後続{}の先行リストに{}が含まれていません",
                            block_id, succ_id, block_id
                        ));
                    }
                } else {
                    return Err(format!("無効な後続ブロックID: {}", succ_id));
                }
            }
            
            // 各先行ブロックの後続リストに現在のブロックが含まれていることを確認
            for &pred_id in &block.predecessors {
                if let Some(pred_block) = func.basic_blocks.get(&pred_id) {
                    if !pred_block.successors.contains(&block_id) {
                        return Err(format!(
                            "制御フロー不整合: ブロック{}の先行{}の後続リストに{}が含まれていません",
                            block_id, pred_id, block_id
                        ));
                    }
                } else {
                    return Err(format!("無効な先行ブロックID: {}", pred_id));
                }
            }
            
            // 最後の命令が制御フローに一致していることを確認
            if let Some(last_instr) = block.instructions.last() {
                match last_instr {
                    Instruction::Jump { target } => {
                        if block.successors.len() != 1 || block.successors[0] != *target {
                            return Err(format!(
                                "制御フロー不整合: ブロック{}のJump命令の対象{}が後続リスト{:?}と一致しません",
                                block_id, target, block.successors
                            ));
                        }
                    },
                    Instruction::Branch { condition: _, true_target, false_target } => {
                        if block.successors.len() != 2 || 
                           !block.successors.contains(true_target) || 
                           !block.successors.contains(false_target) {
                            return Err(format!(
                                "制御フロー不整合: ブロック{}のBranch命令の対象{},{}が後続リスト{:?}と一致しません",
                                block_id, true_target, false_target, block.successors
                            ));
                        }
                    },
                    Instruction::Return { .. } => {
                        // 戻り命令の場合、後続は空のはず
                        if !block.successors.is_empty() {
                            return Err(format!(
                                "制御フロー不整合: ブロック{}のReturn命令があるのに後続リスト{:?}が空ではありません",
                                block_id, block.successors
                            ));
                        }
                    },
                    Instruction::Unreachable { .. } => {
                        // Unreachableの場合も後続は空のはず
                        if !block.successors.is_empty() {
                            return Err(format!(
                                "制御フロー不整合: ブロック{}のUnreachable命令があるのに後続リスト{:?}が空ではありません",
                                block_id, block.successors
                            ));
                        }
                    },
                    _ => {
                        // 他の命令の場合、後続ブロックが必要
                        if block.successors.is_empty() {
                            return Err(format!(
                                "制御フロー不整合: ブロック{}の最後の命令が制御フロー命令ではなく、後続リストも空です",
                                block_id
                            ));
                        }
                    }
                }
            }
        }
        Ok(())
    }
    
    /// インライン化された関数の戻り値を処理
    fn process_inlined_return(&mut self, target_func: &mut Function, block_id: BlockId, instr_idx: usize, return_value: Expression, result_var: &Variable) -> Result<(), String> {
        let block = target_func.basic_blocks.get_mut(&block_id).unwrap();
        
        // 戻り値を結果変数に代入する命令を作成
        let assign_instr = Instruction::Assign {
            target: result_var.clone(),
            value: return_value,
            span: None,
        };
        
        // 出口ブロックを作成
        let exit_block_id = target_func.next_block_id();
        let exit_block = BasicBlock {
            id: exit_block_id,
            instructions: vec![assign_instr],
            predecessors: vec![block_id],
            successors: Vec::new(),
            exception_handler: None,
        };
        
        target_func.basic_blocks.insert(exit_block_id, exit_block);
        target_func.blocks.push(exit_block_id);
        
        // 元の処理に戻るジャンプを追加
        let continue_block_id = target_func.next_block_id();
        let continue_block = BasicBlock {
            id: continue_block_id,
            instructions: Vec::new(), // 元の命令の続きを後で移動
            predecessors: vec![exit_block_id],
            successors: Vec::new(),
            exception_handler: None,
        };
        
        target_func.basic_blocks.insert(continue_block_id, continue_block);
        target_func.blocks.push(continue_block_id);
        
        // 元のブロックの残りの命令を新しいブロックに移動
        let remaining_instructions = block.instructions.split_off(instr_idx + 1);
        let continue_block = target_func.basic_blocks.get_mut(&continue_block_id).unwrap();
        continue_block.instructions = remaining_instructions;
        
        // 出口ブロックから継続ブロックへのジャンプを追加
        let exit_block = target_func.basic_blocks.get_mut(&exit_block_id).unwrap();
        exit_block.instructions.push(Instruction::Jump {
            target: continue_block_id,
        });
        
        exit_block.successors.push(continue_block_id);
        
        // 元のブロックから出口ブロックへのジャンプを追加
        block.instructions.push(Instruction::Jump {
            target: exit_block_id,
        });
        
        block.successors.push(exit_block_id);
        
        Ok(())
    }
    
    /// 最適化統計を取得
    pub fn get_stats(&self) -> &OptimizationStats {
        &self.stats
    }
    
    /// 関数が最適化されたかどうかを確認
    pub fn is_function_optimized(&self, func_id: FunctionId) -> bool {
        self.optimized_functions.contains(&func_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    // テストケースは省略
} 