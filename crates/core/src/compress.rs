//! ファイル/ディレクトリをZIPへ圧縮するロジック。
//!
//! ファイル名エンコーディングは常にUTF-8（`docs/spec.md`のスコープ方針
//! 通り、圧縮出力でのShift-JIS等は対象外）。`zip`クレートは`start_file`
//! にRustの`String`/`&str`しか受け付けないため、非ASCII文字を含む名前は
//! 自動的にZIP general purpose bit flagのbit11(UTF-8)が立つ。ここで
//! 明示的に何かを設定する必要はない。

use std::error::Error;
use std::fs::File;
use std::io::{Seek, Write};
use std::path::{Path, PathBuf};

use zip::write::{SimpleFileOptions, ZipWriter};

/// `inputs`内の各パスをZIPへ追加する。ファイルはbasenameで、ディレクトリ
/// は自身のbasenameをプレフィックスにして再帰的に追加する。
/// 戻り値は(finishしたwriter, 書き込んだエントリ数)。
pub fn compress<W: Write + Seek>(
    writer: W,
    inputs: &[PathBuf],
) -> Result<(W, usize), Box<dyn Error>> {
    let mut zip = ZipWriter::new(writer);
    let options =
        SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    let mut count = 0usize;

    for input in inputs {
        if input.is_dir() {
            let base = input
                .file_name()
                .ok_or_else(|| format!("ディレクトリ名を取得できません: {}", input.display()))?
                .to_string_lossy()
                .into_owned();
            add_dir(&mut zip, input, &base, options, &mut count)?;
        } else if input.is_file() {
            let name = input
                .file_name()
                .ok_or_else(|| format!("ファイル名を取得できません: {}", input.display()))?
                .to_string_lossy()
                .into_owned();
            add_file(&mut zip, input, &name, options, &mut count)?;
        } else {
            return Err(format!("入力パスが見つかりません: {}", input.display()).into());
        }
    }

    let inner = zip
        .finish()
        .map_err(|e| format!("ZIPの書き込みに失敗しました: {e}"))?;
    Ok((inner, count))
}

fn add_file<W: Write + Seek>(
    zip: &mut ZipWriter<W>,
    path: &Path,
    entry_name: &str,
    options: SimpleFileOptions,
    count: &mut usize,
) -> Result<(), Box<dyn Error>> {
    zip.start_file(entry_name, options)
        .map_err(|e| format!("エントリの作成に失敗しました: {entry_name}: {e}"))?;
    let mut f = File::open(path)
        .map_err(|e| format!("ファイルを開けませんでした: {}: {e}", path.display()))?;
    std::io::copy(&mut f, zip).map_err(|e| format!("書き込みに失敗しました: {entry_name}: {e}"))?;
    *count += 1;
    Ok(())
}

fn add_dir<W: Write + Seek>(
    zip: &mut ZipWriter<W>,
    dir: &Path,
    prefix: &str,
    options: SimpleFileOptions,
    count: &mut usize,
) -> Result<(), Box<dyn Error>> {
    let entries = std::fs::read_dir(dir)
        .map_err(|e| format!("ディレクトリを読み込めませんでした: {}: {e}", dir.display()))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("ディレクトリ読み込み中にエラー: {e}"))?;
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        // ZIP仕様上、区切り文字は常に'/'。Path::joinはOS依存になるため使わない。
        let entry_name = format!("{prefix}/{name}");

        if path.is_dir() {
            add_dir(zip, &path, &entry_name, options, count)?;
        } else if path.is_file() {
            add_file(zip, &path, &entry_name, options, count)?;
        }
        // ponytail: シンボリックリンク等の特殊ファイルは対象外(spec通りスコープ外)。黙ってスキップ。
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Cursor, Read};
    use zip::{HasZipMetadata, ZipArchive};

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "easy-archive-test-{tag}-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn compresses_single_flat_file() {
        let dir = temp_dir("flat");
        let file_path = dir.join("hello.txt");
        std::fs::write(&file_path, b"hello world").unwrap();

        let (cursor, count) = compress(Cursor::new(Vec::new()), &[file_path]).unwrap();
        assert_eq!(count, 1);

        let mut archive = ZipArchive::new(cursor).unwrap();
        assert_eq!(archive.len(), 1);
        let mut entry = archive.by_index(0).unwrap();
        assert_eq!(entry.name(), "hello.txt");
        let mut content = String::new();
        entry.read_to_string(&mut content).unwrap();
        assert_eq!(content, "hello world");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn compresses_directory_with_nested_file_using_basename_prefix() {
        let dir = temp_dir("nested");
        let root = dir.join("reports");
        std::fs::create_dir_all(root.join("sub")).unwrap();
        std::fs::write(root.join("a.txt"), b"a").unwrap();
        std::fs::write(root.join("sub").join("b.txt"), b"b").unwrap();

        let (cursor, count) = compress(Cursor::new(Vec::new()), &[root]).unwrap();
        assert_eq!(count, 2);

        let mut archive = ZipArchive::new(cursor).unwrap();
        let mut names: Vec<String> = (0..archive.len())
            .map(|i| archive.by_index(i).unwrap().name().to_string())
            .collect();
        names.sort();
        assert_eq!(names, vec!["reports/a.txt", "reports/sub/b.txt"]);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn japanese_filename_sets_utf8_flag() {
        let dir = temp_dir("ja");
        let file_path = dir.join("日本語.txt");
        std::fs::write(&file_path, b"content").unwrap();

        let (cursor, _) = compress(Cursor::new(Vec::new()), &[file_path]).unwrap();

        let mut archive = ZipArchive::new(cursor).unwrap();
        let entry = archive.by_index_raw(0).unwrap();
        assert!(entry.get_metadata().is_utf8);
        assert_eq!(entry.name_raw(), "日本語.txt".as_bytes());

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn content_bytes_round_trip_unmodified() {
        let dir = temp_dir("bytes");
        let file_path = dir.join("bin.dat");
        let raw: Vec<u8> = (0u8..=255).collect();
        std::fs::write(&file_path, &raw).unwrap();

        let (cursor, _) = compress(Cursor::new(Vec::new()), &[file_path]).unwrap();

        let mut archive = ZipArchive::new(cursor).unwrap();
        let mut entry = archive.by_index(0).unwrap();
        let mut out = Vec::new();
        entry.read_to_end(&mut out).unwrap();
        assert_eq!(out, raw);

        std::fs::remove_dir_all(&dir).ok();
    }
}
