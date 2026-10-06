# Native error-variable callbacks

`probe.c` records ten controls per audited C release. It reads private interpreter fields and physical root cells directly, avoiding a guest error-variable read as an observer. Native object equality is recorded before any report formatting. The manifest pins the source, header, library and observation bytes.

C8.4 has ordinary lazily created globals. C8.5+ starts with undefined error cells and hidden read/unset traces. Read copying requires the legacy-copy flag and retains the same private object; a missing private object only defines an empty public value when needed. Unset reinstalls an undefined root cell. Reset publishes code before info and releases private references.

Private producer controls invoke the actual `error` worker with original ByteArray operands. C8.5+ retains the information object and validates the code on the same object into a List; default classification codes are Lists. The reset controls capture private and physical global objects before any guest observer lookup. Direct native traces see the additional owner held by `Tcl_SaveInterpState`. Script traces enter an evaluation that resets the remaining private fields: the information callback precedes the outer code callback, and both bodies see null private fields. The globals retain the original operands after reset.

`producers.c` and `producer-script-traces.c` have separate manifests containing exact source, header, library, and output hashes for all five audited C releases. These receipts concern original private objects and actual callback order; equal bytes alone do not establish object or cache identity.
