//! # イントリンシックモジュール
//!
//! コンパイラの組み込み関数を定義するモジュールです。
//! これらの関数は、特定のハードウェア命令や最適化されたルーチンに直接マップされます。

use std::collections::HashMap;
use crate::middleend::ir::{Type, Value};
use crate::frontend::ast::Function;
use crate::frontend::error::Result;

/// イントリンシック関数の種類
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IntrinsicKind {
    // メモリ操作
    /// メモリコピー
    MemCpy,
    /// メモリ移動
    MemMove,
    /// メモリセット
    MemSet,
    
    // 数学関数
    /// 正弦
    Sin,
    /// 余弦
    Cos,
    /// 平方根
    Sqrt,
    /// 立方根
    Cbrt,
    /// 指数関数
    Exp,
    /// 自然対数
    Ln,
    /// 常用対数
    Log10,
    /// 累乗
    Pow,
    
    // ビット操作
    /// ビットカウント（1ビットの数を数える）
    PopCount,
    /// 先行ゼロカウント
    LeadingZeros,
    /// 末尾ゼロカウント
    TrailingZeros,
    /// ビット反転
    BitReverse,
    /// バイト反転
    ByteSwap,
    
    // SIMD
    /// SIMD要素追加
    SimdAdd,
    /// SIMD要素乗算
    SimdMul,
    /// SIMDシャッフル
    SimdShuffle,
    /// SIMD比較
    SimdCompare,
    
    // アトミック操作
    /// アトミック読み込み
    AtomicLoad,
    /// アトミック書き込み
    AtomicStore,
    /// アトミック交換
    AtomicExchange,
    /// アトミック比較交換
    AtomicCompareExchange,
    /// アトミック加算
    AtomicAdd,
    /// アトミックAND
    AtomicAnd,
    /// アトミックOR
    AtomicOr,
    /// アトミックXOR
    AtomicXor,
    
    // その他
    /// トラップ（実行時エラー）
    Trap,
    /// デバッグ中断ポイント
    DebugBreak,
    /// コンパイラフェンス
    Fence,
}

/// イントリンシック関数の属性
#[derive(Debug, Clone)]
pub struct IntrinsicAttributes {
    /// 読み取り専用かどうか（副作用がないか）
    pub readonly: bool,
    /// 参照透過性があるか（同じ引数で常に同じ結果を返すか）
    pub referentially_transparent: bool,
    /// メモリにアクセスするか
    pub memory_access: bool,
    /// スレッドセーフか
    pub thread_safe: bool,
    /// どのターゲットで利用可能か
    pub available_targets: Vec<String>,
}

/// イントリンシック関数マネージャ
pub struct IntrinsicManager {
    /// 利用可能なイントリンシック関数のマップ
    intrinsics: HashMap<IntrinsicKind, IntrinsicFunction>,
}

/// イントリンシック関数の定義
pub struct IntrinsicFunction {
    /// イントリンシックの種類
    pub kind: IntrinsicKind,
    /// 関数名
    pub name: String,
    /// 引数の型リスト
    pub parameter_types: Vec<Type>,
    /// 戻り値の型
    pub return_type: Type,
    /// イントリンシック属性
    pub attributes: IntrinsicAttributes,
}

impl IntrinsicManager {
    /// 新しいイントリンシックマネージャを作成
    pub fn new() -> Self {
        let mut manager = Self {
            intrinsics: HashMap::new(),
        };
        manager.register_default_intrinsics();
        manager
    }
    
    /// デフォルトのイントリンシック関数を登録
    fn register_default_intrinsics(&mut self) {
        // メモリ操作系イントリンシック
        self.register(IntrinsicFunction {
            kind: IntrinsicKind::MemCopy,
            name: "swiftlight_memcopy".to_string(),
            parameter_types: vec![
                Type::Pointer(Box::new(Type::Void), false), // 宛先ポインタ
                Type::Pointer(Box::new(Type::Void), true),  // 元ポインタ（読み取り専用）
                Type::Integer(64, false),                   // サイズ（バイト単位）
                Type::Boolean,                              // 揮発性フラグ
            ],
            return_type: Type::Void,
            attributes: IntrinsicAttributes {
                readonly: false,
                referentially_transparent: false,
                memory_access: true,
                thread_safe: true,
                available_targets: vec!["all".to_string()],
            },
        });

        self.register(IntrinsicFunction {
            kind: IntrinsicKind::MemSet,
            name: "swiftlight_memset".to_string(),
            parameter_types: vec![
                Type::Pointer(Box::new(Type::Void), false), // 宛先ポインタ
                Type::Integer(8, false),                    // 設定値
                Type::Integer(64, false),                   // サイズ（バイト単位）
                Type::Boolean,                              // 揮発性フラグ
            ],
            return_type: Type::Void,
            attributes: IntrinsicAttributes {
                readonly: false,
                referentially_transparent: false,
                memory_access: true,
                thread_safe: true,
                available_targets: vec!["all".to_string()],
            },
        });

        // 数学系イントリンシック
        self.register(IntrinsicFunction {
            kind: IntrinsicKind::Sqrt,
            name: "swiftlight_sqrt".to_string(),
            parameter_types: vec![Type::Float(64)],
            return_type: Type::Float(64),
            attributes: IntrinsicAttributes {
                readonly: true,
                referentially_transparent: true,
                memory_access: false,
                thread_safe: true,
                available_targets: vec!["all".to_string()],
            },
        });

        self.register(IntrinsicFunction {
            kind: IntrinsicKind::Sin,
            name: "swiftlight_sin".to_string(),
            parameter_types: vec![Type::Float(64)],
            return_type: Type::Float(64),
            attributes: IntrinsicAttributes {
                readonly: true,
                referentially_transparent: true,
                memory_access: false,
                thread_safe: true,
                available_targets: vec!["all".to_string()],
            },
        });

        self.register(IntrinsicFunction {
            kind: IntrinsicKind::Cos,
            name: "swiftlight_cos".to_string(),
            parameter_types: vec![Type::Float(64)],
            return_type: Type::Float(64),
            attributes: IntrinsicAttributes {
                readonly: true,
                referentially_transparent: true,
                memory_access: false,
                thread_safe: true,
                available_targets: vec!["all".to_string()],
            },
        });

        // ビット操作系イントリンシック
        self.register(IntrinsicFunction {
            kind: IntrinsicKind::BitCount,
            name: "swiftlight_bitcount".to_string(),
            parameter_types: vec![Type::Integer(64, false)],
            return_type: Type::Integer(32, false),
            attributes: IntrinsicAttributes {
                readonly: true,
                referentially_transparent: true,
                memory_access: false,
                thread_safe: true,
                available_targets: vec!["all".to_string()],
            },
        });

        self.register(IntrinsicFunction {
            kind: IntrinsicKind::BitReverse,
            name: "swiftlight_bitreverse".to_string(),
            parameter_types: vec![Type::Integer(64, false)],
            return_type: Type::Integer(64, false),
            attributes: IntrinsicAttributes {
                readonly: true,
                referentially_transparent: true,
                memory_access: false,
                thread_safe: true,
                available_targets: vec!["all".to_string()],
            },
        });

        // アトミック操作系イントリンシック
        self.register(IntrinsicFunction {
            kind: IntrinsicKind::AtomicLoad,
            name: "swiftlight_atomic_load".to_string(),
            parameter_types: vec![
                Type::Pointer(Box::new(Type::Integer(64, false)), true), // 読み取り対象ポインタ
                Type::Integer(32, false),                               // メモリオーダリング
            ],
            return_type: Type::Integer(64, false),
            attributes: IntrinsicAttributes {
                readonly: true,
                referentially_transparent: false, // メモリ状態に依存
                memory_access: true,
                thread_safe: true,
                available_targets: vec!["all".to_string()],
            },
        });

        self.register(IntrinsicFunction {
            kind: IntrinsicKind::AtomicStore,
            name: "swiftlight_atomic_store".to_string(),
            parameter_types: vec![
                Type::Pointer(Box::new(Type::Integer(64, false)), false), // 書き込み対象ポインタ
                Type::Integer(64, false),                                // 書き込む値
                Type::Integer(32, false),                                // メモリオーダリング
            ],
            return_type: Type::Void,
            attributes: IntrinsicAttributes {
                readonly: false,
                referentially_transparent: false,
                memory_access: true,
                thread_safe: true,
                available_targets: vec!["all".to_string()],
            },
        });

        // SIMD操作系イントリンシック
        self.register(IntrinsicFunction {
            kind: IntrinsicKind::VectorAdd,
            name: "swiftlight_vector_add".to_string(),
            parameter_types: vec![
                Type::Vector(Box::new(Type::Float(32)), 4), // 4要素のfloat32ベクトル
                Type::Vector(Box::new(Type::Float(32)), 4), // 4要素のfloat32ベクトル
            ],
            return_type: Type::Vector(Box::new(Type::Float(32)), 4),
            attributes: IntrinsicAttributes {
                readonly: true,
                referentially_transparent: true,
                memory_access: false,
                thread_safe: true,
                available_targets: vec!["x86_64".to_string(), "aarch64".to_string()],
            },
        });

        // 並行処理系イントリンシック
        self.register(IntrinsicFunction {
            kind: IntrinsicKind::Yield,
            name: "swiftlight_yield".to_string(),
            parameter_types: vec![],
            return_type: Type::Void,
            attributes: IntrinsicAttributes {
                readonly: false,
                referentially_transparent: false,
                memory_access: false,
                thread_safe: true,
                available_targets: vec!["all".to_string()],
            },
        });

        // デバッグ系イントリンシック
        self.register(IntrinsicFunction {
            kind: IntrinsicKind::DebugPrint,
            name: "swiftlight_debug_print".to_string(),
            parameter_types: vec![
                Type::Pointer(Box::new(Type::Integer(8, false)), true), // 文字列ポインタ
                Type::Integer(64, false),                              // 文字列長
            ],
            return_type: Type::Void,
            attributes: IntrinsicAttributes {
                readonly: false,
                referentially_transparent: false,
                memory_access: true,
                thread_safe: false, // 出力が混ざる可能性あり
                available_targets: vec!["all".to_string()],
            },
        });
        // デバッグ用イントリンシック - ブレークポイント挿入
        self.register(IntrinsicFunction {
            kind: IntrinsicKind::DebugBreak,
            name: "swiftlight_debug_break".to_string(),
            parameter_types: vec![],
            return_type: Type::Void,
            attributes: IntrinsicAttributes {
                readonly: false,
                referentially_transparent: false,
                memory_access: false,
                thread_safe: true,
                available_targets: vec!["all".to_string()],
            },
        });

        // メモリフェンス - 並行処理における順序付け保証
        self.register(IntrinsicFunction {
            kind: IntrinsicKind::Fence,
            name: "swiftlight_fence".to_string(),
            parameter_types: vec![Type::Integer(32, false)], // フェンスタイプ（Acquire=1, Release=2, AcqRel=3, SeqCst=4）
            return_type: Type::Void,
            attributes: IntrinsicAttributes {
                readonly: false,
                referentially_transparent: false,
                memory_access: true,
                thread_safe: true,
                available_targets: vec!["all".to_string()],
            },
        });

        // アトミックCAS（Compare-And-Swap）操作
        self.register(IntrinsicFunction {
            kind: IntrinsicKind::AtomicCAS,
            name: "swiftlight_atomic_cas".to_string(),
            parameter_types: vec![
                Type::Pointer(Box::new(Type::Integer(64, false)), false), // ターゲットポインタ
                Type::Integer(64, false),                                // 期待値
                Type::Integer(64, false),                                // 新しい値
                Type::Integer(32, false),                                // メモリオーダリング
            ],
            return_type: Type::Integer(64, false), // 操作前の値を返す
            attributes: IntrinsicAttributes {
                readonly: false,
                referentially_transparent: false,
                memory_access: true,
                thread_safe: true,
                available_targets: vec!["all".to_string()],
            },
        });

        // SIMD操作 - ベクトル乗算
        self.register(IntrinsicFunction {
            kind: IntrinsicKind::VectorMul,
            name: "swiftlight_vector_mul".to_string(),
            parameter_types: vec![
                Type::Vector(Box::new(Type::Float(32)), 4), // 4要素のfloat32ベクトル
                Type::Vector(Box::new(Type::Float(32)), 4), // 4要素のfloat32ベクトル
            ],
            return_type: Type::Vector(Box::new(Type::Float(32)), 4),
            attributes: IntrinsicAttributes {
                readonly: true,
                referentially_transparent: true,
                memory_access: false,
                thread_safe: true,
                available_targets: vec!["x86_64".to_string(), "aarch64".to_string()],
            },
        });

        // プロファイリング用イントリンシック - 関数開始
        self.register(IntrinsicFunction {
            kind: IntrinsicKind::ProfileFunctionEnter,
            name: "swiftlight_profile_enter".to_string(),
            parameter_types: vec![
                Type::Pointer(Box::new(Type::Integer(8, false)), true), // 関数名ポインタ
                Type::Integer(64, false),                              // 関数名長さ
            ],
            return_type: Type::Integer(64, false), // プロファイルID
            attributes: IntrinsicAttributes {
                readonly: false,
                referentially_transparent: false,
                memory_access: true,
                thread_safe: true,
                available_targets: vec!["all".to_string()],
            },
        });

        // プロファイリング用イントリンシック - 関数終了
        self.register(IntrinsicFunction {
            kind: IntrinsicKind::ProfileFunctionExit,
            name: "swiftlight_profile_exit".to_string(),
            parameter_types: vec![Type::Integer(64, false)], // プロファイルID
            return_type: Type::Void,
            attributes: IntrinsicAttributes {
                readonly: false,
                referentially_transparent: false,
                memory_access: true,
                thread_safe: true,
                available_targets: vec!["all".to_string()],
            },
        });

        // ハードウェア特化イントリンシック - CPUID取得
        self.register(IntrinsicFunction {
            kind: IntrinsicKind::CPUID,
            name: "swiftlight_cpuid".to_string(),
            parameter_types: vec![Type::Integer(32, false)], // 機能ID
            return_type: Type::Tuple(vec![
                Type::Integer(32, false), // EAX
                Type::Integer(32, false), // EBX
                Type::Integer(32, false), // ECX
                Type::Integer(32, false), // EDX
            ]),
            attributes: IntrinsicAttributes {
                readonly: true,
                referentially_transparent: false, // 実行環境に依存
                memory_access: false,
                thread_safe: true,
                available_targets: vec!["x86_64".to_string()],
            },
        });

        // メモリ最適化イントリンシック - プリフェッチ
        self.register(IntrinsicFunction {
            kind: IntrinsicKind::Prefetch,
            name: "swiftlight_prefetch".to_string(),
            parameter_types: vec![
                Type::Pointer(Box::new(Type::Any), false), // プリフェッチするメモリアドレス
                Type::Integer(32, false),                 // プリフェッチタイプ (0=読み込み, 1=書き込み)
                Type::Integer(32, false),                 // 局所性レベル (0=非局所的, 3=高局所性)
            ],
            return_type: Type::Void,
            attributes: IntrinsicAttributes {
                readonly: true,
                referentially_transparent: true,
                memory_access: false, // 実際のメモリアクセスはない（ヒント）
                thread_safe: true,
                available_targets: vec!["x86_64".to_string(), "aarch64".to_string()],
            },
        });

        // 暗号化イントリンシック - AES命令
        self.register(IntrinsicFunction {
            kind: IntrinsicKind::AESEncrypt,
            name: "swiftlight_aes_encrypt".to_string(),
            parameter_types: vec![
                Type::Vector(Box::new(Type::Integer(8, false)), 16), // 平文ブロック
                Type::Vector(Box::new(Type::Integer(8, false)), 16), // 暗号化キー
            ],
            return_type: Type::Vector(Box::new(Type::Integer(8, false)), 16), // 暗号文
            attributes: IntrinsicAttributes {
                readonly: true,
                referentially_transparent: true,
                memory_access: false,
                thread_safe: true,
                available_targets: vec!["x86_64_aesni".to_string()],
            },
        });

        // 並行処理イントリンシック - スピンロック
        self.register(IntrinsicFunction {
            kind: IntrinsicKind::SpinLock,
            name: "swiftlight_spin_lock".to_string(),
            parameter_types: vec![
                Type::Pointer(Box::new(Type::Integer(32, false)), false), // ロック変数へのポインタ
            ],
            return_type: Type::Void,
            attributes: IntrinsicAttributes {
                readonly: false,
                referentially_transparent: false,
                memory_access: true,
                thread_safe: true,
                available_targets: vec!["all".to_string()],
            },
        });

        // 並行処理イントリンシック - スピンアンロック
        self.register(IntrinsicFunction {
            kind: IntrinsicKind::SpinUnlock,
            name: "swiftlight_spin_unlock".to_string(),
            parameter_types: vec![
                Type::Pointer(Box::new(Type::Integer(32, false)), false), // ロック変数へのポインタ
            ],
            return_type: Type::Void,
            attributes: IntrinsicAttributes {
                readonly: false,
                referentially_transparent: false,
                memory_access: true,
                thread_safe: true,
                available_targets: vec!["all".to_string()],
            },
        });
    }
    /// イントリンシック関数を登録
    pub fn register(&mut self, intrinsic: IntrinsicFunction) {
        self.intrinsics.insert(intrinsic.kind, intrinsic);
    }
    
    /// イントリンシック関数を検索
    pub fn lookup(&self, kind: IntrinsicKind) -> Option<&IntrinsicFunction> {
        self.intrinsics.get(&kind)
    }
    
    /// イントリンシック関数をIR関数に変換
    pub fn to_ir_function(&self, kind: IntrinsicKind) -> Result<Function> {
        let intrinsic = self.lookup(kind)
            .ok_or_else(|| format!("イントリンシック関数が見つかりません: {:?}", kind))?;
            
        // パラメータの作成
        let mut parameters = Vec::new();
        for (i, param_type) in intrinsic.parameter_types.iter().enumerate() {
            parameters.push(Parameter {
                name: format!("param{}", i),
                ty: param_type.clone(),
                id: i,
                location: SourceLocation::builtin(),
            });
        }
        
        // 関数IDの生成
        let function_id = generate_unique_id();
        // イントリンシック関数の作成
        Ok(Function {
            id: function_id,
            name: Identifier::from(intrinsic.name.clone()),  // StringからIdentifierへの変換
            parameters,
            return_type: Some(intrinsic.return_type.clone()), // Optionでラップ
            basic_blocks: Vec::new(),
            body: Statement::Intrinsic(intrinsic.kind), // Statement型でラップ
            type_parameters: Vec::new(),
            visibility: Visibility::Public,
            is_async: false,
            is_extern: true,
            location: SourceLocation::builtin(),
            attributes: vec![FunctionAttribute::Intrinsic(intrinsic.kind)],
            is_declaration: true,  // イントリンシックは宣言のみ
            is_intrinsic: true,    // イントリンシック関数
        })
    }
    /// イントリンシック関数の呼び出しを生成
    pub fn generate_call(&self, kind: IntrinsicKind, arguments: Vec<Value>) -> Result<Value> {
        let intrinsic = self.lookup(kind)
            .ok_or_else(|| format!("イントリンシック関数が見つかりません: {:?}", kind))?;
            
        // 引数の数と型をチェック
        if arguments.len() != intrinsic.parameter_types.len() {
            return Err(CompilerError::new(
                format!("イントリンシック関数 {:?} の引数の数が一致しません (期待値: {}, 実際: {})", 
                    kind, 
                    intrinsic.parameter_types.len(), 
                    arguments.len()
                )
            ));
        }
        
        // 引数の型を厳密にチェック
        for (i, (arg, param_type)) in arguments.iter().zip(&intrinsic.parameter_types).enumerate() {
            if &arg.ty != param_type {
                return Err(CompilerError::new(
                    format!("イントリンシック関数 {:?} の{}番目の引数の型が不一致\n期待: {:?}\n実際: {:?}",
                        kind,
                        i + 1,
                        param_type,
                        arg.ty
                    )
                ));
            }
        }
        
        // 呼び出しを生成
        let call_id = generate_unique_id();
        let function_ref = Value::FunctionRef(intrinsic.name.clone(), intrinsic.return_type.clone());
        
        // 呼び出し命令を生成
        let call_instr = Instruction::Call {
            function: Box::new(function_ref),
            arguments: arguments.clone(),
            result_type: intrinsic.return_type.clone(),
            is_tail_call: false,
            calling_convention: CallingConvention::C,
        };
        
        // 結果値を生成
        Ok(Value::InstructionResult(call_id, intrinsic.return_type.clone(), Box::new(call_instr)))
    }
    
    /// イントリンシック関数が特定のターゲットで利用可能かチェック
    pub fn is_available_for_target(&self, kind: IntrinsicKind, target: &str) -> bool {
        if let Some(intrinsic) = self.lookup(kind) {
            intrinsic.attributes.available_targets.iter().any(|t| t == "all" || t == target)
        } else {
            false
        }
    }
    
    /// すべてのイントリンシック関数をIRモジュールに追加
    pub fn add_all_to_module(&self, module: &mut crate::middleend::ir::Module) -> Result<()> {
        for (kind, _) in &self.intrinsics {
            let function = self.to_ir_function(*kind)?;
            module.functions.push(function);
        }
        Ok(())
    }
}

// 特定のイントリンシック関数の実装
// 実際の実装ではこのようなモジュールが多数定義される

/// メモリ操作イントリンシック
pub mod memory {
    use super::*;
    
    /// memcpyイントリンシックを生成
    pub fn build_memcpy(
        manager: &IntrinsicManager,
        dest: Value,
        src: Value,
        size: Value,
        align: u32,
        is_volatile: bool
    ) -> Result<Value> {
        // アライメントと揮発性フラグを追加パラメータとして渡す
        let align_val = Value::Constant(align as i64);
        let volatile_val = Value::Constant(if is_volatile { 1 } else { 0 });
        
        manager.generate_call(IntrinsicKind::MemCpy, vec![dest, src, size, align_val, volatile_val])
    }
    
    /// memsetイントリンシックを生成
    pub fn build_memset(
        manager: &IntrinsicManager,
        dest: Value,
        val: Value,
        size: Value,
        align: u32,
        is_volatile: bool
    ) -> Result<Value> {
        // アライメントと揮発性フラグを追加パラメータとして渡す
        let align_val = Value::Constant(align as i64);
        let volatile_val = Value::Constant(if is_volatile { 1 } else { 0 });
        
        manager.generate_call(IntrinsicKind::MemSet, vec![dest, val, size, align_val, volatile_val])
    }
    
    /// memmoveイントリンシックを生成（重複領域対応）
    pub fn build_memmove(
        manager: &IntrinsicManager,
        dest: Value,
        src: Value,
        size: Value,
        align: u32,
        is_volatile: bool
    ) -> Result<Value> {
        let align_val = Value::Constant(align as i64);
        let volatile_val = Value::Constant(if is_volatile { 1 } else { 0 });
        
        manager.generate_call(IntrinsicKind::MemMove, vec![dest, src, size, align_val, volatile_val])
    }
    
    /// メモリバリアを生成
    pub fn build_memory_barrier(
        manager: &IntrinsicManager,
        ordering: MemoryOrdering
    ) -> Result<Value> {
        let ordering_val = Value::Constant(ordering as i32);
        manager.generate_call(IntrinsicKind::Fence, vec![ordering_val])
    }
}

/// 数学イントリンシック
pub mod math {
    use super::*;
    
    /// 平方根イントリンシックを生成
    pub fn build_sqrt(
        manager: &IntrinsicManager,
        value: Value
    ) -> Result<Value> {
        manager.generate_call(IntrinsicKind::Sqrt, vec![value])
    }
    
    /// 正弦イントリンシックを生成
    pub fn build_sin(
        manager: &IntrinsicManager,
        value: Value
    ) -> Result<Value> {
        manager.generate_call(IntrinsicKind::Sin, vec![value])
    }
    
    /// 余弦イントリンシックを生成
    pub fn build_cos(
        manager: &IntrinsicManager,
        value: Value
    ) -> Result<Value> {
        manager.generate_call(IntrinsicKind::Cos, vec![value])
    }
    
    /// 指数関数イントリンシックを生成
    pub fn build_exp(
        manager: &IntrinsicManager,
        value: Value
    ) -> Result<Value> {
        manager.generate_call(IntrinsicKind::Exp, vec![value])
    }
    
    /// 対数関数イントリンシックを生成
    pub fn build_log(
        manager: &IntrinsicManager,
        value: Value
    ) -> Result<Value> {
        manager.generate_call(IntrinsicKind::Log, vec![value])
    }
    
    /// 高精度数学演算（多倍長精度）
    pub fn build_high_precision_op(
        manager: &IntrinsicManager,
        op_type: HighPrecisionOpType,
        values: Vec<Value>
    ) -> Result<Value> {
        let op_type_val = Value::Constant(op_type as i32);
        let mut args = vec![op_type_val];
        args.extend(values);
        
        manager.generate_call(IntrinsicKind::HighPrecisionMath, args)
    }
}

/// SIMD操作イントリンシック
pub mod simd {
    use super::*;
    
    /// SIMDシャッフルイントリンシックを生成
    pub fn build_shuffle(
        manager: &IntrinsicManager,
        vec1: Value,
        vec2: Value,
        mask: Value
    ) -> Result<Value> {
        manager.generate_call(IntrinsicKind::VectorShuffle, vec![vec1, vec2, mask])
    }
    
    /// SIMD要素追加イントリンシックを生成
    pub fn build_vector_add(
        manager: &IntrinsicManager,
        vec1: Value,
        vec2: Value
    ) -> Result<Value> {
        manager.generate_call(IntrinsicKind::VectorAdd, vec![vec1, vec2])
    }
    
    /// SIMD要素乗算イントリンシックを生成
    pub fn build_vector_mul(
        manager: &IntrinsicManager,
        vec1: Value,
        vec2: Value
    ) -> Result<Value> {
        manager.generate_call(IntrinsicKind::VectorMul, vec![vec1, vec2])
    }
    
    /// SIMD要素比較イントリンシックを生成
    pub fn build_vector_compare(
        manager: &IntrinsicManager,
        vec1: Value,
        vec2: Value,
        compare_type: VectorCompareType
    ) -> Result<Value> {
        let compare_type_val = Value::Constant(compare_type as i32);
        manager.generate_call(IntrinsicKind::VectorCompare, vec![vec1, vec2, compare_type_val])
    }
    
    /// SIMD要素抽出イントリンシックを生成
    pub fn build_vector_extract(
        manager: &IntrinsicManager,
        vec: Value,
        index: Value
    ) -> Result<Value> {
        manager.generate_call(IntrinsicKind::VectorExtract, vec![vec, index])
    }
    
    /// SIMD要素挿入イントリンシックを生成
    pub fn build_vector_insert(
        manager: &IntrinsicManager,
        vec: Value,
        value: Value,
        index: Value
    ) -> Result<Value> {
        manager.generate_call(IntrinsicKind::VectorInsert, vec![vec, value, index])
    }
}

/// アトミック操作イントリンシック
pub mod atomic {
    use super::*;
    
    /// アトミック読み込みイントリンシックを生成
    pub fn build_load(
        manager: &IntrinsicManager,
        ptr: Value, 
        ordering: MemoryOrdering
    ) -> Result<Value> {
        let ordering_val = Value::Constant(ordering as i32);
        manager.generate_call(IntrinsicKind::AtomicLoad, vec![ptr, ordering_val])
    }
    
    /// アトミック書き込みイントリンシックを生成
    pub fn build_store(
        manager: &IntrinsicManager,
        ptr: Value,
        val: Value,
        ordering: MemoryOrdering
    ) -> Result<Value> {
        let ordering_val = Value::Constant(ordering as i32);
        manager.generate_call(IntrinsicKind::AtomicStore, vec![ptr, val, ordering_val])
    }
    
    /// アトミック加算イントリンシックを生成
    pub fn build_add(
        manager: &IntrinsicManager,
        ptr: Value,
        val: Value,
        ordering: MemoryOrdering
    ) -> Result<Value> {
        let ordering_val = Value::Constant(ordering as i32);
        manager.generate_call(IntrinsicKind::AtomicAdd, vec![ptr, val, ordering_val])
    }
    
    /// アトミックCAS（Compare-And-Swap）イントリンシックを生成
    pub fn build_compare_exchange(
        manager: &IntrinsicManager,
        ptr: Value,
        expected: Value,
        new_value: Value,
        success_ordering: MemoryOrdering,
        failure_ordering: MemoryOrdering
    ) -> Result<Value> {
        let success_ordering_val = Value::Constant(success_ordering as i32);
        let failure_ordering_val = Value::Constant(failure_ordering as i32);
        
        manager.generate_call(
            IntrinsicKind::AtomicCAS, 
            vec![ptr, expected, new_value, success_ordering_val, failure_ordering_val]
        )
    }
    
    /// アトミックフェッチ加算イントリンシックを生成
    pub fn build_fetch_add(
        manager: &IntrinsicManager,
        ptr: Value,
        val: Value,
        ordering: MemoryOrdering
    ) -> Result<Value> {
        let ordering_val = Value::Constant(ordering as i32);
        manager.generate_call(IntrinsicKind::AtomicFetchAdd, vec![ptr, val, ordering_val])
    }
}