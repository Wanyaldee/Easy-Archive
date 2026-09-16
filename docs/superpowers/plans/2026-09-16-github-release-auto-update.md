# GitHub Release連携アップデート機構 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.
>
> **エージェント割り当てについて:** Task 1（テスト作成）とTask 2（実装）は必ず別々のエージェントに割り当てること（ユーザー指示）。Task 1のエージェントはTask 2のコード（実装本体）を書いてはならず、スタブ（`unimplemented!()`）とテストのみを書く。Task 3・4はユニットテストを伴わない実装のみのタスクのため、担当エージェントの分離は不要。

**Goal:** GUIにGitHub Release連携のアップデート機構を追加し、起動時に新バージョンを検知して「アップデートする」ボタン一つで`.deb`の取得・検証・インストールまで完結させる。

**Architecture:** GUI非依存の更新ロジック（バージョン比較・GitHub APIレスポンス解析・チェックサム検証・ダウンロード・`pkexec`経由のインストール実行）を`crates/core/src/update.rs`に実装し、`crates/gui/src/main.rs`が起動時バックグラウンドチェックとボタン操作からそれを呼び出す。CIに新規ワークフローを追加し、タグpush時に`.deb`とチェックサムファイルをGitHub Releaseへ自動アップロードする。

**Tech Stack:** Rust（`ureq`でHTTPS通信、`serde`/`serde_json`でJSON解析、`sha2`でSHA-256検証）、`pkexec`+`apt-get`（インストール実行）、GitHub Actions + `gh` CLI（リリース自動化）

**Spec:** `docs/superpowers/specs/2026-09-16-github-release-auto-update-design.md`

## Global Constraints

- 対象パッケージマネージャーはAPTのみ。DNF（Fedora系）・Pacman（Arch系）は対象外（将来の拡張）
- CLIサブコマンドは追加しない。更新機能はGUI専用
- 独自APTリポジトリは構築しない。GitHub Releasesの`.deb`アセットをそのまま使う
- `semver`クレートは追加しない。バージョン比較は`X.Y.Z`を手書きでタプル比較する
- カスタムpolkitアクション定義は追加しない。標準の`pkexec`認証ダイアログを使う
- 新規依存クレート（`ureq`/`serde`/`serde_json`/`sha2`）は`crates/core`にのみ追加する。`crates/gui`には追加しない
- エラーメッセージは日本語
- 実際のHTTP通信・`pkexec`実行・PolicyKit認証ダイアログはユニットテスト対象外とする（実機検証で担保。ADR 0004/0005/0007と同じ方針）

---

## Task 1: crates/core — update.rsのインターフェースとユニットテスト作成

**担当エージェント:** テスト作成専任。このタスクではスタブ実装（`unimplemented!()`）とテストコードのみを書く。実際のロジックはTask 2で別エージェントが書く。

**Files:**
- Modify: `crates/core/Cargo.toml`
- Modify: `crates/core/src/lib.rs`
- Create: `crates/core/src/update.rs`

**Interfaces:**
- Produces（Task 2が実装するシグネチャ。名前・型を変更してはならない）:
  - `pub struct UpdateInfo { pub version: String, pub deb_url: String, pub checksum_url: String }`（`Debug, PartialEq, Eq, Clone`を導出）
  - `pub fn parse_release_response(body: &str) -> Option<UpdateInfo>`
  - `pub fn is_newer(current: &str, latest: &str) -> Option<bool>`
  - `pub fn verify_checksum(data: &[u8], expected_hex: &str) -> bool`
  - `fn extract_checksum_for(checksum_text: &str, deb_url: &str) -> Result<String, String>`（private）

- [ ] **Step 1: `crates/core/Cargo.toml`に`sha2`を追加**

```toml
[dependencies]
encoding_rs = "0.8.35"
quick-xml = "0.42.0"
zip = { version = "8.6.0", default-features = false, features = ["deflate"] }
sha2 = "0.10"
```

- [ ] **Step 2: `crates/core/src/update.rs`を作成（スタブ+テスト）**

```rust
//! GitHub Releaseと連携したアップデート機構。バージョン比較・リリース
//! レスポンスの解析・チェックサム検証はここで完結させ、GUI(`crates/gui`)は
//! これらの関数を呼び出すだけにする。ネットワーク通信・`pkexec`実行を伴う
//! 関数(`check_latest`/`download_and_install`、Task 2で追加)はユニット
//! テスト対象外(実機検証で担保。設計書「テスト方針」参照)。

/// GitHub Releases APIの`GET .../releases/latest`から得られる、更新に
/// 必要な情報。`version`は`tag_name`の先頭`v`を除去した`X.Y.Z`形式。
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct UpdateInfo {
    pub version: String,
    pub deb_url: String,
    pub checksum_url: String,
}

/// GitHub Releases APIレスポンスのJSON文字列から、`.deb`アセットと
/// `SHA256SUMS.txt`という名前のチェックサムアセットのURLを取り出す。
/// パース失敗、またはいずれかのアセットが見つからない場合は`None`。
pub fn parse_release_response(body: &str) -> Option<UpdateInfo> {
    let _ = body;
    unimplemented!()
}

/// `current`(例: "0.1.0")と`latest`(例: "0.2.0"、先頭`v`は許容)を比較し、
/// `latest`の方が新しければ`Some(true)`。桁上がり("0.9.0" < "0.10.0")を
/// 正しく扱うため、文字列比較ではなく`(u32, u32, u32)`のタプル比較で行う。
/// どちらかがX.Y.Z形式としてパースできない場合は`None`。
pub fn is_newer(current: &str, latest: &str) -> Option<bool> {
    let _ = (current, latest);
    unimplemented!()
}

/// `data`のSHA-256が`expected_hex`(16進数文字列、大文字小文字は区別しない)
/// と一致するかを判定する。
pub fn verify_checksum(data: &[u8], expected_hex: &str) -> bool {
    let _ = (data, expected_hex);
    unimplemented!()
}

/// `sha256sum`形式(`<16進数ハッシュ>  <ファイル名>`)のチェックサムファイルの
/// 中身から、`deb_url`のファイル名部分(パス末尾)に一致する行のハッシュ値を
/// 取り出す。見つからなければエラーメッセージを返す。
fn extract_checksum_for(checksum_text: &str, deb_url: &str) -> Result<String, String> {
    let _ = (checksum_text, deb_url);
    unimplemented!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};

    #[test]
    fn is_newer_returns_true_when_minor_is_higher() {
        assert_eq!(is_newer("0.1.0", "0.2.0"), Some(true));
    }

    #[test]
    fn is_newer_returns_false_when_same_version() {
        assert_eq!(is_newer("0.1.0", "0.1.0"), Some(false));
    }

    #[test]
    fn is_newer_returns_false_when_current_is_ahead() {
        assert_eq!(is_newer("0.2.0", "0.1.9"), Some(false));
    }

    #[test]
    fn is_newer_compares_numerically_not_lexically() {
        // 文字列比較だと"0.9.0" > "0.10.0"と誤判定されてしまう桁上がりのケース。
        assert_eq!(is_newer("0.9.0", "0.10.0"), Some(true));
    }

    #[test]
    fn is_newer_strips_leading_v_from_latest() {
        assert_eq!(is_newer("0.1.0", "v0.2.0"), Some(true));
    }

    #[test]
    fn is_newer_returns_none_for_malformed_current() {
        assert_eq!(is_newer("not-a-version", "0.2.0"), None);
    }

    #[test]
    fn is_newer_returns_none_for_malformed_latest() {
        assert_eq!(is_newer("0.1.0", "not-a-version"), None);
    }

    const SAMPLE_RELEASE_JSON: &str = r#"
    {
        "tag_name": "v0.2.0",
        "assets": [
            {
                "name": "easy-archive_0.2.0-1_amd64.deb",
                "browser_download_url": "https://example.com/dl/easy-archive_0.2.0-1_amd64.deb"
            },
            {
                "name": "SHA256SUMS.txt",
                "browser_download_url": "https://example.com/dl/SHA256SUMS.txt"
            }
        ]
    }
    "#;

    #[test]
    fn parse_release_response_extracts_deb_and_checksum_urls() {
        let info = parse_release_response(SAMPLE_RELEASE_JSON).unwrap();
        assert_eq!(info.version, "0.2.0");
        assert_eq!(
            info.deb_url,
            "https://example.com/dl/easy-archive_0.2.0-1_amd64.deb"
        );
        assert_eq!(
            info.checksum_url,
            "https://example.com/dl/SHA256SUMS.txt"
        );
    }

    #[test]
    fn parse_release_response_returns_none_when_deb_asset_missing() {
        let json = r#"{"tag_name": "v0.2.0", "assets": [
            {"name": "SHA256SUMS.txt", "browser_download_url": "https://example.com/SHA256SUMS.txt"}
        ]}"#;
        assert_eq!(parse_release_response(json), None);
    }

    #[test]
    fn parse_release_response_returns_none_when_checksum_asset_missing() {
        let json = r#"{"tag_name": "v0.2.0", "assets": [
            {"name": "app.deb", "browser_download_url": "https://example.com/app.deb"}
        ]}"#;
        assert_eq!(parse_release_response(json), None);
    }

    #[test]
    fn parse_release_response_returns_none_for_invalid_json() {
        assert_eq!(parse_release_response("not json"), None);
    }

    #[test]
    fn verify_checksum_accepts_matching_sha256() {
        let data = b"easy archive";
        let mut hasher = Sha256::new();
        hasher.update(data);
        let expected = format!("{:x}", hasher.finalize());
        assert!(verify_checksum(data, &expected));
    }

    #[test]
    fn verify_checksum_accepts_uppercase_hex() {
        let data = b"easy archive";
        let mut hasher = Sha256::new();
        hasher.update(data);
        let expected = format!("{:x}", hasher.finalize()).to_uppercase();
        assert!(verify_checksum(data, &expected));
    }

    #[test]
    fn verify_checksum_rejects_mismatched_hash() {
        let data = b"easy archive";
        let wrong = "0".repeat(64);
        assert!(!verify_checksum(data, &wrong));
    }

    #[test]
    fn extract_checksum_for_finds_matching_line() {
        let text = "abc123  easy-archive_0.2.0-1_amd64.deb\ndef456  other.deb\n";
        let url = "https://example.com/dl/easy-archive_0.2.0-1_amd64.deb";
        assert_eq!(extract_checksum_for(text, url), Ok("abc123".to_string()));
    }

    #[test]
    fn extract_checksum_for_returns_err_when_missing() {
        let text = "abc123  other.deb\n";
        let url = "https://example.com/dl/easy-archive_0.2.0-1_amd64.deb";
        assert!(extract_checksum_for(text, url).is_err());
    }
}
```

- [ ] **Step 3: `crates/core/src/lib.rs`に`update`モジュールを追加**

```rust
pub mod auto;
pub mod compress;
pub mod encoding;
pub mod extract;
pub mod integration;
pub mod update;
```

- [ ] **Step 4: テストを実行して失敗することを確認**

Run: `cargo test -p easy-archive-core update::`
Expected: FAIL（`is_newer`/`parse_release_response`/`verify_checksum`/`extract_checksum_for`を呼ぶテストが`not implemented`のパニックで落ちる）

- [ ] **Step 5: コミット**

```bash
git add crates/core/Cargo.toml crates/core/src/lib.rs crates/core/src/update.rs
git commit -m "$(cat <<'EOF'
test: update.rsのインターフェースとユニットテストを追加(RED)

バージョン比較・GitHub APIレスポンス解析・チェックサム検証の
スタブとテストのみ。実装はTask 2で別途行う。
EOF
)"
```

---

## Task 2: crates/core — update.rsの実装（バージョン比較・解析・チェックサム・ダウンロード・インストール）

**担当エージェント:** Task 1とは別のエージェントに割り当てる。Task 1が書いたテストは変更しない。

**Files:**
- Modify: `crates/core/Cargo.toml`
- Modify: `crates/core/src/update.rs`

**Interfaces:**
- Consumes: Task 1が定義した`UpdateInfo`/`parse_release_response`/`is_newer`/`verify_checksum`/`extract_checksum_for`のシグネチャ（変更しない）
- Produces（Task 3が`crates/gui`から呼ぶ）:
  - `pub fn check_latest(current_version: &str) -> Option<UpdateInfo>`
  - `pub fn download_and_install(info: &UpdateInfo) -> Result<(), String>`

- [ ] **Step 1: `crates/core/Cargo.toml`に`ureq`・`serde`・`serde_json`を追加**

```toml
[dependencies]
encoding_rs = "0.8.35"
quick-xml = "0.42.0"
zip = { version = "8.6.0", default-features = false, features = ["deflate"] }
sha2 = "0.10"
ureq = "2.10"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

- [ ] **Step 2: `crates/core/src/update.rs`の先頭に解析用の内部型とインポートを追加**

ファイル先頭のdocコメント直後、`UpdateInfo`定義の前に追加:

```rust
use std::io::Read;

use serde::Deserialize;
use sha2::{Digest, Sha256};

#[derive(Deserialize)]
struct ReleaseAsset {
    name: String,
    browser_download_url: String,
}

#[derive(Deserialize)]
struct ReleaseResponse {
    tag_name: String,
    assets: Vec<ReleaseAsset>,
}
```

- [ ] **Step 3: スタブ4関数の中身を実装に置き換える**

```rust
pub fn parse_release_response(body: &str) -> Option<UpdateInfo> {
    let response: ReleaseResponse = serde_json::from_str(body).ok()?;
    let version = response.tag_name.trim_start_matches('v').to_string();
    let deb_url = response
        .assets
        .iter()
        .find(|a| a.name.ends_with(".deb"))
        .map(|a| a.browser_download_url.clone())?;
    let checksum_url = response
        .assets
        .iter()
        .find(|a| a.name == "SHA256SUMS.txt")
        .map(|a| a.browser_download_url.clone())?;
    Some(UpdateInfo {
        version,
        deb_url,
        checksum_url,
    })
}

fn parse_version(s: &str) -> Option<(u32, u32, u32)> {
    let s = s.trim_start_matches('v');
    let mut parts = s.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    let patch = parts.next()?.parse().ok()?;
    if parts.next().is_some() {
        return None;
    }
    Some((major, minor, patch))
}

pub fn is_newer(current: &str, latest: &str) -> Option<bool> {
    let current = parse_version(current)?;
    let latest = parse_version(latest)?;
    Some(latest > current)
}

pub fn verify_checksum(data: &[u8], expected_hex: &str) -> bool {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let actual = format!("{:x}", hasher.finalize());
    actual.eq_ignore_ascii_case(expected_hex.trim())
}

fn extract_checksum_for(checksum_text: &str, deb_url: &str) -> Result<String, String> {
    let filename = deb_url.rsplit('/').next().unwrap_or(deb_url);
    checksum_text
        .lines()
        .find_map(|line| {
            let mut parts = line.split_whitespace();
            let hash = parts.next()?;
            let name = parts.next()?.trim_start_matches('*');
            (name == filename).then(|| hash.to_string())
        })
        .ok_or_else(|| "チェックサムファイルに該当するエントリが見つかりません".to_string())
}
```

- [ ] **Step 4: テスト（Task 1作成分）を実行してすべて通ることを確認**

Run: `cargo test -p easy-archive-core update::`
Expected: PASS（全テストがグリーンになる。まだ`check_latest`/`download_and_install`は未追加のため、それらのテストは存在しない）

- [ ] **Step 5: コミット（RED→GREEN）**

```bash
git add crates/core/Cargo.toml crates/core/src/update.rs
git commit -m "$(cat <<'EOF'
feat: update.rsのバージョン比較・レスポンス解析・チェックサム検証を実装(GREEN)

Task 1で用意したテストがすべて通ることを確認済み。
EOF
)"
```

- [ ] **Step 6: ネットワーク通信・インストール実行を行う関数を追加**

`update.rs`の末尾（`#[cfg(test)] mod tests`より前）に追加。これらはネットワーク・外部プロセス実行を伴うためユニットテスト対象外（設計書の方針通り）:

```rust
fn fetch_latest_release_json() -> Result<String, String> {
    ureq::get("https://api.github.com/repos/Wanyaldee/Easy-Archive/releases/latest")
        .set("User-Agent", "easy-archive-update-checker")
        .set("Accept", "application/vnd.github+json")
        .call()
        .map_err(|e| format!("GitHub APIへの問い合わせに失敗しました: {e}"))?
        .into_string()
        .map_err(|e| format!("GitHub APIの応答を読み取れませんでした: {e}"))
}

/// 起動時チェック用。ネットワークエラー・パース失敗・更新なしはすべて`None`
/// として扱う(呼び出し側は理由を区別せず、バナーを出さないだけでよいため)。
pub fn check_latest(current_version: &str) -> Option<UpdateInfo> {
    let body = fetch_latest_release_json().ok()?;
    let info = parse_release_response(&body)?;
    if is_newer(current_version, &info.version)? {
        Some(info)
    } else {
        None
    }
}

fn download_bytes(url: &str) -> Result<Vec<u8>, String> {
    let mut buf = Vec::new();
    ureq::get(url)
        .set("User-Agent", "easy-archive-update-checker")
        .call()
        .map_err(|e| format!("ダウンロードに失敗しました: {e}"))?
        .into_reader()
        .read_to_end(&mut buf)
        .map_err(|e| format!("ダウンロードに失敗しました: {e}"))?;
    Ok(buf)
}

/// `.deb`とチェックサムファイルをダウンロードし、SHA-256を照合したうえで
/// `pkexec apt-get install -y <path>`を実行する。呼び出し側(GUI)はこれを
/// バックグラウンドスレッドから呼ぶこと(ネットワーク待ち・認証ダイアログの
/// 待ちでUIスレッドをブロックしないため)。
pub fn download_and_install(info: &UpdateInfo) -> Result<(), String> {
    let deb_bytes = download_bytes(&info.deb_url)?;
    let checksum_bytes = download_bytes(&info.checksum_url)?;
    let checksum_text = String::from_utf8(checksum_bytes)
        .map_err(|e| format!("チェックサムファイルの読み取りに失敗しました: {e}"))?;
    let expected_hex = extract_checksum_for(&checksum_text, &info.deb_url)?;
    if !verify_checksum(&deb_bytes, &expected_hex) {
        return Err("ダウンロードしたファイルの検証に失敗しました".to_string());
    }

    let deb_path = std::env::temp_dir().join("easy-archive-update.deb");
    std::fs::write(&deb_path, &deb_bytes)
        .map_err(|e| format!("ファイルの書き込みに失敗しました: {e}"))?;

    let status = std::process::Command::new("pkexec")
        .arg("apt-get")
        .arg("install")
        .arg("-y")
        .arg(&deb_path)
        .status()
        .map_err(|e| format!("インストールコマンドの起動に失敗しました: {e}"));
    let _ = std::fs::remove_file(&deb_path);
    let status = status?;

    if status.success() {
        Ok(())
    } else {
        match status.code() {
            // pkexecは認証ダイアログをキャンセルされた場合126、認証自体が
            // 得られなかった場合127を返す(man pkexecのRETURN VALUEで確認済み)。
            Some(126) => Err("認証がキャンセルされました".to_string()),
            Some(127) => Err("認証に失敗しました".to_string()),
            _ => Err("インストールに失敗しました".to_string()),
        }
    }
}
```

- [ ] **Step 7: ワークスペース全体のビルドを確認**

Run: `cargo build --workspace`
Expected: 成功（`check_latest`/`download_and_install`はまだどこからも呼ばれないが、`pub`関数なので未使用警告は出ない）

- [ ] **Step 8: コミット**

```bash
git add crates/core/src/update.rs
git commit -m "feat: GitHub APIからの取得とpkexec経由のインストール実行を追加"
```

---

## Task 3: crates/gui — 起動時アップデートチェック・バナー表示・インストール導線・再起動の実装

**Files:**
- Modify: `crates/gui/src/main.rs`

**Interfaces:**
- Consumes: `easy_archive_core::update::{UpdateInfo, check_latest, download_and_install}`（Task 2で実装済み）
- Produces: なし（トップレベルのGUI配線であり、他タスクはこれに依存しない）

- [ ] **Step 1: importと`UpdateState`を追加**

`crates/gui/src/main.rs`冒頭のuse文を以下に置き換える:

```rust
use std::env;
use std::path::PathBuf;
use std::sync::mpsc;

use easy_archive_core::auto;
use easy_archive_core::integration;
use easy_archive_core::update::{self, UpdateInfo};
use winit::platform::x11::EventLoopBuilderExtX11;
```

`fn main()`の直前に追加:

```rust
/// アップデートバナーの表示状態。`Available`以降は`UpdateInfo`を保持し、
/// インストール失敗時の再試行や完了後の表示切り替えに使う。
enum UpdateState {
    Idle,
    Available(UpdateInfo),
    Installing(UpdateInfo),
    Done,
    Error(UpdateInfo, String),
}

/// 起動時にバックグラウンドスレッドでGitHub Releaseの新バージョンを確認
/// する。ネットワーク待ちでUIスレッドをブロックしないよう、結果は
/// `mpsc::channel`経由でUIスレッドが毎フレーム`try_recv`で受け取る。
fn spawn_update_check() -> mpsc::Receiver<Option<UpdateInfo>> {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let result = update::check_latest(env!("CARGO_PKG_VERSION"));
        let _ = tx.send(result);
    });
    rx
}

/// 「アップデートする」ボタン押下時に呼ぶ。ダウンロード・チェックサム
/// 検証・`pkexec`実行(認証ダイアログ待ちを含む)はすべて時間がかかるため、
/// 別スレッドで行い、結果をチャネル経由でUIスレッドに返す。
fn spawn_update_install(info: UpdateInfo) -> mpsc::Receiver<Result<(), String>> {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let result = update::download_and_install(&info);
        let _ = tx.send(result);
    });
    rx
}

/// アップデート適用後にGUIを再起動する。`.deb`インストール直後は
/// `env::current_exe()`(`/proc/self/exe`)が置き換え前の(削除済み)inodeを
/// 指したままになりうる(ADR 0007の`resolve_cli_binary_path`と同種の理由)
/// ため使わず、`.deb`のインストール先として固定の既知パスを直接起動する。
fn restart_application() {
    let _ = std::process::Command::new("/usr/bin/easy-archive-gui").spawn();
    std::process::exit(0);
}
```

- [ ] **Step 2: `App`構造体とデフォルト初期化にアップデート関連フィールドを追加**

```rust
struct App {
    status: String,
    integration_installed: bool,
    update_state: UpdateState,
    update_check_rx: Option<mpsc::Receiver<Option<UpdateInfo>>>,
    update_install_rx: Option<mpsc::Receiver<Result<(), String>>>,
}

impl Default for App {
    fn default() -> Self {
        Self {
            status: String::new(),
            integration_installed: check_integration_installed(),
            update_state: UpdateState::Idle,
            update_check_rx: Some(spawn_update_check()),
            update_install_rx: None,
        }
    }
}
```

- [ ] **Step 3: `ui()`の先頭でチャネルをポーリングして状態を更新**

`fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame)`の本体、既存の`dropped`処理より前に追加:

```rust
if let Some(rx) = &self.update_check_rx {
    match rx.try_recv() {
        Ok(Some(info)) => {
            self.update_state = UpdateState::Available(info);
            self.update_check_rx = None;
        }
        Ok(None) => {
            self.update_check_rx = None;
        }
        Err(mpsc::TryRecvError::Empty) => {}
        Err(mpsc::TryRecvError::Disconnected) => {
            self.update_check_rx = None;
        }
    }
}

if let Some(rx) = &self.update_install_rx {
    match rx.try_recv() {
        Ok(Ok(())) => {
            self.update_state = UpdateState::Done;
            self.update_install_rx = None;
        }
        Ok(Err(e)) => {
            if let UpdateState::Installing(info) = &self.update_state {
                self.update_state = UpdateState::Error(info.clone(), e);
            }
            self.update_install_rx = None;
        }
        Err(mpsc::TryRecvError::Empty) => {}
        Err(mpsc::TryRecvError::Disconnected) => {
            self.update_install_rx = None;
        }
    }
}
```

- [ ] **Step 4: アップデートバナーを描画**

既存の`integration_banner`の`egui::Panel::top(...)`ブロックの直後に追加:

```rust
match &self.update_state {
    UpdateState::Idle => {}
    UpdateState::Available(info) => {
        let info = info.clone();
        egui::Panel::top("update_banner").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(format!("新しいバージョン v{} があります。", info.version));
                if ui.button("アップデートする").clicked() {
                    self.update_install_rx = Some(spawn_update_install(info.clone()));
                    self.update_state = UpdateState::Installing(info);
                }
            });
        });
    }
    UpdateState::Installing(_) => {
        egui::Panel::top("update_banner").show(ui, |ui| {
            ui.label(
                "アップデートをインストール中です…(認証ダイアログが表示されたら許可してください)",
            );
        });
    }
    UpdateState::Done => {
        egui::Panel::top("update_banner").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label("アップデートが完了しました。再起動してください。");
                if ui.button("再起動する").clicked() {
                    restart_application();
                }
            });
        });
    }
    UpdateState::Error(info, message) => {
        let info = info.clone();
        egui::Panel::top("update_banner").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(format!("アップデートに失敗しました: {message}"));
                if ui.button("再試行").clicked() {
                    self.update_install_rx = Some(spawn_update_install(info.clone()));
                    self.update_state = UpdateState::Installing(info);
                }
            });
        });
    }
}
```

- [ ] **Step 5: ビルドと既存テストの確認**

Run: `cargo build --workspace`
Expected: 成功

Run: `cargo test --workspace`
Expected: 既存のテスト(`handle_drop_*`/`resolve_cli_binary_path_*`/`integration_helpers_*`)がすべてPASS。本タスクでは新規ユニットテストは追加しない（実際のHTTP通信・`pkexec`実行・PolicyKit認証ダイアログを伴うため、設計書の方針通りユニットテスト対象外。実機での動作確認は別途ユーザーが実施する）。

- [ ] **Step 6: コミット**

```bash
git add crates/gui/src/main.rs
git commit -m "feat: GUIに起動時アップデートチェックとインストール導線を追加"
```

---

## Task 4: `.github/workflows/release.yml`の新規追加

**Files:**
- Create: `.github/workflows/release.yml`

**Interfaces:**
- Consumes: なし
- Produces: なし（CI設定のみ。`crates/core`/`crates/gui`のコードには依存しない）

- [ ] **Step 1: ワークフローファイルを作成**

```yaml
name: Release

on:
  push:
    tags:
      - "v*"

permissions:
  contents: write

jobs:
  release:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Install GUI build dependencies
        run: sudo apt-get update && sudo apt-get install -y libgtk-3-dev libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev libxkbcommon-dev
      - uses: dtolnay/rust-toolchain@stable
      # ci.ymlのdeb-packageジョブとビルド手順を意図的に重複させている。
      # 変更する際はci.ymlのコメント(deb-packageジョブ)と合わせて確認すること。
      - run: cargo install cargo-deb --version "^3.7"
      - run: cargo build --release --workspace
      - run: (cd crates/gui && cargo deb --no-build)
      - name: Generate checksums
        # `SHA256SUMS.txt`の2列目はファイル名のみにする必要がある
        # (crates/core/src/update.rsのextract_checksum_forがdeb_urlの
        # ファイル名部分と完全一致で照合するため)。target/debian内で
        # sha256sumを実行し、パスを含まないファイル名にする。
        run: (cd target/debian && sha256sum *.deb > SHA256SUMS.txt)
      - name: Create GitHub Release
        env:
          GH_TOKEN: ${{ github.token }}
        run: |
          gh release create "${{ github.ref_name }}" \
            --title "Easy Archive ${{ github.ref_name }}" \
            --generate-notes \
            target/debian/*.deb \
            target/debian/SHA256SUMS.txt
```

- [ ] **Step 2: 既存ワークフローとの整合性を目視確認**

`.github/workflows/ci.yml`の`deb-package`ジョブと比べ、aptパッケージ・`cargo-deb`のバージョン指定・ビルド手順（`cargo build --release --workspace` → `cargo deb --no-build`）が一致していることを確認する。自動的な構文チェックツール（`actionlint`等）は本プロジェクトに未導入のため新規導入しない（YAGNI）。

**注意（実動作確認について）**: 実際にタグ（`v*`）をpushしてこのワークフローを動かす確認は、GitHub Release作成という共有・公開の副作用を伴うため、本タスクの中では行わない。ユーザーの承認を得たうえで、次回バージョンのタグpush時に別途確認する。

- [ ] **Step 3: コミット**

```bash
git add .github/workflows/release.yml
git commit -m "ci: タグpush時に.debとチェックサムをGitHub Releaseへ自動アップロードする"
```

---

## 実機検証(このプランの範囲外・別途実施)

設計書の「テスト方針」の通り、以下は自動化されたタスクの対象外とし、次回バージョンをタグpushしてリリースした後、ユーザーがZorin OS Core（実機/VirtualBox仮想環境）で確認する:

1. 旧バージョンの`.deb`をインストールした状態でGUIを起動し、起動時バナーで新バージョンが検知されること
2. 「アップデートする」→ダウンロード→PolicyKit認証ダイアログ→`apt-get install`成功→「再起動する」ボタンでGUIが再起動し、新バージョンとして動作すること
3. チェックサムを意図的に壊した場合にエラー表示され、インストールが実行されないこと
4. PolicyKitの認証ダイアログでキャンセルした場合に「認証がキャンセルされました」と表示されること

確認結果はADR（例: `docs/adr/0008-github-release-auto-update.md`）に記録する。
