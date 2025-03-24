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
                Type::Integer(32, false),                 // キャッシュレベル (0=L1, 1=L2, 2=L3)
                Type::Integer(32, false),                 // 優先度 (0=低, 10=最高)
            ],
            return_type: Type::Void,
            attributes: IntrinsicAttributes {
                readonly: true,
                referentially_transparent: true,
                memory_access: false, // 実際のメモリアクセスはない（ヒント）
                thread_safe: true,
                available_targets: vec!["x86_64".to_string(), "aarch64".to_string(), "riscv64".to_string(), "wasm32".to_string()],
            },
        });

        // メモリ最適化イントリンシック - 高度なプリフェッチ
        self.register(IntrinsicFunction {
            kind: IntrinsicKind::AdvancedPrefetch,
            name: "swiftlight_advanced_prefetch".to_string(),
            parameter_types: vec![
                Type::Pointer(Box::new(Type::Any), false), // プリフェッチするメモリアドレス
                Type::Integer(64, false),                 // データサイズ
                Type::Integer(32, false),                 // アクセスパターン (0=シーケンシャル, 1=ストライド, 2=ランダム, 3=リバース)
                Type::Integer(32, false),                 // ストライド幅（アクセスパターンがストライドの場合）
                Type::Integer(32, false),                 // 優先度 (0=低, 10=最高)
                Type::Boolean,                           // 投機的実行フラグ
            ],
            return_type: Type::Void,
            attributes: IntrinsicAttributes {
                readonly: true,
                referentially_transparent: true,
                memory_access: false,
                thread_safe: true,
                available_targets: vec!["x86_64".to_string(), "aarch64".to_string(), "riscv64".to_string()],
            },
        });

        // メモリ最適化イントリンシック - キャッシュライン無効化
        self.register(IntrinsicFunction {
            kind: IntrinsicKind::CacheLineInvalidate,
            name: "swiftlight_cache_invalidate".to_string(),
            parameter_types: vec![
                Type::Pointer(Box::new(Type::Any), false), // 無効化するメモリアドレス
                Type::Integer(32, false),                 // キャッシュレベル (0=L1, 1=L2, 2=L3, 3=全レベル)
            ],
            return_type: Type::Void,
            attributes: IntrinsicAttributes {
                readonly: false,
                referentially_transparent: false,
                memory_access: true,
                thread_safe: true,
                available_targets: vec!["x86_64".to_string(), "aarch64".to_string()],
            },
        });

        // メモリ最適化イントリンシック - キャッシュラインフラッシュ
        self.register(IntrinsicFunction {
            kind: IntrinsicKind::CacheLineFlush,
            name: "swiftlight_cache_flush".to_string(),
            parameter_types: vec![
                Type::Pointer(Box::new(Type::Any), false), // フラッシュするメモリアドレス
                Type::Integer(32, false),                 // キャッシュレベル (0=L1, 1=L2, 2=L3, 3=全レベル)
                Type::Boolean,                           // 同期フラグ（trueの場合、フラッシュ完了まで待機）
            ],
            return_type: Type::Void,
            attributes: IntrinsicAttributes {
                readonly: false,
                referentially_transparent: false,
                memory_access: true,
                thread_safe: true,
                available_targets: vec!["x86_64".to_string(), "aarch64".to_string(), "riscv64".to_string()],
            },
        });

        // メモリ最適化イントリンシック - 非時間的メモリアクセス
        self.register(IntrinsicFunction {
            kind: IntrinsicKind::NonTemporalStore,
            name: "swiftlight_non_temporal_store".to_string(),
            parameter_types: vec![
                Type::Pointer(Box::new(Type::Any), false), // 書き込み先アドレス
                Type::Any,                                // 書き込むデータ
            ],
            return_type: Type::Void,
            attributes: IntrinsicAttributes {
                readonly: false,
                referentially_transparent: false,
                memory_access: true,
                thread_safe: true,
                available_targets: vec!["x86_64".to_string(), "aarch64".to_string()],
            },
        });

        // メモリ最適化イントリンシック - メモリアクセスヒント
        self.register(IntrinsicFunction {
            kind: IntrinsicKind::MemoryAccessHint,
            name: "swiftlight_memory_hint".to_string(),
            parameter_types: vec![
                Type::Pointer(Box::new(Type::Any), false), // メモリ領域の開始アドレス
                Type::Integer(64, false),                 // サイズ
                Type::Integer(32, false),                 // アクセスパターン (0=一度だけ, 1=繰り返し読み込み, 2=繰り返し書き込み, 3=ストリーミング)
                Type::Integer(32, false),                 // 優先度 (0=低, 10=最高)
            ],
            return_type: Type::Void,
            attributes: IntrinsicAttributes {
                readonly: true,
                referentially_transparent: true,
                memory_access: false,
                thread_safe: true,
                available_targets: vec!["all".to_string()],
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
                available_targets: vec!["x86_64_aesni".to_string(), "aarch64_crypto".to_string()],
            },
        });

        // 暗号化イントリンシック - AES復号
        self.register(IntrinsicFunction {
            kind: IntrinsicKind::AESDecrypt,
            name: "swiftlight_aes_decrypt".to_string(),
            parameter_types: vec![
                Type::Vector(Box::new(Type::Integer(8, false)), 16), // 暗号文ブロック
                Type::Vector(Box::new(Type::Integer(8, false)), 16), // 暗号化キー
            ],
            return_type: Type::Vector(Box::new(Type::Integer(8, false)), 16), // 平文
            attributes: IntrinsicAttributes {
                readonly: true,
                referentially_transparent: true,
                memory_access: false,
                thread_safe: true,
                available_targets: vec!["x86_64_aesni".to_string(), "aarch64_crypto".to_string()],
            },
        });

        // 暗号化イントリンシック - SHA256ハッシュ
        self.register(IntrinsicFunction {
            kind: IntrinsicKind::SHA256,
            name: "swiftlight_sha256".to_string(),
            parameter_types: vec![
                Type::Pointer(Box::new(Type::Integer(8, false)), false), // データポインタ
                Type::Integer(64, false),                               // データ長
            ],
            return_type: Type::Vector(Box::new(Type::Integer(8, false)), 32), // 256ビットハッシュ
            attributes: IntrinsicAttributes {
                readonly: true,
                referentially_transparent: true,
                memory_access: true,
                thread_safe: true,
                available_targets: vec!["x86_64_sha".to_string(), "aarch64_crypto".to_string(), "all".to_string()],
            },
        });

        // SIMD操作イントリンシック - ベクトル加算
        self.register(IntrinsicFunction {
            kind: IntrinsicKind::VectorAdd,
            name: "swiftlight_vector_add".to_string(),
            parameter_types: vec![
                Type::Vector(Box::new(Type::Float(32)), 4), // ベクトル1
                Type::Vector(Box::new(Type::Float(32)), 4), // ベクトル2
            ],
            return_type: Type::Vector(Box::new(Type::Float(32)), 4), // 結果ベクトル
            attributes: IntrinsicAttributes {
                readonly: true,
                referentially_transparent: true,
                memory_access: false,
                thread_safe: true,
                available_targets: vec!["x86_64_avx".to_string(), "aarch64_neon".to_string(), "all".to_string()],
            },
        });

        // SIMD操作イントリンシック - ベクトル乗算
        self.register(IntrinsicFunction {
            kind: IntrinsicKind::VectorMul,
            name: "swiftlight_vector_mul".to_string(),
            parameter_types: vec![
                Type::Vector(Box::new(Type::Float(32)), 4), // ベクトル1
                Type::Vector(Box::new(Type::Float(32)), 4), // ベクトル2
            ],
            return_type: Type::Vector(Box::new(Type::Float(32)), 4), // 結果ベクトル
            attributes: IntrinsicAttributes {
                readonly: true,
                referentially_transparent: true,
                memory_access: false,
                thread_safe: true,
                available_targets: vec!["x86_64_avx".to_string(), "aarch64_neon".to_string(), "all".to_string()],
            },
        });

        // SIMD操作イントリンシック - ベクトル内積
        self.register(IntrinsicFunction {
            kind: IntrinsicKind::VectorDot,
            name: "swiftlight_vector_dot".to_string(),
            parameter_types: vec![
                Type::Vector(Box::new(Type::Float(32)), 4), // ベクトル1
                Type::Vector(Box::new(Type::Float(32)), 4), // ベクトル2
            ],
            return_type: Type::Float(32), // スカラー結果
            attributes: IntrinsicAttributes {
                readonly: true,
                referentially_transparent: true,
                memory_access: false,
                thread_safe: true,
                available_targets: vec!["x86_64_avx".to_string(), "aarch64_neon".to_string(), "all".to_string()],
            },
        });

        // 並行処理イントリンシック - アトミック比較交換
        self.register(IntrinsicFunction {
            kind: IntrinsicKind::AtomicCmpXchg,
            name: "swiftlight_atomic_cmpxchg".to_string(),
            parameter_types: vec![
                Type::Pointer(Box::new(Type::Integer(64, false)), false), // ポインタ
                Type::Integer(64, false),                               // 期待値
                Type::Integer(64, false),                               // 新値
            ],
            return_type: Type::Tuple(vec![
                Type::Integer(64, false),  // 前の値
                Type::Boolean,             // 成功したかどうか
            ]),
            attributes: IntrinsicAttributes {
                readonly: false,
                referentially_transparent: false,
                memory_access: true,
                thread_safe: true,
                available_targets: vec!["all".to_string()],
            },
        });

        // 並行処理イントリンシック - アトミック加算
        self.register(IntrinsicFunction {
            kind: IntrinsicKind::AtomicAdd,
            name: "swiftlight_atomic_add".to_string(),
            parameter_types: vec![
                Type::Pointer(Box::new(Type::Integer(64, false)), false), // ポインタ
                Type::Integer(64, false),                               // 加算値
            ],
            return_type: Type::Integer(64, false), // 前の値
            attributes: IntrinsicAttributes {
                readonly: false,
                referentially_transparent: false,
                memory_access: true,
                thread_safe: true,
                available_targets: vec!["all".to_string()],
            },
        });

        // 並行処理イントリンシック - メモリバリア
        self.register(IntrinsicFunction {
            kind: IntrinsicKind::MemoryBarrier,
            name: "swiftlight_memory_barrier".to_string(),
            parameter_types: vec![
                Type::Integer(32, false), // バリアタイプ (0=全バリア, 1=読み込みバリア, 2=書き込みバリア)
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

        // 時間認識イントリンシック - 高精度タイマー
        self.register(IntrinsicFunction {
            kind: IntrinsicKind::HighPrecisionTimer,
            name: "swiftlight_high_precision_timer".to_string(),
            parameter_types: vec![],
            return_type: Type::Integer(64, false), // ナノ秒単位のタイムスタンプ
            attributes: IntrinsicAttributes {
                readonly: true,
                referentially_transparent: false, // 時間に依存
                memory_access: false,
                thread_safe: true,
                available_targets: vec!["all".to_string()],
            },
        });

        // 時間認識イントリンシック - スリープ
        self.register(IntrinsicFunction {
            kind: IntrinsicKind::Sleep,
            name: "swiftlight_sleep".to_string(),
            parameter_types: vec![
                Type::Integer(64, false), // ナノ秒単位のスリープ時間
            ],
            return_type: Type::Void,
            attributes: IntrinsicAttributes {
                readonly: false,
                referentially_transparent: false,
                memory_access: false,
                thread_safe: true,
                available_targets: vec!["all".to_string()],
            },
        });

        // 時間認識イントリンシック - デッドライン付き実行
        self.register(IntrinsicFunction {
            kind: IntrinsicKind::ExecuteWithDeadline,
            name: "swiftlight_execute_with_deadline".to_string(),
            parameter_types: vec![
                Type::Function(
                    vec![],
                    Box::new(Type::Any),
                    vec![],
                ), // 実行する関数
                Type::Integer(64, false), // ナノ秒単位のデッドライン
            ],
            return_type: Type::Tuple(vec![
                Type::Any,       // 関数の戻り値
                Type::Boolean,   // デッドライン内に完了したか
            ]),
            attributes: IntrinsicAttributes {
                readonly: false,
                referentially_transparent: false,
                memory_access: true,
                thread_safe: true,
                available_targets: vec!["all".to_string()],
            },
        });

        // ガベージコレクション制御イントリンシック
        self.register(IntrinsicFunction {
            kind: IntrinsicKind::GCControl,
            name: "swiftlight_gc_control".to_string(),
            parameter_types: vec![
                Type::Integer(32, false), // 操作タイプ (0=GC実行, 1=GC一時停止, 2=GC再開, 3=統計取得)
            ],
            return_type: Type::Integer(64, false), // 操作結果または統計情報
            attributes: IntrinsicAttributes {
                readonly: false,
                referentially_transparent: false,
                memory_access: true,
                thread_safe: false, // GC操作は通常スレッドセーフではない
                available_targets: vec!["all".to_string()],
            },
        });

        // ハードウェア特化イントリンシック - RDTSC (タイムスタンプカウンタ読み取り)
        self.register(IntrinsicFunction {
            kind: IntrinsicKind::RDTSC,
            name: "swiftlight_rdtsc".to_string(),
            parameter_types: vec![],
            return_type: Type::Integer(64, false), // 64ビットカウンタ値
            attributes: IntrinsicAttributes {
                readonly: true,
                referentially_transparent: false, // 実行時間に依存
                memory_access: false,
                thread_safe: true,
                available_targets: vec!["x86_64".to_string()],
            },
        });

        // ハードウェア特化イントリンシック - RDRAND (ハードウェア乱数生成)
        self.register(IntrinsicFunction {
            kind: IntrinsicKind::RDRAND,
            name: "swiftlight_rdrand".to_string(),
            parameter_types: vec![],
            return_type: Type::Tuple(vec![
                Type::Integer(64, false), // 乱数値
                Type::Boolean,            // 成功フラグ
            ]),
            attributes: IntrinsicAttributes {
                readonly: true,
                referentially_transparent: false,
                memory_access: false,
                thread_safe: true,
                available_targets: vec!["x86_64_rdrand".to_string()],
            },
        });

        // 依存型サポートイントリンシック - 型レベル計算
        self.register(IntrinsicFunction {
            kind: IntrinsicKind::TypeLevelComputation,
            name: "swiftlight_type_level_compute".to_string(),
            parameter_types: vec![
                Type::TypeValue,                  // 計算する型
                Type::Integer(32, false),         // 操作タイプ
            ],
            return_type: Type::TypeValue,         // 結果の型
            attributes: IntrinsicAttributes {
                readonly: true,
                referentially_transparent: true,
                memory_access: false,
                thread_safe: true,
                available_targets: vec!["all".to_string()],
            },
        });

        // 依存型サポートイントリンシック - 実行時型情報
        self.register(IntrinsicFunction {
            kind: IntrinsicKind::RuntimeTypeInfo,
            name: "swiftlight_runtime_type_info".to_string(),
            parameter_types: vec![
                Type::Any,                        // 型情報を取得する値
            ],
            return_type: Type::TypeDescriptor,    // 型記述子
            attributes: IntrinsicAttributes {
                readonly: true,
                referentially_transparent: true,
                memory_access: false,
                thread_safe: true,
                available_targets: vec!["all".to_string()],
            },
        });

        // メタプログラミングイントリンシック - AST操作
        self.register(IntrinsicFunction {
            kind: IntrinsicKind::ASTManipulation,
            name: "swiftlight_ast_manipulate".to_string(),
            parameter_types: vec![
                Type::ASTNode,                    // 操作対象のASTノード
                Type::Integer(32, false),         // 操作タイプ
                Type::Any,                        // 操作パラメータ
            ],
            return_type: Type::ASTNode,           // 結果のASTノード
            attributes: IntrinsicAttributes {
                readonly: true,
                referentially_transparent: true,
                memory_access: false,
                thread_safe: true,
                available_targets: vec!["all".to_string()],
            },
        });

        // メタプログラミングイントリンシック - コンパイル時評価
        self.register(IntrinsicFunction {
            kind: IntrinsicKind::CompileTimeEval,
            name: "swiftlight_compile_time_eval".to_string(),
            parameter_types: vec![
                Type::Function(
                    vec![],
                    Box::new(Type::Any),
                    vec![],
                ), // 評価する関数
            ],
            return_type: Type::Any,               // 評価結果
            attributes: IntrinsicAttributes {
                readonly: true,
                referentially_transparent: true,
                memory_access: false,
                thread_safe: true,
                available_targets: vec!["all".to_string()],
            },
        });

        // 投機的実行イントリンシック - 分岐予測ヒント
        self.register(IntrinsicFunction {
            kind: IntrinsicKind::BranchPredictionHint,
            name: "swiftlight_branch_prediction_hint".to_string(),
            parameter_types: vec![
                Type::Boolean,                    // 条件
                Type::Integer(32, false),         // ヒントタイプ (0=likely, 1=unlikely)
            ],
            return_type: Type::Boolean,           // 元の条件をそのまま返す
            attributes: IntrinsicAttributes {
                readonly: true,
                referentially_transparent: true,
                memory_access: false,
                thread_safe: true,
                available_targets: vec!["all".to_string()],
            },
        });

        // 投機的実行イントリンシック - 投機的実行
        self.register(IntrinsicFunction {
            kind: IntrinsicKind::SpeculativeExecution,
            name: "swiftlight_speculative_execute".to_string(),
            parameter_types: vec![
                Type::Function(
                    vec![],
                    Box::new(Type::Any),
                    vec![],
                ), // 投機的に実行する関数
                Type::Function(
                    vec![],
                    Box::new(Type::Any),
                    vec![],
                ), // フォールバック関数
            ],
            return_type: Type::Any,               // 実行結果
            attributes: IntrinsicAttributes {
                readonly: false,
                referentially_transparent: false,
                memory_access: true,
                thread_safe: true,
                available_targets: vec!["all".to_string()],
            },
        });

        // 自己適応型最適化イントリンシック - プロファイル情報収集
        self.register(IntrinsicFunction {
            kind: IntrinsicKind::ProfileInfoCollect,
            name: "swiftlight_profile_info_collect".to_string(),
            parameter_types: vec![
                Type::Integer(64, false),         // プロファイルポイントID
                Type::Any,                        // プロファイル情報
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