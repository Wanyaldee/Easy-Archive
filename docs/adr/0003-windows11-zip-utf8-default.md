# 0003. Windows 11のネイティブZIP圧縮はUTF-8既定になっている（設計変更なし、記録のみ）

## ステータス

Accepted（Informational — 設計・実装への変更は無し）

## 背景

マイルストーン1完了後、実際のZIPファイルでCLIプロトタイプを検証した。ユーザーがShift-JISファイル名を含むWord/PDFファイルをWindows上で作成し、以下3パターンでZIP化して判定結果を確認した。

1. サードパーティソフト（"Files"アプリ等）で圧縮
2. 別のサードパーティソフトで圧縮
3. Windows純正エクスプローラの「圧縮(zip形式)フォルダーに送る」で圧縮

**3パターンとも、ファイル名エントリはUTF-8フラグ(GPB bit11)が立っていた。** つまり本プロジェクトが解決対象とする「フラグなしShift-JISファイル名による文字化け」を、この検証では一度も再現できなかった。

### 原因調査

Pythonの`zipfile`で各ZIPのメタデータを独立検証したところ、3パターンとも`flag_bits`に加えて`create_system=3`（Unix）という共通の特徴を持っていた。従来Windows純正の圧縮（`zipfldr.dll`）は`create_system=0`（MS-DOS/FAT）でファイル名をローカルコードページ（日本語環境ならCP932/Shift-JIS）のまま、UTF-8フラグを立てずに書き込むはずである。

Web調査の結果、Microsoftは2023年5月のBuild 2023で、Windows 11のネイティブ圧縮機能に**libarchive**（クロスプラットフォームのOSSライブラリ）を統合し、7-Zip/RAR/tar/gz等の追加フォーマット対応を発表していたことが判明した（[WinBuzzer](https://winbuzzer.com/2023/05/24/windows-11-is-getting-native-rar-7-zip-tar-gz-file-compression-support-xcxwbn/)、[Born's Tech and Windows World](https://borncity.com/win/2023/05/26/windows-11-gets-support-7-zip-rar-and-gz-as-archive-formats-in-addition-to-zip/)）。この刷新以降、Windows 11の「純正」ZIP圧縮も内部的にはlibarchiveベースのエンジンを使うようになっており、UTF-8既定・Unix風メタデータで書き出されると考えられる（従来のWindows専用`zipfldr.dll`は使われなくなった可能性が高い）。

## 決定: 設計変更は不要。前提の再確認として記録する

- 本プロジェクトの発端（[spec.md](../spec.md)「背景・目的」参照）は「**自治体等から届く**ZIP」の文字化けである。自治体・学校側のシステムは必ずしも最新のWindows 11ではなく、旧Windows・専用の電子申請システム・複合機のスキャン機能・古いバックオフィスソフト等、Shift-JISのままフラグなしで書き出す環境が今も現役である可能性が高い。加えて、Windows 11のこの変更より前に作成され既に流通している過去のZIPファイルも対象のままである。
- よって「最新Windows 11で今作ったZIPは(libarchive経由のため)心配ない」ことが分かったのは良いニュースだが、本ツールの必要性・スコープを変える情報ではない。設計・実装（`src/encoding.rs`の`decode_entry_name`）はそのまま維持する。

## 影響

- 判定ロジック自体は`cargo test`（5件）と、手作りしたZIPバイナリ（Shift-JIS非フラグ + UTF-8フラグを1アーカイブ内に混在させたもの。フラグなしのlegacyな挙動を意図的に再現）で検証済み。
- 実データでフラグなしShift-JIS ZIPを追加で確認したい場合は、Windows 10機での作成か、7-Zip等で「Unicode UTF-8を使う」を明示的にオフにして作成する必要がある。
