# Legacy-diagnostic corpus

`corpus/` holds hand-written Tcl and iRule snippets covering the six
`find-legacy` diagnostic codes — `W100`, `W104`, `W110`, `W304`,
`IRULE2001`, and `IRULE5001` — plus a few mixed files that exercise
nested-body recursion and dialect-disabled commands.

It is a fixture set, not a suite of its own. The false-positive sweep
consumes it as one of its corpora:

```sh
cargo xtask fp-sweep --code W100 --corpus scripts/dev/diag_parity/corpus
```

Point any other corpus-driven harness at it the same way. Keep the
snippets small and single-purpose: each one should make exactly the
point its filename claims.
