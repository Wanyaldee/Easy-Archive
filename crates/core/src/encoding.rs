//! ZIPエントリ名の文字コード判定・デコードロジック。
//!
//! 判定は必ずエントリ（ファイル名）単位で行う。アーカイブ全体で1回だけ
//! 判定して全エントリに同じ結果を適用する実装（rc-zipの方式。
//! docs/spec.md参照）は、複数言語のファイル名が混在するZIPで破綻するため
//! 採用しない。
//!
//! 当初`chardetng`による統計的判定を採用する方針だったが、実装時の
//! テストでファイル名程度の短いバイト列では信頼性が低いことが判明した
//! （例: 半角カナを含むShift-JISファイル名をBig5と誤判定する）。代わりに
//! `encoding_rs`でShift-JISとして実際にデコードを試み、エラーなく成功
//! すればShift-JISと確定する方式に変更した（詳細はdocs/spec.md参照）。
//! この方式は対象v1の3エンコーディング(UTF-8/Shift-JIS/CP437)という
//! 閉じた選択肢に対して決定的に働き、短い文字列でも安定する。

use encoding_rs::SHIFT_JIS;

/// エントリ名のデコードに実際に使用した文字コード（CLI表示用）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EncodingUsed {
    Utf8,
    ShiftJis,
    Cp437,
}

impl EncodingUsed {
    pub fn label(self) -> &'static str {
        match self {
            EncodingUsed::Utf8 => "UTF-8",
            EncodingUsed::ShiftJis => "Shift-JIS",
            EncodingUsed::Cp437 => "CP437",
        }
    }
}

/// ZIPエントリの生のファイル名バイト列から文字コードを判定し、デコードする。
///
/// `utf8_flag_set` は ZIP general purpose bit flag のbit 11
/// (Language Encoding Flag)。立っていれば仕様上ファイル名はUTF-8で確定して
/// いるため判定処理自体を行わない。
///
/// 立っていない場合、生バイト列を実際にShift-JISとしてデコードしてみて、
/// 不正なバイト列（エラー）が無ければShift-JISと確定する。エラーが出た
/// 場合のみCP437として扱う（DOSベースの旧来ZIPの既定エンコーディング）。
/// 対象ユーザー（日本の学校・自治体）が本物のCP437(DOS罫線文字)ファイル名
/// を送ってくることは実質的に無いため、Shift-JISとして解釈可能な文字列は
/// 常にShift-JIS優先で確定させるこの方式が実用上正しい。
pub fn decode_entry_name(raw: &[u8], utf8_flag_set: bool) -> (String, EncodingUsed) {
    if utf8_flag_set {
        return (String::from_utf8_lossy(raw).into_owned(), EncodingUsed::Utf8);
    }

    let (decoded, had_errors) = SHIFT_JIS.decode_without_bom_handling(raw);
    if !had_errors {
        return (decoded.into_owned(), EncodingUsed::ShiftJis);
    }

    (decode_cp437(raw), EncodingUsed::Cp437)
}

/// CP437 (IBM PC code page 437) → UTF-8。
///
/// `encoding_rs` はWeb標準エンコーディングのみが対象でCP437を含まない。
/// `zip`クレートは内部に同等のテーブル(src/cp437.rs)を持つが非公開
/// (`mod cp437;`)のため外部から呼べない。CP437の0x80以降は固定の標準
/// テーブルで表現される既知の文字集合なので、そのテーブルをここに複製
/// する。
// ponytail: 128エントリの固定表を直書き。専用crateを新規追加するほどでは
// ないための最小実装。将来CP437を扱う箇所が増えたら独立モジュール化を検討。
fn decode_cp437(raw: &[u8]) -> String {
    raw.iter().map(|&b| cp437_to_char(b)).collect()
}

#[rustfmt::skip]
fn cp437_to_char(b: u8) -> char {
    match b {
        0x00..=0x7f => b as char,
        0x80 => '\u{00c7}', 0x81 => '\u{00fc}', 0x82 => '\u{00e9}', 0x83 => '\u{00e2}',
        0x84 => '\u{00e4}', 0x85 => '\u{00e0}', 0x86 => '\u{00e5}', 0x87 => '\u{00e7}',
        0x88 => '\u{00ea}', 0x89 => '\u{00eb}', 0x8a => '\u{00e8}', 0x8b => '\u{00ef}',
        0x8c => '\u{00ee}', 0x8d => '\u{00ec}', 0x8e => '\u{00c4}', 0x8f => '\u{00c5}',
        0x90 => '\u{00c9}', 0x91 => '\u{00e6}', 0x92 => '\u{00c6}', 0x93 => '\u{00f4}',
        0x94 => '\u{00f6}', 0x95 => '\u{00f2}', 0x96 => '\u{00fb}', 0x97 => '\u{00f9}',
        0x98 => '\u{00ff}', 0x99 => '\u{00d6}', 0x9a => '\u{00dc}', 0x9b => '\u{00a2}',
        0x9c => '\u{00a3}', 0x9d => '\u{00a5}', 0x9e => '\u{20a7}', 0x9f => '\u{0192}',
        0xa0 => '\u{00e1}', 0xa1 => '\u{00ed}', 0xa2 => '\u{00f3}', 0xa3 => '\u{00fa}',
        0xa4 => '\u{00f1}', 0xa5 => '\u{00d1}', 0xa6 => '\u{00aa}', 0xa7 => '\u{00ba}',
        0xa8 => '\u{00bf}', 0xa9 => '\u{2310}', 0xaa => '\u{00ac}', 0xab => '\u{00bd}',
        0xac => '\u{00bc}', 0xad => '\u{00a1}', 0xae => '\u{00ab}', 0xaf => '\u{00bb}',
        0xb0 => '\u{2591}', 0xb1 => '\u{2592}', 0xb2 => '\u{2593}', 0xb3 => '\u{2502}',
        0xb4 => '\u{2524}', 0xb5 => '\u{2561}', 0xb6 => '\u{2562}', 0xb7 => '\u{2556}',
        0xb8 => '\u{2555}', 0xb9 => '\u{2563}', 0xba => '\u{2551}', 0xbb => '\u{2557}',
        0xbc => '\u{255d}', 0xbd => '\u{255c}', 0xbe => '\u{255b}', 0xbf => '\u{2510}',
        0xc0 => '\u{2514}', 0xc1 => '\u{2534}', 0xc2 => '\u{252c}', 0xc3 => '\u{251c}',
        0xc4 => '\u{2500}', 0xc5 => '\u{253c}', 0xc6 => '\u{255e}', 0xc7 => '\u{255f}',
        0xc8 => '\u{255a}', 0xc9 => '\u{2554}', 0xca => '\u{2569}', 0xcb => '\u{2566}',
        0xcc => '\u{2560}', 0xcd => '\u{2550}', 0xce => '\u{256c}', 0xcf => '\u{2567}',
        0xd0 => '\u{2568}', 0xd1 => '\u{2564}', 0xd2 => '\u{2565}', 0xd3 => '\u{2559}',
        0xd4 => '\u{2558}', 0xd5 => '\u{2552}', 0xd6 => '\u{2553}', 0xd7 => '\u{256b}',
        0xd8 => '\u{256a}', 0xd9 => '\u{2518}', 0xda => '\u{250c}', 0xdb => '\u{2588}',
        0xdc => '\u{2584}', 0xdd => '\u{258c}', 0xde => '\u{2590}', 0xdf => '\u{2580}',
        0xe0 => '\u{03b1}', 0xe1 => '\u{00df}', 0xe2 => '\u{0393}', 0xe3 => '\u{03c0}',
        0xe4 => '\u{03a3}', 0xe5 => '\u{03c3}', 0xe6 => '\u{00b5}', 0xe7 => '\u{03c4}',
        0xe8 => '\u{03a6}', 0xe9 => '\u{0398}', 0xea => '\u{03a9}', 0xeb => '\u{03b4}',
        0xec => '\u{221e}', 0xed => '\u{03c6}', 0xee => '\u{03b5}', 0xef => '\u{2229}',
        0xf0 => '\u{2261}', 0xf1 => '\u{00b1}', 0xf2 => '\u{2265}', 0xf3 => '\u{2264}',
        0xf4 => '\u{2320}', 0xf5 => '\u{2321}', 0xf6 => '\u{00f7}', 0xf7 => '\u{2248}',
        0xf8 => '\u{00b0}', 0xf9 => '\u{2219}', 0xfa => '\u{00b7}', 0xfb => '\u{221a}',
        0xfc => '\u{207f}', 0xfd => '\u{00b2}', 0xfe => '\u{25a0}', 0xff => '\u{00a0}',
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utf8_flag_skips_detection() {
        let raw = "日本語.txt".as_bytes();
        let (name, used) = decode_entry_name(raw, true);
        assert_eq!(used, EncodingUsed::Utf8);
        assert_eq!(name, "日本語.txt");
    }

    #[test]
    fn shift_jis_with_half_width_kana_is_detected() {
        // chardetngの統計的判定ではBig5と誤判定されていたケース。
        // ストリクトデコード方式では正しくShift-JISと判定できる。
        let original = "ﾃｽﾄ_日本語ﾌｧｲﾙ.txt";
        let (raw, _, had_errors) = SHIFT_JIS.encode(original);
        assert!(!had_errors);
        let (name, used) = decode_entry_name(&raw, false);
        assert_eq!(used, EncodingUsed::ShiftJis);
        assert_eq!(name, original);
    }

    #[test]
    fn shift_jis_without_half_width_kana_is_still_detected() {
        // 半角カナを含まない純粋な漢字・ひらがなのファイル名。
        let original = "運動会案内.docx";
        let (raw, _, had_errors) = SHIFT_JIS.encode(original);
        assert!(!had_errors);
        let (name, used) = decode_entry_name(&raw, false);
        assert_eq!(used, EncodingUsed::ShiftJis);
        assert_eq!(name, original);
    }

    #[test]
    fn invalid_shift_jis_bytes_fall_back_to_cp437() {
        // 末尾の0xE9はShift-JISの2バイト文字の先頭バイトとして有効な
        // 範囲(0xE0..=0xFC)だが、後続バイトが無いため不正な列となり
        // デコードエラーになる。CP437フォールバックが働くことを確認する。
        let raw = b"caf\xe9";
        let (decoded, had_errors) = SHIFT_JIS.decode_without_bom_handling(raw);
        assert!(had_errors, "テスト前提: このバイト列はShift-JISとして不正であること (got {decoded:?})");

        let (name, used) = decode_entry_name(raw, false);
        assert_eq!(used, EncodingUsed::Cp437);
        assert_eq!(name, "cafΘ");
    }

    #[test]
    fn two_entries_independently_decode_their_own_encoding() {
        // rc-zipが失敗する「1ZIP内で複数言語混在」ケースを、エントリ単位の
        // 呼び出しが2回とも正しく解決できることを示す。
        let (sjis_raw, _, _) = SHIFT_JIS.encode("ﾃｽﾄ_日本語.txt");
        let (sjis_name, sjis_used) = decode_entry_name(&sjis_raw, false);
        assert_eq!(sjis_used, EncodingUsed::ShiftJis);
        assert_eq!(sjis_name, "ﾃｽﾄ_日本語.txt");

        let cp437_raw = b"caf\xe9";
        let (cp437_name, cp437_used) = decode_entry_name(cp437_raw, false);
        assert_eq!(cp437_used, EncodingUsed::Cp437);
        assert_eq!(cp437_name, "cafΘ");
    }
}
