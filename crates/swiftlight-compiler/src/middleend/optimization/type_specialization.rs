// type_specialization.rs - SwiftLight 型特化最適化
//
// このモジュールでは、型情報に基づいた高度な最適化を実装します。
// ジェネリック関数や多相的なコードを、使用される具体的な型に特化させることで、
// 実行時のオーバーヘッドを削減し、パフォーマンスを向上させます。
//
// 主な機能:
// - モノモーフィゼーション: ジェネリック関数を具体的な型に特化
// - インターフェース消去: トレイト/インターフェースの動的ディスパッチを静的ディスパッチに変換
// - 型クラス特化: 型クラスのメソッド呼び出しを特定型に最適化
// - データレイアウト最適化: 型に基づいたメモリレイアウトの最適化
// - 依存型特化: 依存型を含む関数の特殊化
// - 型レベル計算の実体化: コンパイル時の型計算結果を組み込み

use std::collections::{HashMap, HashSet, VecDeque, BTreeMap};
use std::fmt;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use crate::middleend::ir::{
    BasicBlock, Function, Instruction, Module, Value, ValueId,
    Type, TypeId, InstructionId, FunctionId, TypeParameter
};
use crate::middleend::analysis::dataflow::DataFlowEngine;
use crate::middleend::analysis::lifetime::LifetimeAnalyzer;
use crate::middleend::optimization::inlining::InliningDecider;

/// 型特化最適化の種類
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SpecializationKind {
    /// ジェネリック関数のモノモーフィゼーション
    Monomorphization,
    /// インターフェース消去
    InterfaceElimination,
    /// 型クラス特化
    TypeClassSpecialization,
    /// データレイアウト最適化
    DataLayoutOptimization,
    /// 依存型特化
    DependentTypeSpecialization,
    /// 型レベル計算の実体化
    TypeLevelComputationMaterialization,
}

impl fmt::Display for SpecializationKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Monomorphization => write!(f, "モノモーフィゼーション"),
            Self::InterfaceElimination => write!(f, "インターフェース消去"),
            Self::TypeClassSpecialization => write!(f, "型クラス特化"),
            Self::DataLayoutOptimization => write!(f, "データレイアウト最適化"),
            Self::DependentTypeSpecialization => write!(f, "依存型特化"),
            Self::TypeLevelComputationMaterialization => write!(f, "型レベル計算実体化"),
        }
    }
}

/// 型特化の候補
#[derive(Debug, Clone)]
pub struct SpecializationCandidate {
    /// 特化対象の関数ID
    pub function_id: FunctionId,
    /// 特化する型引数のリスト (型ID)
    pub type_arguments: Vec<TypeId>,
    /// 特化の種類
    pub kind: SpecializationKind,
    /// 推定される利益スコア（高いほど利益が大きい）
    pub benefit_score: f64,
    /// 推定されるコードサイズ増加（バイト）
    pub estimated_size_increase: usize,
    /// ホットパス上にあるかどうか
    pub is_on_hot_path: bool,
    /// 呼び出し回数
    pub call_count: usize,
    /// 依存型の値引数（依存型特化の場合）
    pub value_arguments: Option<Vec<ValueId>>,
}

/// 型特化の結果を表す構造体
#[derive(Debug, Clone)]
pub struct SpecializationResult {
    /// 特化元の関数ID
    pub original_function_id: FunctionId,
    /// 特化された関数ID
    pub specialized_function_id: FunctionId,
    /// 特化された型引数のリスト (型ID)
    pub type_arguments: Vec<TypeId>,
    /// 置き換えられた呼び出し命令のリスト
    pub replaced_calls: Vec<InstructionId>,
    /// 特化の種類
    pub kind: SpecializationKind,
    /// 推定されるパフォーマンス改善率
    pub estimated_improvement: f64,
    /// 実際のコードサイズ増加（バイト）
    pub actual_size_increase: usize,
    /// 特化にかかった時間（ミリ秒）
    pub specialization_time_ms: u64,
    /// 最適化された命令数
    pub optimized_instruction_count: usize,
    /// 削除された動的ディスパッチ数
    pub eliminated_dynamic_dispatches: usize,
}

/// 型特化の統計情報
#[derive(Debug, Default, Clone)]
pub struct SpecializationStats {
    /// 特化候補の総数
    pub total_candidates: usize,
    /// 実行された特化の数（種類別）
    pub executed_specializations: HashMap<SpecializationKind, usize>,
    /// 特化によるコードサイズ増加の合計（バイト）
    pub total_code_size_increase: usize,
    /// 特化による推定パフォーマンス改善率の平均
    pub average_improvement: f64,
    /// 特化にかかった合計時間（ミリ秒）
    pub total_time_ms: u64,
    /// 削除された動的ディスパッチの合計数
    pub total_eliminated_dynamic_dispatches: usize,
    /// 特化によって最適化された命令の合計数
    pub total_optimized_instructions: usize,
}

/// 型特化マネージャー
/// モジュール全体の型特化を管理し、最適なタイミングと対象を選択
pub struct TypeSpecializationManager {
    /// 現在処理中のモジュール
    module: Option<Module>,
    /// 特化候補のリスト
    candidates: Vec<SpecializationCandidate>,
    /// 既に特化済みの関数マップ（元関数ID + 型引数のハッシュ → 特化関数ID）
    specialized_functions: HashMap<(FunctionId, Vec<TypeId>), FunctionId>,
    /// 型特化の結果リスト
    results: Vec<SpecializationResult>,
    /// 特化実行中かどうかのフラグ
    in_progress: bool,
    /// メモリ使用量制限（バイト）
    memory_limit: usize,
    /// 現在のメモリ使用量推定（バイト）
    estimated_memory_usage: usize,
    /// コード膨張係数（特化によるコードサイズ増加の制御用）
    code_expansion_factor: f64,
    /// 特化の統計情報
    stats: SpecializationStats,
    /// 特化の優先度マップ（関数ID → 優先度）
    priority_map: HashMap<FunctionId, f64>,
    /// 型レベル計算エンジン
    type_level_engine: Option<Arc<Mutex<TypeLevelComputationEngine>>>,
    /// データフロー解析エンジン
    dataflow_engine: Option<DataFlowEngine>,
    /// ライフタイム解析エンジン
    lifetime_analyzer: Option<LifetimeAnalyzer>,
    /// インライン化決定エンジン
    inlining_decider: Option<InliningDecider>,
    /// 特化キャッシュ（過去の特化結果を再利用）
    specialization_cache: BTreeMap<(FunctionId, Vec<TypeId>, SpecializationKind), SpecializationResult>,
    /// 特化の最大再帰深度
    max_recursion_depth: usize,
    /// 現在の再帰深度
    current_recursion_depth: usize,
    /// 特化の最大数
    max_specializations: usize,
    /// 特化の最小利益スコア
    min_benefit_score: f64,
    /// 特化の最大コードサイズ増加率
    max_code_size_increase_ratio: f64,
    /// 特化の最大時間（ミリ秒）
    max_time_ms: u64,
    /// 開始時間
    start_time: Option<Instant>,
    /// 特化の依存グラフ（特化ID → 依存する特化IDのリスト）
    dependency_graph: HashMap<FunctionId, Vec<FunctionId>>,
    /// 型依存関係グラフ（型ID → 依存する型IDのリスト）
    type_dependency_graph: HashMap<TypeId, Vec<TypeId>>,
    /// 特化の実行順序キュー
    specialization_queue: VecDeque<SpecializationCandidate>,
    /// 特化の実行履歴
    specialization_history: Vec<(FunctionId, SpecializationKind, f64)>,
    /// 型特化の制約条件
    specialization_constraints: HashMap<FunctionId, Vec<SpecializationConstraint>>,
    /// 型特化の最適化ヒント
    optimization_hints: HashMap<FunctionId, Vec<OptimizationHint>>,
    /// 型特化の安全性検証フラグ
    safety_verification_enabled: bool,
    /// 型特化の並列実行フラグ
    parallel_execution_enabled: bool,
    /// 型特化の自動調整フラグ
    auto_tuning_enabled: bool,
    /// 型特化の進捗報告コールバック
    progress_callback: Option<Box<dyn Fn(f64, &str) + Send + Sync>>,
    /// 型特化の詳細ログ記録フラグ
    detailed_logging_enabled: bool,
    /// 型特化の実験的機能フラグ
    experimental_features_enabled: bool,
    /// 型特化の最適化レベル（0-3）
    optimization_level: u8,
    /// 型特化のプロファイリングデータ
    profiling_data: Option<HashMap<FunctionId, FunctionProfile>>,
    /// 型特化の依存型解析エンジン
    dependent_type_analyzer: Option<Arc<Mutex<DependentTypeAnalyzer>>>,
    /// 型特化のメタプログラミングエンジン
    metaprogramming_engine: Option<Arc<Mutex<MetaprogrammingEngine>>>,
}

/// 型レベル計算エンジン
pub struct TypeLevelComputationEngine {
    // 型レベル計算の状態
    state: HashMap<String, TypeId>,
    // 型レベル関数マップ
    type_functions: HashMap<String, Box<dyn Fn(&[TypeId]) -> Result<TypeId, String> + Send + Sync>>,
    // 型レベル計算の評価キャッシュ
    evaluation_cache: HashMap<(String, Vec<TypeId>), Result<TypeId, String>>,
    // 型レベル計算の依存関係グラフ
    dependency_graph: HashMap<String, Vec<String>>,
    // 型レベル計算の実行統計
    execution_stats: HashMap<String, TypeLevelExecutionStats>,
    // 型レベル計算の最大再帰深度
    max_recursion_depth: usize,
    // 型レベル計算の現在の再帰深度
    current_recursion_depth: usize,
    // 型レベル計算のタイムアウト（ミリ秒）
    timeout_ms: u64,
    // 型レベル計算の開始時間
    start_time: Option<Instant>,
    // 型レベル計算の安全モードフラグ
    safe_mode: bool,
    // 型レベル計算のトレースフラグ
    trace_enabled: bool,
    // 型レベル計算のトレースログ
    trace_log: Vec<TypeLevelTraceEntry>,
    // 型レベル計算の型環境
    type_environment: HashMap<String, TypeSchema>,
    // 型レベル計算の型制約
    type_constraints: Vec<TypeConstraint>,
    // 型レベル計算の型推論エンジン
    type_inference_engine: Option<Box<dyn TypeInferenceEngine>>,
}

/// 型レベル計算の実行統計
#[derive(Debug, Default, Clone)]
struct TypeLevelExecutionStats {
    // 呼び出し回数
    call_count: usize,
    // 合計実行時間（ミリ秒）
    total_execution_time_ms: u64,
    // 平均実行時間（ミリ秒）
    average_execution_time_ms: f64,
    // 最大実行時間（ミリ秒）
    max_execution_time_ms: u64,
    // キャッシュヒット回数
    cache_hits: usize,
    // キャッシュミス回数
    cache_misses: usize,
    // エラー発生回数
    error_count: usize,
    // 最後のエラーメッセージ
    last_error: Option<String>,
}

/// 型レベル計算のトレースエントリ
#[derive(Debug, Clone)]
struct TypeLevelTraceEntry {
    // 関数名
    function_name: String,
    // 引数
    arguments: Vec<TypeId>,
    // 結果
    result: Result<TypeId, String>,
    // 実行時間（ミリ秒）
    execution_time_ms: u64,
    // 再帰深度
    recursion_depth: usize,
    // タイムスタンプ
    timestamp: std::time::SystemTime,
}

/// 型スキーマ
#[derive(Debug, Clone)]
struct TypeSchema {
    // 型変数
    variables: Vec<String>,
    // 型本体
    body: TypeExpression,
    // 型制約
    constraints: Vec<TypeConstraint>,
}

/// 型表現
#[derive(Debug, Clone)]
enum TypeExpression {
    // 型変数
    Variable(String),
    // 型定数
    Constant(TypeId),
    // 型適用
    Application {
        constructor: Box<TypeExpression>,
        arguments: Vec<TypeExpression>,
    },
    // 関数型
    Function {
        parameters: Vec<TypeExpression>,
        return_type: Box<TypeExpression>,
    },
    // 依存型
    Dependent {
        name: String,
        value_type: Box<TypeExpression>,
        body: Box<TypeExpression>,
    },
    // 型レベル演算
    Operation {
        operator: String,
        operands: Vec<TypeExpression>,
    },
    // 型レベル条件式
    Conditional {
        condition: Box<TypeExpression>,
        then_type: Box<TypeExpression>,
        else_type: Box<TypeExpression>,
    },
    // 型レベルラムダ
    Lambda {
        parameters: Vec<String>,
        body: Box<TypeExpression>,
    },
    // 型レベル適用
    Apply {
        lambda: Box<TypeExpression>,
        arguments: Vec<TypeExpression>,
    },
    // 型レベルlet束縛
    Let {
        bindings: Vec<(String, TypeExpression)>,
        body: Box<TypeExpression>,
    },
}

/// 型制約
#[derive(Debug, Clone)]
enum TypeConstraint {
    // 等価制約
    Equality(TypeExpression, TypeExpression),
    // サブタイプ制約
    Subtype(TypeExpression, TypeExpression),
    // トレイト制約
    Trait {
        trait_name: String,
        type_expr: TypeExpression,
    },
    // 値依存制約
    ValueDependent {
        value_expr: String,
        type_expr: TypeExpression,
    },
    // 複合制約（AND）
    And(Vec<TypeConstraint>),
    // 複合制約（OR）
    Or(Vec<TypeConstraint>),
    // 否定制約
    Not(Box<TypeConstraint>),
    // 条件付き制約
    Conditional {
        condition: Box<TypeConstraint>,
        then_constraint: Box<TypeConstraint>,
        else_constraint: Box<TypeConstraint>,
    },
}

/// 型推論エンジンのトレイト
trait TypeInferenceEngine: Send + Sync {
    // 型推論を実行
    fn infer(&self, expr: &TypeExpression, env: &HashMap<String, TypeSchema>) -> Result<TypeId, String>;
    // 型制約を解決
    fn solve_constraints(&self, constraints: &[TypeConstraint]) -> Result<HashMap<String, TypeId>, String>;
    // 型の単一化
    fn unify(&self, type1: &TypeExpression, type2: &TypeExpression) -> Result<HashMap<String, TypeExpression>, String>;
    // 型の汎化
    fn generalize(&self, expr: &TypeExpression, env: &HashMap<String, TypeSchema>) -> TypeSchema;
    // 型の具体化
    fn instantiate(&self, schema: &TypeSchema) -> TypeExpression;
}

/// 特化制約
#[derive(Debug, Clone)]
enum SpecializationConstraint {
    // メモリ使用量制約
    MemoryLimit(usize),
    // 実行時間制約
    TimeLimit(u64),
    // コードサイズ制約
    CodeSizeLimit(usize),
    // 再帰深度制約
    RecursionDepthLimit(usize),
    // 型引数制約
    TypeArgumentConstraint {
        param_index: usize,
        allowed_types: Vec<TypeId>,
    },
    // 特化種類制約
    KindConstraint(Vec<SpecializationKind>),
    // 利益スコア制約
    BenefitScoreConstraint(f64),
    // 複合制約（AND）
    And(Vec<SpecializationConstraint>),
    // 複合制約（OR）
    Or(Vec<SpecializationConstraint>),
    // 否定制約
    Not(Box<SpecializationConstraint>),
    // カスタム制約
    Custom(Box<dyn Fn(&SpecializationCandidate) -> bool + Send + Sync>),
}

/// 最適化ヒント
#[derive(Debug, Clone)]
enum OptimizationHint {
    // インライン化ヒント
    Inline {
        threshold: Option<usize>,
        force: bool,
    },
    // ループアンロールヒント
    LoopUnroll {
        factor: usize,
    },
    // ベクトル化ヒント
    Vectorize {
        width: usize,
    },
    // メモリアライメントヒント
    MemoryAlignment {
        alignment: usize,
    },
    // 並列化ヒント
    Parallelize {
        min_work_size: usize,
    },
    // 定数伝播ヒント
    ConstantPropagation {
        aggressive: bool,
    },
    // 共通部分式除去ヒント
    CommonSubexpressionElimination {
        aggressive: bool,
    },
    // デッドコード除去ヒント
    DeadCodeElimination {
        aggressive: bool,
    },
    // 型特化ヒント
    TypeSpecialization {
        kinds: Vec<SpecializationKind>,
        priority: f64,
    },
    // カスタムヒント
    Custom {
        name: String,
        parameters: HashMap<String, String>,
    },
}

/// 関数プロファイル
#[derive(Debug, Clone)]
struct FunctionProfile {
    // 呼び出し回数
    call_count: usize,
    // ホットパス上かどうか
    is_on_hot_path: bool,
    // 実行時間の割合（全体を1.0とした場合）
    execution_time_ratio: f64,
    // 平均実行時間（ナノ秒）
    average_execution_time_ns: u64,
    // 命令数
    instruction_count: usize,
    // 基本ブロック数
    basic_block_count: usize,
    // 循環的複雑度
    cyclomatic_complexity: usize,
    // 呼び出し元関数のリスト
    callers: Vec<FunctionId>,
    // 呼び出し先関数のリスト
    callees: Vec<FunctionId>,
    // メモリアクセスパターン
    memory_access_pattern: MemoryAccessPattern,
    // 型パラメータの使用パターン
    type_parameter_usage: HashMap<usize, TypeParameterUsage>,
    // 最後の呼び出し時刻
    last_call_timestamp: std::time::SystemTime,
    // 特化履歴
    specialization_history: Vec<(SpecializationKind, f64)>,
}

/// メモリアクセスパターン
#[derive(Debug, Clone, Default)]
struct MemoryAccessPattern {
    // 読み取りアクセス数
    read_count: usize,
    // 書き込みアクセス数
    write_count: usize,
    // 連続アクセス数
    sequential_access_count: usize,
    // ランダムアクセス数
    random_access_count: usize,
    // キャッシュ局所性スコア（0.0-1.0）
    cache_locality_score: f64,
    // アライメントされたアクセス数
    aligned_access_count: usize,
    // アライメントされていないアクセス数
    unaligned_access_count: usize,
    // 共有メモリアクセス数
    shared_memory_access_count: usize,
    // プライベートメモリアクセス数
    private_memory_access_count: usize,
}

/// 型パラメータの使用パターン
#[derive(Debug, Clone, Default)]
struct TypeParameterUsage {
    // 使用回数
    usage_count: usize,
    // メソッド呼び出し回数
    method_call_count: usize,
    // 型キャスト回数
    type_cast_count: usize,
    // サイズ依存操作回数
    size_dependent_operation_count: usize,
    // アライメント依存操作回数
    alignment_dependent_operation_count: usize,
    // トレイトメソッド使用回数
    trait_method_usage_count: usize,
    // 具体的な型として使用された回数
    concrete_type_usage_count: usize,
    // 型構築に使用された回数
    type_construction_count: usize,
}

/// 依存型解析エンジン
pub struct DependentTypeAnalyzer {
    // 依存型の環境
    environment: HashMap<String, DependentTypeSchema>,
    // 依存型の制約ソルバー
    constraint_solver: DependentTypeConstraintSolver,
    // 依存型の評価エンジン
    evaluation_engine: DependentTypeEvaluationEngine,
    // 依存型の検証エンジン
    verification_engine: DependentTypeVerificationEngine,
    // 依存型の推論エンジン
    inference_engine: DependentTypeInferenceEngine,
    // 依存型の特化エンジン
    specialization_engine: DependentTypeSpecializationEngine,
    // 依存型の最適化エンジン
    optimization_engine: DependentTypeOptimizationEngine,
    // 依存型の安全性検証フラグ
    safety_verification_enabled: bool,
    // 依存型の詳細ログ記録フラグ
    detailed_logging_enabled: bool,
    // 依存型の実験的機能フラグ
    experimental_features_enabled: bool,
}

/// 依存型スキーマ
#[derive(Debug, Clone)]
struct DependentTypeSchema {
    // 型変数
    type_variables: Vec<String>,
    // 値変数
    value_variables: Vec<(String, TypeId)>,
    // 型本体
    body: DependentTypeExpression,
    // 型制約
    constraints: Vec<DependentTypeConstraint>,
}

/// 依存型表現
#[derive(Debug, Clone)]
enum DependentTypeExpression {
    // 型変数
    TypeVariable(String),
    // 値変数
    ValueVariable(String),
    // 型定数
    TypeConstant(TypeId),
    // 値定数
    ValueConstant(ValueId),
    // 型適用
    TypeApplication {
        constructor: Box<DependentTypeExpression>,
        arguments: Vec<DependentTypeExpression>,
    },
    // 関数型
    FunctionType {
        parameters: Vec<(String, DependentTypeExpression)>,
        return_type: Box<DependentTypeExpression>,
    },
    // 依存積型
    DependentProduct {
        name: String,
        domain: Box<DependentTypeExpression>,
        codomain: Box<DependentTypeExpression>,
    },
    // 依存和型
    DependentSum {
        name: String,
        domain: Box<DependentTypeExpression>,
        codomain: Box<DependentTypeExpression>,
    },
    // 型レベル演算
    TypeOperation {
        operator: String,
        operands: Vec<DependentTypeExpression>,
    },
    // 値レベル演算
    ValueOperation {
        operator: String,
        operands: Vec<DependentTypeExpression>,
    },
    // 型レベル条件式
    TypeConditional {
        condition: Box<DependentTypeExpression>,
        then_type: Box<DependentTypeExpression>,
        else_type: Box<DependentTypeExpression>,
    },
    // 型レベルラムダ
    TypeLambda {
        parameters: Vec<String>,
        body: Box<DependentTypeExpression>,
    },
    // 値レベルラムダ
    ValueLambda {
        parameters: Vec<(String, DependentTypeExpression)>,
        body: Box<DependentTypeExpression>,
    },
    // 型レベル適用
    TypeApply {
        lambda: Box<DependentTypeExpression>,
        arguments: Vec<DependentTypeExpression>,
    },
    // 値レベル適用
    ValueApply {
        lambda: Box<DependentTypeExpression>,
        arguments: Vec<DependentTypeExpression>,
    },
    // 型レベルlet束縛
    TypeLet {
        bindings: Vec<(String, DependentTypeExpression)>,
        body: Box<DependentTypeExpression>,
    },
    // 値レベルlet束縛
    ValueLet {
        bindings: Vec<(String, DependentTypeExpression)>,
        body: Box<DependentTypeExpression>,
    },
    // 型シングルトン
    Singleton(Box<DependentTypeExpression>),
    // 型等価性証明
    Equality(Box<DependentTypeExpression>, Box<DependentTypeExpression>),
    // 型レベル帰納的定義
    Inductive {
        name: String,
        parameters: Vec<(String, DependentTypeExpression)>,
        constructors: Vec<(String, DependentTypeExpression)>,
    },
    // 型レベル再帰
    Recursive {
        name: String,
        body: Box<DependentTypeExpression>,
    },
}

/// 依存型制約
#[derive(Debug, Clone)]
enum DependentTypeConstraint {
    // 型等価制約
    TypeEquality(DependentTypeExpression, DependentTypeExpression),
    // 値等価制約
    ValueEquality(DependentTypeExpression, DependentTypeExpression),
    // サブタイプ制約
    Subtype(DependentTypeExpression, DependentTypeExpression),
    // トレイト制約
    Trait {
        trait_name: String,
        type_expr: DependentTypeExpression,
    },
    // 値依存制約
    ValueDependent {
        value_expr: DependentTypeExpression,
        type_expr: DependentTypeExpression,
    },
    // 複合制約（AND）
    And(Vec<DependentTypeConstraint>),
    // 複合制約（OR）
    Or(Vec<DependentTypeConstraint>),
    // 否定制約
    Not(Box<DependentTypeConstraint>),
    // 条件付き制約
    Conditional {
        condition: Box<DependentTypeConstraint>,
        then_constraint: Box<DependentTypeConstraint>,
        else_constraint: Box<DependentTypeConstraint>,
    },
    // 全称量化制約
    ForAll {
        variables: Vec<(String, DependentTypeExpression)>,
impl TypeLevelComputationEngine {
    /// 新しい型レベル計算エンジンを作成
    pub fn new() -> Self {
        Self {
            state: HashMap::new(),
            type_functions: HashMap::new(),
        }
    }
    
    /// 型レベル関数を登録
    pub fn register_function<F>(&mut self, name: &str, func: F)
    where
        F: Fn(&[TypeId]) -> Result<TypeId, String> + 'static + Send + Sync,
    {
        self.type_functions.insert(name.to_string(), Box::new(func));
    }
    /// 型レベル計算を実行
    pub fn evaluate(&mut self, expr: &str, args: &[TypeId]) -> Result<TypeId, String> {
        self.type_functions
            .get(expr)
            .map(|func| func(args))
            .ok_or_else(|| format!("未定義の型レベル関数: {}", expr))
    }

    /// 構造体初期化用のデフォルト実装
    fn new() -> Self {
        Self {
            candidates: Vec::new(),
            specialized_functions: HashMap::new(),
            results: Vec::new(),
            in_progress: false,
            memory_limit: 100 * 1024 * 1024, // デフォルト: 100MB
            estimated_memory_usage: 0,
            code_expansion_factor: 1.5,      // 最大1.5倍までのコード膨張を許容
            stats: SpecializationStats::default(),
            priority_map: HashMap::new(),
            type_level_engine: Some(Arc::new(Mutex::new(TypeLevelComputationEngine::new()))),
            dataflow_engine: None,
            lifetime_analyzer: None,
            inlining_decider: None,
            specialization_cache: BTreeMap::new(),
            max_recursion_depth: 5,
            current_recursion_depth: 0,
            max_specializations: 1000,
            min_benefit_score: 0.5,
            max_code_size_increase_ratio: 0.3, // 最大30%のコードサイズ増加を許容
            max_time_ms: 10000,              // 最大10秒
            start_time: None,
            dependency_graph: HashMap::new(),
        }
    }

    /// モジュールを設定
    pub fn set_module(&mut self, module: Module) {
        self.module = Some(module);
        self.candidates.clear();
        self.results.clear();
        self.in_progress = false;
        self.estimated_memory_usage = self.estimate_module_size();
        self.stats = SpecializationStats::default();
        self.start_time = Some(Instant::now());

        // データフロー解析エンジンを初期化
        if let Some(module) = &self.module {
            self.dataflow_engine = Some(DataFlowEngine::new(module.clone()));
            self.lifetime_analyzer = Some(LifetimeAnalyzer::new(module.clone()));
            self.inlining_decider = Some(InliningDecider::new());
        }
    }

    /// メモリ制限を設定
    pub fn set_memory_limit(&mut self, limit_bytes: usize) {
        self.memory_limit = limit_bytes;
    }

    /// コード膨張係数を設定
    pub fn set_code_expansion_factor(&mut self, factor: f64) {
        assert!(
            factor >= 1.0,
            "コード膨張係数は1.0以上である必要があります"
        );
        self.code_expansion_factor = factor;
    }

    /// 最大再帰深度を設定
    pub fn set_max_recursion_depth(&mut self, depth: usize) {
        self.max_recursion_depth = depth;
    }

    /// 最大特化数を設定
    pub fn set_max_specializations(&mut self, count: usize) {
        self.max_specializations = count;
    }

    /// 最小利益スコアを設定
    pub fn set_min_benefit_score(&mut self, score: f64) {
        self.min_benefit_score = score;
    }
    /// 最大コードサイズ増加率を設定
    pub fn set_max_code_size_increase_ratio(&mut self, ratio: f64) {
        self.max_code_size_increase_ratio = ratio;
    }
    
    /// 最大時間を設定
    pub fn set_max_time_ms(&mut self, time_ms: u64) {
        self.max_time_ms = time_ms;
    }
    
    /// 型特化最適化を実行
    pub fn run_specialization(&mut self) -> Result<Vec<SpecializationResult>, String> {
        let module = self.module.as_mut().ok_or("モジュールが設定されていません")?;
        
        // 開始時間を記録
        self.start_time = Some(Instant::now());
        
        // 特化候補の収集
        self.collect_candidates()?;
        self.stats.total_candidates = self.candidates.len();
        
        // 候補を利益スコアで並べ替え
        self.candidates.sort_by(|a, b| {
            // まずホットパス上の候補を優先
            if a.is_on_hot_path && !b.is_on_hot_path {
                return std::cmp::Ordering::Less;
            }
            if !a.is_on_hot_path && b.is_on_hot_path {
                return std::cmp::Ordering::Greater;
            }
            
            // 次に利益スコアで比較
            b.benefit_score.partial_cmp(&a.benefit_score).unwrap_or(std::cmp::Ordering::Equal)
        });
        
        // 特化の実行
        self.in_progress = true;
        let mut specialization_count = 0;
        
        // 特化の依存グラフを構築
        self.build_dependency_graph()?;
        
        // 依存関係に基づいて特化を実行
        let mut executed_functions = HashSet::new();
        let mut remaining_candidates = self.candidates.clone();
        
        while !remaining_candidates.is_empty() && specialization_count < self.max_specializations {
            // 時間制限をチェック
            if let Some(start_time) = self.start_time {
                let elapsed = start_time.elapsed().as_millis() as u64;
                if elapsed > self.max_time_ms {
                    break;
                }
            }
            
            // 依存関係のない候補を選択
            let mut selected_index = None;
            for (i, candidate) in remaining_candidates.iter().enumerate() {
                let deps = self.dependency_graph.get(&candidate.function_id).unwrap_or(&Vec::new());
                if deps.iter().all(|dep_id| executed_functions.contains(dep_id)) {
                    selected_index = Some(i);
                    break;
                }
            }
            
            // 依存関係のない候補がなければ、最初の候補を選択（循環依存を回避）
            let selected_index = selected_index.unwrap_or(0);
            let candidate = remaining_candidates.remove(selected_index);
            
            // メモリ使用量とコードサイズを考慮して特化を実行するかどうか判断
            if self.should_specialize(&candidate) {
                // 特化の実行
                let start = Instant::now();
                if let Ok(mut result) = self.specialize_function(&candidate) {
                    // 特化にかかった時間を記録
                    result.specialization_time_ms = start.elapsed().as_millis() as u64;
                    
                    // 結果を保存
                    self.results.push(result.clone());
                    executed_functions.insert(candidate.function_id);
                    
                    // 統計情報を更新
                    *self.stats.executed_specializations.entry(candidate.kind).or_insert(0) += 1;
                    self.stats.total_code_size_increase += result.actual_size_increase;
                    self.stats.total_eliminated_dynamic_dispatches += result.eliminated_dynamic_dispatches;
                    self.stats.total_optimized_instructions += result.optimized_instruction_count;
                    self.stats.total_time_ms += result.specialization_time_ms;
                    
                    // メモリ使用量の更新
                    self.update_memory_usage()?;
                    
                    specialization_count += 1;
                    
                    // メモリ制限に達したら終了
                    if self.estimated_memory_usage >= self.memory_limit {
                        break;
                    }
                }
            }
        }
        self.in_progress = false;
        
        // 平均改善率を計算
        if !self.results.is_empty() {
            self.stats.average_improvement = self.results.iter()
                .map(|r| r.estimated_improvement)
                .sum::<f64>() / self.results.len() as f64;
        }
        
        // 結果のクローンを返す
        Ok(self.results.clone())
    }
    
    /// 特化の依存グラフを構築
    fn build_dependency_graph(&mut self) -> Result<(), String> {
        let module = self.module.as_ref().ok_or("モジュールが設定されていません")?;
        
        // 依存グラフを初期化
        self.dependency_graph.clear();
        
        // 各関数の呼び出し関係を分析
        for (func_id, function) in &module.functions {
            let mut dependencies = Vec::new();
            
            for block in &function.basic_blocks {
                for &inst_id in &block.instructions {
                    let inst = &function.instructions[inst_id];
                    
                    // 関数呼び出し命令を探す
                    if inst.opcode == "call" || inst.opcode == "virtual_call" {
                        if let Some(&callee_id) = inst.operands.get(0) {
                            // 呼び出し先が特化候補なら依存関係を追加
                            if self.candidates.iter().any(|c| c.function_id == callee_id) {
                                dependencies.push(callee_id);
                            }
                        }
                    }
                }
            }
            
            // 依存関係を登録
            self.dependency_graph.insert(*func_id, dependencies);
        }
        
        Ok(())
    }
    
    /// 型特化候補を収集
    fn collect_candidates(&mut self) -> Result<(), String> {
        let module = self.module.as_ref().ok_or("モジュールが設定されていません")?;
        let mut candidates = Vec::new();
        
        // ホットパス分析
        let hot_paths = self.analyze_hot_paths()?;
        
        // ジェネリック関数のモノモーフィゼーション候補を収集
        for (func_id, function) in &module.functions {
            // ジェネリック関数かどうかをチェック
            if !function.type_parameters.is_empty() {
                // 呼び出し元を探して具体的な型引数を収集
                let mut concrete_type_args_set = HashSet::new();
                let mut call_counts = HashMap::new();
                
                for (caller_id, caller) in &module.functions {
                    for block in &caller.basic_blocks {
                        for &inst_id in &block.instructions {
                            let inst = &caller.instructions[inst_id];
                            
                            // 関数呼び出し命令を探す
                            if inst.opcode == "call" && inst.operands.get(0) == Some(func_id) {
                                // 型引数を取得
                                if let Some(type_args) = &inst.type_arguments {
                                    concrete_type_args_set.insert(type_args.clone());
                                    *call_counts.entry(type_args.clone()).or_insert(0) += 1;
                                }
                            }
                        }
                    }
                }
                
                // 各具体的な型引数についてモノモーフィゼーション候補を作成
                for type_args in concrete_type_args_set {
                    // 利益スコアを計算（関数の複雑さと呼び出し頻度に基づく）
                    let benefit = self.calculate_monomorphization_benefit(*func_id, &type_args);
                    let call_count = *call_counts.get(&type_args).unwrap_or(&0);
                    let is_on_hot_path = hot_paths.contains(func_id);
                    let estimated_size_increase = self.estimate_specialization_size_increase(*func_id, &type_args);
                    
                    candidates.push(SpecializationCandidate {
                        function_id: *func_id,
                        type_arguments: type_args,
                        kind: SpecializationKind::Monomorphization,
                        benefit_score: benefit,
                        estimated_size_increase,
                        is_on_hot_path,
                        call_count,
                        value_arguments: None,
                    });
                }
            }
        }
        
        // インターフェース消去候補を収集
        self.collect_interface_elimination_candidates(&mut candidates, &hot_paths)?;
        
        // 型クラス特化候補を収集
        self.collect_type_class_specialization_candidates(&mut candidates, &hot_paths)?;
        
        // データレイアウト最適化候補を収集
        self.collect_data_layout_optimization_candidates(&mut candidates, &hot_paths)?;
        
        // 依存型特化候補を収集
        self.collect_dependent_type_specialization_candidates(&mut candidates, &hot_paths)?;
        
        // 型レベル計算実体化候補を収集
        self.collect_type_level_computation_candidates(&mut candidates, &hot_paths)?;
        
        self.candidates = candidates;
        Ok(())
    }
    
    /// ホットパス分析を実行
    fn analyze_hot_paths(&self) -> Result<HashSet<&FunctionId>, String> {
        let module = self.module.as_ref().ok_or("モジュールが設定されていません")?;
        let mut hot_paths = HashSet::new();
    /// モノモーフィゼーションの利益スコアを計算
    fn calculate_monomorphization_benefit(&self, func_id: FunctionId, type_args: &[TypeId]) -> f64 {
        // 基本的な利益スコアを設定
        let mut benefit = 1.0;
        
        if let Some(module) = &self.module {
            if let Some(function) = module.functions.get(&func_id) {
                // 関数の複雑さ（命令数）に基づいて利益を増加
                benefit *= function.instructions.len() as f64 * 0.01;
                
                // 関数の呼び出し頻度に基づいて利益を増加
                let call_count = self.count_function_calls(func_id, type_args);
                benefit *= call_count as f64 * 0.5;
                
                // 仮想メソッド呼び出しの除去に対する追加ボーナス
                if function.is_virtual {
                    benefit *= 2.0;
                }
            }
        }
        
        benefit
    }
    
    /// 関数の呼び出し回数をカウント
    fn count_function_calls(&self, func_id: FunctionId, type_args: &[TypeId]) -> usize {
        let mut count = 0;
        
        if let Some(module) = &self.module {
            for (_, function) in &module.functions {
                for block in &function.basic_blocks {
                    for &inst_id in &block.instructions {
                        let inst = &function.instructions[inst_id];
                        
                        // 関数呼び出し命令を探す
                        if inst.opcode == "call" && inst.operands.get(0) == Some(&func_id) {
                            // 型引数が一致するかチェック
                            if let Some(call_type_args) = &inst.type_arguments {
                                if call_type_args == type_args {
                                    count += 1;
                                }
                            }
                        }
                    }
                }
            }
        }
        
        count
    }
    
    /// 特化を実行すべきかどうか判断
    fn should_specialize(&self, candidate: &SpecializationCandidate) -> bool {
        // 既に特化済みかチェック
        let key = (candidate.function_id, candidate.type_arguments.clone());
        if self.specialized_functions.contains_key(&key) {
            return false;
        }
        
        // 利益スコアが一定以上であることを確認
        if candidate.benefit_score < 0.5 {
            return false;
        }
        
        // メモリ使用量がまだ余裕があることを確認
        if self.estimated_memory_usage >= self.memory_limit {
            return false;
        }
        
        // 特化によるコードサイズ増加を見積もり、制限内かチェック
        let original_code_size = self.estimate_module_size();
        let max_allowed_size = (original_code_size as f64 * self.code_expansion_factor) as usize;
        
        if self.estimated_memory_usage > max_allowed_size {
            return false;
        }
        
        true
    }
    
    /// 関数を特化
    fn specialize_function(&mut self, candidate: &SpecializationCandidate) -> Result<SpecializationResult, String> {
        let module = self.module.as_mut().ok_or("モジュールが設定されていません")?;
        
        // 特化に適した具体的な関数を生成
        let specialized_function_id = match candidate.kind {
            SpecializationKind::Monomorphization => {
                self.perform_monomorphization(candidate.function_id, &candidate.type_arguments)?
            },
            SpecializationKind::InterfaceElimination => {
                self.perform_interface_elimination(candidate.function_id, &candidate.type_arguments)?
            },
            SpecializationKind::TypeClassSpecialization => {
                self.perform_type_class_specialization(candidate.function_id, &candidate.type_arguments)?
            },
            SpecializationKind::DataLayoutOptimization => {
                self.perform_data_layout_optimization(candidate.function_id, &candidate.type_arguments)?
            },
        };
        
        // 特化マップに登録
        let key = (candidate.function_id, candidate.type_arguments.clone());
        self.specialized_functions.insert(key, specialized_function_id);
        
        // 呼び出し命令を特化バージョンに置き換え
        let replaced_calls = self.replace_function_calls(
            candidate.function_id, 
            &candidate.type_arguments, 
            specialized_function_id
        )?;
        
        // 結果を作成
        let result = SpecializationResult {
            original_function_id: candidate.function_id,
            specialized_function_id,
            type_arguments: candidate.type_arguments.clone(),
            replaced_calls,
            kind: candidate.kind,
            estimated_improvement: candidate.benefit_score,
        };
        
        Ok(result)
    }
    
    /// モノモーフィゼーションの実行
    /// ジェネリック関数を具体的な型で特化する
    fn perform_monomorphization(&mut self, func_id: FunctionId, type_args: &[TypeId]) -> Result<FunctionId, String> {
        let module = self.module.as_mut().ok_or("モジュールが設定されていません")?;
        
        // 元の関数を取得
        let original_function = module.functions.get(&func_id)
            .ok_or_else(|| format!("関数ID {}が見つかりません", func_id))?
            .clone();
        
        // 特化関数名を生成（マングリングを行い一意な名前を保証）
        let specialized_name = format!(
            "{}_specialized_{}",
            original_function.name,
            type_args.iter().map(|id| module.get_type_name(*id).unwrap_or(id.to_string()))
                .collect::<Vec<_>>().join("_")
        );
        
        // 新しい関数IDを生成
        let new_func_id = module.next_function_id();
        
        // 特化された関数のクローンを作成
        let mut specialized_function = original_function.clone();
        specialized_function.id = new_func_id;
        specialized_function.name = specialized_name;
        specialized_function.type_parameters.clear(); // 型パラメータを削除
        specialized_function.attributes.insert("specialized".to_string(), "true".to_string());
        specialized_function.attributes.insert("original_function".to_string(), func_id.to_string());
        
        // 型代入マップを構築
        let mut type_subst = HashMap::new();
        for (i, &type_arg) in type_args.iter().enumerate() {
            if i < original_function.type_parameters.len() {
                type_subst.insert(original_function.type_parameters[i], type_arg);
            } else {
                return Err(format!("型引数の数が多すぎます: 期待={}, 実際={}", 
                    original_function.type_parameters.len(), type_args.len()));
            }
        }
        
        // 関数本体の型を具体化
        self.specialize_function_types(&mut specialized_function, &type_subst)?;
        
        // 最適化: 具体的な型に基づいて命令を最適化
        self.optimize_for_concrete_types(&mut specialized_function, &type_subst)?;
        
        // インライン化の候補としてマーク（小さな特化関数は後でインライン化される可能性がある）
        if self.is_inline_candidate(&specialized_function) {
            specialized_function.attributes.insert("inline_candidate".to_string(), "true".to_string());
        }
        
        // デバッグ情報を更新
        if let Some(debug_info) = &mut specialized_function.debug_info {
            debug_info.source_location.push(format!("specialized from {}", original_function.name));
            debug_info.specialization_info = Some(SpecializationDebugInfo {
                original_function_id: func_id,
                type_arguments: type_args.to_vec(),
                specialization_kind: "monomorphization".to_string(),
            });
        }
        
        // モジュールに特化関数を追加
        module.functions.insert(new_func_id, specialized_function);
        
        // メモリ使用量を更新
        self.update_memory_usage()?;
        
        Ok(new_func_id)
    }
    
    /// インターフェース消去の実行
    /// 動的ディスパッチを静的ディスパッチに変換する高度な最適化
    fn perform_interface_elimination(&mut self, func_id: FunctionId, type_args: &[TypeId]) -> Result<FunctionId, String> {
        let module = self.module.as_mut().ok_or("モジュールが設定されていません")?;
        
        // 元の関数を取得
        let original_function = module.functions.get(&func_id)
            .ok_or_else(|| format!("関数ID {}が見つかりません", func_id))?
            .clone();
        
        // 特化関数名を生成
        let specialized_name = format!(
            "{}_interface_eliminated_{}",
            original_function.name,
            type_args.iter().map(|id| module.get_type_name(*id).unwrap_or(id.to_string()))
                .collect::<Vec<_>>().join("_")
        );
        
        // 新しい関数IDを生成
        let new_func_id = module.next_function_id();
        
        // 特化された関数のクローンを作成
        let mut specialized_function = original_function.clone();
        specialized_function.id = new_func_id;
        specialized_function.name = specialized_name;
        specialized_function.attributes.insert("interface_eliminated".to_string(), "true".to_string());
        
        // 型代入マップを構築
        let mut type_subst = HashMap::new();
        for (i, &type_arg) in type_args.iter().enumerate() {
            if i < original_function.type_parameters.len() {
                type_subst.insert(original_function.type_parameters[i], type_arg);
            }
        }
        
        // インターフェースメソッド呼び出しを特定
        let mut interface_calls = Vec::new();
        for (inst_id, inst) in &specialized_function.instructions {
            if inst.opcode == "interface_call" || inst.opcode == "virtual_call" {
                interface_calls.push(*inst_id);
            }
        }
        
        // 各インターフェース呼び出しを静的呼び出しに変換
        for inst_id in interface_calls {
            let inst = specialized_function.instructions.get_mut(&inst_id).unwrap();
            
            // インターフェースメソッドの情報を取得
            let interface_type_id = inst.type_arguments.as_ref()
                .and_then(|args| args.first().copied())
                .ok_or("インターフェース型が見つかりません")?;
                
            let method_name = inst.metadata.get("method_name")
                .ok_or("メソッド名が見つかりません")?
                .clone();
                
            // 具体的な型の実装を検索
            let concrete_type_id = type_subst.get(&interface_type_id)
                .copied()
                .ok_or("具体的な型が見つかりません")?;
                
            let implementation_func_id = self.find_method_implementation(concrete_type_id, &method_name)?;
            
            // 静的呼び出しに変換
            inst.opcode = "call".to_string();
            inst.operands[0] = implementation_func_id;
            inst.type_arguments = None;
            inst.metadata.insert("static_dispatch".to_string(), "true".to_string());
        }
        
        // 関数本体の型を具体化
        self.specialize_function_types(&mut specialized_function, &type_subst)?;
        
        // 最適化: 具体的な型に基づいて命令を最適化
        self.optimize_for_concrete_types(&mut specialized_function, &type_subst)?;
        
        // デバッグ情報を更新
        if let Some(debug_info) = &mut specialized_function.debug_info {
            debug_info.source_location.push(format!("interface eliminated from {}", original_function.name));
            debug_info.specialization_info = Some(SpecializationDebugInfo {
                original_function_id: func_id,
                type_arguments: type_args.to_vec(),
                specialization_kind: "interface_elimination".to_string(),
            });
        }
        
        // モジュールに特化関数を追加
        module.functions.insert(new_func_id, specialized_function);
        
        // メモリ使用量を更新
        self.update_memory_usage()?;
        
        Ok(new_func_id)
    }
    
    /// 型クラス特化の実行
    /// 型クラスメソッドの呼び出しを具体的な型の実装に置き換える
    fn perform_type_class_specialization(&mut self, func_id: FunctionId, type_args: &[TypeId]) -> Result<FunctionId, String> {
        let module = self.module.as_mut().ok_or("モジュールが設定されていません")?;
        
        // 元の関数を取得
        let original_function = module.functions.get(&func_id)
            .ok_or_else(|| format!("関数ID {}が見つかりません", func_id))?
            .clone();
        
        // 特化関数名を生成
        let specialized_name = format!(
            "{}_typeclass_specialized_{}",
            original_function.name,
            type_args.iter().map(|id| module.get_type_name(*id).unwrap_or(id.to_string()))
                .collect::<Vec<_>>().join("_")
        );
        
        // 新しい関数IDを生成
        let new_func_id = module.next_function_id();
        
        // 特化された関数のクローンを作成
        let mut specialized_function = original_function.clone();
        specialized_function.id = new_func_id;
        specialized_function.name = specialized_name;
        specialized_function.attributes.insert("typeclass_specialized".to_string(), "true".to_string());
        
        // 型代入マップを構築
        let mut type_subst = HashMap::new();
        for (i, &type_arg) in type_args.iter().enumerate() {
            if i < original_function.type_parameters.len() {
                type_subst.insert(original_function.type_parameters[i], type_arg);
            }
        }
        
        // 型クラスメソッド呼び出しを特定
        let mut typeclass_calls = Vec::new();
        for (inst_id, inst) in &specialized_function.instructions {
            if inst.opcode == "typeclass_call" {
                typeclass_calls.push(*inst_id);
            }
        }
        
        // 各型クラスメソッド呼び出しを具体的な実装に置き換え
        for inst_id in typeclass_calls {
            let inst = specialized_function.instructions.get_mut(&inst_id).unwrap();
            
            // 型クラスと型引数の情報を取得
            let typeclass_id = inst.metadata.get("typeclass_id")
                .and_then(|id_str| id_str.parse::<TypeClassId>().ok())
                .ok_or("型クラスIDが見つかりません")?;
                
            let method_name = inst.metadata.get("method_name")
                .ok_or("メソッド名が見つかりません")?
                .clone();
                
            let type_param_id = inst.type_arguments.as_ref()
                .and_then(|args| args.first().copied())
                .ok_or("型引数が見つかりません")?;
                
            // 具体的な型の実装を検索
            let concrete_type_id = type_subst.get(&type_param_id)
                .copied()
                .ok_or("具体的な型が見つかりません")?;
                
            let implementation_func_id = self.find_typeclass_implementation(
                typeclass_id, concrete_type_id, &method_name)?;
            
            // 静的呼び出しに変換
            inst.opcode = "call".to_string();
            inst.operands[0] = implementation_func_id;
            inst.type_arguments = None;
            inst.metadata.insert("static_dispatch".to_string(), "true".to_string());
        }
        
        // 関数本体の型を具体化
        self.specialize_function_types(&mut specialized_function, &type_subst)?;
        
        // 最適化: 具体的な型に基づいて命令を最適化
        self.optimize_for_concrete_types(&mut specialized_function, &type_subst)?;
        
        // デバッグ情報を更新
        if let Some(debug_info) = &mut specialized_function.debug_info {
            debug_info.source_location.push(format!("typeclass specialized from {}", original_function.name));
            debug_info.specialization_info = Some(SpecializationDebugInfo {
                original_function_id: func_id,
                type_arguments: type_args.to_vec(),
                specialization_kind: "typeclass_specialization".to_string(),
            });
        }
        
        // モジュールに特化関数を追加
        module.functions.insert(new_func_id, specialized_function);
        
        // メモリ使用量を更新
        self.update_memory_usage()?;
        
        Ok(new_func_id)
    }
    
    /// データレイアウト最適化の実行
    /// 型に基づいてメモリレイアウトを最適化
    fn perform_data_layout_optimization(&mut self, func_id: FunctionId, type_args: &[TypeId]) -> Result<FunctionId, String> {
        let module = self.module.as_mut().ok_or("モジュールが設定されていません")?;
        
        // 元の関数を取得
        let original_function = module.functions.get(&func_id)
            .ok_or_else(|| format!("関数ID {}が見つかりません", func_id))?
            .clone();
        
        // 特化関数名を生成
        let specialized_name = format!(
            "{}_layout_optimized_{}",
            original_function.name,
            type_args.iter().map(|id| module.get_type_name(*id).unwrap_or(id.to_string()))
                .collect::<Vec<_>>().join("_")
        );
        
        // 新しい関数IDを生成
        let new_func_id = module.next_function_id();
        
        // 特化された関数のクローンを作成
        let mut specialized_function = original_function.clone();
        specialized_function.id = new_func_id;
        specialized_function.name = specialized_name;
        specialized_function.attributes.insert("layout_optimized".to_string(), "true".to_string());
        
        // 型代入マップを構築
        let mut type_subst = HashMap::new();
        for (i, &type_arg) in type_args.iter().enumerate() {
            if i < original_function.type_parameters.len() {
                type_subst.insert(original_function.type_parameters[i], type_arg);
            }
        }
        
        // 関数本体の型を具体化
        self.specialize_function_types(&mut specialized_function, &type_subst)?;
        
        // データレイアウト最適化を適用
        self.optimize_data_layout(&mut specialized_function, &type_subst)?;
        
        // 最適化: 具体的な型に基づいて命令を最適化
        self.optimize_for_concrete_types(&mut specialized_function, &type_subst)?;
        
        // デバッグ情報を更新
        if let Some(debug_info) = &mut specialized_function.debug_info {
            debug_info.source_location.push(format!("layout optimized from {}", original_function.name));
            debug_info.specialization_info = Some(SpecializationDebugInfo {
                original_function_id: func_id,
                type_arguments: type_args.to_vec(),
                specialization_kind: "data_layout_optimization".to_string(),
            });
        }
        
        // モジュールに特化関数を追加
        module.functions.insert(new_func_id, specialized_function);
        
        // メモリ使用量を更新
        self.update_memory_usage()?;
        
        Ok(new_func_id)
    }
    
    /// 関数の型を特化
    fn specialize_function_types(&self, function: &mut Function, type_subst: &HashMap<TypeId, TypeId>) -> Result<(), String> {
        let module = self.module.as_ref().ok_or("モジュールが設定されていません")?;
        
        // 関数パラメータの型を具体化
        for param in &mut function.parameters {
            param.type_id = self.substitute_type(param.type_id, type_subst, module)?;
        }
        
        // 関数の戻り値型を具体化
        function.return_type = self.substitute_type(function.return_type, type_subst, module)?;
        
        // 命令の型を具体化
        for inst in function.instructions.values_mut() {
            // 結果型を具体化
            if let Some(result_type) = inst.result_type {
                inst.result_type = Some(self.substitute_type(result_type, type_subst, module)?);
            }
            
            // 型引数を具体化
            if let Some(type_args) = &mut inst.type_arguments {
                for type_arg in type_args {
                    *type_arg = self.substitute_type(*type_arg, type_subst, module)?;
                }
            }
            
            // オペランドの型を具体化（必要に応じて）
            match inst.opcode.as_str() {
                "alloca" | "load" | "store" | "gep" => {
                    if let Some(element_type) = inst.metadata.get_mut("element_type") {
                        if let Ok(type_id) = element_type.parse::<TypeId>() {
                            let new_type_id = self.substitute_type(type_id, type_subst, module)?;
                            *element_type = new_type_id.to_string();
                        }
                    }
                },
                "cast" => {
                    if let Some(source_type) = inst.metadata.get_mut("source_type") {
                        if let Ok(type_id) = source_type.parse::<TypeId>() {
                            let new_type_id = self.substitute_type(type_id, type_subst, module)?;
                            *source_type = new_type_id.to_string();
                        }
                    }
                    if let Some(target_type) = inst.metadata.get_mut("target_type") {
                        if let Ok(type_id) = target_type.parse::<TypeId>() {
                            let new_type_id = self.substitute_type(type_id, type_subst, module)?;
                            *target_type = new_type_id.to_string();
                        }
                    }
                },
                _ => {}
            }
        }
        
        // 変数の型を具体化
        for value in function.values.values_mut() {
            match value {
                Value::Variable { ref mut ty, .. } => {
                    *ty = self.substitute_type(*ty, type_subst, module)?;
                },
                Value::Constant { ref mut ty, .. } => {
                    *ty = self.substitute_type(*ty, type_subst, module)?;
                },
                _ => {}
            }
        }
        
        // ローカル型変数の具体化
        if let Some(local_types) = &mut function.local_types {
            let mut new_local_types = HashMap::new();
            for (name, type_id) in local_types.drain() {
                let new_type_id = self.substitute_type(type_id, type_subst, module)?;
                new_local_types.insert(name, new_type_id);
            }
            *local_types = new_local_types;
        }
        
        Ok(())
    }
    
    /// 型を代入マップに基づいて置換
    fn substitute_type(&self, type_id: TypeId, type_subst: &HashMap<TypeId, TypeId>, module: &Module) -> Result<TypeId, String> {
        // 直接の置換があればそれを使用
        if let Some(&concrete_type) = type_subst.get(&type_id) {
            return Ok(concrete_type);
        }
        
        // 複合型の場合は再帰的に置換
        if let Some(ty) = module.types.get(&type_id) {
            match ty {
                Type::Array { element_type, .. } => {
                    let new_element_type = self.substitute_type(*element_type, type_subst, module)?;
                    if new_element_type != *element_type {
                        // 新しい配列型を作成
                        let array_type = Type::Array {
                            element_type: new_element_type,
                            size: ty.get_array_size().unwrap_or(0),
                        };
                        return Ok(module.register_type(array_type));
                    }
                },
                Type::Tuple { element_types } => {
                    let mut new_element_types = Vec::new();
                    let mut changed = false;
                    
                    for &elem_type in element_types {
                        let new_elem_type = self.substitute_type(elem_type, type_subst, module)?;
                        new_element_types.push(new_elem_type);
                        if new_elem_type != elem_type {
                            changed = true;
                        }
                    }
                    
                    if changed {
                        // 新しいタプル型を作成
                        let tuple_type = Type::Tuple {
                            element_types: new_element_types,
                        };
                        return Ok(module.register_type(tuple_type));
                    }
                },
                Type::Function { parameter_types, return_type } => {
                    let mut new_parameter_types = Vec::new();
                    let mut changed = false;
                    
                    for &param_type in parameter_types {
                        let new_param_type = self.substitute_type(param_type, type_subst, module)?;
                        new_parameter_types.push(new_param_type);
                        if new_param_type != param_type {
                            changed = true;
                        }
                    }
                    
                    let new_return_type = self.substitute_type(*return_type, type_subst, module)?;
                    if new_return_type != *return_type {
                        changed = true;
                    }
                    
                    if changed {
                        // 新しい関数型を作成
                        let function_type = Type::Function {
                            parameter_types: new_parameter_types,
                            return_type: Box::new(new_return_type),
                        };
                        return Ok(module.register_type(function_type));
                    }
                },
                Type::Struct { fields, .. } => {
                    let mut new_fields = Vec::new();
                    let mut changed = false;
                    
                    for field in fields {
                        let new_field_type = self.substitute_type(field.type_id, type_subst, module)?;
                        new_fields.push(StructField {
                            name: field.name.clone(),
                            type_id: new_field_type,
                            offset: field.offset,
                        });
                        
                        if new_field_type != field.type_id {
                            changed = true;
                        }
                    }
                    
                    if changed {
                        // 新しい構造体型を作成
                        let struct_type = Type::Struct {
                            name: ty.get_name().unwrap_or_else(|| "anonymous_struct".to_string()),
                            fields: new_fields,
                            size: ty.get_size().unwrap_or(0),
                            alignment: ty.get_alignment().unwrap_or(8),
                        };
                        return Ok(module.register_type(struct_type));
                    }
                },
                Type::Enum { variants, .. } => {
                    let mut new_variants = Vec::new();
                    let mut changed = false;
                    
                    for variant in variants {
                        let mut new_fields = Vec::new();
                        let mut variant_changed = false;
                        
                        for field in &variant.fields {
                            let new_field_type = self.substitute_type(field.type_id, type_subst, module)?;
                            new_fields.push(StructField {
                                name: field.name.clone(),
                                type_id: new_field_type,
                                offset: field.offset,
                            });
                            
                            if new_field_type != field.type_id {
                                variant_changed = true;
                            }
                        }
                        
                        if variant_changed {
                            changed = true;
                            new_variants.push(EnumVariant {
                                name: variant.name.clone(),
                                tag: variant.tag,
                                fields: new_fields,
                            });
                        } else {
                            new_variants.push(variant.clone());
                        }
                    }
                    
                    if changed {
                        // 新しい列挙型を作成
                        let enum_type = Type::Enum {
                            name: ty.get_name().unwrap_or_else(|| "anonymous_enum".to_string()),
                            variants: new_variants,
                            size: ty.get_size().unwrap_or(0),
                            alignment: ty.get_alignment().unwrap_or(8),
                        };
                        return Ok(module.register_type(enum_type));
                    }
                },
                Type::Generic { .. } => {
                    // ジェネリック型は特化プロセスで解決されるべき
                    return Err(format!("未解決のジェネリック型: {:?}", ty));
                },
                _ => {}
            }
        }
        
        // 変更がなければ元の型をそのまま返す
        Ok(type_id)
    }
    
    /// 具体的な型に基づく命令の最適化
    fn optimize_for_concrete_types(&self, function: &mut Function, type_subst: &HashMap<TypeId, TypeId>) -> Result<(), String> {
        let module = self.module.as_ref().ok_or("モジュールが設定されていません")?;
        
        // 型に基づいた特殊な最適化
        for inst in function.instructions.values_mut() {
            match inst.opcode.as_str() {
                // 演算命令の最適化
                "add" | "sub" | "mul" | "div" | "rem" => {
                    if let Some(result_type) = inst.result_type {
                        // 整数型の場合は高速な整数演算を使用
                        if self.is_integer_type(result_type) {
                            inst.flags.insert("fast_math".to_string());
                            
                            // 定数畳み込み最適化
                            if let (Some(op1), Some(op2)) = (
                                self.get_constant_value(inst.operands[0], function),
                                self.get_constant_value(inst.operands[1], function)
                            ) {
                                if let (Value::Integer { value: v1, .. }, Value::Integer { value: v2, .. }) = (op1, op2) {
                                    let result = match inst.opcode.as_str() {
                                        "add" => v1 + v2,
                                        "sub" => v1 - v2,
                                        "mul" => v1 * v2,
                                        "div" if v2 != 0 => v1 / v2,
                                        "rem" if v2 != 0 => v1 % v2,
                                        _ => continue,
                                    };
                                    
                                    inst.flags.insert("constant_folded".to_string());
                                    inst.metadata.insert("original_opcode".to_string(), inst.opcode.clone());
                                    inst.opcode = "constant".to_string();
                                    inst.operands.clear();
                                    inst.metadata.insert("constant_value".to_string(), result.to_string());
                                }
                            }
                            
                            // 特殊ケースの最適化
                            if inst.opcode == "mul" {
                                if let Some(Value::Integer { value: 2, .. }) = self.get_constant_value(inst.operands[1], function) {
                                    // 2倍は左シフト1ビットに変換
                                    inst.opcode = "shl".to_string();
                                    inst.operands[1] = function.add_constant(Value::Integer { 
                                        value: 1, 
                                        ty: result_type,
                                        is_signed: true 
                                    });
                                    inst.flags.insert("optimized_mul_to_shift".to_string());
                                }
                            }
                        }
                        // 浮動小数点型の場合はSIMD最適化を有効化
                        else if self.is_float_type(result_type) {
                            inst.flags.insert("simd".to_string());
                            inst.flags.insert("fast_math".to_string());
                            inst.flags.insert("reassociate".to_string());
                            inst.flags.insert("no_signed_zeros".to_string());
                            inst.flags.insert("allow_reciprocal".to_string());
                            
                            // 定数畳み込み最適化
                            if let (Some(op1), Some(op2)) = (
                                self.get_constant_value(inst.operands[0], function),
                                self.get_constant_value(inst.operands[1], function)
                            ) {
                        }
                    }
                },
                
                // メモリ操作の最適化
                "load" | "store" => {
                    if let Some(result_type) = inst.result_type {
                        // 特定のサイズの型に対してアライメント最適化
                        let size = self.get_type_size(result_type);
                        if size.is_some() && size.unwrap() % 16 == 0 {
                            inst.flags.insert("aligned_16".to_string());
                        }
                    }
                },
                
                // 動的ディスパッチの静的ディスパッチへの変換
                "virtual_call" => {
                    // 具体的な型が分かっている場合は静的な関数呼び出しに置き換え
                    if let Some(type_args) = &inst.type_arguments {
                        if !type_args.is_empty() && type_subst.contains_key(&type_args[0]) {
                            inst.opcode = "call".to_string();
                            // 実装関数のIDを具体的な型から決定（実際にはより複雑な処理が必要）
                            // ここでは簡略化のため、直接operandsを置き換える処理は省略
                        }
                    }
                },
                
                _ => {}
            }
        }
        
        Ok(())
    }
    
    /// 関数呼び出しを特化バージョンに置き換え
    fn replace_function_calls(&mut self, original_func_id: FunctionId, type_args: &[TypeId], specialized_func_id: FunctionId) -> Result<Vec<InstructionId>, String> {
        let module = self.module.as_mut().ok_or("モジュールが設定されていません")?;
        let mut replaced_calls = Vec::new();
        
        // すべての関数を走査
        for function in module.functions.values_mut() {
            for block in &function.basic_blocks {
                for &inst_id in &block.instructions {
                    let inst = &mut function.instructions[inst_id];
                    
                    // 元の関数への呼び出しを探す
                    if inst.opcode == "call" && inst.operands.get(0) == Some(&original_func_id) {
                        // 型引数が一致するかチェック
                        if let Some(call_type_args) = &inst.type_arguments {
                            if call_type_args == type_args {
                                // 特化関数に置き換え
                                inst.operands[0] = specialized_func_id;
                                
                                // 型引数の削除（特化関数は型パラメータを持たない）
                                inst.type_arguments = None;
                                
                                replaced_calls.push(inst_id);
                            }
                        }
                    }
                }
            }
        }
        
        Ok(replaced_calls)
    }
    
    /// モジュールのサイズを推定（バイト単位）
    fn estimate_module_size(&self) -> usize {
        let mut size = 0;
        
        if let Some(module) = &self.module {
            // 関数ごとのサイズを計算
            for (_, function) in &module.functions {
                // 関数の基本情報（名前など）
                size += function.name.len();
                size += function.parameters.len() * 8;
                
                // 命令のサイズ
                size += function.instructions.len() * 16;
                
                // 基本ブロックの情報
                size += function.basic_blocks.len() * 8;
                
                // 変数の情報
                size += function.values.len() * 16;
            }
            
            // 型情報のサイズ
            size += module.types.len() * 12;
            
            // 定数のサイズ
            for (_, value) in &module.constants {
                match value {
                    Value::Integer { .. } => size += 8,
                    Value::Float { .. } => size += 8,
                    Value::String { value, .. } => size += value.len(),
                    _ => size += 8,
                }
            }
        }
        
        size
    }
    
    /// メモリ使用量を更新
    fn update_memory_usage(&mut self) -> Result<(), String> {
        self.estimated_memory_usage = self.estimate_module_size();
        Ok(())
    }
    
    /// 型が整数型かどうか判定
    fn is_integer_type(&self, type_id: TypeId) -> bool {
        if let Some(module) = &self.module {
            if let Some(ty) = module.types.get(&type_id) {
                match ty {
                    Type::Integer { .. } => return true,
                    _ => {}
                }
            }
        }
        false
    }
    
    /// 型が浮動小数点型かどうか判定
    fn is_float_type(&self, type_id: TypeId) -> bool {
        if let Some(module) = &self.module {
            if let Some(ty) = module.types.get(&type_id) {
                match ty {
                    Type::Float { .. } => return true,
                    _ => {}
                }
            }
        }
        false
    }
    
    /// 型のサイズを取得（バイト単位）
    fn get_type_size(&self, type_id: TypeId) -> Option<usize> {
        if let Some(module) = &self.module {
            if let Some(ty) = module.types.get(&type_id) {
                return match ty {
                    Type::Integer { width, .. } => Some((*width as usize + 7) / 8),
                    Type::Float { width, .. } => Some(*width as usize / 8),
                    Type::Struct { fields, .. } => {
                        // 構造体の場合はフィールドのサイズの合計
                        let mut total = 0;
                        for &field_type in fields {
                            if let Some(field_size) = self.get_type_size(field_type) {
                                total += field_size;
                            } else {
                                return None;
                            }
                        }
                        Some(total)
                    },
                    Type::Array { element_type, size, .. } => {
                        // 配列の場合は要素のサイズ × 要素数
                        if let Some(elem_size) = self.get_type_size(*element_type) {
                            Some(elem_size * *size)
                        } else {
                            None
                        }
                    },
                    Type::Pointer { .. } => Some(8), // 64ビットポインタを仮定
                    Type::Reference { .. } => Some(8), // 参照も64ビットポインタと同じ
                    _ => None,
                };
            }
        }
        None
    }
}

/// 型特化最適化パスの実装
pub struct TypeSpecializationPass;

impl TypeSpecializationPass {
    /// 新しい型特化最適化パスを作成
    pub fn new() -> Self {
        Self {}
    }
    
    /// 最適化パスを実行
    pub fn run(&self, module: &mut Module) -> Result<Vec<SpecializationResult>, String> {
        // 特化マネージャーを初期化
        let mut manager = TypeSpecializationManager::new();
        
        // モジュールを設定
        manager.set_module(module.clone());
        
        // メモリ制限をモジュールサイズの2倍に設定
        let module_size = manager.estimate_module_size();
        manager.set_memory_limit(module_size * 2);
        
        // コード膨張係数を設定（2.0 = 元のサイズの最大2倍まで許容）
        manager.set_code_expansion_factor(2.0);
        
        // 型特化最適化を実行
        let results = manager.run_specialization()?;
        
        // 特化されたモジュールを元のモジュールに反映
        *module = manager.module.unwrap();
        
        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::middleend::ir::representation::{Module, Function, BasicBlock, Instruction, Type, Value};
    use std::collections::HashMap;
    
    /// テスト用のモジュールを構築するヘルパー関数
    fn create_test_module() -> Module {
        let mut module = Module::new("test_module");
        
        // ジェネリック関数を追加
        let generic_func = Function::new(
            "generic_function",
            vec![Type::Generic { name: "T".to_string() }],
            Type::Generic { name: "T".to_string() },
            true
        );
        
        let func_id = module.add_function(generic_func);
        
        // 基本ブロックを追加
        let mut entry_block = BasicBlock::new("entry");
        
        // パラメータを取得する命令
        let param = Instruction::GetParam { index: 0 };
        let param_id = entry_block.add_instruction(param);
        
        // 単純な計算を行う命令（例：パラメータを返す）
        let ret = Instruction::Return { value: Value::InstructionResult(param_id) };
        entry_block.add_instruction(ret);
        
        // 基本ブロックを関数に追加
        module.add_basic_block(func_id, entry_block);
        
        module
    }
    
    #[test]
    fn test_type_specialization_manager_initialization() {
        let manager = TypeSpecializationManager::new();
        assert_eq!(manager.memory_limit, usize::MAX);
        assert_eq!(manager.code_expansion_factor, 1.5);
        assert!(manager.module.is_none());
        assert!(manager.specialized_functions.is_empty());
    }
    
    #[test]
    fn test_module_size_estimation() {
        let mut manager = TypeSpecializationManager::new();
        let module = create_test_module();
        
        manager.set_module(module);
        
        let size = manager.estimate_module_size();
        assert!(size > 0, "モジュールサイズの推定値は0より大きくなければなりません");
    }
    
    #[test]
    fn test_memory_limit_setting() {
        let mut manager = TypeSpecializationManager::new();
        
        // メモリ制限を設定
        manager.set_memory_limit(1024);
        assert_eq!(manager.memory_limit, 1024);
        
        // 0のメモリ制限は許可されない
        manager.set_memory_limit(0);
        assert_eq!(manager.memory_limit, 1024, "メモリ制限は0に設定されるべきではありません");
    }
    
    #[test]
    fn test_code_expansion_factor_setting() {
        let mut manager = TypeSpecializationManager::new();
        
        // 有効な拡張係数を設定
        manager.set_code_expansion_factor(2.5);
        assert_eq!(manager.code_expansion_factor, 2.5);
        
        // 1.0未満の拡張係数は許可されない
        manager.set_code_expansion_factor(0.5);
        assert_eq!(manager.code_expansion_factor, 2.5, "拡張係数は1.0未満に設定されるべきではありません");
    }
    
    #[test]
    fn test_get_type_size() {
        let mut manager = TypeSpecializationManager::new();
        let module = create_test_module();
        
        manager.set_module(module);
        
        // 基本型のサイズをテスト
        assert_eq!(manager.get_type_size(Type::Int { bits: 32 }), Some(4));
        assert_eq!(manager.get_type_size(Type::Int { bits: 64 }), Some(8));
        assert_eq!(manager.get_type_size(Type::Float { bits: 32 }), Some(4));
        assert_eq!(manager.get_type_size(Type::Float { bits: 64 }), Some(8));
        assert_eq!(manager.get_type_size(Type::Bool), Some(1));
        assert_eq!(manager.get_type_size(Type::Char), Some(4)); // Unicode文字
        
        // 配列型のサイズをテスト
        assert_eq!(
            manager.get_type_size(Type::Array { 
                element_type: Box::new(Type::Int { bits: 32 }), 
                size: 10 
            }),
            Some(40)
        );
        
        // ポインタと参照型のサイズをテスト
        assert_eq!(
            manager.get_type_size(Type::Pointer { 
                pointee_type: Box::new(Type::Int { bits: 32 }) 
            }),
            Some(8)
        );
        assert_eq!(
            manager.get_type_size(Type::Reference { 
                referenced_type: Box::new(Type::Int { bits: 32 }) 
            }),
            Some(8)
        );
        
        // ジェネリック型のサイズはNoneを返す
        assert_eq!(
            manager.get_type_size(Type::Generic { name: "T".to_string() }),
            None
        );
    }
    
    #[test]
    fn test_type_specialization() {
        let mut module = create_test_module();
        let pass = TypeSpecializationPass::new();
        
        // 特化対象の型を含む呼び出しを追加
        let mut caller_func = Function::new(
            "caller",
            vec![],
            Type::Void,
            false
        );
        
        let mut caller_entry = BasicBlock::new("caller_entry");
        
        // Int型の引数でジェネリック関数を呼び出す
        let int_const = Instruction::Constant { 
            value: Value::Int(42), 
            ty: Type::Int { bits: 32 } 
        };
        let int_const_id = caller_entry.add_instruction(int_const);
        
        let call_with_int = Instruction::Call { 
            function: "generic_function".to_string(), 
            arguments: vec![Value::InstructionResult(int_const_id)],
            type_arguments: vec![Type::Int { bits: 32 }]
        };
        caller_entry.add_instruction(call_with_int);
        
        // Float型の引数でジェネリック関数を呼び出す
        let float_const = Instruction::Constant { 
            value: Value::Float(3.14), 
            ty: Type::Float { bits: 32 } 
        };
        let float_const_id = caller_entry.add_instruction(float_const);
        
        let call_with_float = Instruction::Call { 
            function: "generic_function".to_string(), 
            arguments: vec![Value::InstructionResult(float_const_id)],
            type_arguments: vec![Type::Float { bits: 32 }]
        };
        caller_entry.add_instruction(call_with_float);
        
        // 関数を追加
        let caller_id = module.add_function(caller_func);
        module.add_basic_block(caller_id, caller_entry);
        
        // 型特化最適化を実行
        let results = pass.run(&mut module).expect("型特化最適化が失敗しました");
        
        // 結果を検証
        assert_eq!(results.len(), 2, "2つの特化された関数が生成されるべきです");
        
        // 特化された関数の名前を確認
        let specialized_names: Vec<String> = results.iter()
            .map(|r| r.specialized_function_name.clone())
            .collect();
        
        assert!(specialized_names.contains(&"generic_function.Int32".to_string()));
        assert!(specialized_names.contains(&"generic_function.Float32".to_string()));
        
        // モジュール内の関数数を確認
        assert_eq!(module.functions.len(), 3, "元の関数と2つの特化された関数があるべきです");
    }
    
    #[test]
    fn test_specialization_with_complex_types() {
        let mut module = create_test_module();
        
        // 複合型を使用するジェネリック関数を追加
        let complex_generic_func = Function::new(
            "complex_generic",
            vec![
                Type::Array { 
                    element_type: Box::new(Type::Generic { name: "T".to_string() }), 
                    size: 5 
                }
            ],
            Type::Generic { name: "T".to_string() },
            true
        );
        
        let func_id = module.add_function(complex_generic_func);
        
        // 基本ブロックを追加
        let mut entry_block = BasicBlock::new("entry");
        
        // 配列の最初の要素を取得
        let param = Instruction::GetParam { index: 0 };
        let param_id = entry_block.add_instruction(param);
        
        let get_element = Instruction::GetArrayElement { 
            array: Value::InstructionResult(param_id), 
            index: Value::Int(0) 
        };
        let element_id = entry_block.add_instruction(get_element);
        
        // 結果を返す
        let ret = Instruction::Return { value: Value::InstructionResult(element_id) };
        entry_block.add_instruction(ret);
        
        // 基本ブロックを関数に追加
        module.add_basic_block(func_id, entry_block);
        
        // 呼び出し関数を追加
        let mut caller_func = Function::new(
            "complex_caller",
            vec![],
            Type::Void,
            false
        );
        
        let mut caller_entry = BasicBlock::new("complex_caller_entry");
        
        // Int型の配列を作成
        let create_int_array = Instruction::CreateArray { 
            element_type: Type::Int { bits: 32 }, 
            size: 5,
            initial_values: vec![
                Value::Int(1), Value::Int(2), Value::Int(3), 
                Value::Int(4), Value::Int(5)
            ]
        };
        let int_array_id = caller_entry.add_instruction(create_int_array);
        
        // 配列を引数にジェネリック関数を呼び出す
        let call_with_int_array = Instruction::Call { 
            function: "complex_generic".to_string(), 
            arguments: vec![Value::InstructionResult(int_array_id)],
            type_arguments: vec![Type::Int { bits: 32 }]
        };
        caller_entry.add_instruction(call_with_int_array);
        
        // 関数を追加
        let caller_id = module.add_function(caller_func);
        module.add_basic_block(caller_id, caller_entry);
        
        // 型特化最適化を実行
        let pass = TypeSpecializationPass::new();
        let results = pass.run(&mut module).expect("型特化最適化が失敗しました");
        
        // 結果を検証
        assert_eq!(results.len(), 1, "1つの特化された関数が生成されるべきです");
        
        // 特化された関数の名前を確認
        assert_eq!(
            results[0].specialized_function_name,
            "complex_generic.Array5.Int32"
        );
        
        // 特化された関数のパラメータ型を確認
        assert!(module.functions.iter().any(|f| 
            f.name == "complex_generic.Array5.Int32" && 
            f.parameters.len() == 1 &&
            matches!(f.parameters[0], Type::Array { .. })
        ));
    }
    
    #[test]
    fn test_specialization_with_memory_constraints() {
        let mut module = create_test_module();
        let mut pass = TypeSpecializationPass::new();
        
        // 多数の型引数を持つ呼び出しを追加
        let mut caller_func = Function::new(
            "memory_constrained_caller",
            vec![],
            Type::Void,
            false
        );
        
        let mut caller_entry = BasicBlock::new("memory_constrained_entry");
        
        // 20種類の異なる型でジェネリック関数を呼び出す
        for i in 1..21 {
            let int_const = Instruction::Constant { 
                value: Value::Int(i), 
                ty: Type::Int { bits: 32 * (i % 2 + 1) } // 32ビットと64ビットを交互に
            };
            let const_id = caller_entry.add_instruction(int_const);
            
            let call = Instruction::Call { 
                function: "generic_function".to_string(), 
                arguments: vec![Value::InstructionResult(const_id)],
                type_arguments: vec![Type::Int { bits: 32 * (i % 2 + 1) }]
            };
            caller_entry.add_instruction(call);
        }
        
        // 関数を追加
        let caller_id = module.add_function(caller_func);
        module.add_basic_block(caller_id, caller_entry);
        
        // 非常に厳しいメモリ制限を設定（モジュールサイズの1.1倍）
        let manager = TypeSpecializationManager::new();
        let module_size = manager.estimate_module_size();
        
        // TypeSpecializationPassを修正してメモリ制限を設定
        let mut manager = TypeSpecializationManager::new();
        manager.set_module(module.clone());
        manager.set_memory_limit(module_size * 1.1 as usize);
        manager.set_code_expansion_factor(1.1);
        
        // 特化を実行
        let results = manager.run_specialization().expect("型特化最適化が失敗しました");
        
        // 結果を検証 - メモリ制限により全ての特化が行われないはず
        assert!(results.len() < 20, "メモリ制限により一部の特化のみが行われるべきです");
        
        // 優先度の高い特化（より頻繁に使用される型）が優先されることを確認
        // （このテストでは単純化のため、最初に出現する型が優先されると仮定）
        if !results.is_empty() {
            assert_eq!(
                results[0].specialized_function_name,
                "generic_function.Int32",
                "最も頻繁に使用される型が最初に特化されるべきです"
            );
        }
    }
    
    #[test]
    fn test_specialization_result_tracking() {
        let mut module = create_test_module();
        let pass = TypeSpecializationPass::new();
        
        // 呼び出し関数を追加
        let mut caller_func = Function::new(
            "tracking_caller",
            vec![],
            Type::Void,
            false
        );
        
        let mut caller_entry = BasicBlock::new("tracking_entry");
        
        // Int型の引数でジェネリック関数を呼び出す
        let int_const = Instruction::Constant { 
            value: Value::Int(42), 
            ty: Type::Int { bits: 32 } 
        };
        let int_const_id = caller_entry.add_instruction(int_const);
        
        let call_with_int = Instruction::Call { 
            function: "generic_function".to_string(), 
            arguments: vec![Value::InstructionResult(int_const_id)],
            type_arguments: vec![Type::Int { bits: 32 }]
        };
        let call_id = caller_entry.add_instruction(call_with_int);
        
        // 呼び出し結果を使用
        let add = Instruction::BinaryOp { 
            op: "add".to_string(), 
            left: Value::InstructionResult(call_id), 
            right: Value::Int(10) 
        };
        caller_entry.add_instruction(add);
        
        // 関数を追加
        let caller_id = module.add_function(caller_func);
        module.add_basic_block(caller_id, caller_entry);
        
        // 型特化最適化を実行
        let results = pass.run(&mut module).expect("型特化最適化が失敗しました");
        
        // 結果を検証
        assert_eq!(results.len(), 1, "1つの特化された関数が生成されるべきです");
        
        // 特化結果の詳細を確認
        let result = &results[0];
        assert_eq!(result.specialized_function_name, "generic_function.Int32");
        assert_eq!(result.original_function_name, "generic_function");
        assert_eq!(result.type_arguments, vec![Type::Int { bits: 32 }]);
        assert!(result.call_sites.len() > 0, "呼び出しサイトが追跡されるべきです");
        
        // 呼び出しサイトが正しく更新されていることを確認
        let updated_caller = module.functions.iter()
            .find(|f| f.name == "tracking_caller")
            .expect("呼び出し元関数が見つかりません");
        
        let updated_block = module.get_basic_blocks_for_function(
            module.functions.iter().position(|f| f.name == "tracking_caller").unwrap()
        ).expect("呼び出し元の基本ブロックが見つかりません");
        
        let updated_call = updated_block[0].instructions.iter()
            .find(|i| matches!(i, Instruction::Call { .. }))
            .expect("呼び出し命令が見つかりません");
        
        if let Instruction::Call { function, .. } = updated_call {
            assert_eq!(function, "generic_function.Int32", "呼び出しが特化された関数に更新されるべきです");
        }
    }
    
    #[test]
    fn test_recursive_specialization() {
        let mut module = Module::new("recursive_test");
        
        // 再帰的なジェネリック関数を追加
        let recursive_func = Function::new(
            "recursive_generic",
            vec![
                Type::Generic { name: "T".to_string() },
                Type::Int { bits: 32 }
            ],
            Type::Generic { name: "T".to_string() },
            true
        );
        
        let func_id = module.add_function(recursive_func);
        
        // 基本ブロックを追加
        let mut entry_block = BasicBlock::new("entry");
        
        // 再帰終了条件をチェック
        let get_count = Instruction::GetParam { index: 1 };
        let count_id = entry_block.add_instruction(get_count);
        
        let zero = Instruction::Constant { 
            value: Value::Int(0), 
            ty: Type::Int { bits: 32 } 
        };
        let zero_id = entry_block.add_instruction(zero);
        
        let cmp = Instruction::Compare { 
            op: "eq".to_string(), 
            left: Value::InstructionResult(count_id), 
            right: Value::InstructionResult(zero_id) 
        };
        let cmp_id = entry_block.add_instruction(cmp);
        
        // 条件分岐
        let branch = Instruction::ConditionalBranch { 
            condition: Value::InstructionResult(cmp_id), 
            true_block: "return_block".to_string(), 
            false_block: "recursive_block".to_string() 
        };
        entry_block.add_instruction(branch);
        
        // 再帰終了時の処理
        let mut return_block = BasicBlock::new("return_block");
        
        let get_value = Instruction::GetParam { index: 0 };
        let value_id = return_block.add_instruction(get_value);
        
        let ret = Instruction::Return { value: Value::InstructionResult(value_id) };
        return_block.add_instruction(ret);
        
        // 再帰呼び出しの処理
        let mut recursive_block = BasicBlock::new("recursive_block");
        
        // カウントをデクリメント
        let get_count_rec = Instruction::GetParam { index: 1 };
        let count_rec_id = recursive_block.add_instruction(get_count_rec);
        
        let one = Instruction::Constant { 
            value: Value::Int(1), 
            ty: Type::Int { bits: 32 } 
        };
        let one_id = recursive_block.add_instruction(one);
        
        let sub = Instruction::BinaryOp { 
            op: "sub".to_string(), 
            left: Value::InstructionResult(count_rec_id), 
            right: Value::InstructionResult(one_id) 
        };
        let new_count_id = recursive_block.add_instruction(sub);
        
        // 値を取得
        let get_value_rec = Instruction::GetParam { index: 0 };
        let value_rec_id = recursive_block.add_instruction(get_value_rec);
        
        // 再帰呼び出し
        let recursive_call = Instruction::Call { 
            function: "recursive_generic".to_string(), 
            arguments: vec![
                Value::InstructionResult(value_rec_id),
                Value::InstructionResult(new_count_id)
            ],
            type_arguments: vec![Type::Generic { name: "T".to_string() }]
        };
        let result_id = recursive_block.add_instruction(recursive_call);
        
        let ret_rec = Instruction::Return { value: Value::InstructionResult(result_id) };
        recursive_block.add_instruction(ret_rec);
        
        // 基本ブロックを関数に追加
        module.add_basic_block(func_id, entry_block);
        module.add_basic_block(func_id, return_block);
        module.add_basic_block(func_id, recursive_block);
        
        // 呼び出し関数を追加
        let mut caller_func = Function::new(
            "recursive_caller",
            vec![],
            Type::Int { bits: 32 },
            false
        );
        
        let mut caller_entry = BasicBlock::new("caller_entry");
        
        // Int型の引数と再帰回数でジェネリック関数を呼び出す
        let int_const = Instruction::Constant { 
            value: Value::Int(42), 
            ty: Type::Int { bits: 32 } 
        };
        let int_const_id = caller_entry.add_instruction(int_const);
        
        let count_const = Instruction::Constant { 
            value: Value::Int(5), 
            ty: Type::Int { bits: 32 } 
        };
        let count_const_id = caller_entry.add_instruction(count_const);
        
        let call = Instruction::Call { 
            function: "recursive_generic".to_string(), 
            arguments: vec![
                Value::InstructionResult(int_const_id),
                Value::InstructionResult(count_const_id)
            ],
            type_arguments: vec![Type::Int { bits: 32 }]
        };
        let call_id = caller_entry.add_instruction(call);
        
        let ret_caller = Instruction::Return { value: Value::InstructionResult(call_id) };
        caller_entry.add_instruction(ret_caller);
        
        // 関数を追加
        let caller_id = module.add_function(caller_func);
        module.add_basic_block(caller_id, caller_entry);
        
        // 型特化最適化を実行
        let pass = TypeSpecializationPass::new();
        let results = pass.run(&mut module).expect("型特化最適化が失敗しました");
        
        // 結果を検証
        assert_eq!(results.len(), 1, "1つの特化された関数が生成されるべきです");
        
        // 特化された関数の名前を確認
        assert_eq!(
            results[0].specialized_function_name,
            "recursive_generic.Int32"
        );
        
        // 再帰呼び出しが特化された関数を呼び出すように更新されていることを確認
        let specialized_func_id = module.functions.iter().position(|f| 
            f.name == "recursive_generic.Int32"
        ).expect("特化された関数が見つかりません");
        
        let specialized_blocks = module.get_basic_blocks_for_function(specialized_func_id)
            .expect("特化された関数の基本ブロックが見つかりません");
        
        let recursive_block = specialized_blocks.iter()
            .find(|b| b.name == "recursive_block")
            .expect("再帰ブロックが見つかりません");
        
        let recursive_call = recursive_block.instructions.iter()
            .find(|i| matches!(i, Instruction::Call { .. }))
            .expect("再帰呼び出しが見つかりません");
        
        if let Instruction::Call { function, .. } = recursive_call {
            assert_eq!(
                function, 
                "recursive_generic.Int32", 
                "再帰呼び出しが特化された関数自身を呼び出すように更新されるべきです"
            );
        }
    }
    
    #[test]
    fn test_performance_improvement() {
        // このテストは実際のパフォーマンス向上を測定するものではなく、
        // 型特化が期待通りに行われることを確認するためのものです
        
        let mut module = create_test_module();
        
        // 複雑な計算を行うジェネリック関数を追加
        let complex_func = Function::new(
            "complex_calculation",
            vec![Type::Generic { name: "T".to_string() }],
            Type::Generic { name: "T".to_string() },
            true
        );
        
        let func_id = module.add_function(complex_func);
        
        // 基本ブロックを追加
        let mut entry_block = BasicBlock::new("entry");
        
        // パラメータを取得
        let param = Instruction::GetParam { index: 0 };
        let param_id = entry_block.add_instruction(param);
        
        // 型チェックと分岐（実際の最適化対象）
        let type_check = Instruction::TypeCheck { 
            value: Value::InstructionResult(param_id), 
            expected_type: Type::Int { bits: 32 } 
        };
        let check_id = entry_block.add_instruction(type_check);
        
        let branch = Instruction::ConditionalBranch { 
            condition: Value::InstructionResult(check_id), 
            true_block: "int_block".to_string(), 
            false_block: "other_block".to_string() 
        };
        entry_block.add_instruction(branch);
        
        // Int型の場合の処理
        let mut int_block = BasicBlock::new("int_block");
        
        // Int型として処理
        let cast_to_int = Instruction::Cast { 
            value: Value::InstructionResult(param_id), 
            target_type: Type::Int { bits: 32 } 
        };
        let int_id = int_block.add_instruction(cast_to_int);
        
        // 何か計算を行う
        let add = Instruction::BinaryOp { 
            op: "add".to_string(), 
            left: Value::InstructionResult(int_id), 
            right: Value::Int(10) 
        };
        let result_id = int_block.add_instruction(add);
        
        let ret_int = Instruction::Return { value: Value::InstructionResult(result_id) };
        int_block.add_instruction(ret_int);
        
        // その他の型の場合の処理
        let mut other_block = BasicBlock::new("other_block");
        
        // そのまま返す
        let ret_other = Instruction::Return { value: Value::InstructionResult(param_id) };
        other_block.add_instruction(ret_other);
        
        // 基本ブロックを関数に追加
        module.add_basic_block(func_id, entry_block);
        module.add_basic_block(func_id, int_block);
        module.add_basic_block(func_id, other_block);
        
        // 呼び出し関数を追加
        let mut caller_func = Function::new(
            "performance_caller",
            vec![],
            Type::Int { bits: 32 },
            false
        );
        
        let mut caller_entry = BasicBlock::new("caller_entry");
        
        // Int型の引数でジェネリック関数を呼び出す
        let int_const = Instruction::Constant { 
            value: Value::Int(42), 
            ty: Type::Int { bits: 32 } 
        };
        let int_const_id = caller_entry.add_instruction(int_const);
        
        let call = Instruction::Call { 
            function: "complex_calculation".to_string(), 
            arguments: vec![Value::InstructionResult(int_const_id)],
            type_arguments: vec![Type::Int { bits: 32 }]
        };
        let call_id = caller_entry.add_instruction(call);
} 