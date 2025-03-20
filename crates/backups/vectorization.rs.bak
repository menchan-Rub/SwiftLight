//! # ベクトル化モジュール
//!
//! ループと演算のベクトル化を扱うモジュールです。
//! このモジュールは、SwiftLightコンパイラのバックエンドで使用され、
//! 高度なSIMD命令を活用した自動ベクトル化を実現します。

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use crate::frontend::error::{Result, CompilerError};
use crate::middleend::ir::representation::{Module, Function, BasicBlock, Instruction, Value, Type, Loop};
use crate::middleend::analysis::{DataFlowAnalysis, DependenceAnalysis, LoopAnalysis};
use crate::backend::analysis::{AnalysisManager, AnalysisKey};
use crate::backend::target::{TargetInfo, TargetFeature};
use crate::backend::codegen::InstructionSelection;
use crate::utils::logger::{Logger, LogLevel};

/// ベクトル化戦略
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VectorizationStrategy {
    /// ベクトル化なし
    None,
    /// 安全なベクトル化のみ
    Safe,
    /// 標準的なベクトル化
    Standard,
    /// 積極的なベクトル化
    Aggressive,
    /// 自動検出
    Auto,
}

/// SIMD命令セット
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SIMDInstructionSet {
    /// Intel SSE
    SSE,
    /// Intel SSE2
    SSE2,
    /// Intel SSE3
    SSE3,
    /// Intel SSSE3
    SSSE3,
    /// Intel SSE4.1
    SSE4_1,
    /// Intel SSE4.2
    SSE4_2,
    /// Intel AVX
    AVX,
    /// Intel AVX2
    AVX2,
    /// Intel AVX-512
    AVX512,
    /// ARM NEON
    NEON,
    /// RISC-V V extension
    RVECTOREXT,
    /// WebAssembly SIMD
    WASMSIMD,
}

impl SIMDInstructionSet {
    /// 命令セットのビット幅を取得
    pub fn bit_width(&self) -> u32 {
        match self {
            Self::SSE | Self::SSE2 | Self::SSE3 | Self::SSSE3 | 
            Self::SSE4_1 | Self::SSE4_2 => 128,
            Self::AVX | Self::AVX2 => 256,
            Self::AVX512 => 512,
            Self::NEON => 128,
            Self::RVECTOREXT => 128, // 可変長だが基本は128bit
            Self::WASMSIMD => 128,
        }
    }
    
    /// 命令セットが特定のターゲットでサポートされているか確認
    pub fn is_supported_by_target(&self, target_info: &TargetInfo) -> bool {
        match self {
            Self::SSE => target_info.has_feature(TargetFeature::SSE),
            Self::SSE2 => target_info.has_feature(TargetFeature::SSE2),
            Self::SSE3 => target_info.has_feature(TargetFeature::SSE3),
            Self::SSSE3 => target_info.has_feature(TargetFeature::SSSE3),
            Self::SSE4_1 => target_info.has_feature(TargetFeature::SSE4_1),
            Self::SSE4_2 => target_info.has_feature(TargetFeature::SSE4_2),
            Self::AVX => target_info.has_feature(TargetFeature::AVX),
            Self::AVX2 => target_info.has_feature(TargetFeature::AVX2),
            Self::AVX512 => target_info.has_feature(TargetFeature::AVX512F),
            Self::NEON => target_info.has_feature(TargetFeature::NEON),
            Self::RVECTOREXT => target_info.has_feature(TargetFeature::RVV),
            Self::WASMSIMD => target_info.has_feature(TargetFeature::WASMSIMD),
        }
    }
    
    /// 命令セットの優先度（高いほど新しく高性能）
    pub fn priority(&self) -> u32 {
        match self {
            Self::SSE => 10,
            Self::SSE2 => 20,
            Self::SSE3 => 30,
            Self::SSSE3 => 40,
            Self::SSE4_1 => 50,
            Self::SSE4_2 => 60,
            Self::AVX => 70,
            Self::AVX2 => 80,
            Self::AVX512 => 90,
            Self::NEON => 75,
            Self::RVECTOREXT => 85,
            Self::WASMSIMD => 65,
        }
    }
}

/// ベクトル化変換の種類
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VectorizationTransform {
    /// ループ内のロード・ストアのベクトル化
    LoadStoreVectorization,
    /// 算術演算のベクトル化
    ArithmeticVectorization,
    /// 論理演算のベクトル化
    LogicalVectorization,
    /// 比較演算のベクトル化
    ComparisonVectorization,
    /// Reduction変数のベクトル化
    ReductionVectorization,
    /// ループ間の依存関係の無視（安全でない場合がある）
    IgnorePossibleDependencies,
    /// スカラー拡張（scalar expansion）
    ScalarExpansion,
    /// If文の変換（マスク処理）
    BranchToMask,
    /// ループアンロール後のベクトル化
    UnrollAndVectorize,
    /// ループ分割後のベクトル化
    SplitAndVectorize,
    /// ループ入れ替え後のベクトル化
    InterchangeAndVectorize,
    /// ループ分布後のベクトル化
    DistributeAndVectorize,
    /// ループ融合後のベクトル化
    FuseAndVectorize,
    /// ループタイリング後のベクトル化
    TileAndVectorize,
    /// ギャザー/スキャッター操作の使用
    UseGatherScatter,
    /// 数学関数のベクトル化
    MathFunctionVectorization,
}

/// ベクトル化オプション
#[derive(Debug, Clone)]
pub struct VectorizationOptions {
    /// ベクトル化戦略
    pub strategy: VectorizationStrategy,
    /// 使用するSIMD命令セット
    pub instruction_sets: Vec<SIMDInstructionSet>,
    /// ベクトル幅（ビット）
    pub vector_width: u32,
    /// 最小ループ繰り返し回数（この回数以上でベクトル化を検討）
    pub min_loop_iterations: u32,
    /// ベクトル化中に許可する変換
    pub allowed_transforms: Vec<VectorizationTransform>,
    /// コスト閾値（この値以上の利益が見込める場合のみベクトル化）
    pub cost_threshold: f64,
    /// 最大ベクトル長（要素数）
    pub max_vector_length: u32,
    /// ベクトル化前の最適化パス
    pub pre_vectorization_passes: Vec<String>,
    /// ベクトル化後の最適化パス
    pub post_vectorization_passes: Vec<String>,
    /// ベクトル化のデバッグ情報出力
    pub debug_level: u32,
    /// 自動ベクトル化ヒントを使用するか
    pub use_vectorization_hints: bool,
    /// 浮動小数点の精度要件を緩和するか
    pub relax_fp_precision: bool,
    /// 数学関数の近似を使用するか
    pub use_approx_math: bool,
    /// ターゲット固有の最適化を行うか
    pub target_specific_optimizations: bool,
}

impl Default for VectorizationOptions {
    fn default() -> Self {
        Self {
            strategy: VectorizationStrategy::Auto,
            instruction_sets: vec![
                SIMDInstructionSet::SSE2,
                SIMDInstructionSet::AVX2,
                SIMDInstructionSet::NEON,
            ],
            vector_width: 256,
            min_loop_iterations: 4,
            allowed_transforms: vec![
                VectorizationTransform::LoadStoreVectorization,
                VectorizationTransform::ArithmeticVectorization,
                VectorizationTransform::LogicalVectorization,
                VectorizationTransform::ComparisonVectorization,
                VectorizationTransform::ReductionVectorization,
            ],
            cost_threshold: 1.5,
            max_vector_length: 32,
            pre_vectorization_passes: vec![
                "loop-simplify".to_string(),
                "loop-rotate".to_string(),
                "licm".to_string(),
                "indvar-simplify".to_string(),
            ],
            post_vectorization_passes: vec![
                "instcombine".to_string(),
                "dce".to_string(),
            ],
            debug_level: 0,
            use_vectorization_hints: true,
            relax_fp_precision: false,
            use_approx_math: false,
            target_specific_optimizations: true,
        }
    }
}

/// ベクトル化の依存関係の種類
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DependenceType {
    /// 依存関係なし
    None,
    /// RAW (Read After Write) 依存
    ReadAfterWrite,
    /// WAR (Write After Read) 依存
    WriteAfterRead,
    /// WAW (Write After Write) 依存
    WriteAfterWrite,
    /// 制御依存
    Control,
    /// メモリ依存（解析不能）
    Memory,
}

/// ベクトル化の候補
#[derive(Debug, Clone)]
pub struct VectorizationCandidate {
    /// 候補ID
    pub id: String,
    /// 候補の種類（ループまたは関数）
    pub kind: VectorizationCandidateKind,
    /// 予測されるスピードアップ
    pub estimated_speedup: f64,
    /// 使用する命令セット
    pub instruction_set: SIMDInstructionSet,
    /// ベクトル幅
    pub vector_width: u32,
    /// 適用する変換
    pub transforms: Vec<VectorizationTransform>,
    /// 依存関係
    pub dependencies: Vec<(String, DependenceType)>,
    /// ベクトル化コスト
    pub cost: VectorizationCost,
}

/// ベクトル化候補の種類
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VectorizationCandidateKind {
    /// ループベクトル化
    Loop(String),
    /// 関数ベクトル化
    Function(String),
    /// 基本ブロックベクトル化
    BasicBlock(String),
    /// 命令列ベクトル化
    InstructionSequence(Vec<String>),
}

/// ベクトル化コスト
#[derive(Debug, Clone)]
pub struct VectorizationCost {
    /// スカラー実行時の命令数
    pub scalar_instructions: u32,
    /// ベクトル実行時の命令数
    pub vector_instructions: u32,
    /// スカラー実行時の推定サイクル数
    pub scalar_cycles: u32,
    /// ベクトル実行時の推定サイクル数
    pub vector_cycles: u32,
    /// コード増加サイズ（バイト）
    pub code_size_increase: i32,
    /// メモリアクセスパターン効率（0.0-1.0）
    pub memory_access_efficiency: f64,
}

/// ベクトル化解析結果
#[derive(Debug)]
pub struct VectorizationInfo {
    /// ベクトル化可能なループのID
    pub vectorizable_loops: Vec<String>,
    /// ベクトル化可能な関数のID
    pub vectorizable_functions: Vec<String>,
    /// ベクトル化可能な基本ブロックのID
    pub vectorizable_basic_blocks: Vec<String>,
    /// ベクトル化できない理由（ID -> 理由）
    pub non_vectorizable_reasons: HashMap<String, String>,
    /// ベクトル化候補
    pub candidates: Vec<VectorizationCandidate>,
    /// 選択された候補
    pub selected_candidates: Vec<String>,
    /// ベクトル化統計情報
    pub statistics: VectorizationStatistics,
}

/// ベクトル化統計情報
#[derive(Debug, Default, Clone)]
pub struct VectorizationStatistics {
    /// 解析されたループ数
    pub analyzed_loops: u32,
    /// 解析された関数数
    pub analyzed_functions: u32,
    /// ベクトル化されたループ数
    pub vectorized_loops: u32,
    /// ベクトル化された関数数
    pub vectorized_functions: u32,
    /// ベクトル化された基本ブロック数
    pub vectorized_basic_blocks: u32,
    /// 推定総スピードアップ
    pub estimated_total_speedup: f64,
    /// 変換ごとの適用回数
    pub transform_counts: HashMap<VectorizationTransform, u32>,
    /// 命令セットごとの使用回数
    pub instruction_set_usage: HashMap<SIMDInstructionSet, u32>,
}

/// ベクトル化処理
pub struct Vectorizer {
    /// ベクトル化オプション
    pub options: VectorizationOptions,
    /// 解析マネージャ
    analysis_manager: Option<AnalysisManager>,
    /// ターゲット情報
    target_info: Option<Arc<TargetInfo>>,
    /// ロガー
    logger: Option<Arc<Logger>>,
    /// 命令選択
    instruction_selector: Option<Arc<InstructionSelection>>,
    /// ベクトル化統計情報
    statistics: VectorizationStatistics,
    /// ベクトル化候補
    candidates: Vec<VectorizationCandidate>,
    /// 選択された候補
    selected_candidates: HashSet<String>,
}

impl Vectorizer {
    /// 新しいベクトル化処理を作成
    pub fn new(options: VectorizationOptions) -> Self {
        Self {
            options,
            analysis_manager: None,
            target_info: None,
            logger: None,
            instruction_selector: None,
            statistics: VectorizationStatistics::default(),
            candidates: Vec::new(),
            selected_candidates: HashSet::new(),
        }
    }
    
    /// 解析マネージャを設定
    pub fn set_analysis_manager(&mut self, analysis_manager: AnalysisManager) {
        self.analysis_manager = Some(analysis_manager);
    }
    
    /// ターゲット情報を設定
    pub fn set_target_info(&mut self, target_info: Arc<TargetInfo>) {
        self.target_info = Some(target_info);
    }
    
    /// ロガーを設定
    pub fn set_logger(&mut self, logger: Arc<Logger>) {
        self.logger = Some(logger);
    }
    
    /// 命令選択を設定
    pub fn set_instruction_selector(&mut self, instruction_selector: Arc<InstructionSelection>) {
        self.instruction_selector = Some(instruction_selector);
    }
    
    /// ログ出力
    fn log(&self, level: LogLevel, message: &str) {
        if let Some(logger) = &self.logger {
            logger.log(level, &format!("[Vectorizer] {}", message));
        }
    }
    
    /// 最適な命令セットを選択
    fn select_best_instruction_set(&self) -> Option<SIMDInstructionSet> {
        if let Some(target_info) = &self.target_info {
            // ターゲットがサポートする命令セットのみをフィルタリング
            let supported_sets: Vec<_> = self.options.instruction_sets.iter()
                .filter(|&set| set.is_supported_by_target(target_info))
                .collect();
            
            // 優先度順にソート
            if !supported_sets.is_empty() {
                return supported_sets.into_iter()
                    .max_by_key(|set| set.priority())
                    .copied();
            }
        }
        
        // ターゲット情報がない場合はデフォルトを返す
        self.options.instruction_sets.first().copied()
    }
    
    /// ベクトル化解析を実行
    pub fn analyze(&mut self, module: &Module) -> Result<VectorizationInfo> {
        self.log(LogLevel::Info, &format!("ベクトル化解析を開始: 戦略={:?}", self.options.strategy));
        
        // 解析マネージャが設定されていない場合はエラー
        let analysis_manager = self.analysis_manager.as_ref()
            .ok_or_else(|| CompileError::new(
                ErrorKind::InternalError,
                "ベクトル化の前に解析マネージャを設定してください".to_string()
            ))?;
        
        // 統計情報をリセット
        self.statistics = VectorizationStatistics::default();
        self.candidates.clear();
        self.selected_candidates.clear();
        
        // ベクトル化可能性の解析
        let mut vectorizable_loops = Vec::new();
        let mut vectorizable_functions = Vec::new();
        let mut vectorizable_basic_blocks = Vec::new();
        let mut non_vectorizable_reasons = HashMap::new();
        
        // 各関数に対する解析
        for function in &module.functions {
            self.analyze_function(
                &function, 
                module,
                analysis_manager,
                &mut vectorizable_loops,
                &mut vectorizable_functions,
                &mut vectorizable_basic_blocks,
                &mut non_vectorizable_reasons
            )?;
        }
        
        // 候補の選択
        self.select_candidates();
        
        // 選択された候補IDを取得
        let selected_candidates = self.selected_candidates.iter()
            .cloned()
            .collect();
        
        // 統計情報を更新
        self.statistics.analyzed_loops = vectorizable_loops.len() as u32 + 
            non_vectorizable_reasons.len() as u32;
        self.statistics.analyzed_functions = module.functions.len() as u32;
        
        self.log(LogLevel::Info, &format!(
            "ベクトル化解析完了: ループ={}/{}, 関数={}/{}, 基本ブロック={}",
            vectorizable_loops.len(),
            self.statistics.analyzed_loops,
            vectorizable_functions.len(),
            self.statistics.analyzed_functions,
            vectorizable_basic_blocks.len()
        ));
        
        Ok(VectorizationInfo {
            vectorizable_loops,
            vectorizable_functions,
            vectorizable_basic_blocks,
            non_vectorizable_reasons,
            candidates: self.candidates.clone(),
            selected_candidates,
            statistics: self.statistics.clone(),
        })
    }
    
    /// 関数内のループと命令を解析
    fn analyze_function(
        &mut self,
        function: &Function,
        module: &Module,
        analysis_manager: &AnalysisManager,
        vectorizable_loops: &mut Vec<String>,
        vectorizable_functions: &mut Vec<String>,
        vectorizable_basic_blocks: &mut Vec<String>,
        non_vectorizable_reasons: &mut HashMap<String, String>
    ) -> Result<()> {
        self.log(LogLevel::Debug, &format!("関数の解析: {}", function.name));
        
        // ループ情報の取得
        let loop_info = analysis_manager.get_analysis::<LoopInfo>(function)?;
        
        // 依存関係解析の取得
        let dependence_analysis = analysis_manager.get_analysis::<DependenceAnalysis>(function)?;
        
        // エイリアス解析の取得
        let alias_analysis = analysis_manager.get_analysis::<AliasAnalysis>(function)?;
        
        // 各ループに対する解析
        for loop_data in &loop_info.loops {
            self.analyze_loop(
                loop_data,
                function,
                module,
                dependence_analysis,
                alias_analysis,
                vectorizable_loops,
                non_vectorizable_reasons
            )?;
        }
        
        // 各基本ブロックに対する解析（ループ外の命令列）
        for (bb_idx, bb) in function.basic_blocks.iter().enumerate() {
            // このブロックがループ内にない場合のみ解析
            if !loop_info.is_in_loop(bb_idx) {
                self.analyze_basic_block(
                    bb,
                    bb_idx,
                    function,
                    module,
                    alias_analysis,
                    vectorizable_basic_blocks
                )?;
            }
        }
        
        // 関数全体がベクトル化可能かチェック
        if self.is_vectorizable_function(function, module, alias_analysis)? {
            vectorizable_functions.push(function.name.clone());
            
            // 関数ベクトル化の候補を追加
            let instruction_set = self.select_best_instruction_set()
                .ok_or_else(|| CompileError::new(
                    ErrorKind::InternalError,
                    "サポートされているSIMD命令セットがありません".to_string()
                ))?;
            
            let candidate = VectorizationCandidate {
                id: format!("func:{}", function.name),
                kind: VectorizationCandidateKind::Function(function.name.clone()),
                estimated_speedup: self.estimate_function_speedup(function, instruction_set),
                instruction_set,
                vector_width: instruction_set.bit_width(),
                transforms: vec![
                    VectorizationTransform::ArithmeticVectorization,
                    VectorizationTransform::LogicalVectorization
                ],
                dependencies: Vec::new(),
                cost: self.calculate_function_vectorization_cost(function, instruction_set),
            };
            
            self.candidates.push(candidate);
        }
        
        Ok(())
    }
    
    /// ループを解析
    fn analyze_loop(
        &mut self,
        loop_data: &Loop,
        function: &Function,
        module: &Module,
        dependence_analysis: &DependenceAnalysis,
        alias_analysis: &AliasAnalysis,
        vectorizable_loops: &mut Vec<String>,
        non_vectorizable_reasons: &mut HashMap<String, String>
    ) -> Result<()> {
        let loop_id = format!("loop:{}", loop_data.id);
        self.log(LogLevel::Debug, &format!("ループの解析: {}", loop_id));
        
        // ループがベクトル化可能かチェック
        let (vectorizable, reason) = self.check_loop_vectorizability(
            loop_data, 
            function, 
            module,
            dependence_analysis,
            alias_analysis
        )?;
        
        if vectorizable {
            vectorizable_loops.push(loop_id.clone());
            
            // ループベクトル化の候補を追加
            let instruction_set = self.select_best_instruction_set()
                .ok_or_else(|| CompileError::new(
                    ErrorKind::InternalError,
                    "サポートされているSIMD命令セットがありません".to_string()
                ))?;
            
            // 適用可能な変換を決定
            let transforms = self.determine_loop_transforms(loop_data, function, dependence_analysis);
            
            // 依存関係を収集
            let dependencies = self.collect_loop_dependencies(loop_data, function, dependence_analysis);
            
            let candidate = VectorizationCandidate {
                id: loop_id,
                kind: VectorizationCandidateKind::Loop(loop_data.id.to_string()),
                estimated_speedup: self.estimate_loop_speedup(loop_data, function, instruction_set, &transforms),
                instruction_set,
                vector_width: instruction_set.bit_width(),
                transforms,
                dependencies,
                cost: self.calculate_loop_vectorization_cost(loop_data, function, instruction_set),
            };
            
            self.candidates.push(candidate);
        } else if let Some(reason_str) = reason {
            non_vectorizable_reasons.insert(loop_id, reason_str);
        }
        
        // 内側ループも再帰的に解析
        for inner_loop in &loop_data.inner_loops {
            self.analyze_loop(
                inner_loop,
                function,
                module,
                dependence_analysis,
                alias_analysis,
                vectorizable_loops,
                non_vectorizable_reasons
            )?;
        }
        
        Ok(())
    }
    
    /// 基本ブロックを解析（ループ外の命令列のベクトル化）
    fn analyze_basic_block(
        &mut self,
        bb: &BasicBlock,
        bb_idx: usize,
        function: &Function,
        module: &Module,
        alias_analysis: &AliasAnalysis,
        vectorizable_basic_blocks: &mut Vec<String>
    ) -> Result<()> {
        let bb_id = format!("bb:{}", bb_idx);
        
        // 連続した算術/論理演算のシーケンスを探す
        let sequences = self.find_vectorizable_sequences(bb, function);
        
        if !sequences.is_empty() {
            vectorizable_basic_blocks.push(bb_id.clone());
            
            // 各シーケンスに対して候補を作成
            for (seq_idx, seq) in sequences.iter().enumerate() {
                let instruction_set = self.select_best_instruction_set()
                    .ok_or_else(|| CompileError::new(
                        ErrorKind::InternalError,
                        "サポートされているSIMD命令セットがありません".to_string()
                    ))?;
                
                let seq_id = format!("{}:seq{}", bb_id, seq_idx);
                let instr_ids = seq.iter().map(|&idx| format!("instr:{}", idx)).collect();
                
                let candidate = VectorizationCandidate {
                    id: seq_id.clone(),
                    kind: VectorizationCandidateKind::InstructionSequence(instr_ids),
                    estimated_speedup: self.estimate_sequence_speedup(seq, bb, function, instruction_set),
                    instruction_set,
                    vector_width: instruction_set.bit_width(),
                    transforms: vec![
                        VectorizationTransform::ArithmeticVectorization,
                        VectorizationTransform::LogicalVectorization
                    ],
                    dependencies: Vec::new(),
                    cost: self.calculate_sequence_vectorization_cost(seq, bb, function, instruction_set),
                };
                
                self.candidates.push(candidate);
            }
        }
        
        Ok(())
    }
    
    /// ベクトル化可能な命令シーケンスを探す
    fn find_vectorizable_sequences(&self, bb: &BasicBlock, function: &Function) -> Vec<Vec<usize>> {
        let mut sequences = Vec::new();
        let mut current_sequence = Vec::new();
        let mut current_type: Option<Type> = None;
        
        for (idx, instr) in bb.instructions.iter().enumerate() {
            // 命令がベクトル化可能か判断
            if self.is_vectorizable_instruction(instr, function) {
                let instr_type = self.get_instruction_data_type(instr, function);
                
                // 同じ型の命令が続いているか確認
                match current_type {
                    Some(curr_type) if curr_type == instr_type => {
                        current_sequence.push(idx);
                    },
                    _ => {
                        if current_sequence.len() >= self.min_vector_length {
                            sequences.push(current_sequence);
                        }
                        current_sequence = vec![idx];
                        current_type = Some(instr_type);
                    }
                }
            } else {
                // 非ベクトル化可能命令でシーケンス終了
                if current_sequence.len() >= self.min_vector_length {
                    sequences.push(current_sequence);
                }
                current_sequence = Vec::new();
                current_type = None;
            }
        }
        
        // 最後のシーケンスをチェック
        if current_sequence.len() >= self.min_vector_length {
            sequences.push(current_sequence);
        }
        
        sequences
    }

    /// ベクトル化を実行
    pub fn vectorize(&mut self, module: &mut Module) -> Result<usize> {
        let analysis_result = self.analyze(module)?;
        let mut transformed_count = 0;
        
        // ループベクトル化
        for loop_info in &analysis_result.vectorizable_loops {
            if self.vectorize_loop(module, &loop_info.id)? {
                transformed_count += 1;
                self.log_vectorization(&loop_info.id, "loop", loop_info.vector_width)?;
            }
        }
        
        // 関数ベクトル化
        for func_info in &analysis_result.vectorizable_functions {
            if self.vectorize_function(module, &func_info.id)? {
                transformed_count += 1;
                self.log_vectorization(&func_info.id, "function", func_info.vector_width)?;
            }
        }
        
        // 基本ブロックベクトル化
        for bb_info in &analysis_result.vectorizable_basic_blocks {
            if self.vectorize_basic_block(module, &bb_info.id)? {
                transformed_count += 1;
                self.log_vectorization(&bb_info.id, "basic_block", bb_info.vector_width)?;
            }
        }
        
        Ok(transformed_count)
    }
    
    /// ループをベクトル化
    fn vectorize_loop(&mut self, module: &mut Module, loop_id: &str) -> Result<bool> {
        let loop_info = module.get_loop(loop_id)
            .ok_or_else(|| CompileError::new(ErrorKind::NotFound, format!("Loop {} not found", loop_id)))?;
        
        // 依存関係解析
        let dep_analysis = DependenceAnalysis::analyze(&loop_info.body, module)?;
        if dep_analysis.has_loop_carried_dependence() {
            return Err(CompileError::new(ErrorKind::OptimizationFailed, 
                format!("Loop {} has loop-carried dependencies", loop_id)));
        }
        
        // ベクトル化係数決定
        let vector_width = self.target_info.simd_width(loop_info.element_type)?;
        let vectorized = self.transform_loop(module, loop_info, vector_width)?;
        
        Ok(vectorized)
    }
    
    /// 関数をベクトル化
    fn vectorize_function(&mut self, module: &mut Module, function_id: &str) -> Result<bool> {
        let function = module.get_function_mut(function_id)
            .ok_or_else(|| CompileError::new(ErrorKind::NotFound, format!("Function {} not found", function_id)))?;
        
        // 関数全体のベクトル化可能命令を探索
        let mut transformed = false;
        for bb in &mut function.basic_blocks {
            let sequences = self.find_vectorizable_sequences(bb, function);
            if !sequences.is_empty() {
                self.apply_vectorization_transforms(bb, &sequences)?;
                transformed = true;
            }
        }
        
        Ok(transformed)
    }

    /// 基本ブロックのベクトル化
    fn vectorize_basic_block(&mut self, module: &mut Module, bb_id: &str) -> Result<bool> {
        let (function, bb_idx) = module.find_basic_block(bb_id)
            .ok_or_else(|| CompileError::new(ErrorKind::NotFound, format!("Basic block {} not found", bb_id)))?;
        
        let bb = &mut function.basic_blocks[bb_idx];
        let sequences = self.find_vectorizable_sequences(bb, function);
        
        if sequences.is_empty() {
            return Ok(false);
        }
        
        self.apply_vectorization_transforms(bb, &sequences)?;
        Ok(true)
    }
}