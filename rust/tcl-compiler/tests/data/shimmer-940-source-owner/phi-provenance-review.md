# S102 phi evidence review

This is a read-only design review, not an implemented or validated compiler
change. Restore the actual source-body facts before deciding the S102 verdict.

`classify_thunking_phi` currently imports the entire union of a shimmered
backedge whenever any statement in the loop reads and writes the same symbol.
A conditional append merges its actual List definition with the untouched
header version. That untouched version can still carry the entry empty String.
Importing both types labels the entry representation as though the body
recreated it on every iteration.

A narrow repair would retain the shimmered backedge as a candidate, omit that
union import, and use the actual Known definitions collected by
`per_loop_body_types`. It must be tested with real branch definitions. Quiet
controls whose bodies are opaque calls cannot establish this repair.

For a stronger provenance rule, use the existing SSA/def-use owners:

1. Use the shared natural-loop owner and actual executable edges. Keep the
   current `Symbol`/`Version` and retained cell identity throughout.
2. Follow a backedge through intermediate phi inputs to actual in-loop
   statement definitions. A return to the same header phi is pass-through,
   supplying no body-created representation. Do not treat a phi's joined
   lattice as a representation-producing operation.
3. Count only actual value definitions. Exclude empty resets, `may_defs`,
   `destruction_defs` and synthetic array-root refreshes. Unknown/overdefined
   types do not supply a representation by elimination.
4. Require a real consumed-version dependency for repeated conversion.
   Different independent assignments of fresh values do not prove conversion.
   A symbol-wide self-reference somewhere else in the loop is insufficient:
   inspect the version read by the operation that consumes the representation.
5. Preserve conversions within an iteration even when the header has one
   stable type. `lappend acc ...; set acc "$acc,"` consumes the prior
   accumulator, then its List result, on every pass. A branchy expr/range loop
   similarly retains the two actual source operations and their dependencies.
6. Apply the shared externally-mutable/alias/trace refusal to every header
   classification. Do not combine evidence from sibling loops or independent
   storage cells. Keep source ownership separate from current Native execution.

`per_loop_body_types` presently scans the loop's blocks without filtering
executable blocks or excluding each statement's may/destruction definitions.
It is an evidence boundary to refine, not independent proof that every listed
type was produced by a reached value write.

The existing test `thunking_detected_for_two_type_loop_body` expects S102 for
`set x "s"; set x [list 1]`. Its expectation is semantically wrong. The retained
`phi-provenance-oracles.tcl` runs three iterations and inspects both objects.
C Tcl 8.6.18, 9.0.4 and 9.1.0 each reuse one stable pure-string object and a
different stable List object. Neither object changes representation. The
fresh sequential and branchy reset cases in the matrix therefore expect no
S102, independently of that pre-existing assertion.

The same probe confirms repeated Int/String values in the dependent expr/range
loop, and repeated List/String conversions in the dependent
lappend/interpolation loop. Its explicit same-object control uses `lindex`
followed by numeric `expr`: the same object alternates List/Int on all three
represented C Tcl versions. `llength` is unsuitable for that particular control
on Tcl 9, which can answer directly from an integer without converting it.
Tcl 8.4.20, 8.5.19 and JimTcl also execute the complete portable program, but
they do not expose the same unsupported representation API; no representation
claim is made for them. Raw output and exact input SHA256 are retained in
`phi-provenance-oracles.json`.
