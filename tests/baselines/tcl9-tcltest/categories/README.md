# Per-stem tcltest categorization

Each `<stem>.toml` file here categorizes the *currently failing* tests
in `tmp/tcl9.0.3/tests/<stem>.test` against three buckets.  The default
bucket for any unlisted failing test is **must_pass**, which forces
explicit triage on every new failure.

## Buckets

| Bucket | Gating | Meaning |
|---|---|---|
| `must_pass` | **gates** the sweep — count must be 0 | Real Tcl 9 user-level semantics (variable scoping, control flow, proc dispatch, expr/arithmetic, error semantics, list/dict/string ops, catch/return, namespace resolution, command introspection).  Failure = real bug. |
| `good_to_have` | tracked count; cannot grow but doesn't gate | Real semantic features we haven't implemented yet but intend to.  Less-common subcommands, edge cases, features blocked behind another stream's work. |
| `wasm_n_a` | **skipped** — not run, not counted | Tests that probe *exact Tcl VM bytecode* on the wasm side: `disassemble` output, `tcl::test`/`testbcompiled`/instruction-count probes, `info frame` fields populated from the C-side bytecode compiler (line numbers tied to bcc instructions), `info args [info commands]` shapes that depend on the C interp's proc-table layout, segment-driver vs asyncify-only behaviours we explicitly diverge on.  We don't have a Tcl VM — we have a wasm codegen — so these tests can't pass even in principle. |
| `just_to_match_ctcl` | counted for visibility, never gating | Implementation-detail differences we don't claim to match where the test *can* run but its outcome differs in a non-semantic way: exact error wording where ours is functionally equivalent (different word order, more / fewer surrounding quotes), C-tcl interp internal table layout that is observable but not user-meaningful (e.g. `dict` insertion-order shimmer, refcount probes), display details (hex-vs-decimal of the same numeric value). |

## Schema

Each file is a flat array of `[[entries]]` structs.  All fields are
mandatory:

```toml
[meta]
bundle = "expr-old.test"      # source filename in tmp/tcl9.0.3/tests/
total = 461                   # total tests in the bundle (sanity check)

[[entries]]
id = "32.50"                  # test name suffix after "<stem>-"
bucket = "just_to_match_ctcl" # one of must_pass / good_to_have / just_to_match_ctcl
reason = "srand seed→sequence; we use Zig's PRNG, behaviour is correct"
```

Entry order is irrelevant; the loader bins by `bucket`.

## Adding entries

When a new failure appears:

1. **Default**: it goes into `must_pass` (you don't add an entry).
   The sweep will fail until you fix the bug or move it to a
   different bucket.
2. **Triage**: if you decide the failure is `good_to_have` or
   `just_to_match_ctcl`, add an `[[entries]]` block with `id`,
   `bucket`, and a one-sentence `reason` that names the root cause.
3. **Promotion**: when you fix a `good_to_have` test, delete its
   entry — the default-`must_pass` rule then keeps regressions out.

## Why per-stem

Multiple workstreams touch different bundles in parallel.  Per-stem
files merge cleanly; a single file would funnel every stream through
the same conflict point.  Use `grep -r '^id = ' categories/` for the
global view.
