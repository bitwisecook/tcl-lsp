# Lane: `sslictcl-loader` — finish the `.sslictcl` front end

Issue #1543 (epic #1524, sibling #1530). Branch `claude/sslictcl-loader`,
worktree `/home/user/tcl-lsp-wt-a`.

## Goal

Turn the `.sslictcl` loader in `rust/tcl-sslictcl` into an authoring-grade
front end:

1. ranged, multi-error, coded diagnostics (`SSLIC1xxx`);
2. the complete vocabulary-1 declaration set;
3. a deterministic model → text emitter with a semantic round-trip test;
4. declarative policy evaluation as a separate phase.

Everything stays a pure CST walk. No Tcl evaluation, no substitution, ever.

The registry pack for this DSL is written concurrently by another lane from
the same vocabulary spec, and a later lane publishes these diagnostics through
the LSP, so declaration names, member names, and value domains are a contract:
implement exactly, never rename or extend.

## Milestones

| # | Scope | State |
|---|-------|-------|
| 1 | `DiagSection::Sslic` + 15 codes; ranged, recovering, coded loader API | done |
| 2 | vocabulary 1 complete; facts in the estimator; `policy.rs` | done |
| 3 | `SslicModel::to_sslictcl` + fixpoint round-trip | pending |
| 4 | sample document, docs, re-exports, final gates | pending |

## Decisions

**Codes are user-configurable, not `internal`.** `diag(Sslic, true, …)`, not
`diag_internal`. Two things forced it: `cargo xtask diag-emission-check` skips
internal codes (the brief names the loader as the emission site, which only
means something for a non-internal code), and the AI catalogues
(`ai/shared/diagnostics.json`, `rust/tcl-mcp/diagnostics.json`) are built only
from non-internal, non-reserved rows. Consequence: the generated editor
settings catalogues gain a `Diagnostics — SslicTcl` group, so
`editors/vscode/package.json`, `editors/vscode/src/generated/diagnosticCatalog.ts`
and the three JetBrains settings files are regenerated in this lane. **These are
generated files only** — no hand-written editor code is touched — and AGENTS.md
requires the regenerated catalogues to be committed with the diagnostic change.

**Section placement.** `DiagSection::Sslic` sits between `Bigip` and `Tclpkg`
in declaration order (that order drives the generated table order), `as_str` is
`"sslictcl"`, and `gen_ai::section_to_category` maps it to `security` — these
are TLS-assurance findings and `security` is an existing `CATEGORY_DEFS` key.

**Ranges are `tcl_lexer::Span`**, absolute byte offsets into the *original*
document. The loader still re-parses each braced body as a fresh string, but
carries a base **offset** (not just a base line) so a nested member's span is
absolute. `Word.content_start` is `token.span.start() + token.content_offset`,
i.e. the first byte inside `{`; that is the base offset a nested block's
statements are re-based on. Line numbers are derived from a single
`LineIndex` over the whole document, so `DslError.line` / `DslNotice.line`
keep working for existing consumers.

**Recovery rule.** `load_with_diagnostics` returns `document: Some(_)` when
(a) the top-level statement stream segmented and (b) a usable
`sslictcl VERSION` header was seen. A bad statement is skipped and loading
continues; a bad member inside a block is skipped and the block continues.
`document` is `None` only for a top-level `SSLIC1001` or a missing header
(`SSLIC1003`). `load()` is the thin wrapper: it returns the *first*
`DslSeverity::Error` diagnostic as a `DslError`, so a document with only
notices still loads.

**Duplicate header** keeps the first `sslictcl` version and reports
`SSLIC1004` on the second — the document still loads.
**Vocabulary 0** is `SSLIC1009` (outside its domain); **vocabulary > 1** is the
`SSLIC1102` forwards-compatibility warning and still loads.

## Code inventory

| Site | Status |
|---|---|
| `rust/tcl-core-types/src/diag_code.rs` — `DiagSection::Sslic`, `as_str`, 15 rows, section test | done |
| `rust/xtask/src/diag_emission.rs` — `rust/tcl-sslictcl/src` search root | done |
| `rust/xtask/src/gen_ai.rs` — `"sslictcl" => "security"` | done |
| `rust/xtask/src/{gen_editor_settings,gen_vscode_package,gen_jetbrains}.rs` — `SECTIONS` row | done |
| `rust/tcl-sslictcl/Cargo.toml` — `tcl-core-types`, `tcl-syntax` deps | done |
| `rust/tcl-sslictcl/src/dsl.rs` — rewritten: `Sink`, absolute spans, recovery | done |
| `rust/tcl-sslictcl/src/lib.rs` — re-exports | done (milestones 1-2) |
| `rust/tcl-sslictcl/src/model.rs` — new typed declarations | done |
| `rust/tcl-sslictcl/src/trust.rs` — `ClientFamily`/`TrustPurpose` `as_str`+`FromStr` | done |
| `rust/tcl-sslictcl/src/vocabulary.rs` — the declaration table + drift test | done |
| `rust/tcl-sslictcl/src/policy.rs` — `evaluate_policy` | done |
| `rust/tcl-sslictcl/src/estimate.rs` — `EstimateInput.facts`, `Grade::rank`/`FromStr` | done |
| `rust/bigip-report-gen/rust/src/tls.rs` — one `EstimateInput` literal (`facts: None`) | done |
| `samples/sslictcl/example.sslictcl` | pending |
| `docs/design/sslictcl-vocabulary.md` + index link | pending |

## Deltas accepted

- `DslError` / `DslNotice` gained `code` and `range` fields. Both are
  constructed only inside `dsl.rs`, and the only workspace consumers of the
  loader are this crate's own tests, so nothing outside had to move.
- `DslError`'s `Display` now appends ` [SSLICxxxx]`.
- Loader messages were reworded slightly (`field` → `member`) to match the
  vocabulary spec's wording.
- `diag_section_as_str_covers_every_variant` in `diag_code.rs` was missing
  `Bigip`; added alongside `Sslic` so the test matches its name.

## Milestone 2 decisions

**Resolution is a post-pass.** `endpoint.chain`, `endpoint.policy`, and every
name in `chain.certificates` are recorded as `PendingRef`s while walking, then
resolved after the whole document is read, so declaration order is irrelevant.
Resolving `endpoint.chain` fills `Endpoint.certificate_chain` from the named
chain, leaving downstream consumers unchanged; `Endpoint.chain` /
`Endpoint.policy` record the names. An unresolved name is `SSLIC1011` at the
referenced word's range. Declaring both `chain` and `certificate-chain` on one
endpoint is `SSLIC1012`, ranged over both members.

**Facts are consulted before the heuristics, never instead of them.**
`EstimateInput.facts: Option<&TlsFacts>`; a declared `protocol … score N`
replaces the built-in protocol score for that version, `cipher … bits N`
replaces `cipher_strength`, `forward-secrecy` replaces the ECDHE/DHE name
heuristic for that suite, and `status prohibited` on either caps the grade at
`F` with a new `SSLICTL1104` / `SSLICTL1204` finding. Anything undeclared keeps
the old behaviour exactly, so `facts: None` is byte-identical to before.

**Policy evaluation is a separate phase.** `policy::evaluate_policy(policy,
endpoint, certificates, estimate)`. A check is the conjunction of its populated
members; a failing check yields exactly one `PolicyFinding` keyed
`(check_id, endpoint)` with code `SSLICTL-POLICY-<check_id>`, default severity
`warning`, and one evidence line per unsatisfied conjunct. `forbid-ciphers`
patterns go through the workspace's shared `tcl_syntax::glob::string_match`, so
they are exactly `string match` semantics. `grade.minimum` below the estimate's
rank produces the `grade` finding. `predicate` is retained and never evaluated
(`SSLIC1103` notice at load).

**Unknown key size fails a `min-key-bits` check.** A policy cannot demonstrate
compliance from missing evidence, so a missing leaf or unknown key size
produces the finding with explicit evidence rather than passing silently.

**The vocabulary table is executable.** `vocabulary::DECLARATIONS` describes
every declaration, its key domain, its members, and its openness.
`every_declared_word_is_known_to_the_parser` synthesises a fully-declared
instance of each declaration and asserts the loader reports neither
`SSLIC1007` nor `SSLIC1101` for it, and the converse test proves an
undeclared word is rejected by a closed block and preserved by an open one.
Adding a member to the table without teaching the parser fails the test, and
vice versa.

## Open questions

- `diag-emission-check` was not green at the milestone-1 checkpoint
  (`SSLIC1011`, `SSLIC1012`, `SSLIC1103` had no emission site yet). It is
  green from milestone 2 onward.
- `ClientFamily`'s serde spelling is kebab-case (`open-jdk`), but the
  vocabulary pins `openjdk`. `ClientFamily::as_str` emits `openjdk` and
  `FromStr` accepts both spellings; serde is untouched, so the embedded
  dataset's JSON is unchanged.
- `Member` deliberately keeps the spec's `{name, value, nested}` shape, so
  required-ness (`chain.certificates`, `certificate`'s `pem`/`material`,
  `trust-program.client`) is not in the table — it lives in the loader and in
  `docs/design/sslictcl-vocabulary.md`. If the registry lane needs it as data,
  adding a `required` field is additive.
- `purposes` is spelled `LIST` in the vocabulary table, as the contract says,
  but its elements are `TrustPurpose` spellings and an unknown element is
  `SSLIC1009`.
