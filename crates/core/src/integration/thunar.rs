//! Thunar(XFCE、Zorin OS Lite/Xubuntu系)のカスタムアクション(`uca.xml`)を
//! 安全にマージする。
//!
//! 配置先は`~/.config/Thunar/uca.xml`。既存のファイルには他の自作カスタム
//! アクションが含まれている可能性があるため、文字列連結ではなく正規のXML
//! パーサ(`quick-xml`)で`</actions>`終了タグの妥当な位置を検出し、その直前に
//! 新しい`<action>`要素を挿入する。既存内容はバイト単位でそのまま保持する。
//!
//! Thunar自身は`<unique-id>`の重複チェックを行わない単なる文字列として
//! 扱う(ADR 0005でソースコードを直接確認済み)ため、固定文字列のIDを
//! 目印にして「既に追加済みか」を自作ツール側で判定し、冪等性を担保する。

use std::error::Error;

use quick_xml::events::Event;
use quick_xml::reader::Reader;

use super::GeneratedFile;

const EXTRACT_UNIQUE_ID: &str = "easy-archive-extract-here";
const COMPRESS_UNIQUE_ID: &str = "easy-archive-compress-here";

const EMPTY_UCA_XML: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<actions>\n</actions>\n";

/// `crates/core/src/integration/mod.rs`の他DE向け`generate`と違い、Thunarは
/// 既存ファイルへの安全なマージが必要なため、呼び出し側が読み込んだ既存の
/// uca.xml(存在しなければ`None`)を渡してもらう形にする。戻り値は書き込むべき
/// 新しいuca.xml全体の内容。
pub fn merge(existing: Option<&str>, binary_path: &str) -> Result<GeneratedFile, Box<dyn Error>> {
    let base = existing.unwrap_or(EMPTY_UCA_XML);

    let mut to_insert = String::new();
    if !base.contains(EXTRACT_UNIQUE_ID) {
        to_insert.push_str(&extract_action_xml(binary_path));
    }
    if !base.contains(COMPRESS_UNIQUE_ID) {
        to_insert.push_str(&compress_action_xml(binary_path));
    }

    let content = if to_insert.is_empty() {
        base.to_string()
    } else {
        let insert_at = find_actions_close_tag_offset(base)?;
        let mut result = String::with_capacity(base.len() + to_insert.len());
        result.push_str(&base[..insert_at]);
        result.push_str(&to_insert);
        result.push_str(&base[insert_at..]);
        result
    };

    Ok(GeneratedFile {
        relative_path: std::path::PathBuf::from(".config/Thunar/uca.xml"),
        content,
        executable: false,
    })
}

fn extract_action_xml(binary_path: &str) -> String {
    format!(
        "  <action>\n\
         \x20\x20\x20\x20<icon>archive-extract</icon>\n\
         \x20\x20\x20\x20<name>ここに解凍</name>\n\
         \x20\x20\x20\x20<submenu></submenu>\n\
         \x20\x20\x20\x20<unique-id>{EXTRACT_UNIQUE_ID}</unique-id>\n\
         \x20\x20\x20\x20<command>{binary_path} auto %f</command>\n\
         \x20\x20\x20\x20<description>ZIPファイルをこの場所に解凍します</description>\n\
         \x20\x20\x20\x20<patterns>*.zip;*.ZIP</patterns>\n\
         \x20\x20\x20\x20<other-files/>\n\
         \x20\x20</action>\n"
    )
}

fn compress_action_xml(binary_path: &str) -> String {
    format!(
        "  <action>\n\
         \x20\x20\x20\x20<icon>archive-insert</icon>\n\
         \x20\x20\x20\x20<name>ここを圧縮</name>\n\
         \x20\x20\x20\x20<submenu></submenu>\n\
         \x20\x20\x20\x20<unique-id>{COMPRESS_UNIQUE_ID}</unique-id>\n\
         \x20\x20\x20\x20<command>{binary_path} auto %f</command>\n\
         \x20\x20\x20\x20<description>選択したファイル/フォルダをZIPに圧縮します</description>\n\
         \x20\x20\x20\x20<patterns>*</patterns>\n\
         \x20\x20\x20\x20<directories/>\n\
         \x20\x20\x20\x20<other-files/>\n\
         \x20\x20</action>\n"
    )
}

/// `</actions>`終了タグの開始バイトオフセットを、正規のXMLパーサで検出する。
/// 文字列検索(`find("</actions>")`)ではなく実際にXMLとして妥当な位置にある
/// `</actions>`かどうかをquick-xmlで検証する(コメント/CDATA内の同名文字列に
/// 惑わされないようにするため)。
fn find_actions_close_tag_offset(xml: &str) -> Result<usize, Box<dyn Error>> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(false);

    loop {
        let pos_before = reader.buffer_position() as usize;
        match reader.read_event() {
            Ok(Event::End(e)) if e.name().as_ref() == "actions" => {
                return Ok(pos_before);
            }
            Ok(Event::Eof) => {
                return Err("uca.xmlに<actions>要素が見つかりませんでした".into());
            }
            Ok(_) => continue,
            Err(e) => return Err(format!("uca.xmlの解析に失敗しました: {e}").into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_creates_new_uca_xml_when_absent() {
        let result = merge(None, "/usr/bin/easy-archive").unwrap();

        assert_eq!(
            result.relative_path,
            std::path::PathBuf::from(".config/Thunar/uca.xml")
        );
        assert!(!result.executable);
        assert!(result.content.contains(EXTRACT_UNIQUE_ID));
        assert!(result.content.contains(COMPRESS_UNIQUE_ID));
        assert!(result.content.contains("<actions>"));
        assert!(result.content.contains("</actions>"));
        assert!(result.content.contains("/usr/bin/easy-archive auto %f"));

        // 挿入後も妥当なXMLであることを検証する。
        assert_actions_close_tag_is_findable(&result.content);
    }

    #[test]
    fn merge_preserves_existing_unrelated_action() {
        let existing = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
                         <actions>\n\
                         \x20\x20<action>\n\
                         \x20\x20\x20\x20<icon>utilities-terminal</icon>\n\
                         \x20\x20\x20\x20<name>Open Terminal Here</name>\n\
                         \x20\x20\x20\x20<unique-id>1633867917298852-1</unique-id>\n\
                         \x20\x20\x20\x20<command>xfce4-terminal --working-directory %f</command>\n\
                         \x20\x20\x20\x20<patterns>*</patterns>\n\
                         \x20\x20\x20\x20<directories/>\n\
                         \x20\x20</action>\n\
                         </actions>\n";

        let result = merge(Some(existing), "/usr/bin/easy-archive").unwrap();

        assert!(result.content.contains("Open Terminal Here"));
        assert!(result.content.contains("1633867917298852-1"));
        assert!(result.content.contains(EXTRACT_UNIQUE_ID));
        assert!(result.content.contains(COMPRESS_UNIQUE_ID));
    }

    #[test]
    fn merge_is_idempotent() {
        let first = merge(None, "/usr/bin/easy-archive").unwrap();
        let second = merge(Some(&first.content), "/usr/bin/easy-archive").unwrap();

        assert_eq!(first.content, second.content);
        assert_eq!(
            second.content.matches(EXTRACT_UNIQUE_ID).count(),
            1,
            "重複登録されている: {}",
            second.content
        );
        assert_eq!(second.content.matches(COMPRESS_UNIQUE_ID).count(), 1);
    }

    #[test]
    fn find_actions_close_tag_offset_ignores_comment_containing_tag_name() {
        let xml = "<?xml version=\"1.0\"?>\n\
                   <actions>\n\
                   \x20\x20<!-- </actions> in a comment -->\n\
                   </actions>\n";

        let offset = find_actions_close_tag_offset(xml).unwrap();
        assert_eq!(&xml[offset..offset + "</actions>".len()], "</actions>");
    }

    fn assert_actions_close_tag_is_findable(xml: &str) {
        let offset = find_actions_close_tag_offset(xml).unwrap();
        assert_eq!(&xml[offset..offset + "</actions>".len()], "</actions>");
    }
}
