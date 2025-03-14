// SwiftLight IR表現モジュール
//
// このモジュールはコンパイラの中間表現(IR)に関する構造体と列挙型を定義します。
// LLVM IRへの変換前の抽象表現として機能します。

use std::collections::{HashMap, HashSet};
use std::fmt;

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
    /// 配列型
    Array(Box<Type>, usize),
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
    /// 未知の型
    Unknown,
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
            Type::Char => 1,
            Type::String => 8, // ポインタサイズ
            Type::Pointer(_) => 8, // 64ビットアーキテクチャを仮定
            Type::Array(elem_type, count) => elem_type.size() * count,
            Type::Struct(_, fields) => {
                // 単純な合計（アラインメントは無視）
                fields.iter().map(|field| field.size()).sum()
            }
            Type::Function(_, _) => 8, // 関数ポインタ
            Type::Union(types) => {
                // ユニオンは最大のメンバーサイズ
                types.iter().map(|t| t.size()).max().unwrap_or(0)
            }
            Type::Intersection(_) => 8, // インターフェースとして扱う
            Type::Generic(_, _) => 8,   // 具体化されていないのでポインタサイズ
            Type::Meta(_) => 8,         // 型情報のポインタ
            Type::Optional(inner) => inner.size() + 1, // 内部型 + フラグ
            Type::Unknown => 0,
        }
    }
    
    /// 型が値型かどうか
    pub fn is_value_type(&self) -> bool {
        match self {
            Type::Void | Type::Integer(_) | Type::Float | Type::Double |
            Type::Boolean | Type::Char => true,
            Type::Pointer(_) | Type::String | Type::Function(_, _) => false,
            Type::Array(_, _) | Type::Struct(_, _) => false, // SwiftLightでは参照型
            Type::Union(_) | Type::Intersection(_) => false,
            Type::Generic(_, _) | Type::Meta(_) => false,
            Type::Optional(inner) => inner.is_value_type(),
            Type::Unknown => false,
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
            Type::Array(elem, size) => write!(f, "[{} x {}]", size, elem),
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
    Ult,      // 順序なし小なり
    Ule,      // 順序なし以下
    Ugt,      // 順序なし大なり
    Uge,      // 順序なし以上
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

/// 関数
#[derive(Debug, Clone)]
pub struct Function {
    /// 関数名
    pub name: String,
    /// 戻り値の型
    pub return_type: Type,
    /// パラメータリスト
    pub parameters: Vec<Parameter>,
    /// 基本ブロック
    pub blocks: Vec<BasicBlock>,
    /// 可変引数関数か
    pub is_variadic: bool,
    /// 外部関数か
    pub is_external: bool,
    /// 属性
    pub attributes: HashSet<String>,
    /// ローカル変数の型情報
    pub locals: HashMap<String, Type>,
    /// 一時変数カウンタ
    pub temp_counter: usize,
    /// インライン関数か
    pub is_inline: bool,
    /// 再帰関数か
    pub is_recursive: bool,
    /// 純粋関数か（副作用なし）
    pub is_pure: bool,
}

impl Function {
    /// 新しい関数を作成
    pub fn new(name: impl Into<String>, return_type: Type) -> Self {
        Self {
            name: name.into(),
            return_type,
            parameters: Vec::new(),
            blocks: Vec::new(),
            is_variadic: false,
            is_external: false,
            attributes: HashSet::new(),
            locals: HashMap::new(),
            temp_counter: 0,
            is_inline: false,
            is_recursive: false,
            is_pure: false,
        }
    }
    
    /// パラメータを追加
    pub fn add_parameter(&mut self, param: Parameter) {
        self.parameters.push(param);
    }
    
    /// 基本ブロックを追加
    pub fn add_block(&mut self, block: BasicBlock) {
        self.blocks.push(block);
    }
    
    /// エントリーブロックの参照を取得
    pub fn entry_block(&self) -> Option<&BasicBlock> {
        self.blocks.first()
    }
    
    /// エントリーブロックの可変参照を取得
    pub fn entry_block_mut(&mut self) -> Option<&mut BasicBlock> {
        self.blocks.first_mut()
    }
    
    /// 関数の属性を設定
    pub fn with_attribute(mut self, attribute: impl Into<String>) -> Self {
        self.attributes.insert(attribute.into());
        self
    }
    
    /// 外部関数として設定
    pub fn external(mut self) -> Self {
        self.is_external = true;
        self
    }
    
    /// 可変引数関数として設定
    pub fn variadic(mut self) -> Self {
        self.is_variadic = true;
        self
    }
    
    /// インライン関数として設定
    pub fn inline(mut self) -> Self {
        self.is_inline = true;
        self.attributes.insert("inline".to_string());
        self
    }
    
    /// 純粋関数として設定
    pub fn pure(mut self) -> Self {
        self.is_pure = true;
        self.attributes.insert("pure".to_string());
        self
    }
    
    /// 再帰関数として設定
    pub fn recursive(mut self) -> Self {
        self.is_recursive = true;
        self.attributes.insert("recursive".to_string());
        self
    }
    
    /// 新しい一時変数名を生成
    pub fn new_temp(&mut self) -> String {
        let temp_name = format!("t{}", self.temp_counter);
        self.temp_counter += 1;
        temp_name
    }
    
    /// ブロックを名前で検索
    pub fn get_block(&self, label: &str) -> Option<&BasicBlock> {
        self.blocks.iter().find(|block| block.label == label)
    }
    
    /// ブロックを名前で検索（可変参照）
    pub fn get_block_mut(&mut self, label: &str) -> Option<&mut BasicBlock> {
        self.blocks.iter_mut().find(|block| block.label == label)
    }
    
    /// ローカル変数を追加
    pub fn add_local(&mut self, name: impl Into<String>, typ: Type) {
        self.locals.insert(name.into(), typ);
    }
    
    /// ローカル変数の型を取得
    pub fn get_local_type(&self, name: &str) -> Option<&Type> {
        self.locals.get(name)
    }
    
    /// 関数が空かどうかを確認
    pub fn is_empty(&self) -> bool {
        self.blocks.is_empty() || self.blocks.iter().all(|block| block.instructions.is_empty())
    }
}

/// グローバル変数
#[derive(Debug, Clone)]
pub struct GlobalVariable {
    /// 変数名
    pub name: String,
    /// 変数の型
    pub typ: Type,
    /// 初期値（オプション）
    pub initializer: Option<Value>,
    /// 定数か
    pub is_constant: bool,
    /// リンケージ情報
    pub linkage: Linkage,
    /// アラインメント
    pub alignment: Option<usize>,
    /// スレッドローカルか
    pub is_thread_local: bool,
}

impl GlobalVariable {
    /// 新しいグローバル変数を作成
    pub fn new(name: impl Into<String>, typ: Type) -> Self {
        Self {
            name: name.into(),
            typ,
            initializer: None,
            is_constant: false,
            linkage: Linkage::Internal,
            alignment: None,
            is_thread_local: false,
        }
    }
    
    /// 初期値を設定
    pub fn with_initializer(mut self, value: Value) -> Self {
        self.initializer = Some(value);
        self
    }
    
    /// 定数として設定
    pub fn constant(mut self) -> Self {
        self.is_constant = true;
        self
    }
    
    /// リンケージを設定
    pub fn with_linkage(mut self, linkage: Linkage) -> Self {
        self.linkage = linkage;
        self
    }
    
    /// アラインメントを設定
    pub fn with_alignment(mut self, alignment: usize) -> Self {
        self.alignment = Some(alignment);
        self
    }
    
    /// スレッドローカルとして設定
    pub fn thread_local(mut self) -> Self {
        self.is_thread_local = true;
        self
    }
}

/// リンケージ種別
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Linkage {
    /// 内部シンボル
    Internal,
    /// 外部シンボル
    External,
    /// 弱いシンボル
    Weak,
    /// プライベートシンボル
    Private,
    /// 共通シンボル
    Common,
    /// アピアランス
    Appending,
    /// リンクワンス
    LinkOnce,
    /// リンクワンスODR
    LinkOnceODR,
    /// 弱ODR
    WeakODR,
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
