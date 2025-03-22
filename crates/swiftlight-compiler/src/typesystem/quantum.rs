// SwiftLight Type System - Quantum
// 量子型システムの実装

//! # 量子型システム
//! 
//! SwiftLight言語の量子コンピューティングサポートのための型システムを実装します。
//! このモジュールは、量子ビット、量子回路、量子状態の表現と操作を型安全に提供します。
//! 
//! - 量子ビット型（Qubit）
//! - 量子回路型（QuantumCircuit）
//! - 量子状態型（QuantumState）
//! - 量子レジスタ型（QuantumRegister）
//! - 線形代数演算のサポート
//! - 量子操作の静的検証

use std::collections::{HashMap, HashSet, BTreeSet};
use std::fmt;
use std::sync::{Arc, Mutex, RwLock};

use crate::frontend::ast::{Expr, ExprKind};
use crate::frontend::error::{CompilerError, ErrorKind, Result, SourceLocation};
use crate::typesystem::{
    Type, TypeId, TypeRegistry, Symbol, Kind,
    TypeError, TypeManager,
    effects::{EffectSet, EffectKind},
};

/// 量子ビットの型
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QubitType {
    /// 単一量子ビット
    Single,
    /// 量子ビット配列
    Array(usize),
    /// サイズ可変の量子ビット配列
    DynamicArray,
    /// エンタングル状態の量子ビット
    Entangled(Vec<TypeId>),
}

/// 量子操作の種類
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum QuantumOperation {
    /// パウリのX（NOT）ゲート
    PauliX,
    /// パウリのYゲート
    PauliY,
    /// パウリのZゲート
    PauliZ,
    /// アダマールゲート
    Hadamard,
    /// 位相ゲート
    Phase(f64),
    /// 制御NOTゲート
    ControlledNot,
    /// 制御Zゲート
    ControlledZ,
    /// 制御位相ゲート
    ControlledPhase(f64),
    /// SWAPゲート
    Swap,
    /// 測定操作
    Measure,
    /// リセット操作
    Reset,
    /// トフォリゲート
    Toffoli,
    /// カスタム操作
    Custom(Symbol),
}

/// 量子回路
#[derive(Debug, Clone)]
pub struct QuantumCircuit {
    /// 回路の名前
    pub name: Symbol,
    /// 入力量子ビット
    pub input_qubits: Vec<TypeId>,
    /// 補助量子ビット
    pub ancilla_qubits: Vec<TypeId>,
    /// 出力量子ビット
    pub output_qubits: Vec<TypeId>,
    /// 回路内の操作
    pub operations: Vec<(QuantumOperation, Vec<usize>)>,
}

/// 量子状態
#[derive(Debug, Clone)]
pub struct QuantumState {
    /// 量子ビット数
    pub num_qubits: usize,
    /// 状態の種類
    pub state_type: QuantumStateType,
}

/// 量子状態の種類
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QuantumStateType {
    /// 純粋状態（状態ベクトル）
    Pure,
    /// 混合状態（密度行列）
    Mixed,
    /// エンタングル状態
    Entangled,
    /// スタビライザー状態
    Stabilizer,
}

/// 量子型エラー
#[derive(Debug, Clone)]
pub enum QuantumTypeError {
    /// 非量子型に対する量子操作
    NonQuantumOperation,
    /// 量子ビット数の不一致
    QubitCountMismatch,
    /// 古典ビットへの量子操作
    ClassicalBitQuantumOperation,
    /// 測定済み量子ビットの再利用
    MeasuredQubitReuse,
    /// エンタングル状態の不正な操作
    InvalidEntangledOperation,
    /// その他のエラー
    Other(String),
}

/// 量子レジスタ
#[derive(Debug, Clone)]
pub struct QuantumRegister {
    /// レジスタの名前
    pub name: Symbol,
    /// 量子ビット数
    pub size: usize,
    /// 量子ビットの型
    pub qubit_type: TypeId,
    /// 測定状態（インデックス -> 測定済みかどうか）
    pub measured: HashMap<usize, bool>,
}

/// 量子型チェッカー
pub struct QuantumTypeChecker {
    /// 量子回路
    circuits: HashMap<Symbol, QuantumCircuit>,
    /// 量子レジスタ
    registers: HashMap<Symbol, QuantumRegister>,
    /// 量子変数
    quantum_vars: HashMap<Symbol, TypeId>,
    /// 量子エラー
    errors: Vec<QuantumTypeError>,
    /// 型レジストリへの参照
    type_registry: Arc<TypeRegistry>,
}

impl QuantumTypeChecker {
    /// 新しい量子型チェッカーを作成
    pub fn new(type_registry: Arc<TypeRegistry>) -> Self {
        Self {
            circuits: HashMap::new(),
            registers: HashMap::new(),
            quantum_vars: HashMap::new(),
            errors: Vec::new(),
            type_registry,
        }
    }
    
    /// 量子回路を登録
    pub fn register_circuit(&mut self, circuit: QuantumCircuit) {
        self.circuits.insert(circuit.name, circuit);
    }
    
    /// 量子レジスタを登録
    pub fn register_quantum_register(&mut self, register: QuantumRegister) {
        self.registers.insert(register.name, register);
    }
    
    /// 量子変数を登録
    pub fn register_quantum_variable(&mut self, name: Symbol, type_id: TypeId) {
        self.quantum_vars.insert(name, type_id);
    }
    
    /// 型が量子型かどうかをチェック
    pub fn is_quantum_type(&self, type_id: TypeId) -> bool {
        let ty = self.type_registry.resolve(type_id);
        
        match &*ty {
            Type::Named { name, .. } => {
                // 名前に基づいて量子型かどうかを判断
                name.to_string().contains("Qubit") ||
                name.to_string().contains("Quantum") ||
                name.to_string().contains("Circuit")
            },
            // 他の型は量子型ではない
            _ => false,
        }
    }
    
    /// 量子操作が適用可能かチェック
    pub fn check_quantum_operation(&mut self, op: &QuantumOperation, targets: &[TypeId], location: SourceLocation) -> Result<()> {
        match op {
            QuantumOperation::PauliX | QuantumOperation::PauliY | QuantumOperation::PauliZ | QuantumOperation::Hadamard => {
                // 単一量子ビット操作
                if targets.len() != 1 {
                    return Err(CompilerError::new(
                        ErrorKind::TypeSystem,
                        format!("{:?}ゲートは単一の量子ビットに適用する必要があります", op),
                        location,
                    ));
                }
                
                let target_type_id = targets[0];
                if !self.is_quantum_type(target_type_id) {
                    return Err(CompilerError::new(
                        ErrorKind::TypeSystem,
                        format!("非量子型に対する量子操作"),
                        location,
                    ));
                }
            },
            
            QuantumOperation::ControlledNot | QuantumOperation::ControlledZ | QuantumOperation::Swap => {
                // 2量子ビット操作
                if targets.len() != 2 {
                    return Err(CompilerError::new(
                        ErrorKind::TypeSystem,
                        format!("{:?}ゲートは2つの量子ビットに適用する必要があります", op),
                        location,
                    ));
                }
                
                // 両方の対象が量子型であることを確認
                for &target in targets {
                    if !self.is_quantum_type(target) {
                        return Err(CompilerError::new(
                            ErrorKind::TypeSystem,
                            format!("非量子型に対する量子操作"),
                            location,
                        ));
                    }
                }
            },
            
            QuantumOperation::Toffoli => {
                // 3量子ビット操作
                if targets.len() != 3 {
                    return Err(CompilerError::new(
                        ErrorKind::TypeSystem,
                        format!("Toffoliゲートは3つの量子ビットに適用する必要があります"),
                        location,
                    ));
                }
                
                // 全ての対象が量子型であることを確認
                for &target in targets {
                    if !self.is_quantum_type(target) {
                        return Err(CompilerError::new(
                            ErrorKind::TypeSystem,
                            format!("非量子型に対する量子操作"),
                            location,
                        ));
                    }
                }
            },
            
            QuantumOperation::Measure => {
                // 測定操作
                if targets.len() < 1 {
                    return Err(CompilerError::new(
                        ErrorKind::TypeSystem,
                        format!("測定操作には少なくとも1つの量子ビットが必要です"),
                        location,
                    ));
                }
                
                // 第1引数は量子型、第2引数は古典型であることを確認
                if !self.is_quantum_type(targets[0]) {
                    return Err(CompilerError::new(
                        ErrorKind::TypeSystem,
                        format!("非量子型に対する測定操作"),
                        location,
                    ));
                }
                
                if targets.len() > 1 {
                    let result_type_id = targets[1];
                    let result_ty = self.type_registry.resolve(result_type_id);
                    
                    // 結果格納先が適切な型（bool、intなど）であることを確認
                    match &*result_ty {
                        Type::Builtin(_) => {
                            // 基本型はOK
                        },
                        _ => {
                            // その他の型は許可しない
                            return Err(CompilerError::new(
                                ErrorKind::TypeSystem,
                                format!("測定結果の格納先が不適切な型です"),
                                location,
                            ));
                        }
                    }
                }
            },
            
            // 他の量子操作も必要に応じてチェック
            
            _ => {
                // 未対応の操作は一旦許可
            }
        }
        
        Ok(())
    }
    
    /// 量子回路の型チェック
    pub fn check_quantum_circuit(&mut self, circuit_name: Symbol, location: SourceLocation) -> Result<()> {
        if let Some(circuit) = self.circuits.get(&circuit_name) {
            // 回路内の各操作をチェック
            for (op, qubit_indices) in &circuit.operations {
                // 操作対象の量子ビットが範囲内かチェック
                for &idx in qubit_indices {
                    if idx >= circuit.input_qubits.len() + circuit.ancilla_qubits.len() {
                        return Err(CompilerError::new(
                            ErrorKind::TypeSystem,
                            format!("量子回路内の無効な量子ビットインデックス: {}", idx),
                            location,
                        ));
                    }
                }
                
                // 操作対象の量子ビットのIDを取得
                let mut target_qubits = Vec::new();
                for &idx in qubit_indices {
                    let qubit_id = if idx < circuit.input_qubits.len() {
                        circuit.input_qubits[idx]
                    } else {
                        circuit.ancilla_qubits[idx - circuit.input_qubits.len()]
                    };
                    target_qubits.push(qubit_id);
                }
                
                // 量子操作のチェック
                self.check_quantum_operation(op, &target_qubits, location)?;
            }
            
            Ok(())
        } else {
            Err(CompilerError::new(
                ErrorKind::TypeSystem,
                format!("未定義の量子回路: '{}'", circuit_name),
                location,
            ))
        }
    }
    
    /// 式の量子型チェック
    pub fn check_expr(&mut self, expr: &Expr) -> Result<Option<TypeId>> {
        match &expr.kind {
            ExprKind::Call { function, args } => {
                // 関数呼び出しの場合、量子操作かどうかをチェック
                if let ExprKind::Variable(func_name) = &function.kind {
                    let func_str = func_name.to_string();
                    
                    // 量子操作の関数名をチェック
                    let quantum_op = match func_str.as_str() {
                        "x" | "pauliX" => Some(QuantumOperation::PauliX),
                        "y" | "pauliY" => Some(QuantumOperation::PauliY),
                        "z" | "pauliZ" => Some(QuantumOperation::PauliZ),
                        "h" | "hadamard" => Some(QuantumOperation::Hadamard),
                        "cx" | "cnot" => Some(QuantumOperation::ControlledNot),
                        "cz" => Some(QuantumOperation::ControlledZ),
                        "swap" => Some(QuantumOperation::Swap),
                        "measure" => Some(QuantumOperation::Measure),
                        "reset" => Some(QuantumOperation::Reset),
                        "toffoli" | "ccnot" => Some(QuantumOperation::Toffoli),
                        _ => None,
                    };
                    
                    if let Some(op) = quantum_op {
                        // 引数の型を取得
                        let mut arg_types = Vec::new();
                        for arg in args {
                            if let Some(type_id) = self.check_expr(arg)? {
                                arg_types.push(type_id);
                            } else {
                                return Err(CompilerError::new(
                                    ErrorKind::TypeSystem,
                                    format!("量子操作の引数の型が解決できません"),
                                    arg.location,
                                ));
                            }
                        }
                        
                        // 量子操作の型チェック
                        self.check_quantum_operation(&op, &arg_types, expr.location)?;
                        
                        // 操作の結果型を決定
                        match op {
                            QuantumOperation::Measure => {
                                // 測定操作は古典的な結果を返す
                                if let Ok(bool_type) = self.type_registry.lookup_builtin(self.type_registry::BuiltinType::Bool) {
                                    return Ok(Some(bool_type));
                                }
                            },
                            _ => {
                                // 他の量子操作は量子ビットを返す
                                if args.len() > 0 {
                                    if let Some(type_id) = self.check_expr(&args[0])? {
                                        return Ok(Some(type_id));
                                    }
                                }
                            }
                        }
                    }
                }
            },
            
            ExprKind::Variable(name) => {
                // 変数が量子変数かどうかをチェック
                if let Some(&type_id) = self.quantum_vars.get(name) {
                    return Ok(Some(type_id));
                }
            },
            
            // 他の式タイプも必要に応じてチェック
            
            _ => {
                // 未対応の式タイプは無視
            }
        }
        
        // 量子型ではない場合はNoneを返す
        Ok(None)
    }
    
    /// 量子ビット初期化の型チェック
    pub fn check_qubit_initialization(&mut self, num_qubits: usize, location: SourceLocation) -> Result<TypeId> {
        // 量子ビット型を作成
        let qubit_type = match num_qubits {
            1 => self.type_registry.get_quantum_qubit_type()?,
            _ => self.type_registry.get_quantum_register_type(num_qubits)?,
        };
        
        Ok(qubit_type)
    }
    
    /// エラーのリストを取得
    pub fn get_errors(&self) -> &[QuantumTypeError] {
        &self.errors
    }
}

/// 拡張: TypeRegistryへの量子型関連メソッドの追加
impl TypeRegistry {
    /// 単一量子ビット型を取得
    pub fn get_quantum_qubit_type(&self) -> Result<TypeId> {
        // 量子ビット型が存在しない場合は作成
        let qubit_name = Symbol::intern("Qubit");
        
        // まず既存の型を探す
        for (id, ty) in self.types.read().unwrap().iter() {
            if let Type::Named { name, .. } = &**ty {
                if *name == qubit_name {
                    return Ok(*id);
                }
            }
        }
        
        // 見つからなければ新しく作成
        let type_id = self.register_named_type(qubit_name, Vec::new(), Kind::Type);
        
        Ok(type_id)
    }
    
    /// 量子レジスタ型を取得（指定サイズ）
    pub fn get_quantum_register_type(&self, size: usize) -> Result<TypeId> {
        // 量子ビット型を取得
        let qubit_type = self.get_quantum_qubit_type()?;
        
        // 量子レジスタ型を作成
        let register_type = self.array_type(qubit_type, Some(size));
        
        Ok(register_type)
    }
    
    /// 量子回路型を取得
    pub fn get_quantum_circuit_type(&self) -> Result<TypeId> {
        // 量子回路型が存在しない場合は作成
        let circuit_name = Symbol::intern("QuantumCircuit");
        
        // まず既存の型を探す
        for (id, ty) in self.types.read().unwrap().iter() {
            if let Type::Named { name, .. } = &**ty {
                if *name == circuit_name {
                    return Ok(*id);
                }
            }
        }
        
        // 見つからなければ新しく作成
        let type_id = self.register_named_type(circuit_name, Vec::new(), Kind::Type);
        
        Ok(type_id)
    }
} 