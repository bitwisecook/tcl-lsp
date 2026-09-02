# Lane: SslicTcl LSP authoring end to end (#1543, epic #1524)

## Goal

Finish issue #1543. Two earlier lanes on this branch already landed the
`.sslictcl` loader (`rust/tcl-sslictcl`) and the `sslictcl` authoring
*dialect* (profile, environment, detection, registry pack,
`DefinerFamily::SslicTcl`, regenerated editor catalogues). What remains is
the LSP surface an author actually sees:

1. Publish `tcl_sslictcl::dsl::load_with_diagnostics` findings as ordinary
   document diagnostics, with precise UTF-16 ranges.
2. Make the analyser leave a well-formed `.sslictcl` document alone
   (no unknown-command noise on extension words, no W127 on `enabled TRUE`,
   no analysis of a `predicate { … }` body) — with a dialect-level policy,
   never a declaration-name check.
3. A native e2e suite `rust/tcl-lsp-server/tests/e2e/sslictcl.rs`.
4. A VS Code mocha test over a `.sslictcl` fixture.
5. Owner-map rows for `tcl-sslictcl` in the shared-utility contract.
6. Docs.

## Design decisions

### Diagnostics are a projection, not a second validator

`DslDiagnostic` already carries a published `tcl_core_types::DiagCode`, a
`DslSeverity`, a `tcl_lexer::Span` of absolute byte offsets, and a message —
which is precisely the shape of `tcl_compiler::analyser::Diagnostic`. So the
new provider `rust/tcl-lsp-core/src/sslictcl_diagnostics.rs` maps one to the
other and stops there:

* `diagnostics(source, disabled, suppressed) -> Vec<analyser::Diagnostic>`
  runs the loader, drops codes the user disabled
  (`tclLsp.diagnostics.<CODE> = false`) and codes suppressed by `# noqa` /
  `# tcl-lsp: disable=…`, and maps `Error`/`Warning`/`Hint` across.
* the server then lifts them with its existing
  `lift_analyser_diagnostics`, which is what gives them the same
  `source: "tcl-lsp"`, the same code spelling (`DiagCode::as_str`), the same
  `finalise_diagnostics` tag/severity-override treatment, and — the point —
  the same UTF-16-correct `lift_span` conversion every other diagnostic gets.

No new lift machinery, no new wire shape, and nothing in the module names a
declaration.

### Routing is by resolved authoring surface, not by dialect name

`applies_to(&DialectProfile)` asks
`document_context_for_profile(profile).authoring_query().packages` for the
`sslictcl` package — the exact shape `Backend::is_bigip_dialect` uses for
`bigip`. Aliases (`sslic-tcl`, `tls-sslictcl`) and every detection route
(extension, editor language id, the `sslictcl VERSION` content signature)
fold into the same answer.

The server branch is the `xc_for_irules` shape, in both diagnostic surfaces:

* push — `refine_and_lift_diagnostics` (`rust/tcl-lsp-server/src/lib.rs`),
  inside the existing `spawn_blocking` worker, after the XC block.
* pull — `Backend::full_diagnostics_for`, same position.

Both were required: the pull path recomputes its report rather than reading
the push cache, so a single wiring point would have left pull-mode editors
short.

The loader parses the *analysis* form of the buffer (`normalise_lone_cr`) and
the spans lift against the client's buffer; the rewrite preserves byte
length, so the two agree on every offset.

## Site inventory

| Item | Status |
|---|---|
| `rust/tcl-lsp-core/Cargo.toml` — `tcl-sslictcl` dependency | done |
| `rust/tcl-lsp-core/src/sslictcl_diagnostics.rs` + `lib.rs` module row | done |
| Push wiring (`refine_and_lift_diagnostics`) | done |
| Pull wiring (`full_diagnostics_for`) | done |
| wasm / wasi `cargo check` | remaining |
| Analyser interaction (item 2) | remaining |
| `tests/e2e/sslictcl.rs` + `tests/e2e.rs` row | remaining |
| server `lib.rs` language-id table + `dialect_for_open` test | remaining |
| VS Code mocha test + fixture | remaining |
| Owner-map rows + `AGENTS.md` row | remaining |
| Docs (`docs/capabilities.md`, READMEs, vocabulary doc) | remaining |

## Open uncertainties

* Whether the crypto crates behind `tcl-sslictcl` (x509-parser, rsa, p256,
  p384) build for `wasm32-unknown-unknown` / `wasm32-wasip1` once the LSP
  crates depend on them. The crate already declares the `getrandom` js /
  wasm_js backends for the browser target, which suggests they were expected
  to. If not, the fallback is a default-on `crypto` feature on
  `tcl-sslictcl` and `default-features = false` from the LSP crates.
* The brief asks for a test that "`tclLsp.diagnostics.exclude` with
  `SSLIC1101` suppresses only that code". `tclLsp.diagnostics.exclude` is a
  **file glob** list (#1556), not a code list; the per-code switch is
  `tclLsp.diagnostics.<CODE>: false`. Both are covered by the e2e suite,
  under their real meanings.
