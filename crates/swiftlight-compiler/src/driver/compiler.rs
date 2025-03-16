//! # コンパイラドライバー
//! 
//! コンパイルプロセス全体を実行するドライバークラスを提供します。
//! ソースコードの解析からコード生成までのパイプラインを管理し、
//! 並列処理やインクリメンタルコンパイルをサポートします。
//! 高度な最適化、メモリ効率、ビルド速度を重視した設計となっています。

use std::path::{Path, PathBuf};
use std::fs;
use std::collections::{HashMap, HashSet, VecDeque};
use std::time::{Instant, Duration};
use std::sync::{Arc, Mutex, RwLock};
use std::io::{self, Write};
use rayon::prelude::*;
use parking_lot::{ReentrantMutex, Condvar};
use crossbeam_channel::{bounded, Sender, Receiver};
use dashmap::DashMap;
use memmap2::MmapOptions;

use crate::frontend::{
    lexer::Lexer,
    parser::Parser,
    semantic::SemanticAnalyzer,
    ast::{Program, Node, NodeId, NodeKind},
    error::{CompilerError, ErrorKind, Result, SourceLocation, DiagnosticReporter, Severity},
    symbol_table::{SymbolTable, Symbol, SymbolKind, Visibility},
    type_checker::TypeChecker,
    name_resolution::NameResolver,
    dependency_analyzer::DependencyAnalyzer,
    macro_expander::MacroExpander,
    constant_evaluator::ConstantEvaluator
};
use crate::middleend::{
    ir,
    optimization::{self, OptimizationLevel, PassManager, Pass, PassContext},
    ir_validator::IRValidator,
    ir_generator::IRGenerator,
    ir_serializer::IRSerializer,
    ir_deserializer::IRDeserializer,
    dataflow_analyzer::DataFlowAnalyzer,
    alias_analyzer::AliasAnalyzer,
    memory_layout_optimizer::MemoryLayoutOptimizer,
    parallel_region_analyzer::ParallelRegionAnalyzer
};
use crate::backend::{
    self, Target, Backend, CodegenOptions, TargetFeature, 
    register_allocator::RegisterAllocator,
    instruction_scheduler::InstructionScheduler,
    code_emitter::CodeEmitter,
    binary_generator::BinaryGenerator,
    debug_info_generator::DebugInfoGenerator,
    platform_specific::PlatformSpecificGenerator
};
use crate::driver::{
    config::CompilerConfig,
    options::CompileOptions,
    dependency::{DependencyGraph, DependencyNode, DependencyType},
    cache::{CompilationCache, CacheEntry, CacheMetadata, CacheStrategy},
    incremental::{IncrementalCompilationManager, ChangeDetector, ChangeImpactAnalyzer},
    module_manager::ModuleManager,
    plugin_manager::PluginManager,
    build_plan::BuildPlan
};
use crate::utils::{
    profiler::{Profiler, ProfilingEvent, ProfilingScope},
    file_system::{FileSystem, VirtualFileSystem, FileWatcher, FileChangeEvent},
    parallel::{WorkQueue, Task, TaskPriority, ThreadPool},
    memory_tracker::{MemoryTracker, MemoryUsageSnapshot},
    hash::{HashAlgorithm, ContentHasher},
    logging::{Logger, LogLevel, LogMessage},
    error_formatter::{ErrorFormatter, FormattingOptions},
    string_interner::StringInterner,
    arena::{Arena, TypedArena},
    config_parser::ConfigParser
};

/// コンパイル時の詳細な統計情報
#[derive(Debug, Default, Clone)]
pub struct CompileStats {
    /// 字句解析にかかった時間
    pub lexing_time: Duration,
    /// 構文解析にかかった時間
    pub parsing_time: Duration,
    /// 名前解決にかかった時間
    pub name_resolution_time: Duration,
    /// 型チェックにかかった時間
    pub type_checking_time: Duration,
    /// 意味解析にかかった時間（名前解決と型チェックを含む）
    pub semantic_time: Duration,
    /// マクロ展開にかかった時間
    pub macro_expansion_time: Duration,
    /// 定数評価にかかった時間
    pub constant_evaluation_time: Duration,
    /// 依存関係分析にかかった時間
    pub dependency_analysis_time: Duration,
    /// IR生成にかかった時間
    pub ir_gen_time: Duration,
    /// IR検証にかかった時間
    pub ir_validation_time: Duration,
    /// データフロー分析にかかった時間
    pub dataflow_analysis_time: Duration,
    /// エイリアス分析にかかった時間
    pub alias_analysis_time: Duration,
    /// メモリレイアウト最適化にかかった時間
    pub memory_layout_time: Duration,
    /// 並列領域分析にかかった時間
    pub parallel_region_time: Duration,
    /// 最適化にかかった時間
    pub optimization_time: Duration,
    /// コード生成にかかった時間
    pub codegen_time: Duration,
    /// レジスタ割り当てにかかった時間
    pub register_allocation_time: Duration,
    /// 命令スケジューリングにかかった時間
    pub instruction_scheduling_time: Duration,
    /// バイナリ生成にかかった時間
    pub binary_generation_time: Duration,
    /// デバッグ情報生成にかかった時間
    pub debug_info_time: Duration,
    /// リンク時間
    pub linking_time: Duration,
    /// 合計コンパイル時間
    pub total_time: Duration,
    /// コンパイルしたファイル数
    pub file_count: usize,
    /// 合計行数
    pub total_lines: usize,
    /// 検出されたエラー数
    pub error_count: usize,
    /// 検出された警告数
    pub warning_count: usize,
    /// キャッシュヒット数
    pub cache_hits: usize,
    /// キャッシュミス数
    pub cache_misses: usize,
    /// インクリメンタルビルドでスキップされたファイル数
    pub skipped_files: usize,
    /// メモリ使用量（ピーク時、バイト単位）
    pub peak_memory_usage: usize,
    /// 現在のメモリ使用量（バイト単位）
    pub current_memory_usage: usize,
    /// 各最適化パスの実行時間
    pub optimization_passes: HashMap<String, Duration>,
    /// 並列処理で使用したスレッド数
    pub thread_count: usize,
    /// 並列タスクの総数
    pub total_tasks: usize,
    /// 並列タスクの最大同時実行数
    pub max_concurrent_tasks: usize,
    /// I/O待ち時間
    pub io_wait_time: Duration,
    /// CPU使用率（0.0〜1.0）
    pub cpu_usage: f64,
    /// 各フェーズのメモリ使用量
    pub phase_memory_usage: HashMap<String, usize>,
    /// 各モジュールのコンパイル時間
    pub module_compile_times: HashMap<String, Duration>,
    /// 依存関係グラフの深さ
    pub dependency_graph_depth: usize,
    /// 依存関係グラフのノード数
    pub dependency_graph_nodes: usize,
    /// 依存関係グラフのエッジ数
    pub dependency_graph_edges: usize,
    /// 型チェックで検出された型の数
    pub type_count: usize,
    /// シンボルテーブルのエントリ数
    pub symbol_count: usize,
    /// 生成されたIR命令の数
    pub ir_instruction_count: usize,
    /// 生成されたアセンブリ命令の数
    pub assembly_instruction_count: usize,
    /// 生成されたバイナリのサイズ（バイト単位）
    pub binary_size: usize,
    /// 並列コンパイルの効率（0.0〜1.0）
    pub parallel_efficiency: f64,
    /// コンパイル速度（行/秒）
    pub compilation_speed: f64,
    /// 最適化による削減率（0.0〜1.0）
    pub optimization_reduction_rate: f64,
    /// キャッシュ使用量（バイト単位）
    pub cache_size: usize,
    /// プラグインの実行時間
    pub plugin_execution_time: Duration,
    /// 増分コンパイルの分析時間
    pub incremental_analysis_time: Duration,
}

impl CompileStats {
    /// 新しい統計情報を作成
    pub fn new(thread_count: usize) -> Self {
        Self {
            thread_count,
            ..CompileStats::default()
        }
    }

    /// 別の統計情報を合併する
    pub fn merge(&mut self, other: &CompileStats) {
        self.lexing_time += other.lexing_time;
        self.parsing_time += other.parsing_time;
        self.name_resolution_time += other.name_resolution_time;
        self.type_checking_time += other.type_checking_time;
        self.semantic_time += other.semantic_time;
        self.macro_expansion_time += other.macro_expansion_time;
        self.constant_evaluation_time += other.constant_evaluation_time;
        self.dependency_analysis_time += other.dependency_analysis_time;
        self.ir_gen_time += other.ir_gen_time;
        self.ir_validation_time += other.ir_validation_time;
        self.dataflow_analysis_time += other.dataflow_analysis_time;
        self.alias_analysis_time += other.alias_analysis_time;
        self.memory_layout_time += other.memory_layout_time;
        self.parallel_region_time += other.parallel_region_time;
        self.optimization_time += other.optimization_time;
        self.codegen_time += other.codegen_time;
        self.register_allocation_time += other.register_allocation_time;
        self.instruction_scheduling_time += other.instruction_scheduling_time;
        self.binary_generation_time += other.binary_generation_time;
        self.debug_info_time += other.debug_info_time;
        self.linking_time += other.linking_time;
        self.file_count += other.file_count;
        self.total_lines += other.total_lines;
        self.error_count += other.error_count;
        self.warning_count += other.warning_count;
        self.cache_hits += other.cache_hits;
        self.cache_misses += other.cache_misses;
        self.skipped_files += other.skipped_files;
        self.peak_memory_usage = self.peak_memory_usage.max(other.peak_memory_usage);
        self.total_tasks += other.total_tasks;
        self.max_concurrent_tasks = self.max_concurrent_tasks.max(other.max_concurrent_tasks);
        self.io_wait_time += other.io_wait_time;
        self.type_count += other.type_count;
        self.symbol_count += other.symbol_count;
        self.ir_instruction_count += other.ir_instruction_count;
        self.assembly_instruction_count += other.assembly_instruction_count;
        self.binary_size += other.binary_size;
        
        // 最適化パスの時間を合併
        for (pass_name, duration) in &other.optimization_passes {
            *self.optimization_passes.entry(pass_name.clone()).or_insert(Duration::default()) += *duration;
        }
        
        // モジュールコンパイル時間を合併
        for (module_name, duration) in &other.module_compile_times {
            *self.module_compile_times.entry(module_name.clone()).or_insert(Duration::default()) += *duration;
        }
        
        // フェーズメモリ使用量を合併
        for (phase_name, usage) in &other.phase_memory_usage {
            let entry = self.phase_memory_usage.entry(phase_name.clone()).or_insert(0);
            *entry = (*entry).max(*usage);
        }
    }
    
    /// 統計情報を計算する
    pub fn calculate_derived_stats(&mut self) {
        if !self.total_time.is_zero() {
            // コンパイル速度（行/秒）
            self.compilation_speed = self.total_lines as f64 / self.total_time.as_secs_f64();
            
            // CPU使用率
            let total_thread_time = self.lexing_time + self.parsing_time + self.semantic_time + 
                                   self.ir_gen_time + self.optimization_time + self.codegen_time;
            self.cpu_usage = (total_thread_time.as_secs_f64() / 
                             (self.total_time.as_secs_f64() * self.thread_count as f64))
                             .min(1.0);
            
            // 並列効率
            if self.thread_count > 1 {
                let ideal_time = total_thread_time.as_secs_f64() / self.thread_count as f64;
                self.parallel_efficiency = (ideal_time / self.total_time.as_secs_f64()).min(1.0);
            } else {
                self.parallel_efficiency = 1.0;
            }
        }
        
        // 最適化による削減率
        if self.ir_instruction_count > 0 && self.assembly_instruction_count > 0 {
            let reduction = self.ir_instruction_count.saturating_sub(self.assembly_instruction_count);
            self.optimization_reduction_rate = reduction as f64 / self.ir_instruction_count as f64;
        }
    }
    
    /// 統計情報をJSONフォーマットで出力
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_else(|_| "{}".to_string())
    }
    
    /// 統計情報をCSVフォーマットで出力
    pub fn to_csv(&self) -> String {
        let mut csv = String::new();
        csv.push_str("メトリック,値\n");
        csv.push_str(&format!("ファイル数,{}\n", self.file_count));
        csv.push_str(&format!("合計行数,{}\n", self.total_lines));
        csv.push_str(&format!("字句解析時間,{:?}\n", self.lexing_time));
        csv.push_str(&format!("構文解析時間,{:?}\n", self.parsing_time));
        csv.push_str(&format!("意味解析時間,{:?}\n", self.semantic_time));
        csv.push_str(&format!("IR生成時間,{:?}\n", self.ir_gen_time));
        csv.push_str(&format!("最適化時間,{:?}\n", self.optimization_time));
        csv.push_str(&format!("コード生成時間,{:?}\n", self.codegen_time));
        csv.push_str(&format!("合計時間,{:?}\n", self.total_time));
        csv.push_str(&format!("キャッシュヒット数,{}\n", self.cache_hits));
        csv.push_str(&format!("キャッシュミス数,{}\n", self.cache_misses));
        csv.push_str(&format!("スキップされたファイル数,{}\n", self.skipped_files));
        csv.push_str(&format!("ピークメモリ使用量,{} バイト\n", self.peak_memory_usage));
        csv.push_str(&format!("コンパイル速度,{:.2} 行/秒\n", self.compilation_speed));
        csv.push_str(&format!("CPU使用率,{:.2}%\n", self.cpu_usage * 100.0));
        csv.push_str(&format!("並列効率,{:.2}%\n", self.parallel_efficiency * 100.0));
        csv
    }
}

/// コンパイル結果
#[derive(Debug)]
pub struct CompileResult {
    /// コンパイル成功したかどうか
    pub success: bool,
    /// 統計情報
    pub stats: CompileStats,
    /// 生成されたファイルのパス
    pub output_files: Vec<PathBuf>,
    /// エラーと警告のリスト
    pub diagnostics: Vec<CompilerError>,
    /// 生成されたIRモジュール
    pub ir_modules: HashMap<String, Arc<ir::Module>>,
    /// 生成されたバイナリデータ
    pub binary_data: Option<Vec<u8>>,
    /// デバッグ情報
    pub debug_info: Option<Vec<u8>>,
    /// 依存関係グラフ
    pub dependency_graph: Option<DependencyGraph>,
    /// ビルド成果物のハッシュ
    pub output_hash: Option<String>,
    /// コンパイル時に生成された追加情報
    pub metadata: HashMap<String, String>,
}

impl CompileResult {
    /// 新しいコンパイル結果を作成
    pub fn new(success: bool, stats: CompileStats) -> Self {
        Self {
            success,
            stats,
            output_files: Vec::new(),
            diagnostics: Vec::new(),
            ir_modules: HashMap::new(),
            binary_data: None,
            debug_info: None,
            dependency_graph: None,
            output_hash: None,
            metadata: HashMap::new(),
        }
    }
    
    /// 出力ファイルを追加
    pub fn add_output_file(&mut self, path: PathBuf) {
        self.output_files.push(path);
    }
    
    /// 診断情報を追加
    pub fn add_diagnostic(&mut self, diagnostic: CompilerError) {
        match diagnostic.kind {
            ErrorKind::Error | ErrorKind::Fatal => self.stats.error_count += 1,
            ErrorKind::Warning => self.stats.warning_count += 1,
            _ => {}
        }
        self.diagnostics.push(diagnostic);
    }
    
    /// IRモジュールを追加
    pub fn add_ir_module(&mut self, name: String, module: Arc<ir::Module>) {
        self.ir_modules.insert(name, module);
    }
    
    /// バイナリデータを設定
    pub fn set_binary_data(&mut self, data: Vec<u8>) {
        self.binary_data = Some(data);
    }
    
    /// デバッグ情報を設定
    pub fn set_debug_info(&mut self, data: Vec<u8>) {
        self.debug_info = Some(data);
    }
    
    /// 依存関係グラフを設定
    pub fn set_dependency_graph(&mut self, graph: DependencyGraph) {
        self.dependency_graph = Some(graph);
    }
    
    /// 出力ハッシュを設定
    pub fn set_output_hash(&mut self, hash: String) {
        self.output_hash = Some(hash);
    }
    
    /// メタデータを追加
    pub fn add_metadata(&mut self, key: String, value: String) {
        self.metadata.insert(key, value);
    }
    
    /// エラーがあるかどうかを確認
    pub fn has_errors(&self) -> bool {
        self.stats.error_count > 0
    }
    
    /// 警告があるかどうかを確認
    pub fn has_warnings(&self) -> bool {
        self.stats.warning_count > 0
    }
    
    /// 結果を標準出力に表示
    pub fn print_summary(&self, verbose: bool) {
        if self.success {
            println!("コンパイル成功: {} ファイル, {} 行, {:?}", 
                     self.stats.file_count, self.stats.total_lines, self.stats.total_time);
        } else {
            println!("コンパイル失敗: {} エラー, {} 警告", 
                     self.stats.error_count, self.stats.warning_count);
        }
        
        if verbose {
            println!("出力ファイル:");
            for file in &self.output_files {
                println!("  {}", file.display());
            }
            
            if let Some(hash) = &self.output_hash {
                println!("出力ハッシュ: {}", hash);
            }
            
            println!("統計情報:");
            println!("  字句解析時間: {:?}", self.stats.lexing_time);
            println!("  構文解析時間: {:?}", self.stats.parsing_time);
            println!("  意味解析時間: {:?}", self.stats.semantic_time);
            println!("  IR生成時間: {:?}", self.stats.ir_gen_time);
            println!("  最適化時間: {:?}", self.stats.optimization_time);
            println!("  コード生成時間: {:?}", self.stats.codegen_time);
            println!("  合計時間: {:?}", self.stats.total_time);
            println!("  キャッシュヒット: {}", self.stats.cache_hits);
            println!("  キャッシュミス: {}", self.stats.cache_misses);
            println!("  スキップされたファイル: {}", self.stats.skipped_files);
            println!("  ピークメモリ使用量: {} MB", self.stats.peak_memory_usage / (1024 * 1024));
            println!("  コンパイル速度: {:.2} 行/秒", self.stats.compilation_speed);
            println!("  CPU使用率: {:.2}%", self.stats.cpu_usage * 100.0);
            println!("  並列効率: {:.2}%", self.stats.parallel_efficiency * 100.0);
        }
    }
}

/// コンパイラドライバー
pub struct Driver {
    /// コンパイラの設定
    config: CompilerConfig,
    /// コンパイルオプション
    options: CompileOptions,
    /// コンパイル統計情報
    stats: CompileStats,
    /// 現在処理中のファイル
    current_file: Option<PathBuf>,
    /// コンパイル済みモジュール
    compiled_modules: DashMap<String, Arc<ir::Module>>,
    /// 依存関係グラフ
    dependency_graph: DependencyGraph,
    /// コンパイルキャッシュ
    compilation_cache: CompilationCache,
    /// 診断レポーター
    diagnostic_reporter: DiagnosticReporter,
    /// プロファイラー
    profiler: Profiler,
    /// ファイルシステムインターフェース
    file_system: FileSystem,
    /// 並列処理ワークキュー
    work_queue: WorkQueue,
    /// グローバルシンボルテーブル
    global_symbols: Arc<RwLock<SymbolTable>>,
    /// モジュールマネージャー
    module_manager: ModuleManager,
    /// プラグインマネージャー
    plugin_manager: PluginManager,
    /// インクリメンタルコンパイル管理
    incremental_manager: IncrementalCompilationManager,
    /// メモリトラッカー
    memory_tracker: MemoryTracker,
    /// 文字列インターナー
    string_interner: StringInterner,
    /// ビルドプラン
    build_plan: BuildPlan,
    /// ロガー
    logger: Logger,
    /// 型アリーナ
    type_arena: TypedArena<ir::Type>,
    /// 命令アリーナ
    instruction_arena: TypedArena<ir::Instruction>,
    /// ファイル変更監視
    file_watcher: Option<FileWatcher>,
    /// 変更検出器
    change_detector: ChangeDetector,
    /// 変更影響分析器
    change_impact_analyzer: ChangeImpactAnalyzer,
    /// コンパイル中のモジュール
    modules_in_progress: DashMap<String, ()>,
    /// コンパイル待機キュー
    waiting_modules: Mutex<VecDeque<String>>,
    /// コンパイル条件変数
    compile_condvar: Condvar,
    /// スレッドプール
    thread_pool: ThreadPool,
    /// 仮想ファイルシステム
    virtual_fs: VirtualFileSystem,
    /// エラーフォーマッタ
    error_formatter: ErrorFormatter,
    /// コンテンツハッシャー
    content_hasher: ContentHasher,
    /// コンフィグパーサー
    config_parser: ConfigParser,
}

impl Driver {
    /// 新しいドライバーを作成
    pub fn new(options: CompileOptions) -> Self {
        let config = CompilerConfig::from_options(&options);
        let thread_count = options.thread_count.unwrap_or_else(num_cpus::get);
        
        // メモリトラッカーを初期化
        let memory_tracker = MemoryTracker::new(options.memory_limit);
        
        // プロファイラーを初期化
        let profiler = Profiler::new(options.enable_profiling);
        
        // ロガーを初期化
        let log_level = if options.verbose {
            LogLevel::Debug
        } else if options.quiet {
            LogLevel::Error
        } else {
            LogLevel::Info
        };
        let logger = Logger::new(log_level, options.log_file.clone());
        
        // ファイルシステムを初期化
        let file_system = FileSystem::new();
        
        // 仮想ファイルシステムを初期化
        let virtual_fs = VirtualFileSystem::new();
        
        // 文字列インターナーを初期化
        let string_interner = StringInterner::new();
        
        // エラーフォーマッタを初期化
        let formatting_options = FormattingOptions {
            color_output: options.color_output,
            error_format: options.error_format.clone(),
            show_line_numbers: true,
            show_source_code: true,
            max_context_lines: 3,
        };
        let error_formatter = ErrorFormatter::new(formatting_options);
        
        // 診断レポーターを初期化
        let diagnostic_reporter = DiagnosticReporter::new(options.error_format.clone(), options.color_output);
        
        // コンパイルキャッシュを初期化
        let cache_strategy = if options.incremental {
            CacheStrategy::Incremental
        } else if options.use_cache {
            CacheStrategy::Full
        } else {
            CacheStrategy::Disabled
        };
        let compilation_cache = CompilationCache::new_with_strategy(
            options.cache_dir.clone(),
            cache_strategy,
            options.cache_size_limit,
        );
        
        // コンテンツハッシャーを初期化
        let hash_algorithm = match options.hash_algorithm.as_deref() {
            Some("sha256") => HashAlgorithm::SHA256,
            Some("sha1") => HashAlgorithm::SHA1,
            Some("xxhash") => HashAlgorithm::XXHash,
            _ => HashAlgorithm::XXHash, // デフォルトは高速なXXHash
        };
        let content_hasher = ContentHasher::new(hash_algorithm);
        
        // 依存関係グラフを初期化
        let dependency_graph = DependencyGraph::new();
        
        // モジュールマネージャーを初期化
        let module_manager = ModuleManager::new();
        
        // プラグインマネージャーを初期化
        let plugin_manager = PluginManager::new(options.plugin_paths.clone());
        
        // インクリメンタルコンパイル管理を初期化
        let incremental_manager = IncrementalCompilationManager::new(
            options.incremental_dir.clone(),
            options.incremental,
        );
        
        // 変更検出器を初期化
        let change_detector = ChangeDetector::new();
        
        // 変更影響分析器を初期化
        let change_impact_analyzer = ChangeImpactAnalyzer::new();
        
        // ビルドプランを初期化
        let build_plan = BuildPlan::new();
        
        // ファイル変更監視を初期化（ウォッチモードの場合）
        let file_watcher = if options.watch_mode {
            Some(FileWatcher::new())
        } else {
            None
        };
        
        // スレッドプールを初期化
        let thread_pool = ThreadPool::new(thread_count);
        
        // コンフィグパーサーを初期化
        let config_parser = ConfigParser::new();
        
        // 型アリーナとインストラクションアリーナを初期化
        let type_arena = TypedArena::new();
        let instruction_arena = TypedArena::new();
        
        // ワークキューを初期化
        Self {
            config,
            options: options.clone(),
            stats: CompileStats {
                thread_count,
                ..CompileStats::default()
            },
            current_file: None,
            compiled_modules: HashMap::new(),
            dependency_graph,
            compilation_cache: CompilationCache::new(options.cache_dir.clone()),
            diagnostic_reporter: DiagnosticReporter::new(options.error_format, options.color_output, options.max_errors),
            profiler: Profiler::new(options.enable_profiling),
            file_system: FileSystem::new(),
            work_queue: WorkQueue::new(thread_count),
            global_symbols: Arc::new(Mutex::new(SymbolTable::new_global())),
            module_manager: ModuleManager::new(),
            plugin_manager: PluginManager::new(options.plugin_paths.clone()),
            incremental_manager: IncrementalCompilationManager::new(
                options.incremental_dir.clone(),
                options.incremental,
            ),
            change_detector: ChangeDetector::new(),
            change_impact_analyzer: ChangeImpactAnalyzer::new(),
            build_plan: BuildPlan::new(),
            file_watcher: if options.watch_mode {
                Some(FileWatcher::new())
            } else {
                None
            },
            thread_pool: ThreadPool::new(thread_count),
            string_interner: StringInterner::new(),
            config_parser: ConfigParser::new(),
            type_arena: TypedArena::new(),
            instruction_arena: TypedArena::new(),
            waiting_modules: Mutex::new(VecDeque::new()),
            compile_condvar: Condvar::new(),
            virtual_fs: VirtualFileSystem::new(),
            error_formatter: ErrorFormatter::new(formatting_options),
            content_hasher: ContentHasher::new(hash_algorithm),
            modules_in_progress: DashMap::new(),
            memory_tracker,
            logger,
        }
    }
    
    /// 単一ファイルのコンパイルを実行
    pub fn compile<P: AsRef<Path>>(&mut self, source_path: P, output_path: P) -> Result<CompileResult> {
        let source_path = source_path.as_ref();
        let output_path = output_path.as_ref();
        
        // コンパイル開始時間を記録
        let start_time = Instant::now();
        self.profiler.start("total_compilation");
        
        // ソースファイルを読み込み
        let source_code = self.file_system.read_to_string(source_path)
            .map_err(|e| CompilerError::new(
                ErrorKind::IO,
                format!("ソースファイル読み込みエラー: {}", e),
                None
            ))?;
        
        // 現在のファイルを設定
        self.current_file = Some(source_path.to_path_buf());
        
        // モジュール名を取得（ファイル名から拡張子を除いたもの）
        let module_name = source_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("main")
            .to_string();
        
        // キャッシュをチェック
        let cache_key = self.compilation_cache.compute_key(source_path, &self.options);
        let use_cached = if self.options.use_cache {
            if let Some(cached_module) = self.compilation_cache.get(&cache_key) {
                self.compiled_modules.insert(module_name.clone(), Arc::new(cached_module));
                self.stats.cache_hits += 1;
                true
            } else {
                false
            }
        } else {
            false
        };
        
        let ir_module = if !use_cached {
            // コンパイルパイプラインを実行
            let ir_module = self.compile_source(&source_code, &module_name)?;
            
            // キャッシュに保存
            if self.options.use_cache {
                self.compilation_cache.put(&cache_key, &ir_module);
            }
            
            ir_module
        } else {
            // キャッシュから取得したモジュールを使用
            self.compiled_modules.get(&module_name).unwrap().clone()
        };
        
        // 適切なバックエンドを選択
        let backend = backend::create_backend(self.options.target, &self.options.target_features);
        
        // コード生成を実行
        self.profiler.start("codegen");
        let codegen_options = CodegenOptions {
            optimization_level: self.options.optimization_level,
            debug_info: self.options.debug_info,
            target_features: self.options.target_features.clone(),
        };
        let code = backend.generate_code(&ir_module)?;
        self.stats.codegen_time = self.profiler.stop("codegen");
        
        // 出力ファイルに書き込み
        backend.write_to_file(&code, output_path)?;
        
        // 合計時間を記録
        self.stats.total_time = self.profiler.stop("total_compilation");
        self.stats.file_count = 1;
        self.stats.total_lines = source_code.lines().count();
        
        // 統計情報を表示（オプションが有効な場合）
        if self.options.show_stats {
            self.print_stats();
        }
        
        // プロファイル情報を出力（オプションが有効な場合）
        if self.options.enable_profiling {
            self.profiler.write_report(&self.options.profile_output);
        }
        
        // コンパイル結果を作成
        let result = CompileResult {
            success: self.diagnostic_reporter.error_count() == 0,
            stats: self.stats.clone(),
            output_files: vec![output_path.to_path_buf()],
            diagnostics: self.diagnostic_reporter.get_diagnostics(),
            ir_modules: HashMap::new(),
            binary_data: None,
            debug_info: None,
            dependency_graph: None,
            output_hash: None,
            metadata: HashMap::new(),
        };
        
        Ok(result)
    }
    
    /// 複数ファイルのコンパイルを実行
    pub fn compile_multiple<P: AsRef<Path>>(&mut self, source_paths: &[P], output_path: P) -> Result<CompileResult> {
        let output_path = output_path.as_ref();
        
        // コンパイル開始時間を記録
        let start_time = Instant::now();
        self.profiler.start("total_compilation");
        
        // 依存関係グラフを構築
        self.profiler.start("dependency_analysis");
        self.build_dependency_graph(source_paths)?;
        let dependency_analysis_time = self.profiler.stop("dependency_analysis");
        
        // 並列コンパイルのためのタスクを作成
        let compilation_order = self.dependency_graph.get_compilation_order();
        
        // 各ソースファイルをコンパイル（並列処理）
        self.profiler.start("parallel_compilation");
        
        let modules_mutex = Arc::new(Mutex::new(Vec::new()));
        let stats_mutex = Arc::new(Mutex::new(CompileStats::default()));
        let global_symbols = self.global_symbols.clone();
        
        // 並列処理の設定
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(self.stats.thread_count)
            .build()
            .unwrap();
        
        pool.install(|| {
            compilation_order.par_iter().try_for_each(|module_name| -> Result<()> {
                let source_path = self.dependency_graph.get_source_path(module_name)
                    .ok_or_else(|| CompilerError::new(
                        ErrorKind::Internal,
                        format!("モジュール {} のソースパスが見つかりません", module_name),
                        None
                    ))?;
                
                // ソースファイルを読み込み
                let source_code = self.file_system.read_to_string(&source_path)
                    .map_err(|e| CompilerError::new(
                        ErrorKind::IO,
                        format!("ソースファイル読み込みエラー: {}", e),
                        None
                    ))?;
                
                // キャッシュをチェック
                let cache_key = self.compilation_cache.compute_key(&source_path, &self.options);
                let ir_module = if self.options.use_cache {
                    if let Some(cached_module) = self.compilation_cache.get(&cache_key) {
                        // キャッシュヒットを記録
                        let mut stats = stats_mutex.lock().unwrap();
                        stats.cache_hits += 1;
                        Arc::new(cached_module)
                    } else {
                        // コンパイルを実行
                        let module = self.compile_source_with_symbols(&source_code, module_name, global_symbols.clone())?;
                        
                        // キャッシュに保存
                        self.compilation_cache.put(&cache_key, &module);
                        
                        Arc::new(module)
                    }
                } else {
                    // コンパイルを実行
                    let module = self.compile_source_with_symbols(&source_code, module_name, global_symbols.clone())?;
                    Arc::new(module)
                };
                
                // モジュールを追加
                let mut modules = modules_mutex.lock().unwrap();
                modules.push((module_name.clone(), ir_module));
                
                // 行数を加算
                let mut stats = stats_mutex.lock().unwrap();
                stats.file_count += 1;
                stats.total_lines += source_code.lines().count();
                
                Ok(())
            })
        })?;
        
        let parallel_compilation_time = self.profiler.stop("parallel_compilation");
        
        // 並列コンパイルの結果を取得
        let modules = Arc::try_unwrap(modules_mutex)
            .expect("並列コンパイル結果の取得に失敗")
            .into_inner()
            .expect("ミューテックスのロック解除に失敗");
        
        // 統計情報を合併
        let thread_stats = Arc::try_unwrap(stats_mutex)
            .expect("並列コンパイル統計の取得に失敗")
            .into_inner()
            .expect("ミューテックスのロック解除に失敗");
        self.stats.merge(&thread_stats);
        
        // モジュールをマップに変換
        let ir_modules: HashMap<String, Arc<ir::Module>> = modules.into_iter().collect();
        
        // IRモジュールをリンク
        self.profiler.start("linking");
        let linked_module = self.link_modules(ir_modules)?;
        self.stats.linking_time = self.profiler.stop("linking");
        
        // 適切なバックエンドを選択
        let backend = backend::create_backend(self.options.target);
        
        // コード生成を実行
        self.profiler.start("codegen");
        let codegen_options = CodegenOptions {
            optimization_level: self.options.optimization_level,
            debug_info: self.options.debug_info,
            target_features: self.options.target_features.clone(),
        };
        let code = backend.generate_code(&linked_module)?;
        self.stats.codegen_time = self.profiler.stop("codegen");
        
        // 出力ファイルに書き込み
        backend.write_to_file(&code, output_path)?;
        
        // 合計時間を記録
        self.stats.total_time = self.profiler.stop("total_compilation");
        
        // 統計情報を表示（オプションが有効な場合）
        if self.options.show_stats {
            self.print_stats();
        }
        
        // プロファイル情報を出力（オプションが有効な場合）
        if self.options.enable_profiling {
            self.profiler.write_report(&self.options.profile_output);
        }
        
        // コンパイル結果を作成
        let result = CompileResult {
            success: self.diagnostic_reporter.error_count() == 0,
            stats: self.stats.clone(),
            output_files: vec![output_path.to_path_buf()],
            diagnostics: self.diagnostic_reporter.get_diagnostics(),
            ir_modules: HashMap::new(),
            binary_data: None,
            debug_info: None,
            dependency_graph: None,
            output_hash: None,
            metadata: HashMap::new(),
        };
        
        Ok(result)
    }
    
    /// 依存関係グラフを構築
    fn build_dependency_graph<P: AsRef<Path>>(&mut self, source_paths: &[P]) -> Result<()> {
        self.dependency_graph = DependencyGraph::new();
        
        for source_path in source_paths {
            let source_path = source_path.as_ref();
            
            // モジュール名を取得
            let module_name = source_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("main")
                .to_string();
            
            // ソースファイルを読み込み
            let source_code = self.file_system.read_to_string(source_path)
                .map_err(|e| CompilerError::new(
                    ErrorKind::IO,
                    format!("ソースファイル読み込みエラー: {}", e),
                    None
                ))?;
            
            // 依存関係を抽出（簡易的な実装）
            let dependencies = self.extract_dependencies(&source_code);
            
            // 依存関係グラフに追加
            self.dependency_graph.add_module(module_name, source_path.to_path_buf(), dependencies);
        }
        
        // 循環依存関係をチェック
        if let Some(cycle) = self.dependency_graph.detect_cycles() {
            return Err(CompilerError::new(
                ErrorKind::CircularDependency,
                format!("循環依存関係が検出されました: {}", cycle.join(" -> ")),
                None
            ));
        }
        
        Ok(())
    }
    
    /// ソースコードから依存関係を抽出
    fn extract_dependencies(&self, source: &str) -> HashSet<String> {
        let mut dependencies = HashSet::new();
        
        // import文を検出（簡易的な実装）
        for line in source.lines() {
            let line = line.trim();
            if line.starts_with("import ") {
                let module_name = line["import ".len()..].trim().trim_end_matches(';').to_string();
                dependencies.insert(module_name);
            }
        }
        
        dependencies
    }
    
    /// ソースコードからIRモジュールを生成（グローバルシンボルテーブル付き）
    fn compile_source_with_symbols(&self, source: &str, module_name: &str, global_symbols: Arc<Mutex<SymbolTable>>) -> Result<ir::Module> {
        let mut local_stats = CompileStats::default();
        let mut profiler = Profiler::new(self.options.enable_profiling);
        
        // 字句解析
        profiler.start("lexing");
        let file_id = self.current_file.as_ref().and_then(|p| p.to_str()).unwrap_or("unknown");
        let lexer = Lexer::new(source, file_id);
        let tokens = lexer.tokenize()?;
        local_stats.lexing_time = profiler.stop("lexing");
        
        // 構文解析
        profiler.start("parsing");
        let mut parser = Parser::new(&tokens, file_id);
        let ast = parser.parse_program()?;
        local_stats.parsing_time = profiler.stop("parsing");
        
        // 名前解決
        profiler.start("name_resolution");
        let mut name_resolver = NameResolver::new(global_symbols.clone());
        let resolved_ast = name_resolver.resolve(ast)?;
        local_stats.name_resolution_time = profiler.stop("name_resolution");
        
        // 型チェック
        profiler.start("type_checking");
        let mut type_checker = TypeChecker::new(global_symbols.clone());
        let typed_ast = type_checker.check(resolved_ast)?;
        local_stats.type_checking_time = profiler.stop("type_checking");
        
        // 意味解析（追加の検証）
        profiler.start("semantic_analysis");
        let mut analyzer = SemanticAnalyzer::new(global_symbols);
        let analyzed_ast = analyzer.analyze(typed_ast)?;
        local_stats.semantic_time = profiler.stop("semantic_analysis");
        
        // IR生成
        profiler.start("ir_generation");
        let mut ir_generator = IRGenerator::new(module_name);
        let ir_module = ir_generator.generate(analyzed_ast)?;
        local_stats.ir_gen_time = profiler.stop("ir_generation");
        
        // IR検証
        profiler.start("ir_validation");
        let mut validator = IRValidator::new();
        validator.validate(&ir_module)?;
        local_stats.ir_validation_time = profiler.stop("ir_validation");
        
        // IR最適化
        profiler.start("optimization");
        let optimized_module = self.optimize_ir(ir_module)?;
        local_stats.optimization_time = profiler.stop("optimization");
        
        // 最適化パスの時間を記録
        for (pass_name, duration) in profiler.get_pass_durations() {
            local_stats.optimization_passes.insert(pass_name, duration);
        }
        
        Ok(optimized_module)
    }
    
    /// ソースコードからIRモジュールを生成
    fn compile_source(&self, source: &str, module_name: &str) -> Result<ir::Module> {
        self.compile_source_with_symbols(source, module_name, self.global_symbols.clone())
    }
    
    /// IRを最適化
    fn optimize_ir(&self, module: ir::Module) -> Result<ir::Module> {
        let opt_level = match self.options.optimization_level {
            0 => OptimizationLevel::None,
            1 => OptimizationLevel::Basic,
            2 => OptimizationLevel::Standard,
            _ => OptimizationLevel::Aggressive,
        };
        
        // パスマネージャーを作成
        let mut pass_manager = PassManager::new(opt_level);
        
        // 最適化パスを設定
        if opt_level >= OptimizationLevel::Basic {
            pass_manager.add_pass("dead_code_elimination");
            pass_manager.add_pass("constant_folding");
        }
        
        if opt_level >= OptimizationLevel::Standard {
            pass_manager.add_pass("common_subexpression_elimination");
            pass_manager.add_pass("function_inlining");
            pass_manager.add_pass("loop_invariant_code_motion");
        }
        
        if opt_level >= OptimizationLevel::Aggressive {
            pass_manager.add_pass("aggressive_inlining");
            pass_manager.add_pass("loop_unrolling");
            pass_manager.add_pass("vectorization");
            pass_manager.add_pass("memory_to_register_promotion");
        }
        
        // カスタム最適化パスを追加
        for pass_name in &self.options.optimization_passes {
            pass_manager.add_pass(pass_name);
        }
        
        // 最適化を実行
        optimization::optimize_module(module, pass_manager)
    }
    
    /// 複数のIRモジュールをリンク
    fn link_modules(&self, modules: HashMap<String, Arc<ir::Module>>) -> Result<ir::Module> {
        if modules.is_empty() {
            return Err(CompilerError::new(
                ErrorKind::LinkError,
                "リンクするモジュールがありません".to_string(),
                None
            ));
        }
        
        if modules.len() == 1 {
            return Ok(Arc::try_unwrap(modules.into_values().next().unwrap())
                .unwrap_or_else(|arc| (*arc).clone()));
        }
        
        // リンク処理の実装
        let mut linked_module = ir::Module::new("linked_module".to_string());
        
        // 型情報をマージ
        let mut type_map: HashMap<String, usize> = HashMap::new();
        
        for (_, module) in &modules {
            for (id, type_info) in &module.types {
                let type_name = &type_info.name;
                if !type_map.contains_key(type_name) {
                    let new_id = linked_module.add_type(type_info.clone());
                    type_map.insert(type_name.clone(), new_id);
                }
            }
        }
        
        // 関数シグネチャをマージ
        let mut signature_map: HashMap<String, usize> = HashMap::new();
        
        for (_, module) in &modules {
            for (id, sig) in &module.signatures {
                // シグネチャのハッシュを計算
                let sig_hash = format!("{:?}", sig);
                if !signature_map.contains_key(&sig_hash) {
                    // 型IDを変換したシグネチャを作成
                    let mut new_sig = sig.clone();
                    new_sig.return_type_id = *type_map.get(&module.types[&sig.return_type_id].name).unwrap();
                    
                    for i in 0..new_sig.parameter_type_ids.len() {
                        let param_type_id = new_sig.parameter_type_ids[i];
                        let param_type_name = &module.types[&param_type_id].name;
                        new_sig.parameter_type_ids[i] = *type_map.get(param_type_name).unwrap();
                    }
                    
                    let new_id = linked_module.add_signature(new_sig);
                    signature_map.insert(sig_hash, new_id);
                }
            }
        }
        
        // グローバル変数をマージ
        for (_, module) in &modules {
            for (name, global) in &module.globals {
                if !linked_module.globals.contains_key(name) {
                    // 型IDを変換
                    let mut new_global = global.clone();
                    let type_name = &module.types[&global.type_id].name;
                    new_global.type_id = *type_map.get(type_name).unwrap();
                    
                    linked_module.globals.insert(name.clone(), new_global);
                }
            }
        }
        
        // 関数をマージ
        for (_, module) in &modules {
            for (id, func) in &module.functions {
                if !linked_module.functions.contains_key(&func.name) {
                    // シグネチャのハッシュを計算
                    let sig_hash = format!("{:?}", &func.signature);
                    let new_sig_id = *signature_map.get(&sig_hash).unwrap();
                    
                    // 新しい関数を作成
                    let mut new_func = ir::Function {
                        name: func.name.clone(),
                        signature: linked_module.signatures[&new_sig_id].clone(),
                        blocks: func.blocks.clone(),
                        basic_blocks: HashMap::new(),
                        is_external: func.is_external,
                    };
                    
                    // 基本ブロックを変換
                    for (block_id, block) in &func.basic_blocks {
                        let mut new_block = ir::BasicBlock {
                            instructions: Vec::with_capacity(block.instructions.len()),
                        };
                        
                        // 命令を変換
                        for instr in &block.instructions {
                            let mut new_instr = instr.clone();
                            
                            // 命令の種類に応じて型IDを変換
                            match &mut new_instr.kind {
                                ir::InstructionKind::Alloca(type_id) => {
                                    let type_name = &module.types[type_id].name;
                                    *type_id = *type_map.get(type_name).unwrap();
                                },
                                _ => {} // 他の命令タイプはそのまま
                            }
                            
                            // 変換した命令を追加
                            new_block.instructions.push(new_instr);
                        }
                        
                        // 新しいブロックを追加
                        new_func.basic_blocks.insert(*block_id, new_block);
                    }
                    
                    // 関数をリンク済みモジュールに追加
                    linked_module.functions.insert(func.name.clone(), new_func);
                }
            }
        }
        
        // 最初のモジュールのクローンを返す
        let first_module = modules.values().next().unwrap();
        Ok((*first_module).clone())
    }
    
    /// コンパイル統計情報を表示
    fn print_stats(&self) {
        println!("===== コンパイル統計 =====");
        println!("ファイル数: {}", self.stats.file_count);
        println!("合計行数: {}", self.stats.total_lines);
        println!("字句解析時間: {:?}", self.stats.lexing_time);
        println!("構文解析時間: {:?}", self.stats.parsing_time);
        println!("意味解析時間: {:?}", self.stats.semantic_time);
        println!("IR生成時間: {:?}", self.stats.ir_gen_time);
        println!("最適化時間: {:?}", self.stats.optimization_time);
        println!("コード生成時間: {:?}", self.stats.codegen_time);
        println!("合計時間: {:?}", self.stats.total_time);
    }
    
    /// コンパイル統計情報を取得
    pub fn get_stats(&self) -> &CompileStats {
        &self.stats
    }
}
