#!/usr/bin/env python3
import re
import os
import glob
import shutil
from pathlib import Path

# 対象ディレクトリ
target_dir = "crates/swiftlight-compiler/src"

# バックアップディレクトリ
backup_dir = "crates/backups"
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

# 修正カウンター
counters = {
    "build_load": 0,
    "build_struct_gep": 0,
    "address_space": 0,
    "fn_type": 0,
    "other": 0
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
    
    # 1. build_load の修正
    # パターン: build_load(type, ptr, name) -> build_load(ptr, name)
    content = re.sub(
        r'build_load\(([^,]+),\s*([^,]+),\s*([^,\)]+)\)',
        r'build_load(\2, \3)',
        content
    )
    # 1引数のbuild_loadを検出して修正
    content = re.sub(
        r'build_load\(([^,\)]+)\)',
        r'build_load(\1, "load")',
        content
    )
    
    # 2. build_struct_gep の修正
    # パターン: build_struct_gep(type, ptr, idx, name) -> build_struct_gep(ptr, idx, name)
    content = re.sub(
        r'build_struct_gep\(([^,]+),\s*([^,]+),\s*([^,]+),\s*([^,\)]+)\)',
        r'build_struct_gep(\2, \3, \4)',
        content
    )
    
    # 3. AddressSpace::Generic の修正
    # AddressSpace::Generic -> AddressSpace::default()
    content = re.sub(
        r'AddressSpace::Generic',
        r'AddressSpace::default()',
        content
    )
    
    # 4. fn_type の引数型の修正
    # fn_type(&vec, ...) -> fn_type(&[...], ...)
    content = re.sub(
        r'fn_type\(&(param_types|llvm_param_types|arg_types|vec![^,]+),\s*([^,\)]+)\)',
        r'fn_type(&[\1.as_slice()], \2)',
        content
    )
    
    # 5. 型変換の問題
    # BasicTypeEnum -> AnyTypeEnum の問題
    content = re.sub(
        r'(BasicTypeEnum.+)\.into\(\)',
        r'AnyTypeEnum::from(\1)',
        content
    )
    
    # 6. void_type().into() の問題
    content = re.sub(
        r'void_type\(\)\.into\(\)',
        r'void_type()',
        content
    )
    
    # 変更があれば更新
    if content != original_content:
        with open(file_path, "w", encoding="utf-8") as f:
            f.write(content)
        
        # カウンターを更新
        if "build_load" in content:
            counters["build_load"] += 1
        if "build_struct_gep" in content:
            counters["build_struct_gep"] += 1
        if "AddressSpace" in content:
            counters["address_space"] += 1
        if "fn_type" in content:
            counters["fn_type"] += 1
        counters["other"] += 1

print("修正完了！")
for key, count in counters.items():
    print(f"{key}: {count}ファイル修正") 