use crate::utils::span::Span;
use crate::ast::ty::TypeKind;
use crate::frontend::hir::HirId;

/// 高レベル中間表現 (HIR) の基底ノード
#[derive(Debug, Clone, PartialEq)]
pub struct HirNode {
    pub id: HirId,
    pub kind: HirNodeKind,
    pub span: Span,
    pub ownership_mode: OwnershipMode,
    pub lifetime_annotation: Option<Lifetime>,
}

/// HIRノードの種類を表す列挙型
#[derive(Debug, Clone, PartialEq)]
pub enum HirNodeKind {
    /// モジュール定義
    Module(HirModule),
    /// 関数定義
    Function(HirFunction),
    /// 変数宣言
    Variable(HirVariable),
    /// 制御構造
    ControlStructure(ControlStructure),
    /// 式
    Expression(Expression),
    /// 型注釈
    TypeAnnotation(TypeAnnotation),
    /// メモリ操作
    MemoryOperation(MemoryOperation),
    /// トレイト定義
    Trait(HirTrait),
    /// 実装ブロック
    ImplBlock(HirImpl),
}

/// モジュール定義
#[derive(Debug, Clone, PartialEq)]
pub struct HirModule {
    pub name: String,
    pub items: Vec<HirId>,
    pub visibility: Visibility,
    pub dependencies: Vec<String>,
}

/// 関数定義
#[derive(Debug, Clone, PartialEq)]
pub struct HirFunction {
    pub name: String,
    pub parameters: Vec<HirParameter>,
    pub return_type: TypeAnnotation,
    pub body: Vec<HirId>,
    pub generics: Vec<GenericParameter>,
    pub where_clauses: Vec<WhereClause>,
    pub visibility: Visibility,
    pub is_async: bool,
    pub safety_mode: SafetyMode,
}

/// 変数宣言
#[derive(Debug, Clone, PartialEq)]
pub struct HirVariable {
    pub name: String,
    pub ty: TypeAnnotation,
    pub mutability: Mutability,
    pub initializer: Option<HirId>,
    pub ownership_mode: OwnershipMode,
    pub lifetime: Option<Lifetime>,
}

/// 制御構造
#[derive(Debug, Clone, PartialEq)]
pub enum ControlStructure {
    If {
        condition: HirId,
        then_block: Vec<HirId>,
        else_block: Option<Vec<HirId>>,
    },
    Loop {
        body: Vec<HirId>,
        loop_kind: LoopKind,
    },
    Match {
        expr: HirId,
        arms: Vec<MatchArm>,
    },
}

/// 式の種類
#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    Literal(Literal),
    BinaryOp {
        op: BinaryOperator,
        lhs: HirId,
        rhs: HirId,
    },
    UnaryOp {
        op: UnaryOperator,
        operand: HirId,
    },
    Call {
        callee: HirId,
        args: Vec<HirId>,
    },
    FieldAccess {
        base: HirId,
        field: String,
    },
    MethodCall {
        receiver: HirId,
        method: String,
        args: Vec<HirId>,
    },
    Borrow {
        expr: HirId,
        kind: BorrowKind,
        lifetime: Option<Lifetime>,
    },
}

/// 型注釈
#[derive(Debug, Clone, PartialEq)]
pub struct TypeAnnotation {
    pub kind: TypeKind,
    pub lifetime_params: Vec<Lifetime>,
    pub type_params: Vec<TypeParameter>,
}

/// メモリ操作
#[derive(Debug, Clone, PartialEq)]
pub enum MemoryOperation {
    Allocate {
        ty: TypeAnnotation,
        size: Option<HirId>,
    },
    Deallocate {
        ptr: HirId,
    },
    Borrow {
        source: HirId,
        kind: BorrowKind,
        lifetime: Lifetime,
    },
    Move {
        source: HirId,
        destination: HirId,
    },
}

/// 所有権モード
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OwnershipMode {
    Owned,
    Borrowed,
    MutBorrowed,
    Shared,
}

/// ライフタイム情報
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Lifetime {
    pub name: String,
    pub scope_depth: usize,
    pub dependencies: Vec<Lifetime>,
}

/// 安全性モード
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SafetyMode {
    Safe,
    Unsafe,
    Trusted,
}

/// 可視性
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Visibility {
    Public,
    Private,
    Restricted(Vec<String>),
}
