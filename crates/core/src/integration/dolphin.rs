//! Dolphin(KDE Plasma、Kubuntu系)のKIOサービスメニューを生成する。
//!
//! 配置先は`~/.local/share/kio/servicemenus/`(KDE Frameworks 5.85以降、
//! Plasma 5後期〜Plasma 6で共通)。ファイルには実行権限が必須(付与しないと
//! ホームディレクトリ配下は認可されない)。フォーマットはKDE公式Developer
//! Docs(<https://develop.kde.org/docs/apps/dolphin/service-menus/>)を
//! 一次情報として確認済み(詳細はADR 0005を参照)。
//!
//! `Actions=`に列挙した項目は同じ`MimeType`条件下で常に両方表示される
//! (アクションごとの個別MIME条件は付けられない)。実際の処理は`Exec`で呼ぶ
//! `easy-archive auto`が対象パスの実体を見て自己判定するため、どちらの
//! メニューから呼ばれても正しく動作する。

use std::path::PathBuf;

use super::GeneratedFile;

pub fn generate(binary_path: &str) -> GeneratedFile {
    GeneratedFile {
        relative_path: PathBuf::from(".local/share/kio/servicemenus/easy-archive.desktop"),
        content: format!(
            "[Desktop Entry]\n\
             Type=Service\n\
             MimeType=application/zip;inode/directory;application/octet-stream;\n\
             Actions=extractHere;compressHere;\n\
             \n\
             [Desktop Action extractHere]\n\
             Name=ここに解凍\n\
             Icon=archive-extract\n\
             Exec={binary_path} auto %f\n\
             \n\
             [Desktop Action compressHere]\n\
             Name=ここを圧縮\n\
             Icon=archive-insert\n\
             Exec={binary_path} auto %f\n"
        ),
        executable: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_service_menu_with_both_actions() {
        let file = generate("/usr/bin/easy-archive");

        assert_eq!(
            file.relative_path,
            PathBuf::from(".local/share/kio/servicemenus/easy-archive.desktop")
        );
        assert!(file.executable);
        assert!(file.content.contains("Type=Service"));
        assert!(file.content.contains("Actions=extractHere;compressHere;"));
        assert!(file.content.contains("[Desktop Action extractHere]"));
        assert!(file.content.contains("[Desktop Action compressHere]"));
        assert_eq!(
            file.content.matches("Exec=/usr/bin/easy-archive auto %f").count(),
            2
        );
    }
}
