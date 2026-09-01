use std::env;
use std::error::Error;
use std::fs::File;
use std::path::{Path, PathBuf};

use zip::{HasZipMetadata, ZipArchive};

use easy_archive_core::auto;
use easy_archive_core::compress;
use easy_archive_core::encoding::decode_entry_name;
use easy_archive_core::extract;

const USAGE: &str = "使い方:\n  easy-archive list <ZIPファイルパス>\n  easy-archive compress <出力ZIPパス> <入力パス...>\n  easy-archive extract <ZIPファイルパス> <展開先ディレクトリ>\n  easy-archive auto <パス>";

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();

    match args.get(1).map(String::as_str) {
        Some("list") => run_list(&args[2..]),
        Some("compress") => run_compress(&args[2..]),
        Some("extract") => run_extract(&args[2..]),
        Some("auto") => run_auto(&args[2..]),
        Some(other) => Err(format!("不明なサブコマンドです: {other}\n{USAGE}").into()),
        None => Err(USAGE.into()),
    }
}

fn run_list(rest: &[String]) -> Result<(), Box<dyn Error>> {
    let path = rest.first().ok_or(USAGE)?;

    let file = File::open(path).map_err(|e| format!("ファイルを開けませんでした: {path}: {e}"))?;
    let mut archive = ZipArchive::new(file)
        .map_err(|e| format!("ZIPとして読み込めませんでした: {path}: {e}"))?;

    println!("ZIPファイル: {path} (エントリ数: {})", archive.len());

    for i in 0..archive.len() {
        let entry = archive
            .by_index_raw(i)
            .map_err(|e| format!("エントリ {i} の読み込みに失敗しました: {e}"))?;
        let utf8_flag_set = entry.get_metadata().is_utf8;
        let (name, used) = decode_entry_name(entry.name_raw(), utf8_flag_set);

        println!(
            "[{i:>3}] UTF-8フラグ={} 判定方式={:<9} 名前={name}",
            if utf8_flag_set { "有" } else { "無" },
            used.label(),
        );
    }

    Ok(())
}

fn run_compress(rest: &[String]) -> Result<(), Box<dyn Error>> {
    if rest.len() < 2 {
        return Err(USAGE.into());
    }
    let output_path = &rest[0];
    let inputs: Vec<PathBuf> = rest[1..].iter().map(PathBuf::from).collect();

    let file = File::create(output_path)
        .map_err(|e| format!("出力ファイルを作成できませんでした: {output_path}: {e}"))?;
    let (_, count) = compress::compress(file, &inputs)?;

    println!("ZIPファイルを作成しました: {output_path} (エントリ数: {count})");

    Ok(())
}

fn run_extract(rest: &[String]) -> Result<(), Box<dyn Error>> {
    if rest.len() < 2 {
        return Err(USAGE.into());
    }
    let zip_path = PathBuf::from(&rest[0]);
    let dest_dir = PathBuf::from(&rest[1]);

    let count = extract::extract(&zip_path, &dest_dir)?;

    println!("{} に展開しました(エントリ数: {count})", dest_dir.display());

    Ok(())
}

fn run_auto(rest: &[String]) -> Result<(), Box<dyn Error>> {
    let path = rest.first().ok_or(USAGE)?;
    let message = auto::auto(Path::new(path))?;
    println!("{message}");
    Ok(())
}
