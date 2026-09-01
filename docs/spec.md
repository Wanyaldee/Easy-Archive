# Easy Archive 仕様書

技術的な決定の背景・検証結果・議論は [`docs/adr/`](./adr/) を参照。このファイルは現時点で確定している仕様のみを記載する。

## 背景・目的

- ZorinOS導入済みの小学校PCで、自治体等から届くZIPファイル（中身のOfficeファイル名がShift-JIS）を通常の`unzip`で展開すると文字化けする。
- 現状はCLIでの回避策（`unzip -O cp932`等）を教えるしかなく、非エンジニアの教員には運用負荷が高い。
- Linux向けGUIアーカイバ（PeaZip, Xarchiver, Ark等）は文字コード自動判定機能を持たず、文字コードを正しく扱えるツール（unar -e, 7z+iconv等）はCLI限定という空白地帯がある。
- → GUI＋文字コード自動判定を両立したOSSツールをRustで作る。

## スコープ方針

- **対応フォーマットは当面ZIPのみ**。tar.gz等の他フォーマットは後回し。
- **解凍だけでなく圧縮機能も最低限備える**（相手にZIPを送り返す/自分で固めるケースがあるため）。
  - 圧縮時は既定でファイル名エンコーディングをUTF-8（汎用フラグbit11セット）にする。Shift-JIS等での圧縮出力は当面サポート対象外でよい（＝文字コード判定・変換ロジックが必要なのは解凍側のみ）。
- **第一段階はUbuntu系（Zorin OS含む）に限定**。他OS対応は後回し。
- コア部分（展開・圧縮エンジン・GUI）はOS非依存設計にしておき、将来のクロスプラットフォーム化に備える。
- 「文字コード解析」はファイル名/パスのデコードのみが対象。Office/バイナリファイルの中身はバイト列のまま書き出す/読み込むだけでよい（中身の変換は対象外）。
  - 検証済み: ZIP側のファイル名をCP932で正しく展開できていれば、内部のOfficeファイル（.doc/.xls/.ppt含む）自体は文字化けせず開ける。旧形式Office内部はOLE2/CFB構造でコードページ情報を保持しているため、アプリ側（LibreOffice等）が正しく解釈できる。よってZIP側の中身をバイト列として無変換で取り出す方針で問題ない。

## 技術スタック

| 用途 | 採用 | 決定の経緯 |
|---|---|---|
| ZIP読み書き | `zip`クレート | [ADR 0001](./adr/0001-zip-crate-over-rc-zip.md) |
| エンコーディング判定 | `encoding_rs`でのShift-JISストリクトデコード試行（エントリ単位、失敗時CP437フォールバック） | [ADR 0001](./adr/0001-zip-crate-over-rc-zip.md), [ADR 0002](./adr/0002-drop-chardetng-strict-decode.md) |
| エンコーディング変換 | `encoding_rs`（CP437は非公開APIのため独自テーブルを`src/encoding.rs`に実装） | [ADR 0002](./adr/0002-drop-chardetng-strict-decode.md) |
| GUI | `egui`/`eframe`（webview非依存、静的バイナリで完結） | — |
| ファイルダイアログ | `rfd` | — |
| 配布形式 | `.deb`(`cargo-deb`) | [ADR 0007](./adr/0007-deb-packaging.md) |
| シェル統合 | Nautilusスクリプト、`.desktop`（MIME: `application/zip`） | — |

## 当面のマイルストーン

1. CLIプロトタイプでエンコーディング判定ロジックの動作確認（解凍側、エントリ単位） — 完了。`crates/core/src/main.rs` + `crates/core/src/encoding.rs`。`cargo test`で判定ロジックを検証済み。1つのZIP内にShift-JIS(半角カナ含む/含まない)とUTF-8フラグ付きエントリを混在させた実データでも正しく判定できることを確認済み（rc-zipが失敗するケース）。実データ検証時の付随的な発見は[ADR 0003](./adr/0003-windows11-zip-utf8-default.md)を参照
2. CLIプロトタイプでZIP圧縮（UTF-8ファイル名）の動作確認 — 完了。`crates/core/src/compress.rs`（`easy-archive compress <出力ZIP> <入力パス...>`）。ファイル/ディレクトリを再帰的に追加し、日本語ファイル名は自動でUTF-8フラグが立つことを`cargo test`と実データ（`unzip`/Pythonでの独立検証）で確認済み
3. `egui`で最小GUI（ドラッグ&ドロップ→解凍／圧縮） — 完了。`crates/gui`。単一のドロップ領域でZIPファイル(.zip拡張子)なら解凍、それ以外(ディレクトリ/単一ファイル)なら圧縮を自動判別する。詳細設計は[`docs/superpowers/specs/2026-08-23-gui-and-workspace-split-design.md`](./superpowers/specs/2026-08-23-gui-and-workspace-split-design.md)を参照。この変更に伴い`crates/core`(コア+CLI)・`crates/gui`(GUI)の2クレートワークスペースへ再編済み。ウィンドウ描画とNoto Sans JPによる日本語テキスト表示は実機での目視確認済み(文字化けなし)。ドラッグ&ドロップ操作は、WSL開発環境ではwinitのWaylandバックエンド未実装が原因で機能しなかったが、X11強制で回避し実機Zorin OSで解凍・圧縮双方の動作を確認済み（詳細は[ADR 0004](./adr/0004-winit-wayland-dnd-x11-force.md)を参照）。本ツールで圧縮したZIPを本ツールで再度展開すると、ZIP側が既にトップレベルフォルダを含むため`foo/foo/...`のように一段深く展開される（第三者から届く一般的なZIPには影響しない既知の挙動）。
4. 主要ファイルマネージャーへの「ここに解凍」「ここを圧縮」右クリックメニュー連携 — 実装完了、実機検証待ち。対象OS方針（Ubuntu系全般）に合わせ、Nautilus(GNOME)・Nemo(Cinnamon)・Thunar(XFCE)・Dolphin(KDE)・PCManFM-Qt(LXQt)の5系統に対応する。各DEの統合方式の調査結果・技術的決定は[ADR 0005](./adr/0005-file-manager-integration-mechanisms.md)を参照。全DE共通で`easy-archive auto <パス>`(`crates/core/src/auto.rs`、GUIのドロップ処理と共通のロジック)に処理を委譲するため、メニュー表示条件の精度に関わらず常に正しく動作する。`easy-archive install-integration [--dry-run]`/`uninstall-integration`で設置・削除でき、Thunarの`uca.xml`は既存のユーザー独自カスタムアクションを壊さないよう`quick-xml`で冪等にマージ/削除する(`crates/core/src/integration/`)。Nautilusは開発環境(WSL)でGUI操作(右クリック→「Easy Archive」→スクリプトクリック)による解凍・圧縮の目視確認済み。Nemo/Thunar/Dolphin/PCManFM-Qtは実機(Zorin OS Lite、Kubuntu、Lubuntu、Linux Mint等)での確認がまだ（詳細はADR 0005を参照）。PCManFM(GTK/classic、LXDE)は解凍表示がビルド時に焼き込まれた外部アーカイバ検出ロジックに依存する別系統の仕組みで実現性が不確実なため、今回のマイルストーンでは対象外とし将来対応とする
5. `.deb`パッケージング — 実装完了、実機検証待ち（`packaging/build-deb.sh`でのローカルビルドと`dpkg-deb --info`/`--contents`による内容確認までは済んでいるが、実機（Zorin OS等）での`.deb`インストール→GUI起動→設置ボタンの一連の流れは未確認。詳細は[ADR 0007](./adr/0007-deb-packaging.md)の「既知の限界」を参照）。`cargo-deb`(`crates/gui/Cargo.toml`の`[package.metadata.deb]`)で`easy-archive`・`easy-archive-gui`両バイナリ・`.desktop`・アイコンを1つの`.deb`にまとめた。ファイルマネージャー統合の自動設置はpostinst(root権限・`$HOME`不明のため不可能)ではなく、GUI起動時に表示する設置ボタンから行う設計にした(詳細は[ADR 0007](./adr/0007-deb-packaging.md)を参照)。`packaging/build-deb.sh`でRust未導入環境でも確認の上で自動セットアップしてビルドできる。AppImageは当面の対象外(スコープ外、将来必要になれば別途検討)

## Issue化の方針

「コア（判定ロジックのラッパー）」「GUI」「OS統合層」を別Issueに分割し、将来のクロスプラットフォーム化でスコープがぶれないようにする。
