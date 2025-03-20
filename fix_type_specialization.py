#!/usr/bin/env python3
import re

# 修正ターゲットファイル
target_file = "crates/swiftlight-compiler/src/middleend/optimization/type_specialization.rs"
# ファイルを読み込み
with open(target_file, "r") as f:
    content = f.read()

# エラーを修正
# DependentTypeConstraintの構造体定義の問題を修正
# 全称量化制約の閉じる括弧が欠けている
fixed_content = content.replace(
    """    // 全称量化制約
    ForAll {
        variables: Vec<(String, DependentTypeExpression)>,
impl TypeLevelComputationEngine {""",
    """    // 全称量化制約
    ForAll {
        variables: Vec<(String, DependentTypeExpression)>,
        constraint: Box<DependentTypeConstraint>,
    },
}

impl TypeLevelComputationEngine {"""
)

# 修正したコンテンツを書き込む
with open(target_file, "w") as f:
    f.write(fixed_content)

print(f"ファイル {target_file} の修正が完了しました")
