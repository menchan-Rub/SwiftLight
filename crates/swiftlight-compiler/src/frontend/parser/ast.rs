/// # 抽象構文木（AST）の定義
/// 
/// このモジュールはSwiftLight言語の抽象構文木を表現する構造体と列挙型を定義します。
/// ASTはパーサーによって生成され、型チェッカーや中間表現（IR）生成器によって使用されます。

use std::fmt;
use std::rc::Rc;
use std::collections::HashMap;

/// ソースコード内の位置情報
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    /// 開始位置（バイトオフセット）
    pub start: usize,
    /// 終了位置（バイトオフセット）
    pub end: usize,
}

impl Span {
    /// 新しいSpanを作成
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    /// 2つのSpanを結合
    pub fn merge(&self, other: &Span) -> Self {
        Self {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        }
    }
}

/// 識別子
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Identifier {
    /// 識別子の名前
    pub name: String,
    /// ソースコード内の位置
    pub span: Span,
}

impl Identifier {
    /// 新しい識別子を作成
    pub fn new(name: String, span: Span) -> Self {
        Self { name, span }
    }
}

/// リテラル値
#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    /// 整数リテラル
    Integer(i64, Span),
    /// 浮動小数点数リテラル
    Float(f64, Span),
    /// 文字列リテラル
    String(String, Span),
    /// 文字リテラル
    Char(char, Span),
    /// 真偽値リテラル
    Boolean(bool, Span),
    /// ユニットリテラル（空のタプル）
    Unit(Span),
}

impl Literal {
    /// リテラルの位置情報を取得
    pub fn span(&self) -> Span {
        match self {
            Literal::Integer(_, span) => *span,
            Literal::Float(_, span) => *span,
            Literal::String(_, span) => *span,
            Literal::Char(_, span) => *span,
            Literal::Boolean(_, span) => *span,
            Literal::Unit(span) => *span,
        }
    }
}

/// 二項演算子
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOperator {
    // 算術演算子
    Add,      // +
    Subtract, // -
    Multiply, // *
    Divide,   // /
    Modulo,   // %
    
    // 比較演算子
    Equal,        // ==
    NotEqual,     // !=
    LessThan,     // <
    LessEqual,    // <=
    GreaterThan,  // >
    GreaterEqual, // >=
    
    // 論理演算子
    And, // &&
    Or,  // ||
    
    // ビット演算子
    BitAnd, // &
    BitOr,  // |
    BitXor, // ^
    ShiftLeft,  // <<
    ShiftRight, // >>
}

/// 単項演算子
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOperator {
    Negate,    // -
    Not,       // !
    BitNot,    // ~
    Reference, // &
    Dereference, // *
}

/// 式
#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    /// リテラル
    Literal(Literal),
    
    /// 識別子参照
    Identifier(Identifier),
    
    /// 二項演算
    Binary {
        left: Box<Expression>,
        operator: BinaryOperator,
        right: Box<Expression>,
        span: Span,
    },
    
    /// 単項演算
    Unary {
        operator: UnaryOperator,
        operand: Box<Expression>,
        span: Span,
    },
    
    /// 関数呼び出し
    Call {
        callee: Box<Expression>,
        arguments: Vec<Expression>,
        span: Span,
    },
    
    /// メンバーアクセス (例: obj.field)
    MemberAccess {
        object: Box<Expression>,
        member: Identifier,
        span: Span,
    },
    
    /// 配列/スライスインデックスアクセス (例: arr[idx])
    IndexAccess {
        array: Box<Expression>,
        index: Box<Expression>,
        span: Span,
    },
    
    /// if式
    If {
        condition: Box<Expression>,
        then_branch: Box<Expression>,
        else_branch: Option<Box<Expression>>,
        span: Span,
    },
    
    /// ブロック式
    Block {
        statements: Vec<Statement>,
        span: Span,
    },
    
    /// ラムダ式/クロージャ
    Lambda {
        parameters: Vec<Parameter>,
        return_type: Option<Box<TypeExpression>>,
        body: Box<Expression>,
        span: Span,
    },
    
    /// match式
    Match {
        scrutinee: Box<Expression>,
        arms: Vec<MatchArm>,
        span: Span,
    },
    
    /// タプル
    Tuple {
        elements: Vec<Expression>,
        span: Span,
    },
    
    /// 配列
    Array {
        elements: Vec<Expression>,
        span: Span,
    },
}

impl Expression {
    /// 式の位置情報を取得
    pub fn span(&self) -> Span {
        match self {
            Expression::Literal(lit) => lit.span(),
            Expression::Identifier(id) => id.span,
            Expression::Binary { span, .. } => *span,
            Expression::Unary { span, .. } => *span,
            Expression::Call { span, .. } => *span,
            Expression::MemberAccess { span, .. } => *span,
            Expression::IndexAccess { span, .. } => *span,
            Expression::If { span, .. } => *span,
            Expression::Block { span, .. } => *span,
            Expression::Lambda { span, .. } => *span,
            Expression::Match { span, .. } => *span,
            Expression::Tuple { span, .. } => *span,
            Expression::Array { span, .. } => *span,
        }
    }
}

/// match式の腕
#[derive(Debug, Clone, PartialEq)]
pub struct MatchArm {
    /// パターン
    pub pattern: Pattern,
    /// ガード条件（オプション）
    pub guard: Option<Expression>,
    /// 結果式
    pub expression: Expression,
    /// 位置情報
    pub span: Span,
}

/// パターン
#[derive(Debug, Clone, PartialEq)]
pub enum Pattern {
    /// ワイルドカードパターン (_)
    Wildcard(Span),
    
    /// リテラルパターン
    Literal(Literal),
    
    /// 識別子パターン（変数バインディング）
    Identifier(Identifier),
    
    /// 構造体パターン
    Struct {
        name: Identifier,
        fields: Vec<(Identifier, Pattern)>,
        span: Span,
    },
    
    /// タプルパターン
    Tuple {
        elements: Vec<Pattern>,
        span: Span,
    },
    
    /// 列挙型パターン
    Enum {
        path: Vec<Identifier>,
        variant: Identifier,
        payload: Option<Box<Pattern>>,
        span: Span,
    },
    
    /// 参照パターン
    Reference {
        pattern: Box<Pattern>,
        mutable: bool,
        span: Span,
    },
    
    /// 範囲パターン
    Range {
        start: Option<Box<Pattern>>,
        end: Option<Box<Pattern>>,
        inclusive: bool,
        span: Span,
    },
    
    /// OR パターン
    Or {
        patterns: Vec<Pattern>,
        span: Span,
    },
}

impl Pattern {
    /// パターンの位置情報を取得
    pub fn span(&self) -> Span {
        match self {
            Pattern::Wildcard(span) => *span,
            Pattern::Literal(lit) => lit.span(),
            Pattern::Identifier(id) => id.span,
            Pattern::Struct { span, .. } => *span,
            Pattern::Tuple { span, .. } => *span,
            Pattern::Enum { span, .. } => *span,
            Pattern::Reference { span, .. } => *span,
            Pattern::Range { span, .. } => *span,
            Pattern::Or { span, .. } => *span,
        }
    }
}

/// 文
#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    /// 式文
    Expression {
        expression: Expression,
        span: Span,
    },
    
    /// let宣言
    Let {
        pattern: Pattern,
        type_annotation: Option<TypeExpression>,
        initializer: Option<Expression>,
        span: Span,
    },
    
    /// 関数定義
    Function {
        name: Identifier,
        parameters: Vec<Parameter>,
        return_type: Option<TypeExpression>,
        body: Expression,
        span: Span,
    },
    
    /// 構造体定義
    Struct {
        name: Identifier,
        fields: Vec<StructField>,
        span: Span,
    },
    
    /// 列挙型定義
    Enum {
        name: Identifier,
        variants: Vec<EnumVariant>,
        span: Span,
    },
    
    /// トレイト定義
    Trait {
        name: Identifier,
        methods: Vec<TraitMethod>,
        span: Span,
    },
    
    /// トレイト実装
    Impl {
        trait_name: Option<Identifier>,
        for_type: TypeExpression,
        methods: Vec<ImplMethod>,
        span: Span,
    },
    
    /// モジュール定義
    Module {
        name: Identifier,
        statements: Vec<Statement>,
        span: Span,
    },
    
    /// use宣言（インポート）
    Use {
        path: Vec<Identifier>,
        alias: Option<Identifier>,
        span: Span,
    },
    
    /// 型エイリアス
    TypeAlias {
        name: Identifier,
        type_parameters: Vec<TypeParameter>,
        aliased_type: TypeExpression,
        span: Span,
    },
}

impl Statement {
    /// 文の位置情報を取得
    pub fn span(&self) -> Span {
        match self {
            Statement::Expression { span, .. } => *span,
            Statement::Let { span, .. } => *span,
            Statement::Function { span, .. } => *span,
            Statement::Struct { span, .. } => *span,
            Statement::Enum { span, .. } => *span,
            Statement::Trait { span, .. } => *span,
            Statement::Impl { span, .. } => *span,
            Statement::Module { span, .. } => *span,
            Statement::Use { span, .. } => *span,
            Statement::TypeAlias { span, .. } => *span,
        }
    }
}

/// 関数/メソッドのパラメータ
#[derive(Debug, Clone, PartialEq)]
pub struct Parameter {
    /// パラメータ名
    pub name: Identifier,
    /// 型注釈
    pub type_annotation: TypeExpression,
    /// デフォルト値（オプション）
    pub default_value: Option<Expression>,
    /// 位置情報
    pub span: Span,
}

/// 構造体のフィールド
#[derive(Debug, Clone, PartialEq)]
pub struct StructField {
    /// フィールド名
    pub name: Identifier,
    /// 型注釈
    pub type_annotation: TypeExpression,
    /// 可視性
    pub visibility: Visibility,
    /// 位置情報
    pub span: Span,
}

/// 列挙型のバリアント
#[derive(Debug, Clone, PartialEq)]
pub enum EnumVariant {
    /// 単純なバリアント（ペイロードなし）
    Simple {
        name: Identifier,
        span: Span,
    },
    /// タプル型バリアント
    Tuple {
        name: Identifier,
        types: Vec<TypeExpression>,
        span: Span,
    },
    /// 構造体型バリアント
    Struct {
        name: Identifier,
        fields: Vec<StructField>,
        span: Span,
    },
}

impl EnumVariant {
    /// バリアントの位置情報を取得
    pub fn span(&self) -> Span {
        match self {
            EnumVariant::Simple { span, .. } => *span,
            EnumVariant::Tuple { span, .. } => *span,
            EnumVariant::Struct { span, .. } => *span,
        }
    }
    
    /// バリアントの名前を取得
    pub fn name(&self) -> &Identifier {
        match self {
            EnumVariant::Simple { name, .. } => name,
            EnumVariant::Tuple { name, .. } => name,
            EnumVariant::Struct { name, .. } => name,
        }
    }
}

/// トレイトメソッド宣言
#[derive(Debug, Clone, PartialEq)]
pub struct TraitMethod {
    /// メソッド名
    pub name: Identifier,
    /// パラメータ
    pub parameters: Vec<Parameter>,
    /// 戻り値の型
    pub return_type: Option<TypeExpression>,
    /// デフォルト実装（オプション）
    pub default_implementation: Option<Expression>,
    /// 位置情報
    pub span: Span,
}

/// 実装メソッド
#[derive(Debug, Clone, PartialEq)]
pub struct ImplMethod {
    /// メソッド名
    pub name: Identifier,
    /// パラメータ
    pub parameters: Vec<Parameter>,
    /// 戻り値の型
    pub return_type: Option<TypeExpression>,
    /// メソッド本体
    pub body: Expression,
    /// 可視性
    pub visibility: Visibility,
    /// 位置情報
    pub span: Span,
}

/// 可視性
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Visibility {
    /// 公開（public）
    Public,
    /// 非公開（private）
    Private,
    /// モジュール内公開（protected）
    Protected,
}

/// 型式
#[derive(Debug, Clone, PartialEq)]
pub enum TypeExpression {
    /// 名前付き型（プリミティブ型や定義済み型）
    Named {
        path: Vec<Identifier>,
        name: Identifier,
        type_arguments: Vec<TypeExpression>,
        span: Span,
    },
    
    /// タプル型
    Tuple {
        elements: Vec<TypeExpression>,
        span: Span,
    },
    
    /// 関数型
    Function {
        parameters: Vec<TypeExpression>,
        return_type: Box<TypeExpression>,
        span: Span,
    },
    
    /// 配列型
    Array {
        element_type: Box<TypeExpression>,
        size: Option<Expression>,
        span: Span,
    },
    
    /// 参照型
    Reference {
        referenced_type: Box<TypeExpression>,
        mutable: bool,
        span: Span,
    },
    
    /// 型パラメータ
    TypeParameter {
        name: Identifier,
        span: Span,
    },
}

impl TypeExpression {
    /// 型式の位置情報を取得
    pub fn span(&self) -> Span {
        match self {
            TypeExpression::Named { span, .. } => *span,
            TypeExpression::Tuple { span, .. } => *span,
            TypeExpression::Function { span, .. } => *span,
            TypeExpression::Array { span, .. } => *span,
            TypeExpression::Reference { span, .. } => *span,
            TypeExpression::TypeParameter { span, .. } => *span,
        }
    }
}

/// 型パラメータ
#[derive(Debug, Clone, PartialEq)]
pub struct TypeParameter {
    /// パラメータ名
    pub name: Identifier,
    /// 境界制約
    pub bounds: Vec<TypeExpression>,
    /// 位置情報
    pub span: Span,
}

/// プログラム全体を表すルートノード
#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    /// プログラムを構成する文のリスト
    pub statements: Vec<Statement>,
    /// ソースファイル名
    pub source_file: String,
}

impl Program {
    /// 新しいプログラムを作成
    pub fn new(statements: Vec<Statement>, source_file: String) -> Self {
        Self { statements, source_file }
    }
}

/// # パーサー用AST補助機能
/// 
/// 構文解析器が抽象構文木（AST）を構築する際に使用する補助機能を提供します。

use crate::frontend::ast::*;
use crate::frontend::error::SourceLocation;
use crate::frontend::lexer::token::Token;

/// ASTノード構築を支援するファクトリー
pub struct AstFactory {
    /// 次に割り当てるノードID
    next_id: NodeId,
}

impl AstFactory {
    /// 新しいASTファクトリーを作成
    pub fn new() -> Self {
        Self { next_id: 1 }
    }
    
    /// 次のノードIDを取得
    pub fn next_id(&mut self) -> NodeId {
        let id = self.next_id;
        self.next_id += 1;
        id
    }
    
    /// 識別子を作成
    pub fn create_identifier(&mut self, name: &str, token: &Token) -> Identifier {
        Identifier::new(name.to_string(), Some(token.location.clone()))
    }
    
    /// トークンから識別子を作成
    pub fn identifier_from_token(&mut self, token: &Token) -> Identifier {
        debug_assert!(token.is_identifier());
        Identifier::new(token.lexeme.clone(), Some(token.location.clone()))
    }
    
    /// 整数リテラルを作成
    pub fn create_integer_literal(&mut self, value: i64, token: &Token) -> Literal {
        Literal::new(
            LiteralKind::Integer(value),
            self.next_id(),
            Some(token.location.clone())
        )
    }
    
    /// 浮動小数点リテラルを作成
    pub fn create_float_literal(&mut self, value: f64, token: &Token) -> Literal {
        Literal::new(
            LiteralKind::Float(value),
            self.next_id(),
            Some(token.location.clone())
        )
    }
    
    /// 文字列リテラルを作成
    pub fn create_string_literal(&mut self, value: &str, token: &Token) -> Literal {
        Literal::new(
            LiteralKind::String(value.to_string()),
            self.next_id(),
            Some(token.location.clone())
        )
    }
    
    /// 文字リテラルを作成
    pub fn create_char_literal(&mut self, value: char, token: &Token) -> Literal {
        Literal::new(
            LiteralKind::Char(value),
            self.next_id(),
            Some(token.location.clone())
        )
    }
    
    /// 真偽値リテラルを作成
    pub fn create_boolean_literal(&mut self, value: bool, token: &Token) -> Literal {
        Literal::new(
            LiteralKind::Boolean(value),
            self.next_id(),
            Some(token.location.clone())
        )
    }
    
    /// nilリテラルを作成
    pub fn create_nil_literal(&mut self, token: &Token) -> Literal {
        Literal::new(
            LiteralKind::Nil,
            self.next_id(),
            Some(token.location.clone())
        )
    }
    
    /// リテラル式を作成
    pub fn create_literal_expr(&mut self, literal: Literal, location: Option<SourceLocation>) -> Expression {
        Expression::new(
            ExpressionKind::Literal(literal),
            self.next_id(),
            location
        )
    }
    
    /// 識別子式を作成
    pub fn create_identifier_expr(&mut self, identifier: Identifier, location: Option<SourceLocation>) -> Expression {
        Expression::new(
            ExpressionKind::Identifier(identifier),
            self.next_id(),
            location
        )
    }
    
    /// 二項演算式を作成
    pub fn create_binary_op(&mut self, op: BinaryOperator, left: Expression, right: Expression,
                       location: Option<SourceLocation>) -> Expression {
        // 位置情報が提供されていない場合は左辺と右辺の位置情報を結合
        let loc = location.or_else(|| {
            if let (Some(left_loc), Some(right_loc)) = (left.location(), right.location()) {
                Some(left_loc.clone().merge(right_loc))
            } else {
                None
            }
        });
        
        Expression::new(
            ExpressionKind::BinaryOp(op, Box::new(left), Box::new(right)),
            self.next_id(),
            loc
        )
    }
    
    /// 単項演算式を作成
    pub fn create_unary_op(&mut self, op: UnaryOperator, expr: Expression,
                      location: Option<SourceLocation>) -> Expression {
        let loc = location.or_else(|| expr.location().cloned());
        
        Expression::new(
            ExpressionKind::UnaryOp(op, Box::new(expr)),
            self.next_id(),
            loc
        )
    }
    
    /// 関数呼び出し式を作成
    pub fn create_call(&mut self, callee: Expression, arguments: Vec<Expression>,
                  location: Option<SourceLocation>) -> Expression {
        let loc = location.or_else(|| {
            if let Some(callee_loc) = callee.location() {
                if let Some(last_arg_loc) = arguments.last().and_then(|arg| arg.location()) {
                    Some(callee_loc.clone().merge(last_arg_loc))
                } else {
                    Some(callee_loc.clone())
                }
            } else {
                None
            }
        });
        
        Expression::new(
            ExpressionKind::Call(Box::new(callee), arguments),
            self.next_id(),
            loc
        )
    }
    
    /// メンバーアクセス式を作成
    pub fn create_member_access(&mut self, object: Expression, member: Identifier,
                          location: Option<SourceLocation>) -> Expression {
        let loc = location.or_else(|| {
            if let (Some(obj_loc), Some(member_loc)) = (object.location(), member.location()) {
                Some(obj_loc.clone().merge(member_loc))
            } else {
                None
            }
        });
        
        Expression::new(
            ExpressionKind::MemberAccess(Box::new(object), member),
            self.next_id(),
            loc
        )
    }
    
    /// インデックスアクセス式を作成
    pub fn create_index_access(&mut self, array: Expression, index: Expression,
                         location: Option<SourceLocation>) -> Expression {
        let loc = location.or_else(|| {
            if let (Some(array_loc), Some(index_loc)) = (array.location(), index.location()) {
                Some(array_loc.clone().merge(index_loc))
            } else {
                None
            }
        });
        
        Expression::new(
            ExpressionKind::IndexAccess(Box::new(array), Box::new(index)),
            self.next_id(),
            loc
        )
    }
    
    /// 変数宣言を作成
    pub fn create_variable_declaration(&mut self, name: Identifier, type_annotation: Option<TypeAnnotation>,
                                  is_mutable: bool, initializer: Option<Expression>,
                                  visibility: Visibility, location: Option<SourceLocation>) -> VariableDeclaration {
        VariableDeclaration {
            name,
            type_annotation,
            is_mutable,
            initializer,
            visibility,
            id: self.next_id(),
            location,
        }
    }
    
    /// 式文を作成
    pub fn create_expression_statement(&mut self, expr: Expression, location: Option<SourceLocation>) -> Statement {
        let loc = location.or_else(|| expr.location().cloned());
        
        Statement::new(
            StatementKind::ExpressionStmtStatement(expr),
            self.next_id(),
            loc
        )
    }
    
    /// ブロック文を作成
    pub fn create_block(&mut self, statements: Vec<Statement>, location: Option<SourceLocation>) -> Statement {
        Statement::new(
            StatementKind::Block(statements),
            self.next_id(),
            location
        )
    }
    
    /// if文を作成
    pub fn create_if_statement(&mut self, condition: Expression, then_branch: Statement,
                         else_branch: Option<Statement>, location: Option<SourceLocation>) -> Statement {
        // 位置情報の結合
        let loc = location.or_else(|| {
            if let Some(cond_loc) = condition.location() {
                if let Some(else_loc) = else_branch.as_ref().and_then(|s| s.location()) {
                    Some(cond_loc.clone().merge(else_loc))
                } else if let Some(then_loc) = then_branch.location() {
                    Some(cond_loc.clone().merge(then_loc))
                } else {
                    Some(cond_loc.clone())
                }
            } else {
                None
            }
        });
        
        Statement::new(
            StatementKind::IfStmtStatement {
                condition,
                then_branch: Box::new(then_branch),
                else_branch: else_branch.map(Box::new),
            },
            self.next_id(),
            loc
        )
    }
    
    /// while文を作成
    pub fn create_while_statement(&mut self, condition: Expression, body: Statement,
                           location: Option<SourceLocation>) -> Statement {
        let loc = location.or_else(|| {
            if let (Some(cond_loc), Some(body_loc)) = (condition.location(), body.location()) {
                Some(cond_loc.clone().merge(body_loc))
            } else {
                condition.location().cloned()
            }
        });
        
        Statement::new(
            StatementKind::WhileStmtStatement {
                condition,
                body: Box::new(body),
            },
            self.next_id(),
            loc
        )
    }
    
    /// for文を作成
    pub fn create_for_statement(&mut self, variable: Identifier, iterable: Expression, body: Statement,
                         location: Option<SourceLocation>) -> Statement {
        let loc = location.or_else(|| {
            if let (Some(var_loc), Some(body_loc)) = (variable.location(), body.location()) {
                Some(var_loc.clone().merge(body_loc))
            } else {
                None
            }
        });
        
        Statement::new(
            StatementKind::ForStmtStatement {
                variable,
                iterable,
                body: Box::new(body),
            },
            self.next_id(),
            loc
        )
    }
    
    /// return文を作成
    pub fn create_return_statement(&mut self, expr: Option<Expression>, location: Option<SourceLocation>) -> Statement {
        let loc = location.or_else(|| expr.as_ref().and_then(|e| e.location().cloned()));
        
        Statement::new(
            StatementKind::ReturnStmtStatement(expr),
            self.next_id(),
            loc
        )
    }
    
    /// プログラム（ASTのルート）を作成
    pub fn create_program(&mut self, source_path: &str, declarations: Vec<Declaration>) -> Program {
        Program::new(
            declarations,
            source_path.to_string()
        )
    }
}

impl Default for AstFactory {
    fn default() -> Self {
        Self::new()
    }
}

/// 二つのソース位置の間を結合
pub fn merge_locations(start: &SourceLocation, end: &SourceLocation) -> SourceLocation {
    start.clone().merge(end)
}

/// トークン列から位置情報を抽出
pub fn location_from_tokens(start_token: &Token, end_token: &Token) -> SourceLocation {
    start_token.location.clone().merge(&end_token.location)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_ast_factory_node_ids() {
        let mut factory = AstFactory::new();
        
        // 各呼び出しで異なるIDが生成されることを確認
        let id1 = factory.next_id();
        let id2 = factory.next_id();
        let id3 = factory.next_id();
        
        assert_ne!(id1, id2);
        assert_ne!(id2, id3);
        assert_ne!(id1, id3);
    }
    
    #[test]
    fn test_create_identifier() {
        let mut factory = AstFactory::new();
        let location = SourceLocation::new("test.swl", 1, 1, 3);
        let token = Token::new(
            crate::frontend::lexer::token::TokenKind::Identifier("foo".to_string()),
            location.clone()
        );
        
        let ident = factory.create_identifier("foo", &token);
        
        assert_eq!(ident.name, "foo");
        assert!(ident.id > 0);
        assert_eq!(ident.location.unwrap().file_name, "test.swl");
    }
    
    #[test]
    fn test_create_expression() {
        let mut factory = AstFactory::new();
        let location = SourceLocation::new("test.swl", 1, 1, 3); // 引数を4つに修正
        let token = Token::new(
            crate::frontend::lexer::token::TokenKind::IntLiteral(123),
            location.clone()
        );
        
        let literal = factory.create_integer_literal(123, &token);
        let expr = factory.create_literal_expr(literal, Some(token.location.clone()));
        assert!(expr.id > 0);
        assert_eq!(expr.location.unwrap().file_name, "test.swl");
        
        if let ExpressionKind::Literal(lit) = &expr.kind {
            if let LiteralKind::Integer(val) = lit.kind {
                assert_eq!(val, 123);
            } else {
                panic!("Expected integer literal");
            }
        } else {
            panic!("Expected literal expression");
        }
    }
}
