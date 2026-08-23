//! ZIPをディスクへ展開するロジック。
//!
//! エントリ名のデコードは`encoding::decode_entry_name`を使う(CLIの`list`
//! コマンドと同じロジック)。中身のバイト列は無変換で書き出す。

use std::error::Error;
use std::fs::{self, File};
use std::io::Read;
use std::path::Path;

use zip::{HasZipMetadata, ZipArchive};

use crate::encoding::decode_entry_name;

/// zip_pathの中身をdest_dirへ展開する。dest_dirが既に存在する場合は
/// エラーを返す(上書きしない)。戻り値は展開したファイル数
/// (ディレクトリエントリを除く)。
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
