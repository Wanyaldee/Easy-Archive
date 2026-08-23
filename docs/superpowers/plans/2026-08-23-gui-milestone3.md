# マイルストーン3(最小GUI + 2クレートワークスペース化) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 単一クレートのCLIプロトタイプを2クレートのCargoワークスペース(`crates/core`＝コア+CLI、`crates/gui`＝GUI)に再編し、`egui`/`eframe`によるドラッグ&ドロップ専用の最小GUIを追加する。前提として、まだ存在しない「解凍(ファイルをディスクへ書き出す)」機能を`crates/core`に新規実装する。

**Architecture:** `crates/core`が文字コード判定・ZIP読み書きの全ロジック(`compress`/`encoding`/`extract`モジュール)とCLI(`easy-archive`バイナリ)を持つ。`crates/gui`はこれをpath依存し、ドロップされたパスの種別によって`compress::compress`または`extract::extract`をそのまま呼び出すだけの薄いUI層にする。

**Tech Stack:** Rust (edition 2024) / `zip` 8.6.0 (deflateのみ) / `encoding_rs` / `egui` / `eframe` (最新版、`cargo add`で解決)

**Spec:** `docs/superpowers/specs/2026-08-23-gui-and-workspace-split-design.md`

## Global Constraints

- ドキュメント・コメント・コミットメッセージ・Issue/PRは日本語。コード識別子(関数名・変数名)は英語(Rust慣習)
- 文字コード判定は`encoding::decode_entry_name`(既存、変更しない)をそのまま使う。エントリ単位判定のロジックを再実装・改変しない
- 対応フォーマットはZIPのみ、対象OSはUbuntu系のみ
- 新規依存クレートは本当に必要な場合のみ追加(YAGNI)。ファイルダイアログ(`rfd`)は今回追加しない
- GUIはドラッグ&ドロップ専用(`egui`組み込みのドロップ検知のみ使用)
- 圧縮出力のファイル名エンコーディングは常にUTF-8(既存の`compress::compress`のまま、変更しない)
- ZIP内容のバイト列は常に無変換でコピーする
- ドロップが2つ以上の項目を含む場合は何もせずエラー表示のみ
- 解凍先・圧縮先が既に存在する場合は上書きせずエラーにする

---

## Task 1: リポジトリを2クレートのCargoワークスペースに再編する

**Files:**
- Modify: `Cargo.toml` (ルート) — ワークスペース定義のみに変更
- Create: `crates/core/Cargo.toml`
- Move: `src/main.rs` → `crates/core/src/main.rs`
- Move: `src/compress.rs` → `crates/core/src/compress.rs`
- Move: `src/encoding.rs` → `crates/core/src/encoding.rs`
- Create: `crates/core/src/lib.rs`
- Modify: `crates/core/src/main.rs` (モジュール宣言をlib依存に変更)
- Modify: `CLAUDE.md` (ディレクトリ構成図)

**Interfaces:**
- Produces: `easy_archive_core::compress::compress<W: Write + Seek>(writer: W, inputs: &[PathBuf]) -> Result<(W, usize), Box<dyn Error>>`(既存、変更なし)
- Produces: `easy_archive_core::encoding::decode_entry_name(raw: &[u8], utf8_flag_set: bool) -> (String, EncodingUsed)`(既存、変更なし)
- Produces: `easy_archive_core::encoding::EncodingUsed`(既存、変更なし)

- [ ] **Step 1: 既存ファイルの内容を確認する**

```bash
cat Cargo.toml
ls src/
```

現状は`Cargo.toml`(単一パッケージ`easy-archive`)、`src/main.rs`、`src/compress.rs`、`src/encoding.rs`の構成になっているはずです。

- [ ] **Step 2: ディレクトリを作成しファイルをgit mvで移動する**

```bash
mkdir -p crates/core/src
git mv src/main.rs crates/core/src/main.rs
git mv src/compress.rs crates/core/src/compress.rs
git mv src/encoding.rs crates/core/src/encoding.rs
rmdir src 2>/dev/null || true
```

- [ ] **Step 3: ルートの`Cargo.toml`をワークスペース定義に書き換える**

`Cargo.toml`の内容を以下に置き換える(既存の`[package]`/`[dependencies]`セクションは`crates/core/Cargo.toml`へ移す):

```toml
[workspace]
resolver = "2"
members = ["crates/core", "crates/gui"]
```

- [ ] **Step 4: `crates/core/Cargo.toml`を新規作成する**

```toml
[package]
name = "easy-archive-core"
version = "0.1.0"
edition = "2024"

[lib]
name = "easy_archive_core"
path = "src/lib.rs"

[[bin]]
name = "easy-archive"
path = "src/main.rs"

[dependencies]
encoding_rs = "0.8.35"
# 対象は日本の学校・自治体から届く一般的なZIP(deflate/無圧縮)のみ。
# bzip2/lzma/zstd/暗号化等はスコープ外なのでdeflateのみ有効化。
zip = { version = "8.6.0", default-features = false, features = ["deflate"] }
```

(バージョン番号はルートの旧`Cargo.toml`に既にあった値をそのまま使う。もし旧`Cargo.toml`の値がこれと異なる場合はそちらを優先する。)

- [ ] **Step 5: `crates/core/src/lib.rs`を新規作成する**

```rust
pub mod compress;
pub mod encoding;
pub mod extract;
```

(`extract`モジュールはTask 2で作成する。この時点ではまだ存在しないため、次のステップまで`cargo build`は失敗する。)

- [ ] **Step 6: 仮の`extract.rs`を作成してビルドを通す**

Task 2で本実装するが、ワークスペース全体のビルドを一旦通すため、空のプレースホルダを作る:

```bash
mkdir -p crates/core/src
```

`crates/core/src/extract.rs`を作成:

```rust
// Task 2で実装する。
```

- [ ] **Step 7: `crates/core/src/main.rs`のモジュール宣言をlib依存に書き換える**

`use zip::{HasZipMetadata, ZipArchive};`の下にあった以下の3行:

```rust
mod compress;
mod encoding;
use encoding::decode_entry_name;
```

を以下に置き換える:

```rust
use easy_archive_core::compress;
use easy_archive_core::encoding::decode_entry_name;
```

ファイル内の`compress::compress`呼び出し箇所はそのまま(パスは変わらない)。

- [ ] **Step 8: `crates/gui`をダミーで作成し、ワークスペースのビルドを通す**

Task 3で本実装するが、`members = ["crates/core", "crates/gui"]`と宣言した以上、`crates/gui`が存在しないと`cargo build --workspace`が失敗する。最小限のダミーを作る:

```bash
mkdir -p crates/gui/src
```

`crates/gui/Cargo.toml`:
```toml
[package]
name = "easy-archive-gui"
version = "0.1.0"
edition = "2024"

[dependencies]
```

`crates/gui/src/main.rs`:
```rust
fn main() {}
```

- [ ] **Step 9: ビルドとテストを実行し、既存の9件のテストが通ることを確認する**

```bash
cargo build --workspace
cargo test --workspace
```

Expected: ビルド成功。`easy-archive-core`の9テスト(既存の`encoding`5件・`compress`4件)が全て`ok`。

- [ ] **Step 10: `CLAUDE.md`のディレクトリ構成図を更新する**

`CLAUDE.md`の「ディレクトリ構成」セクションのコードブロックを以下に置き換える:

```
Easy-Archive/
├── CLAUDE.md
├── README.md
├── LICENSE                # MIT
├── Cargo.toml              # ワークスペース定義
├── crates/
│   ├── core/                # Issue #1「コア」— 判定ロジック・ZIP読み書き・CLI
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── main.rs       # CLI(list/compress/extract)
│   │       ├── compress.rs
│   │       ├── encoding.rs
│   │       └── extract.rs
│   └── gui/                 # Issue #2「GUI」
│       └── src/
│           └── main.rs
├── docs/
│   ├── spec.md             # 仕様書（確定している仕様のみ）
│   └── adr/                # 技術的決定の経緯・検証結果・議論（番号順）
│       ├── 0001-zip-crate-over-rc-zip.md
│       ├── 0002-drop-chardetng-strict-decode.md
│       └── 0003-windows11-zip-utf8-default.md
└── .claude/
    ├── settings.json       # プロジェクト共有設定（プラグイン・権限・言語）
    └── skills/
        └── easy-archive-core/
            └── SKILL.md    # コア設計方針の要点（判定粒度・クレート選定）
```

- [ ] **Step 11: コミットする**

```bash
git add -A
git commit -m "$(cat <<'EOF'
リポジトリを2クレートのCargoワークスペースに再編

crates/core(コア判定ロジック・ZIP読み書き・CLI)とcrates/gui(GUI、
未実装のダミー)に分割した。既存の9件のテストが移設後も全て通る
ことを確認済み。crates/gui本体の実装はTask 3以降で行う。
EOF
)"
```

---

## Task 2: `extract`モジュールをcoreに実装し、CLIにサブコマンドとして追加する

**Files:**
- Modify: `crates/core/src/extract.rs` (Task 1で作ったプレースホルダを本実装に置き換え)
- Modify: `crates/core/src/main.rs` (extractサブコマンド追加)

**Interfaces:**
- Consumes: `encoding::decode_entry_name(raw: &[u8], utf8_flag_set: bool) -> (String, EncodingUsed)`(Task 1で確認済み、既存)
- Consumes: `compress::compress<W: Write + Seek>(writer: W, inputs: &[PathBuf]) -> Result<(W, usize), Box<dyn Error>>`(テストで使用)
- Produces: `easy_archive_core::extract::extract(zip_path: &Path, dest_dir: &Path) -> Result<usize, Box<dyn Error>>` — Task 4(GUI)がこの関数を直接呼び出す

- [ ] **Step 1: 失敗するテストを書く**

`crates/core/src/extract.rs`の内容を以下に置き換える(実装部分はまだ`todo!()`):

```rust
//! ZIPをディスクへ展開するロジック。
//!
//! エントリ名のデコードは`encoding::decode_entry_name`を使う(CLIの`list`
//! コマンドと同じロジック)。中身のバイト列は無変換で書き出す。

use std::error::Error;
use std::path::Path;

/// zip_pathの中身をdest_dirへ展開する。dest_dirが既に存在する場合は
/// エラーを返す(上書きしない)。戻り値は展開したファイル数
/// (ディレクトリエントリを除く)。
pub fn extract(zip_path: &Path, dest_dir: &Path) -> Result<usize, Box<dyn Error>> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compress;
    use std::io::Read;
    use std::path::PathBuf;

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "easy-archive-test-extract-{tag}-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// テスト用のZIPを実ファイルとして作成する。extract()は実パスを
    /// 受け取る設計のため、Cursorではなく実ファイルを使う。
    fn make_test_zip(zip_path: &Path, inputs: &[PathBuf]) {
        let file = std::fs::File::create(zip_path).unwrap();
        compress::compress(file, inputs).unwrap();
    }

    #[test]
    fn extracts_nested_directory_structure() {
        let dir = temp_dir("nested");
        let source = dir.join("reports");
        std::fs::create_dir_all(source.join("sub")).unwrap();
        std::fs::write(source.join("a.txt"), b"a").unwrap();
        std::fs::write(source.join("sub").join("b.txt"), b"b").unwrap();

        let zip_path = dir.join("out.zip");
        make_test_zip(&zip_path, &[source]);

        let dest = dir.join("extracted");
        let count = extract(&zip_path, &dest).unwrap();
        assert_eq!(count, 2);

        let mut a = String::new();
        std::fs::File::open(dest.join("reports/a.txt"))
            .unwrap()
            .read_to_string(&mut a)
            .unwrap();
        assert_eq!(a, "a");

        let mut b = String::new();
        std::fs::File::open(dest.join("reports/sub/b.txt"))
            .unwrap()
            .read_to_string(&mut b)
            .unwrap();
        assert_eq!(b, "b");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn extracts_japanese_filename_correctly() {
        let dir = temp_dir("ja");
        let file_path = dir.join("日本語.txt");
        std::fs::write(&file_path, "内容".as_bytes()).unwrap();

        let zip_path = dir.join("out.zip");
        make_test_zip(&zip_path, &[file_path]);

        let dest = dir.join("extracted");
        let count = extract(&zip_path, &dest).unwrap();
        assert_eq!(count, 1);

        let mut content = String::new();
        std::fs::File::open(dest.join("日本語.txt"))
            .unwrap()
            .read_to_string(&mut content)
            .unwrap();
        assert_eq!(content, "内容");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn extract_fails_if_dest_dir_already_exists() {
        let dir = temp_dir("exists");
        let file_path = dir.join("a.txt");
        std::fs::write(&file_path, b"a").unwrap();

        let zip_path = dir.join("out.zip");
        make_test_zip(&zip_path, &[file_path]);

        let dest = dir.join("already_here");
        std::fs::create_dir_all(&dest).unwrap();

        let result = extract(&zip_path, &dest);
        assert!(result.is_err());

        std::fs::remove_dir_all(&dir).ok();
    }
}
```

- [ ] **Step 2: テストを実行して失敗することを確認する**

```bash
cargo test -p easy-archive-core extract:: 2>&1 | tail -30
```

Expected: `not yet implemented`(`todo!()`)によりFAIL。

- [ ] **Step 3: `extract`関数を実装する**

`crates/core/src/extract.rs`の`use`宣言と関数本体を以下に置き換える(テストモジュールはそのまま):

```rust
use std::error::Error;
use std::fs::{self, File};
use std::io::Read;
use std::path::Path;

use zip::{HasZipMetadata, ZipArchive};

use crate::encoding::decode_entry_name;

pub fn extract(zip_path: &Path, dest_dir: &Path) -> Result<usize, Box<dyn Error>> {
    if dest_dir.exists() {
        return Err(format!("展開先が既に存在します: {}", dest_dir.display()).into());
    }

    let file = File::open(zip_path)
        .map_err(|e| format!("ファイルを開けませんでした: {}: {e}", zip_path.display()))?;
    let mut archive = ZipArchive::new(file)
        .map_err(|e| format!("ZIPとして読み込めませんでした: {}: {e}", zip_path.display()))?;

    fs::create_dir_all(dest_dir)
        .map_err(|e| format!("展開先を作成できませんでした: {}: {e}", dest_dir.display()))?;

    let mut count = 0usize;
    for i in 0..archive.len() {
        let (name, is_dir) = {
            let entry = archive
                .by_index_raw(i)
                .map_err(|e| format!("エントリ {i} の読み込みに失敗しました: {e}"))?;
            let utf8_flag_set = entry.get_metadata().is_utf8;
            let (name, _used) = decode_entry_name(entry.name_raw(), utf8_flag_set);
            (name, entry.is_dir())
        };
        let out_path = dest_dir.join(&name);

        if is_dir {
            fs::create_dir_all(&out_path).map_err(|e| {
                format!("ディレクトリを作成できませんでした: {}: {e}", out_path.display())
            })?;
            continue;
        }

        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                format!("ディレクトリを作成できませんでした: {}: {e}", parent.display())
            })?;
        }

        let mut entry = archive
            .by_index(i)
            .map_err(|e| format!("エントリ {i} の読み込みに失敗しました: {e}"))?;
        let mut buf = Vec::new();
        entry
            .read_to_end(&mut buf)
            .map_err(|e| format!("読み込みに失敗しました: {}: {e}", out_path.display()))?;
        fs::write(&out_path, &buf)
            .map_err(|e| format!("書き込みに失敗しました: {}: {e}", out_path.display()))?;
        count += 1;
    }

    Ok(count)
}
```

- [ ] **Step 4: テストを実行して通ることを確認する**

```bash
cargo test -p easy-archive-core 2>&1 | tail -20
```

Expected: `extract::tests::`配下の3件を含む全テストが`ok`。

- [ ] **Step 5: CLIに`extract`サブコマンドを追加する**

`crates/core/src/main.rs`の`use easy_archive_core::compress;`の下に追加:

```rust
use easy_archive_core::extract;
```

`USAGE`定数を以下に置き換える:

```rust
const USAGE: &str = "使い方:\n  easy-archive list <ZIPファイルパス>\n  easy-archive compress <出力ZIPパス> <入力パス...>\n  easy-archive extract <ZIPファイルパス> <展開先ディレクトリ>";
```

`main`関数のmatch式に分岐を追加:

```rust
fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();

    match args.get(1).map(String::as_str) {
        Some("list") => run_list(&args[2..]),
        Some("compress") => run_compress(&args[2..]),
        Some("extract") => run_extract(&args[2..]),
        Some(other) => Err(format!("不明なサブコマンドです: {other}\n{USAGE}").into()),
        None => Err(USAGE.into()),
    }
}
```

`run_compress`関数の下に新規関数を追加:

```rust
fn run_extract(rest: &[String]) -> Result<(), Box<dyn Error>> {
    if rest.len() < 2 {
        return Err(USAGE.into());
    }
    let zip_path = PathBuf::from(&rest[0]);
    let dest_dir = PathBuf::from(&rest[1]);

    let count = extract::extract(&zip_path, &dest_dir)?;

    println!("{} に展開しました(エントリ数: {count})", dest_dir.display());

    Ok(())
}
```

- [ ] **Step 6: ビルドし、手動でラウンドトリップ確認する**

```bash
cargo build --workspace
mkdir -p /tmp/extract_check/src_dir
echo "test content" > /tmp/extract_check/src_dir/file.txt
cargo run -p easy-archive-core --bin easy-archive -- compress /tmp/extract_check/out.zip /tmp/extract_check/src_dir
cargo run -p easy-archive-core --bin easy-archive -- extract /tmp/extract_check/out.zip /tmp/extract_check/extracted
diff -r /tmp/extract_check/src_dir /tmp/extract_check/extracted/src_dir
```

Expected: `diff`が差分なし(終了コード0)で終わる。

- [ ] **Step 7: コミットする**

```bash
git add crates/core/src/extract.rs crates/core/src/main.rs
git commit -m "$(cat <<'EOF'
extractモジュールを実装しCLIにサブコマンド追加

crates/core/src/extract.rsを実装。ZIPの各エントリをencoding::
decode_entry_nameで名前判定しつつディスクへ書き出す。中身のバイト列は
無変換。展開先が既に存在する場合はエラーにする(上書きしない)。

CLIにeasy-archive extract <ZIP> <展開先>サブコマンドを追加。
新規3テスト(ネストしたディレクトリ構造の復元、日本語ファイル名、
展開先が既に存在する場合のエラー)で検証。手動での圧縮→解凍
ラウンドトリップでも内容の一致を確認済み。
EOF
)"
```

---

## Task 3: `crates/gui`クレートを本実装し、最小ウィンドウを起動できるようにする

**Files:**
- Modify: `crates/gui/Cargo.toml` (Task 1のダミーから依存追加)
- Modify: `crates/gui/src/main.rs` (Task 1のダミーから最小eframeウィンドウへ)

**Interfaces:**
- Consumes: なし(このタスクではまだcoreのロジックを呼ばない。ウィンドウが起動することだけを確認する)
- Produces: `crates/gui`の実行可能バイナリ(パッケージ名`easy-archive-gui`)

- [ ] **Step 1: `egui`/`eframe`を追加する**

```bash
cd crates/gui
cargo add eframe egui
cargo add easy-archive-core --path ../core
cd ../..
```

- [ ] **Step 2: 最小のeframeアプリを書く**

`crates/gui/src/main.rs`を以下に置き換える:

```rust
fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "Easy Archive",
        options,
        Box::new(|_cc| Ok(Box::new(App::default()))),
    )
}

#[derive(Default)]
struct App {
    status: String,
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.centered_and_justified(|ui| {
                let text = if self.status.is_empty() {
                    "ここにファイル/フォルダをドラッグ&ドロップしてください"
                } else {
                    &self.status
                };
                ui.label(text);
            });
        });
    }
}
```

**注記:** `eframe::run_native`のクロージャの戻り値の型(`Ok(Box::new(...))`か`Box::new(...)`か)は`cargo add`で実際にインストールされた`eframe`のバージョンによって異なる場合がある。次のステップでコンパイルエラーが出た場合は、エラーメッセージに従って型を合わせる(これはこのタスクの想定内の作業であり、設計のやり直しではない)。

- [ ] **Step 3: ビルドする(コンパイルエラーが出たら実際のAPIに合わせて修正する)**

```bash
cargo build -p easy-archive-gui 2>&1 | tail -60
```

Linux(Ubuntu)でのビルドには、`eframe`/`winit`が必要とするシステムのウィンドウ関連開発ヘッダが必要な場合がある。ビルド時に`libxkbcommon`や`libxcb`関連のリンクエラーが出た場合は以下を実行してから再試行する:

```bash
sudo apt-get update && sudo apt-get install -y libgtk-3-dev libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev libxkbcommon-dev
```

Expected: `cargo build -p easy-archive-gui`が成功する。

- [ ] **Step 4: 手動で起動確認する**

```bash
cargo run -p easy-archive-gui
```

Expected: 「ここにファイル/フォルダをドラッグ&ドロップしてください」という文字だけが中央に表示されたウィンドウが開く。目視確認後、ウィンドウを閉じる。

(このステップは自動テストの対象外。GUIウィンドウの起動自体を確認する手動ステップ。)

- [ ] **Step 5: コミットする**

```bash
git add crates/gui/Cargo.toml crates/gui/src/main.rs Cargo.lock
git commit -m "$(cat <<'EOF'
crates/guiに最小eframeウィンドウを実装

egui/eframeを追加し、中央にステータステキストを表示するだけの
最小ウィンドウを起動できることを確認した。ドラッグ&ドロップの
実処理はTask 4で実装する。
EOF
)"
```

---

## Task 4: ドラッグ&ドロップの実処理を実装する(解凍/圧縮の自動判別)

**Files:**
- Modify: `crates/gui/src/main.rs`

**Interfaces:**
- Consumes: `easy_archive_core::extract::extract(zip_path: &Path, dest_dir: &Path) -> Result<usize, Box<dyn Error>>`(Task 2)
- Consumes: `easy_archive_core::compress::compress<W: Write + Seek>(writer: W, inputs: &[PathBuf]) -> Result<(W, usize), Box<dyn Error>>`(既存)

- [ ] **Step 1: ドロップ判定・実行ロジックを追加する**

`crates/gui/src/main.rs`を以下に置き換える:

```rust
use std::path::{Path, PathBuf};

use easy_archive_core::{compress, extract};

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "Easy Archive",
        options,
        Box::new(|_cc| Ok(Box::new(App::default()))),
    )
}

#[derive(Default)]
struct App {
    status: String,
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let dropped: Vec<PathBuf> = ctx.input(|i| {
            i.raw
                .dropped_files
                .iter()
                .filter_map(|f| f.path.clone())
                .collect()
        });

        if !dropped.is_empty() {
            self.status = handle_drop(&dropped);
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.centered_and_justified(|ui| {
                let text = if self.status.is_empty() {
                    "ここにファイル/フォルダをドラッグ&ドロップしてください"
                } else {
                    &self.status
                };
                ui.label(text);
            });
        });
    }
}

/// ドロップされたパスの一覧を受け取り、解凍/圧縮のどちらかを実行して
/// 結果メッセージを返す。2件以上ドロップされた場合は何もせずエラーの
/// メッセージだけを返す。
fn handle_drop(paths: &[PathBuf]) -> String {
    if paths.len() != 1 {
        return "一度に1つだけドロップしてください".to_string();
    }
    let path = &paths[0];

    let is_zip_file = path.is_file()
        && path
            .extension()
            .map(|e| e.eq_ignore_ascii_case("zip"))
            .unwrap_or(false);

    if is_zip_file {
        do_extract(path)
    } else if path.is_dir() || path.is_file() {
        do_compress(path)
    } else {
        format!("入力パスが見つかりません: {}", path.display())
    }
}

fn do_extract(zip_path: &Path) -> String {
    let (Some(parent), Some(stem)) = (zip_path.parent(), zip_path.file_stem().and_then(|s| s.to_str())) else {
        return format!("パスを解析できませんでした: {}", zip_path.display());
    };
    let dest_dir = parent.join(stem);

    match extract::extract(zip_path, &dest_dir) {
        Ok(count) => format!("{} に展開しました(エントリ数: {count})", dest_dir.display()),
        Err(e) => format!("エラー: {e}"),
    }
}

fn do_compress(source: &Path) -> String {
    let (Some(parent), Some(stem)) = (source.parent(), source.file_stem().and_then(|s| s.to_str())) else {
        return format!("パスを解析できませんでした: {}", source.display());
    };
    let output_path = parent.join(format!("{stem}.zip"));

    if output_path.exists() {
        return format!("既に存在します: {}", output_path.display());
    }

    let file = match std::fs::File::create(&output_path) {
        Ok(f) => f,
        Err(e) => {
            return format!(
                "出力ファイルを作成できませんでした: {}: {e}",
                output_path.display()
            )
        }
    };

    match compress::compress(file, &[source.to_path_buf()]) {
        Ok((_, count)) => format!("{} を作成しました(エントリ数: {count})", output_path.display()),
        Err(e) => format!("エラー: {e}"),
    }
}
```

- [ ] **Step 2: ビルドする**

```bash
cargo build -p easy-archive-gui 2>&1 | tail -60
```

Expected: ビルド成功。

- [ ] **Step 3: 手動で解凍・圧縮それぞれを確認する**

```bash
cargo run -p easy-archive-gui &
```

ウィンドウが起動したら、以下を手動で行う:

1. 日本語ファイル名を含むフォルダ(例: 前タスクで作った`/tmp/extract_check/src_dir`のようなもの)をウィンドウにドラッグ&ドロップする → 同じ場所に`src_dir.zip`が作成され、ステータス行に「作成しました」と表示されることを確認する
2. 作成された`src_dir.zip`を再度ドロップする → 同じ場所に`src_dir/`フォルダが展開され、ステータス行に「展開しました」と表示されることを確認する(既に`src_dir`フォルダが存在する場合は先に別名にリネームしておくか別ディレクトリで試す)
3. 2つ以上のファイルを同時にドロップする → 「一度に1つだけドロップしてください」と表示されることを確認する
4. 既に同名の出力(ZIPまたは展開先フォルダ)がある状態で同じものを再度ドロップする → 「既に存在します」系のエラーメッセージが表示されることを確認する

確認後、ウィンドウを閉じる。

(この手動確認はコミットの必須条件。GUIの自動テストは対象外という設計判断のため、ここでの目視確認が唯一の検証手段。)

- [ ] **Step 4: コミットする**

```bash
git add crates/gui/src/main.rs
git commit -m "$(cat <<'EOF'
GUIにドラッグ&ドロップの実処理を実装

ドロップされたパスが.zipファイルなら展開先=同じ場所の拡張子を
除いた同名フォルダへ解凍、ディレクトリまたは.zip以外のファイルなら
出力先=同じ場所の拡張子を除いた同名.zipへ圧縮、と自動判別する。
2件以上ドロップされた場合や出力先が既に存在する場合はエラー
メッセージを表示するのみで、上書き・部分実行はしない。

crates/core(extract::extract, compress::compress)をそのまま呼ぶ
薄いUI層。手動でのドロップ操作4パターン(圧縮/解凍/複数ドロップ/
出力先重複)を目視確認済み。
EOF
)"
```

---

## Task 5: CIをワークスペース対応にし、仕様書を更新する

**Files:**
- Modify: `.github/workflows/ci.yml`
- Modify: `docs/spec.md` (マイルストーン3完了の記録)

**Interfaces:**
- Consumes: なし(ドキュメント・CI設定のみの変更)

- [ ] **Step 1: CIワークフローをワークスペース全体のビルド/テストに変更する**

`.github/workflows/ci.yml`を以下に置き換える:

```yaml
name: CI

on:
  pull_request:
  push:
    branches: [main]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Install GUI build dependencies
        run: sudo apt-get update && sudo apt-get install -y libgtk-3-dev libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev libxkbcommon-dev
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo build --workspace --verbose
      - run: cargo test --workspace --verbose
```

- [ ] **Step 2: `docs/spec.md`のマイルストーン3を完了として更新する**

`docs/spec.md`の該当行:

```
3. `egui`で最小GUI（ドラッグ&ドロップ→解凍／圧縮）
```

を以下に置き換える:

```
3. `egui`で最小GUI（ドラッグ&ドロップ→解凍／圧縮） — 完了。`crates/gui`。単一のドロップ領域でZIPファイル(.zip拡張子)なら解凍、それ以外(ディレクトリ/単一ファイル)なら圧縮を自動判別する。詳細設計は[`docs/superpowers/specs/2026-08-23-gui-and-workspace-split-design.md`](./superpowers/specs/2026-08-23-gui-and-workspace-split-design.md)を参照。この変更に伴い`crates/core`(コア+CLI)・`crates/gui`(GUI)の2クレートワークスペースへ再編済み
```

- [ ] **Step 3: コミットしてpushする**

```bash
git add .github/workflows/ci.yml docs/spec.md
git commit -m "$(cat <<'EOF'
CIをワークスペース対応にし、マイルストーン3完了を記録

GUIクレートのビルドに必要なシステムパッケージ(libgtk-3-dev等)を
CIに追加。cargo build/testを--workspaceでcrates/core・crates/gui
両方に対して実行するよう変更。docs/spec.mdのマイルストーン3を
完了として記録した。
EOF
)"
git push -u origin feature/gui-milestone3
```

- [ ] **Step 4: PRを作成しCIが通ることを確認する**

```bash
gh pr create --repo Wanyaldee/Easy-Archive --base main --head feature/gui-milestone3 \
  --title "マイルストーン3: 最小GUI + 2クレートワークスペース化" \
  --body "$(cat <<'EOF'
## 概要
- crates/core(コア+CLI)・crates/gui(GUI)の2クレートワークスペースへ再編
- crates/coreに解凍(extract)機能を新規実装、CLIにもextractサブコマンド追加
- crates/guiにegui/eframeによる最小GUI(ドラッグ&ドロップ専用、rfd不使用)を実装

## 設計
docs/superpowers/specs/2026-08-23-gui-and-workspace-split-design.md 参照

## 検証
- cargo test --workspace: 全テスト成功(既存9件 + extract新規3件)
- 手動でのドラッグ&ドロップ確認: 圧縮/解凍/複数ドロップ時エラー/出力先重複時エラーの4パターン確認済み
EOF
)"
```

作成後、`gh pr checks <PR番号> --repo Wanyaldee/Easy-Archive`でCIが`SUCCESS`になることを確認する。失敗した場合はログを見て原因(多くはGUI依存のシステムパッケージ不足)を特定し修正する。

---

## Self-Review (このプランを書いた本人による確認用メモ)

- **spec.mdカバレッジ**: アーキテクチャ(Task1)、extract実装(Task2)、GUI最小実装(Task3)、ドロップ挙動仕様(Task4)、テスト方針(Task2/Task4)、検証方法(Task5)の全項目に対応するタスクがある
- **型/シグネチャの一貫性**: `extract::extract(zip_path: &Path, dest_dir: &Path) -> Result<usize, Box<dyn Error>>`はTask2で定義し、Task4の`do_extract`から同一シグネチャで呼び出している。`compress::compress`は既存シグネチャをそのまま使用し変更していない
- **プレースホルダ**: Task1 Step6の`extract.rs`とTask3 Step5コミット対象のCargo.lockはワークフロー上必要な一時的措置であり、それぞれTask2・Task1で実体化される。それ以外に「TODO」「後で実装」等の記述はない
