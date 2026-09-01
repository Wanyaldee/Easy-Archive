//! パスの実体を見て解凍/圧縮を自動判定する共有ロジック。
//!
//! GUI(ドラッグ&ドロップ)・各ファイルマネージャーの右クリックメニュー
//! (`crates/core/src/integration/`)の両方から共通で呼ばれる。「拡張子が
//! `.zip`のファイルなら解凍、それ以外のファイル/ディレクトリなら圧縮」
//! という判定を1箇所に集約し、呼び出し側ごとの実装差異をなくす。

use std::error::Error;
use std::fs::File;
use std::path::Path;

use crate::{compress, extract};

/// 単一のパスを受け取り、解凍/圧縮のどちらかを実行して結果メッセージを返す。
pub fn auto(path: &Path) -> Result<String, Box<dyn Error>> {
    let is_zip_file = path.is_file()
        && path
            .extension()
            .map(|e| e.eq_ignore_ascii_case("zip"))
            .unwrap_or(false);

    if is_zip_file {
        do_extract(path)
    } else if path.is_dir() || path.is_file() {
        do_compress(path)
    } else {
        Err(format!("入力パスが見つかりません: {}", path.display()).into())
    }
}

fn do_extract(zip_path: &Path) -> Result<String, Box<dyn Error>> {
    let parent = zip_path
        .parent()
        .ok_or_else(|| format!("パスを解析できませんでした: {}", zip_path.display()))?;
    let stem = zip_path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| format!("パスを解析できませんでした: {}", zip_path.display()))?;
    let dest_dir = parent.join(stem);

    let count = extract::extract(zip_path, &dest_dir)?;
    Ok(format!("{} に展開しました(エントリ数: {count})", dest_dir.display()))
}

fn do_compress(source: &Path) -> Result<String, Box<dyn Error>> {
    // ディレクトリには「拡張子」の概念がないため、file_stem()で`.`以降を
    // 切り落とすとファイル名が壊れる(例: "R7.4 名簿" → "R7")。
    // ディレクトリはfile_name()、ファイルはfile_stem()を使う。
    let name = if source.is_dir() {
        source.file_name().and_then(|s| s.to_str())
    } else {
        source.file_stem().and_then(|s| s.to_str())
    };
    let parent = source
        .parent()
        .ok_or_else(|| format!("パスを解析できませんでした: {}", source.display()))?;
    let name =
        name.ok_or_else(|| format!("パスを解析できませんでした: {}", source.display()))?;
    let output_path = parent.join(format!("{name}.zip"));

    if output_path.exists() {
        return Err(format!("既に存在します: {}", output_path.display()).into());
    }

    let file = File::create(&output_path)
        .map_err(|e| format!("出力ファイルを作成できませんでした: {}: {e}", output_path.display()))?;

    let (_, count) = compress::compress(file, &[source.to_path_buf()])?;
    Ok(format!("{} を作成しました(エントリ数: {count})", output_path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "easy-archive-test-auto-{tag}-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn auto_compresses_single_file() {
        let dir = temp_dir("file");
        let file_path = dir.join("hello.txt");
        std::fs::write(&file_path, b"hello world").unwrap();

        let msg = auto(&file_path).unwrap();
        assert!(msg.contains("作成しました"), "unexpected message: {msg}");

        let expected_zip = dir.join("hello.zip");
        assert!(expected_zip.exists());

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn auto_compresses_single_directory() {
        let dir = temp_dir("dir");
        let source = dir.join("reports");
        std::fs::create_dir_all(&source).unwrap();
        std::fs::write(source.join("a.txt"), b"a").unwrap();

        let msg = auto(&source).unwrap();
        assert!(msg.contains("作成しました"), "unexpected message: {msg}");
        assert!(msg.contains("エントリ数: 1"), "unexpected message: {msg}");

        let expected_zip = dir.join("reports.zip");
        assert!(expected_zip.exists());

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn auto_compresses_directory_with_dot_in_name() {
        let dir = temp_dir("dotdir");
        // "R7.4"のような、年度表記等で"."を含む自治体・学校の実在フォルダ名を想定。
        let source = dir.join("R7.4");
        std::fs::create_dir_all(&source).unwrap();
        std::fs::write(source.join("a.txt"), b"a").unwrap();

        let msg = auto(&source).unwrap();
        assert!(msg.contains("作成しました"), "unexpected message: {msg}");

        // file_stem()だと"R7.zip"になってしまうバグの回帰テスト。
        let expected_zip = dir.join("R7.4.zip");
        assert!(expected_zip.exists(), "expected {} to exist", expected_zip.display());
        assert!(!dir.join("R7.zip").exists());

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn auto_extracts_single_zip_file() {
        let dir = temp_dir("zip");
        let source = dir.join("src_dir");
        std::fs::create_dir_all(&source).unwrap();
        std::fs::write(source.join("a.txt"), b"a").unwrap();

        let zip_path = dir.join("src_dir.zip");
        let file = std::fs::File::create(&zip_path).unwrap();
        compress::compress(file, &[source.clone()]).unwrap();

        // 展開先に既存フォルダとの衝突が起きないよう、圧縮元は消しておく。
        std::fs::remove_dir_all(&source).unwrap();

        let msg = auto(&zip_path).unwrap();
        assert!(msg.contains("展開しました"), "unexpected message: {msg}");

        let dest = dir.join("src_dir");
        assert!(dest.is_dir());
        // compress()はディレクトリのbasenameをプレフィックスに使うため、
        // ZIP内のエントリ名は"src_dir/a.txt"になる。
        assert!(dest.join("src_dir").join("a.txt").exists());

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn auto_compress_fails_when_output_already_exists() {
        let dir = temp_dir("compress-exists");
        let file_path = dir.join("hello.txt");
        std::fs::write(&file_path, b"hello world").unwrap();

        let existing_zip = dir.join("hello.zip");
        std::fs::write(&existing_zip, b"not a real zip, should not be overwritten").unwrap();

        let err = auto(&file_path).unwrap_err();
        assert!(err.to_string().contains("既に存在します"), "unexpected error: {err}");

        // 上書きされていないことを確認する。
        let content = std::fs::read(&existing_zip).unwrap();
        assert_eq!(content, b"not a real zip, should not be overwritten");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn auto_extract_fails_when_dest_already_exists() {
        let dir = temp_dir("extract-exists");
        let source = dir.join("src_dir");
        std::fs::create_dir_all(&source).unwrap();
        std::fs::write(source.join("a.txt"), b"a").unwrap();

        let zip_path = dir.join("src_dir.zip");
        let file = std::fs::File::create(&zip_path).unwrap();
        compress::compress(file, &[source.clone()]).unwrap();

        // 圧縮元ディレクトリがそのまま展開先(src_dir)と衝突する状態にする。
        let err = auto(&zip_path).unwrap_err();
        assert!(err.to_string().contains("既に存在します"), "unexpected error: {err}");

        // 元のディレクトリの中身がそのまま残っている(上書きされていない)ことを確認する。
        assert!(source.join("a.txt").exists());

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn auto_fails_when_path_does_not_exist() {
        let dir = temp_dir("missing");
        let missing = dir.join("does_not_exist");

        let err = auto(&missing).unwrap_err();
        assert!(err.to_string().contains("見つかりません"), "unexpected error: {err}");

        std::fs::remove_dir_all(&dir).ok();
    }
}
