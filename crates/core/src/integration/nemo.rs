//! Nemo(Cinnamon、Linux Mint系)のカスタムアクション(`.nemo_action`)を生成する。
//!
//! 配置先は`~/.local/share/nemo/actions/`。フォーマットはNemo公式リポジトリの
//! sample.nemo_action
//! (<https://github.com/linuxmint/nemo/blob/master/files/usr/share/nemo/actions/sample.nemo_action>)
//! を一次情報として確認済み(詳細はADR 0005を参照)。
//!
//! 拡張子/選択数(`Selection`/`Extensions`)による表示条件はあくまで見た目の
//! 出し分けであり、実際の処理は`Exec`で呼ぶ`easy-archive auto`が対象パスの
//! 実体を見て自己判定するため、どちらのメニューから呼ばれても正しく動作する。

use std::path::PathBuf;

use super::GeneratedFile;

pub fn generate(binary_path: &str) -> Vec<GeneratedFile> {
    vec![
        GeneratedFile {
            relative_path: PathBuf::from(
                ".local/share/nemo/actions/easy-archive-extract.nemo_action",
            ),
            content: format!(
                "[Nemo Action]\n\
                 Name=ここに解凍\n\
                 Comment=ZIPファイルをこの場所に解凍します\n\
                 Exec={binary_path} auto %f\n\
                 Icon-Name=archive-extract\n\
                 Selection=s\n\
                 Extensions=zip;\n"
            ),
            executable: false,
        },
        GeneratedFile {
            relative_path: PathBuf::from(
                ".local/share/nemo/actions/easy-archive-compress.nemo_action",
            ),
            content: format!(
                "[Nemo Action]\n\
                 Name=ここを圧縮\n\
                 Comment=選択したファイル/フォルダをZIPに圧縮します\n\
                 Exec={binary_path} auto %f\n\
                 Icon-Name=archive-insert\n\
                 Selection=s\n\
                 Extensions=any;\n"
            ),
            executable: false,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_extract_and_compress_actions() {
        let files = generate("/usr/bin/easy-archive");
        assert_eq!(files.len(), 2);

        let extract = &files[0];
        assert_eq!(
            extract.relative_path,
            PathBuf::from(".local/share/nemo/actions/easy-archive-extract.nemo_action")
        );
        assert!(extract.content.contains("[Nemo Action]"));
        assert!(extract.content.contains("Exec=/usr/bin/easy-archive auto %f"));
        assert!(extract.content.contains("Selection=s"));
        assert!(extract.content.contains("Extensions=zip;"));
        assert!(!extract.executable);

        let compress = &files[1];
        assert_eq!(
            compress.relative_path,
            PathBuf::from(".local/share/nemo/actions/easy-archive-compress.nemo_action")
        );
        assert!(compress.content.contains("Exec=/usr/bin/easy-archive auto %f"));
        assert!(compress.content.contains("Extensions=any;"));
    }
}
