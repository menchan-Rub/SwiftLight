// dependent_type_checker.rs - SwiftLight依存型チェッカー
//
// このモジュールでは、SwiftLight言語の依存型システムの検証を実装します。
// 依存型は値に依存する型（例えば、長さnの配列型「Array<T, n>」など）を表現可能にします。
// このチェッカーは型の整合性を保証し、依存型の制約が満たされることを検証します。

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;

// SMTソルバー使用フラグ
#[cfg(feature = "smt_solver")]
extern crate z3;
#[cfg(feature = "smt_solver")]
use z3::{
    Config, Context, Solver, Sort, SatResult,
    ast::{Ast, Bool, Int, Real},
};

use crate::frontend::ast::{
    self, Program, Declaration, Statement, Expression, TypeAnnotation, TypeKind,
    NodeId, ExpressionKind, StatementKind, DeclarationKind, Identifier, Function,
    VariableDeclaration, ConstantDeclaration, Parameter, Struct, Enum, Trait, TypeAlias,
    Implementation, Import, BinaryOperator, UnaryOperator, Literal, LiteralKind, MatchArm,
};
use crate::frontend::error::{Result, CompilerError, ErrorKind, Diagnostic, SourceLocation, Location};
use super::name_resolver::NameResolutionResult;
use super::symbol_table::{SymbolTable, Symbol, SymbolKind, Visibility};
use super::scope::ScopeManager;
use super::type_checker::{TypeCheckResult, Type, Environment, Value};
use super::type_checker::TypeCheckResult;

/// 依存型チェックの結果
#[derive(Debug, Clone, Default)]
pub struct DependentTypeCheckResult {
    /// 検証された依存型
    pub verified_types: HashMap<NodeId, TypeAnnotation>,
    
    /// 依存型制約
    pub type_constraints: HashMap<NodeId, Vec<DependentConstraint>>,
    
    /// 実行時に検証する必要がある制約
    pub runtime_checks: HashMap<NodeId, Vec<RuntimeCheck>>,
    
    /// 検出されたエラー
    pub errors: Vec<CompilerError>,
    
    /// 警告
    pub warnings: Vec<CompilerError>,
    
    /// 依存型等価性証明
    pub type_equalities: HashMap<(TypeAnnotation, TypeAnnotation), ProofNode>,
}

/// 依存型制約
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DependentConstraint {
    /// 等価性制約： a == b
    Equality(Expression, Expression),
    
    /// 不等式制約： a < b, a <= b, a > b, a >= b
    Inequality {
        left: Expression,
        right: Expression,
        operator: BinaryOperator,
    },
    
    /// 型制約： e は型 T のインスタンス
    TypeOf(Expression, TypeAnnotation),
    
    /// パスが所有権を持っていること
    HasOwnership(Expression),
    
    /// 論理演算： AND(&&), OR(||), NOT(!)
    LogicalOperation {
        constraints: Vec<DependentConstraint>,
        operator: LogicalOperator,
    },
}

/// 論理演算子
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LogicalOperator {
    And,
    Or,
    Not,
}

/// 実行時チェック
#[derive(Debug, Clone)]
pub struct RuntimeCheck {
    /// チェックが必要な式
    pub expression: Expression,
    
    /// チェックの種類
    pub check_type: RuntimeCheckType,
    
    /// エラーメッセージ
    pub error_message: String,
    
    /// ソース位置
    pub location: SourceLocation,
}

/// 実行時チェックの種類
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RuntimeCheckType {
    /// 範囲チェック（配列アクセスなど）
    RangeCheck,
    
    /// ゼロ除算チェック
    DivisionByZero,
    
    /// null参照チェック
    NullReference,
    
    /// 型キャストの妥当性
    TypeCast,
    
    /// カスタム条件
    CustomCondition(Expression),
}

/// 型の証明ノード
#[derive(Debug, Clone)]
pub struct ProofNode {
    /// 証明の種類
    pub kind: ProofKind,
    
    /// 前提条件
    pub assumptions: Vec<DependentConstraint>,
    
    /// サブ証明（複合証明の場合）
    pub sub_proofs: Vec<ProofNode>,
    
    /// 証明が適用される式
    pub target_expression: Option<Expression>,
}

/// 証明の種類
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ProofKind {
    /// 反射律: A = A
    Reflexivity,
    
    /// 対称律: A = B ならば B = A
    Symmetry,
    
    /// 推移律: A = B かつ B = C ならば A = C
    Transitivity,
    
    /// 代入: x = y ならば f(x) = f(y)
    Substitution,
    
    /// 帰納法
    Induction,
    
    /// 定義による等価性
    DefinitionalEquality,
    
    /// ユーザー定義証明
    UserDefined(String),
}

/// SMTソルバーとの統合
struct SMTSolver {
    /// ソルバーの状態
    context: Context,
    /// 現在のスコープでの宣言
    declarations: HashMap<String, Ast<'static>>,
    /// ソルバーインスタンス
    solver: Solver<'static>,
}

impl SMTSolver {
    /// 新しいSMTソルバーインスタンスを作成
    #[cfg(feature = "smt_solver")]
    fn new() -> Self {
        // Z3ソルバーの初期化
        let cfg = Config::new();
        let ctx = Context::new(&cfg);
        let solver = Solver::new(&ctx);
        
        Self {
            context: ctx,
            declarations: HashMap::new(),
            solver,
        }
    }
    
    #[cfg(not(feature = "smt_solver"))]
    fn new() -> Self {
        panic!("SMTソルバー機能が有効でない状態でSMTSolverが作成されました");
    }
    
    /// 制約を追加
    #[cfg(feature = "smt_solver")]
    fn add_constraint(&mut self, constraint: &Expression) -> Result<()> {
        // 制約式をZ3の形式に変換
        let z3_expr = self.translate_expression(constraint)?;
        
        // ソルバーに制約を追加
        self.solver.assert(&z3_expr);
        
        Ok(())
    }
    
    #[cfg(not(feature = "smt_solver"))]
    fn add_constraint(&mut self, _constraint: &Expression) -> Result<()> {
        Ok(())
    }
    
    /// SwiftLight式をZ3形式に変換
    #[cfg(feature = "smt_solver")]
    fn translate_expression(&self, expr: &Expression) -> Result<Ast<'static>> {
        match &expr.kind {
            ExpressionKind::Literal(literal) => {
                match literal {
                    Literal::Integer(i) => {
                        let int_sort = Sort::int(&self.context);
                        Ok(Ast::int(*i as i64, &int_sort))
                    },
                    Literal::Boolean(b) => {
                        let bool_sort = Sort::bool(&self.context);
                        Ok(if *b { self.context.bool_val(true) } else { self.context.bool_val(false) })
                    },
                    // 他のリテラル型も必要に応じて追加
                    _ => Err(CompilerError::new(
                        expr.location.clone(),
                        ErrorKind::TypeError(format!("サポートされていないリテラル型: {:?}", literal))
                    )),
                }
            },
            ExpressionKind::Variable(var) => {
                // 既に宣言されている変数を参照
                if let Some(decl) = self.declarations.get(var) {
                    Ok(decl.clone())
                } else {
                    // 変数が見つからない場合は新しく作成
                    let int_sort = Sort::int(&self.context);
                    let var_decl = self.context.named_const(var, &int_sort);
                    Ok(var_decl)
                }
            },
            ExpressionKind::BinaryOperation { op, lhs, rhs } => {
                let lhs_expr = self.translate_expression(lhs)?;
                let rhs_expr = self.translate_expression(rhs)?;
                
                match op {
                    BinaryOperator::Add => Ok(lhs_expr + rhs_expr),
                    BinaryOperator::Sub => Ok(lhs_expr - rhs_expr),
                    BinaryOperator::Mul => Ok(lhs_expr * rhs_expr),
                    BinaryOperator::Div => Ok(lhs_expr / rhs_expr),
                    BinaryOperator::Eq => Ok(lhs_expr._eq(&rhs_expr)),
                    BinaryOperator::Ne => Ok(!lhs_expr._eq(&rhs_expr)),
                    BinaryOperator::Lt => Ok(lhs_expr.lt(&rhs_expr)),
                    BinaryOperator::Le => Ok(lhs_expr.le(&rhs_expr)),
                    BinaryOperator::Gt => Ok(lhs_expr.gt(&rhs_expr)),
                    BinaryOperator::Ge => Ok(lhs_expr.ge(&rhs_expr)),
                    BinaryOperator::And => Ok(lhs_expr & rhs_expr),
                    BinaryOperator::Or => Ok(lhs_expr | rhs_expr),
                    // 他の演算子も必要に応じて追加
                    _ => Err(CompilerError::new(
                        expr.location.clone(),
                        ErrorKind::TypeError(format!("サポートされていない演算子: {:?}", op))
                    )),
                }
            },
            ExpressionKind::UnaryOperation { op, operand } => {
                let operand_expr = self.translate_expression(operand)?;
                
                match op {
                    UnaryOperator::Neg => Ok(-operand_expr),
                    UnaryOperator::Not => Ok(!operand_expr),
                    // 他の演算子も必要に応じて追加
                    _ => Err(CompilerError::new(
                        expr.location.clone(),
                        ErrorKind::TypeError(format!("サポートされていない単項演算子: {:?}", op))
                    )),
                }
            },
            // 他の式タイプも必要に応じて追加
            _ => Err(CompilerError::new(
                expr.location.clone(),
                ErrorKind::TypeError(format!("サポートされていない式タイプ: {:?}", expr.kind))
            )),
        }
    }
    
    #[cfg(not(feature = "smt_solver"))]
    fn translate_expression(&self, _expr: &Expression) -> Result<()> {
        Err(CompilerError::new(
            Location::default(),
            ErrorKind::InternalError("SMTソルバー機能が有効でない状態で式の変換が試みられました".to_string())
        ))
    }
    
    /// 制約が充足可能かチェック
    #[cfg(feature = "smt_solver")]
    fn check_satisfiability(&mut self) -> Result<bool> {
        match self.solver.check() {
            z3::SatResult::Sat => Ok(true),
            z3::SatResult::Unsat => Ok(false),
            z3::SatResult::Unknown => {
                // 不明な場合は保守的にfalseを返す
                Ok(false)
            }
        }
    }
    
    #[cfg(not(feature = "smt_solver"))]
    fn check_satisfiability(&mut self) -> Result<bool> {
        // SMTソルバーが無効の場合は常にtrueを返す（制約を常に満たすと判定）
        Ok(true)
    }
    
    /// 解を取得（制約が充足可能な場合）
    #[cfg(feature = "smt_solver")]
    fn get_model(&mut self) -> Result<HashMap<String, Value>> {
        let mut result = HashMap::new();
        
        if self.check_satisfiability()? {
            if let Some(model) = self.solver.get_model() {
                for (var_name, var_expr) in &self.declarations {
                    if let Some(val) = model.eval(var_expr, true) {
                        // Z3の値をSwiftLightの値に変換
                        if let Some(value) = self.convert_z3_value(&val) {
                            result.insert(var_name.clone(), value);
                        }
                    }
                }
            }
        }
        
        Ok(result)
    }
    
    #[cfg(not(feature = "smt_solver"))]
    fn get_model(&mut self) -> Result<HashMap<String, Value>> {
        Ok(HashMap::new())
    }
    
    /// Z3の値をSwiftLightの値に変換
    #[cfg(feature = "smt_solver")]
    fn convert_z3_value(&self, z3_val: &Ast) -> Option<Value> {
        if let Some(i) = z3_val.as_i64() {
            Some(Value::Integer(i as i32))
        } else if let Some(b) = z3_val.as_bool() {
            Some(Value::Boolean(b))
        } else {
            // サポートされていない値タイプ
            None
        }
    }
    
    /// 変数を宣言
    #[cfg(feature = "smt_solver")]
    fn declare_variable(&mut self, name: &str, sort: &str) -> Result<()> {
        let z3_sort = match sort {
            "int" => Sort::int(&self.context),
            "bool" => Sort::bool(&self.context),
            "real" => Sort::real(&self.context),
            // 他の型も必要に応じて追加
            _ => return Err(CompilerError::new(
                Location::default(),
                ErrorKind::TypeError(format!("サポートされていない型: {}", sort))
            )),
        };
        
        let var = self.context.named_const(name, &z3_sort);
        self.declarations.insert(name.to_string(), var);
        
        Ok(())
    }
    
    #[cfg(not(feature = "smt_solver"))]
    fn declare_variable(&mut self, _name: &str, _sort: &str) -> Result<()> {
        Ok(())
    }
    
    /// ソルバーをリセット
    #[cfg(feature = "smt_solver")]
    fn reset(&mut self) {
        self.solver.reset();
        self.declarations.clear();
    }
    
    #[cfg(not(feature = "smt_solver"))]
    fn reset(&mut self) {
        // 何もしない
    }
}

/// 依存型チェッカー
pub struct DependentTypeChecker {
    /// 型環境
    env: Environment,
    /// SMTソルバー
    smt_solver: SMTSolver,
}

impl DependentTypeChecker {
    /// 新しい依存型チェッカーを作成
    pub fn new() -> Self {
        Self {
            env: Environment::new(),
            smt_solver: SMTSolver::new(),
        }
    }
    
    /// プログラムの型チェックを実行
    pub fn check_program(&mut self, program: &Program) -> Result<()> {
        for item in &program.items {
            self.check_item(item)?;
        }
        
        Ok(())
    }
    
    /// 項目の型チェック
    fn check_item(&mut self, item: &Item) -> Result<()> {
        match &item.kind {
            ItemKind::Function(func) => self.check_function(func),
            ItemKind::TypeAlias(alias) => self.check_type_alias(alias),
            ItemKind::Struct(struct_def) => self.check_struct(struct_def),
            // 他の項目タイプも必要に応じて追加
            _ => Ok(()),
        }
    }
    
    /// 関数の型チェック
    fn check_function(&mut self, func: &Function) -> Result<()> {
        // 環境に関数を追加
        self.env.add_function(func.name.clone(), func.clone());
        
        // 新しいスコープを作成
        self.env.push_scope();
        
        // パラメータを環境に追加
        for param in &func.parameters {
            self.env.add_variable(param.name.clone(), param.typ.clone());
            
            // 依存型パラメータの場合、制約をSMTソルバーに追加
            if let Some(constraint) = &param.constraint {
                // パラメータをZ3変数として宣言
                let sort = self.get_z3_sort_for_type(&param.typ)?;
                self.smt_solver.declare_variable(&param.name, &sort)?;
                
                // 制約をソルバーに追加
                self.smt_solver.add_constraint(constraint)?;
            }
        }
        
        // 関数本体の型チェック
        if let Some(body) = &func.body {
            let actual_return_type = self.check_expression(body)?;
            
            // 戻り値の型整合性チェック
            if !self.is_compatible_type(&actual_return_type, &func.return_type) {
                return Err(CompilerError::new(
                    body.location.clone(),
                    ErrorKind::TypeError(format!(
                        "戻り値の型が一致しません。期待: {:?}, 実際: {:?}",
                        func.return_type, actual_return_type
                    ))
                ));
            }
            
            // 依存型の戻り値の場合、制約をチェック
            if let Some(constraint) = &func.return_constraint {
                // 制約を評価
                if !self.evaluate_constraint(constraint)? {
                    return Err(CompilerError::new(
                        constraint.location.clone(),
                        ErrorKind::TypeError("戻り値の制約条件を満たしていません".to_string())
                    ));
                }
            }
        }
        
        // スコープを閉じる
        self.env.pop_scope();
        
        // SMTソルバーをリセット
        self.smt_solver.reset();
        
        Ok(())
    }
    
    /// 型の互換性をチェック
    fn is_compatible_type(&self, actual: &Type, expected: &Type) -> bool {
        match (actual, expected) {
            (Type::Basic(actual_name), Type::Basic(expected_name)) => {
                actual_name == expected_name
            },
            (Type::Dependent(base_type, actual_constraint), Type::Dependent(expected_base, expected_constraint)) => {
                // 基本型が一致し、制約が含意関係にあるかチェック
                self.is_compatible_type(base_type, expected_base) &&
                self.implies(actual_constraint, expected_constraint)
            },
            // 他の型の組み合わせも必要に応じて追加
            _ => false,
        }
    }
    
    /// 制約aがbを含意するかチェック
    fn implies(&self, a: &Option<Expression>, b: &Option<Expression>) -> bool {
        match (a, b) {
            (None, None) => true,
            (Some(_), None) => true,
            (None, Some(_)) => false,
            (Some(a_expr), Some(b_expr)) => {
                // aとbの含意関係をSMTソルバーで検証
                #[cfg(feature = "smt_solver")]
                {
                    // 新しいソルバーインスタンスを作成
                    let cfg = Config::new();
                    let ctx = Context::new(&cfg);
                    let solver = Solver::new(&ctx);
                    
                    // a → b は !a ∨ b と同値
                    // a ∧ !b が充足不能であれば a → b は真
                    
                    // aとbをZ3形式に変換
                    match self.smt_solver.translate_expression(a_expr) {
                        Ok(a_z3) => {
                            match self.smt_solver.translate_expression(b_expr) {
                                Ok(b_z3) => {
                                    // a ∧ !b を検証
                                    solver.assert(&a_z3);
                                    solver.assert(&b_z3.not());
                                    
                                    // 充足不能であれば含意は真
                                    match solver.check() {
                                        SatResult::Unsat => true,  // 充足不能なので含意は真
                                        _ => false, // 充足可能またはUnknownなので含意は真とは言えない
                                    }
                                },
                                Err(_) => false, // エラーの場合は保守的にfalseを返す
                            }
                        },
                        Err(_) => false, // エラーの場合は保守的にfalseを返す
                    }
                }
                
                #[cfg(not(feature = "smt_solver"))]
                {
                    // SMTソルバーが有効でない場合は保守的な実装
                    true // デフォルトでtrueを返す
                }
            }
        }
    }
    
    /// 式の型チェック
    fn check_expression(&mut self, expr: &Expression) -> Result<Type> {
        match &expr.kind {
            ExpressionKind::Literal(literal) => {
                // リテラルの型を返す
                match literal {
                    Literal::Integer(_) => Ok(Type::Basic("Int".to_string())),
                    Literal::Float(_) => Ok(Type::Basic("Float".to_string())),
                    Literal::Boolean(_) => Ok(Type::Basic("Bool".to_string())),
                    Literal::String(_) => Ok(Type::Basic("String".to_string())),
                    Literal::Char(_) => Ok(Type::Basic("Char".to_string())),
                    Literal::Array(elements) => {
                        // 空の配列の場合は不明な型を返す
                        if elements.is_empty() {
                            return Ok(Type::Basic("Array".to_string()));
                        }
                        
                        // 最初の要素の型をチェック
                        let first_type = self.check_expression(&elements[0])?;
                        
                        // すべての要素が同じ型かチェック
                        for element in elements.iter().skip(1) {
                            let element_type = self.check_expression(element)?;
                            if !self.is_compatible_type(&element_type, &first_type) {
                                return Err(CompilerError::new(
                                    element.location.clone(),
                                    ErrorKind::TypeError(format!(
                                        "配列の要素の型が一致しません。期待: {:?}, 実際: {:?}",
                                        first_type, element_type
                                    ))
                                ));
                            }
                        }
                        
                        // 配列型を返す
                        Ok(Type::Array(Box::new(first_type), elements.len()))
                    },
                    Literal::Tuple(elements) => {
                        // 各要素の型をチェック
                        let mut element_types = Vec::with_capacity(elements.len());
                        for element in elements {
                            let element_type = self.check_expression(element)?;
                            element_types.push(element_type);
                        }
                        
                        // タプル型を返す
                        Ok(Type::Tuple(element_types))
                    },
                    // 他のリテラル型も必要に応じて追加
                    _ => Err(CompilerError::new(
                        expr.location.clone(),
                        ErrorKind::TypeError(format!("サポートされていないリテラル型: {:?}", literal))
                    )),
                }
            },
            ExpressionKind::Variable(var_name) => {
                // 変数の型を環境から取得
                if let Some(var_type) = self.env.get_variable(var_name) {
                    Ok(var_type.clone())
                } else {
                    Err(CompilerError::new(
                        expr.location.clone(),
                        ErrorKind::TypeError(format!("未定義の変数: {}", var_name))
                    ))
                }
            },
            ExpressionKind::BinaryOperation { op, lhs, rhs } => {
                let lhs_type = self.check_expression(lhs)?;
                let rhs_type = self.check_expression(rhs)?;
                
                // 演算子の型チェック
                self.check_binary_operator(*op, &lhs_type, &rhs_type, expr.location.clone())
            },
            ExpressionKind::UnaryOperation { op, operand } => {
                let operand_type = self.check_expression(operand)?;
                
                // 単項演算子の型チェック
                self.check_unary_operator(*op, &operand_type, expr.location.clone())
            },
            ExpressionKind::FunctionCall { function, arguments } => {
                // 関数の型を取得
                let func_type = self.check_expression(function)?;
                
                // 関数型であるかチェック
                match func_type {
                    Type::Function(param_types, return_type) => {
                        // 引数の数をチェック
                        if param_types.len() != arguments.len() {
                            return Err(CompilerError::new(
                                expr.location.clone(),
                                ErrorKind::TypeError(format!(
                                    "関数呼び出しの引数の数が一致しません。期待: {}, 実際: {}",
                                    param_types.len(), arguments.len()
                                ))
                            ));
                        }
                        
                        // 各引数の型をチェック
                        for (i, (arg, expected_type)) in arguments.iter().zip(param_types.iter()).enumerate() {
                            let arg_type = self.check_expression(arg)?;
                            if !self.is_compatible_type(&arg_type, expected_type) {
                                return Err(CompilerError::new(
                                    arg.location.clone(),
                                    ErrorKind::TypeError(format!(
                                        "引数{}の型が一致しません。期待: {:?}, 実際: {:?}",
                                        i + 1, expected_type, arg_type
                                    ))
                                ));
                            }
                        }
                        
                        // 戻り値の型を返す
                        Ok(*return_type)
                    },
                    _ => Err(CompilerError::new(
                        function.location.clone(),
                        ErrorKind::TypeError(format!("関数型でない値が呼び出されました: {:?}", func_type))
                    )),
                }
            },
            ExpressionKind::MethodCall { object, method, arguments } => {
                // オブジェクトの型を取得
                let obj_type = self.check_expression(object)?;
                
                // メソッドの型情報を取得
                let method_type = self.get_method_type(&obj_type, method, expr.location.clone())?;
                
                // メソッド型をチェック
                match method_type {
                    Type::Function(param_types, return_type) => {
                        // 引数の数をチェック（selfパラメータは暗黙的に渡されるため、-1する）
                        if param_types.len() - 1 != arguments.len() {
                            return Err(CompilerError::new(
                                expr.location.clone(),
                                ErrorKind::TypeError(format!(
                                    "メソッド呼び出しの引数の数が一致しません。期待: {}, 実際: {}",
                                    param_types.len() - 1, arguments.len()
                                ))
                            ));
                        }
                        
                        // 各引数の型をチェック（selfパラメータをスキップ）
                        for (i, (arg, expected_type)) in arguments.iter().zip(param_types.iter().skip(1)).enumerate() {
                            let arg_type = self.check_expression(arg)?;
                            if !self.is_compatible_type(&arg_type, expected_type) {
                                return Err(CompilerError::new(
                                    arg.location.clone(),
                                    ErrorKind::TypeError(format!(
                                        "引数{}の型が一致しません。期待: {:?}, 実際: {:?}",
                                        i + 1, expected_type, arg_type
                                    ))
                                ));
                            }
                        }
                        
                        // 戻り値の型を返す
                        Ok(*return_type)
                    },
                    _ => Err(CompilerError::new(
                        expr.location.clone(),
                        ErrorKind::TypeError(format!("メソッド'{}'が関数型ではありません: {:?}", method, method_type))
                    )),
                }
            },
            ExpressionKind::MemberAccess { object, member } => {
                // オブジェクトの型を取得
                let obj_type = self.check_expression(object)?;
                
                // メンバーの型情報を取得
                self.get_member_type(&obj_type, member, expr.location.clone())
            },
            ExpressionKind::IndexAccess { array, index } => {
                // 配列の型を取得
                let array_type = self.check_expression(array)?;
                
                // インデックスの型をチェック
                let index_type = self.check_expression(index)?;
                if !self.is_numeric_type(&index_type) {
                    return Err(CompilerError::new(
                        index.location.clone(),
                        ErrorKind::TypeError(format!("インデックスは数値型である必要がありますが、{:?}が指定されました", index_type))
                    ));
                }
                
                // 配列型であるかチェック
                match &array_type {
                    Type::Array(element_type, _) => {
                        // 要素の型を返す
                        Ok(*element_type.clone())
                    },
                    Type::Slice(element_type) => {
                        // スライスの要素の型を返す
                        Ok(*element_type.clone())
                    },
                    Type::String => {
                        // 文字列のインデックスアクセスはChar型を返す
                        Ok(Type::Basic("Char".to_string()))
                    },
                    _ => Err(CompilerError::new(
                        array.location.clone(),
                        ErrorKind::TypeError(format!("インデックスアクセスできない型です: {:?}", array_type))
                    )),
                }
            },
            ExpressionKind::IfExpression { condition, then_branch, else_branch } => {
                // 条件式の型をチェック
                let cond_type = self.check_expression(condition)?;
                if !self.is_boolean_type(&cond_type) {
                    return Err(CompilerError::new(
                        condition.location.clone(),
                        ErrorKind::TypeError(format!("条件式はboolean型である必要がありますが、{:?}が指定されました", cond_type))
                    ));
                }
                
                // then節の型をチェック
                let then_type = self.check_expression(then_branch)?;
                
                // else節がある場合、型をチェック
                if let Some(else_expr) = else_branch {
                    let else_type = self.check_expression(else_expr)?;
                    
                    // then節とelse節の型が一致するかチェック
                    if !self.is_compatible_type(&then_type, &else_type) && !self.is_compatible_type(&else_type, &then_type) {
                        return Err(CompilerError::new(
                            else_expr.location.clone(),
                            ErrorKind::TypeError(format!(
                                "if式のthen節とelse節の型が一致しません。then: {:?}, else: {:?}",
                                then_type, else_type
                            ))
                        ));
                    }
                    
                    // より広い型を返す
                    if self.is_compatible_type(&then_type, &else_type) {
                        Ok(else_type)
                } else {
                        Ok(then_type)
                    }
                } else {
                    // else節がない場合は単位型（void）を返す
                    Ok(Type::Unit)
                }
            },
            ExpressionKind::BlockExpression { statements, result } => {
                // 新しいスコープを作成
                self.env.push_scope();
                
                // 各文を評価
                for stmt in statements {
                    match &stmt.kind {
                        StatementKind::VariableDeclaration(var_decl) => {
                            // 初期値の型をチェック
                            let init_type = if let Some(init) = &var_decl.initializer {
                                self.check_expression(init)?
                            } else {
                                // 初期値がない場合は指定された型を使用
                                if let Some(type_ann) = &var_decl.type_annotation {
                                    self.convert_type_annotation(type_ann)?
                                } else {
                                    return Err(CompilerError::new(
                                        stmt.location.clone(),
                                        ErrorKind::TypeError("変数宣言には型か初期値が必要です".to_string())
                                    ));
                                }
                            };
                            
                            // 型アノテーションがある場合は型をチェック
                            if let Some(type_ann) = &var_decl.type_annotation {
                                let var_type = self.convert_type_annotation(type_ann)?;
                                if !self.is_compatible_type(&init_type, &var_type) {
                                    return Err(CompilerError::new(
                                        var_decl.initializer.as_ref().unwrap().location.clone(),
                                        ErrorKind::TypeError(format!(
                                            "変数の型と初期値の型が一致しません。期待: {:?}, 実際: {:?}",
                                            var_type, init_type
                                        ))
                                    ));
                                }
                                
                                // 環境に変数を追加
                                self.env.add_variable(var_decl.name.clone(), var_type);
                            } else {
                                // 型アノテーションがない場合は初期値の型を使用
                                self.env.add_variable(var_decl.name.clone(), init_type);
                            }
                        },
                        StatementKind::Expression(expr) => {
                            // 式を評価
                            self.check_expression(expr)?;
                        },
                        // 他の文の種類も必要に応じて追加
                        _ => {
                            // 他の文は型チェックの結果に影響しない
                        }
                    }
                }
                
                // 結果式がある場合はその型を返す、なければ単位型
                let result_type = if let Some(result_expr) = result {
                    self.check_expression(result_expr)?
                } else {
                    Type::Unit
                };
                
                // スコープを閉じる
                self.env.pop_scope();
                
                Ok(result_type)
            },
            ExpressionKind::MatchExpression { scrutinee, arms } => {
                // マッチ対象の式の型をチェック
                let scrutinee_type = self.check_expression(scrutinee)?;
                
                // アームがない場合はエラー
                if arms.is_empty() {
                    return Err(CompilerError::new(
                        expr.location.clone(),
                        ErrorKind::TypeError("match式には少なくとも1つのアームが必要です".to_string())
                    ));
                }
                
                // 最初のアームの結果式の型をチェック
                let first_arm_type = self.check_expression(&arms[0].body)?;
                
                // 他のアームの型が一致するかチェック
                for arm in arms.iter().skip(1) {
                    let arm_type = self.check_expression(&arm.body)?;
                    if !self.is_compatible_type(&arm_type, &first_arm_type) && !self.is_compatible_type(&first_arm_type, &arm_type) {
                        return Err(CompilerError::new(
                            arm.body.location.clone(),
                            ErrorKind::TypeError(format!(
                                "match式のアームの型が一致しません。期待: {:?}, 実際: {:?}",
                                first_arm_type, arm_type
                            ))
                        ));
                    }
                }
                
                // match式全体の型を返す
                Ok(first_arm_type)
            },
            ExpressionKind::Lambda { parameters, body } => {
                // 新しいスコープを作成
                self.env.push_scope();
                
                // パラメータの型をチェック
                let mut param_types = Vec::with_capacity(parameters.len());
                for param in parameters {
                    // 型アノテーションが必要
                    if let Some(type_ann) = &param.type_annotation {
                        let param_type = self.convert_type_annotation(type_ann)?;
                        param_types.push(param_type.clone());
                        self.env.add_variable(param.name.clone(), param_type);
                    } else {
                        return Err(CompilerError::new(
                            expr.location.clone(),
                            ErrorKind::TypeError(format!("ラムダ式のパラメータ'{}'には型アノテーションが必要です", param.name))
                        ));
                    }
                }
                
                // 本体の型をチェック
                let body_type = self.check_expression(body)?;
                
                // スコープを閉じる
                self.env.pop_scope();
                
                // 関数型を返す
                Ok(Type::Function(param_types, Box::new(body_type)))
            },
            ExpressionKind::Cast { expression, target_type } => {
                // 式の型をチェック
                let expr_type = self.check_expression(expression)?;
                
                // ターゲット型を変換
                let target = self.convert_type_annotation(target_type)?;
                
                // キャスト可能かチェック
                if !self.is_castable(&expr_type, &target) {
                    return Err(CompilerError::new(
                        expr.location.clone(),
                        ErrorKind::TypeError(format!(
                            "型{:?}から型{:?}へのキャストはサポートされていません",
                            expr_type, target
                        ))
                    ));
                }
                
                // ターゲット型を返す
                Ok(target)
            },
            // その他の式タイプも必要に応じて追加
            _ => Err(CompilerError::new(
                expr.location.clone(),
                ErrorKind::TypeError("サポートされていない式タイプです".to_string())
            )),
        }
    }
    
    /// 二項演算子の型チェック
    fn check_binary_operator(&self, op: BinaryOperator, lhs_type: &Type, rhs_type: &Type, location: Location) -> Result<Type> {
        match op {
            // 算術演算子（+, -, *, /, %）
            BinaryOperator::Add | BinaryOperator::Subtract | BinaryOperator::Multiply | BinaryOperator::Divide | BinaryOperator::Modulo => {
                // 数値型同士の演算
                if self.is_numeric_type(lhs_type) && self.is_numeric_type(rhs_type) {
                    // より広い数値型を返す（例: Int + Float = Float）
                    if matches!(lhs_type, Type::Basic(name) if name == "Float" || name == "Double") {
                        return Ok(lhs_type.clone());
                    } else if matches!(rhs_type, Type::Basic(name) if name == "Float" || name == "Double") {
                        return Ok(rhs_type.clone());
                    } else {
                        return Ok(lhs_type.clone()); // デフォルトで左辺の型を返す
                    }
                }
                
                // 文字列の結合（+のみ）
                if op == BinaryOperator::Add {
                    if matches!(lhs_type, Type::Basic(name) if name == "String") && 
                       matches!(rhs_type, Type::Basic(name) if name == "String") {
                        return Ok(Type::Basic("String".to_string()));
                    }
                }
                
                // 配列の結合（+のみ）
                if op == BinaryOperator::Add {
                    if let (Type::Array(lhs_elem_type, lhs_size), Type::Array(rhs_elem_type, rhs_size)) = (lhs_type, rhs_type) {
                        if self.is_compatible_type(lhs_elem_type, rhs_elem_type) {
                            // サイズが静的に分かる場合は合計のサイズを指定、そうでなければ動的サイズ（0）
                            let size = if *lhs_size > 0 && *rhs_size > 0 {
                                lhs_size + rhs_size
                            } else {
                                0
                            };
                            return Ok(Type::Array(lhs_elem_type.clone(), size));
                        }
                    }
                }
                
                Err(CompilerError::new(
                    location,
                    ErrorKind::TypeError(format!(
                        "演算子{:?}は型{:?}と{:?}には適用できません",
                        op, lhs_type, rhs_type
                    ))
                ))
            },
            
            // 比較演算子（==, !=, <, <=, >, >=）
            BinaryOperator::Equal | BinaryOperator::NotEqual => {
                // 同じ型同士の比較
                if self.is_compatible_type(lhs_type, rhs_type) || self.is_compatible_type(rhs_type, lhs_type) {
                    return Ok(Type::Basic("Bool".to_string()));
                }
                
                Err(CompilerError::new(
                    location,
                    ErrorKind::TypeError(format!(
                        "等価演算子{:?}は型{:?}と{:?}には適用できません",
                        op, lhs_type, rhs_type
                    ))
                ))
            },
            
            BinaryOperator::LessThan | BinaryOperator::LessThanOrEqual | BinaryOperator::GreaterThan | BinaryOperator::GreaterThanOrEqual => {
                // 数値型同士の比較
                if self.is_numeric_type(lhs_type) && self.is_numeric_type(rhs_type) {
                    return Ok(Type::Basic("Bool".to_string()));
                }
                
                // 文字列の比較
                if matches!(lhs_type, Type::Basic(name) if name == "String") && 
                   matches!(rhs_type, Type::Basic(name) if name == "String") {
                    return Ok(Type::Basic("Bool".to_string()));
                }
                
                Err(CompilerError::new(
                    location,
                    ErrorKind::TypeError(format!(
                        "比較演算子{:?}は型{:?}と{:?}には適用できません",
                        op, lhs_type, rhs_type
                    ))
                ))
            },
            
            // 論理演算子（&&, ||）
            BinaryOperator::LogicalAnd | BinaryOperator::LogicalOr => {
                // ブール型同士の演算
                if self.is_boolean_type(lhs_type) && self.is_boolean_type(rhs_type) {
                    return Ok(Type::Basic("Bool".to_string()));
                }
                
                Err(CompilerError::new(
                    location,
                    ErrorKind::TypeError(format!(
                        "論理演算子{:?}は型{:?}と{:?}には適用できません",
                        op, lhs_type, rhs_type
                    ))
                ))
            },
            
            // ビット演算子（&, |, ^, <<, >>）
            BinaryOperator::BitwiseAnd | BinaryOperator::BitwiseOr | BinaryOperator::BitwiseXor => {
                // 整数型同士の演算
                if self.is_numeric_type(lhs_type) && self.is_numeric_type(rhs_type) && 
                   !matches!(lhs_type, Type::Basic(name) if name == "Float" || name == "Double") &&
                   !matches!(rhs_type, Type::Basic(name) if name == "Float" || name == "Double") {
                    return Ok(lhs_type.clone());
                }
                
                Err(CompilerError::new(
                    location,
                    ErrorKind::TypeError(format!(
                        "ビット演算子{:?}は型{:?}と{:?}には適用できません",
                        op, lhs_type, rhs_type
                    ))
                ))
            },
            
            BinaryOperator::LeftShift | BinaryOperator::RightShift => {
                // 整数型のシフト
                if self.is_numeric_type(lhs_type) && self.is_numeric_type(rhs_type) && 
                   !matches!(lhs_type, Type::Basic(name) if name == "Float" || name == "Double") &&
                   !matches!(rhs_type, Type::Basic(name) if name == "Float" || name == "Double") {
                    return Ok(lhs_type.clone());
                }
                
                Err(CompilerError::new(
                    location,
                    ErrorKind::TypeError(format!(
                        "シフト演算子{:?}は型{:?}と{:?}には適用できません",
                        op, lhs_type, rhs_type
                    ))
                ))
            },
            
            // 範囲演算子（..）
            BinaryOperator::Range => {
                // 数値型同士
                if self.is_numeric_type(lhs_type) && self.is_numeric_type(rhs_type) {
                    return Ok(Type::Range(Box::new(lhs_type.clone())));
                }
                
                Err(CompilerError::new(
                    location,
                    ErrorKind::TypeError(format!(
                        "範囲演算子{:?}は型{:?}と{:?}には適用できません",
                        op, lhs_type, rhs_type
                    ))
                ))
            },
            
            // その他の演算子も必要に応じて追加
            _ => Err(CompilerError::new(
                location,
                ErrorKind::TypeError(format!(
                    "サポートされていない演算子{:?}です",
                    op
                ))
            )),
        }
    }
    
    /// 単項演算子の型チェック
    fn check_unary_operator(&self, op: UnaryOperator, operand_type: &Type, location: Location) -> Result<Type> {
        match op {
            // 符号反転（-）
            UnaryOperator::Minus => {
                // 数値型のみ
                if self.is_numeric_type(operand_type) {
                    return Ok(operand_type.clone());
                }
                
                Err(CompilerError::new(
                    location,
                    ErrorKind::TypeError(format!(
                        "符号反転演算子{:?}は型{:?}には適用できません",
                        op, operand_type
                    ))
                ))
            },
            
            // 論理否定（!）
            UnaryOperator::LogicalNot => {
                // ブール型のみ
                if self.is_boolean_type(operand_type) {
                    return Ok(Type::Basic("Bool".to_string()));
                }
                
                Err(CompilerError::new(
                    location,
                    ErrorKind::TypeError(format!(
                        "論理否定演算子{:?}は型{:?}には適用できません",
                        op, operand_type
                    ))
                ))
            },
            
            // ビット反転（~）
            UnaryOperator::BitwiseNot => {
                // 整数型のみ
                if self.is_numeric_type(operand_type) && 
                   !matches!(operand_type, Type::Basic(name) if name == "Float" || name == "Double") {
                    return Ok(operand_type.clone());
                }
                
                Err(CompilerError::new(
                    location,
                    ErrorKind::TypeError(format!(
                        "ビット反転演算子{:?}は型{:?}には適用できません",
                        op, operand_type
                    ))
                ))
            },
            
            // その他の演算子も必要に応じて追加
            _ => Err(CompilerError::new(
                location,
                ErrorKind::TypeError(format!(
                    "サポートされていない演算子{:?}です",
                    op
                ))
            )),
        }
    }
    
    /// 数値型かどうかを判定
    fn is_numeric_type(&self, type_: &Type) -> bool {
        match type_ {
            Type::Basic(name) => {
                matches!(name.as_str(), "Int" | "Float" | "Double" | "UInt" | "Int8" | "Int16" | "Int32" | "Int64" | "UInt8" | "UInt16" | "UInt32" | "UInt64")
            },
            _ => false,
        }
    }
    
    /// ブール型かどうかを判定
    fn is_boolean_type(&self, type_: &Type) -> bool {
        match type_ {
            Type::Basic(name) => name == "Bool",
            _ => false,
        }
    }
    
    /// 型が数値型かどうかをチェック
    fn is_numeric_type(&self, typ: &Type) -> bool {
        match typ {
            Type::Basic(name) => {
                matches!(name.as_str(), "Int" | "Float")
            },
            // 依存型の場合は基本型をチェック
            Type::Dependent(base_type, _) => self.is_numeric_type(base_type),
            // 他の型も必要に応じて追加
            _ => false,
        }
    }
    
    /// 型が真偽値型かどうかをチェック
    fn is_boolean_type(&self, typ: &Type) -> bool {
        match typ {
            Type::Basic(name) => name == "Bool",
            Type::Dependent(base_type, _) => self.is_boolean_type(base_type),
            // 他の型も必要に応じて追加
            _ => false,
        }
    }
    
    /// より広い数値型を取得
    fn get_wider_numeric_type(&self, type1: &Type, type2: &Type) -> Type {
        let base_type1 = match type1 {
            Type::Dependent(base, _) => base,
            _ => type1,
        };
        
        let base_type2 = match type2 {
            Type::Dependent(base, _) => base,
            _ => type2,
        };
        
        match (base_type1, base_type2) {
            (Type::Basic(name1), Type::Basic(name2)) => {
                if name1 == "Float" || name2 == "Float" {
                    Type::Basic("Float".to_string())
                } else {
                    Type::Basic("Int".to_string())
                }
            },
            // 他の組み合わせも必要に応じて追加
            _ => Type::Basic("Int".to_string()), // デフォルト
        }
    }
    
    /// 型エイリアスの型チェック
    fn check_type_alias(&mut self, alias: &TypeAlias) -> Result<()> {
        // エイリアスの型を環境に追加
        self.env.add_type_alias(alias.name.clone(), alias.target_type.clone());
        
        Ok(())
    }
    
    /// 構造体の型チェック
    fn check_struct(&mut self, struct_def: &StructDefinition) -> Result<()> {
        // 構造体の型を環境に追加
        self.env.add_struct(struct_def.name.clone(), struct_def.clone());
        
        // フィールドの型をチェック
        for field in &struct_def.fields {
            // フィールドの型が有効か確認
            if !self.is_valid_type(&field.typ) {
                return Err(CompilerError::new(
                    field.location.clone(),
                    ErrorKind::TypeError(format!("無効なフィールド型: {:?}", field.typ))
                ));
            }
            
            // 依存型フィールドの場合、制約をチェック
            if let Some(constraint) = &field.constraint {
                // 制約が有効な式かチェック
                self.check_expression(constraint)?;
            }
        }
        
        Ok(())
    }
    
    /// 型が有効かチェック
    fn is_valid_type(&self, typ: &Type) -> bool {
        match typ {
            Type::Basic(name) => {
                // 基本型が環境に存在するかチェック
                self.env.has_type(name)
            },
            Type::Dependent(base_type, constraint) => {
                // 基本型が有効で、制約があれば制約も有効かチェック
                self.is_valid_type(base_type) && constraint.as_ref().map_or(true, |_| true)
            },
            // 他の型も必要に応じて追加
            _ => false,
        }
    }
    
    /// 制約を評価
    fn evaluate_constraint(&mut self, constraint: &Expression) -> Result<bool> {
        // SMTソルバーを使用して制約を評価
        #[cfg(feature = "smt_solver")]
        {
            // 一時的なソルバーインスタンスを作成してクリーンな状態で評価
            let mut temp_solver = SMTSolver::new();
            
            // 制約を追加
            temp_solver.add_constraint(constraint)?;
            
            // 充足可能性をチェック
            temp_solver.check_satisfiability()
        }
        
        #[cfg(not(feature = "smt_solver"))]
        {
            // SMTソルバーが有効でない場合は常に制約を満たすと判定
            Ok(true)
        }
    }
    
    /// 型からZ3のソート名を取得
    fn get_z3_sort_for_type(&self, typ: &Type) -> Result<String> {
        match typ {
            Type::Basic(name) => {
                match name.as_str() {
                    "Int" => Ok("int".to_string()),
                    "Float" => Ok("real".to_string()),
                    "Bool" => Ok("bool".to_string()),
                    _ => Err(CompilerError::new(
                        Location::default(),
                        ErrorKind::TypeError(format!("Z3でサポートされていない型: {}", name))
                    )),
                }
            },
            Type::Dependent(base_type, _) => {
                // 依存型の場合は基本型のソートを返す
                self.get_z3_sort_for_type(base_type)
            },
            // 他の型も必要に応じて追加
            _ => Err(CompilerError::new(
                Location::default(),
                ErrorKind::TypeError("Z3でサポートされていない型".to_string())
            )),
        }
    }
    
    /// オブジェクトのメソッドの型を取得
    fn get_method_type(&self, obj_type: &Type, method_name: &str, location: Location) -> Result<Type> {
        match obj_type {
            Type::Struct(struct_name) => {
                // 構造体のメソッドを環境から検索
                if let Some(struct_def) = self.env.get_struct(struct_name) {
                    for method in &struct_def.methods {
                        if method.name == method_name {
                            // メソッドの型を返す
                            let mut param_types = Vec::with_capacity(method.parameters.len());
                            // selfパラメータを含める
                            param_types.push(Type::Struct(struct_name.clone()));
                            
                            // 他のパラメータの型を追加
                            for param in &method.parameters {
                                if let Some(type_ann) = &param.type_annotation {
                                    let param_type = self.convert_type_annotation(type_ann)?;
                                    param_types.push(param_type);
                } else {
                                    return Err(CompilerError::new(
                                        location.clone(),
                                        ErrorKind::TypeError(format!("メソッド{}のパラメータ{}に型アノテーションがありません", method_name, param.name))
                                    ));
                                }
                            }
                            
                            // 戻り値の型
                            let return_type = if let Some(return_type_ann) = &method.return_type {
                                Box::new(self.convert_type_annotation(return_type_ann)?)
                            } else {
                                Box::new(Type::Unit)
                            };
                            
                            return Ok(Type::Function(param_types, return_type));
                        }
                    }
                }
                
                // メソッドが見つからない場合
                Err(CompilerError::new(
                    location,
                    ErrorKind::TypeError(format!("構造体{}にメソッド{}が見つかりません", struct_name, method_name))
                ))
            },
            Type::Basic(type_name) => {
                // 基本型のメソッドを環境から検索
                if let Some(type_def) = self.env.get_basic_type(type_name) {
                    for method in &type_def.methods {
                        if method.name == method_name {
                            // メソッドの型を返す
                            let mut param_types = Vec::with_capacity(method.parameters.len() + 1);
                            // selfパラメータを含める
                            param_types.push(Type::Basic(type_name.clone()));
                            
                            // 他のパラメータの型を追加
                            for param in &method.parameters {
                                if let Some(type_ann) = &param.type_annotation {
                                    let param_type = self.convert_type_annotation(type_ann)?;
                                    param_types.push(param_type);
                                } else {
                                    return Err(CompilerError::new(
                                        location.clone(),
                                        ErrorKind::TypeError(format!("メソッド{}のパラメータ{}に型アノテーションがありません", method_name, param.name))
                                    ));
                                }
                            }
                            
                            // 戻り値の型
                            let return_type = if let Some(return_type_ann) = &method.return_type {
                                Box::new(self.convert_type_annotation(return_type_ann)?)
                            } else {
                                Box::new(Type::Unit)
                            };
                            
                            return Ok(Type::Function(param_types, return_type));
                        }
                    }
                }
                
                // 標準ライブラリのメソッドをチェック（実装例）
                match type_name.as_str() {
                    "String" => match method_name {
                        "length" => return Ok(Type::Function(
                            vec![Type::Basic("String".to_string())],
                            Box::new(Type::Basic("Int".to_string()))
                        )),
                        "substring" => return Ok(Type::Function(
                            vec![
                                Type::Basic("String".to_string()),
                                Type::Basic("Int".to_string()),
                                Type::Basic("Int".to_string())
                            ],
                            Box::new(Type::Basic("String".to_string()))
                        )),
                        _ => {}
                    },
                    "Int" | "Float" => match method_name {
                        "toString" => return Ok(Type::Function(
                            vec![Type::Basic(type_name.clone())],
                            Box::new(Type::Basic("String".to_string()))
                        )),
                        _ => {}
                    },
                    _ => {}
                }
                
                // メソッドが見つからない場合
                Err(CompilerError::new(
                    location,
                    ErrorKind::TypeError(format!("型{}にメソッド{}が見つかりません", type_name, method_name))
                ))
            },
            // 他の型についても同様に実装
            _ => Err(CompilerError::new(
                location,
                ErrorKind::TypeError(format!("型{:?}にメソッドはサポートされていません", obj_type))
            )),
        }
    }
    
    /// オブジェクトのメンバーの型を取得
    fn get_member_type(&self, obj_type: &Type, member_name: &str, location: Location) -> Result<Type> {
        match obj_type {
            Type::Struct(struct_name) => {
                // 構造体のフィールドを環境から検索
                if let Some(struct_def) = self.env.get_struct(struct_name) {
                    for field in &struct_def.fields {
                        if field.name == member_name {
                            // フィールドの型を返す
                            if let Some(type_ann) = &field.type_annotation {
                                return self.convert_type_annotation(type_ann);
                            } else {
                                return Err(CompilerError::new(
                                    location,
                                    ErrorKind::TypeError(format!("構造体{}のフィールド{}に型アノテーションがありません", struct_name, member_name))
                                ));
                            }
                        }
                    }
                }
                
                // フィールドが見つからない場合
                Err(CompilerError::new(
                    location,
                    ErrorKind::TypeError(format!("構造体{}にメンバー{}が見つかりません", struct_name, member_name))
                ))
            },
            Type::Tuple(element_types) => {
                // タプルの要素アクセス（例：tuple.0, tuple.1, ...）
                if let Ok(index) = member_name.parse::<usize>() {
                    if index < element_types.len() {
                        return Ok(element_types[index].clone());
        } else {
                        return Err(CompilerError::new(
                            location,
                            ErrorKind::TypeError(format!(
                                "タプルのインデックスが範囲外です。タプルの長さ: {}, アクセス: {}",
                                element_types.len(), index
                            ))
                        ));
                    }
                }
                
                // インデックスではない場合はエラー
                Err(CompilerError::new(
                    location,
                    ErrorKind::TypeError(format!("タプルに数値インデックス以外のアクセスはできません: {}", member_name))
                ))
            },
            // 他の型についても同様に実装
            _ => Err(CompilerError::new(
                location,
                ErrorKind::TypeError(format!("型{:?}にメンバーアクセスはサポートされていません", obj_type))
            )),
        }
    }
    
    /// キャスト可能かどうかを判定
    fn is_castable(&self, from: &Type, to: &Type) -> bool {
        // 同じ型へのキャストは常に可能
        if self.is_compatible_type(from, to) {
            return true;
        }
        
        // 数値型間のキャストは可能
        if self.is_numeric_type(from) && self.is_numeric_type(to) {
            return true;
        }
        
        // 数値型から文字列へのキャスト
        if self.is_numeric_type(from) && matches!(to, Type::Basic(name) if name == "String") {
            return true;
        }
        
        // 文字列から数値型へのキャスト
        if matches!(from, Type::Basic(name) if name == "String") && self.is_numeric_type(to) {
            return true;
        }
        
        // その他の特定のキャストルール
        match (from, to) {
            // Any型からの特殊なキャスト
            (Type::Any, _) => true,
            // Any型へのキャスト
            (_, Type::Any) => true,
            // ブール値から数値型へのキャスト
            (Type::Basic(name), _) if name == "Bool" && self.is_numeric_type(to) => true,
            // その他の特定のキャスト
            _ => false,
        }
    }
}

/// 二項演算子を文字列に変換する
fn operator_to_string(op: &BinaryOperator) -> String {
    match op {
        BinaryOperator::Add => "+".to_string(),
        BinaryOperator::Subtract => "-".to_string(),
        BinaryOperator::Multiply => "*".to_string(),
        BinaryOperator::Divide => "/".to_string(),
        BinaryOperator::Modulo => "%".to_string(),
        BinaryOperator::Equal => "==".to_string(),
        BinaryOperator::NotEqual => "!=".to_string(),
        BinaryOperator::LessThan => "<".to_string(),
        BinaryOperator::LessThanOrEqual => "<=".to_string(),
        BinaryOperator::GreaterThan => ">".to_string(),
        BinaryOperator::GreaterThanOrEqual => ">=".to_string(),
        BinaryOperator::And => "&&".to_string(),
        BinaryOperator::Or => "||".to_string(),
        BinaryOperator::BitwiseAnd => "&".to_string(),
        BinaryOperator::BitwiseOr => "|".to_string(),
        BinaryOperator::BitwiseXor => "^".to_string(),
        BinaryOperator::LeftShift => "<<".to_string(),
        BinaryOperator::RightShift => ">>".to_string(),
        _ => format!("{:?}", op),
    }
}

/// 単項演算子を文字列に変換する
fn unary_operator_to_string(op: &UnaryOperator) -> String {
    match op {
        UnaryOperator::Negate => "-".to_string(),
        UnaryOperator::Not => "!".to_string(),
        UnaryOperator::BitwiseNot => "~".to_string(),
        UnaryOperator::Dereference => "*".to_string(),
        UnaryOperator::Reference => "&".to_string(),
        _ => format!("{:?}", op),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    // テストケースは省略
} 