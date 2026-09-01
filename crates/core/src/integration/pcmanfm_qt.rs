//! PCManFM-Qt(LXQt系)のカスタムアクション(DES-EMA形式`.desktop`)を生成する。
//!
//! 配置先は`~/.local/share/file-manager/actions/`。DES-EMA
//! (Desktop Entry Specification - Extension for Menus and Actions)は
//! Nautilus-Actions由来の非標準拡張仕様で、PCManFM-Qtが実装を継承している。
//! フォーマットはlxqt-project.org公式Wiki
//! (<https://lxqt-project.org/wiki/custom_actions.html>)および
//! `gitlab.com/radio_dude/pcmanfm-context-menu`の実ファイルを一次情報として
//! 確認済み(詳細はADR 0005を参照)。
//!
//! 複数ファイル選択時の挙動に既知の不具合報告がある(lxqt/pcmanfm-qt#1039)
//! ため、`SelectionCount=1`で単一選択のみに絞る。
//! 「ここに解凍」「ここを圧縮」を分けず単一項目にしているのは、DES-EMAの
//! MIMEタイプ否定構文の挙動が未確認で、2項目に分けると片方が意図せず
//! 全ファイルに表示される可能性があるため。実際の処理は`Exec`で呼ぶ
//! `easy-archive auto`が対象パスの実体を見て自己判定するため、単一項目でも
//! 正しく動作する。

use std::path::PathBuf;

use super::GeneratedFile;

pub fn generate(binary_path: &str) -> GeneratedFile {
    GeneratedFile {
        relative_path: PathBuf::from(".local/share/file-manager/actions/easy-archive.desktop"),
        content: format!(
            "[Desktop Entry]\n\
             Type=Action\n\
             Name=Easy Archiveで処理\n\
             Icon=archive-extract\n\
             Profiles=auto;\n\
             \n\
             [X-Action-Profile auto]\n\
             Exec={binary_path} auto %f\n\
             MimeTypes=application/zip;inode/directory;all/allfiles;\n\
             SelectionCount=1\n"
        ),
        executable: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_des_ema_action() {
        let file = generate("/usr/bin/easy-archive");

        assert_eq!(
            file.relative_path,
            PathBuf::from(".local/share/file-manager/actions/easy-archive.desktop")
        );
        assert!(!file.executable);
        assert!(file.content.contains("Type=Action"));
        assert!(file.content.contains("Profiles=auto;"));
        assert!(file.content.contains("[X-Action-Profile auto]"));
        assert!(file.content.contains("Exec=/usr/bin/easy-archive auto %f"));
        assert!(file.content.contains("SelectionCount=1"));
    }
}
