use std::env;
use std::path::PathBuf;
use std::sync::mpsc;

use easy_archive_core::auto;
use easy_archive_core::integration;
use easy_archive_core::update::{self, UpdateInfo};
use winit::platform::x11::EventLoopBuilderExtX11;

/// アップデートバナーの表示状態。`Available`以降は`UpdateInfo`を保持し、
/// インストール失敗時の再試行や完了後の表示切り替えに使う。
enum UpdateState {
    Idle,
    Available(UpdateInfo),
    Installing(UpdateInfo),
    Done,
    Error(UpdateInfo, String),
}

/// 起動時にバックグラウンドスレッドでGitHub Releaseの新バージョンを確認
/// する。ネットワーク待ちでUIスレッドをブロックしないよう、結果は
/// `mpsc::channel`経由でUIスレッドが毎フレーム`try_recv`で受け取る。
fn spawn_update_check() -> mpsc::Receiver<Option<UpdateInfo>> {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let result = update::check_latest(env!("CARGO_PKG_VERSION"));
        let _ = tx.send(result);
    });
    rx
}

/// 「アップデートする」ボタン押下時に呼ぶ。ダウンロード・チェックサム
/// 検証・`pkexec`実行(認証ダイアログ待ちを含む)はすべて時間がかかるため、
/// 別スレッドで行い、結果をチャネル経由でUIスレッドに返す。
fn spawn_update_install(info: UpdateInfo) -> mpsc::Receiver<Result<(), String>> {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let result = update::download_and_install(&info);
        let _ = tx.send(result);
    });
    rx
}

/// アップデート適用後にGUIを再起動する。`.deb`インストール直後は
/// `env::current_exe()`(`/proc/self/exe`)が置き換え前の(削除済み)inodeを
/// 指したままになりうる(ADR 0007の`resolve_cli_binary_path`と同種の理由)
/// ため使わず、`.deb`のインストール先として固定の既知パスを直接起動する。
fn restart_application() {
    let _ = std::process::Command::new("/usr/bin/easy-archive-gui").spawn();
    std::process::exit(0);
}

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

/// GUIプロセス自身の実行ファイルパスから、隣接するCLIバイナリ(`easy-archive`)
/// のパスを解決する。ファイルマネージャー統合のExec行はCLIの`auto`サブ
/// コマンドを呼ぶ設計のため、GUIバイナリ自身のパス(`env::current_exe()`)を
/// そのまま使うと、統合ファイルがGUIを起動するだけでauto処理を一切実行
/// しない不具合になる(GUIはargvを読まない設計のため)。同じディレクトリに
/// 並んでインストールされる前提(.debでは/usr/bin/に両方設置される)で、
/// 存在すればそのパスを、見つからなければPATH解決に委ねる文字列
/// "easy-archive"にフォールバックする。
fn resolve_cli_binary_path() -> Result<String, String> {
    let current_exe =
        env::current_exe().map_err(|e| format!("実行ファイルのパスを取得できませんでした: {e}"))?;
    let cli_path = current_exe
        .parent()
        .map(|dir| dir.join("easy-archive"))
        .filter(|p| p.exists())
        .unwrap_or_else(|| PathBuf::from("easy-archive"));
    Ok(cli_path.to_string_lossy().into_owned())
}

/// `install-integration`同様、`$HOME`とファイルマネージャー統合が呼ぶべき
/// バイナリのパスを解決する。GUIプロセス自身の実行ユーザー権限で動くため、
/// CLIの`install-integration`と異なりpostinst(root権限)経由では不可能だった
/// 統合設置をここで安全に行える。バイナリのパスはGUI自身ではなく隣接する
/// CLI(`resolve_cli_binary_path`)を指す点に注意。
fn resolve_home_and_binary() -> Result<(PathBuf, String), String> {
    let home = env::var("HOME")
        .map(PathBuf::from)
        .map_err(|_| "HOME環境変数が設定されていません".to_string())?;
    let binary_path = resolve_cli_binary_path()?;
    Ok((home, binary_path))
}

/// 統合ファイルが未設置ならバナーを表示するための判定。解決できない場合
/// (通常の環境では起こらない)はバナーを表示しない側に倒す。
fn check_integration_installed() -> bool {
    match resolve_home_and_binary() {
        Ok((home, binary_path)) => integration::is_installed(&home, &binary_path).unwrap_or(true),
        Err(_) => true,
    }
}

/// 設置ボタンの押下時に呼ぶ。結果メッセージを返す。
fn install_integration() -> String {
    match resolve_home_and_binary() {
        Ok((home, binary_path)) => match integration::install_all(&home, &binary_path) {
            Ok(written) => format!(
                "ファイルマネージャー統合を設置しました({}件)",
                written.len()
            ),
            Err(e) => format!("エラー: {e}"),
        },
        Err(e) => format!("エラー: {e}"),
    }
}

struct App {
    status: String,
    integration_installed: bool,
    update_state: UpdateState,
    update_check_rx: Option<mpsc::Receiver<Option<UpdateInfo>>>,
    update_install_rx: Option<mpsc::Receiver<Result<(), String>>>,
}

impl Default for App {
    fn default() -> Self {
        Self {
            status: String::new(),
            integration_installed: check_integration_installed(),
            update_state: UpdateState::Idle,
            update_check_rx: Some(spawn_update_check()),
            update_install_rx: None,
        }
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        if let Some(rx) = &self.update_check_rx {
            match rx.try_recv() {
                Ok(Some(info)) => {
                    self.update_state = UpdateState::Available(info);
                    self.update_check_rx = None;
                }
                Ok(None) => {
                    self.update_check_rx = None;
                }
                Err(mpsc::TryRecvError::Empty) => {}
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.update_check_rx = None;
                }
            }
        }

        if let Some(rx) = &self.update_install_rx {
            match rx.try_recv() {
                Ok(Ok(())) => {
                    self.update_state = UpdateState::Done;
                    self.update_install_rx = None;
                }
                Ok(Err(e)) => {
                    if let UpdateState::Installing(info) = &self.update_state {
                        self.update_state = UpdateState::Error(info.clone(), e);
                    }
                    self.update_install_rx = None;
                }
                Err(mpsc::TryRecvError::Empty) => {}
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.update_install_rx = None;
                }
            }
        }

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

        if !self.integration_installed {
            egui::Panel::top("integration_banner").show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        "ファイルマネージャーの右クリックメニューにEasy Archiveを追加できます。",
                    );
                    if ui.button("設置する").clicked() {
                        self.status = install_integration();
                        self.integration_installed = check_integration_installed();
                    }
                });
            });
        }

        match &self.update_state {
            UpdateState::Idle => {}
            UpdateState::Available(info) => {
                let info = info.clone();
                egui::Panel::top("update_banner").show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(format!("新しいバージョン v{} があります。", info.version));
                        if ui.button("アップデートする").clicked() {
                            self.update_install_rx = Some(spawn_update_install(info.clone()));
                            self.update_state = UpdateState::Installing(info);
                        }
                    });
                });
            }
            UpdateState::Installing(_) => {
                egui::Panel::top("update_banner").show(ui, |ui| {
                    ui.label(
                        "アップデートをインストール中です…(認証ダイアログが表示されたら許可してください)",
                    );
                });
            }
            UpdateState::Done => {
                egui::Panel::top("update_banner").show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("アップデートが完了しました。再起動してください。");
                        if ui.button("再起動する").clicked() {
                            restart_application();
                        }
                    });
                });
            }
            UpdateState::Error(info, message) => {
                let info = info.clone();
                let message = message.clone();
                egui::Panel::top("update_banner").show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(format!("アップデートに失敗しました: {message}"));
                        if ui.button("再試行").clicked() {
                            self.update_install_rx = Some(spawn_update_install(info.clone()));
                            self.update_state = UpdateState::Installing(info);
                        }
                    });
                });
            }
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

    /// ファイルマネージャー統合のExec行はCLIの`auto`サブコマンドを呼ぶ設計
    /// のため、GUIバイナリ自身のパスを埋め込むと右クリックメニューが何も
    /// しなくなる(GUIはargvを読まない)。この回帰を防ぐためのテスト。
    /// なお`cargo test`環境では`current_exe()`が`target/debug/deps/`配下の
    /// テストランナーを指し、そこに`easy-archive`は並んでいないため、実際に
    /// 通るのはPATHフォールバック側の枝になる(文字列の契約は同じく検証できる)。
    #[test]
    fn resolve_cli_binary_path_never_returns_the_gui_binary_itself() {
        let path = resolve_cli_binary_path().unwrap();
        assert!(
            path.ends_with("easy-archive"),
            "CLIバイナリを指しているはず: {path}"
        );
        assert!(
            !path.ends_with("easy-archive-gui"),
            "GUIバイナリ自身を指してはいけない: {path}"
        );
    }

    /// `HOME`環境変数を書き換えるテスト同士を直列化するための排他ロック。
    /// `std::env::set_var`/`var`はプロセス全体で共有される環境変数テーブルを
    /// 操作するため、「他のテストが`HOME`という名前を読み書きしない」という
    /// 条件だけでは競合を防げない — `cargo test`はデフォルトでテストを別
    /// スレッドで並行実行するため、同時に走る別のテスト(またはその依存
    /// コード内部)がどのキーであれ`set_var`/`var`を呼べば、同じグローバル
    /// テーブルへのアクセスが競合しうる。これが`env::set_var`/`remove_var`
    /// が`unsafe`である理由そのものであり、このMutexを保持している間だけ
    /// `HOME`を差し替えることで実際に直列化を保証する。
    static HOME_ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    /// テスト中だけ`HOME`を差し替え、`Drop`で必ず元の値に復元するRAIIガード。
    /// `assert!`がガード構築と復元処理の間でパニックしても、スタック巻き戻し
    /// 時に`Drop::drop`が呼ばれるため、破損した`HOME`のグローバル状態が
    /// 後続のテストに漏れることはない。`HOME_ENV_LOCK`を保持したまま
    /// 差し替え・復元の両方を行うため、他スレッドとの競合も起きない。
    struct HomeEnvGuard {
        _lock: std::sync::MutexGuard<'static, ()>,
        original: Option<String>,
    }

    impl HomeEnvGuard {
        fn set(new_home: &std::path::Path) -> Self {
            let lock = HOME_ENV_LOCK
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            let original = std::env::var("HOME").ok();
            // SAFETY: `_lock`がプロセス内の`HOME`書き換えを直列化しているため、
            // このスレッドの外から同時に`set_var`/`var`が呼ばれることはない。
            unsafe {
                std::env::set_var("HOME", new_home);
            }
            Self {
                _lock: lock,
                original,
            }
        }
    }

    impl Drop for HomeEnvGuard {
        fn drop(&mut self) {
            // SAFETY: `_lock`を保持したまま復元するため、他スレッドとの競合は
            // 起きない(パニックによる巻き戻し中でも`_lock`はまだ生きている)。
            unsafe {
                match &self.original {
                    Some(value) => std::env::set_var("HOME", value),
                    None => std::env::remove_var("HOME"),
                }
            }
        }
    }

    /// `resolve_home_and_binary`は`$HOME`環境変数を直接読む(GUIプロセス自身
    /// の実行ユーザー権限で動くという設計上の理由。ヘルパーコメント参照)ため、
    /// パラメータ化できない。ここでは`install_all`/`is_installed`本体の
    /// ロジックはcrates/coreの単体テスト(Task 1)で網羅済みという前提のもと、
    /// `check_integration_installed`/`install_integration`が実際に環境変数
    /// 経由の`home`/`binary_path`をcore側へ正しく橋渡ししていることだけを
    /// 確認する。`HomeEnvGuard`が差し替え・復元・直列化のすべてを担う。
    #[test]
    fn integration_helpers_reflect_install_state_via_home_env() {
        let dir = temp_dir("integration-home");
        let _guard = HomeEnvGuard::set(&dir);

        assert!(!check_integration_installed(), "設置前はfalseを返すべき");

        let msg = install_integration();
        assert!(msg.contains("設置しました"), "unexpected message: {msg}");

        assert!(
            check_integration_installed(),
            "install_integration後はtrueを返すべき"
        );

        std::fs::remove_dir_all(&dir).ok();
    }
}
