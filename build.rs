use std::path::Path;
use std::{env, fs};

fn main() {
    println!("cargo:rustc-env=TARGET={}", env::var("TARGET").unwrap());

    breaking_changes()
}

fn breaking_changes() {
    let out_dir_s = &env::var("OUT_DIR").unwrap();
    let out_dir = Path::new(out_dir_s);
    let version_str = env::var("CARGO_PKG_VERSION").unwrap();
    let changelog = parse_changelog::parse(include_str!("CHANGELOG.md")).expect("Invalid CHANGELOG.md");
    let release = changelog
        .get(&*version_str)
        .expect("Current release not found in CHANGELOG.md");
    let breaking_changes = release
        .notes
        // Get the part after the header
        .split_once("### Breaking changes")
        // Until the next header
        .map(|(_, after)| {
            after
                .split_once("###")
                .map(|(before, _)| before)
                // If this is the last section, just return the entire thing
                .unwrap_or(after)
        });
    fs::write(
        out_dir.join("breaking_changes.txt"),
        breaking_changes.unwrap_or("").trim(),
    )
    .unwrap();
}
