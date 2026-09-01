# 0007. `.deb`パッケージングとファイルマネージャー統合の設置方式

## ステータス

Accepted（`packaging/build-deb.sh`でのローカルビルド・`dpkg-deb --info`/`--contents`による内容検証までは完了。実機（Zorin OS等）での`.deb`インストール・GUI起動・統合設置ボタンの動作は、ADR 0004/0005と同様に実機検証待ち）

## 背景

- マイルストーン5の目的は、これまで`cargo run`でしか実行できなかったEasy Archiveを、CLAUDE.mdが想定する対象ユーザー（日本の学校・自治体の非エンジニア職員）がファイルマネージャーからダブルクリックでインストールできる配布形態にすること。
- ADR 0006で作成した`packaging/icons/easy-archive.svg`（ZIPバッジ案）は、そのADR時点では「`.desktop`ファイル・`cargo-deb`への組み込みはまだ着手していない」状態だった。本マイルストーンで初めて、`.desktop`の`Icon=`と`cargo-deb`の`assets`を通じて実際に配布物へ組み込む。
- マイルストーン4（ADR 0005）で実装した`easy-archive install-integration`（ファイルマネージャー右クリックメニュー設置）は、ターミナルから手動でCLIサブコマンドを叩く前提だった。`.deb`インストール後にこれをどう自動化するか（あるいは自動化しないか）が、本マイルストーンで新たに解決すべき課題として残っていた。

## 決定

### 1. AppImageではなく`cargo-deb`による`.deb`のみを採用する

`docs/spec.md`の技術スタック欄はもともと「`.deb` / AppImage」の両方を候補として残していたが、実装では`.deb`のみを作り、AppImageは当面の対象外とした。

CLAUDE.mdは「対応フォーマットは当面ZIPのみ、対象OSはUbuntu系（Zorin OS含む）のみ。他フォーマット・他OS対応をついでに広げない」とスコープを固定している。AppImageの主な利点は「distro非依存の単一実行ファイルとして動く」ことだが、対象OSがUbuntu系（dpkg/apt採用ディストリ）一本に絞られている以上、その利点はここでは効かない。一方`cargo-deb`は`crates/gui/Cargo.toml`の`[package.metadata.deb]`をそのまま読み、`cargo metadata`のtarget_directory経由でビルド成果物を自動的に解決する（実地検証は後述の3.）ため、Rustの既存ビルドパイプラインに`appimagetool`や自作`AppRun`スクリプトのような外部ツール・仕組みを追加する必要がない。CLAUDE.mdのYAGNI方針（新規依存を追加する前にスコープが本当に必要としているか確認する）に照らし、当面はUbuntu系一本化に対して`.deb`のみで十分と判断した。AppImageは`docs/spec.md`に「将来必要になれば別途検討」として明記し、選択肢として残す。

**注記**: この決定は、ADR 0004（`winit`のソースコードを直接読んで原因を特定）やADR 0005（各ファイルマネージャーの公式リポジトリ・一次情報を確認）のような、AppImage自体を対象にした専用の実地調査を経たものではない。上記の理由づけは、CLAUDE.mdに既に書かれているスコープ方針（対象OSをUbuntu系のみに固定する）から導いた妥当な帰結であり、`appimagetool`でのビルド試行やAppImage形式そのものの技術的な検証は行っていない。将来AppImageが必要になった場合は、この帰結ベースの判断ではなく、ADR 0004/0005と同様の実地調査を伴う別ADRとして再検討すべきである。

### 2. ファイルマネージャー統合の自動設置は postinst ではなく GUI起動時のボタンにする

`crates/gui/src/main.rs`の実装:

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
```

`App::default()`が起動時に`check_integration_installed()`を呼び、未設置なら`egui::Panel::top`でバナーと「設置する」ボタンを表示する。ボタン押下で`integration::install_all(&home, &binary_path)`（Task 1で`crates/core/src/integration/mod.rs`にCLI/GUI共用ライブラリ関数として抽出済み）を呼び、成功後は`check_integration_installed()`を再実行してバナーを消す。

理由は技術的制約と製品方針の両方から来ている。

- **技術的制約**: Debianのmaintainer script（`postinst`）は`dpkg`インストール中に**root権限**で実行される。`install_all`は`~/.local/share/nautilus/scripts/...`や`~/.config/Thunar/uca.xml`のような、実際にファイルマネージャーを使う一般ユーザーのホームディレクトリへ書き込む設計（ADR 0005）。root権限で動くpostinstの中では`$HOME`は`/root`（あるいは呼び出し経路次第で不定）であり、「今このPCでファイルマネージャーを使っているユーザー」のホームを確実に特定する手段がない。`/etc/passwd`の全ユーザー走査や`SUDO_USER`環境変数に頼る方法もあるが、共有PC（学校の複数アカウント環境）やGUIソフトウェアインストーラ経由（`sudo`を介さない）でのインストールでは信頼できない。対してGUIアプリ本体は一般ユーザー権限のプロセスとして起動するため、`env::var("HOME")`は常に「今使っている人」のホームを指す。上記コードのコメントはこの理由を`resolve_home_and_binary`のドキュメントコメントとして明記している。
- **製品方針**: README「インストール(.debパッケージ)」節に明記の通り、対象ユーザー（非エンジニアの教員・自治体職員）にターミナル操作をさせないという方針がある。`.deb`をダブルクリックでインストールし、GUIを起動してボタンを1回押すだけで、CLIコマンド（`easy-archive install-integration`）を意識させずに統合設置まで完結する。

Task 1で`install_all`/`is_installed`をCLI専用コードから`crates/core`のライブラリ関数として抽出したのは、この設計（CLIの`install-integration`とGUIのボタンが同じロジックを呼ぶ）を成立させるための下準備だった。

### 3. `cargo-deb`のアセットパス解決（実地検証結果）

`crates/gui/Cargo.toml`の`[package.metadata.deb]`:

```toml
# assetsのソースパスは2種類の起点が混在する: "target/release/..."はcargo-debが
# `cargo metadata`のtarget_directory(常にワークスペースルートのtarget/)経由で
# 解決するため、このCargo.tomlのディレクトリ(crates/gui)を起点とした文字通りの
# 相対パスではない(crates/gui/targetは存在しない)。一方"../../packaging/..."の
# 静的アセットはCargo.tomlのあるディレクトリ(crates/gui)を起点とした文字通りの
# 相対パスとして解決される(ワークスペースルート起点にすると見つからない)。
# 1回目の試行(このtarget/release起点+../../packaging起点の組み合わせ)で
# `cargo deb --no-build`が成功することをcargo-deb 3.7.0で実地検証済み。
maintainer-scripts = "../../packaging/debian"
assets = [
    ["target/release/easy-archive-gui", "usr/bin/", "755"],
    ["target/release/easy-archive", "usr/bin/", "755"],
    ["../../packaging/easy-archive.desktop", "usr/share/applications/easy-archive.desktop", "644"],
    ["../../packaging/icons/easy-archive.svg", "usr/share/icons/hicolor/scalable/apps/easy-archive.svg", "644"],
]
```

`target/release/...`と`../../packaging/...`という起点の異なる2種類のパス表記が同じ`assets`配列に混在しているのは矛盾ではない。`target/release/...`で始まるパスは`cargo-deb`が特別扱いし、`cargo metadata`が返すワークスペース共通の`target_directory`（本ワークスペースでは常にリポジトリルート直下の`target/`。`crates/gui/target`は存在しない）を起点に解決する。一方それ以外の静的アセットパスは、Cargo.tomlが置かれているディレクトリ（`crates/gui`）を起点とした文字通りの相対パスとして解決される。この2種類の起点混在に気づかず「`crates/gui/target/release/...`が存在しないから壊れているはず」と誤って`../../target/release/...`のように書き換えると、今度は本当に解決に失敗する。`cargo build --release --workspace`後に`cd crates/gui && cargo deb --no-build`を実行したところ、パス調整なしの1回目の試行でそのまま成功し、`dpkg-deb --contents`で`./usr/bin/easy-archive`・`./usr/bin/easy-archive-gui`・`./usr/share/applications/easy-archive.desktop`・`./usr/share/icons/hicolor/scalable/apps/easy-archive.svg`の4パスすべてが期待通りに含まれることを確認した。

`depends = "$auto"`の実際の出力は`Depends: libc6 (>= 2.34), libc6 (>= 2.39)`のみだった（`ldd target/release/easy-archive-gui`でも`libgcc_s.so.1`・`libm.so.6`・`libc.so.6`・`ld-linux`のみが確認され、GTK/XCB系の共有ライブラリは動的リンクされていない）。この事実の技術的な理由と、それが意味する既知の限界は次節にまとめる。

## 既知の限界

- **実機未検証**: `.deb`をZorin OS等の実機でダブルクリックインストール→GUI起動→初回バナー表示→「設置する」ボタン→実際のファイルマネージャー右クリックメニューへの反映、という一連の流れはまだ確認していない。ADR 0004/0005と同様、実機検証待ちとして扱う。GUI側のバナー/ボタン配線自体は、`$HOME`環境変数を差し替えたユニットテスト（`integration_helpers_reflect_install_state_via_home_env`、`crates/gui/src/main.rs`）で「設置前はis_installed=false・ボタン押下でinstall_all実行・設置後はtrue」という状態遷移のみ検証済みで、実際のレンダリング結果（バナーの見た目、日本語表示の崩れの有無）は未確認。
- **AppImageは対象外**（決定1参照）。`docs/spec.md`に将来の再検討候補として明記してある。
- **`depends = "$auto"`はdlopenベースの実行時依存を検出できない**: `cargo-deb`の`depends = "$auto"`は`dpkg-shlibdeps`（ELFの`DT_NEEDED`＝ビルド時に動的リンクした共有ライブラリを見る仕組み）に基づいており、実行時に`dlopen(3)`で読み込むライブラリは原理的に検出対象外。ビルドした`easy-archive-gui`バイナリを`strings`で確認したところ、`winit`の`x11`フィーチャー（`x11-dl`/`x11rb`クレート経由）は`libX11.so.6`・`libX11-xcb.so.1`・`libXcursor.so.1`・`libXi.so.6`・`libxcb.so.1`・`libxkbcommon.so.0`・`libxkbcommon-x11.so.0`という7つのライブラリ名を`dlopen`対象として埋め込んでいるが、ELFレベルでは`libc.so.6`/`libm.so.6`/`libgcc_s.so.1`以外を一切リンクしていない。そのため生成された`.deb`の`Depends:`は`libc6`のみで、GUIアプリから素朴に期待されるGTK/XCB系ライブラリへの依存が一切現れない。これはバグではなく`dpkg-shlibdeps`の仕組み上の限界であり、`cargo-deb`側で回避する手段もない。実害としては、これら7ライブラリはX11セッション（またはXWayland）を持つデスクトップLinux環境であれば、Xサーバー本体や任意のデスクトップ環境自体がすでに依存しているため、通常のUbuntu系デスクトップ環境に別途インストールが必要になる場面は考えにくいと判断し、`Depends`を手動で補完する対応は見送った。ただし「`.deb`だけを最小構成のサーバー用イメージ等に単体インストールする」ような非デスクトップ環境では、これらのライブラリが不足してGUIが起動しない可能性がある（本ツールの対象は学校・自治体のデスクトップPCのみのため、現時点では許容範囲とする）。

## 影響

- 新規作成: `packaging/easy-archive.desktop`（ランチャー用Desktop Entry、`Icon=easy-archive`・`MimeType=application/zip;`）、`packaging/debian/postinst`・`packaging/debian/postrm`（`update-desktop-database`・`gtk-update-icon-cache`の更新のみ。ファイルマネージャー統合の設置はここでは行わず、決定2の通りGUIボタンに委譲）、`packaging/build-deb.sh`（rustup/`cargo-deb`/GUIビルド用aptパッケージの不足を検知し、確認の上で自動セットアップしてから`cargo build --release --workspace`→`cargo deb --no-build`を実行する）。
- `crates/core/Cargo.toml`・`crates/gui/Cargo.toml`: `[package]`に`license = "MIT"`を追加（`cargo-deb`が`license-file`と併せて要求する）。`crates/gui/Cargo.toml`に`[package.metadata.deb]`を新規追加（内容は決定3参照）。
- `crates/core/src/integration/mod.rs`: Task 1で`install_all`/`is_installed`をCLI専用コードからライブラリ関数として抽出し、Task 2でGUIの設置ボタンから直接呼べるようにした（決定2の前提）。
- `crates/gui/src/main.rs`: `App`に`integration_installed`フィールドと、起動時バナー（`egui::Panel::top`）・「設置する」ボタンを追加した。
- README.mdに「インストール(.debパッケージ)」節を、`docs/spec.md`のマイルストーン5をそれぞれ本ADRの内容に合わせて更新した。
- **配布チャネル**: `packaging/build-deb.sh`（またはマイルストーン5のCI、Task 6で追加予定）が生成する`.deb`は、正式リリース時にはGitHub Releasesへ添付して配布する想定（プロジェクトオーナーの方針）。Task 6で追加予定のCIジョブはビルド検証（`.deb`が壊れずビルドできることの確認）のみをスコープとし、GitHub Releasesへのアップロード自動化は本マイルストーンの対象外。将来アップロード自動化を行う場合は別途Issue/ADRで扱う。
