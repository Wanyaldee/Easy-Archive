use std::path::{Path, PathBuf};

use easy_archive_core::{compress, extract};
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
/// メッセージだけを返す。
fn handle_drop(paths: &[PathBuf]) -> String {
    if paths.len() != 1 {
        return "一度に1つだけドロップしてください".to_string();
    }
    let path = &paths[0];

    let is_zip_file = path.is_file()
        && path
            .extension()
            .map(|e| e.eq_ignore_ascii_case("zip"))
            .unwrap_or(false);

    if is_zip_file {
        do_extract(path)
    } else if path.is_dir() || path.is_file() {
        do_compress(path)
    } else {
        format!("入力パスが見つかりません: {}", path.display())
    }
}

fn do_extract(zip_path: &Path) -> String {
    let (Some(parent), Some(stem)) = (zip_path.parent(), zip_path.file_stem().and_then(|s| s.to_str())) else {
        return format!("パスを解析できませんでした: {}", zip_path.display());
    };
    let dest_dir = parent.join(stem);

    match extract::extract(zip_path, &dest_dir) {
        Ok(count) => format!("{} に展開しました(エントリ数: {count})", dest_dir.display()),
        Err(e) => format!("エラー: {e}"),
    }
}

fn do_compress(source: &Path) -> String {
    // ディレクトリには「拡張子」の概念がないため、file_stem()で`.`以降を
    // 切り落とすとファイル名が壊れる(例: "R7.4 名簿" → "R7")。
    // ディレクトリはfile_name()、ファイルはfile_stem()を使う。
    let name = if source.is_dir() {
        source.file_name().and_then(|s| s.to_str())
    } else {
        source.file_stem().and_then(|s| s.to_str())
    };
    let (Some(parent), Some(name)) = (source.parent(), name) else {
        return format!("パスを解析できませんでした: {}", source.display());
    };
    let output_path = parent.join(format!("{name}.zip"));

    if output_path.exists() {
        return format!("既に存在します: {}", output_path.display());
    }

    let file = match std::fs::File::create(&output_path) {
        Ok(f) => f,
        Err(e) => {
            return format!(
                "出力ファイルを作成できませんでした: {}: {e}",
                output_path.display()
            )
        }
    };

    match compress::compress(file, &[source.to_path_buf()]) {
        Ok((_, count)) => format!("{} を作成しました(エントリ数: {count})", output_path.display()),
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

    #[test]
    fn handle_drop_compresses_single_file() {
        let dir = temp_dir("file");
        let file_path = dir.join("hello.txt");
        std::fs::write(&file_path, b"hello world").unwrap();

        let msg = handle_drop(&[file_path.clone()]);
        assert!(msg.contains("作成しました"), "unexpected message: {msg}");

        let expected_zip = dir.join("hello.zip");
        assert!(expected_zip.exists());

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn handle_drop_compresses_single_directory() {
        let dir = temp_dir("dir");
        let source = dir.join("reports");
        std::fs::create_dir_all(&source).unwrap();
        std::fs::write(source.join("a.txt"), b"a").unwrap();

        let msg = handle_drop(&[source.clone()]);
        // core::compress自体はcompress.rsのテストで内容の正しさを検証済み
        // なので、ここではGUI層の判定・呼び出し結果(出力先とエントリ数の
        // 報告)だけを確認する。
        assert!(msg.contains("作成しました"), "unexpected message: {msg}");
        assert!(msg.contains("エントリ数: 1"), "unexpected message: {msg}");

        let expected_zip = dir.join("reports.zip");
        assert!(expected_zip.exists());

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn handle_drop_compresses_directory_with_dot_in_name() {
        let dir = temp_dir("dotdir");
        // "R7.4"のような、年度表記等で"."を含む自治体・学校の実在フォルダ名を想定。
        let source = dir.join("R7.4");
        std::fs::create_dir_all(&source).unwrap();
        std::fs::write(source.join("a.txt"), b"a").unwrap();

        let msg = handle_drop(&[source.clone()]);
        assert!(msg.contains("作成しました"), "unexpected message: {msg}");

        // file_stem()だと"R7.zip"になってしまうバグの回帰テスト。
        let expected_zip = dir.join("R7.4.zip");
        assert!(expected_zip.exists(), "expected {} to exist", expected_zip.display());
        assert!(!dir.join("R7.zip").exists());

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn handle_drop_extracts_single_zip_file() {
        let dir = temp_dir("zip");
        let source = dir.join("src_dir");
        std::fs::create_dir_all(&source).unwrap();
        std::fs::write(source.join("a.txt"), b"a").unwrap();

        let zip_path = dir.join("src_dir.zip");
        let file = std::fs::File::create(&zip_path).unwrap();
        compress::compress(file, &[source.clone()]).unwrap();

        // 展開先に既存フォルダとの衝突が起きないよう、圧縮元は消しておく。
        std::fs::remove_dir_all(&source).unwrap();

        let msg = handle_drop(&[zip_path.clone()]);
        assert!(msg.contains("展開しました"), "unexpected message: {msg}");

        let dest = dir.join("src_dir");
        assert!(dest.is_dir());
        // compress()はディレクトリのbasenameをプレフィックスに使うため、
        // ZIP内のエントリ名は"src_dir/a.txt"になる。
        assert!(dest.join("src_dir").join("a.txt").exists());

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

    #[test]
    fn handle_drop_compress_fails_when_output_already_exists() {
        let dir = temp_dir("compress-exists");
        let file_path = dir.join("hello.txt");
        std::fs::write(&file_path, b"hello world").unwrap();

        let existing_zip = dir.join("hello.zip");
        std::fs::write(&existing_zip, b"not a real zip, should not be overwritten").unwrap();

        let msg = handle_drop(&[file_path]);
        assert!(msg.contains("既に存在します"), "unexpected message: {msg}");

        // 上書きされていないことを確認する。
        let content = std::fs::read(&existing_zip).unwrap();
        assert_eq!(content, b"not a real zip, should not be overwritten");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn handle_drop_extract_fails_when_dest_already_exists() {
        let dir = temp_dir("extract-exists");
        let source = dir.join("src_dir");
        std::fs::create_dir_all(&source).unwrap();
        std::fs::write(source.join("a.txt"), b"a").unwrap();

        let zip_path = dir.join("src_dir.zip");
        let file = std::fs::File::create(&zip_path).unwrap();
        compress::compress(file, &[source.clone()]).unwrap();

        // 圧縮元ディレクトリがそのまま展開先(src_dir)と衝突する状態にする。
        let msg = handle_drop(&[zip_path]);
        assert!(msg.contains("既に存在します"), "unexpected message: {msg}");

        // 元のディレクトリの中身がそのまま残っている(上書きされていない)ことを確認する。
        assert!(source.join("a.txt").exists());

        std::fs::remove_dir_all(&dir).ok();
    }
}
