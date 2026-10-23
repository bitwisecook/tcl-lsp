# KCS: feature — Minifier

> **Audience:** User
> **Type:** Functionality

## Summary

Minification removes comments and unnecessary spacing. For Tcl and Jim, basic
mode also removes safe spacing inside selected script bodies and braced
expressions. Quoted values, variable spellings, expression string data and
expansion prefixes retain their meaning. Compact and aggressive modes report
when a requested naming or semantic pass is unavailable.

## Applies to

All editors, the Tcl CLI, and the F5 CLI.

## How to use

- **VS Code:** `Tcl: Minify Document` (Ctrl+Alt+M / Cmd+Alt+M).
- **JetBrains:** **Tools → Tcl → Minify Document**.
- **CLI:** `tcl minify script.tcl`, or request further compaction with
  `tcl minify --compact script.tcl --symbol-map map.txt`.
- **LSP:** `tcl-lsp.minifyDocument` accepts
  `(uri, compact?, aggressive?, isolated?)`.

The CLI prints unavailable-pass reasons to stderr and still writes any supported
syntax compaction. An empty symbol map means no names were changed. The editor
result includes `source`, `originalLength`, and `minifiedLength`; requested name
changes and optimiser counts have separate result fields.

## What each mode does

| Mode | Tcl/Jim source with a retained native naming recipe | Explicit lexical authoring simulation |
|---|---|---|
| Basic | Removes comments and compacts separators in selected scripts and braced expressions; preserves expression tokens and literal data | Applies the supported recursive body and expression formatting passes |
| Compact | Basic compaction plus supported closed local scalar compaction; isolated procedure/global renaming is refused | Naming passes are refused; supported syntax compaction still runs |
| Aggressive | Includes supported local scalar compaction; other name changes, inserted aliases, semantic rewrites, and keyword shortening report their independent missing checks | Naming and semantic passes are refused; supported syntax compaction still runs |

The selected analysis determines which column applies. Changing a dialect label
or passing `--isolated` does not turn a Tcl/Jim refusal into permission.

## Example

**Input:**

```tcl
# top-level comment
  proc greet {name} {
    puts "hello $name"
  }
  greet world
```

**Basic Tcl output:**

```tcl
proc greet {name} {puts "hello $name"};greet world
```

The tool checks each selected script under the same source configuration.
A braced expression such as `{3 + 4 * 2}` becomes `{3+4*2}` only when its
selected expression tokens remain unchanged. Spaces inside string data remain.

## Limits

- Malformed input is retained unchanged rather than emitting only its valid
  prefix. Stale analysis or unavailable original source/configuration also
  prevents changes.
- Basic compaction changes source layout and line numbers. It does not claim
  identical error locations, source reflection, or interpreter observations.
- Tcl/Jim formal compaction currently requires a complete source containing
  one procedure, one required scalar formal, and a body that only returns that
  formal as one object. It preserves the public procedure name. Defaults, rest
  arguments, extra commands, conversion, links, observers, or incomplete source
  references prevent this pass. Arbitrary external procedure introspection is
  outside this closed-source transformation contract.
- Global and procedure renaming, inserted aliases, general semantic rewrites,
  and keyword shortening still require their own checks. Isolation alone does
  not supply them; supported syntax or scalar-formal compaction can still run.
- Isolated procedure and global renaming is unavailable in compact and
  aggressive modes. Both modes return a refusal and retain supported syntax
  compaction; isolation does not override that restriction.
- An execution observer or an unresolved command selection prevents expression
  gap changes. Dynamic or malformed expression syntax remains unchanged.
- Array indices and package names are not compacted symbols.

## Operational context

The minifier uses the current complete source and source configuration. It
checks syntax compaction separately from name changes and inserted assignments.
Available names or correctly escaped replacement text do not themselves
permit a new store, movement of evaluation, or a change in observable names.

## Failure modes

- A changed source or configuration makes the analysis stale.
- A malformed command prevents complete-word compaction.
- A requested naming or insertion pass reports that its independent checks
  are unavailable.

## Test anchors

- `rust/tcl-lsp-core/src/minify/original.rs` — original-word correspondence,
  stale-source and malformed-tail refusal, separate naming permissions, and
  lexical authoring behaviour.
- `editors/vscode/src/test/commandExecution.test.ts` — editor-visible behaviour.

## See also

- [KCS: Unminify Error](kcs-feature-unminify-error.md) — translate compacted
  names in an error using an available symbol map.

## Discoverability

- [KCS feature index](README.md)
