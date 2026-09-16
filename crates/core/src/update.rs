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
