//! Shared scaffolding for built-in rules that need to inspect every
//! function in a file (currently `fn_length` and `param_count`). Both
//! rules want the same boilerplate: pick a language, parse, enumerate
//! functions, and then apply rule-specific threshold logic to each.

use sextant_core::{EvalContext, Finding, SourceFile};
use sextant_lang::{function_ranges, parse, FunctionRange, Language};

/// Drive a per-function rule. `f` is called once per function with the
/// `FunctionRange` and the file's repo-relative path; it returns an
/// optional finding. Unsupported languages and parse failures produce
/// an empty result.
pub(crate) fn for_each_function<F>(
    file: &SourceFile,
    ctx: &EvalContext<'_>,
    mut f: F,
) -> Vec<Finding>
where
    F: FnMut(&FunctionRange, &std::path::Path) -> Option<Finding>,
{
    let Some(hint) = file.language_hint() else {
        return Vec::new();
    };
    let Some(lang) = Language::from_hint(hint) else {
        return Vec::new();
    };
    let parsed = match parse(file.contents.clone(), lang) {
        Ok(p) => p,
        Err(err) => {
            tracing::debug!(?err, path=?file.path, "parse failed");
            return Vec::new();
        }
    };
    let fns = match function_ranges(&parsed) {
        Ok(f) => f,
        Err(err) => {
            tracing::debug!(?err, path=?file.path, "function_ranges failed");
            return Vec::new();
        }
    };
    let path = file.relative_to(ctx.repo_root);
    let mut out = Vec::new();
    for fr in &fns {
        if let Some(finding) = f(fr, &path) {
            out.push(finding);
        }
    }
    out
}
