---
title: TypeScript pack
description: Strict TypeScript rules for AI agents — bans any, unknown, casts, branded types, and the rest.
sidebar:
  order: 3
---

The first shipped pack. Bans the TypeScript escape-hatches that AI
agents reach for when the type system pushes back.

## Install

```sh
sextant rules add github:kylebastien/sextant-mcp@<tag>#packs/typescript
```

Recommended `.sextant/config.toml`:

```toml
[verdict]
max_errors = 0
max_warns = 0
max_info = 0
```

Every pack rule ships at `severity: error`. With `max_errors = 0`,
any new violation in a `--diff` or `--pr` grade fails the gate.

## What it bans

| Rule id | Bans | Notes |
|---|---|---|
| `vendor.typescript.no-any` | `any` in any type position | Use a generic or a precise type. |
| `vendor.typescript.no-unknown` | `unknown` | **Allowed** in `catch (e: unknown)` only. |
| `vendor.typescript.no-object-type` | lowercase `object` type | Describe the shape with an interface or `Record<string, T>`. |
| `vendor.typescript.no-empty-object-type` | `{}` as a type | Outside of an `interface` declaration; the `no-empty-interface` rule covers `interface Foo {}`. |
| `vendor.typescript.no-branded-types` | `unique symbol` types | Branded / nominal types. Use a tagged record or a class. |
| `vendor.typescript.no-as-cast` | `x as Foo` | `as const` is allowed (it narrows literals, not the opposite). |
| `vendor.typescript.no-type-assertion` | `<Foo>x` syntax | TS-only — `.tsx` doesn't allow this anyway. |
| `vendor.typescript.no-non-null-assertion` | `x!` | Narrow with a type guard. |
| `vendor.typescript.no-ts-ignore` | `@ts-ignore`, `@ts-expect-error`, `@ts-nocheck` | Fix the underlying error. |
| `vendor.typescript.no-var` | `var` declarations | Use `const` (default) or `let`. |
| `vendor.typescript.no-function-type` | `: Function` type | Spell out the call signature. |
| `vendor.typescript.no-empty-interface` | `interface Foo {}` | Add members or remove the declaration. |
| `vendor.typescript.no-eval` | `eval()` calls | Use a real parser, JSON.parse, or a function map. |
| `vendor.typescript.prefer-inferred-types` | `const x: string = "hi"` and friends | Drop the redundant primitive annotation. |

All rules use the [`ast`
evaluator](/sextant-mcp/concepts/evaluator/#ast--tree-sitter-query),
so matches respect the parsed TypeScript syntax tree — `any` inside a
string literal or comment doesn't fire.

## Detection details

A few rules deserve a closer look:

### `no-unknown`: the `catch` exemption

```ts
// Banned
const x: unknown = parseJSON(input);

// Allowed
try {
  doWork();
} catch (e: unknown) {
  // narrow `e` here before using it
  if (e instanceof Error) console.error(e.message);
}
```

The exemption is implemented via the `ast` evaluator's `not_under`
field: a match is dropped if any ancestor node is a `catch_clause`.
Outside that exact context, `unknown` is banned.

### `no-as-cast`: `as const` stays legal

```ts
// Banned
const s = data as string;
const xs = data as ReadonlyArray<string>;

// Allowed
const tags = ["a", "b"] as const;       // narrows literal types
const direction = "north" as const;
```

`as const` is the opposite of casting away type information — it
narrows literals to their unit type. The query specifically captures
the type child of an `as_expression` and only fires when it's a real
type, not the `const` keyword.

### `no-empty-object-type` vs `no-empty-interface`

Both rules target the same anti-pattern (the `{}` type), but they
fire on different AST shapes:

| Rule | Fires on |
|---|---|
| `no-empty-interface` | `interface Foo {}` |
| `no-empty-object-type` | `type X = {}`, `function f(arg: {}) {}`, `let x: {} = …`, generic constraints, intersections |

If you want to ban `{}` everywhere, both rules should be enabled —
which is the default since both ship in this pack at
`severity: error`.

### `no-branded-types`: the `unique symbol` ban

```ts
// Banned
type UserId = string & { readonly __brand: unique symbol };
type OrderId = number & { readonly _tag: unique symbol };
const FOO: unique symbol = Symbol("foo");

// Use one of these instead
type UserId = { kind: "user"; id: string };
type OrderId = { kind: "order"; id: number };
class UserId { constructor(public readonly value: string) {} }
```

Branded / nominal types in TypeScript are typically implemented with
`unique symbol`. The pack bans the mechanism so agents reach for a
tagged record or a class — both of which TypeScript treats nominally
without the brand dance.

### `prefer-inferred-types`: only primitives

The rule fires on `const`/`let` declarations whose annotation is a
primitive (`string`, `number`, `boolean`, …) and whose initializer is
a primitive literal. It ignores:

- Annotations involving named types (`SpecialType`, `User`, …)
- Initializers that aren't literals (calls, member access, JSX)
- Declarations without an initializer (`let x: string;`)

So `const greeting: string = "hello"` fires; `const x: User = makeUser()`
doesn't.

## Bypass attempts

Because this pack is loaded via the integrity-checked vendor model,
none of the usual escape hatches work:

- Editing `rules/no-any.md` to set `enabled: false`: hash mismatch.
- Adding a repo rule with `overrides: [vendor.typescript.no-any]`:
  silently ignored.
- Adding a repo rule with the same id: load error.
- `// @ts-ignore` to silence a TypeScript error: caught by
  `no-ts-ignore`.
- Casting your way out: caught by `no-as-cast` /
  `no-type-assertion` / `no-non-null-assertion`.

See [Bypass attempts that don't
work](/sextant-mcp/packs/installing/#bypass-attempts-that-dont-work)
for the full table.

## See also

- [Rule packs overview](/sextant-mcp/packs/)
- [Installing packs](/sextant-mcp/packs/installing/)
- [`ast` evaluator](/sextant-mcp/concepts/evaluator/#ast--tree-sitter-query) — the engine that drives the pack rules.
