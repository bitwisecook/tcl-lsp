# Static name consumer oracles

`observations.json` retains actual stdout, stderr and exit codes from the C Tcl
reference pins in `tcl-dialect/data/reference-toolchains.tsv`. Each program runs
in a fresh interpreter. The alias and variable programs apply to C Tcl
8.4.20–9.1.0; the class program requires built-in TclOO (8.6.18–9.1.0).
Namespace path cases are conditional on Tcl 8.5 or later.

The compiler regressions use these results for current/path/global command
precedence and non-alias shadowing through retained `SourceCommandBindings`,
plus source colon runs, empty simple names and
colon-named namespace segments through typed path inventories. Plain authored C
maps canonicalise written source before reading their compatibility keys.
Constructor inference remains compatibility advice when
command bindings, path and lifetime facts are unavailable. These observations
do not establish Jim or F5 behaviour.

Run `TCL_LSP_REQUIRE_TCLSH=1 cargo test -p tcl-compiler --test static_name_consumers`
and `TCL_LSP_REQUIRE_TCLSH=1 cargo test -p tcl-compiler --lib
constructor_types_follow_current_namespace_and_written_name_grammar` with the
five pinned interpreters available through shared oracle discovery.
