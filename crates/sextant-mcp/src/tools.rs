//! Tool definitions and dispatch for the Sextant MCP server.
//!
//! Each tool wraps an engine call. Inputs and outputs are JSON; tool results
//! follow MCP's `{ content: [{ type: "text", text: "..." }] }` shape, with
//! the JSON encoded as a string (clients are expected to parse it). For
//! `explain_rule` we return raw markdown text instead of JSON.

use std::path::PathBuf;

use serde::Deserialize;
use serde_json::{json, Value};
use sextant_engine::{
    explain_rule as engine_explain, grade as engine_grade, list_rules as engine_list,
    load_config as engine_load_config, DiffOptions, EngineError, GradeMode, RuleSummary,
};

use crate::protocol::codes;

#[derive(Debug, Deserialize, Default)]
struct GradeDiffArgs {
    #[serde(default)]
    base: Option<String>,
    #[serde(default)]
    head: Option<String>,
    #[serde(default)]
    working_tree: bool,
}

#[derive(Debug, Deserialize, Default)]
struct GradeFilesArgs {
    #[serde(default)]
    paths: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ExplainRuleArgs {
    id: String,
}

type ToolResult = Result<Value, (i32, String)>;

/// Static descriptors for every tool we serve.
pub(crate) fn descriptors() -> Vec<Value> {
    vec![
        descriptor_grade_diff(),
        descriptor_grade_files(),
        descriptor_list_rules(),
        descriptor_explain_rule(),
        descriptor_get_config(),
    ]
}

fn descriptor_grade_diff() -> Value {
    json!({
        "name": "grade_diff",
        "description": "Grade only the lines that changed since `base`. \
            Fast — call this in the inner edit loop after each modification \
            and self-correct before ending the turn. Returns a JSON Report \
            with findings, severity counts, and a verdict. Some findings \
            include a `patch` field with a unified diff against the file at \
            HEAD; prefer applying the proposed patch over re-deriving the \
            fix from scratch.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "base": { "type": "string", "description": "Base ref. Defaults to merge-base with origin/main, then HEAD~1." },
                "head": { "type": "string", "description": "Head ref. Defaults to working tree." },
                "working_tree": { "type": "boolean", "description": "Force diff against the working tree even if `head` is set." }
            }
        }
    })
}

fn descriptor_grade_files() -> Value {
    json!({
        "name": "grade_files",
        "description": "Grade entire current contents of the given files (or the whole repo if `paths` is empty). \
            Slower than grade_diff; use for thorough review. Findings may include a \
            `patch` field with a unified diff proposing the fix; apply it directly \
            when present.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "paths": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Paths to grade. Empty = whole repo."
                }
            }
        }
    })
}

fn descriptor_list_rules() -> Value {
    json!({
        "name": "list_rules",
        "description": "List every rule loaded for this repository. Each entry includes id, name, severity, category, scope, source (builtin/repo), and a one-line description. Use `explain_rule` with the id to fetch full markdown documentation.",
        "inputSchema": { "type": "object", "properties": {} }
    })
}

fn descriptor_explain_rule() -> Value {
    json!({
        "name": "explain_rule",
        "description": "Return the full markdown documentation for a single rule, including thresholds and how to fix findings. Always call this after seeing an unfamiliar `rule_id` in a finding.",
        "inputSchema": {
            "type": "object",
            "required": ["id"],
            "properties": {
                "id": { "type": "string", "description": "Rule id (e.g. `builtin.size.fn-length`)." }
            }
        }
    })
}

fn descriptor_get_config() -> Value {
    json!({
        "name": "get_config",
        "description": "Return the resolved Sextant configuration as JSON — verdict thresholds, size-rule limits, judge settings. Use this to debug why a rule is firing or being skipped.",
        "inputSchema": { "type": "object", "properties": {} }
    })
}

/// Dispatch a `tools/call` request. Returns either a JSON-RPC `result` value
/// (the MCP `CallToolResult`) or a `(code, message)` error pair.
pub(crate) fn dispatch(name: &str, args: Value) -> ToolResult {
    match name {
        "grade_diff" => call_grade_diff(args),
        "grade_files" => call_grade_files(args),
        "list_rules" => call_list_rules(),
        "explain_rule" => call_explain_rule(args),
        "get_config" => call_get_config(),
        other => Err((codes::METHOD_NOT_FOUND, format!("unknown tool: {other}"))),
    }
}

fn call_grade_diff(args: Value) -> ToolResult {
    let parsed: GradeDiffArgs = parse_args_optional(args)?;
    let cwd = repo_root()?;
    let report = engine_grade(
        &cwd,
        GradeMode::Diff(DiffOptions {
            base: parsed.base,
            head: parsed.head,
            working_tree: parsed.working_tree,
        }),
    )
    .map_err(internal)?;
    Ok(text_result(json_pretty(&report)?))
}

fn call_grade_files(args: Value) -> ToolResult {
    let parsed: GradeFilesArgs = parse_args_optional(args)?;
    let cwd = repo_root()?;
    let paths: Vec<PathBuf> = parsed.paths.into_iter().map(PathBuf::from).collect();
    let report = engine_grade(&cwd, GradeMode::Files { paths }).map_err(internal)?;
    Ok(text_result(json_pretty(&report)?))
}

fn call_list_rules() -> ToolResult {
    let cwd = repo_root()?;
    let rules = engine_list(&cwd).map_err(internal)?;
    Ok(text_result(json_pretty(&rules)?))
}

fn call_explain_rule(args: Value) -> ToolResult {
    let parsed: ExplainRuleArgs = parse_args_required(args)?;
    let cwd = repo_root()?;
    match engine_explain(&cwd, &parsed.id).map_err(internal)? {
        Some(rule) => Ok(text_result(format_rule_markdown(&rule))),
        None => Ok(json!({
            "isError": true,
            "content": [{ "type": "text", "text": format!("no rule with id `{}`", parsed.id) }],
        })),
    }
}

fn call_get_config() -> ToolResult {
    let cwd = repo_root()?;
    let config = engine_load_config(&cwd).map_err(internal)?;
    Ok(text_result(json_pretty(&config)?))
}

fn format_rule_markdown(rule: &RuleSummary) -> String {
    if rule.body.is_empty() {
        format!("# {} ({})\n\n{}\n", rule.name, rule.id, rule.description)
    } else {
        format!(
            "# {} ({})\n\n{}\n\n{}\n",
            rule.name, rule.id, rule.description, rule.body
        )
    }
}

fn parse_args_optional<T: serde::de::DeserializeOwned + Default>(
    value: Value,
) -> Result<T, (i32, String)> {
    if value.is_null() {
        return Ok(T::default());
    }
    serde_json::from_value(value).map_err(|err| (codes::INVALID_PARAMS, err.to_string()))
}

fn parse_args_required<T: serde::de::DeserializeOwned>(value: Value) -> Result<T, (i32, String)> {
    serde_json::from_value(value).map_err(|err| (codes::INVALID_PARAMS, err.to_string()))
}

fn repo_root() -> Result<PathBuf, (i32, String)> {
    std::env::current_dir().map_err(|err| (codes::INTERNAL_ERROR, err.to_string()))
}

fn json_pretty<T: serde::Serialize>(value: &T) -> Result<String, (i32, String)> {
    serde_json::to_string_pretty(value).map_err(|err| (codes::INTERNAL_ERROR, err.to_string()))
}

fn internal(err: EngineError) -> (i32, String) {
    (codes::INTERNAL_ERROR, format!("{err:#}"))
}

fn text_result(text: String) -> Value {
    json!({
        "content": [{
            "type": "text",
            "text": text,
        }]
    })
}
