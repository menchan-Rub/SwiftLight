//! # 抽象構文木（AST）
//! 
//! SwiftLight言語のソースコードを表現する抽象構文木（AST）の定義です。
//! パーサーが生成し、後続の処理段階（意味解析、型チェック、コード生成）で使用されます。

use std::fmt;
use std::path::PathBuf;

use crate::frontend::error::SourceLocation;

/// ノードID（AST内の各ノードを一意に識別する整数）
pub type NodeId = usize;

/// 位置情報を持つトレイト
pub trait Locatable {
    /// ソースコード内の位置情報を取得
    fn location(&self) -> Option<&SourceLocation>;
}

/// プログラム全体を表すルートノード
#[derive(Debug, Clone)]
pub struct Program {
    /// モジュール名
    pub name: String,
    /// ソースファイルパス
    pub source_path: PathBuf,
    /// トップレベル宣言のリスト
    pub declarations: Vec<Declaration>,
    /// ノードID
    pub id: NodeId,
    /// 位置情報
    pub location: Option<SourceLocation>,
}

impl Program {
    /// 新しいプログラムを作成
    pub fn new(name: String, source_path: PathBuf, declarations: Vec<Declaration>, id: NodeId, location: Option<SourceLocation>) -> Self {
        Self {
            name,
            source_path,
            declarations,
            id,
            location,
        }
    }
    
    /// 宣言を追加
    pub fn add_declaration(&mut self, declaration: Declaration) {
        self.declarations.push(declaration);
    }
}

impl Locatable for Program {
    fn location(&self) -> Option<&SourceLocation> {
        self.location.as_ref()
    }
}

/// 宣言ノード
#[derive(Debug, Clone)]
pub struct Declaration {
    /// 宣言の種類
    pub kind: DeclarationKind,
    /// ノードID
    pub id: NodeId,
    /// 位置情報
    pub location: Option<SourceLocation>,
}

impl Declaration {
    /// 新しい宣言を作成
    pub fn new(kind: DeclarationKind, id: NodeId, location: Option<SourceLocation>) -> Self {
        Self {
            kind,
            id,
            location,
        }
    }
}

impl Locatable for Declaration {
    fn location(&self) -> Option<&SourceLocation> {
        self.location.as_ref()
    }
}

/// 宣言の種類
#[derive(Debug, Clone)]
pub enum DeclarationKind {
    /// 変数宣言 (let x = 1;)
    VariableDeclaration(VariableDeclaration),
    /// 定数宣言 (const PI = 3.14;)
    ConstantDeclaration(ConstantDeclaration),
    /// 関数宣言 (fn add(a: int, b: int) -> int { ... })
    FunctionDeclaration(Function),
    /// 構造体宣言 (struct Point { x: int, y: int })
    StructDeclaration(Struct),
    /// 列挙型宣言 (enum Color { Red, Green, Blue })
    EnumDeclaration(Enum),
    /// トレイト宣言 (trait Comparable { ... })
    TraitDeclaration(Trait),
    /// 実装 (impl Trait for Type { ... })
    ImplementationDeclaration(Implementation),
    /// 型エイリアス (type StringArray = [String];)
    TypeAliasDeclaration(TypeAlias),
    /// インポート宣言 (import path.to.module;)
    ImportDeclaration(Import),
    /// モジュール宣言 (module name;)
    ModuleDeclaration(Module),
}

/// 変数宣言
#[derive(Debug, Clone)]
pub struct VariableDeclaration {
    /// 変数名
    pub name: Identifier,
    /// 型注釈（オプション）
    pub type_annotation: Option<TypeAnnotation>,
    /// 可変かどうか (mut キーワードの有無)
    pub is_mutable: bool,
    /// 初期化式（オプション）
    pub initializer: Option<Expression>,
    /// 可視性
    pub visibility: Visibility,
    /// ノードID
    pub id: NodeId,
    /// 位置情報
    pub location: Option<SourceLocation>,
}

impl Locatable for VariableDeclaration {
    fn location(&self) -> Option<&SourceLocation> {
        self.location.as_ref()
    }
}

/// 定数宣言
#[derive(Debug, Clone)]
pub struct ConstantDeclaration {
    /// 定数名
    pub name: Identifier,
    /// 型注釈（オプション）
    pub type_annotation: Option<TypeAnnotation>,
    /// 初期化式（必須）
    pub initializer: Expression,
    /// 可視性
    pub visibility: Visibility,
    /// ノードID
    pub id: NodeId,
    /// 位置情報
    pub location: Option<SourceLocation>,
}

impl Locatable for ConstantDeclaration {
    fn location(&self) -> Option<&SourceLocation> {
        self.location.as_ref()
    }
}

/// 関数宣言
#[derive(Debug, Clone)]
pub struct Function {
    /// 関数名
    pub name: Identifier,
    /// 型パラメータ（ジェネリクス用）
    pub type_parameters: Vec<TypeParameter>,
    /// 引数リスト
    pub parameters: Vec<Parameter>,
    /// 戻り値型（オプション）
    pub return_type: Option<TypeAnnotation>,
    /// 関数本体
    pub body: Statement,
    /// 可視性
    pub visibility: Visibility,
    /// 非同期関数かどうか
    pub is_async: bool,
    /// 外部関数かどうか
    pub is_extern: bool,
    /// ノードID
    pub id: NodeId,
    /// 位置情報
    pub location: Option<SourceLocation>,
    pub(crate) basic_blocks: Vec<_>,
    pub(crate) basic_blocks: Vec<_>,
    pub(crate) is_declaration: bool,
    pub(crate) is_intrinsic: bool,
    pub(crate) attributes: Vec<_>,
}

impl Locatable for Function {
    fn location(&self) -> Option<&SourceLocation> {
        self.location.as_ref()
    }
}

/// 関数パラメータ
#[derive(Debug, Clone)]
pub struct Parameter {
    /// パラメータ名
    pub name: Identifier,
    /// 型注釈
    pub type_annotation: Option<TypeAnnotation>,
    /// デフォルト値（オプション）
    pub default_value: Option<Expression>,
    /// 可変パラメータかどうか
    pub is_mutable: bool,
    /// ノードID
    pub id: NodeId,
    /// 位置情報
    pub location: Option<SourceLocation>,
}

impl Locatable for Parameter {
    fn location(&self) -> Option<&SourceLocation> {
        self.location.as_ref()
    }
}

/// 型パラメータ（ジェネリック型用）
#[derive(Debug, Clone)]
pub struct TypeParameter {
    /// 型パラメータ名
    pub name: Identifier,
    /// 制約（トレイト境界）
    pub constraints: Vec<TypeAnnotation>,
    /// デフォルト型（オプション）
    pub default_type: Option<TypeAnnotation>,
    /// ノードID
    pub id: NodeId,
    /// 位置情報
    pub location: Option<SourceLocation>,
}

impl Locatable for TypeParameter {
    fn location(&self) -> Option<&SourceLocation> {
        self.location.as_ref()
    }
}

/// 構造体宣言
#[derive(Debug, Clone)]
pub struct Struct {
    /// 構造体名
    pub name: Identifier,
    /// 型パラメータ
    pub type_parameters: Vec<TypeParameter>,
    /// フィールドリスト
    pub fields: Vec<StructField>,
    /// 可視性
    pub visibility: Visibility,
    /// ノードID
    pub id: NodeId,
    /// 位置情報
    pub location: Option<SourceLocation>,
}

impl Locatable for Struct {
    fn location(&self) -> Option<&SourceLocation> {
        self.location.as_ref()
    }
}

/// 構造体フィールド
#[derive(Debug, Clone)]
pub struct StructField {
    /// フィールド名
    pub name: Identifier,
    /// 型注釈
    pub type_annotation: TypeAnnotation,
    /// 可視性
    pub visibility: Visibility,
    /// ノードID
    pub id: NodeId,
    /// 位置情報
    pub location: Option<SourceLocation>,
}

impl Locatable for StructField {
    fn location(&self) -> Option<&SourceLocation> {
        self.location.as_ref()
    }
}

/// 列挙型宣言
#[derive(Debug, Clone)]
pub struct Enum {
    /// 列挙型名
    pub name: Identifier,
    /// 型パラメータ
    pub type_parameters: Vec<TypeParameter>,
    /// バリアントリスト
    pub variants: Vec<EnumVariant>,
    /// 可視性
    pub visibility: Visibility,
    /// ノードID
    pub id: NodeId,
    /// 位置情報
    pub location: Option<SourceLocation>,
}

impl Locatable for Enum {
    fn location(&self) -> Option<&SourceLocation> {
        self.location.as_ref()
    }
}

/// 列挙型バリアント
#[derive(Debug, Clone)]
pub struct EnumVariant {
    /// バリアント名
    pub name: Identifier,
    /// 関連値（タプル型）
    pub associated_values: Option<Vec<TypeAnnotation>>,
    /// 関連値（構造体型）
    pub associated_fields: Option<Vec<EnumField>>,
    /// 識別子値（整数値など）
    pub discriminant: Option<Expression>,
    /// ノードID
    pub id: NodeId,
    /// 位置情報
    pub location: Option<SourceLocation>,
}

impl Locatable for EnumVariant {
    fn location(&self) -> Option<&SourceLocation> {
        self.location.as_ref()
    }
}

/// 列挙型フィールド（構造体型のバリアント用）
#[derive(Debug, Clone)]
pub struct EnumField {
    /// フィールド名
    pub name: Identifier,
    /// 型注釈
    pub type_annotation: TypeAnnotation,
    /// ノードID
    pub id: NodeId,
    /// 位置情報
    pub location: Option<SourceLocation>,
}

impl Locatable for EnumField {
    fn location(&self) -> Option<&SourceLocation> {
        self.location.as_ref()
    }
}

/// トレイト宣言
#[derive(Debug, Clone)]
pub struct Trait {
    /// トレイト名
    pub name: Identifier,
    /// 型パラメータ
    pub type_parameters: Vec<TypeParameter>,
    /// スーパートレイト（継承元）
    pub supertraits: Vec<TypeAnnotation>,
    /// 関連型
    pub associated_types: Vec<AssociatedType>,
    /// メソッド宣言
    pub methods: Vec<Function>,
    /// 可視性
    pub visibility: Visibility,
    /// ノードID
    pub id: NodeId,
    /// 位置情報
    pub location: Option<SourceLocation>,
}

impl Locatable for Trait {
    fn location(&self) -> Option<&SourceLocation> {
        self.location.as_ref()
    }
}

/// 関連型（トレイト内の型定義）
#[derive(Debug, Clone)]
pub struct AssociatedType {
    /// 型名
    pub name: Identifier,
    /// 制約（トレイト境界）
    pub constraints: Vec<TypeAnnotation>,
    /// デフォルト型（オプション）
    pub default_type: Option<TypeAnnotation>,
    /// ノードID
    pub id: NodeId,
    /// 位置情報
    pub location: Option<SourceLocation>,
}

impl Locatable for AssociatedType {
    fn location(&self) -> Option<&SourceLocation> {
        self.location.as_ref()
    }
}

/// 実装宣言
#[derive(Debug, Clone)]
pub struct Implementation {
    /// 実装対象の型
    pub target_type: TypeAnnotation,
    /// 実装するトレイト（オプション）
    pub trait_name: Option<Identifier>,
    /// 型パラメータ
    pub type_parameters: Vec<TypeParameter>,
    /// 関連型の定義
    pub associated_types: Option<Vec<(Identifier, TypeAnnotation)>>,
    /// メソッド実装
    pub methods: Vec<Function>,
    /// ノードID
    pub id: NodeId,
    /// 位置情報
    pub location: Option<SourceLocation>,
}

impl Locatable for Implementation {
    fn location(&self) -> Option<&SourceLocation> {
        self.location.as_ref()
    }
}

/// 型エイリアス
#[derive(Debug, Clone)]
pub struct TypeAlias {
    /// エイリアス名
    pub name: Identifier,
    /// 型パラメータ
    pub type_parameters: Vec<TypeParameter>,
    /// 元の型
    pub target_type: TypeAnnotation,
    /// 可視性
    pub visibility: Visibility,
    /// ノードID
    pub id: NodeId,
    /// 位置情報
    pub location: Option<SourceLocation>,
}

impl Locatable for TypeAlias {
    fn location(&self) -> Option<&SourceLocation> {
        self.location.as_ref()
    }
}

/// インポート宣言
#[derive(Debug, Clone)]
pub struct Import {
    /// インポートパス
    pub path: Vec<Identifier>,
    /// エイリアス（オプション）
    pub alias: Option<Identifier>,
    /// インポート種類
    pub kind: ImportKind,
    /// 絶対パスかどうか
    pub is_absolute: bool,
    /// シンボルの上書きを許可するか
    pub allow_overrides: bool,
    /// ノードID
    pub id: NodeId,
    /// 位置情報
    pub location: Option<SourceLocation>,
}

impl Locatable for Import {
    fn location(&self) -> Option<&SourceLocation> {
        self.location.as_ref()
    }
}

/// インポート種類
#[derive(Debug, Clone)]
pub enum ImportKind {
    /// モジュール内のすべてのシンボルをインポート (import module.*)
    AllSymbols,
    
    /// 指定されたシンボルのみをインポート (import module.{Symbol1, Symbol2})
    SelectedSymbols(Vec<ImportSymbol>),
    
    /// モジュール自体のみをインポート (import module)
    ModuleOnly,
}

/// インポートするシンボル
#[derive(Debug, Clone)]
pub struct ImportSymbol {
    /// シンボル名
    pub name: Identifier,
    
    /// エイリアス（オプション）
    pub alias: Option<Identifier>,
    
    /// ノードID
    pub id: NodeId,
    
    /// 位置情報
    pub location: Option<SourceLocation>,
}

/// モジュール宣言
#[derive(Debug, Clone)]
pub struct Module {
    /// モジュール名
    pub name: Identifier,
    /// ノードID
    pub id: NodeId,
    /// 位置情報
    pub location: Option<SourceLocation>,
}

impl Locatable for Module {
    fn location(&self) -> Option<&SourceLocation> {
        self.location.as_ref()
    }
}

/// 文ノード
#[derive(Debug, Clone)]
pub struct Statement {
    /// 文の種類
    pub kind: StatementKind,
    /// ノードID
    pub id: NodeId,
    /// 位置情報
    pub location: Option<SourceLocation>,
}

impl Statement {
    /// 新しい文を作成
    pub fn new(kind: StatementKind, id: NodeId, location: Option<SourceLocation>) -> Self {
        Self {
            kind,
            id,
            location,
        }
    }
}

impl Locatable for Statement {
    fn location(&self) -> Option<&SourceLocation> {
        self.location.as_ref()
    }
}

/// 文の種類
#[derive(Debug, Clone)]
pub enum StatementKind {
    /// 式文 (expr;)
    ExpressionStatement(Expression),
    /// ブロック文 ({ ... })
    Block(Vec<Statement>),
    /// 変数宣言文 (let x = 1;)
    VariableDeclaration(VariableDeclaration),
    /// 定数宣言文 (const PI = 3.14;)
    ConstantDeclaration(ConstantDeclaration),
    /// if文 (if cond { ... } else { ... })
    IfStatement {
        /// 条件式
        condition: Expression,
        /// then部分
        then_branch: Box<Statement>,
        /// else部分（オプション）
        else_branch: Option<Box<Statement>>,
    },
    /// while文 (while cond { ... })
    WhileStatement {
        /// 条件式
        condition: Expression,
        /// ループ本体
        body: Box<Statement>,
    },
    /// for文 (for x in iterable { ... })
    ForStatement {
        /// イテレーション変数
        variable: Identifier,
        /// イテレーション対象
        iterable: Expression,
        /// ループ本体
        body: Box<Statement>,
    },
    /// return文 (return expr;)
    ReturnStatement(Option<Expression>),
    /// break文 (break;)
    BreakStatement,
    /// continue文 (continue;)
    ContinueStatement,
    /// 空文 (;)
    EmptyStatement,
}

/// 式ノード
#[derive(Debug, Clone)]
pub struct Expression {
    /// 式の種類
    pub kind: ExpressionKind,
    /// ノードID
    pub id: NodeId,
    /// 位置情報
    pub location: Option<SourceLocation>,
}

impl Expression {
    /// 新しい式を作成
    pub fn new(kind: ExpressionKind, id: NodeId, location: Option<SourceLocation>) -> Self {
        Self {
            kind,
            id,
            location,
        }
    }
}

impl Locatable for Expression {
    fn location(&self) -> Option<&SourceLocation> {
        self.location.as_ref()
    }
}

/// 式の種類
#[derive(Debug, Clone)]
pub enum ExpressionKind {
    /// リテラル (1, "hello", true など)
    Literal(Literal),
    /// 識別子 (変数名など)
    Identifier(Identifier),
    /// 二項演算 (a + b など)
    BinaryOp(BinaryOperator, Box<Expression>, Box<Expression>),
    /// 単項演算 (!x, -y など)
    UnaryOp(UnaryOperator, Box<Expression>),
    /// 関数呼び出し (func(arg1, arg2))
    Call(Box<Expression>, Vec<Expression>),
    /// メンバーアクセス (obj.member)
    MemberAccess(Box<Expression>, Identifier),
    /// インデックスアクセス (array[index])
    IndexAccess(Box<Expression>, Box<Expression>),
    /// 配列リテラル ([1, 2, 3])
    ArrayLiteral(Vec<Expression>),
    /// 構造体リテラル (Point { x: 1, y: 2 })
    StructLiteral(Identifier, Vec<(Identifier, Expression)>),
    /// タプルリテラル ((1, "hello", true))
    TupleLiteral(Vec<Expression>),
    /// キャスト式 (expr as Type)
    Cast(Box<Expression>, TypeAnnotation),
    /// ラムダ式 (|x, y| x + y)
    Lambda(Vec<Parameter>, Box<Expression>),
    /// ブロック式 ({ let x = 1; x + 2 })
    BlockExpr(Vec<Statement>, Option<Box<Expression>>),
    /// if式 (if cond { expr1 } else { expr2 })
    IfExpr(Box<Expression>, Box<Expression>, Option<Box<Expression>>),
    /// match式 (match expr { pat1 => expr1, pat2 => expr2 })
    MatchExpr(Box<Expression>, Vec<(Expression, Expression)>),
    /// try-catch式 (try { ... } catch e: Error { ... })
    TryCatchExpr(Box<Expression>, Vec<(TypeAnnotation, Identifier, Expression)>),
    /// throw式 (throw error)
    ThrowExpr(Box<Expression>),
    /// async式 (async { ... })
    AsyncExpr(Box<Expression>),
    /// await式 (await expr)
    AwaitExpr(Box<Expression>),
    /// range式 (1..10, 1..=10)
    RangeExpr(Box<Expression>, Box<Expression>, bool),
}

/// 識別子
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Identifier {
    /// 識別子名
    pub name: String,
    /// ノードID
    pub id: NodeId,
    /// 位置情報
    pub location: Option<SourceLocation>,
}

impl Identifier {
    /// 新しい識別子を作成
    pub fn new(name: impl Into<String>, id: NodeId, location: Option<SourceLocation>) -> Self {
        Self {
            name: name.into(),
            id,
            location,
        }
    }
}

impl Locatable for Identifier {
    fn location(&self) -> Option<&SourceLocation> {
        self.location.as_ref()
    }
}

impl fmt::Display for Identifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

/// リテラル
#[derive(Debug, Clone)]
pub struct Literal {
    /// リテラルの種類
    pub kind: LiteralKind,
    /// ノードID
    pub id: NodeId,
    /// 位置情報
    pub location: Option<SourceLocation>,
}

impl Literal {
    /// 新しいリテラルを作成
    pub fn new(kind: LiteralKind, id: NodeId, location: Option<SourceLocation>) -> Self {
        Self {
            kind,
            id,
            location,
        }
    }
}

impl Locatable for Literal {
    fn location(&self) -> Option<&SourceLocation> {
        self.location.as_ref()
    }
}

/// リテラルの種類
#[derive(Debug, Clone)]
pub enum LiteralKind {
    /// 整数リテラル
    Integer(i64),
    /// 浮動小数点リテラル
    Float(f64),
    /// 文字列リテラル
    String(String),
    /// 文字リテラル
    Char(char),
    /// 真偽値リテラル
    Boolean(bool),
    /// nil (null) リテラル
    Nil,
}

/// 型注釈
#[derive(Debug, Clone)]
pub struct TypeAnnotation {
    /// 型の種類
    pub kind: TypeKind,
    /// ノードID
    pub id: NodeId,
    /// 位置情報
    pub location: Option<SourceLocation>,
}

impl TypeAnnotation {
    /// 新しい型注釈を作成
    pub fn new(kind: TypeKind, id: NodeId, location: Option<SourceLocation>) -> Self {
        Self {
            kind,
            id,
            location,
        }
    }
    
    /// デフォルトの型注釈を作成（実際には使用されない）
    pub fn default() -> Self {
        Self {
            kind: TypeKind::Any,
            id: 0,
            location: None,
        }
    }
}

impl Locatable for TypeAnnotation {
    fn location(&self) -> Option<&SourceLocation> {
        self.location.as_ref()
    }
}

/// 型の種類
#[derive(Debug, Clone)]
pub enum TypeKind {
    /// 任意の型 (any)
    Any,
    /// 名前付き型（ユーザー定義型、組み込み型など）
    Named(Identifier),
    /// 参照型 (&T, &mut T)
    Reference(Box<TypeAnnotation>, bool),
    /// ポインタ型 (*T, *mut T)
    Pointer(Box<TypeAnnotation>, bool),
    /// 配列型 ([T; N] または [T])
    Array(Box<TypeAnnotation>, Option<usize>),
    /// スライス型 (&[T])
    Slice(Box<TypeAnnotation>),
    /// 関数型 (fn(T1, T2) -> R)
    Function(Vec<TypeAnnotation>, Option<Box<TypeAnnotation>>),
    /// タプル型 ((T1, T2, T3))
    Tuple(Vec<TypeAnnotation>),
    /// オプション型 (Option<T>)
    Optional(Box<TypeAnnotation>),
    /// ジェネリック型の適用 (Vec<T>, Map<K, V> など)
    Generic(Box<TypeAnnotation>, Vec<TypeAnnotation>),
    /// 存在型 (impl Trait)
    Existential(Box<TypeAnnotation>),
    /// パス型 (module::Type)
    Path(Vec<Identifier>, Box<TypeAnnotation>),
    /// 「ワイルドカード」型（型推論用）
    Inferred,
    /// 自己型（Self）
    SelfType,
    /// ユニット型 (())
    Unit,
    /// 決して値を返さない型 (!)
    Never,
    /// エラー型（型チェック失敗時に使用）
    Error,
}

/// 二項演算子
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOperator {
    // 算術演算子
    Add,        // +
    Subtract,   // -
    Multiply,   // *
    Divide,     // /
    Modulo,     // %
    Power,      // **
    
    // 比較演算子
    Equal,           // ==
    NotEqual,        // !=
    LessThan,        // <
    GreaterThan,     // >
    LessEqual,       // <=
    GreaterEqual,    // >=
    
    // 論理演算子
    And,        // &&
    Or,         // ||
    
    // ビット演算子
    BitAnd,     // &
    BitOr,      // |
    BitXor,     // ^
    LeftShift,  // <<
    RightShift, // >>
    
    // 代入演算子
    Assign,         // =
    AddAssign,      // +=
    SubtractAssign, // -=
    MultiplyAssign, // *=
    DivideAssign,   // /=
    ModuloAssign,   // %=
    BitAndAssign,   // &=
    BitOrAssign,    // |=
    BitXorAssign,   // ^=
    LeftShiftAssign,  // <<=
    RightShiftAssign, // >>=
    
    // Range演算子
    Range,          // ..
    RangeInclusive, // ..=
    
    // その他
    NullCoalesce,   // ??
}

/// 単項演算子
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOperator {
    // 算術演算子
    Plus,       // +x
    Minus,      // -x
    
    // 論理演算子
    Not,        // !x
    
    // ビット演算子
    BitNot,     // ~x
    
    // 参照演算子
    Ref,        // &x
    RefMut,     // &mut x
    Deref,      // *x
    
    // その他
    PreIncrement,   // ++x
    PreDecrement,   // --x
    PostIncrement,  // x++
    PostDecrement,  // x--
}

/// 可視性
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Visibility {
    /// パブリック（モジュール外からアクセス可能）
    Public,
    /// プライベート（モジュール内からのみアクセス可能）
    Private,
    /// 内部（パッケージ内からのみアクセス可能）
    Internal,
    /// 制限付き（指定されたモジュールからのみアクセス可能）
    Restricted(Vec<String>),
}

impl Default for Visibility {
    fn default() -> Self {
        Self::Private
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_create_identifier() {
        let location = SourceLocation::new("test.swl", 1, 1, 0, 3);
        let id = Identifier::new("foo", 1, Some(location.clone()));
        
        assert_eq!(id.name, "foo");
        assert_eq!(id.id, 1);
        assert_eq!(id.location, Some(location));
    }
    
    #[test]
    fn test_create_expression() {
        let location = SourceLocation::new("test.swl", 1, 1, 0, 3);
        let id = Identifier::new("foo", 1, Some(location.clone()));
        let expr = Expression::new(ExpressionKind::Identifier(id), 2, Some(location.clone()));
        
        assert_eq!(expr.id, 2);
        assert_eq!(expr.location, Some(location));
        
        if let ExpressionKind::Identifier(ident) = &expr.kind {
            assert_eq!(ident.name, "foo");
        } else {
            panic!("Expected Identifier expression");
        }
    }
    
    #[test]
    fn test_create_binary_op() {
        let location = SourceLocation::new("test.swl", 1, 1, 0, 7);
        
        // Create left operand: 1
        let left_lit = Literal::new(LiteralKind::Integer(1), 1, Some(location.clone()));
        let left = Expression::new(ExpressionKind::Literal(left_lit), 2, Some(location.clone()));
        
        // Create right operand: 2
        let right_lit = Literal::new(LiteralKind::Integer(2), 3, Some(location.clone()));
        let right = Expression::new(ExpressionKind::Literal(right_lit), 4, Some(location.clone()));
        
        // Create binary op: 1 + 2
        let binary_op = Expression::new(
            ExpressionKind::BinaryOp(BinaryOperator::Add, Box::new(left), Box::new(right)),
            5,
            Some(location.clone()),
        );
        
        assert_eq!(binary_op.id, 5);
        
        if let ExpressionKind::BinaryOp(op, left_expr, right_expr) = &binary_op.kind {
            assert_eq!(*op, BinaryOperator::Add);
            
            if let ExpressionKind::Literal(left_lit) = &left_expr.kind {
                if let LiteralKind::Integer(val) = left_lit.kind {
                    assert_eq!(val, 1);
                } else {
                    panic!("Expected Integer literal");
                }
            } else {
                panic!("Expected Literal expression");
            }
            
            if let ExpressionKind::Literal(right_lit) = &right_expr.kind {
                if let LiteralKind::Integer(val) = right_lit.kind {
                    assert_eq!(val, 2);
                } else {
                    panic!("Expected Integer literal");
                }
            } else {
                panic!("Expected Literal expression");
            }
        } else {
            panic!("Expected BinaryOp expression");
        }
    }
} 