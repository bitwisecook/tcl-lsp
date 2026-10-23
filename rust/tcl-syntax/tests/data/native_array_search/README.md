# Native array searches

`probe.c` calls the actual C ensemble with original handle objects. The 65 rows
record result bytes and the original handle primary cache and name offset. The
final error-state field is an ensemble observation; it is not an assertion about
a primitive getter preserving seeded state. Source-level error-update contracts
are tested separately. The manifests retain source/header/library and capture
hashes.

`search-lifecycle.tcl` records 65 actual script observations, including undefined
trace-shell collection, mutation invalidation and newest-active-ID reuse. C9
matches retained handle identity and then CString spelling; C8 caches the decimal
ID and name offset. These captures use a 64-bit C unsigned long. Independent
32-bit recipe controls do not claim to be captures from that ABI.
