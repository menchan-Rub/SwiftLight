/*
 * ownership_checker.rs - SwiftLight 所有権/借用チェック実装
 *
 * このモジュールでは、SwiftLightのASTに対して所有権と借用のルールを検証します。
 * 変数宣言、代入、関数宣言、ブロックなどのノードを走査し、
 * 変数が適切に初期化され、所有権が維持されているかを確認します。
 */

use std::collections::{HashMap, HashSet};
use crate::frontend::ast::{
    NodeId, Expression, ExpressionKind, Statement, StatementKind,
    Identifier, TypeAnnotation, Program, Declaration, DeclarationKind,
    VariableDeclaration, Function, Parameter, Block
};
use crate::frontend::error::{Result, CompilerError, ErrorKind, SourceLocation};
use super::symbol_table::SymbolTable;
use super::type_checker::TypeCheckResult;

/// 変数の所有権状態
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OwnershipState {
    /// 変数が初期化されていない
    Uninitialized,
    
    /// 変数が所有権を持っている
    Owned,
    
    /// 変数が移動された（所有権が他に移った）
    Moved,
    
    /// 変数が不変借用されている
    Borrowed(Vec<NodeId>), // 借用している式のID
    
    /// 変数が可変借用されている
    MutableBorrowed(NodeId), // 借用している式のID
    
    /// 変数が参照として所有されている
    OwnedReference(bool), // 可変参照かどうか
}

/// 変数定義のスコープ情報
#[derive(Debug, Clone)]
struct VariableScope {
    /// 変数ID
    id: NodeId,
    
    /// 変数名
    name: String,
    
    /// 変数の型
    type_id: Option<NodeId>,
    
    /// 現在の所有権状態
    state: OwnershipState,
    
    /// 定義された位置
    location: SourceLocation,
}

/// 借用スコープ情報
#[derive(Debug, Clone)]
struct BorrowScope {
    /// 借用元の変数ID
    source_id: NodeId,
    
    /// 借用先の変数ID
    target_id: NodeId,
    
    /// 可変借用かどうか
    is_mutable: bool,
    
    /// 借用の位置
    location: SourceLocation,
}

/// 所有権チェック結果
#[derive(Debug, Clone, Default)]
pub struct OwnershipCheckResult {
    /// 所有権エラー
    pub errors: Vec<CompilerError>,
    
    /// 所有権警告
    pub warnings: Vec<CompilerError>,
    
    /// 借用スコープ情報
    pub borrow_scopes: HashMap<NodeId, (NodeId, NodeId)>, // (借用元, 借用先)
    
    /// 各変数の所有権状態の最終状態
    pub final_states: HashMap<NodeId, OwnershipState>,
}

/// 変数の所有権パス
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum OwnershipPath {
    /// 変数全体
    Root(NodeId),
    
    /// フィールドアクセス (base.field_name)
    Field(Box<OwnershipPath>, String),
    
    /// インデックスアクセス (base[index])
    Index(Box<OwnershipPath>, usize),
    
    /// 参照解除 (*base)
    Dereference(Box<OwnershipPath>),
    
    /// マッピング（所有権パスから派生した変数）
    Projection(Box<OwnershipPath>, NodeId),
}

impl OwnershipPath {
    /// ルートノードIDを取得
    pub fn root_id(&self) -> NodeId {
        match self {
            OwnershipPath::Root(id) => *id,
            OwnershipPath::Field(base, _) => base.root_id(),
            OwnershipPath::Index(base, _) => base.root_id(),
            OwnershipPath::Dereference(base) => base.root_id(),
            OwnershipPath::Projection(base, _) => base.root_id(),
        }
    }
    
    /// 所有権パスの文字列表現を取得
    pub fn to_string(&self) -> String {
        match self {
            OwnershipPath::Root(id) => format!("var_{}", id),
            OwnershipPath::Field(base, field) => format!("{}.{}", base.to_string(), field),
            OwnershipPath::Index(base, index) => format!("{}[{}]", base.to_string(), index),
            OwnershipPath::Dereference(base) => format!("*{}", base.to_string()),
            OwnershipPath::Projection(base, id) => format!("{}#{}", base.to_string(), id),
        }
    }
}

/// 所有権パスの使用情報
#[derive(Debug, Clone)]
struct PathUsage {
    /// 所有権パス
    path: OwnershipPath,
    
    /// 使用の種類
    kind: PathUsageKind,
    
    /// 使用位置
    location: SourceLocation,
    
    /// 使用元のノードID
    node_id: NodeId,
}

/// 所有権パスの使用種類
#[derive(Debug, Clone, PartialEq, Eq)]
enum PathUsageKind {
    /// 読み取り
    Read,
    
    /// 書き込み
    Write,
    
    /// 所有権移動
    Move,
    
    /// 不変借用
    Borrow,
    
    /// 可変借用
    MutableBorrow,
}

/// OwnershipChecker は、所有権と借用のルールを検証するための構造体です。
pub struct OwnershipChecker {
    /// 変数の所有権状態マップ: 変数ID -> 所有権状態
    ownership_states: HashMap<NodeId, OwnershipState>,
    
    /// スコープスタック: スコープレベル -> (変数ID -> 変数スコープ情報)
    scope_stack: Vec<HashMap<NodeId, VariableScope>>,
    
    /// 現在のスコープにある変数: 変数名 -> 変数ID
    current_variables: HashMap<String, NodeId>,
    
    /// 借用スコープ: 借用先ID -> 借用スコープ情報
    borrows: HashMap<NodeId, BorrowScope>,
    
    /// 型チェック結果
    type_check_result: Option<TypeCheckResult>,
    
    /// チェック中に検出されたエラー
    errors: Vec<CompilerError>,
    
    /// チェック中に検出された警告
    warnings: Vec<CompilerError>,
}

impl OwnershipChecker {
    /// 新しい OwnershipChecker を生成します。
    pub fn new(type_check_result: Option<TypeCheckResult>) -> Self {
        Self {
            ownership_states: HashMap::new(),
            scope_stack: vec![HashMap::new()],
            current_variables: HashMap::new(),
            borrows: HashMap::new(),
            type_check_result,
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    /// プログラム全体の所有権チェックを実行します。
    pub fn check_program(&mut self, program: &Program) -> Result<OwnershipCheckResult> {
        // グローバル宣言を処理
        for decl in &program.declarations {
            self.check_declaration(decl)?;
        }
        
        // 結果を構築
        let result = OwnershipCheckResult {
            errors: self.errors.clone(),
            warnings: self.warnings.clone(),
            borrow_scopes: self.borrows.iter()
                .map(|(k, v)| (*k, (v.source_id, v.target_id)))
                .collect(),
            final_states: self.ownership_states.clone(),
        };
        
        Ok(result)
    }
    
    /// 宣言の所有権チェックを実行します。
    fn check_declaration(&mut self, declaration: &Declaration) -> Result<()> {
        match &declaration.kind {
            DeclarationKind::FunctionDecl(function) => {
                self.check_function(function)?;
            },
            DeclarationKind::VariableDecl(var_decl) => {
                self.check_variable_declaration(var_decl)?;
            },
            DeclarationKind::StructDecl(_) => {
                // 構造体宣言の所有権チェックは、フィールドの型チェックが主
                // 今回は実装を省略
            },
            DeclarationKind::TypeAliasDecl(_) => {
                // 型エイリアスは所有権に影響しない
            },
            DeclarationKind::EnumDecl(_) => {
                // 列挙型の所有権チェックは列挙値のチェックが主
                // 今回は実装を省略
            },
            DeclarationKind::TraitDecl(_) => {
                // トレイトの所有権チェックはメソッドのチェックが主
                // 今回は実装を省略
            },
            DeclarationKind::ImplementationDecl(impl_block) => {
                // 実装ブロックのメソッド所有権チェック
                for method in &impl_block.methods {
                    self.check_function(method)?;
                }
            },
            DeclarationKind::ImportDecl(_) => {
                // インポートは所有権に影響しない
            },
            // その他の宣言タイプは所有権に影響しないため省略
            _ => {}
        }
        
        Ok(())
    }
    
    /// 関数の所有権チェックを実行します。
    fn check_function(&mut self, function: &Function) -> Result<()> {
        // 新しいスコープを開始
        self.push_scope();
        
        // パラメータを現在のスコープに追加
        for param in &function.parameters {
            self.register_parameter(param)?;
        }
        
        // 関数本体をチェック
        if let Some(body) = &function.body {
            self.check_statement(body)?;
        }
        
        // スコープを終了
        self.pop_scope();
        
        Ok(())
    }
    
    /// 変数宣言の所有権チェックを実行します。
    fn check_variable_declaration(&mut self, var_decl: &VariableDeclaration) -> Result<()> {
        // 初期化式があるかチェック
        let initial_state = if let Some(initializer) = &var_decl.initializer {
            // 初期化式をチェック
            let expr_state = self.check_expression(initializer)?;
            
            // 初期化式の所有権状態に基づいて変数の状態を決定
            if self.is_move_type(&var_decl.name.id) {
                // 所有型の場合、初期化式から所有権を移動
                self.handle_move_from_expression(initializer)?;
                OwnershipState::Owned
            } else {
                // 値型の場合はコピー
                OwnershipState::Owned
            }
        } else {
            // 初期化式がない場合は未初期化状態
            OwnershipState::Uninitialized
        };
        
        // 変数を現在のスコープに登録
        self.register_variable(
            var_decl.name.id,
            &var_decl.name.name,
            var_decl.type_annotation.as_ref().map(|t| t.id),
            initial_state,
            var_decl.location.clone()
        );
        
        Ok(())
    }
    
    /// 文の所有権チェックを実行します。
    fn check_statement(&mut self, statement: &Statement) -> Result<()> {
        match &statement.kind {
            StatementKind::Block(block) => {
                self.check_block(block)?;
            },
            StatementKind::ExpressionStmt(expr) => {
                self.check_expression(expr)?;
            },
            StatementKind::VariableDeclaration(var_decl) => {
                self.check_variable_declaration(var_decl)?;
            },
            StatementKind::IfStmt { condition, then_branch, else_branch } => {
                // 条件式をチェック
                self.check_expression(condition)?;
                
                // スナップショットを作成して分岐をチェック
                let snapshot = self.take_ownership_snapshot();
                
                // then分岐をチェック
                self.check_statement(then_branch)?;
                
                if let Some(else_branch) = else_branch {
                    // 所有権状態を元に戻す
                    self.restore_ownership_snapshot(&snapshot);
                    
                    // else分岐をチェック
                    self.check_statement(else_branch)?;
                }
                
                // then/elseの両方のパスでの所有権状態を統合
                self.merge_ownership_states(&snapshot);
            },
            StatementKind::WhileStmt { condition, body } => {
                // 条件式をチェック
                self.check_expression(condition)?;
                
                // スナップショットを作成してループ内をチェック
                let snapshot = self.take_ownership_snapshot();
                
                // ループ本体をチェック
                self.check_statement(body)?;
                
                // ループを抜けた後の所有権状態を統合
                self.merge_ownership_states(&snapshot);
            },
            StatementKind::ReturnStmt(expr_opt) => {
                if let Some(expr) = expr_opt {
                    // 戻り値式をチェック
                    self.check_expression(expr)?;
                    
                    // 戻り値が所有型の場合、所有権を移動
                    if self.is_move_type(&expr.id) {
                        self.handle_move_from_expression(expr)?;
                    }
                }
            },
            // その他の文タイプは現在実装を省略
            _ => {},
        }
        
        Ok(())
    }
    
    /// ブロックの所有権チェックを実行します。
    fn check_block(&mut self, block: &Block) -> Result<()> {
        // 新しいスコープを開始
        self.push_scope();
        
        // ブロック内の各文をチェック
        for stmt in &block.statements {
            self.check_statement(stmt)?;
        }
        
        // スコープを終了
        self.pop_scope();
        
        Ok(())
    }
    
    /// 式の所有権チェックを実行します。
    fn check_expression(&mut self, expression: &Expression) -> Result<OwnershipState> {
        // 式の種類に応じたチェック
        match &expression.kind {
            ExpressionKind::Identifier(id) => {
                // 識別子参照の所有権チェック
                self.check_identifier_reference(id, &expression.location)?;
                
                // 変数の所有権状態を返す
                let var_id = self.lookup_variable(&id.name)?;
                Ok(self.get_ownership_state(var_id))
            },
            ExpressionKind::Binary { left, operator, right } => {
                // 左右の式をチェック
                self.check_expression(left)?;
                self.check_expression(right)?;
                
                // 二項演算の結果は新しい値
                Ok(OwnershipState::Owned)
            },
            ExpressionKind::Unary { operator, operand } => {
                // オペランドをチェック
                self.check_expression(operand)?;
                
                // 単項演算の結果は新しい値
                Ok(OwnershipState::Owned)
            },
            ExpressionKind::Call { callee, arguments } => {
                // 関数呼び出しのチェック
                self.check_expression(callee)?;
                
                // 各引数をチェック
                for arg in arguments {
                    let arg_state = self.check_expression(arg)?;
                    
                    // 所有型の引数は関数に所有権が移動する
                    if self.is_move_type(&arg.id) {
                        self.handle_move_from_expression(arg)?;
                    }
                }
                
                // 関数呼び出しの結果は新しい値
                Ok(OwnershipState::Owned)
            },
            ExpressionKind::Reference { is_mutable, expr } => {
                // 参照作成のチェック
                self.check_reference_creation(is_mutable, expr, &expression.location, expression.id)?;
                
                // 参照の所有権状態を返す
                Ok(OwnershipState::OwnedReference(*is_mutable))
            },
            ExpressionKind::Dereference(expr) => {
                // 参照解除のチェック
                let expr_state = self.check_expression(expr)?;
                self.check_dereference(&expr_state, &expression.location)?;
                
                // 参照解除の結果は参照先の値
                match expr_state {
                    OwnershipState::OwnedReference(is_mutable) => {
                        if is_mutable {
                            Ok(OwnershipState::Owned)
                        } else {
                            // 不変参照の場合、結果は「借用された値」であり所有権は持たない
                            Ok(OwnershipState::Owned)
                        }
                    },
                    _ => Ok(OwnershipState::Owned), // エラーは既にcheck_dereferenceで報告済み
                }
            },
            ExpressionKind::Assignment { left, right } => {
                // 代入式のチェック
                self.check_assignment(left, right, &expression.location)?;
                
                // 代入式の結果は単位型（所有権なし）
                Ok(OwnershipState::Owned)
            },
            ExpressionKind::FieldAccess { expr, field } => {
                // フィールドアクセスのチェック
                let base_state = self.check_expression(expr)?;
                
                // フィールドアクセスの所有権状態は基本的に同じだが、
                // 移動可能型のフィールドにアクセスした場合は所有権を持つ
                if self.is_move_type(&expression.id) {
                    Ok(OwnershipState::Owned)
                } else {
                    Ok(base_state)
                }
            },
            ExpressionKind::IndexAccess { expr, index } => {
                // インデックスアクセスのチェック
                let base_state = self.check_expression(expr)?;
                let _index_state = self.check_expression(index)?;
                
                // インデックスアクセスの所有権状態は基本的に同じだが、
                // 移動可能型の要素にアクセスした場合は所有権を持つ
                if self.is_move_type(&expression.id) {
                    Ok(OwnershipState::Owned)
                } else {
                    Ok(base_state)
                }
            },
            ExpressionKind::Block(block) => {
                // ブロック式のチェック
                self.check_block(block)?;
                
                // ブロック式の結果は最後の式の所有権状態
                if let Some(last_stmt) = block.statements.last() {
                    if let StatementKind::ExpressionStmt(expr) = &last_stmt.kind {
                        return self.check_expression(expr);
                    }
                }
                
                // 式がない場合は単位型（所有権なし）
                Ok(OwnershipState::Owned)
            },
            ExpressionKind::If { condition, then_branch, else_branch } => {
                // 条件式をチェック
                self.check_expression(condition)?;
                
                // スナップショットを作成して分岐をチェック
                let snapshot = self.take_ownership_snapshot();
                
                // then分岐をチェック
                let then_state = self.check_expression(then_branch)?;
                
                if let Some(else_branch) = else_branch {
                    // 所有権状態を元に戻す
                    self.restore_ownership_snapshot(&snapshot);
                    
                    // else分岐をチェック
                    let else_state = self.check_expression(else_branch)?;
                    
                    // then/elseの両方のパスでの所有権状態を統合
                    self.merge_ownership_states(&snapshot);
                    
                    // 結果の所有権状態も統合
                    Ok(self.merge_states(&then_state, &else_state))
                } else {
                    // elseがない場合は単位型（所有権なし）
                    Ok(then_state)
                }
            },
            // その他の式タイプはOwnedとして扱う
            _ => Ok(OwnershipState::Owned),
        }
    }
    
    /// 参照式の作成をチェックします。
    fn check_reference_creation(&mut self, is_mutable: &bool, expr: &Expression, location: &SourceLocation, ref_id: NodeId) -> Result<()> {
        // 参照対象の式をチェック
        if let ExpressionKind::Identifier(id) = &expr.kind {
            let var_id = self.lookup_variable(&id.name)?;
            let state = self.get_ownership_state(var_id);
            
            // 参照可能な状態かチェック
            match state {
                OwnershipState::Uninitialized => {
                    self.errors.push(CompilerError::new(
                        ErrorKind::UninitializedVariable,
                        format!("初期化されていない変数 '{}' を参照することはできません", id.name),
                        Some(location.clone())
                    ));
                },
                OwnershipState::Moved => {
                    self.errors.push(CompilerError::new(
                        ErrorKind::OwnershipViolation,
                        format!("所有権が移動した変数 '{}' を参照することはできません", id.name),
                        Some(location.clone())
                    ));
                },
                OwnershipState::MutableBorrowed(borrower_id) => {
                    // 可変借用中の変数は再度借用できない
                    self.errors.push(CompilerError::new(
                        ErrorKind::OwnershipViolation,
                        format!("可変借用中の変数 '{}' を参照することはできません", id.name),
                        Some(location.clone())
                    ));
                },
                OwnershipState::Borrowed(refs) if *is_mutable => {
                    // 不変借用中の変数は可変参照できない
                    self.errors.push(CompilerError::new(
                        ErrorKind::OwnershipViolation,
                        format!("不変借用中の変数 '{}' を可変参照することはできません", id.name),
                        Some(location.clone())
                    ));
                },
                OwnershipState::Owned => {
                    // 参照を作成し、変数の所有権状態を更新
                    if *is_mutable {
                        // 可変参照は排他的（可変借用）
                        self.set_ownership_state(var_id, OwnershipState::MutableBorrowed(ref_id));
                    } else {
                        // 不変参照は共有可能（不変借用）
                        self.set_ownership_state(var_id, OwnershipState::Borrowed(vec![ref_id]));
                    }
                    
                    // 借用情報を記録
                    self.borrows.insert(ref_id, BorrowScope {
                        source_id: var_id,
                        target_id: ref_id,
                        is_mutable: *is_mutable,
                        location: location.clone(),
                    });
                },
                OwnershipState::Borrowed(mut refs) if !*is_mutable => {
                    // 不変借用中の変数に対する不変参照は追加可能
                    refs.push(ref_id);
                    self.set_ownership_state(var_id, OwnershipState::Borrowed(refs));
                    
                    // 借用情報を記録
                    self.borrows.insert(ref_id, BorrowScope {
                        source_id: var_id,
                        target_id: ref_id,
                        is_mutable: false,
                        location: location.clone(),
                    });
                },
                _ => {
                    // その他の状態は参照可能
                }
            }
        } else {
            // 識別子以外への参照は現在サポートしていない
            self.warnings.push(CompilerError::new(
                ErrorKind::UnsupportedFeature,
                "識別子以外への参照は完全にはサポートされていません".to_string(),
                Some(location.clone())
            ));
            
            // 式自体はチェックする
            self.check_expression(expr)?;
        }
        
        Ok(())
    }
    
    /// 参照解除のチェックを実行します。
    fn check_dereference(&mut self, expr_state: &OwnershipState, location: &SourceLocation) -> Result<()> {
        match expr_state {
            OwnershipState::OwnedReference(_) => {
                // 参照の解除は可能
                Ok(())
            },
            _ => {
                // 参照型でない値の参照解除はエラー
                self.errors.push(CompilerError::new(
                    ErrorKind::TypeMismatch,
                    "参照型でない値を参照解除することはできません".to_string(),
                    Some(location.clone())
                ));
                Ok(())
            }
        }
    }
    
    /// 代入式のチェックを実行します。
    fn check_assignment(&mut self, left: &Expression, right: &Expression, location: &SourceLocation) -> Result<()> {
        // 代入先をチェック
        match &left.kind {
            ExpressionKind::Identifier(id) => {
                // 識別子への代入
                let var_id = self.lookup_variable(&id.name)?;
                let state = self.get_ownership_state(var_id);
                
                // 代入先の変数が代入可能な状態かチェック
                match state {
                    OwnershipState::Moved => {
                        self.errors.push(CompilerError::new(
                            ErrorKind::OwnershipViolation,
                            format!("変数 '{}' の所有権は既に移動しています", id.name),
                            Some(location.clone())
                        ));
                    },
                    OwnershipState::Borrowed(borrowers) => {
                        self.errors.push(CompilerError::new(
                            ErrorKind::OwnershipViolation,
                            format!("変数 '{}' は現在借用されているため代入できません", id.name),
                            Some(location.clone())
                        ));
                    },
                    OwnershipState::MutableBorrowed(borrower_id) => {
                        self.errors.push(CompilerError::new(
                            ErrorKind::OwnershipViolation,
                            format!("変数 '{}' は現在借用されているため代入できません", id.name),
                            Some(location.clone())
                        ));
                    },
                    _ => {}
                }
            },
            _ => {}
        }
        
        Ok(())
    }
    
    /// 識別子参照の所有権チェックを実行します。
    fn check_identifier_reference(&mut self, id: &Identifier, location: &SourceLocation) -> Result<()> {
        // 変数が定義されているかチェック
        let var_id = self.lookup_variable(&id.name)?;
        let state = self.get_ownership_state(var_id);
        
        // 変数の所有権状態をチェック
        match state {
            OwnershipState::Uninitialized => {
                self.errors.push(CompilerError::new(
                    ErrorKind::UninitializedVariable,
                    format!("変数 '{}' は使用前に初期化されていません", id.name),
                    Some(location.clone())
                ));
            },
            OwnershipState::Moved => {
                self.errors.push(CompilerError::new(
                    ErrorKind::OwnershipViolation,
                    format!("変数 '{}' の所有権は既に移動しています", id.name),
                    Some(location.clone())
                ));
            },
            // その他の状態は参照可能
            _ => {}
        }
        
        Ok(())
    }
    
    /// 新しいスコープを作成します。
    fn push_scope(&mut self) {
        self.scope_stack.push(HashMap::new());
    }
    
    /// 現在のスコープを破棄します。
    fn pop_scope(&mut self) {
        if let Some(scope) = self.scope_stack.pop() {
            // スコープ内の変数をcurrent_variablesから削除
            for (name, id) in self.current_variables.clone().iter() {
                if scope.contains_key(id) {
                    self.current_variables.remove(name);
                }
            }
        }
    }
    
    /// 変数を現在のスコープに登録します。
    fn register_variable(&mut self, id: NodeId, name: &str, type_id: Option<NodeId>, 
                        state: OwnershipState, location: SourceLocation) {
        // 既に同名の変数が存在するかチェック
        if let Some(existing_id) = self.current_variables.get(name) {
            self.errors.push(CompilerError::new(
                ErrorKind::DuplicateVariable,
                format!("変数 '{}' は既に定義されています", name),
                Some(location)
            ));
            return;
        }
        
        // 現在のスコープに変数を追加
        let scope = self.scope_stack.last_mut().unwrap();
        scope.insert(id, VariableScope {
            id,
            name: name.to_string(),
            type_id,
            state: state.clone(),
            location,
        });
        
        // 現在の変数マップに追加
        self.current_variables.insert(name.to_string(), id);
        
        // 所有権状態を初期化
        self.ownership_states.insert(id, state);
    }
    
    /// パラメータを現在のスコープに登録します。
    fn register_parameter(&mut self, param: &Parameter) -> Result<()> {
        self.register_variable(
            param.id,
            &param.name,
            param.type_annotation.as_ref().map(|t| t.id),
            OwnershipState::Owned, // パラメータは初期化済みとして扱う
            param.location.clone()
        );
        
        Ok(())
    }
    
    /// 変数名から変数IDを検索します。
    fn lookup_variable(&self, name: &str) -> Result<NodeId> {
        match self.current_variables.get(name) {
            Some(id) => Ok(*id),
            None => Err(CompilerError::new(
                ErrorKind::UndefinedVariable,
                format!("変数 '{}' は定義されていません", name),
                None
            )),
        }
    }
    
    /// 変数の所有権状態を取得します。
    fn get_ownership_state(&self, id: NodeId) -> OwnershipState {
        match self.ownership_states.get(&id) {
            Some(state) => state.clone(),
            None => OwnershipState::Uninitialized,
        }
    }
    
    /// 変数の所有権状態を設定します。
    fn set_ownership_state(&mut self, id: NodeId, state: OwnershipState) {
        self.ownership_states.insert(id, state);
    }
    
    /// 式から所有権を移動させます。
    fn handle_move_from_expression(&mut self, expr: &Expression) -> Result<()> {
        match &expr.kind {
            ExpressionKind::Identifier(id) => {
                let var_id = self.lookup_variable(&id.name)?;
                let state = self.get_ownership_state(var_id);
                
                // 移動可能な状態かチェック
                match state {
                    OwnershipState::Owned => {
                        // 所有権を移動
                        self.set_ownership_state(var_id, OwnershipState::Moved);
                    },
                    OwnershipState::Uninitialized => {
                        self.errors.push(CompilerError::new(
                            ErrorKind::UninitializedVariable,
                            format!("初期化されていない変数 '{}' から所有権を移動することはできません", id.name),
                            Some(expr.location.clone())
                        ));
                    },
                    OwnershipState::Moved => {
                        self.errors.push(CompilerError::new(
                            ErrorKind::OwnershipViolation,
                            format!("変数 '{}' の所有権は既に移動しています", id.name),
                            Some(expr.location.clone())
                        ));
                    },
                    // その他の状態からは移動不可
                    _ => {
                        self.errors.push(CompilerError::new(
                            ErrorKind::OwnershipViolation,
                            format!("変数 '{}' は現在の状態から所有権を移動できません", id.name),
                            Some(expr.location.clone())
                        ));
                    }
                }
            },
            // その他の式タイプは所有権の移動に影響しない
            _ => {},
        }
        
        Ok(())
    }
    
    /// 型が所有権の移動を必要とするかどうかを判定します。
    fn is_move_type(&self, id: &NodeId) -> bool {
        // 型チェック結果から型情報を取得
        if let Some(type_check) = &self.type_check_result {
            if let Some(type_id) = type_check.node_types.get(id) {
                // 型情報に基づいて、型が移動型かどうかを判定
                return self.is_move_type_annotation(type_id);
            }
        }
        
        // 型情報がない場合は安全のため移動型と判定
        true
    }
    
    /// 型注釈が移動型かどうかを判定します。
    fn is_move_type_annotation(&self, type_ann: &TypeAnnotation) -> bool {
        match &type_ann.kind {
            TypeKind::Named(ident) => {
                // 基本型（プリミティブ型）はコピー可能
                match ident.name.as_str() {
                    // プリミティブ型はコピー可能（移動型ではない）
                    "i8" | "i16" | "i32" | "i64" | "i128" | "isize" |
                    "u8" | "u16" | "u32" | "u64" | "u128" | "usize" |
                    "f32" | "f64" |
                    "bool" | "char" => false,
                    
                    // 文字列リテラル型は参照型なのでコピー可能
                    "&str" => false,
                    
                    // String型は所有型（ヒープ確保するため）
                    "String" => true,
                    
                    // 標準ライブラリの一般的なコレクション型は移動型
                    "Vec" | "HashMap" | "HashSet" | "BTreeMap" | "BTreeSet" | 
                    "LinkedList" | "VecDeque" | "BinaryHeap" => true,
                    
                    // ユーザー定義型は基本的に移動型と判定
                    // 実際にはユーザー定義型がCopyトレイトを実装しているかチェックすべき
                    _ => {
                        // シンボルテーブルで型の定義を探す
                        // 本来はシンボルテーブルを参照して、型がCopyトレイトを実装しているか確認する
                        // 今回の実装では、保守的に全てのユーザー定義型を移動型とする
                        if let Some(type_check) = &self.type_check_result {
                            // 将来的にはここで型がCopyトレイトを実装しているか確認する
                            // 現在は単に名前に基づいて判断
                            if ident.name.starts_with("Copy") || ident.name.ends_with("Copy") {
                                return false;
                            }
                        }
                        true
                    }
                }
            },
            
            // 参照型はコピー可能
            TypeKind::Reference(_, _) => false,
            
            // 配列型は要素型に依存する
            TypeKind::Array(element_type, _) => {
                // 配列の要素が移動型なら配列も移動型
                self.is_move_type_annotation(element_type)
            },
            
            // スライス型は参照なのでコピー可能
            TypeKind::Slice(_) => false,
            
            // タプル型は全ての要素がコピー可能なら、タプルもコピー可能
            TypeKind::Tuple(element_types) => {
                // 一つでも移動型の要素があれば、タプル全体が移動型
                element_types.iter().any(|elem_type| self.is_move_type_annotation(elem_type))
            },
            
            // オプション型は内部の型に依存
            TypeKind::Optional(inner_type) => self.is_move_type_annotation(inner_type),
            
            // ジェネリック型は型引数に依存
            TypeKind::Generic(base_type, type_args) => {
                // 基本型がコピー可能でも、型引数が移動型ならジェネリック型全体は移動型
                // 例: Option<i32> はコピー可能だが Option<String> は移動型
                if let TypeKind::Named(base_ident) = &base_type.kind {
                    match base_ident.name.as_str() {
                        // Option<T>は内部の型Tがコピー可能ならコピー可能
                        "Option" if type_args.len() == 1 => self.is_move_type_annotation(&type_args[0]),
                        
                        // Result<T, E>は両方の型がコピー可能ならコピー可能
                        "Result" if type_args.len() == 2 => {
                            self.is_move_type_annotation(&type_args[0]) || 
                            self.is_move_type_annotation(&type_args[1])
                        },
                        
                        // その他のジェネリック型は通常は移動型
                        _ => true
                    }
                } else {
                    // 複雑な基本型の場合は保守的に移動型とする
                    true
                }
            },
            
            // パス型（module::Type）はモジュールパスを無視して内部の型を確認
            TypeKind::Path(_, inner_type) => self.is_move_type_annotation(inner_type),
            
            // 関数型や関数ポインタはコピー可能
            TypeKind::Function(_, _) => false,
            
            // ポインタ型はコピー可能
            TypeKind::Pointer(_, _) => false,
            
            // ユニット型はコピー可能
            TypeKind::Unit => false,
            
            // Never型は実際には値を持たないのでコピー可能とみなす
            TypeKind::Never => false,
            
            // 型推論中や型エラーの場合は保守的に移動型とする
            TypeKind::Inferred | TypeKind::Error => true,
            
            // Self型はコンテキストに依存するが、保守的に移動型とする
            TypeKind::SelfType => true,
            
            // 存在型（トレイト境界）は保守的に移動型とする
            TypeKind::Existential(_) => true,
            
            // Any型は保守的に移動型とする
            TypeKind::Any => true,
        }
    }
    
    /// 型に対して深さ優先の型検査を行うための内部メソッド
    fn check_type_deeply(&self, type_ann: &TypeAnnotation, visited: &mut HashSet<NodeId>, check_fn: impl Fn(&TypeKind) -> bool) -> bool {
        // 循環参照を防ぐために訪問済みの型はスキップ
        if !visited.insert(type_ann.id) {
            return false;
        }
        
        // 型自体をチェック
        if check_fn(&type_ann.kind) {
            return true;
        }
        
        // 型の中の型を再帰的にチェック
        match &type_ann.kind {
            TypeKind::Reference(inner, _) | 
            TypeKind::Pointer(inner, _) | 
            TypeKind::Optional(inner) |
            TypeKind::Existential(inner) => {
                self.check_type_deeply(inner, visited, check_fn)
            },
            
            TypeKind::Array(inner, _) |
            TypeKind::Slice(inner) => {
                self.check_type_deeply(inner, visited, check_fn)
            },
            
            TypeKind::Tuple(elements) => {
                elements.iter().any(|elem| self.check_type_deeply(elem, visited, check_fn))
            },
            
            TypeKind::Function(params, ret_type) => {
                let params_result = params.iter().any(|param| self.check_type_deeply(param, visited, check_fn));
                
                if params_result {
                    return true;
                }
                
                if let Some(ret) = ret_type {
                    return self.check_type_deeply(ret, visited, check_fn);
                }
                
                false
            },
            
            TypeKind::Generic(base, args) => {
                if self.check_type_deeply(base, visited, check_fn) {
                    return true;
                }
                
                args.iter().any(|arg| self.check_type_deeply(arg, visited, check_fn))
            },
            
            TypeKind::Path(_, inner) => {
                self.check_type_deeply(inner, visited, check_fn)
            },
            
            _ => false,
        }
    }
    
    /// 型に特定の条件を満たす型が含まれているかをチェック
    fn type_contains(&self, type_ann: &TypeAnnotation, predicate: impl Fn(&TypeKind) -> bool) -> bool {
        let mut visited = HashSet::new();
        self.check_type_deeply(type_ann, &mut visited, predicate)
    }
    
    /// 型が所有権セマンティクスに関連するトレイトを実装しているかチェック
    fn check_trait_implementation(&self, type_ann: &TypeAnnotation, trait_name: &str) -> bool {
        // 本来はシンボルテーブルやトレイト実装情報を参照して、
        // 型が特定のトレイトを実装しているか確認するべき
        // ここでは簡易的な実装として、型名に基づいて判断
        
        if let TypeKind::Named(ident) = &type_ann.kind {
            // 型名に特定のトレイト名が含まれる場合、そのトレイトを実装していると仮定
            ident.name.contains(trait_name)
        } else {
            false
        }
    }
    
    /// 現在の所有権状態のスナップショットを取得します。
    fn take_ownership_snapshot(&self) -> HashMap<NodeId, OwnershipState> {
        self.ownership_states.clone()
    }
    
    /// 所有権状態のスナップショットを復元します。
    fn restore_ownership_snapshot(&mut self, snapshot: &HashMap<NodeId, OwnershipState>) {
        self.ownership_states = snapshot.clone();
    }
    
    /// 分岐後の所有権状態を統合します。
    fn merge_ownership_states(&mut self, snapshot: &HashMap<NodeId, OwnershipState>) {
        // 各変数について、現在の状態とスナップショットの状態を比較して最も制約の厳しい状態を採用
        for (id, state) in snapshot {
            if let Some(current_state) = self.ownership_states.get(id) {
                let merged_state = self.merge_states(state, current_state);
                self.ownership_states.insert(*id, merged_state);
            }
        }
    }
    
    /// 2つの所有権状態を統合します。
    fn merge_states(&self, state1: &OwnershipState, state2: &OwnershipState) -> OwnershipState {
        match (state1, state2) {
            // 両方が同じ状態なら、その状態を採用
            (s1, s2) if s1 == s2 => s1.clone(),
            
            // 片方が未初期化なら、もう片方が初期化されていても、未初期化として扱う
            (OwnershipState::Uninitialized, _) | (_, OwnershipState::Uninitialized) => {
                OwnershipState::Uninitialized
            },
            
            // 片方が移動済みなら、もう片方が所有状態でも、移動済みとして扱う
            (OwnershipState::Moved, _) | (_, OwnershipState::Moved) => {
                OwnershipState::Moved
            },
            
            // その他の場合は所有状態を採用
            _ => OwnershipState::Owned,
        }
    }

    /// 所有権パスを作成（式から所有権パスを構築）
    fn create_path_from_expression(&self, expr: &Expression) -> Result<OwnershipPath> {
        match &expr.kind {
            ExpressionKind::Identifier(id) => {
                let var_id = self.lookup_variable(&id.name)?;
                Ok(OwnershipPath::Root(var_id))
            },
            ExpressionKind::FieldAccess { expr: base, field } => {
                let base_path = self.create_path_from_expression(base)?;
                Ok(OwnershipPath::Field(Box::new(base_path), field.name.clone()))
            },
            ExpressionKind::IndexAccess { expr: base, index } => {
                let base_path = self.create_path_from_expression(base)?;
                
                // 理想的には、インデックスが定数であれば実際の値を使用
                // 現在は簡易的に0としておく
                Ok(OwnershipPath::Index(Box::new(base_path), 0))
            },
            ExpressionKind::Dereference(base) => {
                let base_path = self.create_path_from_expression(base)?;
                Ok(OwnershipPath::Dereference(Box::new(base_path)))
            },
            _ => {
                // その他の式タイプからは所有権パスを作成できない
                Err(CompilerError::new(
                    ErrorKind::InternalError,
                    "所有権パスを作成できない式です".to_string(),
                    Some(expr.location.clone())
                ))
            }
        }
    }
    
    /// 所有権パスの使用を記録
    fn record_path_usage(&mut self, path: OwnershipPath, kind: PathUsageKind, location: SourceLocation, node_id: NodeId) {
        // ここでは簡易的な実装として、使用情報を記録するだけとする
        // 本来はこの情報を使用して、より高度な所有権分析を行う
        
        // 所有権パスの使用をチェック
        let root_id = path.root_id();
        let state = self.get_ownership_state(root_id);
        
        // 使用種類に応じたチェック
        match kind {
            PathUsageKind::Read => {
                // 読み取りは基本的に許可されるが、未初期化または移動済みの変数からは読み取れない
                match state {
                    OwnershipState::Uninitialized => {
                        self.errors.push(CompilerError::new(
                            ErrorKind::UninitializedVariable,
                            format!("未初期化の変数パス '{}' を読み取ろうとしています", path.to_string()),
                            Some(location)
                        ));
                    },
                    OwnershipState::Moved => {
                        self.errors.push(CompilerError::new(
                            ErrorKind::OwnershipViolation,
                            format!("所有権が移動した変数パス '{}' を読み取ろうとしています", path.to_string()),
                            Some(location)
                        ));
                    },
                    _ => {
                        // その他の状態は読み取り可能
                    }
                }
            },
            PathUsageKind::Write => {
                // 書き込みは借用中の変数には許可されない
                match state {
                    OwnershipState::Borrowed(borrowers) => {
                        self.errors.push(CompilerError::new(
                            ErrorKind::OwnershipViolation,
                            format!("借用中の変数パス '{}' に書き込もうとしています", path.to_string()),
                            Some(location)
                        ));
                    },
                    OwnershipState::MutableBorrowed(borrower_id) => {
                        self.errors.push(CompilerError::new(
                            ErrorKind::OwnershipViolation,
                            format!("可変借用中の変数パス '{}' に書き込もうとしています", path.to_string()),
                            Some(location)
                        ));
                    },
                    OwnershipState::Moved => {
                        self.errors.push(CompilerError::new(
                            ErrorKind::OwnershipViolation,
                            format!("所有権が移動した変数パス '{}' に書き込もうとしています", path.to_string()),
                            Some(location)
                        ));
                    },
                    _ => {
                        // その他の状態は書き込み可能
                        
                        // 書き込みにより変数は初期化される
                        if state == OwnershipState::Uninitialized {
                            self.set_ownership_state(root_id, OwnershipState::Owned);
                        }
                    }
                }
            },
            PathUsageKind::Move => {
                // 移動は所有権を持つ変数からのみ許可される
                match state {
                    OwnershipState::Owned => {
                        // 所有権を移動
                        self.set_ownership_state(root_id, OwnershipState::Moved);
                    },
                    OwnershipState::Uninitialized => {
                        self.errors.push(CompilerError::new(
                            ErrorKind::UninitializedVariable,
                            format!("未初期化の変数パス '{}' の所有権を移動しようとしています", path.to_string()),
                            Some(location)
                        ));
                    },
                    OwnershipState::Moved => {
                        self.errors.push(CompilerError::new(
                            ErrorKind::OwnershipViolation,
                            format!("既に所有権が移動した変数パス '{}' の所有権を再度移動しようとしています", path.to_string()),
                            Some(location)
                        ));
                    },
                    OwnershipState::Borrowed(borrowers) => {
                        self.errors.push(CompilerError::new(
                            ErrorKind::OwnershipViolation,
                            format!("借用中の変数パス '{}' の所有権を移動しようとしています", path.to_string()),
                            Some(location)
                        ));
                    },
                    OwnershipState::MutableBorrowed(borrower_id) => {
                        self.errors.push(CompilerError::new(
                            ErrorKind::OwnershipViolation,
                            format!("可変借用中の変数パス '{}' の所有権を移動しようとしています", path.to_string()),
                            Some(location)
                        ));
                    },
                    _ => {
                        // その他の状態からは移動不可
                        self.errors.push(CompilerError::new(
                            ErrorKind::OwnershipViolation,
                            format!("変数パス '{}' の所有権を移動できません", path.to_string()),
                            Some(location)
                        ));
                    }
                }
            },
            PathUsageKind::Borrow => {
                // 不変借用は、可変借用中以外は許可される
                match state {
                    OwnershipState::Uninitialized => {
                        self.errors.push(CompilerError::new(
                            ErrorKind::UninitializedVariable,
                            format!("未初期化の変数パス '{}' を借用しようとしています", path.to_string()),
                            Some(location)
                        ));
                    },
                    OwnershipState::Moved => {
                        self.errors.push(CompilerError::new(
                            ErrorKind::OwnershipViolation,
                            format!("所有権が移動した変数パス '{}' を借用しようとしています", path.to_string()),
                            Some(location)
                        ));
                    },
                    OwnershipState::MutableBorrowed(borrower_id) => {
                        self.errors.push(CompilerError::new(
                            ErrorKind::OwnershipViolation,
                            format!("可変借用中の変数パス '{}' を不変借用しようとしています", path.to_string()),
                            Some(location)
                        ));
                    },
                    OwnershipState::Owned => {
                        // 不変借用を作成
                        self.set_ownership_state(root_id, OwnershipState::Borrowed(vec![node_id]));
                    },
                    OwnershipState::Borrowed(mut borrowers) => {
                        // 既存の不変借用に追加
                        borrowers.push(node_id);
                        self.set_ownership_state(root_id, OwnershipState::Borrowed(borrowers));
                    },
                    _ => {
                        // その他の状態は借用可能
                    }
                }
            },
            PathUsageKind::MutableBorrow => {
                // 可変借用は、変数が所有状態で他の借用がない場合のみ許可される
                match state {
                    OwnershipState::Owned => {
                        // 可変借用を作成
                        self.set_ownership_state(root_id, OwnershipState::MutableBorrowed(node_id));
                    },
                    OwnershipState::Uninitialized => {
                        self.errors.push(CompilerError::new(
                            ErrorKind::UninitializedVariable,
                            format!("未初期化の変数パス '{}' を可変借用しようとしています", path.to_string()),
                            Some(location)
                        ));
                    },
                    OwnershipState::Moved => {
                        self.errors.push(CompilerError::new(
                            ErrorKind::OwnershipViolation,
                            format!("所有権が移動した変数パス '{}' を可変借用しようとしています", path.to_string()),
                            Some(location)
                        ));
                    },
                    OwnershipState::Borrowed(borrowers) => {
                        self.errors.push(CompilerError::new(
                            ErrorKind::OwnershipViolation,
                            format!("不変借用中の変数パス '{}' を可変借用しようとしています", path.to_string()),
                            Some(location)
                        ));
                    },
                    OwnershipState::MutableBorrowed(borrower_id) => {
                        self.errors.push(CompilerError::new(
                            ErrorKind::OwnershipViolation,
                            format!("既に可変借用中の変数パス '{}' を再度可変借用しようとしています", path.to_string()),
                            Some(location)
                        ));
                    },
                    _ => {
                        // その他の状態は借用可能
                    }
                }
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend::ast::{Expression, ExpressionKind, Statement, StatementKind, Declaration, DeclarationKind};
    
    // テストヘルパー
    fn create_test_type_checker_result() -> TypeCheckResult {
        let mut result = TypeCheckResult::default();
        // テスト用の型情報を設定
        result
    }
    
    // ダミー位置情報を作成
    fn create_dummy_location() -> SourceLocation {
        SourceLocation {
            file: "test.swift".to_string(),
            start_line: 1,
            start_column: 1,
            end_line: 1,
            end_column: 1,
        }
    }
    
    #[test]
    fn test_basic_ownership_tracking() {
        // テスト用の所有権チェッカーを作成
        let mut checker = OwnershipChecker::new(Some(create_test_type_checker_result()));
        
        // 新しいスコープを作成
        checker.push_scope();
        
        // 変数を登録
        let var_id = 1;
        let var_name = "x";
        checker.register_variable(
            var_id,
            var_name,
            None,
            OwnershipState::Uninitialized,
            create_dummy_location()
        );
        
        // 初期状態は未初期化
        assert_eq!(checker.get_ownership_state(var_id), OwnershipState::Uninitialized);
        
        // 所有権状態を変更
        checker.set_ownership_state(var_id, OwnershipState::Owned);
        assert_eq!(checker.get_ownership_state(var_id), OwnershipState::Owned);
        
        // 所有権を移動
        checker.set_ownership_state(var_id, OwnershipState::Moved);
        assert_eq!(checker.get_ownership_state(var_id), OwnershipState::Moved);
        
        // スコープを破棄
        checker.pop_scope();
    }
    
    #[test]
    fn test_basic_move_type_detection() {
        // テスト用の型注釈を作成
        let int_type = TypeAnnotation {
            id: 1,
            kind: TypeKind::Named(Identifier {
                id: 2,
                name: "i32".to_string(),
                location: None,
            }),
            location: None,
        };
        
        let string_type = TypeAnnotation {
            id: 3,
            kind: TypeKind::Named(Identifier {
                id: 4,
                name: "String".to_string(),
                location: None,
            }),
            location: None,
        };
        
        // 型チェック結果を作成
        let mut type_check_result = TypeCheckResult::default();
        type_check_result.node_types.insert(5, int_type.clone());
        type_check_result.node_types.insert(6, string_type.clone());
        
        // 所有権チェッカーを作成
        let checker = OwnershipChecker::new(Some(type_check_result));
        
        // i32はコピー可能型
        assert_eq!(checker.is_move_type_annotation(&int_type), false);
        
        // Stringは移動型
        assert_eq!(checker.is_move_type_annotation(&string_type), true);
    }
    
    #[test]
    fn test_composite_move_type_detection() {
        // テスト用の型注釈を作成
        let int_type = TypeAnnotation {
            id: 1,
            kind: TypeKind::Named(Identifier {
                id: 2,
                name: "i32".to_string(),
                location: None,
            }),
            location: None,
        };
        
        let string_type = TypeAnnotation {
            id: 3,
            kind: TypeKind::Named(Identifier {
                id: 4,
                name: "String".to_string(),
                location: None,
            }),
            location: None,
        };
        
        // タプル型 (i32, i32)
        let int_tuple_type = TypeAnnotation {
            id: 5,
            kind: TypeKind::Tuple(vec![int_type.clone(), int_type.clone()]),
            location: None,
        };
        
        // タプル型 (i32, String)
        let mixed_tuple_type = TypeAnnotation {
            id: 6,
            kind: TypeKind::Tuple(vec![int_type.clone(), string_type.clone()]),
            location: None,
        };
        
        // 型チェック結果を作成
        let mut type_check_result = TypeCheckResult::default();
        type_check_result.node_types.insert(7, int_tuple_type.clone());
        type_check_result.node_types.insert(8, mixed_tuple_type.clone());
        
        // 所有権チェッカーを作成
        let checker = OwnershipChecker::new(Some(type_check_result));
        
        // (i32, i32)はコピー可能型
        assert_eq!(checker.is_move_type_annotation(&int_tuple_type), false);
        
        // (i32, String)は移動型（Stringが移動型なため）
        assert_eq!(checker.is_move_type_annotation(&mixed_tuple_type), true);
    }
}
