use super::*;
use tempfile::tempdir;

fn make_pack(name: &str, files: &[(&str, &str)]) -> (tempfile::TempDir, LockedPack) {
    let dir = tempdir().unwrap();
    let mut locked = LockedPack {
        name: name.into(),
        source: format!("file:./{name}"),
        reference: "v0.0.0".into(),
        revision: "deadbeef".into(),
        subdir: String::new(),
        fetched_at: String::new(),
        files: BTreeMap::new(),
    };
    for (rel, body) in files {
        let path = dir.path().join(rel);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(&path, body).unwrap();
        locked
            .files
            .insert((*rel).into(), hash_bytes(body.as_bytes()));
    }
    (dir, locked)
}

#[test]
fn read_returns_none_when_lock_is_absent() {
    let dir = tempdir().unwrap();
    assert!(LockFile::read(dir.path()).unwrap().is_none());
}

#[test]
fn write_then_read_round_trips() {
    let dir = tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join(".sextant")).unwrap();
    let (_pack_dir, pack) = make_pack("typescript", &[("rules/no-any.md", "x")]);
    let mut lock = LockFile::empty();
    lock.upsert(pack.clone());
    lock.write(dir.path()).unwrap();
    let read_back = LockFile::read(dir.path()).unwrap().unwrap();
    assert_eq!(read_back.version, lock.version);
    assert_eq!(read_back.packs, lock.packs);
}

#[test]
fn unsupported_version_returns_error() {
    let dir = tempdir().unwrap();
    let path = lock_path(dir.path());
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, "version = 99\n").unwrap();
    let err = LockFile::read(dir.path()).unwrap_err();
    assert!(matches!(err, LockError::UnsupportedVersion { .. }));
}

#[test]
fn upsert_replaces_existing_pack_entry() {
    let mut lock = LockFile::empty();
    let (_d1, p1) = make_pack("typescript", &[("a.md", "1")]);
    let (_d2, mut p2) = make_pack("typescript", &[("a.md", "2")]);
    p2.reference = "v1".into();
    lock.upsert(p1);
    lock.upsert(p2.clone());
    assert_eq!(lock.packs.len(), 1);
    assert_eq!(lock.packs[0].reference, "v1");
}

#[test]
fn verify_pack_passes_for_unchanged_directory() {
    let (dir, pack) = make_pack(
        "typescript",
        &[
            ("pack.toml", "name = \"typescript\"\n"),
            ("rules/x.md", "body"),
        ],
    );
    verify_pack(&pack, dir.path()).unwrap();
}

fn pack_with_one_rule() -> (tempfile::TempDir, LockedPack) {
    make_pack("typescript", &[("rules/x.md", "body")])
}

fn verify_after<M: FnOnce(&std::path::Path)>(mutate: M) -> LockError {
    let (dir, pack) = pack_with_one_rule();
    mutate(dir.path());
    verify_pack(&pack, dir.path()).unwrap_err()
}

#[test]
fn verify_pack_detects_modified_file() {
    let err = verify_after(|root| {
        std::fs::write(root.join("rules/x.md"), "tampered").unwrap();
    });
    assert!(matches!(err, LockError::HashMismatch { .. }));
}

#[test]
fn verify_pack_detects_missing_file() {
    let err = verify_after(|root| {
        std::fs::remove_file(root.join("rules/x.md")).unwrap();
    });
    assert!(matches!(err, LockError::MissingFile { .. }));
}

#[test]
fn verify_pack_detects_untracked_file() {
    let err = verify_after(|root| {
        std::fs::write(root.join("rules/sneaky.md"), "extra").unwrap();
    });
    assert!(matches!(err, LockError::UntrackedFile { .. }));
}

#[test]
fn verify_pack_detects_missing_directory() {
    let dir = tempdir().unwrap();
    let pack = LockedPack {
        name: "typescript".into(),
        source: "file:.".into(),
        reference: String::new(),
        revision: String::new(),
        subdir: String::new(),
        fetched_at: String::new(),
        files: BTreeMap::new(),
    };
    let err = verify_pack(&pack, &dir.path().join("does-not-exist")).unwrap_err();
    assert!(matches!(err, LockError::MissingPackDir { .. }));
}

#[test]
fn hash_directory_walks_subdirs() {
    let dir = tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("rules")).unwrap();
    std::fs::write(dir.path().join("pack.toml"), "x").unwrap();
    std::fs::write(dir.path().join("rules/a.md"), "y").unwrap();
    let map = hash_directory(dir.path()).unwrap();
    assert!(map.contains_key("pack.toml"));
    assert!(map.contains_key("rules/a.md"));
    assert_eq!(map.len(), 2);
}
