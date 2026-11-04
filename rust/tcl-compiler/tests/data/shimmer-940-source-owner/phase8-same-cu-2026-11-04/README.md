# Independent S102 same-compilation-unit guard receipt

This is a Rust implementation check against the sealed phase-eight candidate,
not a native Tcl execution record or a claim that the original #940 issue is
fully repaired. The candidate source is HEAD
`6c97a1cdc54ab089f6a4c9e7086177c7106ec531`, patch SHA-256
`89d75e8f0100e49c055525d6dd12d7ca025851e61752f285ed438cde5c8ddd5a`
and child-file SHA-256
`7e50c02d04472f38e8890f65f0126f77ce9c26b3ac3379af24692dac6b9d5720`.
The complete exact emitted library list and its hashes are in `link-receipt.json`.
No Cargo command or target write occurs in this check.

The retained standalone runner links the exact production libraries and builds
one compilation unit per source/profile. It compares that same held unit before
and after warning collection and `run_all_checks`; it does not rebuild a second
unit for the comparison. Bytecode and BytecodeTraced are separate lowerer
observations using the held unit's source options, compared before and after.
Each snapshot queries existing original source tokens, native compiler admission,
Normal result bytes, complete-normal proof, script/procedure host-admission plans
and provider/source/failure obligations. Existing positive capabilities remain
recorded; no blanket must-be-None assertion hides them.

All 65 rows pass: thirteen fixed sources across Tcl 8.4, 8.5, 8.6, 9.0 and 9.1.
The real carried int/list cycle reports exactly one S102 for `x` at offset 43,
and the full checks report the same span with no replacement, fixes or context
action authority. Invalid and copied-invalid seeds, zero/one trips, unknown
iteration lists, binder/body resets, in-loop handler replacement and traces,
and the original conditional/unconditional accumulators and font list remain
quiet. `summary.json` contains all exact sources and counts.

Held IR and every original SSA/body identity stay equal. Admission, conditional
Normal values, complete-normal proofs and provider facts are equal before and
after. Bytecode/Traced snapshots are equal. All emitted rlibs remain byte-exact
through link and the 53.17-second runtime. The compressed log retains every row
and both full snapshots; the source, binary, library and raw-log digests are
recorded alongside it. This verifies the selected candidate's read-only warning
projection; it grants no additional execution authority. Full workspace gates
and the candidate's subsequent lint-only source extraction are separate.
