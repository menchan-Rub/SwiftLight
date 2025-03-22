# SwiftLight プロジェクト構造

このドキュメントでは、SwiftLight言語の実装に関するプロジェクト構造を説明します。
これは現在の実装状況（TODO.mdに基づく）と今後の拡張を考慮した構造です。

```
swiftlight/
├── .github/                              # GitHub関連設定
│   ├── ISSUE_TEMPLATE/                   # Issue用テンプレート
│   │   ├── bug_report.md                 # バグ報告テンプレート
│   │   ├── feature_request.md            # 機能リクエストテンプレート
│   │   └── security_report.md            # セキュリティ報告テンプレート
│   └── workflows/                        # CI/CDワークフロー
│       ├── build.yml                     # ビルドワークフロー定義
│       ├── test.yml                      # テストワークフロー定義
│       └── release.yml                   # リリースワークフロー定義
├── crates/                               # Rustクレート群
│   ├── swiftlight-compiler/              # コンパイラ本体
│   │   ├── src/                          # ソースコード
│   │   │   ├── frontend/                 # フロントエンド
│   │   │   │   ├── lexer/                # 字句解析器
│   │   │   │   │   ├── mod.rs            # モジュール定義
│   │   │   │   │   ├── token.rs          # トークン定義
│   │   │   │   │   └── unicode.rs        # Unicode関連処理
│   │   │   │   ├── parser/               # 構文解析器
│   │   │   │   │   ├── mod.rs            # モジュール定義
│   │   │   │   │   ├── ast.rs            # 抽象構文木定義
│   │   │   │   │   ├── grammar.rs        # 文法規則
│   │   │   │   │   └── error_recovery.rs # エラー回復機能
│   │   │   │   ├── semantic/             # 意味解析
│   │   │   │   │   ├── mod.rs            # モジュール定義
│   │   │   │   │   ├── name_resolution.rs# 名前解決
│   │   │   │   │   ├── type_checker.rs   # 型チェック
│   │   │   │   │   └── ownership_checker.rs # 所有権チェック
│   │   │   │   ├── diagnostic/           # 診断システム
│   │   │   │   │   ├── mod.rs            # モジュール定義
│   │   │   │   │   ├── error_codes.rs    # エラーコード
│   │   │   │   │   ├── message.rs        # メッセージ生成
│   │   │   │   │   └── reporting.rs      # 診断レポート出力
│   │   │   │   ├── source_map.rs         # ソースコード管理
│   │   │   │   └── mod.rs                # フロントエンドモジュール定義
│   │   │   ├── middleend/                # ミドルエンド
│   │   │   │   ├── ir/                   # 中間表現(IR)
│   │   │   │   │   ├── mod.rs            # モジュール定義
│   │   │   │   │   ├── builder.rs        # IR構築機能
│   │   │   │   │   ├── ssa.rs            # 静的単一代入形式
│   │   │   │   │   ├── validation.rs     # IR検証機能
│   │   │   │   │   └── visualization.rs  # IR可視化 (実装予定)
│   │   │   │   ├── optimization/         # 最適化
│   │   │   │   │   ├── mod.rs            # モジュール定義
│   │   │   │   │   ├── inlining.rs       # インライン展開
│   │   │   │   │   ├── constant_folding.rs # 定数畳み込み
│   │   │   │   │   ├── dead_code_elimination.rs # デッドコード削除
│   │   │   │   │   └── vectorization.rs  # ベクトル化
│   │   │   │   ├── analysis/             # 静的解析
│   │   │   │   │   ├── mod.rs            # モジュール定義
│   │   │   │   │   ├── dataflow.rs       # データフロー解析
│   │   │   │   │   └── lifetime.rs       # 寿命解析
│   │   │   │   └── mod.rs                # ミドルエンドモジュール定義
│   │   │   ├── backend/                  # バックエンド
│   │   │   │   ├── codegen.rs            # コード生成抽象レイヤー
│   │   │   │   ├── target.rs             # ターゲット情報
│   │   │   │   ├── optimization.rs       # バックエンド最適化
│   │   │   │   ├── llvm/                 # LLVM連携
│   │   │   │   │   ├── mod.rs            # モジュール定義
│   │   │   │   │   └── codegen.rs        # LLVM IRコード生成
│   │   │   │   ├── wasm/                 # WebAssembly連携 (将来実装)
│   │   │   │   │   ├── mod.rs            # モジュール定義
│   │   │   │   │   └── codegen.rs        # Wasm生成
│   │   │   │   ├── native/               # ネイティブバックエンド (将来実装)
│   │   │   │   │   ├── mod.rs            # モジュール定義
│   │   │   │   │   ├── x86_64.rs         # x86_64アーキテクチャ
│   │   │   │   │   ├── arm64.rs          # ARM64アーキテクチャ
│   │   │   │   │   └── risc_v.rs         # RISC-Vアーキテクチャ
│   │   │   │   ├── debug/                # デバッグ情報
│   │   │   │   │   ├── mod.rs            # モジュール定義
│   │   │   │   │   ├── dwarf.rs          # DWARF debug情報
│   │   │   │   │   └── source_map.rs     # ソースマッピング
│   │   │   │   └── mod.rs                # バックエンドモジュール定義
│   │   │   ├── driver/                   # コンパイラドライバ
│   │   │   │   ├── mod.rs                # モジュール定義
│   │   │   │   ├── compiler.rs           # コンパイルプロセス管理
│   │   │   │   ├── config.rs             # コンパイラ設定
│   │   │   │   └── options.rs            # コマンドラインオプション
│   │   │   └── lib.rs                    # クレートのルートファイル
│   │   ├── Cargo.toml                    # クレート設定
│   │   └── tests/                        # テスト
│   │       ├── lexer_tests.rs            # 字句解析器テスト
│   │       ├── parser_tests.rs           # 構文解析器テスト
│   │       ├── type_system_tests.rs      # 型システムテスト
│   │       ├── ir_tests.rs               # 中間表現テスト
│   │       └── codegen_tests.rs          # コード生成テスト
│   ├── swiftlight-stdlib/                # 標準ライブラリ (開発中)
│   │   ├── src/                          # ソースコード
│   │   │   ├── core/                     # コア機能
│   │   │   │   ├── mod.rs                # モジュール定義
│   │   │   │   ├── types.rs              # 基本型
│   │   │   │   ├── collections.rs        # コレクション型
│   │   │   │   ├── memory.rs             # メモリ管理
│   │   │   │   ├── error.rs              # エラー処理
│   │   │   │   └── iter.rs               # イテレータ
│   │   │   ├── std/                      # 標準機能
│   │   │   │   ├── mod.rs                # モジュール定義
│   │   │   │   ├── io.rs                 # 入出力
│   │   │   │   ├── fmt.rs                # フォーマット
│   │   │   │   ├── time.rs               # 時間関連
│   │   │   │   └── fs.rs                 # ファイルシステム
│   │   │   └── lib.rs                    # クレートのルートファイル
│   │   ├── Cargo.toml                    # クレート設定
│   │   └── tests/                        # テスト
│   │       ├── core_tests.rs             # コアライブラリテスト
│   │       └── std_tests.rs              # 標準ライブラリテスト
│   ├── swiftlight-cli/                   # コマンドラインインターフェース
│   │   ├── src/                          # ソースコード
│   │   │   ├── main.rs                   # エントリポイント
│   │   │   └── cli.rs                    # CLI定義
│   │   └── Cargo.toml                    # クレート設定
│   └── swiftlight-package-manager/       # パッケージマネージャ (将来実装)
│       ├── src/                          # ソースコード
│       │   ├── main.rs                   # エントリポイント
│       │   ├── registry.rs               # パッケージレジストリ
│       │   └── dependency.rs             # 依存関係解決
│       └── Cargo.toml                    # クレート設定
├── examples/                             # サンプルコード
│   ├── hello_world/                      # Hello World例
│   │   └── hello.swl                     # サンプルファイル
│   ├── types/                            # 型システム例
│   │   ├── generics.swl                  # ジェネリクス
│   │   └── traits.swl                    # トレイト
│   ├── ownership/                        # 所有権システム例
│   │   ├── borrowing.swl                 # 借用
│   │   └── lifetimes.swl                 # ライフタイム
│   └── advanced/                         # 高度な機能例
│       ├── metaprogramming.swl           # メタプログラミング (将来実装)
│       └── concurrency.swl               # 並行処理 (将来実装)
├── docs/                                 # ドキュメント
│   ├── language_reference/               # 言語リファレンス
│   │   ├── syntax.md                     # 構文定義
│   │   ├── types.md                      # 型システム
│   │   ├── memory_model.md               # メモリモデル
│   │   └── standard_library.md           # 標準ライブラリ
│   ├── tutorials/                        # チュートリアル
│   │   ├── getting_started.md            # 入門ガイド
│   │   └── advanced_features.md          # 高度な機能ガイド
│   └── internals/                        # 内部実装ドキュメント
│       ├── compiler_architecture.md      # コンパイラアーキテクチャ
│       └── contributor_guide.md          # 貢献ガイド
├── scripts/                              # スクリプト
│   ├── bootstrap.sh                      # 環境セットアップスクリプト
│   ├── build.sh                          # ビルドスクリプト
│   └── release.sh                        # リリーススクリプト
├── tests/                                # テスト (統合テスト、性能テスト等)
│   ├── integration/                      # 統合テスト
│   │   └── end_to_end_tests.rs           # E2Eテスト
│   └── performance/                      # 性能テスト
│       └── compiler_benchmarks.rs        # コンパイラベンチマーク
├── tools/                                # ツール (将来実装)
│   ├── formatter/                        # コードフォーマッタ
│   │   └── src/                          # ソースコード
│   ├── language-server/                  # 言語サーバー (LSP)
│   │   └── src/                          # ソースコード
│   └── analyzer/                         # 静的解析ツール
│       └── src/                          # ソースコード
├── LICENSE                               # ライセンス
├── README.md                             # プロジェクト説明
├── CHANGELOG.md                          # 変更履歴
├── TODO.md                               # TODOリスト
├── Cargo.toml                            # ワークスペース設定
└── .gitignore                            # Git無視ファイル設定
```

## 主要コンポーネントの説明

### 実装済みのコンポーネント

1. **フロントエンド**
   - 字句解析器と構文解析器の基本機能
   - 型チェックと型推論
   - ジェネリクスとトレイトシステム
   - 所有権・借用システム
   - 診断システム（エラーメッセージ生成）

2. **ミドルエンド**
   - 静的単一代入（SSA）形式のIR
   - 型情報を保持するIR設計
   - 基本的な最適化パス（インライン展開、定数畳み込み、デッドコード削除）
   - 制御フロー情報表現

3. **バックエンド**
   - LLVMバックエンド
   - デバッグ情報生成（DWARF）
   - ターゲットに依存しない抽象化レイヤー

4. **ビルドシステム**
   - 環境セットアップスクリプト
   - ビルドスクリプト
   - リリーススクリプト

5. **テストフレームワーク**
   - 単体テスト
   - 統合テスト
   - リグレッションテスト

### 実装予定のコンポーネント

1. **言語機能の拡張**
   - メタプログラミング機能
   - 依存型の拡張
   - パターンマッチングの実装
   - 型制約の詳細実装

2. **標準ライブラリ**
   - コア機能の拡充
   - IO、並行性、コレクションなどの基本機能

3. **開発ツール**
   - 言語サーバープロトコル（LSP）実装
   - コードフォーマッタ
   - パッケージマネージャ
   - リファクタリングツール

4. **最適化とパフォーマンス**
   - インクリメンタルコンパイル
   - パフォーマンス解析ツール
   - 最適化パイプラインの拡張
   - メモリ使用量最適化

5. **新しいターゲット**
   - WebAssembly生成
   - ネイティブコード直接生成
   - GPUコード生成（将来）