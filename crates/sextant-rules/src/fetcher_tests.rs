use super::*;
use std::path::PathBuf;
use tempfile::tempdir;

#[test]
fn parse_github_spec_with_tag() {
    let s = parse_pack_spec("github:owner/repo@v1.0.0").unwrap();
    assert_eq!(
        s.source,
        PackSource::Github {
            owner: "owner".into(),
            repo: "repo".into()
        }
    );
    assert_eq!(s.reference, "v1.0.0");
    assert_eq!(s.subdir, None);
}

#[test]
fn parse_github_spec_with_subdir() {
    let s = parse_pack_spec("github:k/r@v1#packs/typescript").unwrap();
    assert_eq!(s.subdir.as_deref(), Some("packs/typescript"));
    assert_eq!(s.reference, "v1");
}

#[test]
fn parse_file_spec() {
    let s = parse_pack_spec("file:./packs/typescript").unwrap();
    assert_eq!(
        s.source,
        PackSource::File {
            path: PathBuf::from("./packs/typescript")
        }
    );
    assert_eq!(s.subdir, None);
}

#[test]
fn parse_rejects_unknown_scheme() {
    assert!(matches!(
        parse_pack_spec("https://example.com/pack").unwrap_err(),
        FetchError::BadSpec(_, _)
    ));
}

#[test]
fn parse_github_rejects_missing_ref() {
    assert!(matches!(
        parse_pack_spec("github:owner/repo").unwrap_err(),
        FetchError::BadSpec(_, _)
    ));
}

#[test]
fn parse_github_rejects_empty_ref() {
    assert!(matches!(
        parse_pack_spec("github:owner/repo@").unwrap_err(),
        FetchError::BadSpec(_, _)
    ));
}

#[test]
fn parse_github_rejects_missing_owner_or_repo() {
    assert!(matches!(
        parse_pack_spec("github:owner@v1").unwrap_err(),
        FetchError::BadSpec(_, _)
    ));
    assert!(matches!(
        parse_pack_spec("github:/repo@v1").unwrap_err(),
        FetchError::BadSpec(_, _)
    ));
}

fn write_pack(dir: &Path, name: &str) {
    std::fs::create_dir_all(dir.join("rules")).unwrap();
    std::fs::write(
        dir.join("pack.toml"),
        format!("name = \"{name}\"\nversion = \"0.0.1\"\n"),
    )
    .unwrap();
    std::fs::write(
        dir.join("rules/demo.md"),
        r#"---
id: vendor.demo
name: D
description: x
severity: error
category: style
languages: [typescript]
evaluator: { type: regex, pattern: "x" }
---
"#,
    )
    .unwrap();
}

#[test]
fn fetch_file_copies_directory_and_hashes() {
    let src = tempdir().unwrap();
    write_pack(src.path(), "ts");
    let spec = parse_pack_spec(&format!("file:{}", src.path().display())).unwrap();
    let fetched = fetch_pack(&spec).unwrap();
    assert_eq!(fetched.manifest.name, "ts");
    assert!(fetched.files.contains_key("pack.toml"));
    assert!(fetched.files.contains_key("rules/demo.md"));
    // Staged contents should match the source.
    assert!(fetched.staging_dir.path().join("pack.toml").exists());
}

#[test]
fn fetch_file_with_subdir_picks_nested_pack() {
    let src = tempdir().unwrap();
    let subdir = src.path().join("packs").join("ts");
    std::fs::create_dir_all(&subdir).unwrap();
    write_pack(&subdir, "ts");
    let spec = parse_pack_spec(&format!("file:{}#packs/ts", src.path().display())).unwrap();
    let fetched = fetch_pack(&spec).unwrap();
    assert_eq!(fetched.manifest.name, "ts");
    assert_eq!(fetched.subdir.as_deref(), Some("packs/ts"));
}

#[test]
fn fetch_file_errors_on_missing_manifest() {
    let src = tempdir().unwrap();
    std::fs::create_dir_all(src.path().join("rules")).unwrap();
    let spec = parse_pack_spec(&format!("file:{}", src.path().display())).unwrap();
    let err = match fetch_pack(&spec) {
        Ok(_) => panic!("expected MissingManifest"),
        Err(e) => e,
    };
    assert!(matches!(err, FetchError::MissingManifest(_)));
}

#[test]
fn fetch_file_errors_on_missing_subdir() {
    let src = tempdir().unwrap();
    let spec = parse_pack_spec(&format!("file:{}#nope", src.path().display())).unwrap();
    let err = match fetch_pack(&spec) {
        Ok(_) => panic!("expected SubdirMissing"),
        Err(e) => e,
    };
    assert!(matches!(err, FetchError::SubdirMissing { .. }));
}
