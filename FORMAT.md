# FORMAT — Caveman Encoding Rules for SPEC.md

## 1. Tone

- Fragments, not sentences. Omit articles (a/an/the), link verbs (is/are), possessives.
- `:` = "is/are/maps to". `→` = "returns/produces".
- ∀ = "for every/all". `?` after field = optional/Nullable.
- `[]` suffix = Vec/array. `{}` = struct/record literal.
- Backtick code names, paths, identifiers verbatim.

## 2. Section Structure

```
## §X Section Title
```

Sections: §G (Goals), §C (Constraints), §I (Interfaces), §V (Invariants), §T (Tasks), §P6 (Post-v1.1 features), §B (Bugs).

Subsections use `###` or `####`.

Blank line between every entry. No prose paragraphs — each line is one fact.

## 3. §G Goals

```
G<N>: <one-line goal>
```

One line per goal. Active voice. What, not how.

## 4. §C Constraints

```
C<N>: <constraint description>
```

Design decisions, limits, conventions. References to code paths or constants in backticks.

## 5. §I Interfaces

```
api: `command_name` → `ReturnType` (description; takes params)
```

Response types use `{ field, field2 }` syntax. Parenthesized trailing annotation for side effects or notable behavior.

Events:
```
evt: `event-name` → `PayloadType { field, field2 }` (description)
```

Types:
```
type: `TypeName` → `{ field: Type, field2: Type }`
```

Enums:
```
type: `EnumName` → `Variant1 | Variant2`
```

File formats:
```
fmt: format name → description of binary layout
```

Environment vars:
```
env: `VAR_NAME` ? condition → effect
```

## 6. §V Invariants

```
V<N>: ∀ <scope> → <invariant>
```

∀ = "for all". Scope is a function name, operation, or data structure. → separates condition from invariant. One line per invariant. Number sequentially, never reuse.

## 7. §T Tasks (Pipe Table)

```
id|status|task|cites
T<N>|<status>|<one-line task description>|<citations>
```

Status values: `.` = pending, `x` = done, `>` = in progress.

Citations column references §V invariants (V<N>) or §I interfaces (I.<name>), comma separated.

No alignment padding — raw pipe table. Header row required.

## 8. §B Bugs (Pipe Table)

```
id|date|cause|fix
B<N>|<YYYY-MM-DD>|<root cause>|<resolution>
```

Date is when the bug was found. Cause is one-line root cause. Fix references the invariant §V.<N> if applicable.

## 9. §P6 Post-Release Features (Pipe Table)

Same shape as §T. Used for features added after the initial release scope.

```
P6.<N>|<status>|<task>|<cites>
```

## 10. Encoding Rules

- NO markdown formatting (no bold/italic/lists/blockquotes).
- Backticks ONLY for identifiers, paths, code snippets, type names.
- One fact per line. No line wrapping.
- ASCII only for structural markers. Chinese/Japanese/Russian in task descriptions OK.
- Preserve Rust field naming conventions: `snake_case`.
- Boolean fields without `?` default to required. `?` = Option/Nullable.
