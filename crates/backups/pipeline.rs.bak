//! コンパイルパイプラインモジュール
//! 
//! コンパイル処理の各段階を管理するモジュールです。

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use std::sync::{Arc, Mutex};
use std::collections::HashMap;

use crate::parser::Parser;
use crate::lexer::Lexer;
use crate::ir;
use crate::typesystem::TypeChecker;
use crate::semantics::SemanticAnalyzer;
use crate::driver::compiler::{CompileResult, CompileStats, CompilerError, ErrorKind};
use crate::driver::diagnostics::DiagnosticReporter;

/// パイプラインステージ
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PipelineStage {
    /// 字句解析
    Lexing,
    /// 構文解析
    Parsing,
    /// マクロ展開
    MacroExpansion,
    /// 名前解決
    NameResolution,
    /// 型推論
    TypeInference,
    /// 型チェック
    TypeChecking,
    /// 定数評価
    ConstantEvaluation,
    /// 意味解析
    SemanticAnalysis,
    /// IR生成
    IRGeneration,
    /// 最適化
    Optimization,
    /// コード生成
    CodeGeneration,
    /// リンク
    Linking,
}

impl PipelineStage {
    /// ステージ名を取得
    pub fn name(&self) -> &'static str {
        match self {
            PipelineStage::Lexing => "字句解析",
            PipelineStage::Parsing => "構文解析",
            PipelineStage::MacroExpansion => "マクロ展開",
            PipelineStage::NameResolution => "名前解決",
            PipelineStage::TypeInference => "型推論",
            PipelineStage::TypeChecking => "型チェック", 
            PipelineStage::ConstantEvaluation => "定数評価",
            PipelineStage::SemanticAnalysis => "意味解析",
            PipelineStage::IRGeneration => "IR生成",
            PipelineStage::Optimization => "最適化",
            PipelineStage::CodeGeneration => "コード生成",
            PipelineStage::Linking => "リンク",
        }
    }
}

/// パイプラインステージの結果
#[derive(Debug)]
pub struct StageResult<T> {
    /// 結果データ
    pub data: T,
    /// 実行時間
    pub duration: Duration,
    /// 診断情報
    pub diagnostics: Vec<CompilerError>,
}

impl<T> StageResult<T> {
    /// 新しいステージ結果を作成
    pub fn new(data: T, duration: Duration, diagnostics: Vec<CompilerError>) -> Self {
        Self {
            data,
            duration,
            diagnostics,
        }
    }
    
    /// 別の型に変換
    pub fn map<U, F: FnOnce(T) -> U>(self, f: F) -> StageResult<U> {
        StageResult {
            data: f(self.data),
            duration: self.duration,
            diagnostics: self.diagnostics,
        }
    }
}

/// コンパイルパイプライン
pub struct CompilePipeline {
    /// 診断レポーター
    diagnostic_reporter: Arc<Mutex<DiagnosticReporter>>,
    /// 統計情報
    stats: Arc<Mutex<CompileStats>>,
    /// コンパイルオプション
    options: CompileOptions,
    /// 各ステージの実行時間
    stage_durations: HashMap<PipelineStage, Duration>,
    /// 各ステージのメモリ使用量
    stage_memory_usage: HashMap<PipelineStage, usize>,
}

/// コンパイルオプション
#[derive(Debug, Clone)]
pub struct CompileOptions {
    /// 最適化レベル（0-3）
    pub optimization_level: u8,
    /// デバッグ情報を含めるか
    pub debug_info: bool,
    /// 警告をエラーとして扱うか
    pub warnings_as_errors: bool,
    /// 診断情報の詳細レベル（0-3）
    pub verbosity: u8,
    /// クロスコンパイルターゲット
    pub target: Option<String>,
    /// 定義マクロ
    pub defines: HashMap<String, String>,
    /// インクルードパス
    pub include_paths: Vec<PathBuf>,
    /// ライブラリパス
    pub lib_paths: Vec<PathBuf>,
    /// リンクするライブラリ
    pub libs: Vec<String>,
    /// 出力形式
    pub output_type: OutputType,
    /// スレッド数
    pub thread_count: usize,
    /// キャッシュを使用するか
    pub use_cache: bool,
}

/// 出力形式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputType {
    /// 実行可能ファイル
    Executable,
    /// 共有ライブラリ
    SharedLibrary,
    /// 静的ライブラリ
    StaticLibrary,
    /// オブジェクトファイル
    ObjectFile,
    /// LLVM IR
    LLVMIR,
    /// アセンブリ
    Assembly,
}

impl CompileOptions {
    /// デフォルトのコンパイルオプションを作成
    pub fn default() -> Self {
        Self {
            optimization_level: 0,
            debug_info: false,
            warnings_as_errors: false,
            verbosity: 1,
            target: None,
            defines: HashMap::new(),
            include_paths: Vec::new(),
            lib_paths: Vec::new(),
            libs: Vec::new(),
            output_type: OutputType::Executable,
            thread_count: num_cpus::get(),
            use_cache: true,
        }
    }
}

impl CompilePipeline {
    /// 新しいコンパイルパイプラインを作成
    pub fn new(options: CompileOptions) -> Self {
        Self {
            diagnostic_reporter: Arc::new(Mutex::new(DiagnosticReporter::new(100))),
            stats: Arc::new(Mutex::new(CompileStats::new(options.thread_count))),
            options,
            stage_durations: HashMap::new(),
            stage_memory_usage: HashMap::new(),
        }
    }
    
    /// ソースファイルをコンパイル
    pub fn compile<P: AsRef<Path>>(&mut self, source_path: P) -> Result<CompileResult, CompilerError> {
        let source_path = source_path.as_ref();
        let source = std::fs::read_to_string(source_path)
            .map_err(|e| CompilerError::new(
                ErrorKind::IOError,
                format!("ファイルを読み込めませんでした: {}", e),
                Some(source_path.to_path_buf())
            ))?;
        
        self.compile_source(&source, source_path)
    }
    
    /// ソースコードをコンパイル
    pub fn compile_source<P: AsRef<Path>>(&mut self, source: &str, source_path: P) -> Result<CompileResult, CompilerError> {
        let source_path = source_path.as_ref();
        let module_name = source_path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unnamed_module")
            .to_string();
        
        // 字句解析
        let lexer_result = self.run_stage(PipelineStage::Lexing, || {
            let lexer = Lexer::new(source, source_path);
            lexer.lex()
        })?;
        
        // 構文解析
        let parser_result = self.run_stage(PipelineStage::Parsing, || {
            let parser = Parser::new(lexer_result.data, source_path);
            parser.parse()
        })?;
        
        // 名前解決と型チェック
        let semantic_result = self.run_stage(PipelineStage::SemanticAnalysis, || {
            let semantic_analyzer = SemanticAnalyzer::new();
            semantic_analyzer.analyze(parser_result.data.clone())
        })?;
        
        // IR生成
        let ir_result = self.run_stage(PipelineStage::IRGeneration, || {
            let ir_generator = ir::IRGenerator::new(&module_name);
            ir_generator.generate(semantic_result.data)
        })?;
        
        // IR最適化
        let optimized_ir = if self.options.optimization_level > 0 {
            self.run_stage(PipelineStage::Optimization, || {
                let optimizer = ir::Optimizer::new(self.options.optimization_level);
                optimizer.optimize(ir_result.data.clone())
            })?.data
        } else {
            ir_result.data.clone()
        };
        
        // 統計情報を更新
        {
            let mut stats = self.stats.lock().unwrap();
            stats.file_count += 1;
            stats.total_lines += source.lines().count();
            stats.lexing_time += lexer_result.duration;
            stats.parsing_time += parser_result.duration;
            stats.semantic_time += semantic_result.duration;
            stats.ir_gen_time += ir_result.duration;
        }
        
        // 結果を作成
        let success = !self.diagnostic_reporter.lock().unwrap().has_errors();
        let stats = self.stats.lock().unwrap().clone();
        
        let mut result = CompileResult::new(success, stats);
        result.add_ir_module(module_name, Arc::new(optimized_ir));
        
        Ok(result)
    }
    
    /// パイプラインステージを実行
    fn run_stage<T, F>(&mut self, stage: PipelineStage, f: F) -> Result<StageResult<T>, CompilerError>
    where
        F: FnOnce() -> Result<T, CompilerError>
    {
        let start = Instant::now();
        let result = f()?;
        let duration = start.elapsed();
        
        // ステージの実行時間を記録
        self.stage_durations.insert(stage, duration);
        
        // 現在のメモリ使用量を記録（実際のメモリ使用量の取得は環境によって異なる）
        // ここでは簡易的な実装
        let memory_usage = 0;  // 実際はシステムから取得する
        self.stage_memory_usage.insert(stage, memory_usage);
        
        Ok(StageResult::new(result, duration, Vec::new()))
    }
    
    /// 診断レポーターを取得
    pub fn diagnostic_reporter(&self) -> Arc<Mutex<DiagnosticReporter>> {
        self.diagnostic_reporter.clone()
    }
    
    /// 統計情報を取得
    pub fn stats(&self) -> Arc<Mutex<CompileStats>> {
        self.stats.clone()
    }
} 