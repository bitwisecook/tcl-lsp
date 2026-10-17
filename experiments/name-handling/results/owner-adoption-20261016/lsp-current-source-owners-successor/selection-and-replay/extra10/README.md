# Retained extra10 replay launcher V3

This package defines four separate calls to the repository's canonical retained-selection runner: six lexer tests, one syntax test, two SpecTcl tests, and one xtask test. The library calls use the actual combined three-library Cargo artifact manifest, with the requested target as a subset; the xtask call uses its actual binary artifact manifest. It keeps the existing 180-second test and 30-second listing budgets and the admitted Tcl/Jim provider environment.

The launcher binds V10's immutable extra-selector JSON and V11's source-cleared artifact binding, including the exact selector vectors, all actual extra build receipts, the full 39,019-row source snapshots, the five standard ELFs, and the four separate extra ELFs. It checks the canonical tracked/historical/approved-union source inventory and all source bytes before and after the invocations. It writes result files only into new, previously absent output paths.

The package is source-reviewed only. It has not run target listings or tests. The parent standard902 invocation remains separate, and neither this package nor a successful selector capture establishes ordinary acceptance.

The launcher runs all four target groups even if an earlier group fails, recording each result. It records the initial canonical inventory before invocations and records final postflight separately; drift errors cannot be replaced by a second unguarded inventory read.
