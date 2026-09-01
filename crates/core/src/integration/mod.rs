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
use std::path::PathBuf;

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
}
