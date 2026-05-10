use std::path::Path;
use std::process::ExitCode;

use anyhow::{anyhow, Context, Result};
use sextant_core::RuleSource;
use sextant_engine::{explain_rule, list_rules};
use sextant_rules::fetcher::{fetch_pack, parse_pack_spec, FetchedPack};
use sextant_rules::lock::{self, LockFile, LockedPack};
use sextant_rules::{parse_rule_md, EvaluatorSpec};

pub(crate) fn list() -> Result<ExitCode> {
    let cwd = std::env::current_dir().context("getting current dir")?;
    let rules = list_rules(&cwd).context("loading rules")?;
    for r in rules {
        println!(
            "{}\t{}\t{}\t{}\t{}",
            r.id,
            r.severity.as_str(),
            format!("{:?}", r.scope).to_lowercase(),
            r.source.name(),
            r.name
        );
    }
    Ok(ExitCode::from(0))
}

pub(crate) fn explain(id: &str) -> Result<ExitCode> {
    let cwd = std::env::current_dir().context("getting current dir")?;
    let Some(r) = explain_rule(&cwd, id).context("loading rules")? else {
        eprintln!("error: no rule with id `{id}`");
        return Ok(ExitCode::from(2));
    };
    println!("# {} ({})", r.name, r.id);
    println!();
    println!(
        "**severity:** {}  •  **category:** {}  •  **source:** {}",
        r.severity.as_str(),
        r.category.name(),
        r.source.name()
    );
    println!();
    if !r.description.is_empty() {
        println!("{}", r.description);
        println!();
    }
    if !r.body.is_empty() {
        println!("{}", r.body);
    }
    Ok(ExitCode::from(0))
}

pub(crate) fn check(path: &Path) -> Result<ExitCode> {
    let text =
        std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    match parse_rule_md(&text, RuleSource::Repo, Some(path.to_path_buf())) {
        Ok(rule) => {
            println!("OK: {} ({})", rule.id, rule.name);
            println!(
                "  severity={} category={} scope={:?}",
                rule.severity.as_str(),
                rule.category.name(),
                rule.scope,
            );
            match &rule.evaluator {
                EvaluatorSpec::Builtin { name } => {
                    println!("  evaluator=builtin name={name}");
                }
                EvaluatorSpec::Regex { pattern, .. } => {
                    println!("  evaluator=regex pattern={pattern:?}");
                }
                EvaluatorSpec::Llm { model, .. } => {
                    let m = model.as_deref().unwrap_or("<from config>");
                    println!("  evaluator=llm model={m}");
                }
                EvaluatorSpec::Ast { query, .. } => {
                    println!("  evaluator=ast query={query:?}");
                }
            }
            Ok(ExitCode::from(0))
        }
        Err(err) => {
            eprintln!("error: {err}");
            Ok(ExitCode::from(2))
        }
    }
}

pub(crate) fn add(spec: &str, name_override: Option<&str>) -> Result<ExitCode> {
    let cwd = std::env::current_dir().context("getting current dir")?;
    let parsed = parse_pack_spec(spec).with_context(|| format!("parsing spec `{spec}`"))?;
    let fetched = fetch_pack(&parsed).with_context(|| format!("fetching `{spec}`"))?;
    let pack_name = name_override
        .map(str::to_string)
        .unwrap_or_else(|| fetched.manifest.name.clone());
    install_pack(&cwd, &pack_name, &fetched, "added")?;
    Ok(ExitCode::from(0))
}

pub(crate) fn update(packs: &[String]) -> Result<ExitCode> {
    let cwd = std::env::current_dir().context("getting current dir")?;
    let Some(mut lock_file) = LockFile::read(&cwd).context("reading rules.lock")? else {
        eprintln!("error: no `.sextant/rules.lock` — run `sextant rules add` first");
        return Ok(ExitCode::from(2));
    };
    let targets = resolve_update_targets(&lock_file, packs)?;
    if targets.is_empty() {
        println!("no vendor packs installed; nothing to update");
        return Ok(ExitCode::from(0));
    }
    for name in &targets {
        update_one_pack(&cwd, &mut lock_file, name)?;
    }
    Ok(ExitCode::from(0))
}

fn resolve_update_targets(lock_file: &LockFile, packs: &[String]) -> Result<Vec<String>> {
    if packs.is_empty() {
        return Ok(lock_file.packs.iter().map(|p| p.name.clone()).collect());
    }
    for name in packs {
        if lock_file.pack(name).is_none() {
            return Err(anyhow!("pack `{name}` is not installed"));
        }
    }
    Ok(packs.to_vec())
}

fn update_one_pack(repo_root: &Path, lock_file: &mut LockFile, name: &str) -> Result<()> {
    let entry = lock_file
        .pack(name)
        .ok_or_else(|| anyhow!("pack `{name}` vanished from lock"))?
        .clone();
    let spec = pack_spec_from_lock(&entry)?;
    let fetched = fetch_pack(&spec).with_context(|| format!("re-fetching pack `{name}`"))?;
    let new_entry = install_pack_files(repo_root, name, &fetched)?;
    let was_changed = new_entry.revision != entry.revision || new_entry.files != entry.files;
    let revision_after = new_entry.revision.clone();
    lock_file.upsert(new_entry);
    lock_file.write(repo_root).context("updating rules.lock")?;
    if was_changed {
        println!("updated pack `{name}` -> {revision_after}");
    } else {
        println!("pack `{name}` already up to date");
    }
    Ok(())
}

pub(crate) fn remove(name: &str) -> Result<ExitCode> {
    let cwd = std::env::current_dir().context("getting current dir")?;
    let mut lock_file = LockFile::read(&cwd)
        .context("reading rules.lock")?
        .unwrap_or_else(LockFile::empty);
    if !lock_file.remove(name) {
        return Err(anyhow!("pack `{name}` is not installed"));
    }
    let dir = lock::pack_dir(&cwd, name);
    if dir.exists() {
        std::fs::remove_dir_all(&dir).with_context(|| format!("removing {}", dir.display()))?;
    }
    lock_file.write(&cwd).context("writing rules.lock")?;
    println!("removed pack `{name}`");
    Ok(ExitCode::from(0))
}

fn install_pack(
    repo_root: &Path,
    pack_name: &str,
    fetched: &FetchedPack,
    verb: &str,
) -> Result<()> {
    let entry = install_pack_files(repo_root, pack_name, fetched)?;
    let mut lock_file = LockFile::read(repo_root)
        .context("reading rules.lock")?
        .unwrap_or_else(LockFile::empty);
    lock_file.upsert(entry);
    lock_file.write(repo_root).context("writing rules.lock")?;
    println!(
        "{verb} pack `{pack_name}` ({version}) — {n} files",
        version = if fetched.manifest.version.is_empty() {
            "unversioned"
        } else {
            fetched.manifest.version.as_str()
        },
        n = fetched.files.len()
    );
    Ok(())
}

fn install_pack_files(
    repo_root: &Path,
    pack_name: &str,
    fetched: &FetchedPack,
) -> Result<LockedPack> {
    if pack_name.is_empty() {
        return Err(anyhow!(
            "pack name is empty (set `name` in pack.toml or pass --name)"
        ));
    }
    let dest = lock::pack_dir(repo_root, pack_name);
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }
    if dest.exists() {
        std::fs::remove_dir_all(&dest)
            .with_context(|| format!("removing existing {}", dest.display()))?;
    }
    copy_dir_recursive(fetched.staging_dir.path(), &dest)?;
    Ok(LockedPack {
        name: pack_name.to_string(),
        source: fetched.source_label.clone(),
        reference: fetched.reference.clone(),
        revision: fetched.revision.clone(),
        subdir: fetched.subdir.clone().unwrap_or_default(),
        fetched_at: now_iso8601(),
        files: fetched.files.clone(),
    })
}

fn pack_spec_from_lock(entry: &LockedPack) -> Result<sextant_rules::fetcher::PackSpec> {
    let suffix = pack_subdir_suffix(&entry.subdir);
    let raw = if entry.source.starts_with("github:") {
        if entry.reference.is_empty() {
            return Err(anyhow!("locked pack `{}` has no ref", entry.name));
        }
        format!("{}@{}{}", entry.source, entry.reference, suffix)
    } else if entry.source.starts_with("file:") {
        format!("{}{}", entry.source, suffix)
    } else {
        return Err(anyhow!(
            "locked pack `{}` has unsupported source `{}`",
            entry.name,
            entry.source
        ));
    };
    parse_pack_spec(&raw).map_err(|e| anyhow!("re-parsing locked spec for `{}`: {e}", entry.name))
}

fn pack_subdir_suffix(subdir: &str) -> String {
    if subdir.is_empty() {
        String::new()
    } else {
        format!("#{subdir}")
    }
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    std::fs::create_dir_all(dst).with_context(|| format!("creating {}", dst.display()))?;
    for entry in std::fs::read_dir(src).with_context(|| format!("reading {}", src.display()))? {
        let entry = entry?;
        copy_one_entry(&entry, dst)?;
    }
    Ok(())
}

fn copy_one_entry(entry: &std::fs::DirEntry, dst: &Path) -> Result<()> {
    let path = entry.path();
    let target = dst.join(entry.file_name());
    let ft = entry.file_type()?;
    if ft.is_dir() {
        copy_dir_recursive(&path, &target)?;
    } else if ft.is_file() {
        std::fs::copy(&path, &target)
            .with_context(|| format!("copying {} -> {}", path.display(), target.display()))?;
    }
    Ok(())
}

fn now_iso8601() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    // Cheap ISO-8601 without pulling chrono just for a timestamp.
    let (y, mo, d, h, mi, s) = epoch_to_ymdhms(secs);
    format!("{y:04}-{mo:02}-{d:02}T{h:02}:{mi:02}:{s:02}Z")
}

fn epoch_to_ymdhms(mut secs: u64) -> (i32, u32, u32, u32, u32, u32) {
    let s = (secs % 60) as u32;
    secs /= 60;
    let mi = (secs % 60) as u32;
    secs /= 60;
    let h = (secs % 24) as u32;
    let mut days = secs / 24;
    let mut y: i32 = 1970;
    loop {
        let yd = if is_leap(y) { 366 } else { 365 };
        if days < yd {
            break;
        }
        days -= yd;
        y += 1;
    }
    let months: [u32; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let leap_feb = if is_leap(y) { 29 } else { 28 };
    let mut mo = 1u32;
    for (i, len) in months.iter().enumerate() {
        let len = if i == 1 { leap_feb } else { *len };
        if days < len as u64 {
            break;
        }
        days -= len as u64;
        mo += 1;
    }
    (y, mo, days as u32 + 1, h, mi, s)
}

fn is_leap(y: i32) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}
