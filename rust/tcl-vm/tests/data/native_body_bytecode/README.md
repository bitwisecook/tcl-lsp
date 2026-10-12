# Original C script Bytecode storage

The five pinned C Tcl interpreters evaluate original resident string objects through
`Tcl_EvalObjEx`. Each of five fixed sources has an object snapshot immediately
before and after evaluation. Snapshots record the actual primary class, native
reference count and resident-string presence before result or option observers.

Tcl 8.4 leaves parse failures without a Bytecode primary. Tcl 8.5–9.1 retain a
Bytecode primary that executes the parse error; a preceding return can prevent
that error from being reached. Empty and unknown-command sources retain genuine
Bytecode storage in every measured C release. The manifest records the original
producer and library/executable hashes. Jim Script storage uses a separate owner.
