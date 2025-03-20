#!/usr/bin/env python3
import os
import shutil

# 修正ターゲットファイル
target_file = "crates/swiftlight-compiler/src/middleend/ir/mod.rs"

# バックアップ作成
backup_file = target_file + ".final"
shutil.copy2(target_file, backup_file)
print(f"バックアップを作成しました: {backup_file}")

# 元のファイルを読み込み
with open(target_file, "r") as f:
    lines = f.readlines()

# 行を2167行までに切り詰め（末尾の閉じ括弧を除去）
new_lines = lines[:2167]

# 適切な閉じ括弧を追加
new_lines.append("                            };\n")
new_lines.append("                        }\n")
new_lines.append("                    }\n")
new_lines.append("                }\n")
new_lines.append("            }\n")
new_lines.append("        }\n")
new_lines.append("    }\n")
new_lines.append("}\n")

# 修正した内容を書き込む
with open(target_file, "w") as f:
    f.writelines(new_lines)

print(f"ファイル {target_file} の修正が完了しました") 