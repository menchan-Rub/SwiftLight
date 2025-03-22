//! # 解析モジュール
//!
//! コンパイル過程でのプログラム解析機能を提供します。
//! このモジュールは、SwiftLight言語の高度な最適化と安全性保証のための
//! 様々な静的解析アルゴリズムを実装しています。
use crate::middleend::ir::Module;
use crate::frontend::error::{ErrorKind, CompilerError, Result};
use crate::backend::optimization::OptimizationLevel;
use std::collections::{HashMap, HashSet, VecDeque, BTreeMap};
use std::rc::Rc;
use std::cell::RefCell;
use std::time::{Duration, Instant};
use std::fmt;
use log::{debug, info, warn};

/// 解析の種類
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AnalysisKind {
    /// データフロー解析
    DataFlow,
    /// コントロールフロー解析
    ControlFlow,
    /// エイリアス解析
    Alias,
    /// メモリ依存性解析
    MemoryDependency,
    /// 副作用解析
    SideEffect,
    /// リーチ可能性解析
    Reachability,
    /// 不変条件解析
    Invariant,
    /// デッドコード解析
    DeadCode,
    /// 定数伝播解析
    ConstantPropagation,
    /// ループ解析
    Loop,
    /// 帰納変数解析
    InductionVariable,
    /// 値域解析
    RangeAnalysis,
    /// ポインタ解析
    PointerAnalysis,
    /// エスケープ解析
    EscapeAnalysis,
    /// 型解析
    TypeAnalysis,
    /// 並行性解析
    ConcurrencyAnalysis,
    /// メモリアクセスパターン解析
    MemoryAccessPattern,
    /// ホットパス解析
    HotPath,
}

impl fmt::Display for AnalysisKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AnalysisKind::DataFlow => write!(f, "データフロー解析"),
            AnalysisKind::ControlFlow => write!(f, "コントロールフロー解析"),
            AnalysisKind::Alias => write!(f, "エイリアス解析"),
            AnalysisKind::MemoryDependency => write!(f, "メモリ依存性解析"),
            AnalysisKind::SideEffect => write!(f, "副作用解析"),
            AnalysisKind::Reachability => write!(f, "リーチ可能性解析"),
            AnalysisKind::Invariant => write!(f, "不変条件解析"),
            AnalysisKind::DeadCode => write!(f, "デッドコード解析"),
            AnalysisKind::ConstantPropagation => write!(f, "定数伝播解析"),
            AnalysisKind::Loop => write!(f, "ループ解析"),
            AnalysisKind::InductionVariable => write!(f, "帰納変数解析"),
            AnalysisKind::RangeAnalysis => write!(f, "値域解析"),
            AnalysisKind::PointerAnalysis => write!(f, "ポインタ解析"),
            AnalysisKind::EscapeAnalysis => write!(f, "エスケープ解析"),
            AnalysisKind::TypeAnalysis => write!(f, "型解析"),
            AnalysisKind::ConcurrencyAnalysis => write!(f, "並行性解析"),
            AnalysisKind::MemoryAccessPattern => write!(f, "メモリアクセスパターン解析"),
            AnalysisKind::HotPath => write!(f, "ホットパス解析"),
        }
    }
}

/// 解析の優先度
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AnalysisPriority {
    /// 低優先度（バックグラウンドで実行可能）
    Low,
    /// 通常優先度
    Normal,
    /// 高優先度（最適化に必須）
    High,
    /// 最高優先度（安全性検証に必須）
    Critical,
}

/// 解析の設定
#[derive(Debug, Clone)]
pub struct AnalysisConfig {
    /// 解析の詳細度
    pub detail_level: DetailLevel,
    /// 解析の時間制限（ミリ秒）
    pub time_limit_ms: Option<u64>,
    /// メモリ使用量制限（バイト）
    pub memory_limit_bytes: Option<u64>,
    /// 並列実行を許可するか
    pub allow_parallel: bool,
    /// キャッシュを使用するか
    pub use_cache: bool,
    /// 増分解析を使用するか
    pub incremental: bool,
    /// デバッグ情報を出力するか
    pub debug_output: bool,
}

impl Default for AnalysisConfig {
    fn default() -> Self {
        Self {
            detail_level: DetailLevel::Normal,
            time_limit_ms: Some(5000), // 5秒
            memory_limit_bytes: Some(100 * 1024 * 1024), // 100MB
            allow_parallel: true,
            use_cache: true,
            incremental: true,
            debug_output: false,
        }
    }
}

/// 解析の詳細度
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetailLevel {
    /// 最小限の解析（高速）
    Minimal,
    /// 通常の解析
    Normal,
    /// 詳細な解析（低速）
    Detailed,
    /// 網羅的な解析（非常に低速）
    Exhaustive,
}

/// 解析の状態
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnalysisState {
    /// 未実行
    NotRun,
    /// 実行中
    Running,
    /// 完了
    Completed,
    /// 失敗
    Failed,
    /// タイムアウト
    TimedOut,
    /// メモリ制限超過
    MemoryLimitExceeded,
    /// 無効化
    Invalidated,
}

/// 解析の統計情報
#[derive(Debug, Clone)]
pub struct AnalysisStats {
    /// 解析の種類
    pub kind: AnalysisKind,
    /// 解析の状態
    pub state: AnalysisState,
    /// 実行時間
    pub execution_time: Duration,
    /// 使用メモリ量（バイト）
    pub memory_usage: u64,
    /// 処理した命令数
    pub instructions_processed: usize,
    /// 処理した関数数
    pub functions_processed: usize,
    /// 処理した基本ブロック数
    pub blocks_processed: usize,
    /// 発見された問題数
    pub issues_found: usize,
    /// 最適化の機会数
    pub optimization_opportunities: usize,
}

impl AnalysisStats {
    /// 新しい統計情報を作成
    fn new(kind: AnalysisKind) -> Self {
        Self {
            kind,
            state: AnalysisState::NotRun,
            execution_time: Duration::from_secs(0),
            memory_usage: 0,
            instructions_processed: 0,
            functions_processed: 0,
            blocks_processed: 0,
            issues_found: 0,
            optimization_opportunities: 0,
        }
    }
}

/// 解析マネージャ
pub struct AnalysisManager {
    /// 実行済みの解析結果
    results: HashMap<AnalysisKind, Box<dyn AnalysisResult>>,
    /// 解析の依存関係
    dependencies: HashMap<AnalysisKind, Vec<AnalysisKind>>,
    /// 無効化された解析結果
    invalidated: HashSet<AnalysisKind>,
    /// 解析の統計情報
    stats: HashMap<AnalysisKind, AnalysisStats>,
    /// 解析の設定
    config: AnalysisConfig,
    /// 解析の優先度
    priorities: HashMap<AnalysisKind, AnalysisPriority>,
    /// 解析のキャッシュ
    cache: Option<AnalysisCache>,
    /// 最後に解析したモジュールのハッシュ
    last_module_hash: Option<u64>,
}

/// 解析キャッシュ
struct AnalysisCache {
    /// キャッシュされた解析結果
    cached_results: HashMap<(AnalysisKind, u64), Box<dyn AnalysisResult>>,
    /// キャッシュのヒット数
    hits: usize,
    /// キャッシュのミス数
    misses: usize,
    /// キャッシュの最大サイズ
    max_size: usize,
}

impl AnalysisManager {
    /// 新しい解析マネージャを作成
    pub fn new() -> Self {
        let mut dependencies = HashMap::new();
        let mut priorities = HashMap::new();
        
        // 解析の依存関係を設定
        dependencies.insert(AnalysisKind::DataFlow, vec![]);
        dependencies.insert(AnalysisKind::ControlFlow, vec![]);
        dependencies.insert(AnalysisKind::Alias, vec![AnalysisKind::DataFlow]);
        dependencies.insert(AnalysisKind::MemoryDependency, vec![AnalysisKind::DataFlow, AnalysisKind::Alias]);
        dependencies.insert(AnalysisKind::SideEffect, vec![AnalysisKind::MemoryDependency]);
        dependencies.insert(AnalysisKind::Reachability, vec![AnalysisKind::ControlFlow]);
        dependencies.insert(AnalysisKind::Invariant, vec![AnalysisKind::DataFlow, AnalysisKind::ControlFlow]);
        dependencies.insert(AnalysisKind::DeadCode, vec![AnalysisKind::Reachability, AnalysisKind::SideEffect]);
        dependencies.insert(AnalysisKind::ConstantPropagation, vec![AnalysisKind::DataFlow]);
        dependencies.insert(AnalysisKind::Loop, vec![AnalysisKind::ControlFlow]);
        dependencies.insert(AnalysisKind::InductionVariable, vec![AnalysisKind::Loop, AnalysisKind::DataFlow]);
        dependencies.insert(AnalysisKind::RangeAnalysis, vec![AnalysisKind::DataFlow, AnalysisKind::ConstantPropagation]);
        dependencies.insert(AnalysisKind::PointerAnalysis, vec![AnalysisKind::DataFlow, AnalysisKind::Alias]);
        dependencies.insert(AnalysisKind::EscapeAnalysis, vec![AnalysisKind::PointerAnalysis]);
        dependencies.insert(AnalysisKind::TypeAnalysis, vec![]);
        dependencies.insert(AnalysisKind::ConcurrencyAnalysis, vec![AnalysisKind::DataFlow, AnalysisKind::ControlFlow, AnalysisKind::MemoryDependency]);
        dependencies.insert(AnalysisKind::MemoryAccessPattern, vec![AnalysisKind::DataFlow, AnalysisKind::Loop]);
        dependencies.insert(AnalysisKind::HotPath, vec![AnalysisKind::ControlFlow]);
        
        // 解析の優先度を設定
        priorities.insert(AnalysisKind::DataFlow, AnalysisPriority::Critical);
        priorities.insert(AnalysisKind::ControlFlow, AnalysisPriority::Critical);
        priorities.insert(AnalysisKind::Alias, AnalysisPriority::High);
        priorities.insert(AnalysisKind::MemoryDependency, AnalysisPriority::High);
        priorities.insert(AnalysisKind::SideEffect, AnalysisPriority::High);
        priorities.insert(AnalysisKind::Reachability, AnalysisPriority::Normal);
        priorities.insert(AnalysisKind::Invariant, AnalysisPriority::Normal);
        priorities.insert(AnalysisKind::DeadCode, AnalysisPriority::Normal);
        priorities.insert(AnalysisKind::ConstantPropagation, AnalysisPriority::High);
        priorities.insert(AnalysisKind::Loop, AnalysisPriority::High);
        priorities.insert(AnalysisKind::InductionVariable, AnalysisPriority::Normal);
        priorities.insert(AnalysisKind::RangeAnalysis, AnalysisPriority::Normal);
        priorities.insert(AnalysisKind::PointerAnalysis, AnalysisPriority::High);
        priorities.insert(AnalysisKind::EscapeAnalysis, AnalysisPriority::Normal);
        priorities.insert(AnalysisKind::TypeAnalysis, AnalysisPriority::Critical);
        priorities.insert(AnalysisKind::ConcurrencyAnalysis, AnalysisPriority::High);
        priorities.insert(AnalysisKind::MemoryAccessPattern, AnalysisPriority::Low);
        priorities.insert(AnalysisKind::HotPath, AnalysisPriority::Low);
        
        // 統計情報を初期化
        let mut stats = HashMap::new();
        for &kind in dependencies.keys() {
            stats.insert(kind, AnalysisStats::new(kind));
        }
        
        let config = AnalysisConfig::default();
        let cache = if config.use_cache {
            Some(AnalysisCache {
                cached_results: HashMap::new(),
                hits: 0,
                misses: 0,
                max_size: 100, // 最大100個の解析結果をキャッシュ
            })
        } else {
            None
        };
        
        Self {
            results: HashMap::new(),
            dependencies,
            invalidated: HashSet::new(),
            stats,
            config,
            priorities,
            cache,
            last_module_hash: None,
        }
    }
    
    /// 設定を更新
    pub fn with_config(mut self, config: AnalysisConfig) -> Self {
        self.config = config;
        if self.config.use_cache && self.cache.is_none() {
            self.cache = Some(AnalysisCache {
                cached_results: HashMap::new(),
                hits: 0,
                misses: 0,
                max_size: 100,
            });
        } else if !self.config.use_cache {
            self.cache = None;
        }
        self
    }
    
    /// モジュールのハッシュ値を計算
    fn compute_module_hash(&self, module: &Module) -> u64 {
        use std::hash::{Hash, Hasher};
        use std::collections::hash_map::DefaultHasher;
        
        let mut hasher = DefaultHasher::new();
        module.hash(&mut hasher);
        hasher.finish()
    }
    
    /// 解析を実行
    pub fn run_analysis(&mut self, kind: AnalysisKind, module: &Module) -> Result<()> {
        // モジュールのハッシュを計算
        let module_hash = self.compute_module_hash(module);
        
        // キャッシュをチェック
        if self.config.use_cache {
            if let Some(cache) = &mut self.cache {
                if let Some(result) = cache.cached_results.get(&(kind, module_hash)) {
                    // キャッシュヒット
                    self.results.insert(kind, result.clone());
                    self.invalidated.remove(&kind);
                    cache.hits += 1;
                    
                    if let Some(stats) = self.stats.get_mut(&kind) {
                        stats.state = AnalysisState::Completed;
                    }
                    
                    debug!("解析キャッシュヒット: {}", kind);
                    return Ok(());
                } else {
                    cache.misses += 1;
                }
            }
        }
        
        // 依存している解析を先に実行
        if let Some(deps) = self.dependencies.get(&kind).cloned() {
            for dep in deps {
                if !self.has_valid_result(dep) {
                    self.run_analysis(dep, module)?;
                }
            }
        }
        
        // 解析の統計情報を更新
        if let Some(stats) = self.stats.get_mut(&kind) {
            stats.state = AnalysisState::Running;
        }
        
        info!("解析開始: {}", kind);
        let start_time = Instant::now();
        
        // 解析を実行
        let result: Result<Box<dyn AnalysisResult>> = match kind {
            AnalysisKind::DataFlow => {
                Ok(Box::new(DataFlowAnalysis::run(module)?))
            },
            AnalysisKind::ControlFlow => {
                Ok(Box::new(ControlFlowAnalysis::run(module)?))
            },
            AnalysisKind::Alias => {
                let dataflow = self.get_result::<DataFlowAnalysis>(AnalysisKind::DataFlow)?;
                Ok(Box::new(AliasAnalysis::run(module, dataflow)?))
            },
            AnalysisKind::MemoryDependency => {
                let dataflow = self.get_result::<DataFlowAnalysis>(AnalysisKind::DataFlow)?;
                let alias = self.get_result::<AliasAnalysis>(AnalysisKind::Alias)?;
                Ok(Box::new(MemoryDependencyAnalysis::run(module, dataflow, alias)?))
            },
            AnalysisKind::SideEffect => {
                let memdep = self.get_result::<MemoryDependencyAnalysis>(AnalysisKind::MemoryDependency)?;
                Ok(Box::new(SideEffectAnalysis::run(module, memdep)?))
            },
            AnalysisKind::Reachability => {
                let cf = self.get_result::<ControlFlowAnalysis>(AnalysisKind::ControlFlow)?;
                Ok(Box::new(ReachabilityAnalysis::run(module, cf)?))
            },
            AnalysisKind::Invariant => {
                let dataflow = self.get_result::<DataFlowAnalysis>(AnalysisKind::DataFlow)?;
                let cf = self.get_result::<ControlFlowAnalysis>(AnalysisKind::ControlFlow)?;
                Ok(Box::new(InvariantAnalysis::run(module, dataflow, cf)?))
            },
            AnalysisKind::DeadCode => {
                let reach = self.get_result::<ReachabilityAnalysis>(AnalysisKind::Reachability)?;
                let side_effect = self.get_result::<SideEffectAnalysis>(AnalysisKind::SideEffect)?;
                Ok(Box::new(DeadCodeAnalysis::run(module, reach, side_effect)?))
            },
            AnalysisKind::ConstantPropagation => {
                let dataflow = self.get_result::<DataFlowAnalysis>(AnalysisKind::DataFlow)?;
                Ok(Box::new(ConstantPropagationAnalysis::run(module, dataflow)?))
            },
            AnalysisKind::Loop => {
                let cf = self.get_result::<ControlFlowAnalysis>(AnalysisKind::ControlFlow)?;
                Ok(Box::new(LoopAnalysis::run(module, cf)?))
            },
            AnalysisKind::InductionVariable => {
                let loop_analysis = self.get_result::<LoopAnalysis>(AnalysisKind::Loop)?;
                let dataflow = self.get_result::<DataFlowAnalysis>(AnalysisKind::DataFlow)?;
                Ok(Box::new(InductionVariableAnalysis::run(module, loop_analysis, dataflow, &self.config)?))
            },
            AnalysisKind::RangeAnalysis => {
                let dataflow = self.get_result::<DataFlowAnalysis>(AnalysisKind::DataFlow)?;
                let const_prop = self.get_result::<ConstantPropagationAnalysis>(AnalysisKind::ConstantPropagation)?;
                Ok(Box::new(RangeAnalysis::run(module, dataflow, const_prop, &self.config)?))
            },
            AnalysisKind::PointerAnalysis => {
                let dataflow = self.get_result::<DataFlowAnalysis>(AnalysisKind::DataFlow)?;
                let alias = self.get_result::<AliasAnalysis>(AnalysisKind::Alias)?;
                Ok(Box::new(PointerAnalysis::run(module, dataflow, alias, &self.config)?))
            },
            AnalysisKind::EscapeAnalysis => {
                let pointer = self.get_result::<PointerAnalysis>(AnalysisKind::PointerAnalysis)?;
                Ok(Box::new(EscapeAnalysis::run(module, pointer, &self.config)?))
            },
            AnalysisKind::TypeAnalysis => {
                Ok(Box::new(TypeAnalysis::run(module, &self.config)?))
            },
            AnalysisKind::ConcurrencyAnalysis => {
                let dataflow = self.get_result::<DataFlowAnalysis>(AnalysisKind::DataFlow)?;
                let cf = self.get_result::<ControlFlowAnalysis>(AnalysisKind::ControlFlow)?;
                let memdep = self.get_result::<MemoryDependencyAnalysis>(AnalysisKind::MemoryDependency)?;
                Ok(Box::new(ConcurrencyAnalysis::run(module, dataflow, cf, memdep, &self.config)?))
            },
            AnalysisKind::MemoryAccessPattern => {
                let dataflow = self.get_result::<DataFlowAnalysis>(AnalysisKind::DataFlow)?;
                let loop_analysis = self.get_result::<LoopAnalysis>(AnalysisKind::Loop)?;
                Ok(Box::new(MemoryAccessPatternAnalysis::run(module, dataflow, loop_analysis, &self.config)?))
            },
            AnalysisKind::HotPath => {
                let cf = self.get_result::<ControlFlowAnalysis>(AnalysisKind::ControlFlow)?;
                Ok(Box::new(HotPathAnalysis::run(module, cf, &self.config)?))
            },
        };
        
        let execution_time = start_time.elapsed();
        
        match result {
            Ok(analysis_result) => {
                // 結果を保存
                self.results.insert(kind, analysis_result.clone());
                self.invalidated.remove(&kind);
                
                // キャッシュに保存
                if self.config.use_cache {
                    if let Some(cache) = &mut self.cache {
                        cache.cached_results.insert((kind, module_hash), analysis_result);
                        
                        // キャッシュサイズが上限を超えたら古いエントリを削除
                        if cache.cached_results.len() > cache.max_size {
                            if let Some(oldest) = cache.cached_results.keys().next().cloned() {
                                cache.cached_results.remove(&oldest);
                            }
                        }
                    }
                }
                
                // 統計情報を更新
                if let Some(stats) = self.stats.get_mut(&kind) {
                    stats.state = AnalysisState::Completed;
                    stats.execution_time = execution_time;
                    
                    // 解析結果から統計情報を更新
                    if let Some(result) = self.results.get(&kind) {
                        stats.instructions_processed = result.get_stats().instructions_processed;
                        stats.functions_processed = result.get_stats().functions_processed;
                        stats.blocks_processed = result.get_stats().blocks_processed;
                        stats.issues_found = result.get_stats().issues_found;
                        stats.optimization_opportunities = result.get_stats().optimization_opportunities;
                    }
                }
                
                info!("解析完了: {} (実行時間: {:?})", kind, execution_time);
                Ok(())
            },
            Err(e) => {
                // 統計情報を更新
                if let Some(stats) = self.stats.get_mut(&kind) {
                    stats.state = AnalysisState::Failed;
                    stats.execution_time = execution_time;
                }
                
                warn!("解析失敗: {} - エラー: {}", kind, e);
                Err(e)
            }
        }
    }
    
    /// 解析結果を取得
    pub fn get_result<T: AnalysisResult + 'static>(&self, kind: AnalysisKind) -> Result<&T> {
        if self.invalidated.contains(&kind) {
            return Err(CompilerError::new(
                ErrorKind::Analysis,
                format!("解析結果が無効化されています: {}", kind),
                None,
            ));
        }
        
        if let Some(result) = self.results.get(&kind) {
            if let Some(typed_result) = result.as_any().downcast_ref::<T>() {
                Ok(typed_result)
            } else {
                Err(CompilerError::new(
                    ErrorKind::Analysis,
                    format!("解析結果の型が一致しません: {}", kind),
                    None,
                ))
            }
        } else {
            Err(CompilerError::new(
                ErrorKind::Analysis,
                format!("解析結果が見つかりません: {}", kind),
                None,
            ))
        }
    }
    
    /// 有効な解析結果があるかチェック
    fn has_valid_result(&self, kind: AnalysisKind) -> bool {
        self.results.contains_key(&kind) && !self.invalidated.contains(&kind)
    }
    
    /// 解析結果を無効化
    pub fn invalidate(&mut self, kind: AnalysisKind) {
        self.invalidated.insert(kind);
        
        // 依存している他の解析も無効化
        for (k, deps) in &self.dependencies {
            if deps.contains(&kind) {
                self.invalidate(*k);
            }
        }
        
        // 統計情報を更新
        if let Some(stats) = self.stats.get_mut(&kind) {
            stats.state = AnalysisState::Invalidated;
        }
        
        debug!("解析結果を無効化: {}", kind);
    }
    
    /// 全ての解析結果を無効化
    pub fn invalidate_all(&mut self) {
        for kind in self.results.keys().copied().collect::<Vec<_>>() {
            self.invalidate(kind);
        }
    }
    
    /// 解析の統計情報を取得
    pub fn get_stats(&self, kind: AnalysisKind) -> Option<&AnalysisStats> {
        self.stats.get(&kind)
    }
    
    /// 全ての解析の統計情報を取得
    pub fn get_all_stats(&self) -> Vec<&AnalysisStats> {
        self.stats.values().collect()
    }
    
    /// 解析の優先度を取得
    pub fn get_priority(&self, kind: AnalysisKind) -> AnalysisPriority {
        self.priorities.get(&kind).copied().unwrap_or(AnalysisPriority::Normal)
    }
    
    /// 解析の優先度を設定
    pub fn set_priority(&mut self, kind: AnalysisKind, priority: AnalysisPriority) {
        self.priorities.insert(kind, priority);
    }
    
    /// 最適化レベルに基づいて必要な解析を実行
    pub fn run_analyses_for_optimization(&mut self, module: &Module, level: OptimizationLevel) -> Result<()> {
        let analyses = match level {
            OptimizationLevel::None => vec![
                AnalysisKind::DataFlow,
                AnalysisKind::ControlFlow,
            ],
            OptimizationLevel::Less => vec![
                AnalysisKind::DataFlow,
                AnalysisKind::ControlFlow,
                AnalysisKind::Reachability,
                AnalysisKind::DeadCode,
                AnalysisKind::ConstantPropagation,
            ],
            OptimizationLevel::Default => vec![
                AnalysisKind::DataFlow,
                AnalysisKind::ControlFlow,
                AnalysisKind::Alias,
                AnalysisKind::MemoryDependency,
                AnalysisKind::SideEffect,
                AnalysisKind::Reachability,
                AnalysisKind::DeadCode,
                AnalysisKind::ConstantPropagation,
                AnalysisKind::Loop,
            ],
            OptimizationLevel::Aggressive => vec![
                AnalysisKind::DataFlow,
                AnalysisKind::ControlFlow,
                AnalysisKind::Alias,
                AnalysisKind::MemoryDependency,
                AnalysisKind::SideEffect,
                AnalysisKind::Reachability,
                AnalysisKind::DeadCode,
                AnalysisKind::ConstantPropagation,
                AnalysisKind::Loop,
                AnalysisKind::InductionVariable,
                AnalysisKind::RangeAnalysis,
                AnalysisKind::PointerAnalysis,
                AnalysisKind::EscapeAnalysis,
                AnalysisKind::TypeAnalysis,
                AnalysisKind::HotPath,
            ],
            _ => vec![
                AnalysisKind::DataFlow,
                AnalysisKind::ControlFlow,
                AnalysisKind::Alias,
                AnalysisKind::MemoryDependency,
                AnalysisKind::SideEffect,
            ],
        };
        
        for kind in analyses {
            self.run_analysis(module, kind)?;
        }
        
        Ok(())
    }
}

/// 解析結果のトレイト
pub trait AnalysisResult: std::any::Any {
    /// Any型へのダウンキャスト
    fn as_any(&self) -> &dyn std::any::Any;
    
    /// 解析結果の文字列表現を取得
    fn to_string(&self) -> String;
    
    /// 解析の依存関係を取得
    fn dependencies(&self) -> Vec<AnalysisKind>;
    
    /// 解析結果の検証
    fn validate(&self) -> Result<()>;
    
    /// 解析結果のメモリ使用量を取得
    fn memory_usage(&self) -> usize;
}

/// データフロー解析
pub struct DataFlowAnalysis {
    /// 変数の定義と使用の関係
    pub def_use_chains: HashMap<VariableId, Vec<UseLocation>>,
    /// 変数の到達定義
    pub reaching_definitions: HashMap<BlockId, HashSet<Definition>>,
    /// 生存変数解析結果
    pub live_variables: HashMap<BlockId, HashSet<VariableId>>,
    /// SSA形式の変数マッピング
    pub ssa_mappings: HashMap<VariableId, Vec<SSAVariable>>,
    /// 変数間の依存関係グラフ
    pub dependency_graph: Graph<VariableId, DependencyKind>,
    /// 解析の実行時間
    pub execution_time: Duration,
}

impl DataFlowAnalysis {
    fn run(module: &Module) -> Result<Self> {
        let start_time = Instant::now();
        
        // 変数の定義と使用の関係を構築
        let mut def_use_chains = HashMap::new();
        let mut reaching_definitions = HashMap::new();
        let mut live_variables = HashMap::new();
        let mut ssa_mappings = HashMap::new();
        
        // モジュール内の各関数を処理
        for function in module.functions() {
            // 制御フローグラフを構築
            let cfg = Self::build_control_flow_graph(function)?;
            
            // 到達定義解析
            Self::compute_reaching_definitions(function, &cfg, &mut reaching_definitions)?;
            
            // 定義-使用チェーンの構築
            Self::build_def_use_chains(function, &reaching_definitions, &mut def_use_chains)?;
            
            // 生存変数解析
            Self::compute_live_variables(function, &cfg, &mut live_variables)?;
            
            // SSA形式への変換準備
            Self::prepare_ssa_conversion(function, &def_use_chains, &mut ssa_mappings)?;
        }
        
        // 変数間の依存関係グラフを構築
        let dependency_graph = Self::build_dependency_graph(&def_use_chains)?;
        
        let execution_time = start_time.elapsed();
        
        Ok(Self {
            def_use_chains,
            reaching_definitions,
            live_variables,
            ssa_mappings,
            dependency_graph,
            execution_time,
        })
    }
    
    fn build_control_flow_graph(function: &Function) -> Result<ControlFlowGraph> {
        let mut cfg = ControlFlowGraph::new();
        
        // 基本ブロックの追加
        for block in function.blocks() {
            cfg.add_block(block.id());
        }
        
        // エッジの追加
        for block in function.blocks() {
            let terminator = block.terminator();
            match terminator {
                Terminator::Branch { target } => {
                    cfg.add_edge(block.id(), *target, EdgeKind::Unconditional);
                },
                Terminator::ConditionalBranch { condition: _, true_target, false_target } => {
                    cfg.add_edge(block.id(), *true_target, EdgeKind::Conditional(true));
                    cfg.add_edge(block.id(), *false_target, EdgeKind::Conditional(false));
                },
                Terminator::Switch { value: _, cases, default } => {
                    for (_, target) in cases {
                        cfg.add_edge(block.id(), *target, EdgeKind::MultiWay);
                    }
                    cfg.add_edge(block.id(), *default, EdgeKind::Default);
                },
                Terminator::Return { .. } => {
                    cfg.add_edge(block.id(), BlockId::EXIT, EdgeKind::Return);
                },
                Terminator::Unreachable => {
                    // 到達不能なブロックには出エッジを追加しない
                },
            }
        }
        
        Ok(cfg)
    }
    
    fn compute_reaching_definitions(
        function: &Function,
        cfg: &ControlFlowGraph,
        reaching_definitions: &mut HashMap<BlockId, HashSet<Definition>>
    ) -> Result<()> {
        // 各ブロックでの定義を収集
        let mut block_defs = HashMap::new();
        for block in function.blocks() {
            let mut defs = HashSet::new();
            for inst in block.instructions() {
                if let Some(def_var) = inst.defined_variable() {
                    defs.insert(Definition {
                        variable: def_var,
                        location: InstructionLocation {
                            block: block.id(),
                            index: inst.index(),
                        },
                    });
                }
            }
            block_defs.insert(block.id(), defs);
        }
        
        // 到達定義の不動点計算
        let mut worklist = VecDeque::new();
        for block_id in cfg.blocks() {
            worklist.push_back(block_id);
            reaching_definitions.insert(block_id, HashSet::new());
        }
        
        while let Some(block_id) = worklist.pop_front() {
            let mut in_defs = HashSet::new();
            
            // 先行ブロックからの到達定義を集める
            for pred in cfg.predecessors(block_id) {
                if let Some(pred_defs) = reaching_definitions.get(&pred) {
                    in_defs.extend(pred_defs.iter().cloned());
                }
            }
            
            // ブロック内での定義で上書き
            let mut out_defs = in_defs.clone();
            if let Some(defs) = block_defs.get(&block_id) {
                for def in defs {
                    // 同じ変数の古い定義を削除
                    out_defs.retain(|d| d.variable != def.variable);
                    // 新しい定義を追加
                    out_defs.insert(def.clone());
                }
            }
            
            // 変更があれば後続ブロックを再処理
            let old_defs = reaching_definitions.get(&block_id).unwrap();
            if &out_defs != old_defs {
                reaching_definitions.insert(block_id, out_defs);
                for succ in cfg.successors(block_id) {
                    worklist.push_back(succ);
                }
            }
        }
        
        Ok(())
    }
    
    fn build_def_use_chains(
        function: &Function,
        reaching_definitions: &HashMap<BlockId, HashSet<Definition>>,
        def_use_chains: &mut HashMap<VariableId, Vec<UseLocation>>
    ) -> Result<()> {
        for block in function.blocks() {
            let block_defs = reaching_definitions.get(&block.id()).unwrap();
            
            for (inst_idx, inst) in block.instructions().iter().enumerate() {
                for used_var in inst.used_variables() {
                    // この使用に到達する定義を見つける
                    let reaching_defs: Vec<_> = block_defs.iter()
                        .filter(|def| def.variable == used_var)
                        .collect();
                    
                    // 各定義に対して使用位置を記録
                    for def in reaching_defs {
                        def_use_chains.entry(def.variable)
                            .or_insert_with(Vec::new)
                            .push(UseLocation {
                                variable: used_var,
                                location: InstructionLocation {
                                    block: block.id(),
                                    index: inst_idx,
                                },
                            });
                    }
                }
            }
        }
        
        Ok(())
    }
    
    fn compute_live_variables(
        function: &Function,
        cfg: &ControlFlowGraph,
        live_variables: &mut HashMap<BlockId, HashSet<VariableId>>
    ) -> Result<()> {
        // 各ブロックでの定義と使用を収集
        let mut block_defs = HashMap::new();
        let mut block_uses = HashMap::new();
        
        for block in function.blocks() {
            let mut defs = HashSet::new();
            let mut uses = HashSet::new();
            
            for inst in block.instructions() {
                // 使用される変数を先に収集（自己代入のケースを考慮）
                for used_var in inst.used_variables() {
                    if !defs.contains(&used_var) {
                        uses.insert(used_var);
                    }
                }
                
                // 定義される変数を収集
                if let Some(def_var) = inst.defined_variable() {
                    defs.insert(def_var);
                }
            }
            
            block_defs.insert(block.id(), defs);
            block_uses.insert(block.id(), uses);
            live_variables.insert(block.id(), HashSet::new());
        }
        
        // 生存変数の不動点計算（逆方向）
        let mut worklist = VecDeque::new();
        for block_id in cfg.blocks() {
            worklist.push_back(block_id);
        }
        
        while let Some(block_id) = worklist.pop_front() {
            let mut out_live = HashSet::new();
            
            // 後続ブロックの生存変数を集める
            for succ in cfg.successors(block_id) {
                if let Some(succ_live) = live_variables.get(&succ) {
                    out_live.extend(succ_live.iter().cloned());
                }
            }
            
            // ブロック内での定義と使用を考慮
            let defs = block_defs.get(&block_id).unwrap();
            let uses = block_uses.get(&block_id).unwrap();
            
            // OUT - DEF + USE
            let mut in_live = out_live.clone();
            for def in defs {
                in_live.remove(def);
            }
            in_live.extend(uses.iter().cloned());
            
            // 変更があれば先行ブロックを再処理
            let old_live = live_variables.get(&block_id).unwrap();
            if &in_live != old_live {
                live_variables.insert(block_id, in_live);
                for pred in cfg.predecessors(block_id) {
                    worklist.push_back(pred);
                }
            }
        }
        
        Ok(())
    }
    
    fn prepare_ssa_conversion(
        function: &Function,
        def_use_chains: &HashMap<VariableId, Vec<UseLocation>>,
        ssa_mappings: &mut HashMap<VariableId, Vec<SSAVariable>>
    ) -> Result<()> {
        // 支配木の構築
        let cfg = Self::build_control_flow_graph(function)?;
        let dom_tree = DominatorTree::compute(&cfg)?;
        
        // 変数ごとに処理
        for var_id in def_use_chains.keys() {
            let mut var_versions = Vec::new();
            let mut current_version = 0;
            
            // 定義位置を収集
            let mut def_locations = Vec::new();
            for block in function.blocks() {
                for (idx, inst) in block.instructions().iter().enumerate() {
                    if let Some(def_var) = inst.defined_variable() {
                        if def_var == *var_id {
                            def_locations.push(InstructionLocation {
                                block: block.id(),
                                index: idx,
                            });
                        }
                    }
                }
            }
            
            // 各定義に対してSSA変数を割り当て
            for def_loc in def_locations {
                current_version += 1;
                var_versions.push(SSAVariable {
                    original: *var_id,
                    version: current_version,
                    definition: def_loc,
                });
            }
            
            ssa_mappings.insert(*var_id, var_versions);
        }
        
        Ok(())
    }
    
    fn build_dependency_graph(
        def_use_chains: &HashMap<VariableId, Vec<UseLocation>>
    ) -> Result<Graph<VariableId, DependencyKind>> {
        let mut graph = Graph::new();
        
        // 変数をノードとして追加
        for var_id in def_use_chains.keys() {
            graph.add_node(*var_id);
        }
        
        // 使用関係に基づいてエッジを追加
        for (def_var, uses) in def_use_chains {
            for use_loc in uses {
                // 使用位置で定義される変数を見つける
                let used_var = use_loc.variable;
                
                // 依存関係を追加（定義変数 -> 使用変数）
                if graph.has_node(used_var) && *def_var != used_var {
                    graph.add_edge(*def_var, used_var, DependencyKind::DataFlow);
                }
            }
        }
        
        Ok(graph)
    }
}

impl AnalysisResult for DataFlowAnalysis {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    
    fn to_string(&self) -> String {
        format!(
            "DataFlowAnalysis {{ def_use_chains: {} entries, reaching_definitions: {} blocks, live_variables: {} blocks, execution_time: {:?} }}",
            self.def_use_chains.len(),
            self.reaching_definitions.len(),
            self.live_variables.len(),
            self.execution_time
        )
    }
    
    fn dependencies(&self) -> Vec<AnalysisKind> {
        vec![] // データフロー解析は他の解析に依存しない
    }
    
    fn validate(&self) -> Result<()> {
        // 解析結果の整合性チェック
        for (var_id, uses) in &self.def_use_chains {
            if uses.is_empty() {
                // 使用のない定義は警告対象
                log::warn!("Variable {:?} is defined but never used", var_id);
            }
        }
        
        // SSAマッピングの検証
        for (var_id, versions) in &self.ssa_mappings {
            if versions.is_empty() {
                return Err(Error::AnalysisError(format!(
                    "Variable {:?} has no SSA versions assigned", var_id
                )));
            }
        }
        
        Ok(())
    }
    
    fn memory_usage(&self) -> usize {
        // 各データ構造のメモリ使用量を概算
        let def_use_size = self.def_use_chains.len() * std::mem::size_of::<(VariableId, Vec<UseLocation>)>();
        let reach_def_size = self.reaching_definitions.len() * std::mem::size_of::<(BlockId, HashSet<Definition>)>();
        let live_var_size = self.live_variables.len() * std::mem::size_of::<(BlockId, HashSet<VariableId>)>();
        let ssa_size = self.ssa_mappings.len() * std::mem::size_of::<(VariableId, Vec<SSAVariable>)>();
        let graph_size = self.dependency_graph.node_count() * std::mem::size_of::<VariableId>() +
                         self.dependency_graph.edge_count() * std::mem::size_of::<(VariableId, VariableId, DependencyKind)>();
        
        def_use_size + reach_def_size + live_var_size + ssa_size + graph_size
    }
}

/// コントロールフロー解析
pub struct ControlFlowAnalysis {
    /// 制御フローグラフ
    pub cfg: HashMap<FunctionId, ControlFlowGraph>,
    /// ループ情報
    pub loops: HashMap<FunctionId, LoopNest>,
    /// 支配木
    pub dominator_trees: HashMap<FunctionId, DominatorTree>,
    /// 後支配木
    pub post_dominator_trees: HashMap<FunctionId, DominatorTree>,
    /// 制御依存関係
    pub control_dependencies: HashMap<FunctionId, Graph<BlockId, ControlDependencyKind>>,
    /// 自然ループ解析結果
    pub natural_loops: HashMap<FunctionId, Vec<NaturalLoop>>,
    /// 解析の実行時間
    pub execution_time: Duration,
}

impl ControlFlowAnalysis {
    fn run(module: &Module) -> Result<Self> {
        let start_time = Instant::now();
        
        let mut cfg = HashMap::new();
        let mut loops = HashMap::new();
        let mut dominator_trees = HashMap::new();
        let mut post_dominator_trees = HashMap::new();
        let mut control_dependencies = HashMap::new();
        let mut natural_loops = HashMap::new();
        
        // モジュール内の各関数を処理
        for function in module.functions() {
            let function_id = function.id();
            
            // 制御フローグラフを構築
            let function_cfg = Self::build_control_flow_graph(function)?;
            cfg.insert(function_id, function_cfg.clone());
            
            // 支配木を計算
            let dom_tree = DominatorTree::compute(&function_cfg)?;
            dominator_trees.insert(function_id, dom_tree.clone());
            
            // 後支配木を計算
            let post_dom_tree = Self::compute_post_dominator_tree(&function_cfg)?;
            post_dominator_trees.insert(function_id, post_dom_tree.clone());
            
            // 制御依存関係を計算
            let ctrl_deps = Self::compute_control_dependencies(&function_cfg, &post_dom_tree)?;
            control_dependencies.insert(function_id, ctrl_deps);
            
            // 自然ループを検出
            let function_loops = Self::detect_natural_loops(&function_cfg, &dom_tree)?;
            natural_loops.insert(function_id, function_loops.clone());
            
            // ループネストを構築
            let loop_nest = Self::build_loop_nest(&function_cfg, &&function_loops, &dom_tree)?;
            loops.insert(function_id, loop_nest);
        }
        
        let execution_time = start_time.elapsed();
        
        Ok(Self {
            cfg,
            loops,
            dominator_trees,
            post_dominator_trees,
            control_dependencies,
            natural_loops,
            execution_time,
        })
    }
    
    fn build_control_flow_graph(function: &Function) -> Result<ControlFlowGraph> {
        let mut cfg = ControlFlowGraph::new();
        
        // 基本ブロックの追加
        for block in function.blocks() {
            cfg.add_block(block.id());
        }
        
        // エントリーブロックとエグジットブロックを追加
        cfg.add_block(BlockId::ENTRY);
        cfg.add_block(BlockId::EXIT);
        
        // エントリーブロックから最初のブロックへのエッジを追加
        if let Some(first_block) = function.blocks().first() {
            cfg.add_edge(BlockId::ENTRY, first_block.id(), EdgeKind::Unconditional);
        }
        
        // エッジの追加
        for block in function.blocks() {
            let terminator = block.terminator();
            match terminator {
                Terminator::Branch { target } => {
                    cfg.add_edge(block.id(), *target, EdgeKind::Unconditional);
                },
                Terminator::ConditionalBranch { condition: _, true_target, false_target } => {
                    cfg.add_edge(block.id(), *true_target, EdgeKind::Conditional(true));
                    cfg.add_edge(block.id(), *false_target, EdgeKind::Conditional(false));
                },
                Terminator::Switch { value: _, cases, default } => {
                    for (_, target) in cases {
                        cfg.add_edge(block.id(), *target, EdgeKind::MultiWay);
                    }
                    cfg.add_edge(block.id(), *default, EdgeKind::Default);
                },
                Terminator::Return { .. } => {
                    cfg.add_edge(block.id(), BlockId::EXIT, EdgeKind::Return);
                },
                Terminator::Unreachable => {
                    // 到達不能なブロックには出エッジを追加しない
                },
            }
        }
        
        Ok(cfg)
    }
    
    fn compute_post_dominator_tree(cfg: &ControlFlowGraph) -> Result<DominatorTree> {
        // CFGを反転
        let mut reversed_cfg = ControlFlowGraph::new();
        
        // ノードをコピー
        for block_id in cfg.blocks() {
            reversed_cfg.add_block(block_id);
        }
        
        // エッジを反転
        for block_id in cfg.blocks() {
            for succ in cfg.successors(block_id) {
                reversed_cfg.add_edge(succ, block_id, EdgeKind::Reversed);
            }
        }
        
        // 反転したCFGで支配関係を計算
        let post_dom_tree = DominatorTree::compute(&reversed_cfg)?;
        
        Ok(post_dom_tree)
    }
    
    fn compute_control_dependencies(
        cfg: &ControlFlowGraph,
        post_dom_tree: &DominatorTree
    ) -> Result<Graph<BlockId, ControlDependencyKind>> {
        let mut control_deps = Graph::new();
        
        // ノードを追加
        for block_id in cfg.blocks() {
            control_deps.add_node(block_id);
        }
        
        // 制御依存関係を計算
        for block_id in cfg.blocks() {
            for succ in cfg.successors(block_id) {
                // ブロックが後続ブロックを後支配していない場合、制御依存関係がある
                if !post_dom_tree.dominates(succ, block_id) {
                    // 後支配境界を見つける
                    let mut current = succ;
                    while !post_dom_tree.dominates(current, block_id) && 
                          post_dom_tree.immediate_dominator(current) != BlockId::INVALID {
                        current = post_dom_tree.immediate_dominator(current);
                    }
                    
                    // 制御依存関係を追加
                    let edge_kind = if cfg.edge_kind(block_id, succ) == Some(EdgeKind::Conditional(true)) {
                        ControlDependencyKind::True
                    } else if cfg.edge_kind(block_id, succ) == Some(EdgeKind::Conditional(false)) {
                        ControlDependencyKind::False
                    } else {
                        ControlDependencyKind::Always
                    };
                    
                    control_deps.add_edge(block_id, current, edge_kind);
                }
            }
        }
        
        Ok(control_deps)
    }
    
    fn detect_natural_loops(
        cfg: &ControlFlowGraph,
        dom_tree: &DominatorTree
    ) -> Result<Vec<NaturalLoop>> {
        let mut loops = Vec::new();
        
        // バックエッジを検出
        for block_id in cfg.blocks() {
            for succ in cfg.successors(block_id) {
                // バックエッジ: 後続ブロックが現在のブロックを支配している
                if dom_tree.dominates(succ, block_id) {
                    // ループヘッダとバックエッジの終点
                    let header = succ;
                    let latch = block_id;
                    
                    // ループ本体を計算
                    let mut loop_body = HashSet::new();
                    loop_body.insert(header);
                    
                    // ヘッダからバックエッジの始点までの全てのパスを見つける
                    let mut worklist = VecDeque::new();
                    worklist.push_back(latch);
                    
                    while let Some(current) = worklist.pop_front() {
                        if !loop_body.contains(&current) {
                            loop_body.insert(current);
                            
                            // 先行ブロックをワークリストに追加
                            for pred in cfg.predecessors(current) {
                                if pred != header && !loop_body.contains(&pred) {
                                    worklist.push_back(pred);
                                }
                            }
                        }
                    }
                    
                    // 自然ループを作成
                    let natural_loop = NaturalLoop {
                        header,
                        latch,
                        body: loop_body,
                        exits: Self::compute_loop_exits(cfg, &loop_body),
                    };
                    
                    loops.push(natural_loop);
                }
            }
        }
        
        Ok(loops)
    }
    
    fn compute_loop_exits(
        cfg: &ControlFlowGraph,
        loop_body: &HashSet<BlockId>
    ) -> HashSet<BlockId> {
        let mut exits = HashSet::new();
        
        // ループ内の各ブロックについて
        for block_id in loop_body {
            // ループ外への出エッジを持つブロックを見つける
            for succ in cfg.successors(*block_id) {
                if !loop_body.contains(&succ) {
                    exits.insert(*block_id);
                    break;
                }
            }
        }
        
        exits
    }
    
    fn build_loop_nest(
        cfg: &ControlFlowGraph,
        natural_loops: &[NaturalLoop],
        dom_tree: &DominatorTree
    ) -> Result<LoopNest> {
        let mut loop_nest = LoopNest::new();
        
        // ループをIDで整理
        for (i, natural_loop) in natural_loops.iter().enumerate() {
            let loop_id = LoopId(i as u32);
            loop_nest.add_loop(loop_id, natural_loop.clone());
        }
        
        // ループの包含関係を計算
        for i in 0..natural_loops.len() {
            let loop_i_id = LoopId(i as u32);
            let loop_i = &natural_loops[i];
            
            for j in 0..natural_loops.len() {
                if i == j {
                    continue;
                }
                
                let loop_j_id = LoopId(j as u32);
                let loop_j = &natural_loops[j];
                
                // ループiがループjを含む場合
                if dom_tree.dominates(loop_i.header, loop_j.header) && 
                   loop_i.body.contains(&loop_j.header) {
                    // ループjの本体がループiの本体に完全に含まれるか確認
                    let mut is_nested = true;
                    for block_id in &loop_j.body {
                        if !loop_i.body.contains(block_id) {
                            is_nested = false;
                            break;
                        }
                    }
                    
                    if is_nested {
                        loop_nest.add_nested_relationship(loop_i_id, loop_j_id);
                    }
                }
            }
        }
        
        // ループの深さを計算
        loop_nest.compute_loop_depths();
        
        Ok(loop_nest)
    }
}

impl AnalysisResult for ControlFlowAnalysis {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    
    fn to_string(&self) -> String {
        format!(
            "ControlFlowAnalysis {{ functions: {}, loops: {}, execution_time: {:?} }}",
            self.cfg.len(),
            self.loops.values().map(|l| l.loop_count()).sum::<usize>(),
            self.execution_time
        )
    }
    
    fn dependencies(&self) -> Vec<AnalysisKind> {
        vec![] // 制御フロー解析は他の解析に依存しない
    }
    
    fn validate(&self) -> Result<()> {
        // 各関数のCFGが有効かチェック
        for (func_id, cfg) in &self.cfg {
            if cfg.blocks().count() == 0 {
                return Err(Error::AnalysisError(format!(
                    "Function {:?} has empty control flow graph", func_id
                )));
            }
            
            // エントリーブロックとエグジットブロックの存在確認
            if !cfg.has_block(BlockId::ENTRY) || !cfg.has_block(BlockId::EXIT) {
                return Err(Error::AnalysisError(format!(
                    "Function {:?} missing entry or exit block", func_id
                )));
            }
            
            // 到達不能なブロックがないか確認
            let reachable = cfg.compute_reachable_blocks();
            for block_id in cfg.blocks() {
                if !reachable.contains(&block_id) && block_id != BlockId::EXIT {
                    return Err(Error::AnalysisError(format!(
                        "Function {:?} contains unreachable block {:?}", func_id, block_id
                    )));
                }
            }
            
            // 循環依存がないか確認（ループ以外で）
            if let Some(cycle) = cfg.find_non_loop_cycles() {
                return Err(Error::AnalysisError(format!(
                    "Function {:?} contains non-loop cycle: {:?}", func_id, cycle
                )));
            }
        }
        
        // ループ解析の整合性チェック
        for (func_id, loop_info) in &self.loops {
            if !self.cfg.contains_key(func_id) {
                return Err(Error::AnalysisError(format!(
                    "Loop information exists for non-existent function {:?}", func_id
                )));
            }
            
            // 各ループのヘッダーが有効なブロックIDか確認
            for loop_id in loop_info.all_loops() {
                let natural_loop = loop_info.get_loop(loop_id).ok_or_else(|| {
                    Error::AnalysisError(format!("Invalid loop ID {:?} in function {:?}", loop_id, func_id))
                })?;
                
                if !self.cfg[func_id].has_block(natural_loop.header) {
                    return Err(Error::AnalysisError(format!(
                        "Loop {:?} in function {:?} has invalid header block {:?}",
                        loop_id, func_id, natural_loop.header
                    )));
                }
                
                // ループ本体の各ブロックが有効か確認
                for block_id in &natural_loop.body {
                    if !self.cfg[func_id].has_block(*block_id) {
                        return Err(Error::AnalysisError(format!(
                            "Loop {:?} in function {:?} contains invalid block {:?}",
                            loop_id, func_id, block_id
                        )));
                    }
                }
            }
            
            // ループネストの整合性チェック
            if let Err(err) = loop_info.validate_nesting() {
                return Err(Error::AnalysisError(format!(
                    "Loop nesting validation failed for function {:?}: {}", func_id, err
                )));
            }
        }
        
        Ok(())
    }
    
    fn memory_usage(&self) -> usize {
        let mut total = std::mem::size_of::<Self>();
        
        // CFGのメモリ使用量を計算
        for cfg in self.cfg.values() {
            total += cfg.memory_usage();
        }
        
        // ループ情報のメモリ使用量を計算
        for loop_info in self.loops.values() {
            total += loop_info.memory_usage();
        }
        
        // ドミネータツリーのメモリ使用量を計算
        for dom_tree in self.dominators.values() {
            total += dom_tree.memory_usage();
        }
        
        total
    }
}

/// エイリアス解析
/// メモリ内の異なる参照が同じメモリ位置を指す可能性があるかを分析
pub struct AliasAnalysis {
    // 関数ごとのエイリアス情報
    alias_sets: HashMap<FunctionId, AliasSetForest>,
    // ポインタ間の関係マップ
    pointer_relations: HashMap<VariableId, HashSet<VariableId>>,
    // グローバル変数のエイリアス情報
    global_aliases: AliasSetForest,
    // 解析の実行時間
    execution_time: Duration,
}

impl AliasAnalysis {
    fn run(module: &Module, dataflow: &DataFlowAnalysis) -> Result<Self> {
        let start_time = Instant::now();
        
        let mut alias_sets = HashMap::new();
        let mut pointer_relations = HashMap::new();
        let global_aliases = Self::analyze_global_variables(module)?;
        
        // 各関数のエイリアス解析を実行
        for func_id in module.functions.keys() {
            let func_alias_sets = Self::analyze_function(module, func_id, dataflow)?;
            alias_sets.insert(*func_id, func_alias_sets);
        }
        
        // 関数間のポインタ関係を解析
        Self::analyze_interprocedural_aliases(module, &mut pointer_relations, &alias_sets)?;
        
        Ok(Self {
            alias_sets,
            pointer_relations,
            global_aliases,
            execution_time: start_time.elapsed(),
        })
    }
    
    fn analyze_global_variables(module: &Module) -> Result<AliasSetForest> {
        let mut forest = AliasSetForest::new();
        
        // グローバル変数間のエイリアス関係を解析
        for (var_id, var) in &module.global_variables {
            forest.add_variable(*var_id);
            
            // 初期化式に含まれる他のグローバル変数との関係を解析
            if let Some(init) = &var.initializer {
                Self::analyze_expression_aliases(init, &mut forest, module)?;
            }
        }
        
        Ok(forest)
    }
    
    fn analyze_function(
        module: &Module,
        func_id: &FunctionId,
        dataflow: &DataFlowAnalysis
    ) -> Result<AliasSetForest> {
        let mut forest = AliasSetForest::new();
        let func = &module.functions[func_id];
        
        // 関数パラメータのエイリアス関係を解析
        for param in &func.parameters {
            forest.add_variable(param.id);
            
            // ポインタ型パラメータの場合、潜在的なエイリアスとして扱う
            if param.ty.is_pointer_type() {
                forest.mark_as_potential_alias(param.id);
            }
        }
        
        // 関数本体の各基本ブロックを解析
        if let Some(body) = &func.body {
            for block_id in body.blocks() {
                let block = body.get_block(block_id).ok_or_else(|| {
                    Error::AnalysisError(format!("Block {:?} not found in function {:?}", block_id, func_id))
                })?;
                
                // ブロック内の各命令を解析
                for instr in &block.instructions {
                    Self::analyze_instruction_aliases(instr, &mut forest, module, dataflow)?;
                }
            }
        }
        
        Ok(forest)
    }
    
    fn analyze_expression_aliases(
        expr: &Expression,
        forest: &mut AliasSetForest,
        module: &Module
    ) -> Result<()> {
        match expr {
            Expression::Variable(var_id) => {
                forest.add_variable(*var_id);
            },
            Expression::FieldAccess { base, field: _ } => {
                Self::analyze_expression_aliases(base, forest, module)?;
            },
            Expression::IndexAccess { base, index } => {
                Self::analyze_expression_aliases(base, forest, module)?;
                Self::analyze_expression_aliases(index, forest, module)?;
            },
            Expression::Call { function, arguments } => {
                // 関数呼び出しの結果がポインタを返す場合、新しいエイリアスセットを作成
                if let Some(func_id) = function.get_function_id() {
                    let func = &module.functions[&func_id];
                    if func.return_type.is_pointer_type() {
                        // 戻り値のエイリアスセットを作成
                        let result_var = expr.get_result_variable().ok_or_else(|| {
                            Error::AnalysisError("Call expression has no result variable".to_string())
                        })?;
                        forest.add_variable(result_var);
                        forest.mark_as_potential_alias(result_var);
                    }
                }
                
                // 引数のエイリアス関係を解析
                for arg in arguments {
                    Self::analyze_expression_aliases(arg, forest, module)?;
                }
            },
            Expression::Unary { operand, .. } => {
                Self::analyze_expression_aliases(operand, forest, module)?;
            },
            Expression::Binary { left, right, .. } => {
                Self::analyze_expression_aliases(left, forest, module)?;
                Self::analyze_expression_aliases(right, forest, module)?;
            },
            Expression::Cast { expr, .. } => {
                Self::analyze_expression_aliases(expr, forest, module)?;
            },
            // その他の式タイプは必要に応じて追加
            _ => {}
        }
        
        Ok(())
    }
    
    fn analyze_instruction_aliases(
        instr: &Instruction,
        forest: &mut AliasSetForest,
        module: &Module,
        dataflow: &DataFlowAnalysis
    ) -> Result<()> {
        match instr {
            Instruction::Alloca { result, .. } => {
                // 新しいメモリ割り当ては一意のエイリアスセットを持つ
                forest.add_variable(*result);
            },
            Instruction::Load { result, address } => {
                forest.add_variable(*result);
                
                // アドレスから読み込まれた値は、そのアドレスが指す可能性のあるすべての値のエイリアスになる
                if let Some(addr_var) = address.get_variable_id() {
                    let potential_aliases = forest.get_potential_aliases(addr_var);
                    for alias in potential_aliases {
                        forest.merge_sets(*result, alias);
                    }
                }
            },
            Instruction::Store { value, address } => {
                // 値をアドレスに格納する場合、そのアドレスが指す可能性のあるすべての変数に影響
                if let Some(addr_var) = address.get_variable_id() {
                    if let Some(val_var) = value.get_variable_id() {
                        forest.add_potential_alias(addr_var, val_var);
                    }
                }
            },
            Instruction::GetElementPtr { result, base, indices } => {
                forest.add_variable(*result);
                
                // ベースポインタのエイリアス関係を新しいポインタに伝播
                if let Some(base_var) = base.get_variable_id() {
                    forest.propagate_aliases(base_var, *result);
                }
                
                // インデックスのエイリアス関係を解析
                for idx in indices {
                    if let Some(idx_expr) = idx.as_expression() {
                        Self::analyze_expression_aliases(idx_expr, forest, module)?;
                    }
                }
            },
            Instruction::Call { result, function, arguments } => {
                if let Some(res) = result {
                    forest.add_variable(*res);
                    
                    // 関数の戻り値がポインタ型の場合、潜在的なエイリアスとして扱う
                    if let Some(func_id) = function.get_function_id() {
                        let func = &module.functions[&func_id];
                        if func.return_type.is_pointer_type() {
                            forest.mark_as_potential_alias(*res);
                        }
                    }
                }
                
                // 引数のエイリアス関係を解析
                for arg in arguments {
                    if let Some(arg_expr) = arg.as_expression() {
                        Self::analyze_expression_aliases(arg_expr, forest, module)?;
                    }
                }
                
                // 関数呼び出しによるサイドエフェクトを考慮
                // データフロー解析の結果を使用して、関数が変更する可能性のある変数を特定
                if let Some(func_id) = function.get_function_id() {
                    let func_effects = dataflow.get_function_effects(func_id).ok_or_else(|| {
                        Error::AnalysisError(format!("No dataflow information for function {:?}", func_id))
                    })?;
                    
                    for var_id in &func_effects.modified_variables {
                        forest.mark_as_modified(*var_id);
                    }
                }
            },
            Instruction::Phi { result, incoming } => {
                forest.add_variable(*result);
                
                // PHI命令の各入力値からエイリアス関係を伝播
                for (value, _) in incoming {
                    if let Some(var_id) = value.get_variable_id() {
                        forest.merge_sets(*result, var_id);
                    }
                }
            },
            // その他の命令タイプは必要に応じて追加
            _ => {}
        }
        
        Ok(())
    }
    
    fn analyze_interprocedural_aliases(
        module: &Module,
        pointer_relations: &mut HashMap<VariableId, HashSet<VariableId>>,
        alias_sets: &HashMap<FunctionId, AliasSetForest>
    ) -> Result<()> {
        // 関数間の呼び出し関係を解析し、ポインタの関係を構築
        let call_graph = module.compute_call_graph()?;
        
        // 各関数の呼び出し関係に基づいてポインタ関係を伝播
        for (caller_id, callees) in &call_graph.edges {
            let caller_aliases = alias_sets.get(caller_id).ok_or_else(|| {
                Error::AnalysisError(format!("No alias information for function {:?}", caller_id))
            })?;
            
            for callee_id in callees {
                let callee_aliases = alias_sets.get(callee_id).ok_or_else(|| {
                    Error::AnalysisError(format!("No alias information for function {:?}", callee_id))
                })?;
                
                // 呼び出し元から呼び出し先へのパラメータ渡しによるエイリアス関係を解析
                Self::propagate_parameter_aliases(
                    module, *caller_id, *callee_id, 
                    caller_aliases, callee_aliases, 
                    pointer_relations
                )?;
            }
        }
        
        Ok(())
    }
    
    fn propagate_parameter_aliases(
        module: &Module,
        caller_id: FunctionId,
        callee_id: FunctionId,
        caller_aliases: &AliasSetForest,
        callee_aliases: &AliasSetForest,
        pointer_relations: &mut HashMap<VariableId, HashSet<VariableId>>
    ) -> Result<()> {
        let caller = &module.functions[&caller_id];
        let callee = &module.functions[&callee_id];
        
        // 呼び出し元の関数本体から呼び出し先への引数を特定
        if let Some(body) = &caller.body {
            for block_id in body.blocks() {
                let block = body.get_block(block_id).ok_or_else(|| {
                    Error::AnalysisError(format!("Block {:?} not found in function {:?}", block_id, caller_id))
                })?;
                
                for instr in &block.instructions {
                    if let Instruction::Call { function, arguments, .. } = instr {
                        if let Some(func_id) = function.get_function_id() {
                            if func_id == callee_id {
                                // 呼び出し先の各パラメータに対応する引数のエイリアス関係を伝播
                                for (i, param) in callee.parameters.iter().enumerate() {
                                    if i < arguments.len() {
                                        if let Some(arg_expr) = arguments[i].as_expression() {
                                            if let Some(arg_var) = arg_expr.get_variable_id() {
                                                // 引数変数と対応するパラメータ変数の間のポインタ関係を記録
                                                pointer_relations
                                                    .entry(arg_var)
                                                    .or_insert_with(HashSet::new)
                                                    .insert(param.id);
                                                
                                                // 引数のエイリアスセットをパラメータのエイリアスセットに伝播
                                                let arg_aliases = caller_aliases.get_aliases(arg_var);
                                                for alias in arg_aliases {
                                                    pointer_relations
                                                        .entry(alias)
                                                        .or_insert_with(HashSet::new)
                                                        .insert(param.id);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        
        Ok(())
    }
    
    // エイリアス情報の照会メソッド
    pub fn may_alias(&self, var1: VariableId, var2: VariableId, func_id: Option<FunctionId>) -> bool {
        // 同じ変数は常にエイリアス
        if var1 == var2 {
            return true;
        }
        
        // 関数内のローカル変数のエイリアスチェック
        if let Some(func_id) = func_id {
            if let Some(alias_forest) = self.alias_sets.get(&func_id) {
                if alias_forest.in_same_set(var1, var2) {
                    return true;
                }
            }
        }
        
        // グローバル変数のエイリアスチェック
        if self.global_aliases.in_same_set(var1, var2) {
            return true;
        }
        
        // 関数間のポインタ関係によるエイリアスチェック
        if let Some(relations1) = self.pointer_relations.get(&var1) {
            if relations1.contains(&var2) {
                return true;
            }
            
            // 間接的な関係もチェック
            for related in relations1 {
                if self.may_alias(*related, var2, func_id) {
                    return true;
                }
            }
        }
        
        if let Some(relations2) = self.pointer_relations.get(&var2) {
            if relations2.contains(&var1) {
                return true;
            }
            
            // 間接的な関係もチェック
            for related in relations2 {
                if self.may_alias(var1, *related, func_id) {
                    return true;
                }
            }
        }
        
        false
    }
    
    pub fn get_all_aliases(&self, var: VariableId, func_id: Option<FunctionId>) -> HashSet<VariableId> {
        let mut result = HashSet::new();
        result.insert(var); // 変数自身も含める
        
        // 関数内のローカル変数のエイリアス
        if let Some(func_id) = func_id {
            if let Some(alias_forest) = self.alias_sets.get(&func_id) {
                result.extend(alias_forest.get_aliases(var));
            }
        }
        
        // グローバル変数のエイリアス
        result.extend(self.global_aliases.get_aliases(var));
        
        // 関数間のポインタ関係によるエイリアス
        if let Some(relations) = self.pointer_relations.get(&var) {
            for related in relations {
                result.insert(*related);
                result.extend(self.get_all_aliases(*related, func_id));
            }
        }
        
        result
    }
}

impl AnalysisResult for AliasAnalysis {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    
    fn to_string(&self) -> String {
        format!(
            "AliasAnalysis {{ functions: {}, global_aliases: {}, execution_time: {:?} }}",
            self.alias_sets.len(),
            self.global_aliases.set_count(),
            self.execution_time
        )
    }
    
    fn dependencies(&self) -> Vec<AnalysisKind> {
        vec![AnalysisKind::DataFlow] // エイリアス解析はデータフロー解析に依存
    }
    
    fn validate(&self) -> Result<()> {
        // エイリアスセットの整合性チェック
        for (func_id, alias_forest) in &self.alias_sets {
            if alias_forest.validate().is_err() {
                return Err(Error::AnalysisError(format!(
                    "Alias set forest validation failed for function {:?}", func_id
                )));
            }
        }
        
        if self.global_aliases.validate().is_err() {
            return Err(Error::AnalysisError(
                "Global alias set forest validation failed".to_string()
            ));
        }
        
        // ポインタ関係の整合性チェック
        for (var_id, related_vars) in &self.pointer_relations {
            for related in related_vars {
                // 循環参照がないか確認（無限ループ防止）
                let mut visited = HashSet::new();
                visited.insert(*var_id);
                if Self::check_circular_reference(*related, &self.pointer_relations, &mut visited) {
                    return Err(Error::AnalysisError(format!(
                        "Circular pointer reference detected starting from variable {:?}", var_id
                    )));
                }
            }
        }
        
        Ok(())
    }
    
    fn memory_usage(&self) -> usize {
        let mut total = std::mem::size_of::<Self>();
        
        // エイリアスセットのメモリ使用量
        for alias_forest in self.alias_sets.values() {
            total += alias_forest.memory_usage();
        }
        
        // グローバルエイリアスのメモリ使用量
        total += self.global_aliases.memory_usage();
        
        // ポインタ関係のメモリ使用量
        for related_vars in self.pointer_relations.values() {
            total += std::mem::size_of::<HashSet<VariableId>>() + 
                     related_vars.len() * std::mem::size_of::<VariableId>();
        }
        
        total
    }
}

impl AliasAnalysis {
    // 循環参照チェックのヘルパーメソッド
    fn check_circular_reference(
        var_id: VariableId,
        pointer_relations: &HashMap<VariableId, HashSet<VariableId>>,
        visited: &mut HashSet<VariableId>
    ) -> bool {
        if visited.contains(&var_id) {
            return true; // 循環参照を検出
        }
        
        visited.insert(var_id);
        
        if let Some(related_vars) = pointer_relations.get(&var_id) {
            for related in related_vars {
                if Self::check_circular_reference(*related, pointer_relations, visited) {
                    return true;
                }
            }
        }
        
        visited.remove(&var_id);
        false
    }
}

/// メモリ依存性解析
/// 命令間のメモリ依存関係を分析し、並列化や最適化の機会を特定
pub struct MemoryDependencyAnalysis {
    // 関数ごとのメモリ依存グラフ
    dependency_graphs: HashMap<FunctionId, MemoryDependencyGraph>,
    // 命令間の依存関係タイプ
    dependency_types: HashMap<(InstructionId, InstructionId), DependencyType>,
    // 解析の実行時間
    execution_time: Duration,
}

impl MemoryDependencyAnalysis {
    fn run(module: &Module, dataflow: &DataFlowAnalysis, alias: &AliasAnalysis) -> Result<Self> {
        let start_time = Instant::now();
        
        let mut dependency_graphs = HashMap::new();
        let mut dependency_types = HashMap::new();
        
        // 各関数のメモリ依存関係を解析
        for func_id in module.functions.keys() {
            let (graph, types) = Self::analyze_function_dependencies(module, *func_id, dataflow, alias)?;
            dependency_graphs.insert(*func_id, graph);
            dependency_types.extend(types);
        }
        
        Ok(Self {
            dependency_graphs,
            dependency_types,
            execution_time: start_time.elapsed(),
        })
    }
    
    fn analyze_function_dependencies(
        module: &Module,
        func_id: FunctionId,
        dataflow: &DataFlowAnalysis,
        alias: &AliasAnalysis
    ) -> Result<(MemoryDependencyGraph, HashMap<(InstructionId, InstructionId), DependencyType>)> {
        let mut graph = MemoryDependencyGraph::new();
        let mut dep_types = HashMap::new();
        
        let func = &module.functions[&func_id];
        if let Some(body) = &func.body {
            // 命令IDのマッピングを作成
            let mut instr_ids = HashMap::new();
            for block_id in body.blocks() {
                let block = body.get_block(block_id).ok_or_else(|| {
                    Error::AnalysisError(format!("Block {:?} not found in function {:?}", block_id, func_id))
                })?;
                
                for (i, instr) in block.instructions.iter().enumerate() {
                    let instr_id = InstructionId::new(block_id, i as u32);
                    instr_ids.insert(instr_id, instr);
                    graph.add_node(instr_id);
                }
            }
            
            // 制御依存関係を追加
            Self::add_control_dependencies(body, &mut graph, &mut dep_types)?;
            
            // データ依存関係を追加
            Self::add_data_dependencies(body, dataflow, &mut graph, &mut dep_types)?;
            
            // メモリ依存関係を追加
            Self::add_memory_dependencies(body, alias, func_id, &instr_ids, &mut graph, &mut dep_types)?;
        }
        
        Ok((graph, dep_types))
    }
    
    fn add_control_dependencies(
        body: &FunctionBody,
        graph: &mut MemoryDependencyGraph,
        dep_types: &mut HashMap<(InstructionId, InstructionId), DependencyType>
    ) -> Result<()> {
        // 各ブロックの終了命令から後続ブロックの最初の命令への制御依存を追加
        for block_id in body.blocks() {
            let block = body.get_block(block_id).ok_or_else(|| {
                Error::AnalysisError(format!("Block {:?} not found", block_id))
            })?;
            
            if block.instructions.is_empty() {
                continue;
            }
            
            let terminator_idx = block.instructions.len() - 1;
            let terminator_id = InstructionId::new(block_id, terminator_idx as u32);
            
            // 後続ブロックを取得
            let successors = body.get_successors(block_id);
            for succ_id in successors {
                let succ_block = body.get_block(succ_id).ok_or_else(|| {
                    Error::AnalysisError(format!("Block {:?} not found", succ_id))
                })?;
                
                if !succ_block.instructions.is_empty() {
                    let first_instr_id = InstructionId::new(succ_id, 0);
                    graph.add_edge(terminator_id, first_instr_id);
                    dep_types.insert((terminator_id, first_instr_id), DependencyType::Control);
                }
            }
            
            // ブロック内の分岐命令に対する制御依存関係を追加
            if let Some(instr) = block.instructions.last() {
                if instr.is_branch() {
                    // 条件分岐の場合、条件式に依存する命令を追加
                    if let Some(condition) = instr.get_condition() {
                        for (i, instr) in block.instructions.iter().enumerate().take(terminator_idx) {
                            if instr.defines_value(condition) {
                                let def_id = InstructionId::new(block_id, i as u32);
                                graph.add_edge(def_id, terminator_id);
                                dep_types.insert((def_id, terminator_id), DependencyType::Data);
                            }
                        }
                    }
                    
                    // 分岐先ブロックの全命令に対する制御依存を追加
                    for succ_id in successors {
                        let succ_block = body.get_block(succ_id)?;
                        for (i, _) in succ_block.instructions.iter().enumerate() {
                            let succ_instr_id = InstructionId::new(succ_id, i as u32);
                            graph.add_edge(terminator_id, succ_instr_id);
                            dep_types.insert((terminator_id, succ_instr_id), DependencyType::Control);
                        }
                    }
                }
            }
            
            // ブロック内の命令間の制御依存関係を追加（例外処理など）
            for i in 0..block.instructions.len() {
                let instr_id = InstructionId::new(block_id, i as u32);
                let instr = &block.instructions[i];
                
                if instr.may_throw_exception() {
                    // 例外を投げる可能性のある命令の場合、例外ハンドラへの制御依存を追加
                    if let Some(handler_block_id) = body.get_exception_handler(block_id) {
                        let handler_block = body.get_block(handler_block_id)?;
                        if !handler_block.instructions.is_empty() {
                            let handler_first_id = InstructionId::new(handler_block_id, 0);
                            graph.add_edge(instr_id, handler_first_id);
                            dep_types.insert((instr_id, handler_first_id), DependencyType::Exception);
                        }
                    }
                }
                
                // 後続命令への制御フロー依存を追加
                if i < block.instructions.len() - 1 {
                    let next_id = InstructionId::new(block_id, (i + 1) as u32);
                    graph.add_edge(instr_id, next_id);
                    dep_types.insert((instr_id, next_id), DependencyType::Sequential);
                }
            }
        }
        
        // 支配木に基づく制御依存関係の計算
        let dom_tree = body.compute_dominator_tree()?;
        let post_dom_tree = body.compute_post_dominator_tree()?;
        
        // 制御依存関係グラフ（CDG）の構築
        for block_id in body.blocks() {
            let dominators = post_dom_tree.get_dominators(block_id);
            
            for succ_id in body.get_successors(block_id) {
                // ブロックがその後続を後支配していない場合、制御依存関係が存在する
                if !dominators.contains(&succ_id) {
                    let common_dominator = post_dom_tree.find_nearest_common_dominator(block_id, succ_id)?;
                    
                    // 共通支配点から後続ブロックまでのパス上のすべてのブロックは、
                    // 元のブロックに制御依存している
                    for path_block_id in post_dom_tree.get_path(common_dominator, succ_id)? {
                        if path_block_id != common_dominator {
                            let block = body.get_block(block_id)?;
                            let path_block = body.get_block(path_block_id)?;
                            
                            if !block.instructions.is_empty() && !path_block.instructions.is_empty() {
                                let term_id = InstructionId::new(block_id, (block.instructions.len() - 1) as u32);
                                
                                for (i, _) in path_block.instructions.iter().enumerate() {
                                    let path_instr_id = InstructionId::new(path_block_id, i as u32);
                                    graph.add_edge(term_id, path_instr_id);
                                    dep_types.insert((term_id, path_instr_id), DependencyType::Control);
                                }
                            }
                        }
                    }
                }
            }
        }
        
        Ok(())
    }
    
    fn add_data_dependencies(
        body: &FunctionBody,
        dataflow: &DataFlowAnalysis,
        graph: &mut MemoryDependencyGraph,
        dep_types: &mut HashMap<(InstructionId, InstructionId), DependencyType>
    ) -> Result<()> {
        // 各命令の定義と使用の関係を分析
        let def_use_chains = dataflow.get_def_use_chains();
        
        for (def_id, users) in def_use_chains {
            for use_id in users {
                graph.add_edge(def_id, use_id);
                dep_types.insert((def_id, use_id), DependencyType::Data);
            }
        }
        
        // PHI命令の特殊処理
        for block_id in body.blocks() {
            let block = body.get_block(block_id)?;
            
            for (i, instr) in block.instructions.iter().enumerate() {
                if instr.is_phi() {
                    let phi_id = InstructionId::new(block_id, i as u32);
                    
                    // PHI命令の各入力に対する依存関係を追加
                    for (pred_block_id, value) in instr.get_phi_inputs() {
                        // 前任ブロックの終了命令からPHI命令への制御依存を追加
                        let pred_block = body.get_block(pred_block_id)?;
                        if !pred_block.instructions.is_empty() {
                            let pred_term_idx = pred_block.instructions.len() - 1;
                            let pred_term_id = InstructionId::new(pred_block_id, pred_term_idx as u32);
                            
                            graph.add_edge(pred_term_id, phi_id);
                            dep_types.insert((pred_term_id, phi_id), DependencyType::Control);
                        }
                        
                        // 値の定義からPHI命令へのデータ依存を追加
                        if let Some(def_instr_id) = dataflow.get_definition(value) {
                            graph.add_edge(def_instr_id, phi_id);
                            dep_types.insert((def_instr_id, phi_id), DependencyType::Data);
                        }
                    }
                }
            }
        }
        
        Ok(())
    }
    
    fn add_memory_dependencies(
        body: &FunctionBody,
        alias: &AliasAnalysis,
        func_id: FunctionId,
        instr_ids: &HashMap<InstructionId, &Instruction>,
        graph: &mut MemoryDependencyGraph,
        dep_types: &mut HashMap<(InstructionId, InstructionId), DependencyType>
    ) -> Result<()> {
        // メモリアクセス命令（ロード/ストア）を収集
        let mut memory_accesses = Vec::new();
        
        for (&instr_id, instr) in instr_ids {
            if instr.is_load() || instr.is_store() {
                memory_accesses.push((instr_id, instr));
            }
        }
        
        // メモリアクセス間の依存関係を分析
        for i in 0..memory_accesses.len() {
            let (id_i, instr_i) = memory_accesses[i];
            let is_store_i = instr_i.is_store();
            let addr_i = instr_i.get_memory_address().ok_or_else(|| {
                Error::AnalysisError(format!("Failed to get memory address for instruction {:?}", id_i))
            })?;
            
            for j in i+1..memory_accesses.len() {
                let (id_j, instr_j) = memory_accesses[j];
                let is_store_j = instr_j.is_store();
                let addr_j = instr_j.get_memory_address().ok_or_else(|| {
                    Error::AnalysisError(format!("Failed to get memory address for instruction {:?}", id_j))
                })?;
                
                // アドレスのエイリアス関係を確認
                let alias_result = alias.may_alias(func_id, addr_i, addr_j);
                
                if alias_result {
                    // 依存関係のタイプを決定
                    // 1. WAW (Write After Write): 両方がストア
                    // 2. RAW (Read After Write): 最初がストア、次がロード
                    // 3. WAR (Write After Read): 最初がロード、次がストア
                    let dep_type = match (is_store_i, is_store_j) {
                        (true, true) => DependencyType::WriteAfterWrite,
                        (true, false) => DependencyType::ReadAfterWrite,
                        (false, true) => DependencyType::WriteAfterRead,
                        (false, false) => DependencyType::ReadAfterRead,
                    };
                    
                    // 依存関係をグラフに追加
                    graph.add_edge(id_i, id_j);
                    dep_types.insert((id_i, id_j), dep_type);
                }
            }
        }
        
        // バリア命令の処理
        for (&instr_id, instr) in instr_ids {
            if instr.is_memory_barrier() {
                // バリア命令の前のすべてのメモリアクセスからバリアへの依存を追加
                for &(access_id, _) in &memory_accesses {
                    if body.comes_before(access_id, instr_id) {
                        graph.add_edge(access_id, instr_id);
                        dep_types.insert((access_id, instr_id), DependencyType::MemoryBarrier);
                    }
                }
                
                // バリアからバリア後のすべてのメモリアクセスへの依存を追加
                for &(access_id, _) in &memory_accesses {
                    if body.comes_before(instr_id, access_id) {
                        graph.add_edge(instr_id, access_id);
                        dep_types.insert((instr_id, access_id), DependencyType::MemoryBarrier);
                    }
                }
            }
        }
        
        // 関数呼び出しの処理（副作用を持つ可能性がある）
        for (&instr_id, instr) in instr_ids {
            if instr.is_call() {
                let called_func = instr.get_called_function();
                
                // 呼び出された関数が純粋でない場合（副作用を持つ可能性がある）
                if !alias.is_pure_function(called_func) {
                    // 呼び出し前のすべてのメモリアクセスから呼び出しへの依存を追加
                    for &(access_id, _) in &memory_accesses {
                        if body.comes_before(access_id, instr_id) {
                            graph.add_edge(access_id, instr_id);
                            dep_types.insert((access_id, instr_id), DependencyType::FunctionCall);
                        }
                    }
                    
                    // 呼び出しから呼び出し後のすべてのメモリアクセスへの依存を追加
                    for &(access_id, _) in &memory_accesses {
                        if body.comes_before(instr_id, access_id) {
                            graph.add_edge(instr_id, access_id);
                            dep_types.insert((instr_id, access_id), DependencyType::FunctionCall);
                        }
                    }
                }
            }
        }
        
        Ok(())
    }
    
    fn run(module: &Module, dataflow: &DataFlowAnalysis, alias: &AliasAnalysis) -> Result<Self> {
        let start_time = std::time::Instant::now();
        let mut dependency_graphs = HashMap::new();
        let mut dependency_types = HashMap::new();
        
        // 各関数の依存関係グラフを構築
        for func_id in module.functions.keys() {
            let (graph, types) = Self::analyze_function_dependencies(module, *func_id, dataflow, alias)?;
            dependency_graphs.insert(*func_id, graph);
            dependency_types.extend(types);
        }
        
        Ok(Self {
            dependency_graphs,
            dependency_types,
            execution_time: start_time.elapsed(),
        })
    }
}

impl AnalysisResult for MemoryDependencyAnalysis {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    
    fn to_string(&self) -> String {
        todo!()
    }
    
    fn dependencies(&self) -> Vec<AnalysisKind> {
        todo!()
    }
    
    fn validate(&self) -> Result<()> {
        todo!()
    }
    
    fn memory_usage(&self) -> usize {
        todo!()
    }
}

/// 副作用解析
pub struct SideEffectAnalysis {
    // 関数IDから副作用情報へのマッピング
    function_effects: HashMap<FunctionId, EffectInfo>,
    // 式IDから副作用情報へのマッピング
    expression_effects: HashMap<ExpressionId, EffectInfo>,
    // 純粋関数（副作用なし）のセット
    pure_functions: HashSet<FunctionId>,
    // I/O操作を行う関数のセット
    io_functions: HashSet<FunctionId>,
    // メモリ変更を行う関数のセット
    memory_mutating_functions: HashSet<FunctionId>,
    // 例外を発生させる可能性のある関数のセット
    exception_raising_functions: HashSet<FunctionId>,
    // 関数間の副作用伝播グラフ
    effect_propagation_graph: HashMap<FunctionId, HashSet<FunctionId>>,
    // 最適化ヒント
    optimization_hints: HashMap<FunctionId, Vec<OptimizationHint>>,
    // 解析の実行時間
    execution_time: Duration,
}

impl SideEffectAnalysis {
    /// 副作用解析の実行
    /// 各関数や式が持つ副作用（メモリ書き込み、I/O操作、例外発生など）を特定する
    fn run(module: &Module, memdep: &MemoryDependencyAnalysis) -> Result<Self> {
        let start_time = std::time::Instant::now();
        
        // 関数IDから副作用情報へのマッピング
        let mut function_effects = HashMap::new();
        // 式IDから副作用情報へのマッピング
        let mut expression_effects = HashMap::new();
        // 純粋関数（副作用なし）のセット
        let mut pure_functions = HashSet::new();
        // I/O操作を行う関数のセット
        let mut io_functions = HashSet::new();
        // メモリ変更を行う関数のセット
        let mut memory_mutating_functions = HashSet::new();
        // 例外を発生させる可能性のある関数のセット
        let mut exception_raising_functions = HashSet::new();
        
        // 関数間の副作用伝播グラフ
        let mut effect_propagation_graph = HashMap::new();
        
        // 各関数の副作用を分析
        for (func_id, function) in module.functions() {
            let effects = Self::analyze_function_effects(module, *func_id, memdep, &effect_propagation_graph)?;
            
            // 副作用の種類に基づいて関数を分類
            if effects.is_pure() {
                pure_functions.insert(*func_id);
            }
            if effects.has_io_operations() {
                io_functions.insert(*func_id);
            }
            if effects.has_memory_mutations() {
                memory_mutating_functions.insert(*func_id);
            }
            if effects.may_raise_exceptions() {
                exception_raising_functions.insert(*func_id);
            }
            
            // 関数の副作用情報を記録
            function_effects.insert(*func_id, effects);
            
            // 関数内の各式の副作用を分析
            if let Some(body) = &function.body {
                let expr_effects = Self::analyze_expression_effects(module, body, memdep, &function_effects)?;
                expression_effects.extend(expr_effects);
            }
            
            // 副作用伝播グラフを更新
            Self::update_effect_propagation_graph(module, *func_id, &function_effects, &mut effect_propagation_graph)?;
        }
        
        // 副作用の伝播を固定点に達するまで繰り返し計算
        Self::propagate_effects_until_fixpoint(&mut function_effects, &effect_propagation_graph)?;
        
        // 最適化ヒントの生成
        let optimization_hints = Self::generate_optimization_hints(&function_effects, &pure_functions);
        
        Ok(Self {
            function_effects,
            expression_effects,
            pure_functions,
            io_functions,
            memory_mutating_functions,
            exception_raising_functions,
            effect_propagation_graph,
            optimization_hints,
            execution_time: start_time.elapsed(),
        })
    }
    
    /// 関数の副作用を分析する
    fn analyze_function_effects(
        module: &Module,
        func_id: FunctionId,
        memdep: &MemoryDependencyAnalysis,
        effect_graph: &HashMap<FunctionId, HashSet<FunctionId>>
    ) -> Result<EffectInfo> {
        let function = module.get_function(func_id)
            .ok_or_else(|| Error::new(ErrorKind::AnalysisError, format!("関数ID {}が見つかりません", func_id)))?;
        
        let mut effect_info = EffectInfo::new();
        
        // 関数本体がない場合（外部関数など）は保守的に副作用ありと判断
        if function.body.is_none() {
            // 外部関数の場合、アノテーションや既知の関数特性から副作用を推定
            if let Some(annotations) = &function.annotations {
                if annotations.contains("pure") {
                    effect_info.set_pure(true);
                    return Ok(effect_info);
                }
                if annotations.contains("io") {
                    effect_info.add_io_operations();
                }
                if annotations.contains("memory_write") {
                    effect_info.add_memory_mutations();
                }
                if annotations.contains("may_throw") {
                    effect_info.add_exception_possibility();
                }
            } else {
                // アノテーションがない場合は保守的に全ての副作用の可能性ありとする
                effect_info.add_io_operations();
                effect_info.add_memory_mutations();
                effect_info.add_exception_possibility();
            }
            return Ok(effect_info);
        }
        
        // 関数本体の分析
        if let Some(body) = &function.body {
            // メモリ依存性解析の結果を利用してメモリ書き込み操作を検出
            let memory_deps = memdep.get_function_dependencies(func_id)
                .ok_or_else(|| Error::new(ErrorKind::AnalysisError, "メモリ依存性情報が見つかりません"))?;
            
            if !memory_deps.writes.is_empty() {
                effect_info.add_memory_mutations();
            }
            
            // 関数内の各式を再帰的に分析
            Self::analyze_body_effects(module, body, &mut effect_info, memdep, effect_graph)?;
        }
        
        Ok(effect_info)
    }
    
    /// 関数本体の副作用を再帰的に分析
    fn analyze_body_effects(
        module: &Module,
        expr: &Expression,
        effect_info: &mut EffectInfo,
        memdep: &MemoryDependencyAnalysis,
        effect_graph: &HashMap<FunctionId, HashSet<FunctionId>>
    ) -> Result<()> {
        match &expr.kind {
            ExpressionKind::Call { function, arguments } => {
                // 関数呼び出しの副作用を分析
                if let ExpressionKind::FunctionRef { id } = &function.kind {
                    // 呼び出し先関数の副作用を伝播
                    if let Some(callee_effects) = effect_graph.get(id) {
                        for callee_id in callee_effects {
                            if let Some(callee_function) = module.get_function(*callee_id) {
                                // 呼び出し先の副作用を現在の関数に伝播
                                if callee_function.has_io_operations() {
                                    effect_info.add_io_operations();
                                }
                                if callee_function.has_memory_mutations() {
                                    effect_info.add_memory_mutations();
                                }
                                if callee_function.may_raise_exceptions() {
                                    effect_info.add_exception_possibility();
                                }
                            }
                        }
                    }
                }
                
                // 引数の副作用も分析
                for arg in arguments {
                    Self::analyze_body_effects(module, arg, effect_info, memdep, effect_graph)?;
                }
            },
            ExpressionKind::Assignment { target, value } => {
                // 代入操作はメモリ変更の副作用
                effect_info.add_memory_mutations();
                
                // 代入値の副作用も分析
                Self::analyze_body_effects(module, value, effect_info, memdep, effect_graph)?;
            },
            ExpressionKind::MethodCall { receiver, method, arguments } => {
                // メソッド呼び出しは潜在的に副作用を持つ
                effect_info.add_memory_mutations();
                
                // レシーバと引数の副作用も分析
                Self::analyze_body_effects(module, receiver, effect_info, memdep, effect_graph)?;
                for arg in arguments {
                    Self::analyze_body_effects(module, arg, effect_info, memdep, effect_graph)?;
                }
            },
            ExpressionKind::If { condition, then_branch, else_branch } => {
                // 条件式と各分岐の副作用を分析
                Self::analyze_body_effects(module, condition, effect_info, memdep, effect_graph)?;
                Self::analyze_body_effects(module, then_branch, effect_info, memdep, effect_graph)?;
                if let Some(else_expr) = else_branch {
                    Self::analyze_body_effects(module, else_expr, effect_info, memdep, effect_graph)?;
                }
            },
            ExpressionKind::Loop { body, condition } => {
                // ループ本体と条件の副作用を分析
                Self::analyze_body_effects(module, body, effect_info, memdep, effect_graph)?;
                if let Some(cond) = condition {
                    Self::analyze_body_effects(module, cond, effect_info, memdep, effect_graph)?;
                }
            },
            ExpressionKind::Block { statements, result } => {
                // ブロック内の各文と結果式の副作用を分析
                for stmt in statements {
                    Self::analyze_body_effects(module, stmt, effect_info, memdep, effect_graph)?;
                }
                if let Some(res) = result {
                    Self::analyze_body_effects(module, res, effect_info, memdep, effect_graph)?;
                }
            },
            ExpressionKind::Try { body, catch_blocks } => {
                // try式は例外発生の可能性を示す
                effect_info.add_exception_possibility();
                
                // try本体とcatchブロックの副作用を分析
                Self::analyze_body_effects(module, body, effect_info, memdep, effect_graph)?;
                for catch in catch_blocks {
                    Self::analyze_body_effects(module, &catch.handler, effect_info, memdep, effect_graph)?;
                }
            },
            ExpressionKind::Throw { .. } => {
                // throw式は例外発生の副作用
                effect_info.add_exception_possibility();
            },
            ExpressionKind::IOOperation { .. } => {
                // I/O操作は明示的なI/O副作用
                effect_info.add_io_operations();
            },
            // その他の式タイプも適切に処理
            _ => {
                // 式の子要素を再帰的に分析
                for child in expr.children() {
                    Self::analyze_body_effects(module, child, effect_info, memdep, effect_graph)?;
                }
            }
        }
        
        Ok(())
    }
    
    /// 式レベルの副作用を分析
    fn analyze_expression_effects(
        module: &Module,
        expr: &Expression,
        memdep: &MemoryDependencyAnalysis,
        function_effects: &HashMap<FunctionId, EffectInfo>
    ) -> Result<HashMap<ExpressionId, EffectInfo>> {
        let mut expr_effects = HashMap::new();
        Self::analyze_expr_recursive(module, expr, memdep, function_effects, &mut expr_effects)?;
        Ok(expr_effects)
    }
    
    /// 式を再帰的に分析して副作用情報を収集
    fn analyze_expr_recursive(
        module: &Module,
        expr: &Expression,
        memdep: &MemoryDependencyAnalysis,
        function_effects: &HashMap<FunctionId, EffectInfo>,
        expr_effects: &mut HashMap<ExpressionId, EffectInfo>
    ) -> Result<EffectInfo> {
        // 既に分析済みの式はキャッシュから返す
        if let Some(effects) = expr_effects.get(&expr.id) {
            return Ok(effects.clone());
        }
        
        let mut effect_info = EffectInfo::new();
        
        match &expr.kind {
            // 各式タイプに応じた副作用分析ロジック
            // （前述のanalyze_body_effectsと同様のロジックだが、結果を返す）
            // ...
            
            // 簡略化のため省略
        }
        
        // 分析結果をキャッシュして返す
        expr_effects.insert(expr.id, effect_info.clone());
        Ok(effect_info)
    }
    
    /// 副作用伝播グラフを更新
    fn update_effect_propagation_graph(
        module: &Module,
        func_id: FunctionId,
        function_effects: &HashMap<FunctionId, EffectInfo>,
        effect_graph: &mut HashMap<FunctionId, HashSet<FunctionId>>
    ) -> Result<()> {
        let function = module.get_function(func_id)
            .ok_or_else(|| Error::new(ErrorKind::AnalysisError, format!("関数ID {}が見つかりません", func_id)))?;
        
        let mut callees = HashSet::new();
        
        // 関数本体から呼び出し先関数を特定
        if let Some(body) = &function.body {
            Self::collect_function_calls(body, &mut callees);
        }
        
        effect_graph.insert(func_id, callees);
        Ok(())
    }
    
    /// 式から関数呼び出しを収集
    fn collect_function_calls(expr: &Expression, callees: &mut HashSet<FunctionId>) {
        match &expr.kind {
            ExpressionKind::Call { function, .. } => {
                if let ExpressionKind::FunctionRef { id } = &function.kind {
                    callees.insert(*id);
                }
            },
            // その他の式タイプも再帰的に処理
            _ => {
                for child in expr.children() {
                    Self::collect_function_calls(child, callees);
                }
            }
        }
    }
    
    /// 副作用を固定点に達するまで伝播
    fn propagate_effects_until_fixpoint(
        function_effects: &mut HashMap<FunctionId, EffectInfo>,
        effect_graph: &HashMap<FunctionId, HashSet<FunctionId>>
    ) -> Result<()> {
        let mut changed = true;
        
        // 変更がなくなるまで繰り返す
        while changed {
            changed = false;
            
            // 各関数の副作用を伝播
            let effects_copy = function_effects.clone();
            
            for (func_id, callees) in effect_graph {
                let mut updated_effects = effects_copy.get(func_id).cloned().unwrap_or_default();
                
                // 呼び出し先の副作用を現在の関数に伝播
                for callee_id in callees {
                    if let Some(callee_effects) = effects_copy.get(callee_id) {
                        if callee_effects.has_io_operations() && !updated_effects.has_io_operations() {
                            updated_effects.add_io_operations();
                            changed = true;
                        }
                        if callee_effects.has_memory_mutations() && !updated_effects.has_memory_mutations() {
                            updated_effects.add_memory_mutations();
                            changed = true;
                        }
                        if callee_effects.may_raise_exceptions() && !updated_effects.may_raise_exceptions() {
                            updated_effects.add_exception_possibility();
                            changed = true;
                        }
                    }
                }
                
                // 変更があれば更新
                if changed {
                    function_effects.insert(*func_id, updated_effects);
                }
            }
        }
        
        Ok(())
    }
    
    /// 最適化ヒントを生成
    fn generate_optimization_hints(
        function_effects: &HashMap<FunctionId, EffectInfo>,
        pure_functions: &HashSet<FunctionId>
    ) -> HashMap<FunctionId, Vec<OptimizationHint>> {
        let mut hints = HashMap::new();
        
        for (func_id, effects) in function_effects {
            let mut function_hints = Vec::new();
            
            // 純粋関数は様々な最適化が可能
            if pure_functions.contains(func_id) {
                function_hints.push(OptimizationHint::Memoizable);
                function_hints.push(OptimizationHint::Parallelizable);
                function_hints.push(OptimizationHint::Reorderable);
                function_hints.push(OptimizationHint::ConstantFoldable);
            }
            
            // I/O操作のない関数は特定の最適化が可能
            if !effects.has_io_operations() {
                function_hints.push(OptimizationHint::IOFree);
            }
            
            // メモリ変更のない関数は特定の最適化が可能
            if !effects.has_memory_mutations() {
                function_hints.push(OptimizationHint::MemoryReadOnly);
            }
            
            // 例外を発生させない関数は特定の最適化が可能
            if !effects.may_raise_exceptions() {
                function_hints.push(OptimizationHint::NoExceptions);
            }
            
            hints.insert(*func_id, function_hints);
        }
        
        hints
    }
    
    /// 関数が純粋かどうかを判定
    pub fn is_pure_function(&self, func_id: FunctionId) -> bool {
        self.pure_functions.contains(&func_id)
    }
    
    /// 関数がI/O操作を行うかどうかを判定
    pub fn has_io_operations(&self, func_id: FunctionId) -> bool {
        self.io_functions.contains(&func_id)
    }
    
    /// 関数がメモリ変更を行うかどうかを判定
    pub fn has_memory_mutations(&self, func_id: FunctionId) -> bool {
        self.memory_mutating_functions.contains(&func_id)
    }
    
    /// 関数が例外を発生させる可能性があるかどうかを判定
    pub fn may_raise_exceptions(&self, func_id: FunctionId) -> bool {
        self.exception_raising_functions.contains(&func_id)
    }
    
    /// 関数の最適化ヒントを取得
    pub fn get_optimization_hints(&self, func_id: FunctionId) -> Option<&Vec<OptimizationHint>> {
        self.optimization_hints.get(&func_id)
    }
    
    /// 式の副作用情報を取得
    pub fn get_expression_effects(&self, expr_id: ExpressionId) -> Option<&EffectInfo> {
        self.expression_effects.get(&expr_id)
    }
}

/// 副作用情報を表す構造体
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct EffectInfo {
    /// I/O操作の有無
    io_operations: bool,
    /// メモリ変更の有無
    memory_mutations: bool,
    /// 例外発生の可能性
    exception_possibility: bool,
    /// 純粋関数かどうか
    pure: bool,
}

impl EffectInfo {
    fn new() -> Self {
        Self {
            io_operations: false,
            memory_mutations: false,
            exception_possibility: false,
            pure: true, // デフォルトは純粋と仮定
        }
    }
    
    fn add_io_operations(&mut self) {
        self.io_operations = true;
        self.pure = false;
    }
    
    fn add_memory_mutations(&mut self) {
        self.memory_mutations = true;
        self.pure = false;
    }
    
    fn add_exception_possibility(&mut self) {
        self.exception_possibility = true;
    }
    
    fn set_pure(&mut self, value: bool) {
        self.pure = value;
        if value {
            self.io_operations = false;
            self.memory_mutations = false;
        }
    }
    
    fn is_pure(&self) -> bool {
        self.pure
    }
    
    fn has_io_operations(&self) -> bool {
        self.io_operations
    }
    
    fn has_memory_mutations(&self) -> bool {
        self.memory_mutations
    }
    
    fn may_raise_exceptions(&self) -> bool {
        self.exception_possibility
    }
}

/// 最適化ヒントの種類
#[derive(Clone, Debug, PartialEq, Eq)]
enum OptimizationHint {
    /// メモ化可能（同じ入力に対して常に同じ出力を返す）
    Memoizable,
    /// 並列化可能（副作用がないため並列実行可能）
    Parallelizable,
    /// 実行順序変更可能（他のコードとの実行順序を変更可能）
    Reorderable,
    /// 定数畳み込み可能（コンパイル時に評価可能）
    ConstantFoldable,
    /// I/O操作なし
    IOFree,
    /// メモリ読み取りのみ（書き込みなし）
    MemoryReadOnly,
    /// 例外発生なし
    NoExceptions,
}

impl AnalysisResult for SideEffectAnalysis {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// リーチ可能性解析
/// プログラム内の到達可能なコードパスを特定する解析
pub struct ReachabilityAnalysis {
    /// 関数IDから到達可能なブロックのセットへのマッピング
    reachable_blocks: HashMap<FunctionId, HashSet<BlockId>>,
    /// 到達不可能と判断されたブロックのセット
    unreachable_blocks: HashSet<BlockId>,
    /// 関数間の呼び出しグラフ
    call_graph: HashMap<FunctionId, HashSet<FunctionId>>,
    /// 到達可能な関数のセット
    reachable_functions: HashSet<FunctionId>,
}

impl ReachabilityAnalysis {
    fn run(module: &Module, cf: &ControlFlowAnalysis) -> Result<Self> {
        let mut analysis = Self {
            reachable_blocks: HashMap::new(),
            unreachable_blocks: HashSet::new(),
            call_graph: HashMap::new(),
            reachable_functions: HashSet::new(),
        };
        
        // 呼び出しグラフの構築
        analysis.build_call_graph(module)?;
        
        // エントリポイントから到達可能な関数を特定
        analysis.compute_reachable_functions(module)?;
        
        // 各関数内の到達可能なブロックを特定
        for function_id in &analysis.reachable_functions {
            if let Some(function) = module.get_function(*function_id) {
                if let Some(body) = &function.body {
                    analysis.compute_reachable_blocks(function_id, body, cf)?;
                }
            }
        }
        
        Ok(analysis)
    }
    
    /// モジュール内の関数間の呼び出し関係を解析して呼び出しグラフを構築
    fn build_call_graph(&mut self, module: &Module) -> Result<()> {
        for (function_id, function) in module.functions() {
            let mut callees = HashSet::new();
            
            if let Some(body) = &function.body {
                for block_id in body.blocks() {
                    let block = body.get_block(block_id).ok_or_else(|| {
                        Error::AnalysisError(format!("ブロック {:?} が見つかりません", block_id))
                    })?;
                    
                    for (instr_idx, instr) in block.instructions.iter().enumerate() {
                        if let Instruction::Call { function: callee_id, .. } = instr {
                            callees.insert(*callee_id);
                        } else if let Instruction::IndirectCall { .. } = instr {
                            // 間接呼び出しの場合、型情報と別名解析を使用して可能な呼び出し先を特定
                            // 保守的に対応するため、現時点では特定の型の関数をすべて候補とする
                            let instr_id = InstructionId::new(block_id, instr_idx as u32);
                            self.handle_indirect_call(module, instr_id, instr, &mut callees)?;
                        }
                    }
                }
            }
            
            self.call_graph.insert(function_id, callees);
        }
        
        Ok(())
    }
    
    /// 間接呼び出しの可能な呼び出し先を解析
    fn handle_indirect_call(
        &self,
        module: &Module,
        instr_id: InstructionId,
        instr: &Instruction,
        callees: &mut HashSet<FunctionId>
    ) -> Result<()> {
        if let Instruction::IndirectCall { function_type, .. } = instr {
            // 指定された型と互換性のある関数をすべて候補とする
            for (func_id, func) in module.functions() {
                if func.type_signature.is_compatible_with(function_type) {
                    callees.insert(func_id);
                }
            }
        }
        
        Ok(())
    }
    
    /// エントリポイントから到達可能な関数を特定
    fn compute_reachable_functions(&mut self, module: &Module) -> Result<()> {
        let mut worklist = VecDeque::new();
        
        // エントリポイント関数を特定してワークリストに追加
        if let Some(entry_id) = module.entry_point {
            worklist.push_back(entry_id);
            self.reachable_functions.insert(entry_id);
        } else {
            // エントリポイントが指定されていない場合、外部から呼び出される可能性のある関数をすべて追加
            for (func_id, func) in module.functions() {
                if func.linkage.is_externally_visible() {
                    worklist.push_back(func_id);
                    self.reachable_functions.insert(func_id);
                }
            }
        }
        
        // 到達可能な関数を幅優先探索で特定
        while let Some(current_id) = worklist.pop_front() {
            if let Some(callees) = self.call_graph.get(&current_id) {
                for callee_id in callees {
                    if !self.reachable_functions.contains(callee_id) {
                        self.reachable_functions.insert(*callee_id);
                        worklist.push_back(*callee_id);
                    }
                }
            }
        }
        
        Ok(())
    }
    
    /// 関数内の到達可能なブロックを特定
    fn compute_reachable_blocks(
        &mut self,
        function_id: &FunctionId,
        body: &FunctionBody,
        cf: &ControlFlowAnalysis
    ) -> Result<()> {
        let mut reachable = HashSet::new();
        let mut worklist = VecDeque::new();
        
        // エントリブロックをワークリストに追加
        if let Some(entry_block) = body.entry_block() {
            worklist.push_back(entry_block);
            reachable.insert(entry_block);
        } else {
            return Err(Error::AnalysisError(format!(
                "関数 {:?} にエントリブロックがありません", function_id
            )));
        }
        
        // 到達可能なブロックを幅優先探索で特定
        while let Some(block_id) = worklist.pop_front() {
            // 後続ブロックを取得
            let successors = body.get_successors(block_id);
            for succ_id in successors {
                if !reachable.contains(&succ_id) {
                    reachable.insert(succ_id);
                    worklist.push_back(succ_id);
                }
            }
        }
        
        // 到達不可能なブロックを特定
        for block_id in body.blocks() {
            if !reachable.contains(&block_id) {
                self.unreachable_blocks.insert(block_id);
            }
        }
        
        self.reachable_blocks.insert(*function_id, reachable);
        Ok(())
    }
    
    /// 指定されたブロックが到達可能かどうかを判定
    pub fn is_block_reachable(&self, function_id: FunctionId, block_id: BlockId) -> bool {
        if let Some(reachable) = self.reachable_blocks.get(&function_id) {
            reachable.contains(&block_id)
        } else {
            false
        }
    }
    
    /// 指定された関数が到達可能かどうかを判定
    pub fn is_function_reachable(&self, function_id: FunctionId) -> bool {
        self.reachable_functions.contains(&function_id)
    }
    
    /// 到達不可能なブロックの一覧を取得
    pub fn get_unreachable_blocks(&self) -> &HashSet<BlockId> {
        &self.unreachable_blocks
    }
    
    /// 呼び出しグラフを取得
    pub fn get_call_graph(&self) -> &HashMap<FunctionId, HashSet<FunctionId>> {
        &self.call_graph
    }
}

impl AnalysisResult for ReachabilityAnalysis {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    
    fn to_string(&self) -> String {
        let mut result = String::from("リーチ可能性解析結果:\n");
        
        // 到達可能な関数の数
        result.push_str(&format!("到達可能な関数数: {}\n", self.reachable_functions.len()));
        
        // 到達不可能なブロックの数
        result.push_str(&format!("到達不可能なブロック数: {}\n", self.unreachable_blocks.len()));
        
        // 関数ごとの到達可能なブロック数
        result.push_str("関数ごとの到達可能なブロック数:\n");
        for (func_id, blocks) in &self.reachable_blocks {
            result.push_str(&format!("  関数 {:?}: {} ブロック\n", func_id, blocks.len()));
        }
        
        // 呼び出しグラフの概要
        result.push_str("呼び出しグラフ概要:\n");
        for (caller, callees) in &self.call_graph {
            result.push_str(&format!("  関数 {:?} から {} 関数を呼び出し\n", caller, callees.len()));
        }
        
        result
    }
    
    fn dependencies(&self) -> Vec<AnalysisKind> {
        // この解析が依存する他の解析を返す
        vec![AnalysisKind::ControlFlow]
    }
    
    fn validate(&self) -> Result<()> {
        // 解析結果の整合性を検証
        
        // 1. すべての到達可能な関数には少なくとも1つの到達可能なブロックがあるべき
        for func_id in &self.reachable_functions {
            if let Some(blocks) = self.reachable_blocks.get(func_id) {
                if blocks.is_empty() {
                    return Err(Error::AnalysisError(format!(
                        "到達可能な関数 {:?} に到達可能なブロックがありません", func_id
                    )));
                }
            } else {
                return Err(Error::AnalysisError(format!(
                    "到達可能な関数 {:?} の到達可能なブロック情報がありません", func_id
                )));
            }
        }
        
        // 2. 到達不可能なブロックは、どの関数の到達可能なブロックセットにも含まれていないことを確認
        for block_id in &self.unreachable_blocks {
            for (func_id, blocks) in &self.reachable_blocks {
                if blocks.contains(block_id) {
                    return Err(Error::AnalysisError(format!(
                        "ブロック {:?} は到達不可能としてマークされていますが、関数 {:?} の到達可能なブロックセットに含まれています",
                        block_id, func_id
                    )));
                }
            }
        }
        
        // 3. 呼び出しグラフの整合性を検証
        for (caller, callees) in &self.call_graph {
            for callee in callees {
                // 到達可能な関数から呼び出される関数も到達可能であるべき
                if self.reachable_functions.contains(caller) && !self.reachable_functions.contains(callee) {
                    return Err(Error::AnalysisError(format!(
                        "到達可能な関数 {:?} から呼び出される関数 {:?} が到達不可能としてマークされています",
                        caller, callee
                    )));
                }
            }
        }
        
        Ok(())
    }
    
    fn memory_usage(&self) -> usize {
        // 解析結果が使用しているメモリ量を計算
        let mut total_size = std::mem::size_of::<Self>();
        
        // reachable_blocks のメモリ使用量
        for (_, blocks) in &self.reachable_blocks {
            total_size += std::mem::size_of::<FunctionId>() + std::mem::size_of::<HashSet<BlockId>>()
                + blocks.len() * std::mem::size_of::<BlockId>();
        }
        
        // unreachable_blocks のメモリ使用量
        total_size += std::mem::size_of::<HashSet<BlockId>>()
            + self.unreachable_blocks.len() * std::mem::size_of::<BlockId>();
        
        // call_graph のメモリ使用量
        for (_, callees) in &self.call_graph {
            total_size += std::mem::size_of::<FunctionId>() + std::mem::size_of::<HashSet<FunctionId>>()
                + callees.len() * std::mem::size_of::<FunctionId>();
        }
        
        // reachable_functions のメモリ使用量
        total_size += std::mem::size_of::<HashSet<FunctionId>>()
            + self.reachable_functions.len() * std::mem::size_of::<FunctionId>();
        
        total_size
    }
}

/// 不変条件解析
/// ループや条件分岐内で変化しない値や式を特定する解析
pub struct InvariantAnalysis {
    /// ループIDから不変な命令IDのセットへのマッピング
    loop_invariants: HashMap<LoopId, HashSet<InstructionId>>,
    /// ブロックIDから不変な命令IDのセットへのマッピング
    block_invariants: HashMap<BlockId, HashSet<InstructionId>>,
    /// 関数全体で不変な命令IDのセット
    function_invariants: HashMap<FunctionId, HashSet<InstructionId>>,
    /// 不変条件の種類（ループ不変、ブロック不変、関数不変）
    invariant_types: HashMap<InstructionId, InvariantType>,
}

/// 不変条件の種類
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvariantType {
    /// ループ内で不変
    LoopInvariant(LoopId),
    /// ブロック内で不変
    BlockInvariant(BlockId),
    /// 関数全体で不変
    FunctionInvariant(FunctionId),
}

impl InvariantAnalysis {
    fn run(module: &Module, dataflow: &DataFlowAnalysis, cf: &ControlFlowAnalysis) -> Result<Self> {
        let mut analysis = Self {
            loop_invariants: HashMap::new(),
            block_invariants: HashMap::new(),
            function_invariants: HashMap::new(),
            invariant_types: HashMap::new(),
        };
        
        // 各関数に対して不変条件解析を実行
        for (function_id, function) in module.functions() {
            if let Some(body) = &function.body {
                analysis.analyze_function(function_id, body, dataflow, cf)?;
            }
        }
        
        Ok(analysis)
    }
    
    /// 関数内の不変条件を解析
    fn analyze_function(
        &mut self,
        function_id: FunctionId,
        body: &FunctionBody,
        dataflow: &DataFlowAnalysis,
        cf: &ControlFlowAnalysis
    ) -> Result<()> {
        // 関数内のループを特定
        let loops = cf.get_loops(function_id)?;
        
        // 関数全体の不変条件を特定
        let mut function_invariants = HashSet::new();
        self.identify_function_invariants(function_id, body, dataflow, &mut function_invariants)?;
        self.function_invariants.insert(function_id, function_invariants.clone());
        
        // 各ループの不変条件を特定
        for loop_id in loops.get_all_loops() {
            let mut loop_invariants = HashSet::new();
            self.identify_loop_invariants(loop_id, body, dataflow, &mut loop_invariants)?;
            
            // ループ不変命令を記録
            for instr_id in &loop_invariants {
                self.invariant_types.insert(*instr_id, InvariantType::LoopInvariant(loop_id));
            }
            
            self.loop_invariants.insert(loop_id, loop_invariants);
        }
        
        // 各ブロックの不変条件を特定
        for block_id in body.blocks() {
            let mut block_invariants = HashSet::new();
            self.identify_block_invariants(block_id, body, dataflow, &mut block_invariants)?;
            
            // ブロック不変命令を記録
            for instr_id in &block_invariants {
                // すでにループ不変として記録されている場合は上書きしない
                if !self.invariant_types.contains_key(instr_id) {
                    self.invariant_types.insert(*instr_id, InvariantType::BlockInvariant(block_id));
                }
            }
            
            self.block_invariants.insert(block_id, block_invariants);
        }
        
        // 関数不変命令を記録
        for instr_id in &function_invariants {
            // すでに他の不変タイプとして記録されている場合は上書きしない
            if !self.invariant_types.contains_key(instr_id) {
                self.invariant_types.insert(*instr_id, InvariantType::FunctionInvariant(function_id));
            }
        }
        
        Ok(())
    }
    
    /// 関数全体で不変な命令を特定
    fn identify_function_invariants(
        &self,
        function_id: FunctionId,
        body: &FunctionBody,
        dataflow: &DataFlowAnalysis,
        invariants: &mut HashSet<InstructionId>
    ) -> Result<()> {
        // 定数値や関数引数のみに依存する命令を特定
        for block_id in body.blocks() {
            let block = body.get_block(block_id).ok_or_else(|| {
                Error::AnalysisError(format!("ブロック {:?} が見つかりません", block_id))
            })?;
            
            for (instr_idx, instr) in block.instructions.iter().enumerate() {
                let instr_id = InstructionId::new(block_id, instr_idx as u32);
                
                // 命令が関数不変かどうかを判定
                if self.is_function_invariant(instr_id, instr, body, dataflow)? {
                    invariants.insert(instr_id);
                }
            }
        }
        
        Ok(())
    }
    
    /// 命令が関数全体で不変かどうかを判定
    fn is_function_invariant(
        &self,
        instr_id: InstructionId,
        instr: &Instruction,
        body: &FunctionBody,
        dataflow: &DataFlowAnalysis
    ) -> Result<bool> {
        // 定数命令は常に不変
        if matches!(instr, Instruction::Constant { .. }) {
            return Ok(true);
        }
        
        // 関数引数の参照は常に不変
        if matches!(instr, Instruction::GetParam { .. }) {
            return Ok(true);
        }
        
        // 副作用のある命令は不変ではない
        if instr.has_side_effects() {
            return Ok(false);
        }
        
        // 命令のオペランドがすべて不変な場合、その命令も不変
        let operands = instr.get_operands();
        for operand in operands {
            if let Some(def_instr) = dataflow.get_reaching_def(instr_id, operand)? {
                let def_block = def_instr.block();
                let def_idx = def_instr.index();
                let def_instr_ref = body.get_instruction(def_block, def_idx as usize).ok_or_else(|| {
                    Error::AnalysisError(format!("命令 {:?} が見つかりません", def_instr))
                })?;
                
                // 再帰的に依存命令が不変かどうかを確認
                if !self.is_function_invariant(def_instr, def_instr_ref, body, dataflow)? {
                    return Ok(false);
                }
            } else {
                // 定義が見つからない場合は外部からの入力と見なし、不変ではない
                return Ok(false);
            }
        }
        
        Ok(true)
    }
    
    /// ループ内で不変な命令を特定
    fn identify_loop_invariants(
        &self,
        loop_id: LoopId,
        body: &FunctionBody,
        dataflow: &DataFlowAnalysis,
        invariants: &mut HashSet<InstructionId>
    ) -> Result<()> {
        // ループのヘッダーブロックとボディブロックを取得
        let loop_info = dataflow.get_loop_info(loop_id)?;
        let header = loop_info.header;
        let body_blocks = loop_info.body;
        
        // ループ内の各ブロックの各命令を調査
        for block_id in &body_blocks {
            let block = body.get_block(*block_id).ok_or_else(|| {
                Error::AnalysisError(format!("ブロック {:?} が見つかりません", block_id))
            })?;
            
            for (instr_idx, instr) in block.instructions.iter().enumerate() {
                let instr_id = InstructionId::new(*block_id, instr_idx as u32);
                
                // 命令がループ不変かどうかを判定
                if self.is_loop_invariant(instr_id, instr, loop_id, body, dataflow)? {
                    invariants.insert(instr_id);
                }
            }
        }
        
        Ok(())
    }
    
    /// 命令がループ内で不変かどうかを判定
    fn is_loop_invariant(
        &self,
        instr_id: InstructionId,
        instr: &Instruction,
        loop_id: LoopId,
        body: &FunctionBody,
        dataflow: &DataFlowAnalysis
    ) -> Result<bool> {
        // 定数命令は常に不変
        if matches!(instr, Instruction::Constant { .. }) {
            return Ok(true);
        }
        
        // 副作用のある命令は不変ではない
        if instr.has_side_effects() {
            return Ok(false);
        }
        
        // ループ内で定義される変数への書き込みは不変ではない
        if instr.is_memory_write() {
            return Ok(false);
        }
        
        // ループの外部で定義され、ループ内で再定義されない値を使用する命令は不変
        let loop_info = dataflow.get_loop_info(loop_id)?;
        let loop_blocks = loop_info.body;
        
        // 命令のオペランドがすべてループ不変な場合、その命令も不変
        let operands = instr.get_operands();
        for operand in operands {
            if let Some(def_instr) = dataflow.get_reaching_def(instr_id, operand)? {
                let def_block = def_instr.block();
                
                // 定義がループ外にある場合は不変
                if !loop_blocks.contains(&def_block) {
                    continue;
                }
                
                // 定義がループ内にある場合、その定義自体が不変かどうかを再帰的に確認
                let def_idx = def_instr.index();
                let def_instr_ref = body.get_instruction(def_block, def_idx as usize).ok_or_else(|| {
                    Error::AnalysisError(format!("命令 {:?} が見つかりません", def_instr))
                })?;
                
                if !self.is_loop_invariant(def_instr, def_instr_ref, loop_id, body, dataflow)? {
                    return Ok(false);
                }
            } else {
                // 定義が見つからない場合は外部からの入力と見なし、不変と見なす
                continue;
            }
        }
        
        Ok(true)
    }
    
    /// ブロック内で不変な命令を特定
    fn identify_block_invariants(
        &self,
        block_id: BlockId,
        body: &FunctionBody,
        dataflow: &DataFlowAnalysis,
        invariants: &mut HashSet<InstructionId>
    ) -> Result<()> {
        let block = body.get_block(block_id).ok_or_else(|| {
            Error::AnalysisError(format!("ブロック {:?} が見つかりません", block_id))
        })?;
        
        // ブロック内の各命令を調査
        for (instr_idx, instr) in block.instructions.iter().enumerate() {
            let instr_id = InstructionId::new(block_id, instr_idx as u32);
            
            // 命令がブロック不変かどうかを判定
            if self.is_block_invariant(instr_id, instr, block_id, body, dataflow)? {
                invariants.insert(instr_id);
            }
        }
        
        Ok(())
    }
    
    /// 命令がブロック内で不変かどうかを判定
    fn is_block_invariant(
        &self,
        instr_id: InstructionId,
        instr: &Instruction,
        block_id: BlockId,
        body: &FunctionBody,
        dataflow: &DataFlowAnalysis
    ) -> Result<bool> {
        // 定数命令は常に不変
        if matches!(instr, Instruction::Constant { .. }) {
            return Ok(true);
        }
        
        // 副作用のある命令は不変ではない
        if instr.has_side_effects() {
            return Ok(false);
        }
        
        // ブロック内で定義される変数への書き込みは不変ではない
        if instr.is_memory_write() {
            return Ok(false);
        }
        
        // 命令のオペランドがすべてブロック不変な場合、その命令も不変
        let operands = instr.get_operands();
        for operand in operands {
            if let Some(def_instr) = dataflow.get_reaching_def(instr_id, operand)? {
                let def_block = def_instr.block();
                
                // 定義が別のブロックにある場合は不変
                if def_block != block_id {
                    continue;
                }
                
                // 定義が同じブロック内にある場合、その定義自体が不変かどうかを再帰的に確認
                let def_idx = def_instr.index();
                let def_instr_ref = body.get_instruction(def_block, def_idx as usize).ok_or_else(|| {
                    Error::AnalysisError(format!("命令 {:?} が見つかりません", def_instr))
                })?;
                
                if !self.is_block_invariant(def_instr, def_instr_ref, block_id, body, dataflow)? {
                    return Ok(false);
                }
            } else {
                // 定義が見つからない場合は外部からの入力と見なし、不変と見なす
                continue;
            }
        }
        
        Ok(true)
    }
    
    /// 指定された命令が不変かどうかを判定
    pub fn is_invariant(&self, instr_id: InstructionId) -> bool {
        self.invariant_types.contains_key(&instr_id)
    }
    
    /// 指定された命令の不変タイプを取得
    pub fn get_invariant_type(&self, instr_id: InstructionId) -> Option<InvariantType> {
        self.invariant_types.get(&instr_id).copied()
    }
    
    /// 指定されたループの不変命令を取得
    pub fn get_loop_invariants(&self, loop_id: LoopId) -> Option<&HashSet<InstructionId>> {
        self.loop_invariants.get(&loop_id)
    }
    
    /// 指定されたブロックの不変命令を取得
    pub fn get_block_invariants(&self, block_id: BlockId) -> Option<&HashSet<InstructionId>> {
        self.block_invariants.get(&block_id)
    }
    
    fn run(module: &Module, dataflow: &DataFlowAnalysis, cf: &ControlFlowAnalysis) -> Result<Self> {
        let mut invariant_types = HashMap::new();
        let mut loop_invariants = HashMap::new();
        let mut block_invariants = HashMap::new();
        
        // 各関数に対して不変解析を実行
        for function in module.functions() {
            let func_id = function.id();
            let body = module.get_function_body(func_id)?;
            
            // 各ループに対して不変解析を実行
            for loop_info in cf.get_loops(func_id)? {
                let loop_id = loop_info.id;
                let mut loop_invariant_instrs = HashSet::new();
                
                // ループ内の各ブロックを処理
                for block_id in &loop_info.blocks {
                    let block = body.get_block(*block_id)?;
                    let mut block_invariant_instrs = HashSet::new();
                    
                    // ブロック内の各命令を処理
                    for instr_idx in 0..block.instructions.len() {
                        let instr_ref = &block.instructions[instr_idx];
                        let instr_id = InstructionId::new(*block_id, instr_idx as u32);
                        
                        // 命令が不変かどうかを判定
                        if self.is_block_invariant(instr_id, instr_ref, *block_id, body, dataflow)? {
                            block_invariant_instrs.insert(instr_id);
                            
                            // ループ不変性も確認
                            if self.is_loop_invariant(instr_id, instr_ref, loop_info, body, dataflow)? {
                                loop_invariant_instrs.insert(instr_id);
                                invariant_types.insert(instr_id, InvariantType::Loop(loop_id));
                            } else {
                                invariant_types.insert(instr_id, InvariantType::Block(*block_id));
                            }
                        }
                    }
                    
                    // ブロック不変命令を記録
                    if !block_invariant_instrs.is_empty() {
                        block_invariants.insert(*block_id, block_invariant_instrs);
                    }
                }
                
                // ループ不変命令を記録
                if !loop_invariant_instrs.is_empty() {
                    loop_invariants.insert(loop_id, loop_invariant_instrs);
                }
            }
            
            // ループに属さないブロックの不変解析
            for block_id in body.blocks.keys() {
                // すでに処理済みのブロックはスキップ
                if block_invariants.contains_key(block_id) {
                    continue;
                }
                
                let block = body.get_block(*block_id)?;
                let mut block_invariant_instrs = HashSet::new();
                
                for instr_idx in 0..block.instructions.len() {
                    let instr_ref = &block.instructions[instr_idx];
                    let instr_id = InstructionId::new(*block_id, instr_idx as u32);
                    
                    if self.is_block_invariant(instr_id, instr_ref, *block_id, body, dataflow)? {
                        block_invariant_instrs.insert(instr_id);
                        invariant_types.insert(instr_id, InvariantType::Block(*block_id));
                    }
                }
                
                if !block_invariant_instrs.is_empty() {
                    block_invariants.insert(*block_id, block_invariant_instrs);
                }
            }
        }
        
        Ok(Self {
            invariant_types,
            loop_invariants,
            block_invariants,
            function_invariants: todo!(),
        })
    }
    
    /// ループ不変性を判定する
    fn is_loop_invariant(&self, instr_id: InstructionId, instr: &Instruction, 
                         loop_info: &LoopInfo, body: &FunctionBody, 
                         dataflow: &DataFlowAnalysis) -> Result<bool> {
        // メモリ書き込みはループ不変ではない
        if instr.is_memory_write() {
            return Ok(false);
        }
        
        // 命令のオペランドがすべてループ不変な場合、その命令も不変
        let operands = instr.get_operands();
        for operand in operands {
            if let Some(def_instr) = dataflow.get_reaching_def(instr_id, operand)? {
                let def_block = def_instr.block();
                
                // 定義がループの外部にある場合は不変
                if !loop_info.blocks.contains(&def_block) {
                    continue;
                }
                
                // 定義がループ内にある場合、その定義自体が不変かどうかを再帰的に確認
                let def_idx = def_instr.index();
                let def_instr_ref = body.get_instruction(def_block, def_idx as usize).ok_or_else(|| {
                    Error::AnalysisError(format!("命令 {:?} が見つかりません", def_instr))
                })?;
                
                if !self.is_loop_invariant(def_instr, def_instr_ref, loop_info, body, dataflow)? {
                    return Ok(false);
                }
            } else {
                // 定義が見つからない場合は外部からの入力と見なし、不変と見なす
                continue;
            }
        }
        
        // 制御依存関係の確認
        // ループ内の条件分岐に依存する命令は不変ではない
        for &block_id in &loop_info.blocks {
            if let Some(control_deps) = dataflow.get_control_dependencies(instr_id)? {
                for &dep_id in control_deps {
                    if dep_id.block() == block_id {
                        return Ok(false);
                    }
                }
            }
        }
        
        Ok(true)
    }
}

impl AnalysisResult for InvariantAnalysis {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    
    fn to_string(&self) -> String {
        let mut result = String::from("ループ不変解析結果:\n");
        
        for (func_id, loop_map) in &self.invariant_instructions {
            result.push_str(&format!("関数 {:?}:\n", func_id));
            
            for (loop_id, instrs) in loop_map {
                result.push_str(&format!("  ループ {:?}: {} 個の不変命令\n", loop_id, instrs.len()));
                
                for instr_id in instrs {
                    result.push_str(&format!("    - 命令 {:?}\n", instr_id));
                }
            }
        }
        
        result
    }
    
    fn dependencies(&self) -> Vec<AnalysisKind> {
        vec![
            AnalysisKind::DataFlow,
            AnalysisKind::LoopInfo,
            AnalysisKind::DominatorTree
        ]
    }
    
    fn validate(&self) -> Result<()> {
        // 不変命令の検証
        for (func_id, loop_map) in &self.invariant_instructions {
            // 各ループに対して
            for (loop_id, instrs) in loop_map {
                // 不変命令が空でないことを確認
                if instrs.is_empty() {
                    log::warn!("関数 {:?} のループ {:?} には不変命令が見つかりませんでした", func_id, loop_id);
                }
                
                // 不変命令の依存関係が循環していないことを確認
                let mut visited = HashSet::new();
                let mut stack = Vec::new();
                
                for &instr_id in instrs {
                    if !visited.contains(&instr_id) {
                        if !self.validate_dependencies(instr_id, instrs, &mut visited, &mut stack)? {
                            return Err(Error::AnalysisError(
                                format!("関数 {:?} のループ {:?} の不変命令 {:?} に循環依存関係があります", 
                                        func_id, loop_id, instr_id)
                            ));
                        }
                    }
                }
            }
        }
        
        // 移動候補命令の検証
        for (func_id, loop_map) in &self.hoistable_instructions {
            for (loop_id, instrs) in loop_map {
                // 移動候補命令が不変命令のサブセットであることを確認
                if let Some(invariants) = self.invariant_instructions.get(func_id) {
                    if let Some(inv_instrs) = invariants.get(loop_id) {
                        for &instr_id in instrs {
                            if !inv_instrs.contains(&instr_id) {
                                return Err(Error::AnalysisError(
                                    format!("関数 {:?} のループ {:?} の移動候補命令 {:?} は不変命令ではありません", 
                                            func_id, loop_id, instr_id)
                                ));
                            }
                        }
                    }
                }
            }
        }
        
        Ok(())
    }
    
    fn memory_usage(&self) -> usize {
        let mut size = 0;
        
        // invariant_instructionsのメモリ使用量を計算
        for (_, loop_map) in &self.invariant_instructions {
            size += std::mem::size_of::<FunctionId>();
            size += std::mem::size_of::<HashMap<LoopId, HashSet<InstructionId>>>();
            
            for (_, instrs) in loop_map {
                size += std::mem::size_of::<LoopId>();
                size += std::mem::size_of::<HashSet<InstructionId>>();
                size += instrs.len() * std::mem::size_of::<InstructionId>();
            }
        }
        
        // hoistable_instructionsのメモリ使用量を計算
        for (_, loop_map) in &self.hoistable_instructions {
            size += std::mem::size_of::<FunctionId>();
            size += std::mem::size_of::<HashMap<LoopId, HashSet<InstructionId>>>();
            
            for (_, instrs) in loop_map {
                size += std::mem::size_of::<LoopId>();
                size += std::mem::size_of::<HashSet<InstructionId>>();
                size += instrs.len() * std::mem::size_of::<InstructionId>();
            }
        }
        
        // insertion_pointsのメモリ使用量を計算
        for (_, loop_map) in &self.insertion_points {
            size += std::mem::size_of::<FunctionId>();
            size += std::mem::size_of::<HashMap<LoopId, BlockId>>();
            
            for _ in loop_map.keys() {
                size += std::mem::size_of::<LoopId>();
                size += std::mem::size_of::<BlockId>();
            }
        }
        
        size
    }
}

/// デッドコード解析
pub struct DeadCodeAnalysis {
    // 到達不能なブロック
    unreachable_blocks: HashMap<FunctionId, HashSet<BlockId>>,
    // 使用されない命令
    unused_instructions: HashMap<FunctionId, HashSet<InstructionId>>,
    // 使用されない変数
    unused_variables: HashMap<FunctionId, HashSet<VariableId>>,
    // 冗長な命令（同じ計算を行う命令）
    redundant_instructions: HashMap<FunctionId, HashMap<InstructionId, InstructionId>>,
}

impl DeadCodeAnalysis {
    fn run(module: &Module, reach: &ReachabilityAnalysis, side_effect: &SideEffectAnalysis) -> Result<Self> {
        let mut unreachable_blocks = HashMap::new();
        let mut unused_instructions = HashMap::new();
        let mut unused_variables = HashMap::new();
        let mut redundant_instructions = HashMap::new();
        
        // 各関数に対してデッドコード解析を実行
        for function in module.functions() {
            let func_id = function.id();
            let body = module.get_function_body(func_id)?;
            
            // 到達不能なブロックを特定
            let mut func_unreachable = HashSet::new();
            for block_id in body.blocks.keys() {
                if !reach.is_reachable(func_id, *block_id)? {
                    func_unreachable.insert(*block_id);
                }
            }
            
            if !func_unreachable.is_empty() {
                unreachable_blocks.insert(func_id, func_unreachable);
            }
            
            // 使用されない命令と変数を特定
            let mut func_unused_instrs = HashSet::new();
            let mut func_unused_vars = HashSet::new();
            let mut func_redundant = HashMap::new();
            
            // 命令の値等価性を追跡するマップ
            let mut value_equiv_map: HashMap<String, InstructionId> = HashMap::new();
            
            for (block_id, block) in &body.blocks {
                // 到達不能なブロックの命令はすべて未使用と見なす
                if unreachable_blocks.get(&func_id).map_or(false, |set| set.contains(block_id)) {
                    for (idx, _) in block.instructions.iter().enumerate() {
                        func_unused_instrs.insert(InstructionId::new(*block_id, idx as u32));
                    }
                    continue;
                }
                
                for (idx, instr) in block.instructions.iter().enumerate() {
                    let instr_id = InstructionId::new(*block_id, idx as u32);
                    
                    // 副作用のない命令で、その結果が使用されていない場合は未使用
                    if !side_effect.has_side_effect(instr_id)? && !reach.is_result_used(instr_id)? {
                        func_unused_instrs.insert(instr_id);
                    }
                    
                    // 冗長な命令を検出（値等価性に基づく）
                    if let Some(canonical_repr) = instr.get_canonical_representation() {
                        if let Some(existing_id) = value_equiv_map.get(&canonical_repr) {
                            // 同じ計算を行う命令が既に存在する場合
                            func_redundant.insert(instr_id, *existing_id);
                        } else {
                            // この命令を新たな等価クラスの代表として登録
                            value_equiv_map.insert(canonical_repr, instr_id);
                        }
                    }
                    
                    // 変数の使用状況を追跡
                    if let Some(var_id) = instr.defines_variable() {
                        if !reach.is_variable_used(var_id)? {
                            func_unused_vars.insert(var_id);
                        }
                    }
                }
            }
            
            if !func_unused_instrs.is_empty() {
                unused_instructions.insert(func_id, func_unused_instrs);
            }
            
            if !func_unused_vars.is_empty() {
                unused_variables.insert(func_id, func_unused_vars);
            }
            
            if !func_redundant.is_empty() {
                redundant_instructions.insert(func_id, func_redundant);
            }
        }
        
        Ok(Self {
            unreachable_blocks,
            unused_instructions,
            unused_variables,
            redundant_instructions,
        })
    }
    
    /// 指定されたブロックが到達不能かどうかを判定
    pub fn is_block_unreachable(&self, func_id: FunctionId, block_id: BlockId) -> bool {
        self.unreachable_blocks.get(&func_id)
            .map_or(false, |blocks| blocks.contains(&block_id))
    }
    
    /// 指定された命令が未使用かどうかを判定
    pub fn is_instruction_unused(&self, func_id: FunctionId, instr_id: InstructionId) -> bool {
        self.unused_instructions.get(&func_id)
            .map_or(false, |instrs| instrs.contains(&instr_id))
    }
    
    /// 指定された変数が未使用かどうかを判定
    pub fn is_variable_unused(&self, func_id: FunctionId, var_id: VariableId) -> bool {
        self.unused_variables.get(&func_id)
            .map_or(false, |vars| vars.contains(&var_id))
    }
    
    /// 指定された命令が冗長かどうかを判定し、等価な命令のIDを返す
    pub fn get_redundant_equivalent(&self, func_id: FunctionId, instr_id: InstructionId) -> Option<InstructionId> {
        self.redundant_instructions.get(&func_id)
            .and_then(|map| map.get(&instr_id).copied())
    }
    
    /// 関数内の到達不能なブロックを取得
    pub fn get_unreachable_blocks(&self, func_id: FunctionId) -> Option<&HashSet<BlockId>> {
        self.unreachable_blocks.get(&func_id)
    }
    
    /// 関数内の未使用命令を取得
    pub fn get_unused_instructions(&self, func_id: FunctionId) -> Option<&HashSet<InstructionId>> {
        self.unused_instructions.get(&func_id)
    }
    
    /// 関数内の未使用変数を取得
    pub fn get_unused_variables(&self, func_id: FunctionId) -> Option<&HashSet<VariableId>> {
        self.unused_variables.get(&func_id)
    }
}

impl AnalysisResult for DeadCodeAnalysis {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// 定数伝播解析
pub struct ConstantPropagationAnalysis {
    // 命令IDから定数値へのマッピング
    constant_values: HashMap<InstructionId, ConstantValue>,
    // 変数IDから定数値へのマッピング
    variable_constants: HashMap<VariableId, ConstantValue>,
    // 条件分岐の結果が定数と判明している場合のマッピング
    branch_outcomes: HashMap<InstructionId, bool>,
    // 関数呼び出しの結果が定数と判明している場合のマッピング
    function_call_results: HashMap<InstructionId, ConstantValue>,
}
/// 定数値の表現
#[derive(Clone, Debug, PartialEq)]
pub enum ConstantValue {
    Integer(i64),
    Float(f64),
    Boolean(bool),
    String(String),
    Null,
    Undefined,
    Composite(Vec<ConstantValue>),
    Tuple(Vec<ConstantValue>),
    Array(Vec<ConstantValue>),
    Map(HashMap<String, ConstantValue>),
    Function(FunctionId),
    Reference(usize),  // メモリ参照（コンパイル時評価用）
    Range(Box<ConstantValue>, Box<ConstantValue>, Box<Option<ConstantValue>>), // 開始、終了、ステップ
    Optional(Option<Box<ConstantValue>>),
    Type(TypeId),
}

impl ConstantPropagationAnalysis {
    pub fn new() -> Self {
        Self {
            constant_values: HashMap::new(),
            variable_constants: HashMap::new(),
            branch_outcomes: HashMap::new(),
            function_call_results: HashMap::new(),
        }
    }

    pub fn run(module: &Module, dataflow: &DataFlowAnalysis) -> Result<Self> {
        let mut analysis = Self::new();
        
        // 各関数に対して定数伝播解析を実行
        for function in module.functions() {
            let func_id = function.id();
            let body = module.get_function_body(func_id)?;
            
            // 不動点に達するまで繰り返し解析
            let mut changed = true;
            let mut iteration_count = 0;
            const MAX_ITERATIONS: usize = 100; // 無限ループ防止
            
            while changed && iteration_count < MAX_ITERATIONS {
                changed = false;
                iteration_count += 1;
                
                // 各ブロックをデータフロー順に処理
                for block_id in dataflow.get_block_execution_order(func_id)? {
                    let block = body.get_block(block_id)?;
                    
                    // ブロック内の各命令を処理
                    for instr_idx in 0..block.instructions.len() {
                        let instr_id = InstructionId::new(block_id, instr_idx as u32);
                        let instr = &block.instructions[instr_idx];
                        
                        // 命令の種類に応じた定数伝播
                        if analysis.process_instruction(module, dataflow, func_id, instr_id, instr)? {
                            changed = true;
                        }
                    }
                }
            }
            
            if iteration_count == MAX_ITERATIONS {
                log::warn!("定数伝播解析が最大反復回数に達しました: 関数 {:?}", func_id);
            }
        }
        
        Ok(analysis)
    }
    
    /// 命令を処理し、状態が変化した場合はtrueを返す
    fn process_instruction(
        &mut self,
        module: &Module,
        dataflow: &DataFlowAnalysis,
        func_id: FunctionId,
        instr_id: InstructionId,
        instr: &Instruction,
    ) -> Result<bool> {
        let mut changed = false;
        
        match instr {
            Instruction::Constant(value) => {
                // 定数命令は直接マッピング
                let const_value = Self::convert_to_constant_value(value)?;
                let old_value = self.constant_values.insert(instr_id, const_value.clone());
                if old_value.as_ref() != Some(&const_value) {
                    changed = true;
                }
                
                // 変数に代入される場合は変数も定数として記録
                if let Some(var_id) = instr.defines_variable() {
                    let old_var_value = self.variable_constants.insert(var_id, const_value);
                    if old_var_value.is_none() {
                        changed = true;
                    }
                }
            },
            Instruction::BinaryOp(op, lhs, rhs) => {
                // 両オペランドが定数の場合、演算結果も定数
                if let (Some(lhs_val), Some(rhs_val)) = (
                    self.get_operand_constant_value(module, dataflow, lhs, instr_id)?,
                    self.get_operand_constant_value(module, dataflow, rhs, instr_id)?
                ) {
                    if let Some(result) = Self::evaluate_binary_op(op, &lhs_val, &rhs_val)? {
                        let old_value = self.constant_values.insert(instr_id, result.clone());
                        if old_value.as_ref() != Some(&result) {
                            changed = true;
                        }
                        
                        if let Some(var_id) = instr.defines_variable() {
                            let old_var_value = self.variable_constants.insert(var_id, result);
                            if old_var_value.is_none() {
                                changed = true;
                            }
                        }
                    }
                }
            },
            Instruction::UnaryOp(op, operand) => {
                // オペランドが定数の場合、演算結果も定数
                if let Some(operand_val) = self.get_operand_constant_value(
                    module, dataflow, operand, instr_id
                )? {
                    if let Some(result) = Self::evaluate_unary_op(op, &operand_val)? {
                        let old_value = self.constant_values.insert(instr_id, result.clone());
                        if old_value.as_ref() != Some(&result) {
                            changed = true;
                        }
                        
                        if let Some(var_id) = instr.defines_variable() {
                            let old_var_value = self.variable_constants.insert(var_id, result);
                            if old_var_value.is_none() {
                                changed = true;
                            }
                        }
                    }
                }
            },
            Instruction::Branch(cond) => {
                // 条件が定数の場合、分岐結果も定数
                if let Some(cond_val) = self.get_operand_constant_value(
                    module, dataflow, cond, instr_id
                )? {
                    if let ConstantValue::Boolean(b) = cond_val {
                        let old_value = self.branch_outcomes.insert(instr_id, b);
                        if old_value != Some(b) {
                            changed = true;
                        }
                    }
                }
            },
            Instruction::Call(func, args) => {
                // 純粋関数で引数がすべて定数の場合、結果も定数になる可能性がある
                if let Some(callee_id) = module.resolve_function_reference(func) {
                    if module.is_pure_function(callee_id)? {
                        let mut all_args_constant = true;
                        let mut arg_values = Vec::new();
                        
                        for arg in args {
                            if let Some(arg_val) = self.get_operand_constant_value(
                                module, dataflow, arg, instr_id
                            )? {
                                arg_values.push(arg_val);
                            } else {
                                all_args_constant = false;
                                break;
                            }
                        }
                        
                        if all_args_constant {
                            // 純粋関数の定数評価を試みる
                            if let Some(result) = self.evaluate_pure_function(
                                module, callee_id, &arg_values
                            )? {
                                let old_value = self.constant_values.insert(instr_id, result.clone());
                                if old_value.as_ref() != Some(&result) {
                                    changed = true;
                                }
                                
                                if let Some(var_id) = instr.defines_variable() {
                                    let old_var_value = self.variable_constants.insert(var_id, result);
                                    if old_var_value.is_none() {
                                        changed = true;
                                    }
                                }
                            }
                        }
                    }
                }
            },
            Instruction::Phi(operands) => {
                // すべての入力が同じ定数値の場合、Phi命令も定数
                let mut const_value: Option<ConstantValue> = None;
                let mut all_same = true;
                
                for (operand, _block) in operands {
                    if let Some(value) = self.get_operand_constant_value(module, dataflow, operand, instr_id)? {
                        if let Some(ref existing) = const_value {
                            if *existing != value {
                                all_same = false;
                                break;
                            }
                        } else {
                            const_value = Some(value);
                        }
                    } else {
                        all_same = false;
                        break;
                    }
                }
                
                if all_same && const_value.is_some() {
                    let result = const_value.unwrap();
                    let old_value = self.constant_values.insert(instr_id, result.clone());
                    if old_value.as_ref() != Some(&result) {
                        changed = true;
                    }
                    
                    if let Some(var_id) = instr.defines_variable() {
                        let old_var_value = self.variable_constants.insert(var_id, result);
                        if old_var_value.is_none() {
                            changed = true;
                        }
                    }
                }
            },
            Instruction::Select(cond, true_val, false_val) => {
                // 条件が定数の場合、結果も定数になる
                if let Some(cond_val) = self.get_operand_constant_value(module, dataflow, cond, instr_id)? {
                    if let ConstantValue::Boolean(b) = cond_val {
                        let selected_operand = if b { true_val } else { false_val };
                        if let Some(result) = self.get_operand_constant_value(module, dataflow, selected_operand, instr_id)? {
                            let old_value = self.constant_values.insert(instr_id, result.clone());
                            if old_value.as_ref() != Some(&result) {
                                changed = true;
                            }
                            
                            if let Some(var_id) = instr.defines_variable() {
                                let old_var_value = self.variable_constants.insert(var_id, result);
                                if old_var_value.is_none() {
                                    changed = true;
                                }
                            }
                        }
                    }
                }
            },
            Instruction::GetElement(aggregate, index) => {
                // 配列/タプル/マップと添字が定数の場合、結果も定数になる可能性がある
                if let (Some(agg_val), Some(idx_val)) = (
                    self.get_operand_constant_value(module, dataflow, aggregate, instr_id)?,
                    self.get_operand_constant_value(module, dataflow, index, instr_id)?
                ) {
                    if let Some(result) = Self::evaluate_get_element(&agg_val, &idx_val)? {
                        let old_value = self.constant_values.insert(instr_id, result.clone());
                        if old_value.as_ref() != Some(&result) {
                            changed = true;
                        }
                        
                        if let Some(var_id) = instr.defines_variable() {
                            let old_var_value = self.variable_constants.insert(var_id, result);
                            if old_var_value.is_none() {
                                changed = true;
                            }
                        }
                    }
                }
            },
            Instruction::GetField(object, field_name) => {
                // オブジェクトが定数の場合、フィールドアクセスも定数になる可能性がある
                if let Some(obj_val) = self.get_operand_constant_value(module, dataflow, object, instr_id)? {
                    if let Some(result) = Self::evaluate_get_field(&obj_val, field_name)? {
                        let old_value = self.constant_values.insert(instr_id, result.clone());
                        if old_value.as_ref() != Some(&result) {
                            changed = true;
                        }
                        
                        if let Some(var_id) = instr.defines_variable() {
                            let old_var_value = self.variable_constants.insert(var_id, result);
                            if old_var_value.is_none() {
                                changed = true;
                            }
                        }
                    }
                }
            },
            Instruction::Cast(value, target_type) => {
                // 値が定数の場合、キャストも定数になる可能性がある
                if let Some(val) = self.get_operand_constant_value(module, dataflow, value, instr_id)? {
                    if let Some(result) = Self::evaluate_cast(&val, target_type, module)? {
                        let old_value = self.constant_values.insert(instr_id, result.clone());
                        if old_value.as_ref() != Some(&result) {
                            changed = true;
                        }
                        
                        if let Some(var_id) = instr.defines_variable() {
                            let old_var_value = self.variable_constants.insert(var_id, result);
                            if old_var_value.is_none() {
                                changed = true;
                            }
                        }
                    }
                }
            },
            Instruction::CreateTuple(elements) | Instruction::CreateArray(elements) => {
                // すべての要素が定数の場合、結果も定数
                let mut all_constant = true;
                let mut const_elements = Vec::new();
                
                for elem in elements {
                    if let Some(elem_val) = self.get_operand_constant_value(module, dataflow, elem, instr_id)? {
                        const_elements.push(elem_val);
                    } else {
                        all_constant = false;
                        break;
                    }
                }
                
                if all_constant {
                    let result = match instr {
                        Instruction::CreateTuple(_) => ConstantValue::Tuple(const_elements),
                        Instruction::CreateArray(_) => ConstantValue::Array(const_elements),
                        _ => unreachable!(),
                    };
                    
                    let old_value = self.constant_values.insert(instr_id, result.clone());
                    if old_value.as_ref() != Some(&result) {
                        changed = true;
                    }
                    
                    if let Some(var_id) = instr.defines_variable() {
                        let old_var_value = self.variable_constants.insert(var_id, result);
                        if old_var_value.is_none() {
                            changed = true;
                        }
                    }
                }
            },
            Instruction::CreateMap(entries) => {
                // すべてのキーと値が定数の場合、結果も定数
                let mut all_constant = true;
                let mut const_map = HashMap::new();
                
                for (key, value) in entries {
                    if let (Some(key_val), Some(val)) = (
                        self.get_operand_constant_value(module, dataflow, key, instr_id)?,
                        self.get_operand_constant_value(module, dataflow, value, instr_id)?
                    ) {
                        // キーは文字列に変換可能である必要がある
                        if let Some(key_str) = Self::constant_to_string(&key_val) {
                            const_map.insert(key_str, val);
                        } else {
                            all_constant = false;
                            break;
                        }
                    } else {
                        all_constant = false;
                        break;
                    }
                }
                
                if all_constant {
                    let result = ConstantValue::Map(const_map);
                    let old_value = self.constant_values.insert(instr_id, result.clone());
                    if old_value.as_ref() != Some(&result) {
                        changed = true;
                    }
                    
                    if let Some(var_id) = instr.defines_variable() {
                        let old_var_value = self.variable_constants.insert(var_id, result);
                        if old_var_value.is_none() {
                            changed = true;
                        }
                    }
                }
            },
            Instruction::CreateObject(type_id, fields) => {
                // すべてのフィールド値が定数の場合、結果も定数
                let mut all_constant = true;
                let mut const_fields = HashMap::new();
                
                for (field_name, value) in fields {
                    if let Some(val) = self.get_operand_constant_value(module, dataflow, value, instr_id)? {
                        const_fields.insert(field_name.clone(), val);
                    } else {
                        all_constant = false;
                        break;
                    }
                }
                
                if all_constant {
                    let result = ConstantValue::Object(*type_id, const_fields);
                    let old_value = self.constant_values.insert(instr_id, result.clone());
                    if old_value.as_ref() != Some(&result) {
                        changed = true;
                    }
                    
                    if let Some(var_id) = instr.defines_variable() {
                        let old_var_value = self.variable_constants.insert(var_id, result);
                        if old_var_value.is_none() {
                            changed = true;
                        }
                    }
                }
            },
            Instruction::CreateClosure(func_id, captures) => {
                // すべてのキャプチャ値が定数の場合、クロージャも定数とみなせる
                let mut all_constant = true;
                let mut const_captures = HashMap::new();
                
                for (var_id, value) in captures {
                    if let Some(val) = self.get_operand_constant_value(module, dataflow, value, instr_id)? {
                        const_captures.insert(*var_id, val);
                    } else {
                        all_constant = false;
                        break;
                    }
                }
                
                if all_constant {
                    let result = ConstantValue::Closure(*func_id, const_captures);
                    let old_value = self.constant_values.insert(instr_id, result.clone());
                    if old_value.as_ref() != Some(&result) {
                        changed = true;
                    }
                    
                    if let Some(var_id) = instr.defines_variable() {
                        let old_var_value = self.variable_constants.insert(var_id, result);
                        if old_var_value.is_none() {
                            changed = true;
                        }
                    }
                }
            },
            Instruction::CreateOptional(value) => {
                // 値が定数の場合、Optional型も定数
                if let Some(inner_val) = value {
                    if let Some(val) = self.get_operand_constant_value(module, dataflow, inner_val, instr_id)? {
                        let result = ConstantValue::Optional(Some(Box::new(val)));
                        let old_value = self.constant_values.insert(instr_id, result.clone());
                        if old_value.as_ref() != Some(&result) {
                            changed = true;
                        }
                        
                        if let Some(var_id) = instr.defines_variable() {
                            let old_var_value = self.variable_constants.insert(var_id, result);
                            if old_var_value.is_none() {
                                changed = true;
                            }
                        }
                    }
                } else {
                    // None値のOptional
                    let result = ConstantValue::Optional(None);
                    let old_value = self.constant_values.insert(instr_id, result.clone());
                    if old_value.as_ref() != Some(&result) {
                        changed = true;
                    }
                    
                    if let Some(var_id) = instr.defines_variable() {
                        let old_var_value = self.variable_constants.insert(var_id, result);
                        if old_var_value.is_none() {
                            changed = true;
                        }
                    }
                }
            },
            Instruction::CreateRange(start, end, step) => {
                // 範囲の開始、終了、ステップが定数の場合、範囲も定数
                if let (Some(start_val), Some(end_val)) = (
                    self.get_operand_constant_value(module, dataflow, start, instr_id)?,
                    self.get_operand_constant_value(module, dataflow, end, instr_id)?
                ) {
                    let step_val = if let Some(step_op) = step {
                        if let Some(val) = self.get_operand_constant_value(module, dataflow, step_op, instr_id)? {
                            Some(Box::new(val))
                        } else {
                            None
                        }
                    } else {
                        None
                    };
                    
                    // ステップが指定されていない場合でも範囲は定数
                    if step.is_none() || step_val.is_some() {
                        let result = ConstantValue::Range(
                            Box::new(start_val),
                            Box::new(end_val),
                            Box::new(step_val)
                        );
                        let old_value = self.constant_values.insert(instr_id, result.clone());
                        if old_value.as_ref() != Some(&result) {
                            changed = true;
                        }
                        
                        if let Some(var_id) = instr.defines_variable() {
                            let old_var_value = self.variable_constants.insert(var_id, result);
                            if old_var_value.is_none() {
                                changed = true;
                            }
                        }
                    }
                }
            },
            Instruction::IsNull(value) => {
                // 値が定数の場合、null判定も定数
                if let Some(val) = self.get_operand_constant_value(module, dataflow, value, instr_id)? {
                    let is_null = matches!(val, ConstantValue::Null);
                    let result = ConstantValue::Boolean(is_null);
                    let old_value = self.constant_values.insert(instr_id, result.clone());
                    if old_value.as_ref() != Some(&result) {
                        changed = true;
                    }
                    
                    if let Some(var_id) = instr.defines_variable() {
                        let old_var_value = self.variable_constants.insert(var_id, result);
                        if old_var_value.is_none() {
                            changed = true;
                        }
                    }
                }
            },
            Instruction::IsUndefined(value) => {
                // 値が定数の場合、undefined判定も定数
                if let Some(val) = self.get_operand_constant_value(module, dataflow, value, instr_id)? {
                    let is_undefined = matches!(val, ConstantValue::Undefined);
                    let result = ConstantValue::Boolean(is_undefined);
                    let old_value = self.constant_values.insert(instr_id, result.clone());
                    if old_value.as_ref() != Some(&result) {
                        changed = true;
                    }
                    
                    if let Some(var_id) = instr.defines_variable() {
                        let old_var_value = self.variable_constants.insert(var_id, result);
                        if old_var_value.is_none() {
                            changed = true;
                        }
                    }
                }
            },
            Instruction::TypeOf(value) => {
                // 値が定数の場合、型判定も定数
                if let Some(val) = self.get_operand_constant_value(module, dataflow, value, instr_id)? {
                    let type_name = Self::get_type_name(&val);
                    let result = ConstantValue::String(type_name);
                    let old_value = self.constant_values.insert(instr_id, result.clone());
                    if old_value.as_ref() != Some(&result) {
                        changed = true;
                    }
                    
                    if let Some(var_id) = instr.defines_variable() {
                        let old_var_value = self.variable_constants.insert(var_id, result);
                        if old_var_value.is_none() {
                            changed = true;
                        }
                    }
                }
            },
            Instruction::InstanceOf(value, type_ref) => {
                // 値と型参照が定数の場合、instanceof判定も定数
                if let (Some(val), Some(type_val)) = (
                    self.get_operand_constant_value(module, dataflow, value, instr_id)?,
                    self.get_operand_constant_value(module, dataflow, type_ref, instr_id)?
                ) {
                    if let ConstantValue::Type(type_id) = type_val {
                        let is_instance = Self::check_instance_of(&val, type_id, module)?;
                        let result = ConstantValue::Boolean(is_instance);
                        let old_value = self.constant_values.insert(instr_id, result.clone());
                        if old_value.as_ref() != Some(&result) {
                            changed = true;
                        }
                        
                        if let Some(var_id) = instr.defines_variable() {
                            let old_var_value = self.variable_constants.insert(var_id, result);
                            if old_var_value.is_none() {
                                changed = true;
                            }
                        }
                    }
                }
            },
            Instruction::Unwrap(optional) => {
                // Optional値が定数の場合、アンラップ結果も定数
                if let Some(opt_val) = self.get_operand_constant_value(module, dataflow, optional, instr_id)? {
                    if let ConstantValue::Optional(Some(inner)) = opt_val {
                        let result = *inner;
                        let old_value = self.constant_values.insert(instr_id, result.clone());
                        if old_value.as_ref() != Some(&result) {
                            changed = true;
                        }
                        
                        if let Some(var_id) = instr.defines_variable() {
                            let old_var_value = self.variable_constants.insert(var_id, result);
                            if old_var_value.is_none() {
                                changed = true;
                            }
                        }
                    }
                    // None値のアンラップは実行時エラーになるため、ここでは処理しない
                }
            },
            Instruction::HasValue(optional) => {
                // Optional値が定数の場合、値の有無判定も定数
                if let Some(opt_val) = self.get_operand_constant_value(module, dataflow, optional, instr_id)? {
                    if let ConstantValue::Optional(inner) = opt_val {
                        let has_value = inner.is_some();
                        let result = ConstantValue::Boolean(has_value);
                        let old_value = self.constant_values.insert(instr_id, result.clone());
                        if old_value.as_ref() != Some(&result) {
                            changed = true;
                        }
                        
                        if let Some(var_id) = instr.defines_variable() {
                            let old_var_value = self.variable_constants.insert(var_id, result);
                            if old_var_value.is_none() {
                                changed = true;
                            }
                        }
                    }
                }
            },
            Instruction::StringInterpolation(parts) => {
                // すべての部分が定数の場合、文字列補間も定数
                let mut all_constant = true;
                let mut result_string = String::new();
                
                for part in parts {
                    if let Some(part_val) = self.get_operand_constant_value(module, dataflow, part, instr_id)? {
                        // 各部分を文字列に変換
                        if let Some(part_str) = Self::constant_to_string(&part_val) {
                            result_string.push_str(&part_str);
                        } else {
                            all_constant = false;
                            break;
                        }
                    } else {
                        all_constant = false;
                        break;
                    }
                }
                
                if all_constant {
                    let result = ConstantValue::String(result_string);
                    let old_value = self.constant_values.insert(instr_id, result.clone());
                    if old_value.as_ref() != Some(&result) {
                        changed = true;
                    }
                    
                    if let Some(var_id) = instr.defines_variable() {
                        let old_var_value = self.variable_constants.insert(var_id, result);
                        if old_var_value.is_none() {
                            changed = true;
                        }
                    }
                }
            },
            Instruction::LoadModule(module_path) => {
                // モジュールパスが定数の場合、モジュール読み込みを定数として最適化
                if let Some(path_val) = self.get_operand_constant_value(module, dataflow, module_path, instr_id)? {
                    if let ConstantValue::String(path) = path_val {
                        if let Some(module_id) = module.resolve_module_path(&path)? {
                            // モジュールの依存関係を解析して循環参照がないことを確認
                            if !module.has_circular_dependency(module_id)? {
                                // モジュールの内容が全て定数か確認
                                let module_exports = module.get_module_exports(module_id)?;
                                let mut all_exports_constant = true;
                                let mut export_values = HashMap::new();
                                
                                for (export_name, export_id) in &module_exports {
                                    match export_id {
                                        ExportId::Variable(var_id) => {
                                            if let Some(var_module) = module.get_variable_module(*var_id) {
                                                if let Some(var_value) = self.get_variable_constant_in_module(var_id, var_module)? {
                                                    export_values.insert(export_name.clone(), var_value.clone());
                                                } else {
                                                    all_exports_constant = false;
                                                    break;
                                                }
                                            } else {
                                                all_exports_constant = false;
                                                break;
                                            }
                                        },
                                        ExportId::Function(func_id) => {
                                            // 関数は常に定数として扱う（関数参照）
                                            export_values.insert(export_name.clone(), ConstantValue::Function(*func_id));
                                        },
                                        ExportId::Type(type_id) => {
                                            // 型も定数として扱う
                                            export_values.insert(export_name.clone(), ConstantValue::Type(*type_id));
                                        },
                                        ExportId::Module(nested_module_id) => {
                                            // 入れ子モジュールは再帰的に解析
                                            if self.is_module_constant(*nested_module_id, module)? {
                                                export_values.insert(export_name.clone(), ConstantValue::Module(*nested_module_id));
                                            } else {
                                                all_exports_constant = false;
                                                break;
                                            }
                                        }
                                    }
                                }
                                
                                // モジュールの内容が全て定数の場合、モジュール全体を定数として扱う
                                if all_exports_constant {
                                    let result = ConstantValue::ModuleWithExports(module_id, export_values);
                                    let old_value = self.constant_values.insert(instr_id, result.clone());
                                    if old_value.as_ref() != Some(&result) {
                                        changed = true;
                                    }
                                    
                                    if let Some(var_id) = instr.defines_variable() {
                                        let old_var_value = self.variable_constants.insert(var_id, result);
                                        if old_var_value.is_none() || old_var_value.as_ref() != Some(&result) {
                                            changed = true;
                                        }
                                    }
                                } else {
                                    // 一部が定数でない場合でも、モジュール参照自体は定数
                                    let result = ConstantValue::Module(module_id);
                                    let old_value = self.constant_values.insert(instr_id, result.clone());
                                    if old_value.as_ref() != Some(&result) {
                                        changed = true;
                                    }
                                    
                                    if let Some(var_id) = instr.defines_variable() {
                                        let old_var_value = self.variable_constants.insert(var_id, result);
                                        if old_var_value.is_none() || old_var_value.as_ref() != Some(&result) {
                                            changed = true;
                                        }
                                    }
                                }
                            } else {
                                // 循環参照がある場合でも、モジュール参照自体は定数
                                let result = ConstantValue::Module(module_id);
                                let old_value = self.constant_values.insert(instr_id, result.clone());
                                if old_value.as_ref() != Some(&result) {
                                    changed = true;
                                }
                                
                                if let Some(var_id) = instr.defines_variable() {
                                    let old_var_value = self.variable_constants.insert(var_id, result);
                                    if old_var_value.is_none() || old_var_value.as_ref() != Some(&result) {
                                        changed = true;
                                    }
                                }
                            }
                        }
                    }
                }
            },
            Instruction::LoadProperty(object, property_name) => {
                // オブジェクトとプロパティ名が両方定数の場合、プロパティ読み込みも定数になる可能性がある
                if let (Some(obj_val), Some(prop_val)) = (
                    self.get_operand_constant_value(module, dataflow, object, instr_id)?,
                    self.get_operand_constant_value(module, dataflow, property_name, instr_id)?
                ) {
                    if let ConstantValue::String(prop_name) = &prop_val {
                        let result = match &obj_val {
                            ConstantValue::ModuleWithExports(_, exports) => {
                                // モジュールからエクスポートを取得
                                exports.get(prop_name).cloned()
                            },
                            ConstantValue::Module(module_id) => {
                                // モジュールIDからエクスポートを取得
                                if let Some(export_id) = module.get_module_export(*module_id, prop_name)? {
                                    match export_id {
                                        ExportId::Variable(var_id) => {
                                            if let Some(var_module) = module.get_variable_module(var_id) {
                                                self.get_variable_constant_in_module(&var_id, var_module)?.cloned()
                                            } else {
                                                None
                                            }
                                        },
                                        ExportId::Function(func_id) => Some(ConstantValue::Function(func_id)),
                                        ExportId::Type(type_id) => Some(ConstantValue::Type(type_id)),
                                        ExportId::Module(nested_module_id) => Some(ConstantValue::Module(nested_module_id)),
                                    }
                                } else {
                                    None
                                }
                            },
                            ConstantValue::Composite(elements) => {
                                // 配列の場合、数値インデックスとして解釈
                                if let Ok(index) = prop_name.parse::<usize>() {
                                    if index < elements.len() {
                                        Some(elements[index].clone())
                                    } else {
                                        None
                                    }
                                } else {
                                    None
                                }
                            },
                            ConstantValue::Object(properties) => {
                                // オブジェクトからプロパティを取得
                                properties.get(prop_name).cloned()
                            },
                            _ => None,
                        };
                        
                        if let Some(prop_result) = result {
                            let old_value = self.constant_values.insert(instr_id, prop_result.clone());
                            if old_value.as_ref() != Some(&prop_result) {
                                changed = true;
                            }
                            
                            if let Some(var_id) = instr.defines_variable() {
                                let old_var_value = self.variable_constants.insert(var_id, prop_result);
                                if old_var_value.is_none() || old_var_value.as_ref() != Some(&prop_result) {
                                    changed = true;
                                }
                            }
                        }
                    }
                }
            },
            Instruction::Call(func, args) => {
                // 関数と引数が全て定数の場合、関数呼び出しも定数になる可能性がある
                if let Some(func_val) = self.get_operand_constant_value(module, dataflow, func, instr_id)? {
                    let mut all_args_constant = true;
                    let mut const_args = Vec::new();
                    
                    for arg in args {
                        if let Some(arg_val) = self.get_operand_constant_value(module, dataflow, arg, instr_id)? {
                            const_args.push(arg_val.clone());
                        } else {
                            all_args_constant = false;
                            break;
                        }
                    }
                    
                    if all_args_constant {
                        match &func_val {
                            ConstantValue::Function(func_id) => {
                                // 純粋関数かどうかを確認
                                if module.is_pure_function(*func_id)? {
                                    // コンパイル時に関数を評価
                                    if let Some(result) = self.evaluate_constant_function(*func_id, &const_args, module)? {
                                        let old_value = self.constant_values.insert(instr_id, result.clone());
                                        if old_value.as_ref() != Some(&result) {
                                            changed = true;
                                        }
                                        
                                        if let Some(var_id) = instr.defines_variable() {
                                            let old_var_value = self.variable_constants.insert(var_id, result);
                                            if old_var_value.is_none() || old_var_value.as_ref() != Some(&result) {
                                                changed = true;
                                            }
                                        }
                                        
                                        // 関数呼び出し結果をキャッシュ
                                        self.function_call_results.insert(instr_id, result);
                                    }
                                }
                            },
                            ConstantValue::BuiltinFunction(builtin_func) => {
                                // 組み込み関数の評価
                                if let Some(result) = self.evaluate_builtin_function(builtin_func, &const_args)? {
                                    let old_value = self.constant_values.insert(instr_id, result.clone());
                                    if old_value.as_ref() != Some(&result) {
                                        changed = true;
                                    }
                                    
                                    if let Some(var_id) = instr.defines_variable() {
                                        let old_var_value = self.variable_constants.insert(var_id, result);
                                        if old_var_value.is_none() || old_var_value.as_ref() != Some(&result) {
                                            changed = true;
                                        }
                                    }
                                    
                                    // 関数呼び出し結果をキャッシュ
                                    self.function_call_results.insert(instr_id, result);
                                }
                            },
                            _ => {}
                        }
                    }
                }
            },
            Instruction::CreateObject(properties) => {
                // プロパティが全て定数の場合、オブジェクト生成も定数
                let mut all_props_constant = true;
                let mut const_props = HashMap::new();
                
                for (prop_name, prop_value) in properties {
                    if let Some(value) = self.get_operand_constant_value(module, dataflow, prop_value, instr_id)? {
                        const_props.insert(prop_name.clone(), value.clone());
                    } else {
                        all_props_constant = false;
                        break;
                    }
                }
                
                if all_props_constant {
                    let result = ConstantValue::Object(const_props);
                    let old_value = self.constant_values.insert(instr_id, result.clone());
                    if old_value.as_ref() != Some(&result) {
                        changed = true;
                    }
                    
                    if let Some(var_id) = instr.defines_variable() {
                        let old_var_value = self.variable_constants.insert(var_id, result);
                        if old_var_value.is_none() || old_var_value.as_ref() != Some(&result) {
                            changed = true;
                        }
                    }
                }
            },
            Instruction::CreateArray(elements) => {
                // 要素が全て定数の場合、配列生成も定数
                let mut all_elements_constant = true;
                let mut const_elements = Vec::new();
                
                for element in elements {
                    if let Some(value) = self.get_operand_constant_value(module, dataflow, element, instr_id)? {
                        const_elements.push(value.clone());
                    } else {
                        all_elements_constant = false;
                        break;
                    }
                }
                
                if all_elements_constant {
                    let result = ConstantValue::Composite(const_elements);
                    let old_value = self.constant_values.insert(instr_id, result.clone());
                    if old_value.as_ref() != Some(&result) {
                        changed = true;
                    }
                    
                    if let Some(var_id) = instr.defines_variable() {
                        let old_var_value = self.variable_constants.insert(var_id, result);
                        if old_var_value.is_none() || old_var_value.as_ref() != Some(&result) {
                            changed = true;
                        }
                    }
                }
            },
            Instruction::TypeOf(operand) => {
                // オペランドが定数の場合、typeof演算子も定数
                if let Some(value) = self.get_operand_constant_value(module, dataflow, operand, instr_id)? {
                    let type_str = match value {
                        ConstantValue::Integer(_) => "number",
                        ConstantValue::Float(_) => "number",
                        ConstantValue::Boolean(_) => "boolean",
                        ConstantValue::String(_) => "string",
                        ConstantValue::Null => "object",
                        ConstantValue::Undefined => "undefined",
                        ConstantValue::Function(_) | ConstantValue::BuiltinFunction(_) => "function",
                        ConstantValue::Module(_) | ConstantValue::ModuleWithExports(_, _) => "object",
                        ConstantValue::Composite(_) => "object",
                        ConstantValue::Object(_) => "object",
                        ConstantValue::Type(_) => "function",
                        _ => "object",
                    };
                    
                    let result = ConstantValue::String(type_str.to_string());
                    let old_value = self.constant_values.insert(instr_id, result.clone());
                    if old_value.as_ref() != Some(&result) {
                        changed = true;
                    }
                    
                    if let Some(var_id) = instr.defines_variable() {
                        let old_var_value = self.variable_constants.insert(var_id, result);
                        if old_var_value.is_none() || old_var_value.as_ref() != Some(&result) {
                            changed = true;
                        }
                    }
                }
            },
            _ => {}
        }
        
        Ok(changed)
    }
    
    /// 特定のモジュール内の変数の定数値を取得
    fn get_variable_constant_in_module(&self, var_id: &VariableId, module_id: ModuleId) -> Result<Option<&ConstantValue>> {
        // モジュール内の変数の定数値を取得するロジック
        // 実際の実装では、モジュール間の変数参照を解決する必要がある
        Ok(self.variable_constants.get(var_id))
    }
    
    /// モジュールが定数かどうかを判定（全てのエクスポートが定数）
    fn is_module_constant(&self, module_id: ModuleId, module: &Module) -> Result<bool> {
        let exports = module.get_module_exports(module_id)?;
        
        for (_, export_id) in exports {
            match export_id {
                ExportId::Variable(var_id) => {
                    if let Some(var_module) = module.get_variable_module(var_id) {
                        if self.get_variable_constant_in_module(&var_id, var_module)?.is_none() {
                            return Ok(false);
                        }
                    } else {
                        return Ok(false);
                    }
                },
                ExportId::Module(nested_module_id) => {
                    if !self.is_module_constant(nested_module_id, module)? {
                        return Ok(false);
                    }
                },
                // 関数と型は常に定数
                ExportId::Function(_) | ExportId::Type(_) => {}
            }
        }
        
        Ok(true)
    }
    
    /// 純粋関数をコンパイル時に評価
    fn evaluate_constant_function(&self, func_id: FunctionId, args: &[ConstantValue], module: &Module) -> Result<Option<ConstantValue>> {
        // 関数の本体を取得
        let func_body = module.get_function_body(func_id)?;
        
        // 関数が単純な場合のみ評価（複雑な関数は実行時まで遅延）
        if func_body.is_simple_enough_for_constant_evaluation() {
            // 関数の引数と本体を使って評価
            let interpreter = ConstantInterpreter::new(module, self);
            interpreter.evaluate_function(func_id, args)
        } else {
            Ok(None)
        }
    }
    
    /// 組み込み関数をコンパイル時に評価
    fn evaluate_builtin_function(&self, func: &str, args: &[ConstantValue]) -> Result<Option<ConstantValue>> {
        match func {
            "Math.abs" => {
                if args.len() == 1 {
                    match &args[0] {
                        ConstantValue::Integer(i) => Ok(Some(ConstantValue::Integer(i.abs()))),
                        ConstantValue::Float(f) => Ok(Some(ConstantValue::Float(f.abs()))),
                        _ => Ok(None)
                    }
                } else {
                    Ok(None)
                }
            },
            "Math.max" => {
                if args.len() >= 2 {
                    let mut max_int: Option<i64> = None;
                    let mut max_float: Option<f64> = None;
                    let mut has_float = false;
                    
                    for arg in args {
                        match arg {
                            ConstantValue::Integer(i) => {
                                if !has_float {
                                    max_int = Some(max_int.map_or(*i, |m| m.max(*i)));
                                } else {
                                    max_float = Some(max_float.map_or(*i as f64, |m| m.max(*i as f64)));
                                }
                            },
                            ConstantValue::Float(f) => {
                                has_float = true;
                                if let Some(mi) = max_int {
                                    max_float = Some(max_float.map_or(mi as f64, |m| m.max(mi as f64)));
                                    max_int = None;
                                }
                                max_float = Some(max_float.map_or(*f, |m| m.max(*f)));
                            },
                            _ => return Ok(None)
                        }
                    }
                    
                    if has_float {
                        Ok(max_float.map(ConstantValue::Float))
                    } else {
                        Ok(max_int.map(ConstantValue::Integer))
                    }
                } else {
                    Ok(None)
                }
            },
            "Math.min" => {
                // Math.maxと同様の実装（最小値を求める）
                if args.len() >= 2 {
                    let mut min_int: Option<i64> = None;
                    let mut min_float: Option<f64> = None;
                    let mut has_float = false;
                    
                    for arg in args {
                        match arg {
                            ConstantValue::Integer(i) => {
                                if !has_float {
                                    min_int = Some(min_int.map_or(*i, |m| m.min(*i)));
                                } else {
                                    min_float = Some(min_float.map_or(*i as f64, |m| m.min(*i as f64)));
                                }
                            },
                            ConstantValue::Float(f) => {
                                has_float = true;
                                if let Some(mi) = min_int {
                                    min_float = Some(min_float.map_or(mi as f64, |m| m.min(mi as f64)));
                                    min_int = None;
                                }
                                min_float = Some(min_float.map_or(*f, |m| m.min(*f)));
                            },
                            _ => return Ok(None)
                        }
                    }
                    
                    if has_float {
                        Ok(min_float.map(ConstantValue::Float))
                    } else {
                        Ok(min_int.map(ConstantValue::Integer))
                    }
                } else {
                    Ok(None)
                }
            },
            "String.concat" => {
                let mut result = String::new();
                for arg in args {
                    if let Some(s) = Self::constant_to_string(arg) {
                        result.push_str(&s);
                    } else {
                        return Ok(None);
                    }
                }
                Ok(Some(ConstantValue::String(result)))
            },
            _ => Ok(None)
        }
    }
    /// 命令の定数値を取得
    pub fn get_constant_value(&self, instr_id: InstructionId) -> Option<&ConstantValue> {
        self.constant_values.get(&instr_id)
    }
    
    /// 変数の定数値を取得
    pub fn get_variable_constant(&self, var_id: VariableId) -> Option<&ConstantValue> {
        self.variable_constants.get(&var_id)
    }
    
    /// 分岐命令の結果を取得
    pub fn get_branch_outcome(&self, branch_id: InstructionId) -> Option<bool> {
        self.branch_outcomes.get(&branch_id).copied()
    }
    
    /// 関数呼び出しの結果を取得
    pub fn get_function_call_result(&self, call_id: InstructionId) -> Option<&ConstantValue> {
        self.function_call_results.get(&call_id)
    }
    
    /// 命令が定数かどうかを判定
    pub fn is_constant(&self, instr_id: InstructionId) -> bool {
        self.constant_values.contains_key(&instr_id)
    }
    
    /// 変数が定数かどうかを判定
    pub fn is_variable_constant(&self, var_id: VariableId) -> bool {
        self.variable_constants.contains_key(&var_id)
    }
    
    /// IR定数値をConstantValue列挙型に変換
    fn convert_to_constant_value(value: &ir::Constant) -> Result<ConstantValue> {
        match value {
            ir::Constant::Integer(i) => Ok(ConstantValue::Integer(*i)),
            ir::Constant::Float(f) => Ok(ConstantValue::Float(*f)),
            ir::Constant::Boolean(b) => Ok(ConstantValue::Boolean(*b)),
            ir::Constant::String(s) => Ok(ConstantValue::String(s.clone())),
            ir::Constant::Null => Ok(ConstantValue::Null),
            ir::Constant::Undefined => Ok(ConstantValue::Undefined),
            ir::Constant::Composite(elements) => {
                let mut const_elements = Vec::new();
                for elem in elements {
                    const_elements.push(Self::convert_to_constant_value(elem)?);
                }
                Ok(ConstantValue::Composite(const_elements))
            },
            ir::Constant::Tuple(elements) => {
                let mut const_elements = Vec::new();
                for elem in elements {
                    const_elements.push(Self::convert_to_constant_value(elem)?);
                }
                Ok(ConstantValue::Tuple(const_elements))
            },
            ir::Constant::Array(elements) => {
                let mut const_elements = Vec::new();
                for elem in elements {
                    const_elements.push(Self::convert_to_constant_value(elem)?);
                }
                Ok(ConstantValue::Array(const_elements))
            },
            ir::Constant::Map(entries) => {
                let mut const_map = HashMap::new();
                for (key, value) in entries {
                    const_map.insert(key.clone(), Self::convert_to_constant_value(value)?);
                }
                Ok(ConstantValue::Map(const_map))
            },
            ir::Constant::Function(func_id) => {
                Ok(ConstantValue::Function(*func_id))
            },
            ir::Constant::Type(type_id) => {
                Ok(ConstantValue::Type(*type_id))
            },
            ir::Constant::Optional(value) => {
                match value {
                    Some(v) => Ok(ConstantValue::Optional(Some(Box::new(Self::convert_to_constant_value(v)?)))),
                    None => Ok(ConstantValue::Optional(None)),
                }
            },
            ir::Constant::Range(start, end, step) => {
                let start_val = Self::convert_to_constant_value(start)?;
                let end_val = Self::convert_to_constant_value(end)?;
                let step_val = match step {
                    Some(s) => Some(Box::new(Self::convert_to_constant_value(s)?)),
                    None => None,
                };
                Ok(ConstantValue::Range(Box::new(start_val), Box::new(end_val), Box::new(step_val)))
            },
            _ => Err(Error::AnalysisError(format!("未対応の定数型: {:?}", value))),
        }
    }
    
    /// オペランドの定数値を取得
    fn get_operand_constant_value(
        &self,
        module: &Module,
        dataflow: &DataFlowAnalysis,
        operand: &Operand,
        use_point: InstructionId,
    ) -> Result<Option<ConstantValue>> {
        match operand {
            Operand::Constant(c) => Ok(Some(Self::convert_to_constant_value(c)?)),
            Operand::Variable(var_id) => {
                if let Some(const_val) = self.variable_constants.get(var_id) {
                    return Ok(Some(const_val.clone()));
                }
                
                // 変数の定義命令を取得して、その命令の定数値を確認
                if let Some(def_instr) = dataflow.get_reaching_def_for_var(use_point, *var_id)? {
                    if let Some(const_val) = self.constant_values.get(&def_instr) {
                        return Ok(Some(const_val.clone()));
                    }
                }
                
                Ok(None)
            },
            Operand::Instruction(instr_id) => {
                if let Some(const_val) = self.constant_values.get(instr_id) {
                    return Ok(Some(const_val.clone()));
                }
                Ok(None)
            },
            Operand::GlobalVariable(global_id) => {
                // グローバル変数が定数として初期化されているか確認
                if let Some(init_value) = module.get_global_initializer(*global_id)? {
                    return Ok(Some(Self::convert_to_constant_value(&init_value)?));
                }
                Ok(None)
            },
            Operand::FunctionReference(func_id) => {
                // 関数参照は常に定数
                if let Some(resolved_id) = module.resolve_function_reference(func_id) {
                    return Ok(Some(ConstantValue::Function(resolved_id)));
                }
                Ok(None)
            },
            Operand::TypeReference(type_id) => {
                // 型参照は常に定数
                Ok(Some(ConstantValue::Type(*type_id)))
            },
            _ => Ok(None),
        }
    }
    
    /// 二項演算の結果を評価
    fn evaluate_binary_op(
        op: &BinaryOperator,
        lhs: &ConstantValue,
        rhs: &ConstantValue,
    ) -> Result<Option<ConstantValue>> {
        match (op, lhs, rhs) {
            // 整数演算
            (BinaryOperator::Add, ConstantValue::Integer(a), ConstantValue::Integer(b)) => {
                a.checked_add(*b)
                    .map(ConstantValue::Integer)
                    .ok_or_else(|| Error::AnalysisError("整数オーバーフロー".to_string()))
                    .map(Some)
            },
            (BinaryOperator::Subtract, ConstantValue::Integer(a), ConstantValue::Integer(b)) => {
                a.checked_sub(*b)
                    .map(ConstantValue::Integer)
                    .ok_or_else(|| Error::AnalysisError("整数アンダーフロー".to_string()))
                    .map(Some)
            },
            (BinaryOperator::Multiply, ConstantValue::Integer(a), ConstantValue::Integer(b)) => {
                a.checked_mul(*b)
                    .map(ConstantValue::Integer)
                    .ok_or_else(|| Error::AnalysisError("整数オーバーフロー".to_string()))
                    .map(Some)
            },
            (BinaryOperator::Divide, ConstantValue::Integer(a), ConstantValue::Integer(b)) => {
                if *b == 0 {
                    return Err(Error::AnalysisError("ゼロ除算".to_string()));
                }
                a.checked_div(*b)
                    .map(ConstantValue::Integer)
                    .ok_or_else(|| Error::AnalysisError("整数除算エラー".to_string()))
                    .map(Some)
            },
            (BinaryOperator::Modulo, ConstantValue::Integer(a), ConstantValue::Integer(b)) => {
                if *b == 0 {
                    return Err(Error::AnalysisError("ゼロによる剰余演算".to_string()));
                }
                a.checked_rem(*b)
                    .map(ConstantValue::Integer)
                    .ok_or_else(|| Error::AnalysisError("整数剰余演算エラー".to_string()))
                    .map(Some)
            },
            (BinaryOperator::BitwiseAnd, ConstantValue::Integer(a), ConstantValue::Integer(b)) => {
                Ok(Some(ConstantValue::Integer(a & b)))
            },
            (BinaryOperator::BitwiseOr, ConstantValue::Integer(a), ConstantValue::Integer(b)) => {
                Ok(Some(ConstantValue::Integer(a | b)))
            },
            (BinaryOperator::BitwiseXor, ConstantValue::Integer(a), ConstantValue::Integer(b)) => {
                Ok(Some(ConstantValue::Integer(a ^ b)))
            },
            (BinaryOperator::ShiftLeft, ConstantValue::Integer(a), ConstantValue::Integer(b)) => {
                if *b < 0 || *b >= 64 {
                    return Err(Error::AnalysisError(format!("無効なシフト量: {}", b)));
                }
                a.checked_shl(*b as u32)
                    .map(ConstantValue::Integer)
                    .ok_or_else(|| Error::AnalysisError("シフトオーバーフロー".to_string()))
                    .map(Some)
            },
            (BinaryOperator::ShiftRight, ConstantValue::Integer(a), ConstantValue::Integer(b)) => {
                if *b < 0 || *b >= 64 {
                    return Err(Error::AnalysisError(format!("無効なシフト量: {}", b)));
                }
                a.checked_shr(*b as u32)
                    .map(ConstantValue::Integer)
                    .ok_or_else(|| Error::AnalysisError("シフトオーバーフロー".to_string()))
                    .map(Some)
            },
            
            // 浮動小数点演算
            (BinaryOperator::Add, ConstantValue::Float(a), ConstantValue::Float(b)) => {
                Ok(Some(ConstantValue::Float(a + b)))
            },
            (BinaryOperator::Subtract, ConstantValue::Float(a), ConstantValue::Float(b)) => {
                Ok(Some(ConstantValue::Float(a - b)))
            },
            (BinaryOperator::Multiply, ConstantValue::Float(a), ConstantValue::Float(b)) => {
                Ok(Some(ConstantValue::Float(a * b)))
            },
            (BinaryOperator::Divide, ConstantValue::Float(a), ConstantValue::Float(b)) => {
                if *b == 0.0 {
                    return Err(Error::AnalysisError("浮動小数点数のゼロ除算".to_string()));
                }
                Ok(Some(ConstantValue::Float(a / b)))
            },
            (BinaryOperator::Modulo, ConstantValue::Float(a), ConstantValue::Float(b)) => {
                if *b == 0.0 {
                    return Err(Error::AnalysisError("浮動小数点数のゼロによる剰余演算".to_string()));
                }
                Ok(Some(ConstantValue::Float(a % b)))
            },
            
            // 文字列演算
            (BinaryOperator::Add, ConstantValue::String(a), ConstantValue::String(b)) => {
                let mut result = a.clone();
                result.push_str(b);
                Ok(Some(ConstantValue::String(result)))
            },
            
            // 論理演算
            (BinaryOperator::And, ConstantValue::Boolean(a), ConstantValue::Boolean(b)) => {
                Ok(Some(ConstantValue::Boolean(*a && *b)))
            },
            (BinaryOperator::Or, ConstantValue::Boolean(a), ConstantValue::Boolean(b)) => {
                Ok(Some(ConstantValue::Boolean(*a || *b)))
            },
            // 比較演算
            (BinaryOperator::Equal, a, b) => {
                Ok(Some(ConstantValue::Boolean(a == b)))
            },
            // その他の演算子も同様に実装...
            _ => Ok(None), // 評価できない場合はNoneを返す
        }
    }
    
    /// 単項演算の結果を評価
    fn evaluate_unary_op(
        op: &UnaryOperator,
        operand: &ConstantValue,
    ) -> Result<Option<ConstantValue>> {
        match (op, operand) {
            (UnaryOperator::Negate, ConstantValue::Integer(a)) => {
                Ok(Some(ConstantValue::Integer(-a)))
            },
            (UnaryOperator::Negate, ConstantValue::Float(a)) => {
                Ok(Some(ConstantValue::Float(-a)))
            },
            (UnaryOperator::Not, ConstantValue::Boolean(a)) => {
                Ok(Some(ConstantValue::Boolean(!a)))
            },
            // その他の演算子も同様に実装...
            _ => Ok(None), // 評価できない場合はNoneを返す
        }
    }
    
    /// 純粋関数の結果を評価
    fn evaluate_pure_function(
        module: &Module,
        func_id: FunctionId,
        args: &[ConstantValue],
    ) -> Result<Option<ConstantValue>> {
        // 組み込み関数の場合は特別な処理
        if module.is_builtin_function(func_id)? {
            return Self::evaluate_builtin_function(module, func_id, args);
        }
        
        // ユーザー定義関数の場合は、関数の実装を解析して定数評価を試みる
        // （実際の実装では、関数の本体を解析して定数評価を行う複雑なロジックが必要）
        
        // この例では単純化のためにNoneを返す
        Ok(None)
    }
    
    /// 組み込み関数の結果を評価
    fn evaluate_builtin_function(
        module: &Module,
        func_id: FunctionId,
        args: &[ConstantValue],
    ) -> Result<Option<ConstantValue>> {
        let func_name = module.get_function_name(func_id)?;
    fn run(module: &Module, _dataflow: &DataFlowAnalysis) -> Result<Self> {
        // 実際の解析を実行
        Ok(Self {})
    }
}

impl AnalysisResult for ConstantPropagationAnalysis {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}