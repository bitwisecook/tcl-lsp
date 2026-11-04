# #2376 current production baseline

This is an observational baseline, not a passing fix. Source is
`8e4ec201404463ffdd2f265bec2a4d5bd97e5bf6`; its production files were unchanged.
The exact driver/input hashes matched before and after the build/run.

Compilation used the shared wrapper with `--no-run`; the emitted binary ran
directly outside its lock. The recorded baseline environment explicitly
bypassed proposed acceptance assertions to collect all 115 case/profile rows.
The two successful probe tests mean observation completed, not that expected
warnings or values passed.

Across Tcl 8.4–9.1, safe AND, negated OR, double-not AND and a true OR guarding
another variable each retain a false W210. The unsafe OR/negated AND and
standalone missing-read controls warn. The later real-read case is reported at
the earlier skipped read, reproducing the deduplication defect. Every retained
final-Normal probe is unavailable; the fix must preserve that absence, rather
than manufacture published completion. Mutation-order evidence belongs to the
existing internal model and the separately recorded native controls.

The completed-value proposal remains unapplied at this baseline. No code or
failing regression was installed in CI by this checkpoint. Full gates remain
pending; this successful probe compilation is not a rust-check/prep-pr pass.
