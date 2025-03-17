//! 最適化モジュール
//!
//! SwiftLightコンパイラの最適化パスとフレームワークを提供します。

use std::collections::{HashMap, HashSet};
use std::fmt;
use std::time::Duration;

/// 最適化レベル
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptimizationLevel {
    /// 最適化なし（O0）
    O0,
    /// 最小限の最適化（O1）
    O1,
    /// 標準的な最適化（O2、デフォルト）
    O2,
    /// 積極的な最適化（O3）
    O3,
    /// サイズ優先の最適化（Os）
    Os,
    /// デバッグ優先の最適化（Og）
    Og,
}

impl Default for OptimizationLevel {
    fn default() -> Self {
        OptimizationLevel::O2
    }
}

impl fmt::Display for OptimizationLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OptimizationLevel::O0 => write!(f, "O0"),
            OptimizationLevel::O1 => write!(f, "O1"),
            OptimizationLevel::O2 => write!(f, "O2"),
            OptimizationLevel::O3 => write!(f, "O3"),
            OptimizationLevel::Os => write!(f, "Os"),
            OptimizationLevel::Og => write!(f, "Og"),
        }
    }
}

/// 最適化パスの種類
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OptimizationPassKind {
    /// 前処理最適化
    Preprocessing,
    
    /// AST最適化
    AST,
    
    /// 中間表現(IR)最適化
    IR,
    
    /// バックエンド最適化
    Backend,
    
    /// リンク時最適化
    LinkTime,
}

impl fmt::Display for OptimizationPassKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OptimizationPassKind::Preprocessing => write!(f, "前処理"),
            OptimizationPassKind::AST => write!(f, "AST"),
            OptimizationPassKind::IR => write!(f, "IR"),
            OptimizationPassKind::Backend => write!(f, "バックエンド"),
            OptimizationPassKind::LinkTime => write!(f, "リンク時"),
        }
    }
}

/// 最適化パスのIDと種類
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct OptimizationPassId {
    /// パスの名前
    pub name: String,
    
    /// パスの種類
    pub kind: OptimizationPassKind,
}

impl OptimizationPassId {
    /// 新しい最適化パスIDを作成
    pub fn new(name: &str, kind: OptimizationPassKind) -> Self {
        Self {
            name: name.to_string(),
            kind,
        }
    }
}

impl fmt::Display for OptimizationPassId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}({})", self.name, self.kind)
    }
}

/// 最適化パスの実行結果
#[derive(Debug, Clone)]
pub struct OptimizationPassResult {
    /// 変更が行われたか
    pub changed: bool,
    
    /// 実行時間
    pub duration: Duration,
    
    /// 最適化の統計情報
    pub stats: HashMap<String, usize>,
}

impl OptimizationPassResult {
    /// 新しい最適化パス結果を作成
    pub fn new(changed: bool, duration: Duration) -> Self {
        Self {
            changed,
            duration,
            stats: HashMap::new(),
        }
    }
    
    /// 統計情報を追加
    pub fn with_stat(mut self, name: &str, value: usize) -> Self {
        self.stats.insert(name.to_string(), value);
        self
    }
}

/// 最適化パスのインターフェース
pub trait OptimizationPass {
    /// パスのID
    fn id(&self) -> OptimizationPassId;
    
    /// パスの説明
    fn description(&self) -> &str;
    
    /// 依存するパス
    fn dependencies(&self) -> Vec<OptimizationPassId> {
        Vec::new()
    }
    
    /// 無効化条件
    fn is_disabled(&self, _config: &super::config::CompilerConfig) -> bool {
        false
    }
    
    /// 実行メソッド（パスの種類によって具体的な実装が異なる）
}

/// AST最適化パスのインターフェース
pub trait ASTOptimizationPass: OptimizationPass {
    /// ASTに対する最適化を実行
    fn run_on_ast(&self, ast: &mut super::frontend::ast::AST) -> OptimizationPassResult;
}

/// IR最適化パスのインターフェース
pub trait IROptimizationPass: OptimizationPass {
    /// IRに対する最適化を実行
    fn run_on_ir(&self, ir: &mut super::ir::Module) -> OptimizationPassResult;
}

/// 最適化パイプラインの設定
#[derive(Debug, Clone)]
pub struct OptimizationPipelineConfig {
    /// 有効な最適化パス
    pub enabled_passes: HashSet<OptimizationPassId>,
    
    /// 無効化された最適化パス
    pub disabled_passes: HashSet<OptimizationPassId>,
    
    /// パスの実行順序
    pub pass_order: Vec<OptimizationPassId>,
    
    /// 最大イテレーション数
    pub max_iterations: usize,
    
    /// 時間制限
    pub time_limit: Option<Duration>,
}

impl Default for OptimizationPipelineConfig {
    fn default() -> Self {
        Self {
            enabled_passes: HashSet::new(),
            disabled_passes: HashSet::new(),
            pass_order: Vec::new(),
            max_iterations: 10,
            time_limit: None,
        }
    }
}

impl OptimizationPipelineConfig {
    /// 新しい最適化パイプライン設定を作成
    pub fn new() -> Self {
        Self::default()
    }
    
    /// パスを有効化
    pub fn enable_pass(&mut self, pass_id: OptimizationPassId) {
        self.enabled_passes.insert(pass_id.clone());
        self.disabled_passes.remove(&pass_id);
    }
    
    /// パスを無効化
    pub fn disable_pass(&mut self, pass_id: OptimizationPassId) {
        self.disabled_passes.insert(pass_id.clone());
        self.enabled_passes.remove(&pass_id);
    }
    
    /// パスが有効かチェック
    pub fn is_pass_enabled(&self, pass_id: &OptimizationPassId) -> bool {
        if self.disabled_passes.contains(pass_id) {
            return false;
        }
        
        if self.enabled_passes.is_empty() {
            return true;
        }
        
        self.enabled_passes.contains(pass_id)
    }
    
    /// パスの順序を設定
    pub fn set_pass_order(&mut self, order: Vec<OptimizationPassId>) {
        self.pass_order = order;
    }
    
    /// 最大イテレーション数を設定
    pub fn set_max_iterations(&mut self, iterations: usize) {
        self.max_iterations = iterations;
    }
    
    /// 時間制限を設定
    pub fn set_time_limit(&mut self, limit: Duration) {
        self.time_limit = Some(limit);
    }
}

/// 最適化パイプラインの実行結果
#[derive(Debug, Clone)]
pub struct OptimizationPipelineResult {
    /// 各パスの実行結果
    pub pass_results: HashMap<OptimizationPassId, OptimizationPassResult>,
    
    /// イテレーション数
    pub iterations: usize,
    
    /// 合計実行時間
    pub total_duration: Duration,
    
    /// 変更が行われたか
    pub changed: bool,
}

impl OptimizationPipelineResult {
    /// 新しい最適化パイプライン結果を作成
    pub fn new() -> Self {
        Self {
            pass_results: HashMap::new(),
            iterations: 0,
            total_duration: Duration::new(0, 0),
            changed: false,
        }
    }
    
    /// パス結果を追加
    pub fn add_pass_result(
        &mut self,
        pass_id: OptimizationPassId,
        result: OptimizationPassResult,
    ) {
        if result.changed {
            self.changed = true;
        }
        
        self.total_duration += result.duration;
        self.pass_results.insert(pass_id, result);
    }
    
    /// イテレーションをインクリメント
    pub fn increment_iteration(&mut self) {
        self.iterations += 1;
    }
}

/// 最適化マネージャー
pub struct OptimizationManager {
    /// 利用可能なAST最適化パス
    ast_passes: Vec<Box<dyn ASTOptimizationPass>>,
    
    /// 利用可能なIR最適化パス
    ir_passes: Vec<Box<dyn IROptimizationPass>>,
    
    /// パイプラインの設定
    config: OptimizationPipelineConfig,
}

impl OptimizationManager {
    /// 新しい最適化マネージャーを作成
    pub fn new(config: OptimizationPipelineConfig) -> Self {
        Self {
            ast_passes: Vec::new(),
            ir_passes: Vec::new(),
            config,
        }
    }
    
    /// AST最適化パスを登録
    pub fn register_ast_pass(&mut self, pass: Box<dyn ASTOptimizationPass>) {
        self.ast_passes.push(pass);
    }
    
    /// IR最適化パスを登録
    pub fn register_ir_pass(&mut self, pass: Box<dyn IROptimizationPass>) {
        self.ir_passes.push(pass);
    }
    
    /// 標準の最適化パスを登録
    pub fn register_standard_passes(&mut self) {
        // 標準のASTパスを登録
        self.register_ast_pass(Box::new(ConstantFolding::new()));
        self.register_ast_pass(Box::new(DeadCodeElimination::new()));
        
        // 標準のIRパスを登録
        self.register_ir_pass(Box::new(CommonSubexpressionElimination::new()));
        self.register_ir_pass(Box::new(InstructionCombining::new()));
        self.register_ir_pass(Box::new(LoopOptimization::new()));
    }
    
    /// AST最適化を実行
    pub fn run_ast_optimizations(
        &self,
        ast: &mut super::frontend::ast::AST,
        compiler_config: &super::config::CompilerConfig,
    ) -> OptimizationPipelineResult {
        let mut result = OptimizationPipelineResult::new();
        let start_time = std::time::Instant::now();
        
        // 実行するパスを決定
        let mut passes_to_run = self.ast_passes.iter()
            .filter(|pass| {
                !pass.is_disabled(compiler_config) &&
                self.config.is_pass_enabled(&pass.id())
            })
            .collect::<Vec<_>>();
        
        // パスの順序を適用
        if !self.config.pass_order.is_empty() {
            passes_to_run.sort_by_key(|pass| {
                self.config.pass_order.iter()
                    .position(|id| id == &pass.id())
                    .unwrap_or(usize::MAX)
            });
        }
        
        // 依存関係に基づいてパスを実行
        let mut changed = true;
        let mut iteration = 0;
        
        while changed && iteration < self.config.max_iterations {
            changed = false;
            
            for pass in &passes_to_run {
                // 時間制限をチェック
                if let Some(limit) = self.config.time_limit {
                    if start_time.elapsed() > limit {
                        break;
                    }
                }
                
                // パスを実行
                let pass_result = pass.run_on_ast(ast);
                
                if pass_result.changed {
                    changed = true;
                }
                
                result.add_pass_result(pass.id(), pass_result);
            }
            
            result.increment_iteration();
            iteration += 1;
        }
        
        result
    }
    
    /// IR最適化を実行
    pub fn run_ir_optimizations(
        &self,
        ir: &mut super::ir::Module,
        compiler_config: &super::config::CompilerConfig,
    ) -> OptimizationPipelineResult {
        let mut result = OptimizationPipelineResult::new();
        let start_time = std::time::Instant::now();
        
        // 実行するパスを決定
        let mut passes_to_run = self.ir_passes.iter()
            .filter(|pass| {
                !pass.is_disabled(compiler_config) &&
                self.config.is_pass_enabled(&pass.id())
            })
            .collect::<Vec<_>>();
        
        // パスの順序を適用
        if !self.config.pass_order.is_empty() {
            passes_to_run.sort_by_key(|pass| {
                self.config.pass_order.iter()
                    .position(|id| id == &pass.id())
                    .unwrap_or(usize::MAX)
            });
        }
        
        // 依存関係に基づいてパスを実行
        let mut changed = true;
        let mut iteration = 0;
        
        while changed && iteration < self.config.max_iterations {
            changed = false;
            
            for pass in &passes_to_run {
                // 時間制限をチェック
                if let Some(limit) = self.config.time_limit {
                    if start_time.elapsed() > limit {
                        break;
                    }
                }
                
                // パスを実行
                let pass_result = pass.run_on_ir(ir);
                
                if pass_result.changed {
                    changed = true;
                }
                
                result.add_pass_result(pass.id(), pass_result);
            }
            
            result.increment_iteration();
            iteration += 1;
        }
        
        result
    }
}

//------------------------------------------------------------------------------
// 標準AST最適化パス
//------------------------------------------------------------------------------

/// 定数畳み込み最適化パス
pub struct ConstantFolding;

impl ConstantFolding {
    /// 新しい定数畳み込みパスを作成
    pub fn new() -> Self {
        Self
    }
}

impl OptimizationPass for ConstantFolding {
    fn id(&self) -> OptimizationPassId {
        OptimizationPassId::new("ConstantFolding", OptimizationPassKind::AST)
    }
    
    fn description(&self) -> &str {
        "コンパイル時に計算可能な定数式を評価します"
    }
}

impl ASTOptimizationPass for ConstantFolding {
    fn run_on_ast(&self, _ast: &mut super::frontend::ast::AST) -> OptimizationPassResult {
        // 実装はスケルトンのみ
        OptimizationPassResult::new(false, Duration::new(0, 0))
    }
}

/// デッドコード除去最適化パス
pub struct DeadCodeElimination;

impl DeadCodeElimination {
    /// 新しいデッドコード除去パスを作成
    pub fn new() -> Self {
        Self
    }
}

impl OptimizationPass for DeadCodeElimination {
    fn id(&self) -> OptimizationPassId {
        OptimizationPassId::new("DeadCodeElimination", OptimizationPassKind::AST)
    }
    
    fn description(&self) -> &str {
        "到達不能コードや使用されない変数を削除します"
    }
}

impl ASTOptimizationPass for DeadCodeElimination {
    fn run_on_ast(&self, _ast: &mut super::frontend::ast::AST) -> OptimizationPassResult {
        // 実装はスケルトンのみ
        OptimizationPassResult::new(false, Duration::new(0, 0))
    }
}

//------------------------------------------------------------------------------
// 標準IR最適化パス
//------------------------------------------------------------------------------

/// 共通部分式除去最適化パス
pub struct CommonSubexpressionElimination;

impl CommonSubexpressionElimination {
    /// 新しい共通部分式除去パスを作成
    pub fn new() -> Self {
        Self
    }
}

impl OptimizationPass for CommonSubexpressionElimination {
    fn id(&self) -> OptimizationPassId {
        OptimizationPassId::new("CommonSubexpressionElimination", OptimizationPassKind::IR)
    }
    
    fn description(&self) -> &str {
        "重複する計算を検出して除去します"
    }
}

impl IROptimizationPass for CommonSubexpressionElimination {
    fn run_on_ir(&self, _ir: &mut super::ir::Module) -> OptimizationPassResult {
        // 実装はスケルトンのみ
        OptimizationPassResult::new(false, Duration::new(0, 0))
    }
}

/// 命令結合最適化パス
pub struct InstructionCombining;

impl InstructionCombining {
    /// 新しい命令結合パスを作成
    pub fn new() -> Self {
        Self
    }
}

impl OptimizationPass for InstructionCombining {
    fn id(&self) -> OptimizationPassId {
        OptimizationPassId::new("InstructionCombining", OptimizationPassKind::IR)
    }
    
    fn description(&self) -> &str {
        "単純な命令を組み合わせてより効率的な命令に変換します"
    }
}

impl IROptimizationPass for InstructionCombining {
    fn run_on_ir(&self, _ir: &mut super::ir::Module) -> OptimizationPassResult {
        // 実装はスケルトンのみ
        OptimizationPassResult::new(false, Duration::new(0, 0))
    }
}

/// ループ最適化パス
pub struct LoopOptimization;

impl LoopOptimization {
    /// 新しいループ最適化パスを作成
    pub fn new() -> Self {
        Self
    }
}

impl OptimizationPass for LoopOptimization {
    fn id(&self) -> OptimizationPassId {
        OptimizationPassId::new("LoopOptimization", OptimizationPassKind::IR)
    }
    
    fn description(&self) -> &str {
        "ループの最適化（展開、不変式移動、ベクトル化）を行います"
    }
}

impl IROptimizationPass for LoopOptimization {
    fn run_on_ir(&self, _ir: &mut super::ir::Module) -> OptimizationPassResult {
        // 実装はスケルトンのみ
        OptimizationPassResult::new(false, Duration::new(0, 0))
    }
} 