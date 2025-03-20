#!/usr/bin/env python3
import re
import os

# 対象ファイル
target_file = "crates/swiftlight-compiler/src/utils/logger.rs"

# バックアップディレクトリ
backup_dir = "crates/backups"
os.makedirs(backup_dir, exist_ok=True)

# バックアップ作成
backup_path = os.path.join(backup_dir, "logger.rs.bak")
if os.path.exists(target_file):
    with open(target_file, "r") as src:
        with open(backup_path, "w") as dst:
            dst.write(src.read())
    print(f"バックアップを作成しました: {backup_path}")

# ファイルを読み込み
if os.path.exists(target_file):
    with open(target_file, "r", encoding="utf-8") as f:
        content = f.read()
    
    # 修正1: Loggerトレイトからジェネリックメソッドを削除
    # 元のLoggerトレイト定義を検索
    logger_trait_pattern = r"pub trait Logger(\s*):([^{]+)\{(.*?)\}"
    logger_trait_match = re.search(logger_trait_pattern, content, re.DOTALL)
    
    if logger_trait_match:
        # 現在のトレイト内容を取得
        trait_content = logger_trait_match.group(3)
        
        # ジェネリックメソッドを検出して削除
        non_generic_methods = []
        for method in re.finditer(r"fn\s+([^<{]+)(<[^>]+>)?\s*(\([^)]*\))\s*->\s*([^;]+);", trait_content):
            method_name = method.group(1).strip()
            is_generic = method.group(2) is not None
            
            if not is_generic:
                non_generic_methods.append(method.group(0))
        
        # 新しいトレイト定義を作成
        non_generic_methods_str = "\n    ".join(non_generic_methods)
        new_logger_trait = """pub trait Logger: Send + Sync {
    // 非ジェネリックメソッドのみ
    """ + non_generic_methods_str + """
    
    // 文字列バージョン
    fn debug_str(&self, message: &str);
    fn info_str(&self, message: &str);
    fn warn_str(&self, message: &str);
    fn error_str(&self, message: &str);
    fn trace_str(&self, message: &str);
    fn critical_str(&self, message: &str);
}

// ジェネリックなロギング機能を提供するトレイト
pub trait GenericLogger: Logger {
    fn debug<M: fmt::Display>(&self, message: M);
    fn info<M: fmt::Display>(&self, message: M);
    fn warn<M: fmt::Display>(&self, message: M);
    fn error<M: fmt::Display>(&self, message: M);
    fn trace<M: fmt::Display>(&self, message: M);
    fn critical<M: fmt::Display>(&self, message: M);
}

// GenericLoggerの基本実装
impl<T: Logger> GenericLogger for T {
    fn debug<M: fmt::Display>(&self, message: M) {
        self.debug_str(&message.to_string());
    }
    
    fn info<M: fmt::Display>(&self, message: M) {
        self.info_str(&message.to_string());
    }
    
    fn warn<M: fmt::Display>(&self, message: M) {
        self.warn_str(&message.to_string());
    }
    
    fn error<M: fmt::Display>(&self, message: M) {
        self.error_str(&message.to_string());
    }
    
    fn trace<M: fmt::Display>(&self, message: M) {
        self.trace_str(&message.to_string());
    }
    
    fn critical<M: fmt::Display>(&self, message: M) {
        self.critical_str(&message.to_string());
    }
}"""
        
        # トレイト定義を置換
        content = content.replace(logger_trait_match.group(0), new_logger_trait)
        
        # 修正2: dyn Loggerを使用している部分を修正
        # dyn LoggerをBox<dyn Logger>に置き換え
        content = re.sub(r"(pub|let|const)?\s+([a-zA-Z_][a-zA-Z0-9_]*)\s*:\s*dyn Logger", 
                        r"\1 \2: Box<dyn Logger>", content)
        
        # &mut dyn LoggerをBox<dyn Logger>に置き換え
        content = re.sub(r"&mut dyn Logger", r"Box<dyn Logger>", content)
        
        # ファイルに書き戻す
        with open(target_file, "w", encoding="utf-8") as f:
            f.write(content)
        
        print(f"ファイル {target_file} を修正しました")
    else:
        print("Loggerトレイトが見つかりませんでした")
else:
    print(f"ファイル {target_file} が見つかりません") 