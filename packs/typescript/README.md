# Sextant TypeScript pack

Strict TypeScript rules designed to keep AI coding agents from reaching for
the easy-but-wrong escape hatches that the TS type system offers.

## Install

```sh
sextant rules add github:kylebastien/sextant-mcp@<tag>#packs/typescript
```

To pin to a development checkout:

```sh
sextant rules add file:./packs/typescript
```

Recommended `.sextant/config.toml`:

```toml
[verdict]
max_errors = 0
max_warns = 0
max_info = 0
```

Every rule in this pack ships at `severity: error`. With `max_errors = 0`
any new violation in a `--diff`/`--pr` grade fails the gate.

## What the pack bans

| Rule | Bans |
|---|---|
| `vendor.typescript.no-any`              | `any` in any type position |
| `vendor.typescript.no-unknown`          | `unknown` (allowed only in `catch (e: unknown)`) |
| `vendor.typescript.no-object-type`      | lowercase `object` type |
| `vendor.typescript.no-empty-object-type` | `{}` as a type (anywhere outside an `interface`) |
| `vendor.typescript.no-branded-types`    | `unique symbol` types — i.e. branded / nominal types |
| `vendor.typescript.no-as-cast`          | `x as Foo` (allows `as const`) |
| `vendor.typescript.no-type-assertion`   | `<Foo>x` syntax (TS only) |
| `vendor.typescript.no-non-null-assertion` | `x!` (use type narrowing) |
| `vendor.typescript.no-ts-ignore`        | `// @ts-ignore` / `@ts-expect-error` / `@ts-nocheck` |
| `vendor.typescript.no-var`              | `var` declarations |
| `vendor.typescript.no-function-type`    | `: Function` type |
| `vendor.typescript.no-empty-interface`  | `interface Foo {}` (empty) |
| `vendor.typescript.no-eval`             | `eval()` calls |
| `vendor.typescript.prefer-inferred-types` | redundant primitive type annotations |
| `vendor.typescript.no-never-annotation` | `: never` annotations (allows `T extends X ? Y : never`) |
| `vendor.typescript.no-jsdoc-types`      | JSDoc type comments (`@type {…}`, `@param {…}`, `@returns {…}`) — **ships native autofix** |
| `vendor.typescript.no-ambient-module-shim` | empty `declare module "x" {}` shims — **ships native autofix** |
| `vendor.typescript.no-empty-type-construction` | `Pick<T, never>`, `Record<never, V>`, `Omit<T, keyof T>` |
| `vendor.typescript.no-implicit-any-field` | interface, type-literal, or class fields without a type annotation |

## Why these rules cannot be turned off

Vendor packs are loaded with **integrity checks**. Every pack file's
SHA-256 hash is recorded in `.sextant/rules.lock` and verified at every
grade. The following bypass attempts all fail loudly:

- Editing a rule file (`enabled: false`, weakening the query, ...) — hash mismatch.
- Deleting a rule file — missing-file error.
- A repo-local rule with `overrides: [vendor.typescript.no-any]` — silently ignored
  (lower-priority sources can't disable higher-priority ones).
- A repo-local rule with the same id — hard "shadows vendor pack rule" error.

The right response to a finding is to fix the underlying code. There is
no exemption mechanism by design.

## Detection details

Most rules use Sextant's tree-sitter `ast` evaluator, so matches are made
against the parsed TypeScript syntax tree — not raw text. That means:

- `any` inside a string literal or comment doesn't fire the rule.
- The rule fires on the actual type-position keyword.

For `no-unknown`, the `not_under: [catch_clause]` exemption walks each
match's ancestors and skips matches inside a `try { ... } catch (e: unknown)`
clause. Outside that exact context, `unknown` is banned — use generics.

For `no-never-annotation`, the `not_under: [conditional_type]` exemption
allows `T extends X ? Y : never` — the legitimate use of `never` inside a
conditional type — while still banning `: never` annotations on values.

## Autofix coverage

`no-jsdoc-types` and `no-ambient-module-shim` use the `regex` evaluator
with a `replacement` template, so each finding ships a proposed
unified-diff patch that consumers (the CLI's `--show-patches`, the LSP
code-action provider, the SARIF emitter, the MCP `patch` field) can
apply directly.

The other rules use the `ast` evaluator and don't carry native patches.
Set `[autofix] llm_synthesis = true` in `.sextant/config.toml` to opt
into the LLM-synthesis fallback, which proposes patches for AST-rule
findings via a second judge pass (cost-capped at
`max_synthesis_findings = 25`).
