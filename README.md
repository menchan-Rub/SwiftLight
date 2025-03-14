# SwiftLight プログラミング言語

SwiftLightは、メモリ安全性、並行処理、高性能を重視した現代的なプログラミング言語です。

## 特徴

- **メモリ安全性**: 所有権システムによるコンパイル時のメモリ安全性保証
- **モダンな構文**: クリーンで読みやすく、表現力豊かな構文
- **高速な実行速度**: LLVMバックエンドによる最適化されたネイティブコード生成
- **強力な型システム**: 静的型付けと型推論によるコンパイル時の安全性と表現力
- **クロスプラットフォーム**: 主要なプラットフォームで動作
- **並行処理**: 軽量プロセスと組み込まれた並行プリミティブ

## インストール

まだ開発初期段階のため、公式のインストーラーは提供していません。ソースからビルドするには：

```bash
git clone https://github.com/menchan-Rub/swiftlight.git
cd swiftlight
cargo build --release
```

ビルド後、`target/release/swiftlight` が実行可能ファイルとして生成されます。

## 使い方

### Hello, World!

```swift
func main() {
    println("Hello, SwiftLight World!");
}
```

### コンパイルと実行

```bash
swiftlight build hello.swl
./hello
```

## 言語の特徴

### 変数と定数

```swift
let x = 10;        // 不変変数
let mut y = 20;    // 可変変数
const PI = 3.14;   // コンパイル時定数
```

### 関数

```swift
func add(a: Int, b: Int) -> Int {
    return a + b;
}

// 型推論
func subtract(a: Int, b: Int) {
    return a - b;  // 戻り値の型はIntと推論される
}
```

### 構造体

```swift
struct Point {
    x: Int,
    y: Int,
}

impl Point {
    func new(x: Int, y: Int) -> Point {
        return Point { x, y };
    }
    
    func distance(self, other: Point) -> Float {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        return sqrt((dx * dx + dy * dy) as Float);
    }
}
```

### 列挙型

```swift
enum Result<T, E> {
    Ok(T),
    Err(E),
}

func divide(a: Int, b: Int) -> Result<Int, String> {
    if b == 0 {
        return Result::Err("ゼロ除算エラー");
    }
    return Result::Ok(a / b);
}
```

### トレイト（インターフェース）

```swift
trait Printable {
    func to_string(self) -> String;
}

impl Printable for Point {
    func to_string(self) -> String {
        return "Point({}, {})".format(self.x, self.y);
    }
}
```

## 開発状況

SwiftLightは現在アルファ段階にあり、活発に開発が進行中です。以下の機能が実装中または計画中です：

- [x] 字句解析器
- [x] 構文解析器
- [ ] 型チェッカー（実装中）
- [ ] コード生成（実装中）
- [ ] パッケージマネージャ（計画中）
- [ ] 標準ライブラリ（実装中）
- [ ] IDE統合（計画中）

## コントリビューション

SwiftLightへの貢献を歓迎します！コードの貢献、バグレポート、機能リクエストなど、さまざまな形で参加できます。詳細は[CONTRIBUTING.md](CONTRIBUTING.md)をご覧ください。

## ライセンス

SwiftLightはMITライセンスとApache License 2.0のデュアルライセンスで提供されています。詳細は[LICENSE](LICENSE)ファイルをご確認ください。
