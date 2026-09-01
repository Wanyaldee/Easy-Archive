//! Nautilus(GNOME Files)の「スクリプト」メニュー用ラッパースクリプトを生成する。
//!
//! `nautilus-python`拡張は使わず、`~/.local/share/nautilus/scripts/`配下に
//! シバン付きシェルスクリプトを置くシンプルな方式を採用する(解凍/圧縮という
//! 単純な処理にリッチな拡張は不要と判断。詳細はADR 0005を参照)。
//!
//! この方式には拡張子等によるメニュー項目自体の出し分けができない制約が
//! あるが、スクリプト内部で`easy-archive auto`を呼ぶことで対象パスの実体を
//! 見て自己判定するため、どのファイルに対して実行しても正しく動作する。
//! Nautilusは標準出力/終了コードを表示しないため、`notify-send`で結果を
//! 通知する。

use std::path::PathBuf;

use super::GeneratedFile;

/// サブディレクトリ名(Nautilusではサブメニューとして表示される)。
const SCRIPT_DIR: &str = ".local/share/nautilus/scripts/Easy Archive";

pub fn generate(binary_path: &str) -> GeneratedFile {
    GeneratedFile {
        relative_path: PathBuf::from(SCRIPT_DIR).join("ここに解凍・圧縮"),
        content: format!(
            "#!/bin/sh\n\
             set -u\n\
             path=$(printf '%s\\n' \"$NAUTILUS_SCRIPT_SELECTED_FILE_PATHS\" | head -n1)\n\
             message=$(\"{binary_path}\" auto \"$path\" 2>&1)\n\
             status=$?\n\
             if command -v notify-send >/dev/null 2>&1; then\n\
             \x20\x20\x20\x20if [ \"$status\" -eq 0 ]; then\n\
             \x20\x20\x20\x20\x20\x20\x20\x20notify-send \"Easy Archive\" \"$message\"\n\
             \x20\x20\x20\x20else\n\
             \x20\x20\x20\x20\x20\x20\x20\x20notify-send -u critical \"Easy Archive\" \"$message\"\n\
             \x20\x20\x20\x20fi\n\
             fi\n"
        ),
        executable: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_executable_wrapper_script_under_submenu_dir() {
        let file = generate("/usr/bin/easy-archive");

        assert_eq!(
            file.relative_path,
            PathBuf::from(".local/share/nautilus/scripts/Easy Archive/ここに解凍・圧縮")
        );
        assert!(file.executable);
        assert!(file.content.starts_with("#!/bin/sh\n"));
        assert!(file.content.contains("NAUTILUS_SCRIPT_SELECTED_FILE_PATHS"));
        assert!(file.content.contains("\"/usr/bin/easy-archive\" auto \"$path\""));
        assert!(file.content.contains("notify-send"));
    }

    #[test]
    fn guards_notify_send_call_with_command_dash_v() {
        let file = generate("/usr/bin/easy-archive");

        // notify-send未インストール環境(実際のWSL開発環境で確認済み)でも
        // "command not found"エラーで異常終了しないことを保証する。
        assert!(file.content.contains("if command -v notify-send >/dev/null 2>&1; then"));
    }
}
