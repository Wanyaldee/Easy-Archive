//! GitHub Releaseと連携したアップデート機構。バージョン比較・リリース
//! レスポンスの解析・チェックサム検証はここで完結させ、GUI(`crates/gui`)は
//! これらの関数を呼び出すだけにする。ネットワーク通信・`pkexec`実行を伴う
//! 関数(`check_latest`/`download_and_install`、Task 2で追加)はユニット
//! テスト対象外(実機検証で担保。設計書「テスト方針」参照)。

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

/// `current`(例: "0.1.0")と`latest`(例: "0.2.0"、先頭`v`は許容)を比較し、
/// `latest`の方が新しければ`Some(true)`。桁上がり("0.9.0" < "0.10.0")を
/// 正しく扱うため、文字列比較ではなく`(u32, u32, u32)`のタプル比較で行う。
/// どちらかがX.Y.Z形式としてパースできない場合は`None`。
pub fn is_newer(current: &str, latest: &str) -> Option<bool> {
    let current = parse_version(current)?;
    let latest = parse_version(latest)?;
    Some(latest > current)
}

/// `data`のSHA-256が`expected_hex`(16進数文字列、大文字小文字は区別しない)
/// と一致するかを判定する。
pub fn verify_checksum(data: &[u8], expected_hex: &str) -> bool {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let actual = format!("{:x}", hasher.finalize());
    actual.eq_ignore_ascii_case(expected_hex.trim())
}

/// `sha256sum`形式(`<16進数ハッシュ>  <ファイル名>`)のチェックサムファイルの
/// 中身から、`deb_url`のファイル名部分(パス末尾)に一致する行のハッシュ値を
/// 取り出す。見つからなければエラーメッセージを返す。
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
