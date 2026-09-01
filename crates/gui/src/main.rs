use std::path::PathBuf;

use easy_archive_core::auto;
use winit::platform::x11::EventLoopBuilderExtX11;

fn main() -> eframe::Result<()> {
    let mut options = eframe::NativeOptions::default();
    // winitのWaylandバックエンドはファイルドロップ(WindowEvent::DroppedFile)を
    // 実装していない(X11/Windows/macOSのみ実装)ため、Waylandセッション上では
    // ドラッグ&ドロップが一切反応しない。対象OS(Ubuntu/Zorin OS)は標準で
    // XWaylandを同梱しているため、X11バックエンドを強制してこれを回避する。
    options.event_loop_builder = Some(Box::new(|builder| {
        builder.with_x11();
    }));
    eframe::run_native(
        "Easy Archive",
        options,
        Box::new(|cc| {
            setup_japanese_font(&cc.egui_ctx);
            Ok(Box::new(App::default()))
        }),
    )
}

/// UIの日本語テキスト(ドロップ領域の案内文・結果メッセージ)を正しく表示する
/// ため、Noto Sans JPをegui既定のプロポーショナルフォントの先頭に追加する。
/// eguiの同梱フォントは日本語グリフを含まないため、これをしないと豆腐
/// (□)になる。ライセンスは`assets/fonts/NotoSansJP-LICENSE.txt`(SIL OFL)参照。
fn setup_japanese_font(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        "NotoSansJP".to_owned(),
        egui::FontData::from_static(include_bytes!(
            "../assets/fonts/NotoSansJP-Regular.otf"
        ))
        .into(),
    );
    fonts
        .families
        .entry(egui::FontFamily::Proportional)
        .or_default()
        .insert(0, "NotoSansJP".to_owned());
    ctx.set_fonts(fonts);
}

#[derive(Default)]
struct App {
    status: String,
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let dropped: Vec<PathBuf> = ui.ctx().input(|i| {
            i.raw
                .dropped_files
                .iter()
                .map(|f| f.path().to_path_buf())
                .collect()
        });

        if !dropped.is_empty() {
            self.status = handle_drop(&dropped);
        }

        egui::CentralPanel::default().show(ui, |ui| {
            ui.centered_and_justified(|ui| {
                let text = if self.status.is_empty() {
                    "ここにファイル/フォルダをドラッグ&ドロップしてください"
                } else {
                    &self.status
                };
                ui.label(text);
            });
        });
    }
}

/// ドロップされたパスの一覧を受け取り、解凍/圧縮のどちらかを実行して
/// 結果メッセージを返す。2件以上ドロップされた場合は何もせずエラーの
/// メッセージだけを返す。実際の判定・実行は`easy_archive_core::auto`に
/// 委譲する(ファイルマネージャー右クリックメニュー統合と共通のロジック)。
fn handle_drop(paths: &[PathBuf]) -> String {
    if paths.len() != 1 {
        return "一度に1つだけドロップしてください".to_string();
    }

    match auto::auto(&paths[0]) {
        Ok(message) => message,
        Err(e) => format!("エラー: {e}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "easy-archive-test-gui-{tag}-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// 解凍/圧縮の判定・命名規則・エラーケースは`easy_archive_core::auto`側の
    /// テスト(`crates/core/src/auto.rs`)で網羅済み。ここでは単一パスの結果を
    /// そのまま返す配線が壊れていないことだけをスモークテストで確認する。
    #[test]
    fn handle_drop_delegates_single_path_to_core_auto() {
        let dir = temp_dir("smoke");
        let file_path = dir.join("hello.txt");
        std::fs::write(&file_path, b"hello world").unwrap();

        let msg = handle_drop(&[file_path]);
        assert!(msg.contains("作成しました"), "unexpected message: {msg}");
        assert!(dir.join("hello.zip").exists());

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn handle_drop_rejects_multiple_paths() {
        let dir = temp_dir("multi");
        let a = dir.join("a.txt");
        let b = dir.join("b.txt");
        std::fs::write(&a, b"a").unwrap();
        std::fs::write(&b, b"b").unwrap();

        let msg = handle_drop(&[a.clone(), b.clone()]);
        assert_eq!(msg, "一度に1つだけドロップしてください");

        // 何も作成されていないことを確認する。
        assert!(!dir.join("a.zip").exists());
        assert!(!dir.join("b.zip").exists());

        std::fs::remove_dir_all(&dir).ok();
    }

}
