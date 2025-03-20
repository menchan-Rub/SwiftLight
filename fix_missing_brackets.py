#!/usr/bin/env python3
import os
import shutil

# 修正ターゲットファイル
target_file = "crates/swiftlight-compiler/src/middleend/ir/mod.rs"

# バックアップ作成
backup_file = target_file + ".brackets-bak"
shutil.copy2(target_file, backup_file)
print(f"バックアップを作成しました: {backup_file}")

# ファイル内容を読み込み
with open(target_file, "r") as f:
    content = f.read()

# 欠けている閉じ括弧を追加
if not content.strip().endswith("}"):
    # ファイルの末尾に閉じ括弧を追加
    closing_brackets = """
                            }
                        }
                    }
                }
            }
        }
    }
}"""
    
    # 修正した内容を書き込む
    with open(target_file, "w") as f:
        f.write(content + closing_brackets)
    
    print("ファイルの末尾に閉じ括弧を追加しました")
else:
    print("ファイルは既に適切に閉じられています")

print(f"ファイル {target_file} の修正が完了しました") 