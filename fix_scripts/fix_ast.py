#!/usr/bin/env python3
import re
import os
import glob
import shutil

# 対象ディレクトリ
target_dir = "crates/swiftlight-compiler/src"

# バックアップディレクトリ
backup_dir = "crates/backups/ast"
os.makedirs(backup_dir, exist_ok=True)

# 修正対象のファイルパターン
file_patterns = [
    "**/*.rs",
]

# 全ての対象ファイルを取得
all_files = []
for pattern in file_patterns:
    all_files.extend(glob.glob(f"{target_dir}/{pattern}", recursive=True))

print(f"修正対象ファイル数: {len(all_files)}")

# AST変更マッピング
replacements = [
    # TypeKind変更
    (r'TypeKind::Int', r'TypeKind::Primitive(PrimitiveType::Int)'),
    (r'TypeKind::Float', r'TypeKind::Primitive(PrimitiveType::Float)'),
    (r'TypeKind::Bool', r'TypeKind::Primitive(PrimitiveType::Bool)'),
    (r'TypeKind::String', r'TypeKind::Primitive(PrimitiveType::String)'),
    (r'TypeKind::Char', r'TypeKind::Primitive(PrimitiveType::Char)'),
    (r'TypeKind::Void', r'TypeKind::Primitive(PrimitiveType::Void)'),
    
    # DeclarationKind変更
    (r'DeclarationKind::Function', r'DeclarationKind::FunctionDecl'),
    (r'DeclarationKind::Variable', r'DeclarationKind::VariableDecl'),
    (r'DeclarationKind::Constant', r'DeclarationKind::ConstantDecl'),
    (r'DeclarationKind::Struct', r'DeclarationKind::StructDecl'),
    (r'DeclarationKind::Enum', r'DeclarationKind::EnumDecl'),
    (r'DeclarationKind::Trait', r'DeclarationKind::TraitDecl'),
    (r'DeclarationKind::Implementation', r'DeclarationKind::ImplementationDecl'),
    (r'DeclarationKind::TypeAlias', r'DeclarationKind::TypeAliasDecl'),
    (r'DeclarationKind::Import', r'DeclarationKind::ImportDecl'),
    
    # StatementKind変更
    (r'StatementKind::Expression', r'StatementKind::ExpressionStmt'),
    (r'StatementKind::Declaration', r'StatementKind::DeclarationStmt'),
    (r'StatementKind::If', r'StatementKind::IfStmt'),
    (r'StatementKind::While', r'StatementKind::WhileStmt'),
    (r'StatementKind::For', r'StatementKind::ForStmt'),
    (r'StatementKind::Return', r'StatementKind::ReturnStmt'),
    (r'StatementKind::Break', r'StatementKind::BreakStmt'),
    (r'StatementKind::Continue', r'StatementKind::ContinueStmt'),
    
    # ExpressionKind変更
    (r'ExpressionKind::ResultBind', r'ExpressionKind::ResultBindExpr'),
    (r'ExpressionKind::ResultMap', r'ExpressionKind::ResultMapExpr'),
    
    # BinaryOperator変更
    (r'BinaryOperator::LessThanEqual', r'BinaryOperator::LessEqual'),
    (r'BinaryOperator::GreaterThanEqual', r'BinaryOperator::GreaterEqual'),
    (r'BinaryOperator::LogicalAnd', r'BinaryOperator::And'),
    (r'BinaryOperator::LogicalOr', r'BinaryOperator::Or'),
    (r'BinaryOperator::BitwiseAnd', r'BinaryOperator::BitAnd'),
    (r'BinaryOperator::BitwiseOr', r'BinaryOperator::BitOr'),
    (r'BinaryOperator::BitwiseXor', r'BinaryOperator::BitXor'),
    
    # UnaryOperator変更
    (r'UnaryOperator::BitwiseNot', r'UnaryOperator::BitNot'),
    
    # Program構造体の変更
    (r'program\.file_name', r'program.source_path'),
    (r'program\.statements', r'program.declarations'),
    
    # エラー関数の修正
    (r'CompilerError::code_generation_error\(([^,]+),\s*([^,\)]+)\)', r'CompilerError::new(ErrorKind::CodeGeneration, \1, \2)'),
]

# 修正カウンター
counters = {
    "type_kind": 0,
    "declaration_kind": 0,
    "statement_kind": 0,
    "expression_kind": 0,
    "binary_operator": 0,
    "unary_operator": 0,
    "program_struct": 0,
    "error_function": 0,
}

# 各ファイルを処理
for file_path in all_files:
    # バックアップ作成
    backup_path = os.path.join(backup_dir, os.path.basename(file_path) + ".bak")
    shutil.copy2(file_path, backup_path)
    
    # ファイル内容を読み込む
    with open(file_path, "r", encoding="utf-8", errors="ignore") as f:
        content = f.read()
    
    # 元のコンテンツを保存
    original_content = content
    
    # 各置換を適用
    for old, new in replacements:
        if re.search(old, content):
            if "TypeKind" in old:
                counters["type_kind"] += 1
            elif "DeclarationKind" in old:
                counters["declaration_kind"] += 1
            elif "StatementKind" in old:
                counters["statement_kind"] += 1
            elif "ExpressionKind" in old:
                counters["expression_kind"] += 1
            elif "BinaryOperator" in old:
                counters["binary_operator"] += 1
            elif "UnaryOperator" in old:
                counters["unary_operator"] += 1
            elif "program" in old:
                counters["program_struct"] += 1
            elif "CompilerError" in old:
                counters["error_function"] += 1
            
            content = re.sub(old, new, content)
    
    # 変更があれば更新
    if content != original_content:
        with open(file_path, "w", encoding="utf-8") as f:
            f.write(content)

print("修正完了！")
for key, count in counters.items():
    print(f"{key}: {count}ファイル修正") 