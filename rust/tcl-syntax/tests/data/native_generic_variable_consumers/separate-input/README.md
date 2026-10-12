# Original separate variable inputs

The 50 List/header rows observe the public C `Tcl_ObjSetVar2` operation in
Tcl 8.4.20, 8.5.19, 8.6.18, 9.0.4 and 9.1.0. Each original root and separate
original index remains owned across the operation. `Write` requests
`TCL_LEAVE_ERR_MSG`; `QuietWrite` uses flags zero. The operation writes the
original String value `NEXT`, then records its selected result, interpreter
result, root/index primaries, reference counts and String residency.

Both backend comparisons prepare the original operands, store through the
selected physical receiver, and verify the original value returned by readback
before observing headers. Assignment retains the interpreter result. A transient
lookup-only undefined element has its own release lifetime and is not this setter
observation.

The five shapes are a missing namespace, a scalar root, an existing array,
a fresh parenthesised root, and the same root after an authentic combined-name
cache has been installed. C8.4 can report a fresh collision and continue parsing;
later C releases reject it. Reporting and quiet failures retain their own getter
stages. `getter-stage/` contains 50 native extension-updater/header controls for
those stages.

The manifests identify the exact compiler, consumed headers, linked library,
probe and raw outputs. Native patch levels reference the independently captured
`tcl_patchLevel` result from the same SHA-identified libraries in
`../part2-purpose-manifest.json`. `rows.tsv` projects the raw List/header fields
without changing their expected bytes.
