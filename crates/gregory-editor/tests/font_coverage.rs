use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use skrifa::{FontRef, MetadataProvider};

const HINT: &str = "`npm run font:subset` を実行して subset-chars.txt とフォントを再生成すること";

fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn repo_root() -> PathBuf {
    crate_dir().join("../..")
}

fn rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            rs_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

// Cc・Cs・Co・Cn・Cf は表示しない文字なので対象外（スクリプトの除外規則と同じ）。
fn is_excluded(c: char) -> bool {
    c.is_control()
        || matches!(c, '\u{00AD}' | '\u{0600}'..='\u{0605}' | '\u{061C}' | '\u{06DD}' | '\u{070F}'
            | '\u{180E}' | '\u{200B}'..='\u{200F}' | '\u{202A}'..='\u{202E}'
            | '\u{2060}'..='\u{2064}' | '\u{2066}'..='\u{206F}' | '\u{FEFF}'
            | '\u{FFF9}'..='\u{FFFB}')
}

// 記号群と ASCII 範囲は二重管理を避けるため、スクリプトの Python 部分から読み取る。
fn script_fixed_chars() -> BTreeSet<char> {
    let script = fs::read_to_string(repo_root().join("scripts/subset-ui-font.sh")).unwrap();
    let mut set = BTreeSet::new();

    let range = script
        .lines()
        .find(|l| l.contains("range(0x20, 0x7F)"))
        .expect("ASCII の範囲がスクリプトに見つからない");
    assert!(range.contains("chars = {chr(c) for c in range(0x20, 0x7F)}"));
    set.extend((0x20u8..0x7F).map(char::from));

    let symbols = script
        .lines()
        .find_map(|l| l.trim().strip_prefix("chars |= set(\""))
        .and_then(|l| l.strip_suffix("\")"))
        .expect("記号群がスクリプトに見つからない");
    set.extend(symbols.chars());
    set
}

fn source_chars() -> BTreeSet<char> {
    let mut chars = script_fixed_chars();
    let mut files = Vec::new();
    rs_files(&crate_dir().join("src"), &mut files);
    files.sort();
    files.push(crate_dir().join("index.html"));
    let core = repo_root().join("crates/gregory-core/src");
    files.extend([
        core.join("mesh/presets.rs"),
        core.join("error.rs"),
        core.join("mesh/error.rs"),
    ]);
    for f in files {
        let text = fs::read_to_string(&f).unwrap_or_else(|e| panic!("{}: {e}", f.display()));
        chars.extend(text.chars().filter(|c| !c.is_ascii()));
    }
    chars.retain(|c| !is_excluded(*c));
    chars
}

fn listed_chars() -> BTreeSet<char> {
    let path = crate_dir().join("assets/fonts/subset-chars.txt");
    let text = fs::read_to_string(path).unwrap();
    text.lines()
        .filter(|l| !l.starts_with('#') && !l.is_empty())
        .map(|l| {
            let (code, ch) = l.split_once('\t').expect("U+XXXX<TAB>文字 の形式でない");
            let cp = u32::from_str_radix(code.strip_prefix("U+").unwrap(), 16).unwrap();
            let c = char::from_u32(cp).unwrap();
            assert_eq!(
                ch.chars().collect::<Vec<_>>(),
                vec![c],
                "{code} の文字が一致しない"
            );
            c
        })
        .collect()
}

fn show(set: &BTreeSet<char>) -> String {
    set.iter()
        .map(|c| format!("U+{:04X}({c})", *c as u32))
        .collect::<Vec<_>>()
        .join(" ")
}

#[test]
fn subset_chars_cover_every_char_used_in_sources() {
    let missing: BTreeSet<char> = source_chars()
        .difference(&listed_chars())
        .copied()
        .collect();
    assert!(
        missing.is_empty(),
        "subset-chars.txt に無い文字: {}\n{HINT}",
        show(&missing)
    );
}

#[test]
fn subset_chars_have_no_stale_entries() {
    let stale: BTreeSet<char> = listed_chars()
        .difference(&source_chars())
        .copied()
        .collect();
    assert!(
        stale.is_empty(),
        "ソースに無い文字が残っている: {}\n{HINT}",
        show(&stale)
    );
}

#[test]
fn subset_font_cmap_contains_every_listed_char() {
    let data = fs::read(crate_dir().join("assets/fonts/gregory-ui-sans.ttf")).unwrap();
    let font = FontRef::new(&data).unwrap();
    let charmap = font.charmap();
    let missing: BTreeSet<char> = listed_chars()
        .into_iter()
        .filter(|c| charmap.map(*c).is_none())
        .collect();
    assert!(
        missing.is_empty(),
        "フォントの cmap に無い文字: {}\n{HINT}",
        show(&missing)
    );
}
