//! # 抽象構文木 (AST)
//! 
//! SwiftLight言語のソースコードを解析して得られる抽象構文木を定義します。
//! プログラムの構文構造を表現するノード型を提供します。

use std::fmt::{self, Debug};
use std::collections::HashMap;

use crate::frontend::error::SourceLocation;

/// ノードIDを表す型
pub type NodeId = usize;

/// 位置情報を持つ型のトレイト
pub trait Locatable {
    /// ノードの位置情報を取得
    fn location(&self) -> Option<&SourceLocation>;
}

/// 識別子を表す型
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Identifier {
    /// 識別子の名前
    pub name: String,
    /// ソースコード内の位置
    pub location: Option<SourceLocation>,
}

impl Identifier {
    /// 新しい識別子を作成
    pub fn new(name: impl Into<String>, location: Option<SourceLocation>) -> Self {
        Self {
            name: name.into(),
            location,
        }
    }
}

impl Locatable for Identifier {
    fn location(&self) -> Option<&SourceLocation> {
        self.location.as_ref()
    }
}

/// リテラル値の種類
#[derive(Debug, Clone, PartialEq)]
pub enum LiteralKind {
    /// 整数リテラル（例: 42）
    Integer(i64),
    /// 浮動小数点リテラル（例: 3.14）
    Float(f64),
    /// 文字列リテラル（例: "hello"）
    String(String),
    /// 文字リテラル（例: 'a'）
    Char(char),
    /// 真偽値リテラル（例: true, false）
    Boolean(bool),
    /// nil（nullptr, null）
    Nil,
}

/// リテラル式
#[derive(Debug, Clone, PartialEq)]
pub struct Literal {
    /// リテラルの種類
    pub kind: LiteralKind,
    /// ソースコード内の位置
    pub location: Option<SourceLocation>,
}

impl Literal {
    /// 新しいリテラルを作成
    pub fn new(kind: LiteralKind, location: Option<SourceLocation>) -> Self {
        Self { kind, location }
    }
    
    /// 整数リテラルを作成
    pub fn integer(value: i64, location: Option<SourceLocation>) -> Self {
        Self::new(LiteralKind::Integer(value), location)
    }
    
    /// 浮動小数点リテラルを作成
    pub fn float(value: f64, location: Option<SourceLocation>) -> Self {
        Self::new(LiteralKind::Float(value), location)
    }
    
    /// 文字列リテラルを作成
    pub fn string(value: impl Into<String>, location: Option<SourceLocation>) -> Self {
        Self::new(LiteralKind::String(value.into()), location)
    }
    
    /// 文字リテラルを作成
    pub fn character(value: char, location: Option<SourceLocation>) -> Self {
        Self::new(LiteralKind::Char(value), location)
    }
    
    /// 真偽値リテラルを作成
    pub fn boolean(value: bool, location: Option<SourceLocation>) -> Self {
        Self::new(LiteralKind::Boolean(value), location)
    }
    
    /// nilリテラルを作成
    pub fn nil(location: Option<SourceLocation>) -> Self {
        Self::new(LiteralKind::Nil, location)
    }
}

impl Locatable for Literal {
    fn location(&self) -> Option<&SourceLocation> {
        self.location.as_ref()
    }
}

/// 二項演算子
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOperator {
    /// 加算 (+)
    Add,
    /// 減算 (-)
    Subtract,
    /// 乗算 (*)
    Multiply,
    /// 除算 (/)
    Divide,
    /// 剰余 (%)
    Modulo,
    /// べき乗 (**)
    Power,
    
    /// 等価 (==)
    Equal,
    /// 非等価 (!=)
    NotEqual,
    /// 小なり (<)
    LessThan,
    /// 大なり (>)
    GreaterThan,
    /// 以下 (<=)
    LessThanEqual,
    /// 以上 (>=)
    GreaterThanEqual,
    
    /// 論理積 (&&)
    LogicalAnd,
    /// 論理和 (||)
    LogicalOr,
    
    /// ビット積 (&)
    BitwiseAnd,
    /// ビット和 (|)
    BitwiseOr,
    /// ビット排他的論理和 (^)
    BitwiseXor,
    /// 左シフト (<<)
    LeftShift,
    /// 右シフト (>>)
    RightShift,
}

impl fmt::Display for BinaryOperator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let op_str = match self {
            BinaryOperator::Add => "+",
            BinaryOperator::Subtract => "-",
            BinaryOperator::Multiply => "*",
            BinaryOperator::Divide => "/",
            BinaryOperator::Modulo => "%",
            BinaryOperator::Power => "**",
            
            BinaryOperator::Equal => "==",
            BinaryOperator::NotEqual => "!=",
            BinaryOperator::LessThan => "<",
            BinaryOperator::GreaterThan => ">",
            BinaryOperator::LessThanEqual => "<=",
            BinaryOperator::GreaterThanEqual => ">=",
            
            BinaryOperator::LogicalAnd => "&&",
            BinaryOperator::LogicalOr => "||",
            
            BinaryOperator::BitwiseAnd => "&",
            BinaryOperator::BitwiseOr => "|",
            BinaryOperator::BitwiseXor => "^",
            BinaryOperator::LeftShift => "<<",
            BinaryOperator::RightShift => ">>",
        };
        write!(f, "{}", op_str)
    }
}

/// 単項演算子
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOperator {
    /// 単項プラス (+)
    Plus,
    /// 単項マイナス (-)
    Minus,
    /// 論理否定 (!)
    Not,
    /// ビット否定 (~)
    BitwiseNot,
}

impl fmt::Display for UnaryOperator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let op_str = match self {
            UnaryOperator::Plus => "+",
            UnaryOperator::Minus => "-",
            UnaryOperator::Not => "!",
            UnaryOperator::BitwiseNot => "~",
        };
        write!(f, "{}", op_str)
    }
}

/// 式の種類
#[derive(Debug, Clone, PartialEq)]
pub enum ExpressionKind {
    /// リテラル
    Literal(Literal),
    /// 変数参照
    Identifier(Identifier),
    /// 二項演算
    Binary {
        /// 演算子
        op: BinaryOperator,
        /// 左辺
        left: Box<Expression>,
        /// 右辺
        right: Box<Expression>,
    },
    /// 単項演算
    Unary {
        /// 演算子
        op: UnaryOperator,
        /// オペランド
        operand: Box<Expression>,
    },
    /// 関数呼び出し
    Call {
        /// 呼び出す関数
        callee: Box<Expression>,
        /// 引数リスト
        arguments: Vec<Expression>,
    },
    /// メンバアクセス (obj.member)
    MemberAccess {
        /// オブジェクト
        object: Box<Expression>,
        /// メンバ名
        member: Identifier,
    },
    /// インデックスアクセス (array[index])
    IndexAccess {
        /// 配列
        array: Box<Expression>,
        /// インデックス
        index: Box<Expression>,
    },
    /// 配列リテラル [a, b, c]
    ArrayLiteral(Vec<Expression>),
    /// オブジェクトリテラル {key: value, ...}
    ObjectLiteral(Vec<(Identifier, Expression)>),
    /// ラムダ式
    Lambda {
        /// パラメータ
        params: Vec<Parameter>,
        /// ラムダの本体
        body: Box<Statement>,
    },
    /// 条件式 (三項演算子) cond ? then : else
    Conditional {
        /// 条件
        condition: Box<Expression>,
        /// 真の場合の式
        then_branch: Box<Expression>,
        /// 偽の場合の式
        else_branch: Box<Expression>,
    },
    /// 割り当て式 (a = b)
    Assignment {
        /// 左辺値
        left: Box<Expression>,
        /// 右辺値
        right: Box<Expression>,
    },
}

/// 式
#[derive(Debug, Clone, PartialEq)]
pub struct Expression {
    /// 式の種類
    pub kind: ExpressionKind,
    /// ノードID
    pub id: NodeId,
    /// ソースコード内の位置
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

/// 文の種類
#[derive(Debug, Clone, PartialEq)]
pub enum StatementKind {
    /// 式文
    Expression(Expression),
    /// ブロック文
    Block(Vec<Statement>),
    /// 変数宣言
    VariableDeclaration {
        /// 変数名
        name: Identifier,
        /// 型注釈（オプション）
        type_annotation: Option<TypeAnnotation>,
        /// 初期化式（オプション）
        initializer: Option<Expression>,
    },
    /// 定数宣言
    ConstantDeclaration {
        /// 定数名
        name: Identifier,
        /// 型注釈（オプション）
        type_annotation: Option<TypeAnnotation>,
        /// 初期化式
        initializer: Expression,
    },
    /// if文
    If {
        /// 条件式
        condition: Expression,
        /// 真の場合の文
        then_branch: Box<Statement>,
        /// 偽の場合の文（オプション）
        else_branch: Option<Box<Statement>>,
    },
    /// while文
    While {
        /// 条件式
        condition: Expression,
        /// ループ本体
        body: Box<Statement>,
    },
    /// for文
    For {
        /// イテレータ変数
        variable: Identifier,
        /// イテレート対象の式
        iterable: Expression,
        /// ループ本体
        body: Box<Statement>,
    },
    /// 関数宣言
    FunctionDeclaration {
        /// 関数名
        name: Identifier,
        /// パラメータリスト
        params: Vec<Parameter>,
        /// 戻り値の型（オプション）
        return_type: Option<TypeAnnotation>,
        /// 関数本体
        body: Box<Statement>,
    },
    /// return文
    Return(Option<Expression>),
    /// break文
    Break,
    /// continue文
    Continue,
    /// enum宣言
    EnumDeclaration {
        /// 列挙型名
        name: Identifier,
        /// バリアント
        variants: Vec<EnumVariant>,
    },
    /// struct宣言
    StructDeclaration {
        /// 構造体名
        name: Identifier,
        /// フィールド
        fields: Vec<StructField>,
    },
    /// trait宣言
    TraitDeclaration {
        /// トレイト名
        name: Identifier,
        /// メソッド宣言
        methods: Vec<TraitMethod>,
    },
    /// impl宣言（トレイト実装）
    ImplDeclaration {
        /// 実装する型
        target_type: TypeAnnotation,
        /// 実装するトレイト（オプション）
        trait_name: Option<Identifier>,
        /// メソッド実装
        methods: Vec<Statement>,
    },
    /// import文
    Import {
        /// インポートパス
        path: Vec<Identifier>,
        /// エイリアス（オプション）
        alias: Option<Identifier>,
    },
    /// エラー文（構文解析中にエラーが発生した場合）
    Error,
}

/// 文
#[derive(Debug, Clone, PartialEq)]
pub struct Statement {
    /// 文の種類
    pub kind: StatementKind,
    /// ノードID
    pub id: NodeId,
    /// ソースコード内の位置
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

/// 関数パラメータ
#[derive(Debug, Clone, PartialEq)]
pub struct Parameter {
    /// パラメータ名
    pub name: Identifier,
    /// 型注釈
    pub type_annotation: Option<TypeAnnotation>,
    /// デフォルト値（オプション）
    pub default_value: Option<Expression>,
    /// 可変パラメータかどうか
    pub is_variadic: bool,
    /// ソースコード内の位置
    pub location: Option<SourceLocation>,
}

impl Parameter {
    /// 新しいパラメータを作成
    pub fn new(
        name: Identifier,
        type_annotation: Option<TypeAnnotation>,
        default_value: Option<Expression>,
        is_variadic: bool,
        location: Option<SourceLocation>,
    ) -> Self {
        Self {
            name,
            type_annotation,
            default_value,
            is_variadic,
            location,
        }
    }
}

impl Locatable for Parameter {
    fn location(&self) -> Option<&SourceLocation> {
        self.location.as_ref()
    }
}

/// 型注釈
#[derive(Debug, Clone, PartialEq)]
pub enum TypeAnnotation {
    /// 名前付き型（基本型や定義型）
    Named(Identifier),
    /// ジェネリック型（例: Array<T>）
    Generic {
        /// 基本型
        base: Box<TypeAnnotation>,
        /// 型引数
        arguments: Vec<TypeAnnotation>,
    },
    /// 関数型（例: (Int, String) -> Bool）
    Function {
        /// パラメータ型
        parameters: Vec<TypeAnnotation>,
        /// 戻り値型
        return_type: Box<TypeAnnotation>,
    },
    /// タプル型（例: (Int, String)）
    Tuple(Vec<TypeAnnotation>),
    /// 配列型（例: [Int]）
    Array(Box<TypeAnnotation>),
    /// オプショナル型（例: Int?）
    Optional(Box<TypeAnnotation>),
    /// ユニオン型（例: Int | String）
    Union(Vec<TypeAnnotation>),
    /// 交差型（例: A & B）
    Intersection(Vec<TypeAnnotation>),
}

impl TypeAnnotation {
    /// 型の名前を文字列として取得
    pub fn name(&self) -> String {
        match self {
            TypeAnnotation::Named(ident) => ident.name.clone(),
            TypeAnnotation::Generic { base, arguments } => {
                let args = arguments
                    .iter()
                    .map(|arg| arg.name())
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{}<{}>", base.name(), args)
            }
            TypeAnnotation::Function { parameters, return_type } => {
                let params = parameters
                    .iter()
                    .map(|param| param.name())
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("({}) -> {}", params, return_type.name())
            }
            TypeAnnotation::Tuple(types) => {
                let types_str = types
                    .iter()
                    .map(|t| t.name())
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("({})", types_str)
            }
            TypeAnnotation::Array(element_type) => {
                format!("[{}]", element_type.name())
            }
            TypeAnnotation::Optional(inner) => {
                format!("{}?", inner.name())
            }
            TypeAnnotation::Union(types) => {
                types
                    .iter()
                    .map(|t| t.name())
                    .collect::<Vec<_>>()
                    .join(" | ")
            }
            TypeAnnotation::Intersection(types) => {
                types
                    .iter()
                    .map(|t| t.name())
                    .collect::<Vec<_>>()
                    .join(" & ")
            }
        }
    }
}

/// 列挙型のバリアント
#[derive(Debug, Clone, PartialEq)]
pub struct EnumVariant {
    /// バリアント名
    pub name: Identifier,
    /// 関連値の型（オプション）
    pub associated_values: Option<Vec<TypeAnnotation>>,
    /// ソースコード内の位置
    pub location: Option<SourceLocation>,
}

impl EnumVariant {
    /// 新しい列挙型バリアントを作成
    pub fn new(
        name: Identifier,
        associated_values: Option<Vec<TypeAnnotation>>,
        location: Option<SourceLocation>,
    ) -> Self {
        Self {
            name,
            associated_values,
            location,
        }
    }
}

impl Locatable for EnumVariant {
    fn location(&self) -> Option<&SourceLocation> {
        self.location.as_ref()
    }
}

/// 構造体のフィールド
#[derive(Debug, Clone, PartialEq)]
pub struct StructField {
    /// フィールド名
    pub name: Identifier,
    /// 型注釈
    pub type_annotation: TypeAnnotation,
    /// デフォルト値（オプション）
    pub default_value: Option<Expression>,
    /// ソースコード内の位置
    pub location: Option<SourceLocation>,
}

impl StructField {
    /// 新しい構造体フィールドを作成
    pub fn new(
        name: Identifier,
        type_annotation: TypeAnnotation,
        default_value: Option<Expression>,
        location: Option<SourceLocation>,
    ) -> Self {
        Self {
            name,
            type_annotation,
            default_value,
            location,
        }
    }
}

impl Locatable for StructField {
    fn location(&self) -> Option<&SourceLocation> {
        self.location.as_ref()
    }
}

/// トレイトのメソッド宣言
#[derive(Debug, Clone, PartialEq)]
pub struct TraitMethod {
    /// メソッド名
    pub name: Identifier,
    /// パラメータリスト
    pub params: Vec<Parameter>,
    /// 戻り値の型（オプション）
    pub return_type: Option<TypeAnnotation>,
    /// デフォルト実装（オプション）
    pub default_impl: Option<Box<Statement>>,
    /// ソースコード内の位置
    pub location: Option<SourceLocation>,
}

impl TraitMethod {
    /// 新しいトレイトメソッドを作成
    pub fn new(
        name: Identifier,
        params: Vec<Parameter>,
        return_type: Option<TypeAnnotation>,
        default_impl: Option<Box<Statement>>,
        location: Option<SourceLocation>,
    ) -> Self {
        Self {
            name,
            params,
            return_type,
            default_impl,
            location,
        }
    }
}

impl Locatable for TraitMethod {
    fn location(&self) -> Option<&SourceLocation> {
        self.location.as_ref()
    }
}

/// プログラム（ASTのルート）
#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    /// 文のリスト
    pub statements: Vec<Statement>,
    /// ノードID生成用のカウンター
    next_id: NodeId,
    /// ソースファイル名
    pub file_name: String,
}

impl Program {
    /// 新しいプログラムを作成
    pub fn new(file_name: impl Into<String>) -> Self {
        Self {
            statements: Vec::new(),
            next_id: 1, // 0は特別な値として予約
            file_name: file_name.into(),
        }
    }
    
    /// 文を追加
    pub fn add_statement(&mut self, statement: Statement) {
        self.statements.push(statement);
    }
    
    /// 新しいノードIDを取得
    pub fn next_id(&mut self) -> NodeId {
        let id = self.next_id;
        self.next_id += 1;
        id
    }
    
    /// 文の数を取得
    pub fn len(&self) -> usize {
        self.statements.len()
    }
    
    /// プログラムが空かどうかを判定
    pub fn is_empty(&self) -> bool {
        self.statements.is_empty()
    }
}

/// テスト
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_expression_creation() {
        let mut program = Program::new("test.swl");
        
        // リテラル式の作成
        let literal = Literal::integer(42, None);
        let expr = Expression::new(
            ExpressionKind::Literal(literal.clone()),
            program.next_id(),
            None,
        );
        
        // 式の種類を確認
        match &expr.kind {
            ExpressionKind::Literal(lit) => {
                assert_eq!(lit, &literal);
                match lit.kind {
                    LiteralKind::Integer(val) => assert_eq!(val, 42),
                    _ => panic!("予期しないリテラル型"),
                }
            }
            _ => panic!("予期しない式の種類"),
        }
    }
    
    #[test]
    fn test_statement_creation() {
        let mut program = Program::new("test.swl");
        
        // 変数名
        let var_name = Identifier::new("x", None);
        
        // 初期化式
        let initializer = Expression::new(
            ExpressionKind::Literal(Literal::integer(10, None)),
            program.next_id(),
            None,
        );
        
        // 変数宣言文の作成
        let stmt = Statement::new(
            StatementKind::VariableDeclaration {
                name: var_name.clone(),
                type_annotation: None,
                initializer: Some(initializer),
            },
            program.next_id(),
            None,
        );
        
        // 文の種類を確認
        match &stmt.kind {
            StatementKind::VariableDeclaration { name, initializer, .. } => {
                assert_eq!(name, &var_name);
                assert!(initializer.is_some());
            }
            _ => panic!("予期しない文の種類"),
        }
    }
    
    #[test]
    fn test_type_annotation_name() {
        // 基本型
        let int_type = TypeAnnotation::Named(Identifier::new("Int", None));
        assert_eq!(int_type.name(), "Int");
        
        // 配列型
        let array_type = TypeAnnotation::Array(Box::new(int_type.clone()));
        assert_eq!(array_type.name(), "[Int]");
        
        // ジェネリック型
        let generic_type = TypeAnnotation::Generic {
            base: Box::new(TypeAnnotation::Named(Identifier::new("Array", None))),
            arguments: vec![int_type.clone()],
        };
        assert_eq!(generic_type.name(), "Array<Int>");
        
        // 関数型
        let func_type = TypeAnnotation::Function {
            parameters: vec![int_type.clone(), TypeAnnotation::Named(Identifier::new("String", None))],
            return_type: Box::new(TypeAnnotation::Named(Identifier::new("Bool", None))),
        };
        assert_eq!(func_type.name(), "(Int, String) -> Bool");
    }
} 