# マイルストーン3設計: 最小GUI ＋ 2クレートワークスペース化

## 背景・目的

`docs/spec.md`のマイルストーン3「`egui`で最小GUI（ドラッグ&ドロップ→解凍／圧縮）」に着手する。マイルストーン1（解凍側判定）・2（圧縮）は完了済み。

現状のCLIプロトタイプは単一クレート（`src/main.rs` + `src/compress.rs` + `src/encoding.rs`）。GUIを追加するにあたり、`egui`/`eframe`という比較的重いGUI依存を、Nautilus連携（マイルストーン4）で叩かれる軽量なCLIバイナリに巻き込みたくないこと、また本リポジトリの「Issue化の方針」（コア／GUI／OS統合層を別Issueに分割）と物理的な構造を対応させたいことから、2クレートのCargoワークスペースに再編する。

## アーキテクチャ

```
Easy-Archive/
├── Cargo.toml                # [workspace] members = ["crates/core", "crates/gui"]
└── crates/
    ├── core/                  # Issue #1「コア」に対応
    │   ├── Cargo.toml         # 依存: zip, encoding_rs
    │   └── src/
    │       ├── lib.rs         # pub mod compress; pub mod encoding; pub mod extract;
    │       └── main.rs        # 既存CLI(list/compress/extract サブコマンド)
    └── gui/                   # Issue #2「GUI」に対応
        ├── Cargo.toml         # 依存: egui, eframe, core(path依存)
        └── src/
            └── main.rs        # ドラッグ&ドロップGUI
```

- `crates/core`は現行の`src/compress.rs`・`src/encoding.rs`をほぼそのまま`lib.rs`配下の`pub mod`として移設し、新規`extract.rs`を追加する。CLI(`main.rs`)はこのlibに依存する形に変える（ロジックの実体はlib側、CLIは薄いラッパー）。
- `crates/gui`は`core`をpath依存し、`compress::compress`・`extract::extract`をそのまま呼び出す。文字コード判定・ZIP読み書きのロジックはGUI側で再実装しない。
- Zedのような数十〜百クレート規模へは踏み込まない。現状のコア部分はまだ数百行規模であり、それ以上の細分化は現時点では過剰投資（CLAUDE.mdのYAGNI方針）と判断。OS統合層（Issue #3）はシェルスクリプト/`.desktop`ファイルでありRustクレートではないため、3つ目のcrateは不要。

## 新機能: 解凍(`extract`)の実装

現状のCLIには`list`（一覧表示）と`compress`（圧縮）はあるが、**実際にファイルをディスクへ書き出す「解凍」がまだ存在しない**（マイルストーン1は判定ロジックの確認が目的で、書き出しは範囲外だった）。GUIの自動解凍にはこれが必須のため、`crates/core/src/extract.rs`として新規実装し、CLIにも`easy-archive extract <ZIPパス> <展開先ディレクトリ>`として先に追加する。

```rust
/// zip_pathの中身をdest_dirへ展開する。エントリ名はencoding::decode_entry_nameで
/// 判定・デコードする（listコマンドと同じロジック）。dest_dir配下に
/// ディレクトリ構造を再現し、必要な親ディレクトリを作成する。
/// 戻り値は展開したエントリ数。
pub fn extract(zip_path: &Path, dest_dir: &Path) -> Result<usize, Box<dyn Error>>
```

- `zip::ZipArchive`で読み込み、各エントリを`by_index_raw` + `HasZipMetadata::get_metadata().is_utf8` + `encoding::decode_entry_name`で名前をデコード（`list`コマンドと同一ロジックの再利用）
- デコードした名前が`/`で終わる、またはZIPエントリがディレクトリの場合はディレクトリを作成するのみ
- ファイルエントリは`dest_dir`からの相対パスとして親ディレクトリを`create_dir_all`した上で書き出す。内容バイト列は無変換でコピー（既存の中身無変換ルールを踏襲）
- `dest_dir`が既に存在する場合はエラーを返す（呼び出し側＝GUI/CLIの両方でこのチェックに任せる。上書きは行わない）

## GUI側の挙動仕様

- 1ウィンドウ、中央がドロップ領域のみ。ファイル選択ダイアログ（`rfd`）は今回追加しない。`egui`組み込みのドロップ検知（`ctx.input(|i| i.raw.dropped_files.clone())`）のみを使う
- アイドル時: 「ここにファイル/フォルダをドラッグ&ドロップしてください」を表示
- ドロップされた項目が2つ以上: 「一度に1つだけドロップしてください」とエラー表示のみ。何もしない
- 1項目の場合、種別で自動判別:
  - 拡張子が`.zip`（大文字小文字問わず）のファイル → **解凍**。展開先 = `{ZIPの親フォルダ}/{拡張子を除いたファイル名}/`。既に存在すればエラー表示
  - ディレクトリ、または`.zip`以外のファイル → **圧縮**。出力先 = `{親フォルダ}/{拡張子を除いた名前}.zip`。既に存在すればエラー表示
- 結果/エラーはウィンドウ内のステータス行にテキストで表示するのみ（ダイアログ・ポップアップなし）。直前の結果を1件分だけ保持し、次の操作で上書きする

## エラーハンドリング

`crates/core`側の関数群は既存スタイル（`Result<_, Box<dyn Error>>` + `format!`による日本語メッセージ）を踏襲する。GUI側は`Result`の`Err`をそのままステータス行の文字列として表示するのみで、新規エラー型・ダイアログ表示ライブラリは導入しない。

## テスト方針

- `crates/core/src/extract.rs`: `compress.rs`と同様、`std::env::temp_dir()`ベースの実ファイルを使った`#[cfg(test)]`。最低限「ZIPを展開すると元のディレクトリ構造・内容が復元される」「日本語(Shift-JIS/UTF-8混在)ファイル名が正しくデコードされて書き出される」の2ケースを検証する
- CLIの`extract`サブコマンドは、`compress`→`extract`を連続実行して元の内容が一致するラウンドトリップ確認を手動（`cargo run`）で行う
- GUI（`crates/gui`のイベントループ本体）は自動テストの対象外とする。ドラッグ&ドロップの実機能確認は手動で行う（GUIの自動テストは投資対効果が低いと判断）

## 移行時の注意

既存の`src/main.rs`・`src/compress.rs`・`src/encoding.rs`（ルート直下）は`crates/core/src/`へ移動する。ルートの`Cargo.toml`はワークスペース定義のみになり、既存の依存指定（`zip`のfeature設定等）は`crates/core/Cargo.toml`へ引き継ぐ。既存の`cargo test`が通っていた9件のテスト（`encoding.rs`5件、`compress.rs`4件）は移動後もそのまま通ることを確認する。

## 検証方法

1. `cargo build --workspace` — ワークスペース全体（core・gui両方）のビルドが通ることを確認
2. `cargo test --workspace` — 移設済みの既存9件 + 新規`extract.rs`テストが通ることを確認
3. `cargo run -p core -- extract <ZIPパス> <展開先>` でCLI版の解凍を確認
4. GUIバイナリ（`cargo run -p gui`）を実際に起動し、ZIPのドラッグ&ドロップで解凍、フォルダ/ファイルのドラッグ&ドロップで圧縮が動くことを手動確認
