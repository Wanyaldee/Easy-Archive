//! ZIPをディスクへ展開するロジック。
//!
//! エントリ名のデコードは`encoding::decode_entry_name`を使う(CLIの`list`
//! コマンドと同じロジック)。中身のバイト列は無変換で書き出す。

use std::error::Error;
use std::fs::{self, File};
use std::path::{Component, Path};

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

    // 途中でエラーになった場合は、作成済みのdest_dirを丸ごと削除してから
    // エラーを伝播する(再試行できるようにするため)。
    match extract_entries(&mut archive, dest_dir) {
        Ok(count) => Ok(count),
        Err(e) => {
            let _ = fs::remove_dir_all(dest_dir);
            Err(e)
        }
    }
}

fn extract_entries(
    archive: &mut ZipArchive<File>,
    dest_dir: &Path,
) -> Result<usize, Box<dyn Error>> {
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

        if Path::new(&name)
            .components()
            .any(|c| !matches!(c, Component::Normal(_)))
        {
            return Err(format!("安全でないエントリ名です: {name}").into());
        }
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
        let mut out_file = File::create(&out_path)
            .map_err(|e| format!("書き込みに失敗しました: {}: {e}", out_path.display()))?;
        std::io::copy(&mut entry, &mut out_file)
            .map_err(|e| format!("書き込みに失敗しました: {}: {e}", out_path.display()))?;
        count += 1;
    }

    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compress;
    use std::io::{Read, Write};
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

    /// `zip`クレートは書き込み時にエントリ名をサニタイズしないため、
    /// `..`や絶対パスを含む生のエントリ名でもそのままZIPに書き込める。
    /// zip-slip検証用のテストZIPを作るためのヘルパー。
    fn make_raw_test_zip(zip_path: &Path, entry_names: &[&str]) {
        let file = std::fs::File::create(zip_path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default();
        for name in entry_names {
            zip.start_file(*name, options).unwrap();
            zip.write_all(b"content").unwrap();
        }
        zip.finish().unwrap();
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

    #[test]
    fn extract_rejects_parent_dir_traversal_entry() {
        let dir = temp_dir("zipslip-parent");
        let zip_path = dir.join("evil.zip");
        make_raw_test_zip(&zip_path, &["../escaped.txt"]);

        let dest = dir.join("extracted");
        let result = extract(&zip_path, &dest);
        assert!(result.is_err());

        // dest_dirの外(dirの親)に書き出されていないことを確認する。
        let escape_target = dir.parent().unwrap().join("escaped.txt");
        assert!(!escape_target.exists());

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn extract_rejects_absolute_path_entry() {
        let dir = temp_dir("zipslip-abs");
        let zip_path = dir.join("evil.zip");
        make_raw_test_zip(&zip_path, &["/tmp/easy-archive-zipslip-abs-escape.txt"]);

        let dest = dir.join("extracted");
        let result = extract(&zip_path, &dest);
        assert!(result.is_err());

        let escape_target = Path::new("/tmp/easy-archive-zipslip-abs-escape.txt");
        assert!(!escape_target.exists());

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn extract_removes_partial_dest_dir_on_failure() {
        let dir = temp_dir("partial-cleanup");
        let zip_path = dir.join("evil.zip");
        // 1件目は正常に展開されるエントリ、2件目でzip-slip検知により失敗する。
        make_raw_test_zip(&zip_path, &["ok.txt", "../escaped.txt"]);

        let dest = dir.join("extracted");
        let result = extract(&zip_path, &dest);
        assert!(result.is_err());

        // 途中で作成されたdest_dirが後始末され、再試行可能な状態になっていることを確認する。
        assert!(!dest.exists());

        std::fs::remove_dir_all(&dir).ok();
    }
}
