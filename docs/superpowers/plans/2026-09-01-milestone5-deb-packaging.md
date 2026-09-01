# マイルストーン5(`.deb`パッケージ化) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `easy-archive`(CLI)・`easy-archive-gui`(GUI)を`cargo-deb`で1つの`.deb`にまとめ、Ubuntu/Zorin OS上で`.deb`をダブルクリックするだけでインストールできるようにする。ファイルマネージャーへの右クリックメニュー統合(既存の`install-integration`)は、postinst(root権限・`$HOME`不明)では自動化できないため、GUI起動時に表示するボタンから設置できるようにし、非エンジニアの利用者がターミナルを一切使わずに導入を完了できることを狙う。

**Architecture:** 現状`run_install_integration`(CLI)にベタ書きされているファイル書き込みロジックを`crates/core/src/integration/mod.rs`の`install_all`/`is_installed`関数として抽出し、CLIとGUIの両方から共通で呼べるようにする。GUIは自プロセス内でこの関数を直接呼ぶ(サブプロセス起動なし)。`.deb`のビルドは`crates/gui/Cargo.toml`の`[package.metadata.deb]`で完結させ、前提条件の確認・自動セットアップを行う`packaging/build-deb.sh`でラップする。

**Tech Stack:** 既存のRust(edition 2024)ワークスペースはそのまま。新規ビルドツールとして`cargo-deb`(サブコマンド、`[dependencies]`には追加しない)を導入。

**Spec:** `docs/spec.md`(マイルストーン5)。技術的決定の詳細はTask 6で新規作成する`docs/adr/0007-deb-packaging.md`に記録する。

## Global Constraints

- ドキュメント・コメント・コミットメッセージは日本語。コード識別子は英語(Rust慣習)
- 文字コード判定・ZIP読み書きロジック(`encoding`/`compress`/`extract`)は変更しない。このマイルストーンは配布・統合設置導線のみが対象
- 対応フォーマットはZIPのみ、対象OSはUbuntu系のみ(変更なし)
- 新規`[dependencies]`は追加しない(YAGNI)。`cargo-deb`はビルド時サブコマンドであり、`[package.metadata.deb]`はCargo.tomlのメタデータテーブルであって依存クレートではない
- ファイルマネージャー統合の自動設置は**postinstでは行わない**(root権限で走り、ログインユーザーの`$HOME`が分からないため)。GUI起動時のボタンから、GUIプロセス自身の実行ユーザー権限で行う
- READMEの.debインストール手順は「ダブルクリックでインストール」を主動線とし、`sudo dpkg -i`はトラブルシューティング用の補足に留める
- 各Taskの最後に1コミットする(過去のマイルストーンの粒度に合わせる)
- Maintainer/copyright情報は`git log`のコミット作者情報に合わせ`Wanyaldee <gooya.3322@gmail.com>`を使う
- `sudo`を要するコマンド(`apt-get install`等)は、このplan中のどのTaskでも実際に実行しない。スクリプト内に確認プロンプト付きで用意するに留め、実行確認は各コマンドの分岐ロジックの目視レビューと`bash -n`の構文チェックで行う(この開発環境には既にGUIビルド依存パッケージが揃っているため、`build-deb.sh`本体の実行自体はTask 4で問題なく最後まで走らせられる)

---

## Task 1: `crates/core` — 統合設置ロジックをライブラリ関数として抽出する

**Files:**
- Modify: `crates/core/src/integration/mod.rs`(`install_all`/`is_installed`を追加)
- Modify: `crates/core/src/integration/thunar.rs`(`EXTRACT_UNIQUE_ID`を`pub(crate)`に変更)
- Modify: `crates/core/src/main.rs`(`run_install_integration`を`install_all`を呼ぶ薄いラッパーに置き換え、未使用になる`PermissionsExt`importを削除)

**Interfaces:**
- Produces: `easy_archive_core::integration::install_all(home: &Path, binary_path: &str) -> Result<Vec<PathBuf>, Box<dyn Error>>` — Task 2(GUI)がこの関数を直接呼び出す
- Produces: `easy_archive_core::integration::is_installed(home: &Path, binary_path: &str) -> Result<bool, Box<dyn Error>>` — Task 2(GUI)がこの関数を直接呼び出す
- Consumes: `easy_archive_core::integration::all_generated_files(binary_path: &str, existing_thunar_uca_xml: Option<&str>) -> Result<Vec<GeneratedFile>, Box<dyn Error>>`(既存、変更しない)

- [ ] **Step 1: 現状のファイルを確認する**

```bash
sed -n '1,20p' crates/core/src/integration/mod.rs
sed -n '100,150p' crates/core/src/main.rs
```

`run_install_integration`(非dry-run時)がファイルの書き込み・`chmod`を直接行っていることを確認する。

- [ ] **Step 2: `thunar::EXTRACT_UNIQUE_ID`を`mod.rs`から参照できるようにする**

`crates/core/src/integration/thunar.rs`の

```rust
const EXTRACT_UNIQUE_ID: &str = "easy-archive-extract-here";
```

を

```rust
pub(crate) const EXTRACT_UNIQUE_ID: &str = "easy-archive-extract-here";
```

に変更する(`COMPRESS_UNIQUE_ID`はそのままでよい)。

- [ ] **Step 3: `crates/core/src/integration/mod.rs`のimportを追加する**

ファイル冒頭を以下に置き換える:

```rust
pub mod dolphin;
pub mod nautilus;
pub mod nemo;
pub mod pcmanfm_qt;
pub mod thunar;

use std::error::Error;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
```

(既存の`GeneratedFile`構造体・`all_generated_files`関数はそのまま変更しない)

- [ ] **Step 4: `install_all`/`is_installed`を追加する**

`all_generated_files`関数の直後に以下を追加する:

```rust
/// 対応する全ファイルマネージャー分の統合ファイルを実際に`home`配下へ書き込む。
/// 戻り値は書き込んだファイルの絶対パス一覧。CLIの`install-integration`と
/// GUIの設置ボタンの両方から呼ばれる共通のインストール処理。
pub fn install_all(home: &Path, binary_path: &str) -> Result<Vec<PathBuf>, Box<dyn Error>> {
    let existing_uca = fs::read_to_string(home.join(".config/Thunar/uca.xml")).ok();
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
    let existing_uca = fs::read_to_string(home.join(".config/Thunar/uca.xml")).ok();
    let thunar_installed = existing_uca
        .as_deref()
        .is_some_and(|c| c.contains(thunar::EXTRACT_UNIQUE_ID));

    let files = all_generated_files(binary_path, existing_uca.as_deref())?;
    let others_installed = files
        .iter()
        .filter(|f| !f.relative_path.ends_with("uca.xml"))
        .all(|f| home.join(&f.relative_path).exists());

    Ok(thunar_installed && others_installed)
}
```

- [ ] **Step 5: テストを追加する**

既存の`#[cfg(test)] mod tests`ブロック内、既存の`all_generated_files_covers_every_supported_file_manager`テストの下に追加する:

```rust
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

    #[test]
    fn is_installed_ignores_unrelated_existing_thunar_actions() {
        let home = temp_home("thunar-unrelated");
        let thunar_dir = home.join(".config/Thunar");
        fs::create_dir_all(&thunar_dir).unwrap();
        fs::write(
            thunar_dir.join("uca.xml"),
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<actions>\n  <action><unique-id>someone-else</unique-id></action>\n</actions>\n",
        )
        .unwrap();

        assert!(!is_installed(&home, "/usr/bin/easy-archive").unwrap());

        fs::remove_dir_all(&home).ok();
    }
```

- [ ] **Step 6: テストを実行して通ることを確認する**

```bash
cargo test -p easy-archive-core integration:: 2>&1 | tail -30
```

Expected: 新規3テストを含む`integration::tests::`配下が全て`ok`。

- [ ] **Step 7: `run_install_integration`を`install_all`を呼ぶだけのラッパーに置き換える**

`crates/core/src/main.rs`の`run_install_integration`関数全体を以下に置き換える:

```rust
fn run_install_integration(rest: &[String]) -> Result<(), Box<dyn Error>> {
    let dry_run = rest.iter().any(|a| a == "--dry-run");

    let binary_path = env::current_exe()
        .map_err(|e| format!("実行ファイルのパスを取得できませんでした: {e}"))?
        .to_string_lossy()
        .into_owned();
    let home = home_dir()?;

    if dry_run {
        let existing_uca = fs::read_to_string(thunar_uca_xml_path(&home)).ok();
        let files = integration::all_generated_files(&binary_path, existing_uca.as_deref())?;
        for file in &files {
            let target = home.join(&file.relative_path);
            println!(
                "[dry-run] {}{}",
                target.display(),
                if file.executable { " (実行可能)" } else { "" }
            );
        }
        return Ok(());
    }

    for target in integration::install_all(&home, &binary_path)? {
        println!("設置しました: {}", target.display());
    }

    Ok(())
}
```

- [ ] **Step 8: 未使用になった`PermissionsExt`importを削除する**

`crates/core/src/main.rs`冒頭の

```rust
use std::os::unix::fs::PermissionsExt;
```

の行を削除する(この行は`run_install_integration`のchmod処理でのみ使われていたが、Step 7でその処理が`integration::install_all`側へ移ったため不要になる)。

- [ ] **Step 9: ビルド・テストで警告や失敗がないことを確認する**

```bash
cargo build --workspace 2>&1 | tail -30
cargo test -p easy-archive-core 2>&1 | tail -40
```

Expected: 警告なしでビルド成功。既存テスト+Step 5の新規3テストが全て`ok`。

- [ ] **Step 10: `install-integration`/`--dry-run`の手動確認**

```bash
mkdir -p /tmp/deb-milestone-check/home
HOME=/tmp/deb-milestone-check/home cargo run -p easy-archive-core --bin easy-archive -- install-integration --dry-run
HOME=/tmp/deb-milestone-check/home cargo run -p easy-archive-core --bin easy-archive -- install-integration
find /tmp/deb-milestone-check/home -type f
rm -rf /tmp/deb-milestone-check
```

Expected: `--dry-run`は`[dry-run]`接頭辞付きで6件のパスを表示するのみで何も書き込まない。2回目の実行で実際に6ファイルが書き込まれる(Nemo×2、Dolphin、Nautilusスクリプト、PCManFM-Qt、Thunar uca.xml)。

- [ ] **Step 11: コミットする**

```bash
git add crates/core/src/integration/mod.rs crates/core/src/integration/thunar.rs crates/core/src/main.rs
git commit -m "$(cat <<'EOF'
統合設置ロジックをintegration::install_all/is_installedとして抽出

これまでCLIのrun_install_integrationにベタ書きされていたファイル
書き込み・chmod処理をcrates/core/src/integration/mod.rsのライブラリ
関数install_all/is_installedとして抽出した。CLIはこれを呼ぶだけの
薄いラッパーになった。マイルストーン5(GUIからの統合設置ボタン)で
GUI側からも同じ関数を直接呼び出すための下準備。

is_installedはThunarのuca.xmlが他の自作カスタムアクションを含みうる
共有ファイルであることを踏まえ、ファイル存在ではなく本ツールの
unique-idの有無で判定する。新規3テストで検証済み。
EOF
)"
```

---

## Task 2: `crates/gui` — ファイルマネージャー統合の設置ボタンを追加する

**Files:**
- Modify: `crates/gui/src/main.rs`

**Interfaces:**
- Consumes: `easy_archive_core::integration::install_all(home: &Path, binary_path: &str) -> Result<Vec<PathBuf>, Box<dyn Error>>`(Task 1)
- Consumes: `easy_archive_core::integration::is_installed(home: &Path, binary_path: &str) -> Result<bool, Box<dyn Error>>`(Task 1)

このワークスペースは`eframe 0.36.1`/`egui 0.36.1`をピン留めしている。このバージョンの`eframe::App`トレイトは`fn ui(&mut self, ui: &mut egui::Ui, frame: &mut Frame)`を持ち(標準的な`update(&mut self, ctx: &Context, ...)`ではない)、`egui::TopBottomPanel::show`/`egui::CentralPanel::show`はいずれも`(self, ui: &mut Ui, add_contents)`を受け取る(`&Context`ではない)。既存の`main.rs`の`egui::CentralPanel::default().show(ui, |ui| {...})`という呼び出し方をそのまま踏襲すること。

- [ ] **Step 1: importを追加する**

`crates/gui/src/main.rs`冒頭を以下に置き換える:

```rust
use std::env;
use std::path::{Path, PathBuf};

use easy_archive_core::auto;
use easy_archive_core::integration;
use winit::platform::x11::EventLoopBuilderExtX11;
```

- [ ] **Step 2: `App`構造体を統合状態を持つ形に変更する**

```rust
#[derive(Default)]
struct App {
    status: String,
}
```

を以下に置き換える:

```rust
struct App {
    status: String,
    integration_installed: bool,
}

impl Default for App {
    fn default() -> Self {
        Self {
            status: String::new(),
            integration_installed: check_integration_installed(),
        }
    }
}
```

- [ ] **Step 3: 統合設置の判定・実行ヘルパー関数を追加する**

`setup_japanese_font`関数の下、`#[derive(Default)]`(Step 2で書き換えた)の上に追加する:

```rust
/// `install-integration`同様、`$HOME`と自実行ファイルのパスを解決する。
/// GUIプロセス自身の実行ユーザー権限で動くため、CLIの`install-integration`
/// と異なりpostinst(root権限)経由では不可能だった統合設置をここで安全に
/// 行える。
fn resolve_home_and_binary() -> Result<(PathBuf, String), String> {
    let home = env::var("HOME")
        .map(PathBuf::from)
        .map_err(|_| "HOME環境変数が設定されていません".to_string())?;
    let binary_path = env::current_exe()
        .map_err(|e| format!("実行ファイルのパスを取得できませんでした: {e}"))?
        .to_string_lossy()
        .into_owned();
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
            Ok(written) => format!("ファイルマネージャー統合を設置しました({}件)", written.len()),
            Err(e) => format!("エラー: {e}"),
        },
        Err(e) => format!("エラー: {e}"),
    }
}
```

- [ ] **Step 4: `ui`メソッドにバナーを追加する**

`impl eframe::App for App`ブロック内の`fn ui`を以下に置き換える(ドロップ処理・`CentralPanel`部分は変更しない):

```rust
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

        if !self.integration_installed {
            egui::TopBottomPanel::top("integration_banner").show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label("ファイルマネージャーの右クリックメニューにEasy Archiveを追加できます。");
                    if ui.button("設置する").clicked() {
                        self.status = install_integration();
                        self.integration_installed = check_integration_installed();
                    }
                });
            });
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
```

- [ ] **Step 5: ビルドする**

```bash
cargo build -p easy-archive-gui 2>&1 | tail -60
```

Expected: 警告・エラーなしでビルド成功。もしコンパイルエラーが出た場合(このワークスペースの`eframe`/`egui`が別バージョンに変わっていた場合など)は、エラーメッセージに従って`TopBottomPanel::show`の引数(`ui`か`ui.ctx()`か)を実際のAPIに合わせて修正する — これはこのタスクの想定内の作業であり、設計のやり直しではない。

- [ ] **Step 6: 手動で起動確認する**

```bash
HOME=/tmp/deb-gui-check cargo run -p easy-archive-gui &
```

ウィンドウが起動したら以下を目視確認する:

1. `/tmp/deb-gui-check`は空(統合未設置)の状態なので、上部に「ファイルマネージャーの右クリックメニューにEasy Archiveを追加できます。」バナーと「設置する」ボタンが表示されること
2. 「設置する」ボタンを押すと、ステータス行に「ファイルマネージャー統合を設置しました(6件)」と表示され、バナーが消えること
3. ウィンドウを閉じて`cargo run -p easy-archive-gui`(同じ`HOME`)で再起動すると、既に設置済みのためバナーが表示されないこと

確認後、プロセスを終了する:

```bash
kill %1 2>/dev/null; rm -rf /tmp/deb-gui-check
```

(この手動確認はコミットの必須条件。GUIの自動テストは対象外という既存の設計判断を踏襲する。)

- [ ] **Step 7: コミットする**

```bash
git add crates/gui/src/main.rs
git commit -m "$(cat <<'EOF'
GUIにファイルマネージャー統合の設置ボタンを追加

起動時にintegration::is_installedで統合ファイルの設置状況を判定し、
未設置ならドロップ領域の上に案内バナーと「設置する」ボタンを表示する。
クリックするとintegration::install_allを同一プロセス内で直接呼び出し、
設置後は再判定してバナーを消す。

.debインストール後、postinst(root権限・$HOME不明のため統合設置が
自動化できない)に代わり、GUI起動→ボタン1クリックのみでファイル
マネージャー統合が完了する導線ができた。手動でのボタン表示/非表示
切り替えを目視確認済み。
EOF
)"
```

---

## Task 3: パッケージング用アセット(`.desktop`・postinst/postrm)を作成する

**Files:**
- Create: `packaging/easy-archive.desktop`
- Create: `packaging/debian/postinst`
- Create: `packaging/debian/postrm`

**Interfaces:**
- Consumes: `packaging/icons/easy-archive.svg`(既存、ADR 0006で作成済み)
- Produces: Task 4が`[package.metadata.deb]`の`assets`/`maintainer-scripts`から参照するファイル群

- [ ] **Step 1: `packaging/easy-archive.desktop`を作成する**

```ini
[Desktop Entry]
Type=Application
Name=Easy Archive
Comment=文字コードを自動判定するZIP解凍・圧縮ツール
Exec=easy-archive-gui %U
Icon=easy-archive
Terminal=false
Categories=Utility;Archiving;
MimeType=application/zip;
```

- [ ] **Step 2: `packaging/debian/`ディレクトリとpostinst/postrmを作成する**

```bash
mkdir -p packaging/debian
```

`packaging/debian/postinst`:

```sh
#!/bin/sh
set -e

if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database -q /usr/share/applications || true
fi

if command -v gtk-update-icon-cache >/dev/null 2>&1; then
    gtk-update-icon-cache -f -t /usr/share/icons/hicolor || true
fi

exit 0
```

`packaging/debian/postrm`:

```sh
#!/bin/sh
set -e

if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database -q /usr/share/applications || true
fi

if command -v gtk-update-icon-cache >/dev/null 2>&1; then
    gtk-update-icon-cache -f -t /usr/share/icons/hicolor || true
fi

exit 0
```

- [ ] **Step 3: postinst/postrmに実行権限を付与する**

```bash
chmod +x packaging/debian/postinst packaging/debian/postrm
```

(Debianのmaintainer scriptsは実行権限が必須。cargo-debはパッケージング時にファイルシステム上の権限をそのまま使う想定のため、ここで付与しておく。)

- [ ] **Step 4: 構文チェックする**

```bash
sh -n packaging/debian/postinst
sh -n packaging/debian/postrm
ls -l packaging/debian/
```

Expected: 構文エラーなし。`ls -l`で両ファイルに`x`権限が付いていることを確認する。

- [ ] **Step 5: コミットする**

```bash
git add packaging/easy-archive.desktop packaging/debian/postinst packaging/debian/postrm
git commit -m "$(cat <<'EOF'
.debパッケージ用のデスクトップエントリとmaintainer scriptsを追加

packaging/easy-archive.desktop(ランチャー用、Icon=easy-archive、
MimeType=application/zip)、packaging/debian/postinst・postrm
(update-desktop-database・gtk-update-icon-cacheの更新のみ)を新規
作成した。ファイルマネージャー統合の設置はここでは行わない(Task 2の
GUIボタンに委譲する設計のため)。cargo-debへの組み込みはTask 4で行う。
EOF
)"
```

---

## Task 4: `cargo-deb`メタデータを確定し、`packaging/build-deb.sh`でビルド一式を自動化する

**Files:**
- Modify: `crates/core/Cargo.toml`(`license = "MIT"`追加)
- Modify: `crates/gui/Cargo.toml`(`license = "MIT"`・`[package.metadata.deb]`追加)
- Create: `packaging/build-deb.sh`

**Interfaces:**
- Consumes: `packaging/easy-archive.desktop`・`packaging/debian/postinst`・`packaging/debian/postrm`(Task 3)
- Consumes: `packaging/icons/easy-archive.svg`(既存)

このTaskには実地検証が必要な箇所がある: `cargo-deb`の`assets`のソースパスが、`crates/gui/Cargo.toml`のあるディレクトリ(`crates/gui`)起点か、ワークスペースルート起点かは、このリポジトリでは未検証。以下の手順で実際に試し、動いた方の記法を採用すること。

- [ ] **Step 1: `license`フィールドを追加する**

`crates/core/Cargo.toml`と`crates/gui/Cargo.toml`の両方の`[package]`セクションに以下を追加する(`edition = "2024"`の下):

```toml
license = "MIT"
```

- [ ] **Step 2: `cargo-deb`をインストールする**

```bash
cargo deb --version || cargo install cargo-deb
cargo deb --version
```

Expected: バージョン番号が表示される。

- [ ] **Step 3: `[package.metadata.deb]`を追加する(1回目の試行)**

`crates/gui/Cargo.toml`の末尾に追加する:

```toml
[package.metadata.deb]
name = "easy-archive"
maintainer = "Wanyaldee <gooya.3322@gmail.com>"
copyright = "2026, Wanyaldee <gooya.3322@gmail.com>"
license-file = ["../../LICENSE", "0"]
extended-description = "文字コードを自動判定するZIP解凍・圧縮ツール。日本の学校・自治体などから届くShift-JISファイル名のZIPを文字化けさせずに扱えます。"
depends = "$auto"
section = "utils"
priority = "optional"
maintainer-scripts = "../../packaging/debian"
assets = [
    ["target/release/easy-archive-gui", "usr/bin/", "755"],
    ["target/release/easy-archive", "usr/bin/", "755"],
    ["../../packaging/easy-archive.desktop", "usr/share/applications/easy-archive.desktop", "644"],
    ["../../packaging/icons/easy-archive.svg", "usr/share/icons/hicolor/scalable/apps/easy-archive.svg", "644"],
]
```

- [ ] **Step 4: ワークスペース全体をビルドし、`cargo deb`を試す**

```bash
cargo build --release --workspace
cd crates/gui
cargo deb --no-build
cd ../..
```

- [ ] **Step 5: 失敗した場合はアセットパスを調整する**

`target/release/...`が見つからないエラーが出た場合、`assets`内の`target/release/...`の行を、ワークスペースルート起点のパス(`"../../target/release/easy-archive-gui"`のような形)に書き換えて再試行する。`../../packaging/...`の行が見つからないエラーが出た場合は、逆に`crates/gui`起点の相対パス(`packaging/`ではなく`../../packaging/`が正しい可能性、またはその逆)に調整する。実際にどちらの記法で成功したかを、この`[package.metadata.deb]`テーブルの直前にTOMLコメント(`#`)で1行残しておく(例: `# assetsのソースパスはcrates/gui(このCargo.tomlのあるディレクトリ)起点。cargo-deb 2.x系で実地検証済み`)。

- [ ] **Step 6: 生成された`.deb`の中身を検証する**

```bash
find target/debian -maxdepth 1 -name "*.deb"
DEB=$(find target/debian -maxdepth 1 -name "*.deb" | head -1)
dpkg-deb --info "$DEB"
dpkg-deb --contents "$DEB"
```

Expected: `--info`で`Package: easy-archive`・`Maintainer`・`Depends`(GTK/xcb関連ライブラリを含む)が表示される。`--contents`に以下が全て含まれる:
- `./usr/bin/easy-archive`
- `./usr/bin/easy-archive-gui`
- `./usr/share/applications/easy-archive.desktop`
- `./usr/share/icons/hicolor/scalable/apps/easy-archive.svg`

いずれか欠けている場合はStep 5に戻ってパスを調整する。

- [ ] **Step 7: `packaging/build-deb.sh`を作成する**

Step 2〜6で確定した実際の作業手順を、前提条件の確認・自動セットアップ込みでスクリプト化する:

```bash
#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

confirm() {
    local prompt="$1"
    read -r -p "$prompt [y/N] " reply
    case "$reply" in
        [yY]|[yY][eE][sS]) return 0 ;;
        *) return 1 ;;
    esac
}

if ! command -v cargo >/dev/null 2>&1; then
    echo "Rustツールチェーン(cargo)が見つかりません。"
    if confirm "rustupで自動インストールしますか?"; then
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
        # shellcheck source=/dev/null
        source "$HOME/.cargo/env"
    else
        echo "rustupを手動でインストールしてから再実行してください: https://rustup.rs/"
        exit 1
    fi
fi

if ! cargo deb --version >/dev/null 2>&1; then
    echo "cargo-debが見つかりません。インストールします。"
    cargo install cargo-deb
fi

REQUIRED_APT_PACKAGES="libgtk-3-dev libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev libxkbcommon-dev"
MISSING_APT_PACKAGES=""
for pkg in $REQUIRED_APT_PACKAGES; do
    if ! dpkg -s "$pkg" >/dev/null 2>&1; then
        MISSING_APT_PACKAGES="$MISSING_APT_PACKAGES $pkg"
    fi
done

if [ -n "$MISSING_APT_PACKAGES" ]; then
    echo "GUIビルドに必要なパッケージが不足しています:$MISSING_APT_PACKAGES"
    if confirm "sudo apt install で自動インストールしますか?"; then
        sudo apt-get update
        sudo apt-get install -y $MISSING_APT_PACKAGES
    else
        echo "手動でインストールしてから再実行してください。"
        exit 1
    fi
fi

echo "ビルドしています..."
cargo build --release --workspace
(cd crates/gui && cargo deb --no-build)

echo "完了しました。生成された.debファイル:"
find target/debian -maxdepth 1 -name "*.deb"
```

(Step 5でアセットパスの記法を`crates/gui`起点からワークスペースルート起点に変えた場合でも、このスクリプト内の`cd crates/gui && cargo deb --no-build`という実行方法自体は変わらない。パスの解決はあくまで`assets`欄の記法の問題であることに注意する。)

- [ ] **Step 8: 実行権限を付与し、構文チェックする**

```bash
chmod +x packaging/build-deb.sh
bash -n packaging/build-deb.sh
```

- [ ] **Step 9: スクリプトを実際に実行して確認する**

この開発環境には既に`cargo`・GUIビルド依存のaptパッケージが揃っているため、確認プロンプト付きの分岐(rustup自動インストール・`sudo apt install`)には到達せず、素通りしてビルドまで完走するはずである。`cargo-deb`は前のStepで既にインストール済みならそのメッセージも出ない。

```bash
./packaging/build-deb.sh
```

Expected: 「不足しています」等のメッセージが出ずに(全て揃っているため)、最後に`.deb`ファイルのパスが1行表示されて終了する。`sudo`を要する分岐が実際に実行されないことをこの実行結果で確認する。

- [ ] **Step 10: コミットする**

```bash
git add crates/core/Cargo.toml crates/gui/Cargo.toml packaging/build-deb.sh Cargo.lock
git commit -m "$(cat <<'EOF'
cargo-debメタデータとbuild-deb.shを追加し.debビルドを自動化

crates/gui/Cargo.tomlにpackage.metadata.debを追加(depends = "$auto"
でGTK/xcb系ランタイム依存を自動検出、Task 3で作成したdesktop/icon/
maintainer-scriptsを組み込み)。crates/core・crates/gui双方にlicense
フィールドを追加。

packaging/build-deb.shで、rustup/cargo-deb/aptビルド依存の有無を
確認し、無ければ確認の上で自動セットアップしてから.debをビルドする
一連の流れを1本化した。この開発環境では全ての前提条件が既に揃って
いたため、実行時に実際にビルドが完走することを確認済み。

dpkg-deb --contents/--infoで、生成された.debにeasy-archive・
easy-archive-gui両バイナリ、desktopファイル、アイコンが正しいパスで
含まれることを確認済み。
EOF
)"
```

---

## Task 5: ドキュメントを更新する(README・ADR 0007)

**Files:**
- Modify: `README.md`
- Create: `docs/adr/0007-deb-packaging.md`

**Interfaces:**
- Consumes: Task 1〜4で確定した実際の挙動・コマンド(このTaskはコード変更を伴わない)

- [ ] **Step 1: READMEに「インストール(.debパッケージ)」節を追加する**

`README.md`の「### ビルド・実行」節の直前に、以下のセクションを追加する:

```markdown
### インストール(.debパッケージ)

Ubuntu/Zorin OSでは、ビルド済みの`.deb`ファイルをファイルマネージャーで**ダブルクリック**すればGUIのソフトウェアインストーラーが起動し、インストールできます(ターミナル操作は不要です)。インストール後、GUI(Easy Archive)を起動すると、初回のみ画面上部に「ファイルマネージャーの右クリックメニューにEasy Archiveを追加できます。」というバナーが表示されるので、「設置する」ボタンを押すとNautilus/Nemo/Thunar/Dolphin/PCManFM-Qtへの右クリックメニュー統合が完了します。

開発者・貢献者が`.deb`をビルドする場合:

```sh
./packaging/build-deb.sh
```

Rust未導入の環境でも、rustup/`cargo-deb`/GUIビルド用aptパッケージの不足を検知し、確認の上で自動セットアップしてからビルドします。生成された`.deb`は`target/debian/`以下に出力されます。

(ターミナルでのインストール/アンインストールは`sudo dpkg -i target/debian/easy-archive_*.deb` / `sudo apt-get remove easy-archive`でも可能です。)
```

- [ ] **Step 2: `docs/spec.md`のマイルストーン5を完了として更新する**

`docs/spec.md`の

```
5. `.deb`/AppImageパッケージング
```

を以下に置き換える:

```
5. `.deb`パッケージング — 完了。`cargo-deb`(`crates/gui/Cargo.toml`の`[package.metadata.deb]`)で`easy-archive`・`easy-archive-gui`両バイナリ・`.desktop`・アイコンを1つの`.deb`にまとめた。ファイルマネージャー統合の自動設置はpostinst(root権限・`$HOME`不明のため不可能)ではなく、GUI起動時に表示する設置ボタンから行う設計にした(詳細は[ADR 0007](./adr/0007-deb-packaging.md)を参照)。`packaging/build-deb.sh`でRust未導入環境でも確認の上で自動セットアップしてビルドできる。AppImageは当面の対象外(スコープ外、将来必要になれば別途検討)
```

- [ ] **Step 3: `writing-adrs`スキルを使ってADR 0007を作成する**

`docs/adr/0007-deb-packaging.md`を、既存のADR(特に`docs/adr/0006-app-icon-design.md`)と同じ節構成(ステータス/背景/決定/既知の限界/影響)で作成する。以下を必ず含める:

- **背景**: マイルストーン5の目的、ADR 0006で準備したアイコンをここで実際に使うこと
- **決定**: AppImageではなく`.deb`(`cargo-deb`)を選んだこと。ファイルマネージャー統合をpostinstではなくGUI起動時のボタンにした理由(root権限・`$HOME`不明という技術的制約と、非エンジニアの利用者にターミナル操作をさせないという製品方針の両方から)。Task 4のStep 5で確定した`cargo-deb`のアセットパス解決の実際の挙動(実地検証結果)
- **既知の限界**: 実機(Zorin OS等)での`.deb`インストール・GUI起動・統合ボタンの動作は未検証(ADR 0004/0005と同様、実機検証待ちとして明記する)。AppImageは対象外
- **影響**: 新規作成した`packaging/easy-archive.desktop`・`packaging/debian/`・`packaging/build-deb.sh`・`crates/core`と`crates/gui`の`Cargo.toml`変更点を列挙する

- [ ] **Step 4: コミットする**

```bash
git add README.md docs/spec.md docs/adr/0007-deb-packaging.md
git commit -m "$(cat <<'EOF'
.debパッケージングのREADME/仕様書/ADRを更新

READMEに「インストール(.debパッケージ)」節を追加し、ダブルクリック
インストール→GUI起動時のボタンでの統合設置、という2ステップのみで
完結する導線を明記した(ターミナル操作は開発者向け補足に格下げ)。

docs/spec.mdのマイルストーン5を完了として記録。docs/adr/0007-deb-
packaging.mdに、AppImageではなくcargo-debを選んだ経緯、統合設置を
postinstではなくGUIボタンにした技術的・製品的な理由、cargo-debの
アセットパス解決の実地検証結果を記録した。
EOF
)"
```

---

## Task 6(任意/ストレッチ): CIで`.deb`ビルドを検証する

**Files:**
- Modify: `.github/workflows/ci.yml`

**Interfaces:**
- Consumes: `packaging/build-deb.sh`(Task 4)

このTaskは計画時点で「任意」として承認されている。リリース・公開は範囲外(要望されていないため)、ビルドが通ることの検証のみを行う。

- [ ] **Step 1: CIワークフローに`.deb`ビルドジョブを追加する**

`.github/workflows/ci.yml`の既存`test`ジョブの下に、新しいジョブを追加する:

```yaml
  deb-package:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Install GUI build dependencies
        run: sudo apt-get update && sudo apt-get install -y libgtk-3-dev libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev libxkbcommon-dev
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo install cargo-deb
      - run: cargo build --release --workspace
      - run: (cd crates/gui && cargo deb --no-build)
      - name: Verify .deb contents
        run: |
          DEB=$(find target/debian -maxdepth 1 -name "*.deb" | head -1)
          dpkg-deb --info "$DEB"
          dpkg-deb --contents "$DEB" | tee /tmp/deb-contents.txt
          grep -q "usr/bin/easy-archive$" /tmp/deb-contents.txt
          grep -q "usr/bin/easy-archive-gui$" /tmp/deb-contents.txt
          grep -q "usr/share/applications/easy-archive.desktop$" /tmp/deb-contents.txt
          grep -q "usr/share/icons/hicolor/scalable/apps/easy-archive.svg$" /tmp/deb-contents.txt
```

(GitHub Actionsのランナーは`dtolnay/rust-toolchain@stable`でRustを導入するため、`build-deb.sh`のrustup自動インストール分岐は経由しない。CIでは`cargo install cargo-deb`とビルドコマンドを直接呼ぶ。)

- [ ] **Step 2: ローカルでYAML構文を確認する**

```bash
python3 -c "import yaml; yaml.safe_load(open('.github/workflows/ci.yml'))" 2>&1 || echo "PyYAML未導入の場合はこのチェックをスキップしてよい"
```

- [ ] **Step 3: コミットする**

```bash
git add .github/workflows/ci.yml
git commit -m "$(cat <<'EOF'
CIに.debビルド検証ジョブを追加

既存のtestジョブに加え、deb-packageジョブでcargo-debによる.deb
ビルドとdpkg-deb --contentsでの中身検証(両バイナリ・desktop・
アイコンの存在確認)をCI上でも行うようにした。リリース・公開は
対象外で、ビルドが通ることの検証のみ。
EOF
)"
```

---

## Self-Review (このプランを書いた本人による確認用メモ)

- **spec.mdカバレッジ**: 統合設置ロジックの共通化(Task1)、GUIからのターミナル不要な設置導線(Task2)、`.desktop`/postinst/postrm(Task3)、`cargo-deb`メタデータとビルド自動化(Task4)、ドキュメント(Task5)、CI検証(Task6・任意)の全項目に対応するタスクがある
- **型/シグネチャの一貫性**: `install_all(home: &Path, binary_path: &str) -> Result<Vec<PathBuf>, Box<dyn Error>>`と`is_installed(home: &Path, binary_path: &str) -> Result<bool, Box<dyn Error>>`はTask1で定義し、Task2のGUIヘルパー関数から同一シグネチャで呼び出している
- **API実地確認**: `eframe::App::ui`と`egui::TopBottomPanel::show`/`CentralPanel::show`の実際のシグネチャ(`&mut Ui`を受け取る、`&Context`ではない)は、このリポジトリにインストール済みの`eframe 0.36.1`/`egui 0.36.1`のソースコードを直接読んで確認済み(ADR 0004/0005の一次情報確認の前例に倣った)。Task2のコードはこの実際のAPIに基づく
- **実地検証が必要な箇所の明示**: Task4の`cargo-deb`アセットパス解決のみ、このリポジトリでの実績がなく事前に断定できないため、Step 5で試行錯誤の手順を明示し、確定した記法をコメントとして残すよう指示した
- **sudo/破壊的操作の扱い**: どのTaskの自動実行ステップも`sudo`コマンドを実際には実行しない(この開発環境には既に全依存関係が揃っているため、`build-deb.sh`の確認分岐がすべてスキップされる経路で完走する)。実機での`sudo dpkg -i`によるインストール確認はこのplanの範囲外とし、ADR 0007に「実機検証待ち」として明記する
- **プレースホルダ**: 「TODO」「後で実装」等の記述はない。Task4のStep5のみ、実地確認の結果によって最終的な記法を選ぶ設計上必然な分岐であり、コード自体に未実装箇所は残さない
