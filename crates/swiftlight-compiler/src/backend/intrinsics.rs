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
        // 実際の実装では標準的なイントリンシックを登録する
        // この実装はダミー
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
            name: intrinsic.name.clone(),
            parameters,
            return_type: intrinsic.return_type.clone(),
            basic_blocks: Vec::new(),
            body: None,
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
            return Err(format!("イントリンシック関数 {:?} の引数の数が一致しません", kind).into());
        }
        
        // 引数の型をチェック
        // 実際の実装ではより厳密なチェックが必要
        
        // 呼び出しを生成
        // 実際の実装ではより複雑になる
        
        // ダミー実装：仮の値を返す
        Ok(Value::Constant(0)) // ダミー実装
    }
    
    /// イントリンシック関数が特定のターゲットで利用可能かチェック
    pub fn is_available_for_target(&self, kind: IntrinsicKind, target: &str) -> bool {
        if let Some(intrinsic) = self.lookup(kind) {
            intrinsic.attributes.available_targets.iter().any(|t| t == target)
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
        // 実際の実装
        manager.generate_call(IntrinsicKind::MemCpy, vec![dest, src, size])
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
        // 実際の実装
        manager.generate_call(IntrinsicKind::MemSet, vec![dest, val, size])
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
        // 実際の実装
        manager.generate_call(IntrinsicKind::Sqrt, vec![value])
    }
    
    /// 正弦イントリンシックを生成
    pub fn build_sin(
        manager: &IntrinsicManager,
        value: Value
    ) -> Result<Value> {
        // 実際の実装
        manager.generate_call(IntrinsicKind::Sin, vec![value])
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
        // 実際の実装
        manager.generate_call(IntrinsicKind::SimdShuffle, vec![vec1, vec2, mask])
    }
    
    /// SIMD要素追加イントリンシックを生成
    pub fn build_add(
        manager: &IntrinsicManager,
        vec1: Value,
        vec2: Value
    ) -> Result<Value> {
        // 実際の実装
        manager.generate_call(IntrinsicKind::SimdAdd, vec![vec1, vec2])
    }
}

/// アトミック操作イントリンシック
pub mod atomic {
    use super::*;
    
    /// アトミック読み込みイントリンシックを生成
    pub fn build_load(
        manager: &IntrinsicManager,
        ptr: Value,
        ordering: Value
    ) -> Result<Value> {
        // 実際の実装
        manager.generate_call(IntrinsicKind::AtomicLoad, vec![ptr, ordering])
    }
    
    /// アトミック加算イントリンシックを生成
    pub fn build_add(
        manager: &IntrinsicManager,
        ptr: Value,
        val: Value,
        ordering: Value
    ) -> Result<Value> {
        // 実際の実装
        manager.generate_call(IntrinsicKind::AtomicAdd, vec![ptr, val, ordering])
    }
} 