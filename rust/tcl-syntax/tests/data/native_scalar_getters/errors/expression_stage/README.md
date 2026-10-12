# Seeded expression error-code stages

Ninety observations distinguish direct `Tcl_ExprObj` (route 0), direct
`Tcl_ExprBooleanObj` (route 1), and nested `Tcl_Eval` (route 2), inside an actual
native command. Each call starts with `PROBE BEFORE`. The command retains its
immediate result before the enclosing Eval propagates its return code. C8.5+
reads pending `-errorcode` with `Tcl_GetReturnOptions`; reading an absent global
`errorCode` before propagation can itself disturb the pending error state.
C8.4's immediate global state is observed because that API predates return options.

C8.4 invalid/incompatible operand types leave the seeded state unchanged in the
direct API, then Eval replaces it with NONE. NaN/domain remains explicitly ARITH
DOMAIN and cannot borrow the invalid-type record. Later C retains the actual
structured stage code. Dynamic raw String variable NUL acceptance in these direct
APIs is distinct from literal NUL in expression source; these rows do not grant a
blanket expression parser or condition contract. Jim is not covered by this probe.

Build `probe.c` against each pinned tree with the native commands documented in
`../README.md`, then run the resulting executable without arguments. The manifest
binds source, native library and all eighteen rows per release by SHA-256.
