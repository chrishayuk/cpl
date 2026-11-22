//! Keeps the tutorial honest. In tutorial/*.md, a ```cpl block that is
//! followed by an ```output block (with an ```input block in between if the
//! program reads anything) is a complete program: it is run, and must print
//! exactly what the output block says. A program that ends in an error has
//! the message as a last line beginning "! ". A ```cpl block followed by
//! anything else is a fragment and is not run.
//!
//! Run with CPL_BLESS=1 to rewrite the output blocks from real runs.

use std::path::{Path, PathBuf};

/// The package directory. Inside a WASI runtime only the current directory
/// can be reached, and cargo runs tests from the package directory.
fn root() -> PathBuf {
    if cfg!(target_family = "wasm") {
        PathBuf::from(".")
    } else {
        Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
    }
}

struct Block {
    lang: String,
    text: String,
    /// Byte range of the text within the file.
    start: usize,
    end: usize,
}

fn blocks(src: &str) -> Vec<Block> {
    let mut out = Vec::new();
    let mut pos = 0;
    let mut open: Option<(String, usize)> = None;
    for line in src.split_inclusive('\n') {
        let fence = line.trim_end().strip_prefix("```");
        match (&open, fence) {
            (None, Some(lang)) => open = Some((lang.to_string(), pos + line.len())),
            (Some((lang, start)), Some("")) => {
                out.push(Block {
                    lang: lang.clone(),
                    text: src[*start..pos].to_string(),
                    start: *start,
                    end: pos,
                });
                open = None;
            }
            _ => {}
        }
        pos += line.len();
    }
    assert!(open.is_none(), "a code block is not closed");
    out
}

fn transcript(src: &str, input: &str) -> String {
    let (mut out, result) = cpl::run_source(src, input, Some(20_000_000));
    if let Err(e) = result {
        if !out.is_empty() && !out.ends_with('\n') {
            out.push('\n');
        }
        out.push_str(&format!("! {}\n", e));
    }
    if !out.ends_with('\n') {
        out.push('\n');
    }
    out
}

#[test]
fn every_program_in_the_tutorial_prints_what_the_tutorial_says() {
    let dir = root().join("tutorial");
    let bless = std::env::var("CPL_BLESS").is_ok();
    let mut files: Vec<_> = std::fs::read_dir(&dir)
        .unwrap()
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "md"))
        .collect();
    files.sort();
    assert!(bless || files.len() >= 15, "tutorial chapters are missing");
    let (mut checked, mut failed) = (0, 0);
    for path in files {
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        let mut text = std::fs::read_to_string(&path).unwrap();
        let bs = blocks(&text);
        let mut edits = Vec::new();
        for (i, b) in bs.iter().enumerate() {
            if b.lang != "cpl" {
                continue;
            }
            let mut j = i + 1;
            let mut input = "";
            if bs.get(j).is_some_and(|n| n.lang == "input") {
                input = &bs[j].text;
                j += 1;
            }
            let Some(want) = bs.get(j).filter(|n| n.lang == "output") else {
                continue;
            };
            let got = transcript(&b.text, input);
            checked += 1;
            if got != want.text {
                if bless {
                    edits.push((want.start, want.end, got));
                } else {
                    failed += 1;
                    eprintln!(
                        "---- {} ----\n{}--- the tutorial says\n{}--- it prints\n{}",
                        name, b.text, want.text, got
                    );
                }
            }
        }
        if !edits.is_empty() {
            for (start, end, new) in edits.into_iter().rev() {
                text.replace_range(start..end, &new);
            }
            std::fs::write(&path, text).unwrap();
        }
    }
    assert!(bless || checked >= 100, "only {} tutorial programs were found", checked);
    assert_eq!(failed, 0, "tutorial programs with the wrong output");
}
