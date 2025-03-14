//! # コンパイラドライバー
//! 
//! コンパイルプロセス全体を実行するドライバークラスを提供します。
//! ソースコードの解析からコード生成までのパイプラインを管理し、
//! 並列処理やインクリメンタルコンパイルをサポートします。

use std::path::{Path, PathBuf};
use std::fs;
use std::collections::{HashMap, HashSet};
use std::time::{Instant, Duration};
use std::sync::{Arc, Mutex};
use rayon::prelude::*;

use crate::frontend::{
    lexer::Lexer,
    parser::Parser,
    semantic::SemanticAnalyzer,
    ast::Program,
    error::{CompilerError, ErrorKind, Result, SourceLocation, DiagnosticReporter},
    symbol_table::SymbolTable,
    type_checker::TypeChecker,
    name_resolution::NameResolver
};
use crate::middleend::{
    ir,
    optimization::{self, OptimizationLevel, PassManager},
    ir_validator::IRValidator,
    ir_generator::IRGenerator
};
use crate::backend::{self, Target, Backend, CodegenOptions};
use crate::driver::{
    config::CompilerConfig,
    options::CompileOptions,
    dependency::DependencyGraph,
    cache::CompilationCache
};
use crate::utils::{
    profiler::Profiler,
    file_system::FileSystem,
    parallel::WorkQueue
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
    /// IR生成にかかった時間
    pub ir_gen_time: Duration,
    /// IR検証にかかった時間
    pub ir_validation_time: Duration,
    /// 最適化にかかった時間
    pub optimization_time: Duration,
    /// コード生成にかかった時間
    pub codegen_time: Duration,
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
    /// メモリ使用量（ピーク時、バイト単位）
    pub peak_memory_usage: usize,
    /// 各最適化パスの実行時間
    pub optimization_passes: HashMap<String, Duration>,
    /// 並列処理で使用したスレッド数
    pub thread_count: usize,
}

impl CompileStats {
    /// 別の統計情報を合併する
    pub fn merge(&mut self, other: &CompileStats) {
        self.lexing_time += other.lexing_time;
        self.parsing_time += other.parsing_time;
        self.name_resolution_time += other.name_resolution_time;
        self.type_checking_time += other.type_checking_time;
        self.semantic_time += other.semantic_time;
        self.ir_gen_time += other.ir_gen_time;
        self.ir_validation_time += other.ir_validation_time;
        self.optimization_time += other.optimization_time;
        self.codegen_time += other.codegen_time;
        self.linking_time += other.linking_time;
        self.file_count += other.file_count;
        self.total_lines += other.total_lines;
        self.error_count += other.error_count;
        self.warning_count += other.warning_count;
        self.cache_hits += other.cache_hits;
        self.peak_memory_usage = self.peak_memory_usage.max(other.peak_memory_usage);
        
        // 最適化パスの時間を合併
        for (pass_name, duration) in &other.optimization_passes {
            *self.optimization_passes.entry(pass_name.clone()).or_insert(Duration::default()) += *duration;
        }
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
    compiled_modules: HashMap<String, Arc<ir::Module>>,
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
    global_symbols: Arc<Mutex<SymbolTable>>,
}

impl Driver {
    /// 新しいドライバーを作成
    pub fn new(options: CompileOptions) -> Self {
        let config = CompilerConfig::from_options(&options);
        let thread_count = options.thread_count.unwrap_or_else(num_cpus::get);
        
        Self {
            config,
            options: options.clone(),
            stats: CompileStats {
                thread_count,
                ..CompileStats::default()
            },
            current_file: None,
            compiled_modules: HashMap::new(),
            dependency_graph: DependencyGraph::new(),
            compilation_cache: CompilationCache::new(options.cache_dir.clone()),
            diagnostic_reporter: DiagnosticReporter::new(options.error_format, options.color_output),
            profiler: Profiler::new(options.enable_profiling),
            file_system: FileSystem::new(),
            work_queue: WorkQueue::new(thread_count),
            global_symbols: Arc::new(Mutex::new(SymbolTable::new_global())),
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
        let backend = backend::create_backend(self.options.target);
        
        // コード生成を実行
        self.profiler.start("codegen");
        let codegen_options = CodegenOptions {
            optimization_level: self.options.optimization_level,
            debug_info: self.options.debug_info,
            target_features: self.options.target_features.clone(),
        };
        let code = backend.generate_code(&ir_module, &codegen_options)?;
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
        let code = backend.generate_code(&linked_module, &codegen_options)?;
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
        Ok(modules[0].clone())
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
