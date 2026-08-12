# Formatter engine contracts

How Tcl source is reformatted, and the rules that keep the output stable
between runs and consistent with what the parser and the language features
expect of it.

Formatting is an engine + config pipeline in `tcl-lsp-core`, surfaced through
the LSP formatting handlers, the `tcl fmt` CLI verb, and the MCP tools.
`format_tcl(source, config, registry)` is a **pure function**: source in,
formatted source out. It parses the source into commands, identifies
body / expr / param-list arguments **through the registry** (never a
command-name list in the formatter), recursively formats bodies, and
reconstructs the output — K&R braces, blank-line policy, comment
normalisation, switch bodies, long-line backslash splitting, and `&&`/`||`
expression wrapping.

Document-wide command-identity facts (what a head word actually binds to, after
`rename` / `interp alias`) are computed once per file and threaded through, so
the formatter's registry lookups agree with the analyser's.

## Decision rules / contracts

1. **Formatting is idempotent.** `format_tcl(format_tcl(x)) == format_tcl(x)`,
   including for structurally malformed input — an unbalanced `{`, `[`, or `"`
   must reach a stable shape rather than being reshaped into a guess and
   growing by a delimiter on every pass. The engine's own tests
   (`malformed_clauses_stay_stable_and_idempotent`,
   `param_list_shapes_round_trip_and_are_idempotent`) assert exactly this.
2. Formatting preserves parseability and command semantics. A rewrite that
   changes what the script does is a defect regardless of how it looks.
3. Body / expr / param-list classification is registry-driven (`ArgRole`,
   `Traits`). A new construct is formatted by declaring it in the registry, not
   by adding a branch to the engine.
4. Recursion into nested bodies is depth-capped (`MAX_FORMAT_DEPTH`, 128).
   The crate is consumed both from binaries that format on a generously-sized
   dedicated thread and, through `bigip-query-wasm`, from a WASM host whose
   stack budget this crate does not control, so the cap is set to be safe on a
   small ambient stack. The minifier carries the same discipline with its own
   cap.
5. A new formatting option requires config wiring plus regression coverage.
   Settings are declared once on `FormatterConfig` and code-generated into the
   editor extensions by `cargo xtask gen-editor-settings`.
6. The formatter never rewrites an *existing* docstring. Docstring generation
   is an explicit code action — see
   [docstring-handling.md](docstring-handling.md).

## File-path anchors

- `rust/tcl-lsp-core/src/formatting/engine.rs` — `format_tcl` and its machinery.
- `rust/tcl-lsp-core/src/formatting/config.rs` — `FormatterConfig` and every
  setting.
- `rust/tcl-lsp-core/src/formatting/keywords.rs` — the keyword-canonicalisation
  rewrites (themselves idempotent).
- `rust/tcl-lsp-core/src/formatting/docstring.rs` — docstring parse/render.
- `rust/tcl-lsp-core/src/minify.rs` — the minifier, which shares the
  registry-driven body classification.
- `rust/tcl-syntax/src/format.rs` — value-level formatting primitives.

## Failure modes

- Non-idempotent rewrites that keep changing on repeated format operations.
- Body / expr boundary misclassification causing a semantic change.
- Option-specific regressions from missing config propagation.
- Unbounded recursion on deeply nested bodies (guarded by the depth cap).

## Discoverability

- [Design doc index](../README.md)
- [LSP feature providers](lsp-feature-providers.md)
- [parsing contracts](parsing.md)
