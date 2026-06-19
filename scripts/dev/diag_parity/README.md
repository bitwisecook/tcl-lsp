# `diag` parity harness

Differential debugging harness for the Rust analyser port. It runs both
diagnostic engines over a corpus and classifies every per-diagnostic
divergence, producing a ranked frequency table that drives the analyser
parity work (and is the regression oracle for it).

- **Ground truth:** `python -m tooling.tcl.main diag --dialect D FILE --json`
- **Under test:** `target/debug/tcl diag --dialect D FILE --json`

## Run

```sh
cargo build -p tcl-cli --bin tcl            # build the binary under test
python scripts/dev/diag_parity/run.py       # committed corpus only
python scripts/dev/diag_parity/run.py --tcl-tests 40   # + sampled .test files
python scripts/dev/diag_parity/run.py --show-codes W100,IRULE2001
python scripts/dev/diag_parity/run.py --json report.json
```

## Corpus

`corpus/` holds hand-written legacy snippets covering all six `find-legacy`
codes (`W100`, `W104`, `W110`, `W304`, `IRULE2001`, `IRULE5001`) plus a few
mixed files that exercise nested-body recursion and dialect-disabled commands.
`--tcl-tests N` additionally samples N real `*.test` files from
`tmp/tcl*/tests/` (deterministically seeded) for broader `diag` coverage.

## Divergence kinds

| Kind | Meaning |
|---|---|
| `MISSING_FIRE` | Python emits the code, Rust does not (py-only). |
| `EXTRA_FIRE` | Rust emits the code, Python does not (rust-only). |
| `WRONG_POSITION` | Same code, paired occurrence at a different line/column. |
| `WRONG_MESSAGE` | Same code + position, different message. |
| `WRONG_SEVERITY` | Same code + position, different severity tier. |
| `WRONG_ORDER` | Identical diagnostic multiset, different emission sequence. |

## Reading the table

Python is the parity bar, so `MISSING_FIRE` / `WRONG_POSITION` / `WRONG_MESSAGE`
generally mean "fix Rust". But not always: where Rust is **more correct** than
Python (a real Python bug), an `EXTRA_FIRE` or position difference should be
fixed on the Python side rather than regressing Rust — see
`docs/rust-cli-port.md` for the established precedent of documenting
Python-side bugs.
