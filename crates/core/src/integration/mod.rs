//! 主要ファイルマネージャーへの右クリックメニュー統合。
//!
//! 各ファイルマネージャー(Nautilus/Nemo/Thunar/Dolphin/PCManFM-Qt)は右クリック
//! メニューへのカスタム項目追加の仕組みがそれぞれ異なる。調査結果と技術的決定は
//! [ADR 0005](../../../../docs/adr/0005-file-manager-integration-mechanisms.md)
//! を参照。
//!
//! すべてのファイルマネージャー統合は、生成するコマンドのExec行を
//! `<easy-archiveバイナリのパス> auto %f`(各DEのプレースホルダ記法に読み替え)
//! に統一する。これにより、「ここに解凍」「ここを圧縮」のどちらのメニュー項目
//! から呼ばれても`auto`サブコマンド(`crate::auto::auto`)がパスの実体を見て
//! 正しい処理を選ぶため、DE側のメニュー表示条件(拡張子フィルタ)の精度は
//! 正確性に影響しない。

pub mod dolphin;
pub mod nautilus;
pub mod nemo;
pub mod pcmanfm_qt;
pub mod thunar;

use std::path::PathBuf;

/// ファイルマネージャー統合のために生成する1ファイル分の情報。
/// `relative_path`はホームディレクトリからの相対パス。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneratedFile {
    pub relative_path: PathBuf,
    pub content: String,
    pub executable: bool,
}
