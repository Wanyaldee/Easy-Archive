use std::env;
use std::error::Error;
use std::fs::{self, File};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use zip::{HasZipMetadata, ZipArchive};

use easy_archive_core::auto;
use easy_archive_core::compress;
use easy_archive_core::encoding::decode_entry_name;
use easy_archive_core::extract;
use easy_archive_core::integration::{self, thunar};

const USAGE: &str = "使い方:\n  easy-archive list <ZIPファイルパス>\n  easy-archive compress <出力ZIPパス> <入力パス...>\n  easy-archive extract <ZIPファイルパス> <展開先ディレクトリ>\n  easy-archive auto <パス>\n  easy-archive install-integration [--dry-run]\n  easy-archive uninstall-integration";

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();

    match args.get(1).map(String::as_str) {
        Some("list") => run_list(&args[2..]),
        Some("compress") => run_compress(&args[2..]),
        Some("extract") => run_extract(&args[2..]),
        Some("auto") => run_auto(&args[2..]),
        Some("install-integration") => run_install_integration(&args[2..]),
        Some("uninstall-integration") => run_uninstall_integration(),
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

fn home_dir() -> Result<PathBuf, Box<dyn Error>> {
    env::var("HOME")
        .map(PathBuf::from)
        .map_err(|_| "HOME環境変数が設定されていません".into())
}

fn thunar_uca_xml_path(home: &Path) -> PathBuf {
    home.join(".config/Thunar/uca.xml")
}

/// 対応する主要ファイルマネージャー(Nautilus/Nemo/Thunar/Dolphin/PCManFM-Qt)
/// に「ここに解凍」「ここを圧縮」の右クリックメニューを設置する。環境検出は
/// せず、対応する全ファイルマネージャー分のファイルを無条件に配置する
/// (該当ファイルマネージャーが未インストールでも実害はないため。詳細は
/// ADR 0005を参照)。
fn run_install_integration(rest: &[String]) -> Result<(), Box<dyn Error>> {
    let dry_run = rest.iter().any(|a| a == "--dry-run");

    let binary_path = env::current_exe()
        .map_err(|e| format!("実行ファイルのパスを取得できませんでした: {e}"))?
        .to_string_lossy()
        .into_owned();
    let home = home_dir()?;

    let existing_uca = fs::read_to_string(thunar_uca_xml_path(&home)).ok();
    let files = integration::all_generated_files(&binary_path, existing_uca.as_deref())?;

    for file in &files {
        let target = home.join(&file.relative_path);
        if dry_run {
            println!(
                "[dry-run] {}{}",
                target.display(),
                if file.executable { " (実行可能)" } else { "" }
            );
            continue;
        }

        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("ディレクトリを作成できませんでした: {}: {e}", parent.display()))?;
        }
        fs::write(&target, &file.content)
            .map_err(|e| format!("書き込みに失敗しました: {}: {e}", target.display()))?;

        if file.executable {
            let mut perms = fs::metadata(&target)
                .map_err(|e| format!("権限を取得できませんでした: {}: {e}", target.display()))?
                .permissions();
            perms.set_mode(perms.mode() | 0o755);
            fs::set_permissions(&target, perms)
                .map_err(|e| format!("実行権限を設定できませんでした: {}: {e}", target.display()))?;
        }

        println!("設置しました: {}", target.display());
    }

    Ok(())
}

/// `install-integration`で設置したファイルを取り除く。Nemo/Dolphin/Nautilus/
/// PCManFM-Qtのファイルは無条件に削除する。Thunarのuca.xmlはユーザーの他の
/// カスタムアクションを含みうるため、ファイル全体は削除せず、easy-archive
/// 由来の`<action>`要素だけを取り除く。
fn run_uninstall_integration() -> Result<(), Box<dyn Error>> {
    let home = home_dir()?;

    let removable_paths = [
        ".local/share/nemo/actions/easy-archive-extract.nemo_action",
        ".local/share/nemo/actions/easy-archive-compress.nemo_action",
        ".local/share/kio/servicemenus/easy-archive.desktop",
        ".local/share/file-manager/actions/easy-archive.desktop",
    ];
    for relative in removable_paths {
        let target = home.join(relative);
        match fs::remove_file(&target) {
            Ok(()) => println!("削除しました: {}", target.display()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(format!("削除に失敗しました: {}: {e}", target.display()).into()),
        }
    }

    // Nautilusのスクリプトはサブディレクトリごと削除する(他ファイルを
    // 置いていない前提のディレクトリのため)。
    let nautilus_dir = home.join(".local/share/nautilus/scripts/Easy Archive");
    match fs::remove_dir_all(&nautilus_dir) {
        Ok(()) => println!("削除しました: {}", nautilus_dir.display()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(format!("削除に失敗しました: {}: {e}", nautilus_dir.display()).into()),
    }

    let uca_path = thunar_uca_xml_path(&home);
    if let Ok(existing) = fs::read_to_string(&uca_path) {
        let updated = thunar::remove(&existing)?;
        if updated != existing {
            fs::write(&uca_path, updated)
                .map_err(|e| format!("書き込みに失敗しました: {}: {e}", uca_path.display()))?;
            println!("更新しました: {}", uca_path.display());
        }
    }

    Ok(())
}
