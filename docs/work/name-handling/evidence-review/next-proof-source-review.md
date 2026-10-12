# Next proof source-currency review
+
+Reviewed exactly five remaining Registry owners. Two have defensible
+formatting-only deltas and three implementation joins across three records are
+proposed for renewal in [the exact allowlist](next-proof-source-review.json).
+No digest was renewed and no manifest, proof page or source was changed.
+
+Observed HEAD: `2d1f5a73bf48d252805cbb3b662601fd990e1721`.
+All five working files matched the recovered HEAD tree bytes. Exact old
+snapshots for four owners were recovered through hash-addressed Git objects;
+both their blob identities and recorded SHA-256 values were checked.
+
+| Owner under `rust/tcl-registry/src/` | Decision and exact comparison |
+| --- | --- |
+| `native_tcloo_bootstrap.rs` | Eligible: line wraps and one optional trailing array comma; complete rustfmt outputs identical |
+| `profiles.rs` | Eligible: blank-line removal; raw tokens and complete rustfmt outputs identical |
+| `definer.rs` | Declined: the test import now names `tcl_dialect::model::Family` rather than `tcl_dialect::Family` |
+| `invocation_words.rs` | Declined: Clock ensemble family, bootstrap structure and deferred-member selection added |
+| `native_numeric_error/command.rs` | Declined: exact old snapshot not recovered in bounded 500-commit first-parent lookup |
+
+Comparison used provisioned `rustfmt 1.9.0-stable (48a229ceae 2026-09-01)`
+with actual inherited crate edition **2024**, `skip_children=true` and
+`--emit stdout`. Exact original/current bytes were supplied through stdin;
+the compared output contains source only, without filename headers.
+Comments and literal contents match exactly for both eligible owners;
+their complete formatted token streams and bytes also match. Their contributed
+behavior and fixed assertion meaning are therefore unchanged.
+
+Eligible affected records:
+
+- `naming.embedding.original-host-publication-and-fact-transport`: native TclOO bootstrap owner.
+- `naming.analysis.constructed-key-consumer-projections`: Registry profiles owner.
+- `naming.core.original-registry-source-hover`: Registry profiles owner.
+
+The profiles join in the constructed-key record is distinct from the LSP
+entries excluded during the prior meaning review. The machine report includes
+every affected record for declined candidates, exact original/current hashes,
+recovered blob/revision identities and the three complete
+`(record_id, evidence_id, path, oldhash, newhash)` tuples.
+
+Only these five owners were compared; no Runtime owner was compared. A bounded
+recovery failure is not a claim that no object exists. Recheck each exact old
+binding and current source digest before any future mutation. Provider answers,
+captured observations and execution statuses must remain unchanged. No Git
+mutation, Cargo, Rust test, native provider or full proof audit ran; formatter
+equivalence adds no execution evidence.
+