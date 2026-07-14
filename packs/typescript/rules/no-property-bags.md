---
id: vendor.typescript.no-property-bags
name: "No property-bag types"
description: "Bans open, primitive-valued maps like `Record<string, string>` and index signatures `{ [k: string]: T }`. Name the shape or type the value."
severity: error
category: reliability
scope: file
languages: [typescript, tsx]
evaluator:
  type: ast
  query: |
    (generic_type
      (type_identifier) @name
      (type_arguments (predefined_type) @key (predefined_type))
      (#eq? @name "Record")
      (#match? @key "^(string|number)$")) @g
    (index_signature
      (predefined_type) @idx
      (#match? @idx "^(string|number)$")) @g
  capture: g
  message: "no property bag — describe the shape with an interface, or use `Record<K, NamedType>` / a `Map` for a genuinely dynamic map"
tags: [strict, types]
---

# No property-bag types

A "property bag" is an open-keyed collection whose values are primitives —
`Record<string, string>`, `Record<string, unknown>`, or an index signature
`{ [key: string]: number }`. Like `any` and `object`, it tells every consumer
*nothing* about what's actually inside. The keys are unbounded and the values
are untyped-in-spirit, so autocomplete, refactors, and exhaustiveness checks
all stop working at the boundary.

```ts
// Banned
type Headers = Record<string, string>;
type Cache = Record<string, unknown>;
interface Flags { [key: string]: boolean }
type Lookup = { [id: number]: string };
```

**Do this instead:**

- If the keys are known, name them — that's the whole point of a type:
  ```ts
  interface Headers {
    contentType: string;
    authorization: string;
  }
  ```
- If you have a genuinely dynamic map, give the *value* a named type. A
  Record whose value is a real type is fine — it's the primitive-valued,
  open-keyed form that's banned:
  ```ts
  type UsersById = Record<string, User>;   // allowed — named value
  ```
- If the keys are dynamic *and* the values are primitive, that's a runtime
  data structure, not a type — reach for a `Map`:
  ```ts
  const counts = new Map<string, number>();
  ```

**What fires, and what doesn't:**

- `Record<K, V>` fires only when the key is `string`/`number` **and** the
  value is a primitive (`string`, `number`, `boolean`, `unknown`, `any`,
  `object`, `symbol`, `never`, `void`). Named or structured values
  (`Record<string, User>`, `Record<string, Widget[]>`,
  `Record<string, { id: string }>`) are allowed.
- Finite key unions are allowed — `Record<"a" | "b", string>` is an
  enumerated map, not an open bag.
- Index signatures (`{ [k: string]: T }`, `{ [k: number]: T }`) fire
  regardless of value type: the index-signature syntax is itself the smell.
  Use `Record<K, NamedType>` or named fields instead. Mapped types
  (`{ [K in keyof T]: V }`) are a different construct and are allowed.

**Cannot be disabled:** the lock-integrity check rejects edits to this
file. If you find a case where the rule is wrong, the right fix is to make
the rule smarter, not to turn it off.
