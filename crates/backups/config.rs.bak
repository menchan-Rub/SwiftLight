//! # コンパイラ設定
//! 
//! コンパイラの動作を設定するための構造体とオプションを提供します。
//! SwiftLight言語の高度な機能をサポートするための様々な設定オプションが含まれています。
//! 安全性、効率性、表現力、開発体験の全てにおいて最高水準を目指すSwiftLight言語の
//! コンパイラ設定を管理します。

use std::path::PathBuf;
use std::num::NonZeroUsize;
use std::collections::HashMap;
use crate::backend::{Target, OptimizationLevel, TargetOptions};
use crate::driver::options::CompileOptions;
use crate::frontend::semantic::{OwnershipCheckLevel, TypeCheckLevel};
use crate::utils::diagnostics::DiagnosticLevel;
use crate::frontend::metaprogramming::MetaProgrammingOptions;
use crate::frontend::dependent_types::DependentTypeOptions;
use crate::analysis::formal_verification::FormalVerificationOptions;
use crate::analysis::security::SecurityCheckOptions;
use crate::optimization::pipeline::OptimizationPipeline;

/// コンパイラ設定
#[derive(Debug, Clone)]
pub struct CompilerConfig {
    /// コンパイルターゲット
    pub target: Target,
    /// 最適化レベル
    pub optimization_level: OptimizationLevel,
    /// ターゲット固有のオプション
    pub target_options: TargetOptions,
    /// デバッグ情報を含めるかどうか
    pub debug_info: bool,
    /// デバッグ情報のレベル (0-3)
    pub debug_info_level: u8,
    /// LTO (Link Time Optimization) を有効にするかどうか
    pub lto: bool,
    /// LTOのタイプ (thin, full, etc.)
    pub lto_type: LtoType,
    /// インクルードパス
    pub include_paths: Vec<PathBuf>,
    /// ライブラリパス
    pub library_paths: Vec<PathBuf>,
    /// リンクするライブラリ
    pub libraries: Vec<String>,
    /// 定義済みマクロ
    pub defines: Vec<(String, Option<String>)>,
    /// 警告を有効にするかどうか
    pub warnings_as_errors: bool,
    /// 特定の警告を無視するリスト
    pub ignored_warnings: Vec<String>,
    /// 特定の警告をエラーとして扱うリスト
    pub promoted_warnings: Vec<String>,
    /// 診断レベル
    pub diagnostic_level: DiagnosticLevel,
    /// 並列ビルドを有効にするかどうか
    pub parallel_build: bool,
    /// 並列ビルドで使用するスレッド数（Noneの場合は自動）
    pub parallel_threads: Option<NonZeroUsize>,
    /// キャッシュを有効にするかどうか
    pub use_cache: bool,
    /// キャッシュディレクトリ
    pub cache_dir: Option<PathBuf>,
    /// 中間ファイルを保持するかどうか
    pub keep_intermediates: bool,
    /// 詳細な出力を有効にするかどうか
    pub verbose: bool,
    /// 所有権チェックのレベル
    pub ownership_check_level: OwnershipCheckLevel,
    /// 型チェックのレベル
    pub type_check_level: TypeCheckLevel,
    /// 依存型チェックを有効にするかどうか
    pub enable_dependent_types: bool,
    /// 依存型オプション
    pub dependent_type_options: DependentTypeOptions,
    /// コンパイル時計算の最大ステップ数
    pub compile_time_computation_limit: usize,
    /// コンパイル時計算のメモリ制限（バイト）
    pub compile_time_memory_limit: usize,
    /// メタプログラミングオプション
    pub metaprogramming_options: MetaProgrammingOptions,
    /// WebAssembly出力を有効にするかどうか
    pub wasm_output: bool,
    /// WebAssembly最適化レベル
    pub wasm_opt_level: u8,
    /// WebAssembly特有のオプション
    pub wasm_options: WasmOptions,
    /// 形式検証を有効にするかどうか
    pub formal_verification: bool,
    /// 形式検証オプション
    pub formal_verification_options: FormalVerificationOptions,
    /// ファジングテストを有効にするかどうか
    pub fuzzing: bool,
    /// ファジングオプション
    pub fuzzing_options: FuzzingOptions,
    /// SIMDの自動ベクトル化を有効にするかどうか
    pub auto_vectorization: bool,
    /// ベクトル化オプション
    pub vectorization_options: VectorizationOptions,
    /// インクリメンタルコンパイルを有効にするかどうか
    pub incremental_compilation: bool,
    /// インクリメンタルコンパイルのキャッシュディレクトリ
    pub incremental_cache_dir: Option<PathBuf>,
    /// プロファイルに基づく最適化を有効にするかどうか
    pub profile_guided_optimization: bool,
    /// プロファイルデータのパス
    pub profile_data_path: Option<PathBuf>,
    /// セキュリティチェックを有効にするかどうか
    pub security_checks: bool,
    /// セキュリティチェックオプション
    pub security_check_options: SecurityCheckOptions,
    /// メモリリーク検出を有効にするかどうか
    pub memory_leak_detection: bool,
    /// メモリリーク検出オプション
    pub memory_leak_options: MemoryLeakOptions,
    /// 最適化パイプライン
    pub optimization_pipeline: OptimizationPipeline,
    /// コード生成オプション
    pub codegen_options: CodegenOptions,
    /// 言語拡張機能
    pub language_extensions: HashMap<String, bool>,
    /// 実験的機能
    pub experimental_features: HashMap<String, bool>,
    /// ソースマップを生成するかどうか
    pub generate_source_maps: bool,
    /// ドキュメント生成を有効にするかどうか
    pub generate_docs: bool,
    /// ドキュメント生成オプション
    pub doc_options: DocOptions,
    /// 静的解析オプション
    pub static_analysis_options: StaticAnalysisOptions,
    /// 並行処理モデルオプション
    pub concurrency_options: ConcurrencyOptions,
    /// エラー回復戦略
    pub error_recovery_strategy: ErrorRecoveryStrategy,
    /// プラグインのパス
    pub plugin_paths: Vec<PathBuf>,
    /// プラグインオプション
    pub plugin_options: HashMap<String, String>,
}

/// LTOのタイプ
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LtoType {
    /// LTOなし
    None,
    /// 通常のLTO
    Full,
    /// 薄いLTO
    Thin,
    /// 自動選択
    Auto,
}

/// WebAssembly特有のオプション
#[derive(Debug, Clone)]
pub struct WasmOptions {
    /// WebAssemblyバイナリフォーマットのバージョン
    pub version: WasmVersion,
    /// SIMDを有効にするかどうか
    pub enable_simd: bool,
    /// 例外処理を有効にするかどうか
    pub enable_exceptions: bool,
    /// スレッドを有効にするかどうか
    pub enable_threads: bool,
    /// バルクメモリ操作を有効にするかどうか
    pub enable_bulk_memory: bool,
    /// 参照型を有効にするかどうか
    pub enable_reference_types: bool,
    /// 多値型を有効にするかどうか
    pub enable_multi_value: bool,
    /// テールコールを有効にするかどうか
    pub enable_tail_call: bool,
    /// 64ビットメモリを有効にするかどうか
    pub enable_memory64: bool,
    /// WebAssemblyのエクスポート名
    pub exports: Vec<String>,
    /// WebAssemblyのインポート名
    pub imports: Vec<String>,
    /// スタックサイズ（バイト）
    pub stack_size: usize,
    /// 初期メモリサイズ（ページ）
    pub initial_memory: usize,
    /// 最大メモリサイズ（ページ）
    pub maximum_memory: Option<usize>,
}

/// WebAssemblyバージョン
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WasmVersion {
    /// WebAssembly 1.0
    V1,
    /// WebAssembly 2.0
    V2,
}

/// ファジングオプション
#[derive(Debug, Clone)]
pub struct FuzzingOptions {
    /// ファジングの最大実行回数
    pub max_runs: usize,
    /// ファジングのシード値
    pub seed: Option<u64>,
    /// ファジングの対象関数
    pub target_functions: Vec<String>,
    /// ファジングの最大実行時間（秒）
    pub timeout: u64,
    /// ファジングのコーパスディレクトリ
    pub corpus_dir: Option<PathBuf>,
    /// クラッシュを保存するディレクトリ
    pub crash_dir: Option<PathBuf>,
    /// メモリ使用量の制限（バイト）
    pub memory_limit: usize,
    /// スレッド数
    pub threads: Option<NonZeroUsize>,
    /// カバレッジガイド付きファジングを有効にするかどうか
    pub coverage_guided: bool,
}

/// ベクトル化オプション
#[derive(Debug, Clone)]
pub struct VectorizationOptions {
    /// ベクトル化の最小ループ回数
    pub min_loop_iterations: usize,
    /// ベクトル化の最大ベクトル幅
    pub max_vector_width: usize,
    /// ベクトル化の強制
    pub force_vector_width: Option<usize>,
    /// インターリーブアクセスを有効にするかどうか
    pub interleaved_access: bool,
    /// ベクトル化の詳細レポートを生成するかどうか
    pub verbose_report: bool,
    /// 特定のSIMD命令セットを使用するかどうか
    pub simd_instruction_set: Option<SimdInstructionSet>,
}

/// SIMD命令セット
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SimdInstructionSet {
    /// SSE
    SSE,
    /// SSE2
    SSE2,
    /// SSE3
    SSE3,
    /// SSSE3
    SSSE3,
    /// SSE4.1
    SSE4_1,
    /// SSE4.2
    SSE4_2,
    /// AVX
    AVX,
    /// AVX2
    AVX2,
    /// AVX-512
    AVX512,
    /// NEON
    NEON,
    /// SVE
    SVE,
    /// 自動選択
    Auto,
}

/// メモリリーク検出オプション
#[derive(Debug, Clone)]
pub struct MemoryLeakOptions {
    /// リーク検出の詳細レベル
    pub detail_level: MemoryLeakDetailLevel,
    /// リーク検出のスタックトレース深さ
    pub stack_trace_depth: usize,
    /// リーク検出のレポートファイル
    pub report_file: Option<PathBuf>,
    /// リーク検出の無視リスト
    pub ignore_list: Vec<String>,
    /// リーク検出のしきい値（バイト）
    pub threshold: usize,
}

/// メモリリーク検出の詳細レベル
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryLeakDetailLevel {
    /// 基本的な情報のみ
    Basic,
    /// 詳細な情報
    Detailed,
    /// 完全な情報
    Full,
}

/// コード生成オプション
#[derive(Debug, Clone)]
pub struct CodegenOptions {
    /// インライン化の閾値
    pub inline_threshold: usize,
    /// ループアンロールの閾値
    pub loop_unroll_threshold: usize,
    /// 関数属性
    pub function_attributes: HashMap<String, Vec<String>>,
    /// ターゲット機能
    pub target_features: Vec<String>,
    /// コード生成ユニットの最大サイズ
    pub codegen_units: usize,
    /// リロケーションモデル
    pub relocation_model: RelocationModel,
    /// コードモデル
    pub code_model: CodeModel,
    /// スタックプローブタイプ
    pub stack_probe_type: StackProbeType,
    /// パニック戦略
    pub panic_strategy: PanicStrategy,
}

/// リロケーションモデル
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelocationModel {
    /// 静的
    Static,
    /// 位置独立実行ファイル
    Pic,
    /// 動的非PIC
    DynamicNoPic,
    /// ROPI
    Ropi,
    /// RWPI
    Rwpi,
    /// ROPI-RWPI
    RopiRwpi,
}

/// コードモデル
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodeModel {
    /// 小さい
    Small,
    /// カーネル
    Kernel,
    /// 中間
    Medium,
    /// 大きい
    Large,
    /// デフォルト
    Default,
}

/// スタックプローブタイプ
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StackProbeType {
    /// なし
    None,
    /// インライン
    Inline,
    /// 呼び出し
    Call,
}

/// パニック戦略
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanicStrategy {
    /// アンワインド
    Unwind,
    /// アボート
    Abort,
}

/// ドキュメント生成オプション
#[derive(Debug, Clone)]
pub struct DocOptions {
    /// ドキュメントの出力ディレクトリ
    pub output_dir: Option<PathBuf>,
    /// プライベート項目をドキュメント化するかどうか
    pub document_private_items: bool,
    /// 内部項目をドキュメント化するかどうか
    pub document_internal_items: bool,
    /// テストをドキュメントから実行するかどうか
    pub run_tests: bool,
    /// ドキュメントのテーマ
    pub theme: String,
    /// ドキュメントのロゴパス
    pub logo_path: Option<PathBuf>,
    /// ドキュメントのフッターテキスト
    pub footer_text: Option<String>,
    /// ドキュメントのヘッダーテキスト
    pub header_text: Option<String>,
    /// ドキュメントのメタデータ
    pub metadata: HashMap<String, String>,
}

/// 静的解析オプション
#[derive(Debug, Clone)]
pub struct StaticAnalysisOptions {
    /// 未使用コードの検出を有効にするかどうか
    pub detect_unused_code: bool,
    /// 未使用インポートの検出を有効にするかどうか
    pub detect_unused_imports: bool,
    /// 未使用変数の検出を有効にするかどうか
    pub detect_unused_variables: bool,
    /// デッドコードの検出を有効にするかどうか
    pub detect_dead_code: bool,
    /// 無限ループの検出を有効にするかどうか
    pub detect_infinite_loops: bool,
    /// 到達不能コードの検出を有効にするかどうか
    pub detect_unreachable_code: bool,
    /// 冗長なコードの検出を有効にするかどうか
    pub detect_redundant_code: bool,
    /// 複雑度の閾値
    pub complexity_threshold: usize,
    /// 解析の深さ
    pub analysis_depth: usize,
    /// 解析のタイムアウト（秒）
    pub analysis_timeout: u64,
}

/// 並行処理モデルオプション
#[derive(Debug, Clone)]
pub struct ConcurrencyOptions {
    /// 並行処理モデル
    pub model: ConcurrencyModel,
    /// スレッドプールのサイズ
    pub thread_pool_size: Option<NonZeroUsize>,
    /// タスクの最大数
    pub max_tasks: Option<usize>,
    /// タスクの優先度レベル
    pub priority_levels: usize,
    /// ワークスティーリングを有効にするかどうか
    pub work_stealing: bool,
    /// 並行データ構造の最適化
    pub optimize_concurrent_data_structures: bool,
    /// アクターモデルを有効にするかどうか
    pub enable_actor_model: bool,
    /// チャネルのバッファサイズ
    pub channel_buffer_size: usize,
    /// 並行GCを有効にするかどうか
    pub concurrent_gc: bool,
}

/// 並行処理モデル
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConcurrencyModel {
    /// スレッド
    Threads,
    /// アクター
    Actors,
    /// 非同期タスク
    AsyncTasks,
    /// コルーチン
    Coroutines,
    /// CSP
    Csp,
    /// STM
    Stm,
}

/// エラー回復戦略
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorRecoveryStrategy {
    /// パニック
    Panic,
    /// 継続
    Continue,
    /// スキップ
    Skip,
    /// 再試行
    Retry,
    /// カスタム
    Custom,
}

impl Default for CompilerConfig {
    fn default() -> Self {
        Self {
            target: Target::native(),
            optimization_level: OptimizationLevel::Default,
            target_options: TargetOptions::default(),
            debug_info: false,
            debug_info_level: 2,
            lto: false,
            lto_type: LtoType::Auto,
            include_paths: Vec::new(),
            library_paths: Vec::new(),
            libraries: Vec::new(),
            defines: Vec::new(),
            warnings_as_errors: false,
            ignored_warnings: Vec::new(),
            promoted_warnings: Vec::new(),
            diagnostic_level: DiagnosticLevel::Normal,
            parallel_build: true,
            parallel_threads: None,
            use_cache: true,
            cache_dir: None,
            keep_intermediates: false,
            verbose: false,
            ownership_check_level: OwnershipCheckLevel::Strict,
            type_check_level: TypeCheckLevel::Standard,
            enable_dependent_types: false,
            dependent_type_options: DependentTypeOptions::default(),
            compile_time_computation_limit: 10000,
            compile_time_memory_limit: 1024 * 1024 * 1024, // 1GB
            metaprogramming_options: MetaProgrammingOptions::default(),
            wasm_output: false,
            wasm_opt_level: 2,
            wasm_options: WasmOptions {
                version: WasmVersion::V1,
                enable_simd: true,
                enable_exceptions: false,
                enable_threads: false,
                enable_bulk_memory: true,
                enable_reference_types: true,
                enable_multi_value: true,
                enable_tail_call: false,
                enable_memory64: false,
                exports: Vec::new(),
                imports: Vec::new(),
                stack_size: 1024 * 1024, // 1MB
                initial_memory: 16, // 16 pages (1MB)
                maximum_memory: Some(256), // 256 pages (16MB)
            },
            formal_verification: false,
            formal_verification_options: FormalVerificationOptions::default(),
            fuzzing: false,
            fuzzing_options: FuzzingOptions {
                max_runs: 10000,
                seed: None,
                target_functions: Vec::new(),
                timeout: 60,
                corpus_dir: None,
                crash_dir: None,
                memory_limit: 2 * 1024 * 1024 * 1024, // 2GB
                threads: None,
                coverage_guided: true,
            },
            auto_vectorization: true,
            vectorization_options: VectorizationOptions {
                min_loop_iterations: 4,
                max_vector_width: 512,
                force_vector_width: None,
                interleaved_access: true,
                verbose_report: false,
                simd_instruction_set: None,
            },
            incremental_compilation: true,
            incremental_cache_dir: None,
            profile_guided_optimization: false,
            profile_data_path: None,
            security_checks: true,
            security_check_options: SecurityCheckOptions::default(),
            memory_leak_detection: true,
            memory_leak_options: MemoryLeakOptions {
                detail_level: MemoryLeakDetailLevel::Detailed,
                stack_trace_depth: 16,
                report_file: None,
                ignore_list: Vec::new(),
                threshold: 1024, // 1KB
            },
            optimization_pipeline: OptimizationPipeline::default(),
            codegen_options: CodegenOptions {
                inline_threshold: 225,
                loop_unroll_threshold: 150,
                function_attributes: HashMap::new(),
                target_features: Vec::new(),
                codegen_units: 16,
                relocation_model: RelocationModel::Pic,
                code_model: CodeModel::Default,
                stack_probe_type: StackProbeType::Inline,
                panic_strategy: PanicStrategy::Unwind,
            },
            language_extensions: HashMap::new(),
            experimental_features: HashMap::new(),
            generate_source_maps: true,
            generate_docs: false,
            doc_options: DocOptions {
                output_dir: None,
                document_private_items: false,
                document_internal_items: false,
                run_tests: false,
                theme: "default".to_string(),
                logo_path: None,
                footer_text: None,
                header_text: None,
                metadata: HashMap::new(),
            },
            static_analysis_options: StaticAnalysisOptions {
                detect_unused_code: true,
                detect_unused_imports: true,
                detect_unused_variables: true,
                detect_dead_code: true,
                detect_infinite_loops: true,
                detect_unreachable_code: true,
                detect_redundant_code: true,
                complexity_threshold: 20,
                analysis_depth: 5,
                analysis_timeout: 30,
            },
            concurrency_options: ConcurrencyOptions {
                model: ConcurrencyModel::AsyncTasks,
                thread_pool_size: None,
                max_tasks: None,
                priority_levels: 3,
                work_stealing: true,
                optimize_concurrent_data_structures: true,
                enable_actor_model: true,
                channel_buffer_size: 64,
                concurrent_gc: true,
            },
            error_recovery_strategy: ErrorRecoveryStrategy::Continue,
            plugin_paths: Vec::new(),
            plugin_options: HashMap::new(),
        }
    }
}

impl CompilerConfig {
    /// 新しいコンパイラ設定を作成
    pub fn new() -> Self {
        Self::default()
    }
    
    /// コンパイルオプションから設定を生成
    pub fn from_options(options: &CompileOptions) -> Self {
        let mut config = Self::default();
        
        // ターゲットを設定
        config.target = options.target;
        
        // 最適化レベルを設定
        config.optimization_level = match options.optimization_level {
            0 => OptimizationLevel::None,
            1 => OptimizationLevel::Basic,
            2 => OptimizationLevel::Standard,
            _ => OptimizationLevel::Aggressive,
        };
        
        // デバッグ情報を設定
        config.debug_info = options.debug_info;
        config.debug_info_level = options.debug_info_level.unwrap_or(2);
        
        // LTOを設定
        config.lto = options.lto;
        config.lto_type = options.lto_type.unwrap_or(LtoType::Auto);
        
        // インクルードパスを設定
        if !options.include_paths.is_empty() {
            config.include_paths = options.include_paths.clone();
        }
        
        // ライブラリパスを設定
        if !options.library_paths.is_empty() {
            config.library_paths = options.library_paths.clone();
        }
        
        // リンクするライブラリを設定
        if !options.libraries.is_empty() {
            config.libraries = options.libraries.clone();
        }
        
        // 定義済みマクロを設定
        if !options.defines.is_empty() {
            config.defines = options.defines.clone();
        }
        
        // 警告をエラーとして扱うかどうかを設定
        config.warnings_as_errors = options.warnings_as_errors;
        
        // 無視する警告を設定
        if !options.ignored_warnings.is_empty() {
            config.ignored_warnings = options.ignored_warnings.clone();
        }
        
        // エラーに昇格する警告を設定
        if !options.promoted_warnings.is_empty() {
            config.promoted_warnings = options.promoted_warnings.clone();
        }
        
        // 診断レベルを設定
        if let Some(level) = options.diagnostic_level {
            config.diagnostic_level = level;
        }
        
        // 並列ビルドを設定
        config.parallel_build = options.parallel_build;
        
        // 並列スレッド数を設定
        config.parallel_threads = options.parallel_threads;
        
        // キャッシュを設定
        config.use_cache = options.use_cache;
        
        // キャッシュディレクトリを設定
        if let Some(cache_dir) = &options.cache_dir {
            config.cache_dir = Some(cache_dir.clone());
        }
        
        // 中間ファイルを保持するかどうかを設定
        config.keep_intermediates = options.keep_intermediates;
        
        // 詳細な出力を設定
        config.verbose = options.verbose;
        
        // 所有権チェックレベルを設定
        config.ownership_check_level = options.ownership_check_level;
        
        // 型チェックレベルを設定
        if let Some(level) = options.type_check_level {
            config.type_check_level = level;
        }
        
        // 依存型チェックを設定
        config.enable_dependent_types = options.enable_dependent_types;
        
        // 依存型オプションを設定
        if let Some(opts) = &options.dependent_type_options {
            config.dependent_type_options = opts.clone();
        }
        
        // コンパイル時計算の制限を設定
        config.compile_time_computation_limit = options.compile_time_computation_limit;
        
        // コンパイル時メモリ制限を設定
        if let Some(limit) = options.compile_time_memory_limit {
            config.compile_time_memory_limit = limit;
        }
        
        // メタプログラミングオプションを設定
        if let Some(opts) = &options.metaprogramming_options {
            config.metaprogramming_options = opts.clone();
        }
        
        // WebAssembly出力を設定
        config.wasm_output = options.wasm_output;
        
        // WebAssembly最適化レベルを設定
        config.wasm_opt_level = options.wasm_opt_level;
        
        // WebAssemblyオプションを設定
        if let Some(opts) = &options.wasm_options {
            config.wasm_options = opts.clone();
        }
        
        // 形式検証を設定
        config.formal_verification = options.formal_verification;
        
        // 形式検証オプションを設定
        if let Some(opts) = &options.formal_verification_options {
            config.formal_verification_options = opts.clone();
        }
        
        // ファジングを設定
        config.fuzzing = options.fuzzing;
        
        // 自動ベクトル化を設定
        config.auto_vectorization = options.auto_vectorization;
        
        // インクリメンタルコンパイルを設定
        config.incremental_compilation = options.incremental_compilation;
        
        // インクリメンタルキャッシュディレクトリを設定
        if let Some(dir) = &options.incremental_cache_dir {
            config.incremental_cache_dir = Some(dir.clone());
        }
        
        // プロファイルに基づく最適化を設定
        config.profile_guided_optimization = options.profile_guided_optimization;
        
        // プロファイルデータパスを設定
        if let Some(path) = &options.profile_data_path {
            config.profile_data_path = Some(path.clone());
        }
        
        // セキュリティチェックを設定
        config.security_checks = options.security_checks;
        
        // メモリリーク検出を設定
        config.memory_leak_detection = options.memory_leak_detection;
        
        config
    }
    
    /// インクルードパスを追加
    pub fn add_include_path(&mut self, path: PathBuf) -> &mut Self {
        self.include_paths.push(path);
        self
    }
    
    /// ライブラリパスを追加
    pub fn add_library_path(&mut self, path: PathBuf) -> &mut Self {
        self.library_paths.push(path);
        self
    }
    
    /// リンクするライブラリを追加
    pub fn add_library(&mut self, library: &str) -> &mut Self {
        self.libraries.push(library.to_string());
        self
    }
    
    /// 定義済みマクロを追加
    pub fn add_define(&mut self, name: &str, value: Option<&str>) -> &mut Self {
        self.defines.push((
            name.to_string(),
            value.map(|v| v.to_string())
        ));
        self
    }
    
    /// 所有権チェックレベルを設定
    pub fn set_ownership_check_level(&mut self, level: OwnershipCheckLevel) -> &mut Self {
        self.ownership_check_level = level;
        self
    }
    
    /// 依存型チェックを有効化/無効化
    pub fn set_dependent_types(&mut self, enable: bool) -> &mut Self {
        self.enable_dependent_types = enable;
        self
    }
    
    /// WebAssembly出力を有効化/無効化
    pub fn set_wasm_output(&mut self, enable: bool) -> &mut Self {
        self.wasm_output = enable;
        self
    }
    
    /// WebAssembly最適化レベルを設定
    pub fn set_wasm_opt_level(&mut self, level: u8) -> &mut Self {
        self.wasm_opt_level = level.min(4); // 0-4の範囲に制限
        self
    }
    
    /// 形式検証を有効化/無効化
    pub fn set_formal_verification(&mut self, enable: bool) -> &mut Self {
        self.formal_verification = enable;
        self
    }
    
    /// ファジングを有効化/無効化
    pub fn set_fuzzing(&mut self, enable: bool) -> &mut Self {
        self.fuzzing = enable;
        self
    }
    
    /// 自動ベクトル化を有効化/無効化
    pub fn set_auto_vectorization(&mut self, enable: bool) -> &mut Self {
        self.auto_vectorization = enable;
        self
    }
    
    /// インクリメンタルコンパイルを有効化/無効化
    pub fn set_incremental_compilation(&mut self, enable: bool) -> &mut Self {
        self.incremental_compilation = enable;
        self
    }
    
    /// プロファイルに基づく最適化を有効化/無効化
    pub fn set_profile_guided_optimization(&mut self, enable: bool) -> &mut Self {
        self.profile_guided_optimization = enable;
        self
    }
    
    /// セキュリティチェックを有効化/無効化
    pub fn set_security_checks(&mut self, enable: bool) -> &mut Self {
        self.security_checks = enable;
        self
    }
    
    /// メモリリーク検出を有効化/無効化
    pub fn set_memory_leak_detection(&mut self, enable: bool) -> &mut Self {
        self.memory_leak_detection = enable;
        self
    }
    
    /// 並列スレッド数を設定
    pub fn set_parallel_threads(&mut self, threads: Option<NonZeroUsize>) -> &mut Self {
        self.parallel_threads = threads;
        self
    }
    
    /// コンパイル時計算の制限を設定
    pub fn set_compile_time_computation_limit(&mut self, limit: usize) -> &mut Self {
        self.compile_time_computation_limit = limit;
        self
    }
}
