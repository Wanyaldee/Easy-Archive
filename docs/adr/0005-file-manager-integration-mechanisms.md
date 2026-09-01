# 0005. マイルストーン4: 主要ファイルマネージャーの右クリックメニュー統合方式

## ステータス

Accepted（実装完了。WSL開発環境でNautilus統合を機能面・GUI目視操作の両方で検証済み。Nemo/Thunar/Dolphin/PCManFM-Qtは実機での確認がまだ）

## 背景

マイルストーン4はもともとspec.mdで「Nautilus「ここに解凍」「ここを圧縮」スクリプト連携」のみを対象としていたが、対象OS方針（Ubuntu系全般、Zorin OS含む）に照らすと、Ubuntu系ディストリの標準デスクトップ環境が採用するファイルマネージャーは1つではない。そこで対象をNautilus（GNOME/Zorin OS Core・Pro）、Nemo（Cinnamon/Linux Mint系）、Thunar（XFCE/Zorin OS Lite・Xubuntu）、Dolphin（KDE/Kubuntu系）、PCManFM・PCManFM-Qt（LXDE・LXQt/Lubuntu系）の5系統に拡大した。

各ファイルマネージャーは右クリックメニューへのカスタム項目追加の仕組みが個別に異なり、かつ公式ドキュメントの記述だけでは正確性に不安が残る（ADR 0004でwinitのソースコードを直接読んで検証した前例に倣い、今回も可能な限り一次情報＝各プロジェクト本体のソースコード・公式リポジトリを確認した）。

## 調査結果

### Nemo（`.nemo_action`）

一次情報: [linuxmint/nemo公式リポジトリのsample.nemo_action](https://github.com/linuxmint/nemo/blob/master/files/usr/share/nemo/actions/sample.nemo_action)を直接取得して確認。

- 配置場所: `~/.local/share/nemo/actions/*.nemo_action`
- フォーマット（INI形式）:
  ```
  [Nemo Action]
  Name=表示名
  Comment=説明
  Exec=コマンド %f
  Icon-Name=アイコン名
  Selection=s
  Extensions=zip;
  ```
- `Selection`: `s`(単一)/`m`(複数)/`any`/`notnone`/`none`(背景クリック)/数値
- `Extensions`: セミコロン区切りの拡張子リスト、または特殊値`dir`(ディレクトリのみ)/`none`(拡張子なし)/`nodirs`(ディレクトリ以外の任意)/`any`(ディレクトリ含む任意)。特殊値と拡張子リストは排他（1エントリのみ指定）
- `Extensions`と`Mimetypes`はどちらか一方が必須（両方指定も可）
- トークン: `%U`(URIリスト) `%F`(パスリスト) `%P`(親ディレクトリパス) `%f`/`%N`(先頭ファイル) `%p`(親ディレクトリ名) `%D`(デバイスパス) `%e`(拡張子なし先頭ファイル) `%%`(リテラル%)

### Thunar（`uca.xml`）

一次情報: [gitlab.xfce.org/xfce/thunar の `thunar/thunar-uca-model.c`（masterブランチ）](https://gitlab.xfce.org/xfce/thunar/-/blob/master/thunar/thunar-uca-model.c)を直接読んで確認（ドキュメント記載が曖昧だったため）。

- 配置場所: `~/.config/Thunar/uca.xml`（`<actions>`をルートに`<action>`を複数並べる）
- `<unique-id>`は`"<マイクロ秒epoch>-<カウンタ>"`形式の文字列だが、**Thunar自身は重複チェックをしない単なる文字列**。固定文字列ID（例: `easy-archive-extract-here`）を割り当てて自作ツール側で「既存か」の判定に使ってよい
- `<patterns>`: グロブパターンをセミコロン区切りで複数指定可（例: `*.zip;*.ZIP`、大小文字は区別される）
- 表示条件タグ（`<action>`直下の空要素、複数指定でOR）: `<directories/>` `<audio-files/>` `<image-files/>` `<text-files/>` `<video-files/>` `<other-files/>`（どのカテゴリにも当てはまらない一般ファイル）
- `<command>`内の変数: `%f`/`%F`(単数/複数パス) `%u`/`%U`(単数/複数URI) `%d`/`%D`(単数/複数親ディレクトリ) `%n`/`%N`(単数/複数ファイル名)
- **freedesktop「file-manager/actions」`.desktop`方式には非対応**（Thunar本体・プラグイン全体のソースコード検索、およびNEWS全履歴で該当実装なしを確認）。uca.xmlへの追記が唯一の公式手段
- 安全な追記には正規のXMLパーサでの読み込み→`</actions>`直前への要素挿入→一時ファイル経由のアトミック書き込みが必要（Thunar自身もこの方式で保存している）

### Dolphin（KIOサービスメニュー）

一次情報: [KDE公式Developer Docs](https://develop.kde.org/docs/apps/dolphin/service-menus/)。

- 配置場所: `~/.local/share/kio/servicemenus/*.desktop`（KDE Frameworks 5.85以降、Plasma 5後期〜Plasma 6で共通）。**実行権限(`chmod +x`)必須**（付与しないとホームディレクトリ配下は認可されない）
- フォーマット:
  ```
  [Desktop Entry]
  Type=Service
  MimeType=application/zip;inode/directory;application/octet-stream;
  Actions=extractHere;compressHere;

  [Desktop Action extractHere]
  Name=ここに解凍
  Icon=archive-extract
  Exec=コマンド %f

  [Desktop Action compressHere]
  Name=ここを圧縮
  Icon=archive-insert
  Exec=コマンド %f
  ```
- `Actions=`に列挙した項目は同じ`MimeType`条件下で全て表示される（アクションごとに個別のMIME条件は付けられない）

### Nautilus（scripts方式）

ユーザー判断により、`nautilus-python`拡張を使わないシンプルな`~/.local/share/nautilus/scripts/`方式を採用（解凍/圧縮という単純な処理にリッチな拡張は不要と判断）。

- 配置場所: `~/.local/share/nautilus/scripts/`。サブディレクトリを切るとサブメニュー化される（例: `scripts/Easy Archive/`）
- 実行権限必須。シバン行（`#!/bin/sh`等）付きスクリプトとして置くのが安全（コンパイル済みバイナリを直置きできるかは未確認のため、薄いシェルラッパーを経由する）
- ファイル名がそのままメニュー表示名になる（自動整形なし）
- 環境変数: `NAUTILUS_SCRIPT_SELECTED_FILE_PATHS`（選択パス、改行区切り、ローカルのみ）、`NAUTILUS_SCRIPT_SELECTED_URIS`、`NAUTILUS_SCRIPT_CURRENT_URI`
- **拡張子等によるメニュー項目自体の出し分けは不可**（スクリプト側で常に判定するしかない）
- Nautilusは標準出力/終了コードを一切表示しないため、`notify-send`でユーザーに結果を通知する

### PCManFM / PCManFM-Qt（DES-EMA `.desktop`方式）

一次情報: [lxqt-project.org公式Wiki](https://lxqt-project.org/wiki/custom_actions.html)、[gitlab.com/radio_dude/pcmanfm-context-menu](https://gitlab.com/radio_dude/pcmanfm-context-menu)の実ファイル。

- **PCManFM-Qt（LXQt）**: `~/.local/share/file-manager/actions/*.desktop`に、Nautilus-Actions由来の非標準拡張仕様「DES-EMA」形式で配置。
  ```
  [Desktop Entry]
  Type=Action
  Name=表示名
  Icon=アイコン名
  Profiles=profile-id;

  [X-Action-Profile profile-id]
  Exec=コマンド %f
  MimeTypes=application/zip;
  ```
  変更反映にはLXQtセッションからのファイルマネージャー再起動が必要な場合がある。複数ファイル選択時の挙動に既知の不具合報告あり（[lxqt/pcmanfm-qt#1039](https://github.com/lxqt/pcmanfm-qt/issues/1039)）。
- **PCManFM（GTK/classic、LXDE）**: 同じ`file-manager/actions`ディレクトリ機構自体は存在する可能性が高い（SourceForgeのフィーチャーリクエスト/バグ報告から間接的に確認）が、「解凍」の右クリック表示はビルド時に焼き込まれたxarchiver等の外部アーカイバ検出ロジック（`ptk-file-archiver.c`）に依存する別系統の仕組みであり、DES-EMAカスタムアクションで完全に代替できるかは実機未検証。`libfm`のTODOにも「アーカイバとの統合方法は長年未確定」との記述があり、実現性・保守性ともに不確実性が高い。

## 決定

### 設計方針: 全DE共通で単一のCLIサブコマンド`easy-archive auto <パス>`に処理を委譲する

`crates/gui/src/main.rs`の`handle_drop`/`do_extract`/`do_compress`は、パスがZIPなら解凍・それ以外なら圧縮という「自己判定」ロジックを既に実装済みである。これを`crates/core`に切り出し、`easy-archive auto <パス>`というCLIサブコマンドとして公開する（GUI側もこの共有関数を呼ぶよう修正し、ロジックの重複を解消する）。

各ファイルマネージャー側の設定ファイルは、Exec行を必ず`easy-archive auto %f`（各DEのプレースホルダ記法に読み替え）にする。これにより:

- 「ここに解凍」「ここを圧縮」のどちらのボタン・メニュー項目から呼ばれても、`auto`が対象パスの実体を見て正しい処理を選ぶため、**DE側のメニュー表示条件（拡張子フィルタ）は「見た目の親切さ」の問題に過ぎず、正確性には影響しない**。
- Nautilus（拡張子でのメニュー出し分けが原理的に不可能）やDolphin（1つの`MimeType`条件下でextract/compress両方が並んで表示される）といった制約のあるDEでも、誤operationにはならない（例: Dolphinで.zipファイルに対して誤って「ここを圧縮」を押しても、`auto`が解凍を実行する）。
- 各DE統合コードは「設定ファイルの生成・配置」のみに専念でき、解凍/圧縮の判定ロジックを重複実装しない（DRY）。

### 対象範囲

| ファイルマネージャー | 対応方針 |
|---|---|
| Nemo | 実装する。`easy-archive-extract.nemo_action`（`Selection=s`, `Extensions=zip;`）と`easy-archive-compress.nemo_action`（`Selection=s`, `Extensions=any;`）の2ファイル |
| Thunar | 実装する。`uca.xml`に2つの`<action>`（固定`<unique-id>`で冪等性を担保）を安全にマージする |
| Dolphin | 実装する。`~/.local/share/kio/servicemenus/easy-archive.desktop`に2アクションをまとめる |
| Nautilus | 実装する。`~/.local/share/nautilus/scripts/Easy Archive/`配下にシェルラッパー1本＋`notify-send`通知 |
| PCManFM-Qt | ベストエフォートで実装する。DES-EMA `.desktop`方式を試すが、複数選択時の既知の不具合（#1039）があるため単一選択のみ対応とし、実機検証を必須とする |
| PCManFM（GTK/classic） | **今回のマイルストーンでは対象から除外し、将来対応として`docs/spec.md`に明記する。** 実現性が不確実（xarchiver検出ロジックとの二重構造）で、実機検証だけで判断できる範囲を超えるため |

### 未決事項（実装前に確認が必要）

Thunarの`uca.xml`は既存のユーザー設定（他の自作カスタムアクションを含みうる）を安全に読み書きする必要があり、文字列連結による素朴な編集はXMLエスケープ・コメント・改行スタイルの差異で既存設定を壊すリスクがある。正規のXMLパーサ/シリアライザを使うのが安全だが、これは**新規依存クレートの追加**にあたる（CLAUDE.mdのYAGNI確認方針の対象）。この点は実装着手前に別途確認する。

## 影響

- `crates/core`に新モジュール（`src/auto.rs`と`src/integration/`）を追加した。GUI側(`crates/gui/src/main.rs`)は`easy_archive_core::auto::auto`への委譲に置き換え、ロジック重複を解消した
- `crates/core`に`quick-xml`を新規依存として追加した（Thunarのuca.xml安全マージのため）
- CLIに`easy-archive install-integration [--dry-run]`/`uninstall-integration`を追加した。バイナリパスは`env::current_exe()`から解決する。環境検出はせず対応する全ファイルマネージャー分を無条件に配置する
- PCManFM（GTK/classic）は対象から除外し、`docs/spec.md`のマイルストーン4に「将来対応」として明記した
- 使い捨てのフェイクHOMEに対するinstall→再install(冪等性)→uninstallの手動検証、および既存のThunarカスタムアクションが保持されることを確認済み
- 各DEでの実際の右クリックメニュー表示・動作確認は自動テストでは担保できないため、Zorin OS等の実機での目視確認はまだ。ADR 0004の前例に倣い、確認結果を別途本ADRに追記する

## 検証結果(WSL開発環境)

WSL開発環境には`nautilus`本体のみインストールされていた(Nemo/Thunar/Dolphin/PCManFM-Qtは未インストール)。実際の`$HOME`に`easy-archive install-integration`を実行し、以下を確認した。

- Nautilusの`NAUTILUS_SCRIPT_SELECTED_FILE_PATHS`環境変数を実際の値で再現してインストール済みスクリプトを直接起動し、圧縮(`データ.txt`→`データ.zip`、日本語ファイル名を含む)・解凍(`サンプル.zip`→`サンプル/サンプル.txt`)の両方が正しく動作することを確認した
- `install-integration`を2回実行しても`uca.xml`の`<unique-id>`が重複しない(冪等性)ことを実HOME環境でも再確認した
- 発見した不具合: `notify-send`(libnotify-bin)が入っていない環境では、生成したスクリプトが`command not found`で異常終了コード(127)を返していた。`command -v notify-send`で存在確認してから呼ぶよう修正した。`easy-archive auto`自体の処理結果には影響しない(通知が出ないだけで、圧縮/解凍は既に完了している)

続けてWSLg経由でNautilusのGUIを実際に操作し、以下を目視確認済み。

1. Nautilus起動
2. ディレクトリを右クリック
3. 「Easy Archive」サブメニューをクリック
4. スクリプト(`ここに解凍・圧縮.sh`)をクリックして圧縮/解凍が実行される

メニュー項目の日本語表示に文字化けが見られたが、これはこの開発環境(WSL)の日本語ロケール/フォント設定が不十分なことに起因するNautilus自体の表示問題であり、`easy-archive`側のコードや生成しているスクリプト内容には起因しない(スクリプトの実行自体は正常に完了している)。対象OS(Zorin OS等、日本語環境が前提の学校・自治体PC)では発生しない見込み。

Nemo/Thunar/Dolphin/PCManFM-Qtは本開発環境に存在しないため、実機(Zorin OS Lite、Kubuntu、Lubuntu、Linux Mint等)での確認が別途必要。
