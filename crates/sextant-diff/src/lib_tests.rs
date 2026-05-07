use super::*;
use std::fs;
use std::process::Command;

fn git(dir: &Path, args: &[&str]) {
    let status = Command::new("git")
        .args(args)
        .current_dir(dir)
        .status()
        .expect("running git");
    assert!(status.success(), "git {args:?} failed");
}

fn init_repo(dir: &Path) {
    git(dir, &["init", "-q", "-b", "main"]);
    git(dir, &["config", "user.email", "test@example.com"]);
    git(dir, &["config", "user.name", "Test"]);
    git(dir, &["config", "commit.gpgsign", "false"]);
}

#[test]
fn working_tree_diff_against_base() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    init_repo(root);

    fs::write(root.join("a.txt"), "alpha\nbeta\n").unwrap();
    git(root, &["add", "."]);
    git(root, &["commit", "-q", "-m", "init"]);

    fs::write(root.join("a.txt"), "alpha\nBETA\ngamma\n").unwrap();
    fs::write(root.join("b.txt"), "new\n").unwrap();

    let diff = compute(root, &BaseSpec::Ref("HEAD".into()), &HeadSpec::WorkingTree).expect("diff");

    let a = diff.file_for(Path::new("a.txt")).expect("a.txt in diff");
    assert_eq!(a.status, ChangeKind::Modified);
    assert!(a.changed_lines.contains(&2));
    assert!(a.changed_lines.contains(&3));

    let b = diff.file_for(Path::new("b.txt")).expect("b.txt in diff");
    assert_eq!(b.status, ChangeKind::Added);
    assert!(b.changed_lines.contains(&1));
}

#[test]
fn files_at_ref_reads_full_tree() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    init_repo(root);

    fs::write(root.join("a.txt"), "alpha\n").unwrap();
    fs::create_dir(root.join("sub")).unwrap();
    fs::write(root.join("sub").join("b.txt"), "beta\n").unwrap();
    fs::write(root.join("c.bin"), [0xff, 0xfe, 0xfd]).unwrap();
    git(root, &["add", "."]);
    git(root, &["commit", "-q", "-m", "init"]);

    let snap = files_at_ref(root, "HEAD").expect("snapshot");
    let paths: Vec<_> = snap.files.iter().map(|(p, _)| p.clone()).collect();
    assert!(paths.contains(&PathBuf::from("a.txt")));
    assert!(paths.contains(&PathBuf::from("sub/b.txt")));
    assert!(!paths.iter().any(|p| p.to_string_lossy().ends_with("c.bin")));
    assert!(!snap.oid.is_zero());
}

#[test]
fn ref_to_ref_diff() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    init_repo(root);

    fs::write(root.join("a.txt"), "one\n").unwrap();
    git(root, &["add", "."]);
    git(root, &["commit", "-q", "-m", "init"]);
    let base_sha = String::from_utf8(
        Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(root)
            .output()
            .unwrap()
            .stdout,
    )
    .unwrap()
    .trim()
    .to_string();

    fs::write(root.join("a.txt"), "one\ntwo\n").unwrap();
    git(root, &["commit", "-aq", "-m", "second"]);

    let diff = compute(
        root,
        &BaseSpec::Ref(base_sha),
        &HeadSpec::Ref("HEAD".into()),
    )
    .expect("diff");

    let a = diff.file_for(Path::new("a.txt")).expect("a.txt in diff");
    assert!(a.changed_lines.contains(&2));
    assert_eq!(a.head_contents.as_deref(), Some("one\ntwo\n"));
}
