//! 主要ファイルマネージャーへの右クリックメニュー統合。
//!
//! 各ファイルマネージャー(Nautilus/Nemo/Thunar/Dolphin/PCManFM-Qt)は右クリック
//! メニューへのカスタム項目追加の仕組みがそれぞれ異なる。調査結果と技術的決定は
//! [ADR 0005](../../../../docs/adr/0005-file-manager-integration-mechanisms.md)
//! を参照。
//!
//! すべてのファイルマネージャー統合は、生成するコマンドのExec行を
//! `<easy-archiveバイナリのパス> auto %f`(各DEのプレースホルダ記法に読み替え)
//! に統一する。これにより、「ここに解凍」「ここを圧縮」のどちらのメニュー項目
//! から呼ばれても`auto`サブコマンド(`crate::auto::auto`)がパスの実体を見て
//! 正しい処理を選ぶため、DE側のメニュー表示条件(拡張子フィルタ)の精度は
//! 正確性に影響しない。

pub mod dolphin;
pub mod nautilus;
pub mod nemo;
pub mod pcmanfm_qt;
pub mod thunar;

use std::error::Error;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

/// ファイルマネージャー統合のために生成する1ファイル分の情報。
/// `relative_path`はホームディレクトリからの相対パス。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneratedFile {
    pub relative_path: PathBuf,
    pub content: String,
    pub executable: bool,
}

/// 対応する全ファイルマネージャー分の統合ファイルをまとめて生成する。
/// Thunarのみ既存の`uca.xml`(存在すれば)を渡して冪等マージする。
pub fn all_generated_files(
    binary_path: &str,
    existing_thunar_uca_xml: Option<&str>,
) -> Result<Vec<GeneratedFile>, Box<dyn Error>> {
    let mut files = nemo::generate(binary_path);
    files.push(dolphin::generate(binary_path));
    files.push(nautilus::generate(binary_path));
    files.push(pcmanfm_qt::generate(binary_path));
    files.push(thunar::merge(existing_thunar_uca_xml, binary_path)?);
    Ok(files)
}

/// 対応する主要ファイルマネージャー(Nautilus/Nemo/Thunar/Dolphin/PCManFM-Qt)
/// に「ここに解凍」「ここを圧縮」の右クリックメニューを設置する。環境検出は
/// せず、対応する全ファイルマネージャー分のファイルを無条件に配置する
/// (該当ファイルマネージャーが未インストールでも実害はないため。詳細は
/// ADR 0005を参照)。
///
/// 戻り値は書き込んだファイルの絶対パス一覧。CLIの`install-integration`と
/// GUIの設置ボタンの両方から呼ばれる共通のインストール処理。
pub fn install_all(home: &Path, binary_path: &str) -> Result<Vec<PathBuf>, Box<dyn Error>> {
    let existing_uca = fs::read_to_string(home.join(thunar::THUNAR_UCA_RELATIVE_PATH)).ok();
    let files = all_generated_files(binary_path, existing_uca.as_deref())?;

    let mut written = Vec::new();
    for file in &files {
        let target = home.join(&file.relative_path);
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

        written.push(target);
    }

    Ok(written)
}

/// 対応する統合ファイルが`home`配下に全て設置済みかを判定する。GUIが起動時に
/// 「設置する」ボタンを表示すべきか判断するために使う。Thunarの`uca.xml`は
/// 他の自作カスタムアクションを含みうる共有ファイルのため、ファイルの存在
/// ではなく中身に本ツールの目印(`thunar::EXTRACT_UNIQUE_ID`)が含まれるかで
/// 判定する。
pub fn is_installed(home: &Path, binary_path: &str) -> Result<bool, Box<dyn Error>> {
    let existing_uca = fs::read_to_string(home.join(thunar::THUNAR_UCA_RELATIVE_PATH)).ok();
    let thunar_installed = existing_uca
        .as_deref()
        .is_some_and(|c| c.contains(thunar::EXTRACT_UNIQUE_ID));

    let files = all_generated_files(binary_path, existing_uca.as_deref())?;
    let others_installed = files
        .iter()
        .filter(|f| f.relative_path != Path::new(thunar::THUNAR_UCA_RELATIVE_PATH))
        .all(|f| home.join(&f.relative_path).exists());

    Ok(thunar_installed && others_installed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_generated_files_covers_every_supported_file_manager() {
        let files = all_generated_files("/usr/bin/easy-archive", None).unwrap();

        // Nemo(2) + Dolphin(1) + Nautilus(1) + PCManFM-Qt(1) + Thunar(1) = 6
        assert_eq!(files.len(), 6);

        let paths: Vec<String> = files
            .iter()
            .map(|f| f.relative_path.to_string_lossy().into_owned())
            .collect();
        assert!(paths.iter().any(|p| p.contains("nemo/actions") && p.contains("extract")));
        assert!(paths.iter().any(|p| p.contains("nemo/actions") && p.contains("compress")));
        assert!(paths.iter().any(|p| p.contains("kio/servicemenus")));
        assert!(paths.iter().any(|p| p.contains("nautilus/scripts")));
        assert!(paths.iter().any(|p| p.contains("file-manager/actions")));
        assert!(paths.iter().any(|p| p.contains(".config/Thunar/uca.xml")));
    }

    fn temp_home(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "easy-archive-test-integration-{tag}-{}",
            std::process::id()
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn install_all_writes_every_file_and_flips_is_installed() {
        let home = temp_home("install");
        assert!(!is_installed(&home, "/usr/bin/easy-archive").unwrap());

        let written = install_all(&home, "/usr/bin/easy-archive").unwrap();
        assert_eq!(written.len(), 6);
        for path in &written {
            assert!(path.exists());
        }
        assert!(is_installed(&home, "/usr/bin/easy-archive").unwrap());

        fs::remove_dir_all(&home).ok();
    }

    #[test]
    fn install_all_sets_executable_permission_on_scripts() {
        let home = temp_home("perm");
        let written = install_all(&home, "/usr/bin/easy-archive").unwrap();

        let nautilus_script = written
            .iter()
            .find(|p| p.to_string_lossy().contains("nautilus/scripts"))
            .expect("nautilusスクリプトが書き込まれているはず");
        let mode = fs::metadata(nautilus_script).unwrap().permissions().mode();
        assert_ne!(mode & 0o111, 0, "実行権限が付与されているはず");

        fs::remove_dir_all(&home).ok();
    }

    /// Thunar固有の判定(uca.xmlの中身に`EXTRACT_UNIQUE_ID`が含まれるか)が
    /// 実際に効いていることを確かめる。先に`install_all`で他5ファイルを
    /// 設置して`others_installed`をtrueにしたうえでuca.xmlだけを無関係な
    /// 内容へ上書きするため、`is_installed`がfalseを返す理由はThunar側の
    /// 判定以外にありえない。
    #[test]
    fn is_installed_ignores_unrelated_existing_thunar_actions() {
        let home = temp_home("thunar-unrelated");
        install_all(&home, "/usr/bin/easy-archive").unwrap();
        assert!(is_installed(&home, "/usr/bin/easy-archive").unwrap());

        fs::write(
            home.join(thunar::THUNAR_UCA_RELATIVE_PATH),
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<actions>\n  <action><unique-id>someone-else</unique-id></action>\n</actions>\n",
        )
        .unwrap();

        assert!(
            !is_installed(&home, "/usr/bin/easy-archive").unwrap(),
            "他5ファイルが設置済みでも、uca.xmlに本ツールの目印がなければfalseのはず"
        );

        fs::remove_dir_all(&home).ok();
    }
}
