// SwiftLight IR表現モジュール
//
// このモジュールはコンパイラの中間表現(IR)に関する構造体と列挙型を定義します。
// LLVM IRへの変換前の抽象表現として機能します。
// 高度な最適化、型検証、並行処理モデルをサポートするための豊富な表現力を持ちます。

use std::collections::{HashMap, HashSet, BTreeMap, BTreeSet};
use std::fmt;
use std::sync::Arc;
use std::rc::Rc;
use std::cell::{Cell, RefCell};
use std::path::PathBuf;

/// IR型システム
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Type {
    /// void型
    Void,
    /// 整数型（ビット幅指定）
    Integer(usize),
    /// 浮動小数点型
    Float,
    /// 倍精度浮動小数点型
    Double,
    /// 論理型
    Boolean,
    /// 文字型
    Char,
    /// 文字列型（内部的にはポインタ）
    String,
    /// ポインタ型
    Pointer(Box<Type>),
    /// 参照型（所有権システム用）
    Reference(Box<Type>, bool), // 第2引数はmutableかどうか
    /// 配列型
    Array(Box<Type>, usize),
    /// 可変長配列型
    Vector(Box<Type>),
    /// スライス型
    Slice(Box<Type>),
    /// 構造体型
    Struct(String, Vec<Type>),
    /// 関数型
    Function(Vec<Type>, Box<Type>),
    /// ユニオン型
    Union(Vec<Type>),
    /// インターセクション型
    Intersection(Vec<Type>),
    /// ジェネリック型
    Generic(String, Vec<Type>),
    /// メタ型（型の型）
    Meta(Box<Type>),
    /// オプショナル型
    Optional(Box<Type>),
    /// 結果型（成功または失敗）
    Result(Box<Type>, Box<Type>),
    /// タプル型
    Tuple(Vec<Type>),
    /// トレイト型
    Trait(String),
    /// 存在型（トレイト境界付き）
    Existential(String, Vec<String>),
    /// 依存型（値に依存する型）
    Dependent(String, Box<Expression>),
    /// 型変数（型推論用）
    TypeVar(String),
    /// 制約付き型（型制約を持つ型）
    Constrained(Box<Type>, Vec<TypeConstraint>),
    /// 再帰型
    Recursive(String, Box<Type>),
    /// 未知の型
    Unknown,
    /// エラー型（型チェック失敗時）
    Error,
}

/// 型制約
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TypeConstraint {
    /// トレイト境界
    TraitBound(String),
    /// 型等価性制約
    Equals(Box<Type>),
    /// サブタイプ制約
    SubType(Box<Type>),
    /// スーパータイプ制約
    SuperType(Box<Type>),
    /// 値制約（依存型用）
    ValueConstraint(Box<Expression>),
    /// 構造的制約（フィールドやメソッドの存在）
    Structural(Vec<String>),
}

impl Type {
    /// 型のサイズを計算（バイト単位）
    pub fn size(&self) -> usize {
        match self {
            Type::Void => 0,
            Type::Integer(bits) => (bits + 7) / 8, // 切り上げ
            Type::Float => 4,
            Type::Double => 8,
            Type::Boolean => 1,
            Type::Char => 4, // Unicode対応
            Type::String => 16, // ポインタ + 長さ + キャパシティ
            Type::Pointer(_) => 8, // 64ビットアーキテクチャを仮定
            Type::Reference(_, _) => 8, // 参照も64ビット
            Type::Array(elem_type, count) => elem_type.size() * count,
            Type::Vector(elem_type) => 24, // ポインタ + 長さ + キャパシティ
            Type::Slice(elem_type) => 16, // ポインタ + 長さ
            Type::Struct(name, fields) => {
                // アラインメントを考慮した計算
                let mut total_size = 0;
                let mut max_align = 1;
                
                for field in fields {
                    let field_size = field.size();
                    let field_align = field.alignment();
                    
                    // アラインメント境界に合わせる
                    total_size = (total_size + field_align - 1) / field_align * field_align;
                    total_size += field_size;
                    max_align = max_align.max(field_align);
                }
                
                // 構造体全体のアラインメントに合わせる
                (total_size + max_align - 1) / max_align * max_align
            }
            Type::Function(_, _) => 16, // 関数ポインタ + 環境ポインタ
            Type::Union(types) => {
                // ユニオンは最大のメンバーサイズ + タグ
                let max_size = types.iter().map(|t| t.size()).max().unwrap_or(0);
                let max_align = types.iter().map(|t| t.alignment()).max().unwrap_or(1);
                let tag_size = 4; // 識別子用の整数
                
                // アラインメントを考慮
                let total_size = max_size + tag_size;
                (total_size + max_align - 1) / max_align * max_align
            }
            Type::Intersection(_) => 16, // インターフェースとして扱う（vtable + データポインタ）
            Type::Generic(_, _) => 8,   // 具体化されていないのでポインタサイズ
            Type::Meta(_) => 8,         // 型情報のポインタ
            Type::Optional(inner) => {
                // None値の表現のために追加の1バイトが必要
                let inner_size = inner.size();
                let inner_align = inner.alignment();
                
                // アラインメントを考慮
                let total_size = inner_size + 1;
                (total_size + inner_align - 1) / inner_align * inner_align
            }
            Type::Result(ok_type, err_type) => {
                // 結果型は成功型と失敗型の和集合 + タグ
                let ok_size = ok_type.size();
                let err_size = err_type.size();
                let max_size = ok_size.max(err_size);
                let max_align = ok_type.alignment().max(err_type.alignment());
                let tag_size = 1; // 成功/失敗フラグ
                
                // アラインメントを考慮
                let total_size = max_size + tag_size;
                (total_size + max_align - 1) / max_align * max_align
            }
            Type::Tuple(types) => {
                // タプルはフィールドの合計 + アラインメント
                let mut total_size = 0;
                let mut max_align = 1;
                
                for typ in types {
                    let field_size = typ.size();
                    let field_align = typ.alignment();
                    
                    // アラインメント境界に合わせる
                    total_size = (total_size + field_align - 1) / field_align * field_align;
                    total_size += field_size;
                    max_align = max_align.max(field_align);
                }
                
                // 構造体全体のアラインメントに合わせる
                (total_size + max_align - 1) / max_align * max_align
            }
            Type::Trait(_) => 16, // トレイトオブジェクト（vtable + データポインタ）
            Type::Existential(_, _) => 16, // 存在型もトレイトオブジェクトと同様
            Type::Dependent(_, _) => 8, // 依存型は実行時には通常の値
            Type::TypeVar(_) => 0, // 型変数はコンパイル時のみ
            Type::Constrained(inner, _) => inner.size(), // 制約は実行時サイズに影響しない
            Type::Recursive(_, inner) => inner.size(), // 再帰型は内部型と同じサイズ
            Type::Unknown => 0,
            Type::Error => 0,
        }
    }
    
    /// 型のアラインメントを計算
    pub fn alignment(&self) -> usize {
        match self {
            Type::Void => 1,
            Type::Integer(bits) => {
                let bytes = (bits + 7) / 8;
                if bytes > 8 { 8 } else { bytes }
            },
            Type::Float => 4,
            Type::Double => 8,
            Type::Boolean => 1,
            Type::Char => 4,
            Type::String => 8,
            Type::Pointer(_) => 8,
            Type::Reference(_, _) => 8,
            Type::Array(elem_type, _) => elem_type.alignment(),
            Type::Vector(elem_type) => 8.max(elem_type.alignment()),
            Type::Slice(elem_type) => 8,
            Type::Struct(_, fields) => {
                // 構造体のアラインメントは最大のフィールドアラインメント
                fields.iter().map(|f| f.alignment()).max().unwrap_or(1)
            },
            Type::Function(_, _) => 8,
            Type::Union(types) => {
                // ユニオンのアラインメントは最大のメンバーアラインメント
                types.iter().map(|t| t.alignment()).max().unwrap_or(1)
            },
            Type::Intersection(_) => 8,
            Type::Generic(_, _) => 8,
            Type::Meta(_) => 8,
            Type::Optional(inner) => inner.alignment(),
            Type::Result(ok_type, err_type) => ok_type.alignment().max(err_type.alignment()),
            Type::Tuple(types) => {
                // タプルのアラインメントは最大のフィールドアラインメント
                types.iter().map(|t| t.alignment()).max().unwrap_or(1)
            },
            Type::Trait(_) => 8,
            Type::Existential(_, _) => 8,
            Type::Dependent(_, _) => 8,
            Type::TypeVar(_) => 1,
            Type::Constrained(inner, _) => inner.alignment(),
            Type::Recursive(_, inner) => inner.alignment(),
            Type::Unknown => 1,
            Type::Error => 1,
        }
    }
    
    /// 型が値型かどうか
    pub fn is_value_type(&self) -> bool {
        match self {
            Type::Void | Type::Integer(_) | Type::Float | Type::Double |
            Type::Boolean | Type::Char => true,
            Type::Pointer(_) | Type::String | Type::Function(_, _) => false,
            Type::Reference(_, _) => false,
            Type::Array(_, _) | Type::Vector(_) | Type::Slice(_) => false, // SwiftLightでは参照型
            Type::Struct(_, _) => false, // デフォルトでは参照型として扱う
            Type::Union(_) | Type::Intersection(_) => false,
            Type::Generic(_, _) | Type::Meta(_) => false,
            Type::Optional(inner) => inner.is_value_type(),
            Type::Result(ok, err) => ok.is_value_type() && err.is_value_type(),
            Type::Tuple(types) => types.iter().all(|t| t.is_value_type()),
            Type::Trait(_) | Type::Existential(_, _) => false,
            Type::Dependent(_, _) => false, // 依存型は通常参照型
            Type::TypeVar(_) => false, // 型変数は未確定なので参照型と仮定
            Type::Constrained(inner, _) => inner.is_value_type(),
            Type::Recursive(_, _) => false, // 再帰型は通常参照型
            Type::Unknown | Type::Error => false,
        }
    }
    
    /// 型がnull許容かどうか
    pub fn is_nullable(&self) -> bool {
        matches!(self, Type::Optional(_) | Type::Pointer(_) | Type::Unknown)
    }
    
    /// 型が数値型かどうか
    pub fn is_numeric(&self) -> bool {
        matches!(self, Type::Integer(_) | Type::Float | Type::Double)
    }
    
    /// 型が整数型かどうか
    pub fn is_integer(&self) -> bool {
        matches!(self, Type::Integer(_))
    }
    
    /// 型が浮動小数点型かどうか
    pub fn is_floating_point(&self) -> bool {
        matches!(self, Type::Float | Type::Double)
    }
    
    /// 型が参照型かどうか
    pub fn is_reference_type(&self) -> bool {
        matches!(self, Type::Reference(_, _))
    }
    
    /// 型が可変参照かどうか
    pub fn is_mutable_reference(&self) -> bool {
        if let Type::Reference(_, is_mut) = self {
            *is_mut
        } else {
            false
        }
    }
    
    /// 型がトレイト型かどうか
    pub fn is_trait(&self) -> bool {
        matches!(self, Type::Trait(_) | Type::Existential(_, _))
    }
    
    /// 型が依存型かどうか
    pub fn is_dependent(&self) -> bool {
        matches!(self, Type::Dependent(_, _))
    }
    
    /// 型が多相型かどうか
    pub fn is_polymorphic(&self) -> bool {
        matches!(self, Type::Generic(_, _) | Type::TypeVar(_))
    }
    
    /// 型が再帰型かどうか
    pub fn is_recursive(&self) -> bool {
        matches!(self, Type::Recursive(_, _))
    }
    
    /// 型が具体型かどうか（型変数やジェネリックパラメータを含まない）
    pub fn is_concrete(&self) -> bool {
        match self {
            Type::TypeVar(_) | Type::Generic(_, _) => false,
            Type::Pointer(inner) | Type::Reference(inner, _) | 
            Type::Array(inner, _) | Type::Vector(inner) | Type::Slice(inner) |
            Type::Optional(inner) | Type::Meta(inner) => inner.is_concrete(),
            Type::Result(ok, err) => ok.is_concrete() && err.is_concrete(),
            Type::Struct(_, fields) | Type::Tuple(fields) => fields.iter().all(|f| f.is_concrete()),
            Type::Union(types) | Type::Intersection(types) => types.iter().all(|t| t.is_concrete()),
            Type::Function(params, ret) => {
                params.iter().all(|p| p.is_concrete()) && ret.is_concrete()
            },
            Type::Constrained(inner, _) => inner.is_concrete(),
            Type::Recursive(_, inner) => inner.is_concrete(),
            Type::Dependent(_, _) => false, // 依存型は値に依存するため具体型ではない
            _ => true,
        }
    }
    
    /// 型の自由型変数を収集
    pub fn free_type_vars(&self) -> HashSet<String> {
        let mut vars = HashSet::new();
        self.collect_type_vars(&mut vars);
        vars
    }
    
    /// 型変数を収集する補助メソッド
    fn collect_type_vars(&self, vars: &mut HashSet<String>) {
        match self {
            Type::TypeVar(name) => {
                vars.insert(name.clone());
            },
            Type::Pointer(inner) | Type::Reference(inner, _) | 
            Type::Array(inner, _) | Type::Vector(inner) | Type::Slice(inner) |
            Type::Optional(inner) | Type::Meta(inner) => {
                inner.collect_type_vars(vars);
            },
            Type::Result(ok, err) => {
                ok.collect_type_vars(vars);
                err.collect_type_vars(vars);
            },
            Type::Struct(_, fields) | Type::Tuple(fields) => {
                for field in fields {
                    field.collect_type_vars(vars);
                }
            },
            Type::Union(types) | Type::Intersection(types) => {
                for ty in types {
                    ty.collect_type_vars(vars);
                }
            },
            Type::Function(params, ret) => {
                for param in params {
                    param.collect_type_vars(vars);
                }
                ret.collect_type_vars(vars);
            },
            Type::Generic(_, args) => {
                for arg in args {
                    arg.collect_type_vars(vars);
                }
            },
            Type::Constrained(inner, constraints) => {
                inner.collect_type_vars(vars);
                for constraint in constraints {
                    match constraint {
                        TypeConstraint::Equals(ty) |
                        TypeConstraint::SubType(ty) |
                        TypeConstraint::SuperType(ty) => {
                            ty.collect_type_vars(vars);
                        },
                        _ => {},
                    }
                }
            },
            Type::Recursive(_, inner) => {
                inner.collect_type_vars(vars);
            },
            Type::Dependent(_, expr) => {
                // 依存型の式に含まれる型変数も収集
                expr.collect_type_vars(vars);
            },
            _ => {},
        }
    }
    
    /// 型の代入（型変数を具体型で置き換え）
    pub fn substitute(&self, substitutions: &HashMap<String, Type>) -> Type {
        match self {
            Type::TypeVar(name) => {
                if let Some(ty) = substitutions.get(name) {
                    ty.clone()
                } else {
                    self.clone()
                }
            },
            Type::Pointer(inner) => {
                Type::Pointer(Box::new(inner.substitute(substitutions)))
            },
            Type::Reference(inner, is_mut) => {
                Type::Reference(Box::new(inner.substitute(substitutions)), *is_mut)
            },
            Type::Array(inner, size) => {
                Type::Array(Box::new(inner.substitute(substitutions)), *size)
            },
            Type::Vector(inner) => {
                Type::Vector(Box::new(inner.substitute(substitutions)))
            },
            Type::Slice(inner) => {
                Type::Slice(Box::new(inner.substitute(substitutions)))
            },
            Type::Optional(inner) => {
                Type::Optional(Box::new(inner.substitute(substitutions)))
            },
            Type::Meta(inner) => {
                Type::Meta(Box::new(inner.substitute(substitutions)))
            },
            Type::Result(ok, err) => {
                Type::Result(
                    Box::new(ok.substitute(substitutions)),
                    Box::new(err.substitute(substitutions))
                )
            },
            Type::Struct(name, fields) => {
                Type::Struct(
                    name.clone(),
                    fields.iter().map(|f| f.substitute(substitutions)).collect()
                )
            },
            Type::Tuple(fields) => {
                Type::Tuple(
                    fields.iter().map(|f| f.substitute(substitutions)).collect()
                )
            },
            Type::Union(types) => {
                Type::Union(
                    types.iter().map(|t| t.substitute(substitutions)).collect()
                )
            },
            Type::Intersection(types) => {
                Type::Intersection(
                    types.iter().map(|t| t.substitute(substitutions)).collect()
                )
            },
            Type::Function(params, ret) => {
                Type::Function(
                    params.iter().map(|p| p.substitute(substitutions)).collect(),
                    Box::new(ret.substitute(substitutions))
                )
            },
            Type::Generic(name, args) => {
                Type::Generic(
                    name.clone(),
                    args.iter().map(|a| a.substitute(substitutions)).collect()
                )
            },
            Type::Constrained(inner, constraints) => {
                let new_constraints = constraints.iter().map(|c| match c {
                    TypeConstraint::Equals(ty) => {
                        TypeConstraint::Equals(Box::new(ty.substitute(substitutions)))
                    },
                    TypeConstraint::SubType(ty) => {
                        TypeConstraint::SubType(Box::new(ty.substitute(substitutions)))
                    },
                    TypeConstraint::SuperType(ty) => {
                        TypeConstraint::SuperType(Box::new(ty.substitute(substitutions)))
                    },
                    _ => c.clone(),
                }).collect();
                
                Type::Constrained(
                    Box::new(inner.substitute(substitutions)),
                    new_constraints
                )
            },
            Type::Recursive(name, inner) => {
                // 再帰型の場合、自己参照を避けるために名前をスキップ
                let mut local_subst = substitutions.clone();
                local_subst.remove(name);
                
                Type::Recursive(
                    name.clone(),
                    Box::new(inner.substitute(&local_subst))
                )
            },
            Type::Dependent(name, expr) => {
                // 依存型の式も代入
                Type::Dependent(
                    name.clone(),
                    Box::new(expr.substitute_types(substitutions))
                )
            },
            _ => self.clone(),
        }
    }
    
    /// 型の単一化（2つの型を一致させる代入を見つける）
    pub fn unify(&self, other: &Type) -> Result<HashMap<String, Type>, String> {
        let mut substitutions = HashMap::new();
        self.unify_with(other, &mut substitutions)?;
        Ok(substitutions)
    }
    
    /// 単一化の補助メソッド
    fn unify_with(&self, other: &Type, substitutions: &mut HashMap<String, Type>) -> Result<(), String> {
        match (self, other) {
            // 同じ型は単一化可能
            (a, b) if a == b => Ok(()),
            
            // 型変数の単一化
            (Type::TypeVar(name), other) => {
                if let Some(existing) = substitutions.get(name) {
                    // すでに代入がある場合は再帰的に単一化
                    existing.unify_with(other, substitutions)
                } else if other.free_type_vars().contains(name) {
                    // 出現チェック（無限型を防ぐ）
                    Err(format!("Recursive type detected: {} occurs in {}", name, other))
                } else {
                    substitutions.insert(name.clone(), other.clone());
                    Ok(())
                }
            },
            (other, Type::TypeVar(name)) => {
                // 対称性のため反転
                if let Some(existing) = substitutions.get(name) {
                    other.unify_with(existing, substitutions)
                } else if other.free_type_vars().contains(name) {
                    Err(format!("Recursive type detected: {} occurs in {}", name, other))
                } else {
                    substitutions.insert(name.clone(), other.clone());
                    Ok(())
                }
            },
            
            // 複合型の単一化
            (Type::Pointer(a), Type::Pointer(b)) => {
                a.unify_with(b, substitutions)
            },
            (Type::Reference(a, a_mut), Type::Reference(b, b_mut)) => {
                if a_mut == b_mut {
                    a.unify_with(b, substitutions)
                } else {
                    Err(format!("Cannot unify mutable and immutable references"))
                }
            },
            (Type::Array(a_elem, a_size), Type::Array(b_elem, b_size)) => {
                if a_size == b_size {
                    a_elem.unify_with(b_elem, substitutions)
                } else {
                    Err(format!("Cannot unify arrays of different sizes"))
                }
            },
            (Type::Vector(a), Type::Vector(b)) => {
                a.unify_with(b, substitutions)
            },
            (Type::Slice(a), Type::Slice(b)) => {
                a.unify_with(b, substitutions)
            },
            (Type::Optional(a), Type::Optional(b)) => {
                a.unify_with(b, substitutions)
            },
            (Type::Meta(a), Type::Meta(b)) => {
                a.unify_with(b, substitutions)
            },
            (Type::Result(a_ok, a_err), Type::Result(b_ok, b_err)) => {
                a_ok.unify_with(b_ok, substitutions)?;
                a_err.unify_with(b_err, substitutions)
            },
            (Type::Struct(a_name, a_fields), Type::Struct(b_name, b_fields)) => {
                if a_name != b_name || a_fields.len() != b_fields.len() {
                    return Err(format!("Cannot unify different struct types"));
                }
                
                for (a_field, b_field) in a_fields.iter().zip(b_fields.iter()) {
                    a_field.unify_with(b_field, substitutions)?;
                }
                
                Ok(())
            },
            (Type::Tuple(a_fields), Type::Tuple(b_fields)) => {
                if a_fields.len() != b_fields.len() {
                    return Err(format!("Cannot unify tuples of different lengths"));
                }
                
                for (a_field, b_field) in a_fields.iter().zip(b_fields.iter()) {
                    a_field.unify_with(b_field, substitutions)?;
                }
                
                Ok(())
            },
            (Type::Function(a_params, a_ret), Type::Function(b_params, b_ret)) => {
                if a_params.len() != b_params.len() {
                    return Err(format!("Cannot unify functions with different parameter counts"));
                }
                
                for (a_param, b_param) in a_params.iter().zip(b_params.iter()) {
                    a_param.unify_with(b_param, substitutions)?;
                }
                
                a_ret.unify_with(b_ret, substitutions)
            },
            (Type::Generic(a_name, a_args), Type::Generic(b_name, b_args)) => {
                if a_name != b_name || a_args.len() != b_args.len() {
                    return Err(format!("Cannot unify different generic types"));
                }
                
                for (a_arg, b_arg) in a_args.iter().zip(b_args.iter()) {
                    a_arg.unify_with(b_arg, substitutions)?;
                }
                
                Ok(())
            },
            
            // その他の型の組み合わせは単一化不可能
            _ => Err(format!("Cannot unify types: {:?} and {:?}", self, other)),
        }
    }
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Type::Void => write!(f, "void"),
            Type::Integer(bits) => write!(f, "i{}", bits),
            Type::Float => write!(f, "float"),
            Type::Double => write!(f, "double"),
            Type::Boolean => write!(f, "bool"),
            Type::Char => write!(f, "char"),
            Type::String => write!(f, "string"),
            Type::Pointer(inner) => write!(f, "{}*", inner),
            Type::Reference(inner, is_mut) => {
                        if *is_mut {
                            write!(f, "&mut {}", inner)
                        } else {
                            write!(f, "&{}", inner)
                        }
                    },
            Type::Array(elem, size) => write!(f, "[{} x {}]", size, elem),
            Type::Vector(elem) => write!(f, "vector<{}>", elem),
            Type::Slice(elem) => write!(f, "slice<{}>", elem),
            Type::Struct(name, _) => write!(f, "struct {}", name),
            Type::Function(params, ret) => {
                        write!(f, "fn(")?;
                        for (i, param) in params.iter().enumerate() {
                            if i > 0 {
                                write!(f, ", ")?;
                            }
                            write!(f, "{}", param)?;
                        }
                        write!(f, ") -> {}", ret)
                    }
            Type::Union(types) => {
                        write!(f, "(")?;
                        for (i, ty) in types.iter().enumerate() {
                            if i > 0 {
                                write!(f, " | ")?;
                            }
                            write!(f, "{}", ty)?;
                        }
                        write!(f, ")")
                    }
            Type::Intersection(types) => {
                        write!(f, "(")?;
                        for (i, ty) in types.iter().enumerate() {
                            if i > 0 {
                                write!(f, " & ")?;
                            }
                            write!(f, "{}", ty)?;
                        }
                        write!(f, ")")
                    }
            Type::Generic(name, args) => {
                        write!(f, "{}<", name)?;
                        for (i, arg) in args.iter().enumerate() {
                            if i > 0 {
                                write!(f, ", ")?;
                            }
                            write!(f, "{}", arg)?;
                        }
                        write!(f, ">")
                    }
            Type::Meta(inner) => write!(f, "type<{}>", inner),
            Type::Optional(inner) => write!(f, "{}?", inner),
            Type::Unknown => write!(f, "unknown"),
            Type::Result(ok_type, err_type) => write!(f, "Result<{}, {}>", ok_type, err_type),
            Type::Tuple(items) => {
                write!(f, "(")?;
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", item)?;
                }
                write!(f, ")")
            },
            Type::Trait(name) => write!(f, "dyn {}", name),
            Type::Existential(base, items) => {
                write!(f, "impl {} + ", base)?;
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        write!(f, " + ")?;
                    }
                    write!(f, "{}", item)?;
                }
                Ok(())
            },
            Type::Dependent(name, expr) => write!(f, "{}[{}]", name, expr),
            Type::TypeVar(name) => write!(f, "{}", name),
            Type::Constrained(base_type, type_constraints) => {
                write!(f, "{} where ", base_type)?;
                for (i, constraint) in type_constraints.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    match constraint {
                        TypeConstraint::TraitBound(trait_name) => write!(f, ": {}", trait_name)?,
                        TypeConstraint::Equals(ty) => write!(f, "= {}", ty)?,
                        TypeConstraint::SubType(ty) => write!(f, "<: {}", ty)?,
                        TypeConstraint::SuperType(ty) => write!(f, ">: {}", ty)?,
                        TypeConstraint::ValueConstraint(expr) => write!(f, "| {}", expr)?,
                        TypeConstraint::Structural(fields) => {
                            write!(f, "{{ ")?;
                            for (j, field) in fields.iter().enumerate() {
                                if j > 0 {
                                    write!(f, ", ")?;
                                }
                                write!(f, "{}", field)?;
                            }
                            write!(f, " }}")?;
                        }
                    }
                }
                Ok(())
            },
            Type::Recursive(name, inner_type) => write!(f, "rec {}. {}", name, inner_type),
            Type::Error => write!(f, "<error-type>"),
        }
    }
}

/// IR値
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// 整数値
    Integer(i64),
    /// 浮動小数点値
    Float(f64),
    /// 論理値
    Boolean(bool),
    /// 文字値
    Char(char),
    /// 文字列値
    String(String),
    /// null値
    Null,
    /// 構造体値
    Struct(String, Vec<Value>),
    /// 配列値
    Array(Vec<Value>),
    /// 関数参照
    FunctionRef(String),
    /// グローバル変数参照
    GlobalRef(String),
    /// ローカル変数参照
    LocalRef(String),
    /// 一時変数参照
    TempRef(usize),
    /// 未定義値
    Undefined,
    /// 値なし
    None,
}

impl Value {
    /// 値の型を推論する
    pub fn infer_type(&self) -> Type {
        match self {
            Value::Integer(_) => Type::Integer(64), // デフォルトは64ビット
            Value::Float(_) => Type::Double,
            Value::Boolean(_) => Type::Boolean,
            Value::Char(_) => Type::Char,
            Value::String(_) => Type::String,
            Value::Null => Type::Pointer(Box::new(Type::Void)),
            Value::Struct(name, _) => Type::Struct(name.clone(), Vec::new()),
            Value::Array(elements) => {
                if let Some(first) = elements.first() {
                    Type::Array(Box::new(first.infer_type()), elements.len())
                } else {
                    Type::Array(Box::new(Type::Unknown), 0)
                }
            }
            Value::FunctionRef(_) => Type::Function(Vec::new(), Box::new(Type::Unknown)),
            Value::GlobalRef(_) | Value::LocalRef(_) | Value::TempRef(_) => Type::Unknown,
            Value::Undefined => Type::Unknown,
            Value::None => Type::Void,
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Integer(i) => write!(f, "{}", i),
            Value::Float(fl) => write!(f, "{}", fl),
            Value::Boolean(b) => write!(f, "{}", b),
            Value::Char(c) => write!(f, "'{}'", c),
            Value::String(s) => write!(f, "\"{}\"", s),
            Value::Null => write!(f, "null"),
            Value::Struct(name, _) => write!(f, "{} {{...}}", name),
            Value::Array(_) => write!(f, "[...]"),
            Value::FunctionRef(name) => write!(f, "fn:{}", name),
            Value::GlobalRef(name) => write!(f, "@{}", name),
            Value::LocalRef(name) => write!(f, "%{}", name),
            Value::TempRef(id) => write!(f, "%t{}", id),
            Value::Undefined => write!(f, "undefined"),
            Value::None => write!(f, "none"),
        }
    }
}

/// 命令オペコード
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum OpCode {
    // メモリ操作
    Alloca,   // スタック上に領域確保
    Load,     // メモリから読み込み
    Store,    // メモリに書き込み
    GetElementPtr, // 構造体/配列の要素アドレス計算
    
    // 算術演算
    Add,      // 加算
    Sub,      // 減算
    Mul,      // 乗算
    Div,      // 除算
    Rem,      // 剰余
    Neg,      // 符号反転
    
    // 論理演算
    And,      // 論理積
    Or,       // 論理和
    Xor,      // 排他的論理和
    Not,      // 論理否定
    
    // ビット演算
    Shl,      // 左シフト
    Shr,      // 右シフト
    BitAnd,   // ビットごとのAND
    BitOr,    // ビットごとのOR
    BitXor,   // ビットごとのXOR
    BitNot,   // ビット反転
    
    // 比較演算
    Icmp,     // 整数比較
    Fcmp,     // 浮動小数点比較
    
    // 制御フロー
    Br,       // 分岐
    CondBr,   // 条件分岐
    Switch,   // スイッチ
    Return,   // 関数からの戻り
    
    // 関数呼び出し
    Call,     // 関数呼び出し
    
    // 型操作
    Cast,     // 型キャスト
    Phi,      // ファイ関数（SSA）
    
    // その他
    Nop,      // 何もしない
}

/// 比較述語
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Predicate {
    // 整数比較
    Eq,       // 等しい
    Ne,       // 等しくない
    Slt,      // 符号付き小なり
    Sle,      // 符号付き以下
    Sgt,      // 符号付き大なり
    Sge,      // 符号付き以上
    Ult,      // 符号なし小なり
    Ule,      // 符号なし以下
    Ugt,      // 符号なし大なり
    Uge,      // 符号なし以上
    
    // 浮動小数点比較
    Oeq,      // 順序付き等しい
    One,      // 順序付き等しくない
    Olt,      // 順序付き小なり
    Ole,      // 順序付き以下
    Ogt,      // 順序付き大なり
    Oge,      // 順序付き以上
    Ueq,      // 順序なし等しい
    Une,      // 順序なし等しくない
    Unlt,     // 順序なし小なり
    Unle,     // 順序なし以下
    Ungt,     // 順序なし大なり
    Unge,     // 順序なし以上
}

/// 命令オペランド
#[derive(Debug, Clone, PartialEq)]
pub enum Operand {
    /// 定数値
    Constant(Value),
    /// レジスタ/変数参照
    Register(String),
    /// 基本ブロック参照
    Block(String),
    /// 関数参照
    Function(String),
    /// グローバル変数参照
    Global(String),
    /// 型参照
    Type(Type),
    /// 比較述語
    Predicate(Predicate),
}

impl fmt::Display for Operand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Operand::Constant(val) => write!(f, "{}", val),
            Operand::Register(name) => write!(f, "%{}", name),
            Operand::Block(label) => write!(f, "label %{}", label),
            Operand::Function(name) => write!(f, "@{}", name),
            Operand::Global(name) => write!(f, "@{}", name),
            Operand::Type(ty) => write!(f, "{}", ty),
            Operand::Predicate(pred) => write!(f, "{:?}", pred),
        }
    }
}

/// IR命令
#[derive(Debug, Clone)]
pub struct Instruction {
    /// 操作コード
    pub opcode: OpCode,
    /// 結果の格納先（オプション）
    pub result: Option<String>,
    /// 結果の型
    pub result_type: Type,
    /// オペランドリスト
    pub operands: Vec<Operand>,
    /// デバッグ情報（元のソースコード位置など）
    pub debug_info: Option<String>,
    /// メタデータ
    pub metadata: HashMap<String, String>,
}

impl Instruction {
    /// 新しい命令を作成
    pub fn new(
        opcode: OpCode,
        result: Option<String>,
        result_type: Type,
        operands: Vec<Operand>,
    ) -> Self {
        Self {
            opcode,
            result,
            result_type,
            operands,
            debug_info: None,
            metadata: HashMap::new(),
        }
    }
    
    /// デバッグ情報を設定
    pub fn with_debug_info(mut self, debug_info: impl Into<String>) -> Self {
        self.debug_info = Some(debug_info.into());
        self
    }
    
    /// メタデータを追加
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
    
    /// 結果を生成するかどうか
    pub fn has_result(&self) -> bool {
        self.result.is_some() && !matches!(self.result_type, Type::Void)
    }
    
    /// 終端命令かどうか
    pub fn is_terminator(&self) -> bool {
        matches!(self.opcode, OpCode::Br | OpCode::CondBr | OpCode::Switch | OpCode::Return)
    }
}

impl fmt::Display for Instruction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(result) = &self.result {
            write!(f, "%{} = ", result)?;
        }
        
        write!(f, "{:?}", self.opcode)?;
        
        if !matches!(self.result_type, Type::Void) {
            write!(f, " {}", self.result_type)?;
        }
        
        for operand in &self.operands {
            write!(f, " {}", operand)?;
        }
        
        if let Some(debug) = &self.debug_info {
            write!(f, " ; {}", debug)?;
        }
        
        if !self.metadata.is_empty() {
            write!(f, " !{{ ")?;
            for (i, (key, value)) in self.metadata.iter().enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }
                write!(f, "!{}: \"{}\"", key, value)?;
            }
            write!(f, " }}")?;
        }
        
        Ok(())
    }
}

/// 基本ブロック
#[derive(Debug, Clone)]
pub struct BasicBlock {
    /// ブロックのラベル
    pub label: String,
    /// 命令リスト
    pub instructions: Vec<Instruction>,
    /// 先行ブロック（制御フローグラフ用）
    pub predecessors: HashSet<String>,
    /// 後続ブロック（制御フローグラフ用）
    pub successors: HashSet<String>,
    /// このブロックのドミネータ（制御フロー解析用）
    pub dominator: Option<String>,
    /// このブロックが支配するブロック
    pub dominates: HashSet<String>,
    /// ループヘッダーかどうか
    pub is_loop_header: bool,
    /// ループの深さ
    pub loop_depth: usize,
}

impl BasicBlock {
    /// 新しい基本ブロックを作成
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            instructions: Vec::new(),
            predecessors: HashSet::new(),
            successors: HashSet::new(),
            dominator: None,
            dominates: HashSet::new(),
            is_loop_header: false,
            loop_depth: 0,
        }
    }
    
    /// 命令を追加
    pub fn add_instruction(&mut self, instruction: Instruction) {
        self.instructions.push(instruction);
    }
    
    /// 先行ブロックを追加
    pub fn add_predecessor(&mut self, label: impl Into<String>) {
        self.predecessors.insert(label.into());
    }
    
    /// 後続ブロックを追加
    pub fn add_successor(&mut self, label: impl Into<String>) {
        self.successors.insert(label.into());
    }
    
    /// ドミネータを設定
    pub fn set_dominator(&mut self, label: impl Into<String>) {
        self.dominator = Some(label.into());
    }
    
    /// 支配するブロックを追加
    pub fn add_dominates(&mut self, label: impl Into<String>) {
        self.dominates.insert(label.into());
    }
    
    /// ループヘッダーとして設定
    pub fn set_loop_header(&mut self, depth: usize) {
        self.is_loop_header = true;
        self.loop_depth = depth;
    }
    
    /// 終端命令を取得
    pub fn terminator(&self) -> Option<&Instruction> {
        self.instructions.iter().find(|inst| inst.is_terminator())
    }
    
    /// 終端命令を持つかどうか
    pub fn has_terminator(&self) -> bool {
        self.instructions.iter().any(|inst| inst.is_terminator())
    }
}

/// パラメータ
#[derive(Debug, Clone, PartialEq)]
pub struct Parameter {
    /// パラメータ名
    pub name: String,
    /// パラメータの型
    pub typ: Type,
    /// バイ・リファレンスか
    pub by_reference: bool,
    /// デフォルト値
    pub default_value: Option<Value>,
}

impl Parameter {
    /// 新しいパラメータを作成
    pub fn new(name: impl Into<String>, typ: Type, by_reference: bool) -> Self {
        Self {
            name: name.into(),
            typ,
            by_reference,
            default_value: None,
        }
    }
    
    /// デフォルト値を設定
    pub fn with_default(mut self, value: Value) -> Self {
        self.default_value = Some(value);
        self
    }
}

/// モジュール内の関数定義
#[derive(Debug, Clone)]
pub struct Function {
    /// 関数の名前
    pub name: String,
    /// 関数の型
    pub fn_type: Type,
    /// 関数の引数
    pub params: Vec<Value>,
    /// 関数のボディを構成する基本ブロック
    pub blocks: Vec<BasicBlock>,
    /// 関数の属性
    pub attributes: Vec<String>,
    /// 関数のリンケージタイプ
    pub linkage: Linkage,
    /// 関数の可視性
    pub visibility: Visibility,
    /// 関数のインライン属性
    pub inline: InlineAttr,
    /// 関数が実装されているか（宣言のみの場合はfalse）
    pub is_definition: bool,
    /// コンパイル時評価が可能か
    pub is_constexpr: bool,
    /// 曖昧性回避のためのシンボル名
    pub mangled_name: Option<String>,
    /// ソースコード上の位置
    pub source_location: Option<SourceLocation>,
}

/// モジュール内の構造体定義
#[derive(Debug, Clone)]
pub struct Struct {
    /// 構造体の名前
    pub name: String,
    /// 構造体のフィールド
    pub fields: Vec<(String, Type)>,
    /// 構造体のメソッド
    pub methods: Vec<Function>,
    /// 構造体が実装するトレイト
    pub implemented_traits: Vec<String>,
    /// パディングを自動的に挿入するか
    pub auto_padding: bool,
    /// 構造体のアライメント
    pub alignment: usize,
    /// 構造体のサイズ（バイト単位）
    pub size: usize,
    /// 構造体の属性
    pub attributes: Vec<String>,
    /// ソースコード上の位置
    pub source_location: Option<SourceLocation>,
}

/// モジュール内のグローバル変数定義
#[derive(Debug, Clone)]
pub struct Global {
    /// グローバル変数の名前
    pub name: String,
    /// グローバル変数の型
    pub var_type: Type,
    /// 初期値
    pub initializer: Option<Value>,
    /// グローバル変数の属性
    pub attributes: Vec<String>,
    /// リンケージタイプ
    pub linkage: Linkage,
    /// 可視性
    pub visibility: Visibility,
    /// 定数か否か
    pub is_constant: bool,
    /// スレッドローカルか否か
    pub is_thread_local: bool,
    /// アライメント
    pub alignment: usize,
    /// ソースコード上の位置
    pub source_location: Option<SourceLocation>,
}

/// モジュール
#[derive(Debug, Clone)]
pub struct Module {
    /// モジュール名
    pub name: String,
    /// 関数リスト
    pub functions: HashMap<String, Function>,
    /// グローバル変数リスト
    pub globals: HashMap<String, GlobalVariable>,
    /// 構造体定義
    pub structs: HashMap<String, Vec<Type>>,
    /// 依存モジュール
    pub dependencies: HashSet<String>,
    /// ソースファイル情報
    pub source_file: Option<String>,
    /// モジュールメタデータ
    pub metadata: HashMap<String, String>,
    /// ターゲットトリプル
    pub target_triple: Option<String>,
    /// データレイアウト
    pub data_layout: Option<String>,
}

impl Module {
    /// 新しいモジュールを作成
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            functions: HashMap::new(),
            globals: HashMap::new(),
            structs: HashMap::new(),
            dependencies: HashSet::new(),
            source_file: None,
            metadata: HashMap::new(),
            target_triple: None,
            data_layout: None,
        }
    }
    
    /// 関数を追加
    pub fn add_function(&mut self, function: Function) {
        self.functions.insert(function.name.clone(), function);
    }
    
    /// 関数を取得
    pub fn get_function(&self, name: &str) -> Option<&Function> {
        self.functions.get(name)
    }
    
    /// 関数を取得（可変参照）
    pub fn get_function_mut(&mut self, name: &str) -> Option<&mut Function> {
        self.functions.get_mut(name)
    }
    
    /// グローバル変数を追加
    pub fn add_global(&mut self, global: GlobalVariable) {
        self.globals.insert(global.name.clone(), global);
    }
    
    /// グローバル変数を取得
    pub fn get_global(&self, name: &str) -> Option<&GlobalVariable> {
        self.globals.get(name)
    }
    
    /// 構造体を追加
    pub fn add_struct(&mut self, name: impl Into<String>, fields: Vec<Type>) {
        self.structs.insert(name.into(), fields);
    }
    
    /// 構造体を取得
    pub fn get_struct(&self, name: &str) -> Option<&Vec<Type>> {
        self.structs.get(name)
    }
    
    /// 依存モジュールを追加
    pub fn add_dependency(&mut self, module_name: impl Into<String>) {
        self.dependencies.insert(module_name.into());
    }
    
    /// ソースファイル情報を設定
    pub fn set_source_file(&mut self, file_path: impl Into<String>) {
        self.source_file = Some(file_path.into());
    }
    
    /// ターゲットトリプルを設定
    pub fn set_target_triple(&mut self, triple: impl Into<String>) {
        self.target_triple = Some(triple.into());
    }
    
    /// データレイアウトを設定
    pub fn set_data_layout(&mut self, layout: impl Into<String>) {
        self.data_layout = Some(layout.into());
    }
    
    /// メタデータを設定
    pub fn set_metadata(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.metadata.insert(key.into(), value.into());
    }
}

/// IRダンプユーティリティ
pub fn dump_module(module: &Module) -> String {
    let mut output = String::new();
    
    output.push_str(&format!("; Module: {}\n", module.name));
    if let Some(source) = &module.source_file {
        output.push_str(&format!("; Source: {}\n", source));
    }
    if let Some(triple) = &module.target_triple {
        output.push_str(&format!("; Target: {}\n", triple));
    }
    if let Some(layout) = &module.data_layout {
        output.push_str(&format!("; Data Layout: {}\n", layout));
    }
    output.push_str("\n");
    
    // 構造体定義
    if !module.structs.is_empty() {
        output.push_str("; Struct definitions\n");
        for (name, fields) in &module.structs {
            output.push_str(&format!("%struct.{} = type {{", name));
            for (i, field) in fields.iter().enumerate() {
                if i > 0 {
                    output.push_str(", ");
                }
                output.push_str(&format!("{}", field));
            }
            output.push_str("}\n");
        }
        output.push_str("\n");
    }
    
    // グローバル変数
    if !module.globals.is_empty() {
        output.push_str("; Global variables\n");
        for global in module.globals.values() {
            let linkage = match global.linkage {
                Linkage::External => "external ",
                Linkage::Internal => "",
                Linkage::Private => "private ",
                Linkage::Weak => "weak ",
                Linkage::Common => "common ",
                Linkage::Appending => "appending ",
                Linkage::LinkOnce => "linkonce ",
                Linkage::LinkOnceODR => "linkonce_odr ",
                Linkage::WeakODR => "weak_odr ",
            };
            
            let constant = if global.is_constant { "constant " } else { "global " };
            let thread_local = if global.is_thread_local { "thread_local " } else { "" };
            
            output.push_str(&format!("@{} = {}{}{}{}", global.name, linkage, thread_local, constant, global.typ));
            
            if let Some(init) = &global.initializer {
                output.push_str(&format!(" {}", init));
            }
            
            if let Some(align) = global.alignment {
                output.push_str(&format!(", align {}", align));
            }
            
            output.push_str("\n");
        }
        output.push_str("\n");
    }
    
    // 関数宣言・定義
    for function in module.functions.values() {
        // 関数シグネチャ
        if function.is_external {
            output.push_str("declare ");
        } else {
            output.push_str("define ");
        }
        
        output.push_str(&format!("{} @{}(", function.return_type, function.name));
        
        // パラメータ
        for (i, param) in function.parameters.iter().enumerate() {
            if i > 0 {
                output.push_str(", ");
            }
            
            if param.by_reference {
                output.push_str(&format!("{} * %{}", param.typ, param.name));
            } else {
                output.push_str(&format!("{} %{}", param.typ, param.name));
            }
        }
        
        if function.is_variadic {
            if !function.parameters.is_empty() {
                output.push_str(", ");
            }
            output.push_str("...");
        }
        
        output.push_str(")");
        
        // 属性
        if !function.attributes.is_empty() {
            for attr in &function.attributes {
                output.push_str(&format!(" {}", attr));
            }
        }
        
        if function.is_external {
            output.push_str("\n");
            continue;
        }
        
        output.push_str(" {\n");
        
        // 基本ブロック
        for block in &function.blocks {
            output.push_str(&format!("{}:\n", block.label));
            
            // 命令
            for inst in &block.instructions {
                output.push_str(&format!("  {}\n", inst));
            }
            
            output.push_str("\n");
        }
        
        output.push_str("}\n\n");
    }
    
    output
}
