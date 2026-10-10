# Native dictionary body controls

The fixed scripts keep compiled and generic `dict update` and `dict with`
entry paths separate. They cover body completion, root replacement and removal,
read and write callbacks, and original raw-NUL, modified-NUL, invalid-byte and
array-looking key objects.

The six tables contain 329 evaluated code/result references from complete
native processes. C Tcl 8.4 records command absence; Jim trace cases record its
trace command absence. Seven signal-terminated write-error controls remain in
the manifest with original raw output and exit status, outside positive tables.
Each original counted script is retained in `cases.tsv`; the C probe, native
libraries, headers, executables and raw logs have recorded hashes.

The command catch runs before trace removal and final variable observations.
These references describe the final evaluated result containing that capture;
they do not certify private return state or unobserved object cache effects.

The current pinned C8.5.19 `compiled-update-dict-write-error` lifetime probe
records final release of the dictionary while its defined scalar cell still
selects the same header. Its outer completion, error message and caller read
status remain bounded public comparisons. The later caller-value bytes grant
no defined storage-equivalence contract. The VM checks all328 complete result
windows and those valid fields in the remaining completed row, and separately
checks its live owned dictionary after the write error. The instrumented current
build and the independent original public capture remain distinct evidence; see
[the lifetime proof](../../../../../docs/design/analysis/name-resolution-proofs/dictionary-c85-original-update-write-error-object-lifetime.md).
