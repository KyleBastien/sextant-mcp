---
id: vendor.typescript.no-implicit-any-field
name: "No implicit-any field"
description: "Bans interface, type-literal, and class field declarations that lack a type annotation. Implicit fields default to `any` under loose configs."
severity: error
category: reliability
scope: file
languages: [typescript, tsx]
evaluator:
  type: ast
  query: |
    (property_signature !type) @p
    (public_field_definition !type !value) @p
  capture: p
  message: "no implicit-any field — annotate the type or give it an initializer"
tags: [strict, types]
---

# No implicit-any field

A property declared without a type annotation — and, for class
fields, without an initializer that the compiler can use for
inference — is `any` under the default `tsc` config and an unsound
shape once `strictPropertyInitialization` is turned on.

```ts
// Banned
interface User {
  id;
  email: string;
}

class Cart {
  count;
  ready: boolean = false;
}

type Settings = {
  theme;
  locale: string;
};
```

Even when strict mode catches the immediate issue, the failure mode
isn't great: tooling that consumes the type sees `any`, the diff
that introduces the missing annotation looks innocuous, and reviewers
miss it.

**Do this instead** — always annotate:

```ts
interface User {
  id: string;
  email: string;
}

class Cart {
  count: number = 0;
  ready: boolean = false;
}
```

The rule fires on:

- `property_signature` nodes (interface and `type = { … }` members)
  with no `type` field.
- `public_field_definition` nodes (class fields) with no `type` field
  **and** no `value` field — i.e. a bare `field;` declaration. Class
  fields with an initializer (`readonly start = () => {}`) get their
  type from the initializer and aren't covered.

It uses tree-sitter's `!type` negative-field predicate — exactly the
same field name the [`prefer-inferred-types`
rule](./prefer-inferred-types.md) inspects when it asserts the field
*is* present.

**Note:** if "any" is genuinely intended on a field, write it
explicitly — and the [`no-any` rule](./no-any.md) will still object.
That's the point: every implicit `any` should be either a precise
type or an explicit, audited `any`.

**Cannot be disabled:** the lock-integrity check rejects edits to
this file.
