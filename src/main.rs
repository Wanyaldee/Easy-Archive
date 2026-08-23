use std::env;
use std::error::Error;
use std::fs::File;

use zip::{HasZipMetadata, ZipArchive};

mod encoding;
use encoding::decode_entry_name;

fn main() -> Result<(), Box<dyn Error>> {
    let path = env::args()
        .nth(1)
        .ok_or("使い方: easy-archive <ZIPファイルパス>")?;

    let file =
        File::open(&path).map_err(|e| format!("ファイルを開けませんでした: {path}: {e}"))?;
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
