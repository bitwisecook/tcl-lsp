# KCS diagnostic-page drift audit — 2026-08-12

Source of truth: `rust/tcl-core-types/src/diag_code.rs` (187 codes) diffed
against the 164 pages under `docs/kcs/codes/`, with behaviour claims
checked against the emitters. Follow-up work items, not yet applied.

## Missing pages (user-facing, toggleable)

`W137`, `W138`, `W139`, `W141`, `W142`, `S110`, `T104`, `T105`,
`IRULE3004`. W139 is the worst gap: the W144 page links to it as a peer.
Fourteen further `internal` (non-configurable) codes have no page, which
is defensible; the codes README should say so — today its internal-codes
note names only T103/T106, and the index omits every code above.

## Retired pages

None — every page filename maps to a live code. (`W122` is retired in the
source and correctly has no page.)

## Pages contradicting the source (worst first)

1. **W304** — the worked example (`file exists $path`) cannot fire: W304
   needs an `OptionSpec` named `--`, which `file` declares only on
   `copy`/`delete`/`rename`. Page also omits `reserved_trailing_words`.
2. **W135** — names a registry field `min_version` that has never
   existed; the real carriers are the `lifecycle` fields and
   `ArgValue::min_tcl`.
3. **Severity drift** — W104, W110, W214, W220 pages say "yellow
   squiggle"; the source emits Hint (W214/W220 additionally faded via
   `DiagTag::Unnecessary`). W304 is Suggestion with escalation.
4. **W127** — page describes only the positional emitter; the
   closed-option-value emitter also raises W127, and the enabling field
   (`closed_value_args`) is never named.
5. **IRULE3102** — page presents a fixed three-command list; the real
   gate is generic (any spec declaring a `-normalized` option joins).
6. **W001 / W113 / W120 / W123 / W210** — message-text drift plus
   omitted suppressions a spec author controls
   (`allow_unknown_subcommands`, `default_form_first_word`,
   `implementation_namespace`; the W113 package/ambient exclusions; the
   W120 ambient-package and same-file suppressions).
7. **Systemic**: ~90 pages paraphrase the message after "with the
   message" instead of quoting the real `format!` string — worth one
   mechanical pass.
8. **codes README** — files W123 under Security; the source declares it
   a Hint.

## Python-era terms in pages

None found. One invented field name (`min_version`, above); the IRULE3102
fixed-list framing matches a retired hardcoded table the Rust code
explicitly replaced.

## Source-level finding surfaced by the audit

`CommandSpec::safe_on_uninit` / `SubCommand::safe_on_uninit` have **no
production consumer**: every lowering site constructs
`safe_on_uninit: false`, so `use_site_safe_initialises` can only fire on
`Statement::Incr`. W210 is actually kept quiet by `ArgRole::VarWrite`
defs. The spec-side data is stamped and correct; the wiring through
lowering is the open gap. Recorded in
[command-registry.md](compiler/command-registry.md) and the CommandSpec
reference.
