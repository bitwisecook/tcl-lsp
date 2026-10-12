# Native concat argument expansion

Each C fixture calls the original `Tcl_ParseCommand`, records token types and exact source extents, defines a procedure through `Tcl_EvalObjv`, and records its actual result header, bytecode instructions and literals. The Jim fixture uses `Jim_CreateInterp`/`Jim_RegisterCoreCommands` and original `Jim_EvalObjVector` operands; its Script body and result are recorded separately from C bytecode.

Pure TEXT expansion with literal list members becomes C SIMPLE_WORD tokens before compiler selection. Empty expansion removes the original word. Backslash-generated, substituted and malformed expansions retain EXPAND_WORD and generic expanded dispatch. C8.4 rejects the expansion syntax; C8.5 has no concat compiler hook. C8.6–9.1 compile projected dynamic operands with CONCAT_STK and fold projected constants through the selected release's literal allocation. Jim performs Script evaluation without a C compiler receipt and preserves its lenient list grammar.

The manifest records source, native header/parser/compiler, library and executable hashes, fixed 60-second budgets, and both shared execution slots. Results and primary/reference windows are observed before result string materialization.
