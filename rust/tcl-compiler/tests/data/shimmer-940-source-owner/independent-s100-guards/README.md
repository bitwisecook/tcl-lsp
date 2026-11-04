# Independent S100 guard comparison

This direct-link probe uses the values worker's exact emitted production rlibs
from `lsp-values-clean-target`, with digests checked before and after linking
and running. It performs no Cargo rebuild and changes neither source nor target.
Source custody: product `ff1ff34bb96468b57c110c12c590aa6a06c65c1c` plus
patch `fa5b8d7f7ccf5db1e8cd9b31c46186a3950961c5ee585005d2225830462dc0c0`,
representation child `2a477d0290db5f949d242c29ae98a16a58ebc278af53c2f64fe40499aa81ff12`
and integration test `686b203230b105e012b8fd67c468548a4960fcd74bbae1a3eeb17c55a00f1902`.

All thirteen same-held-CU comparisons pass in 15.05 seconds. Before and after
warning collection, original IR and SSA, source Normal values and separate
completes-normally gates, Native invocation/admission, and script/provider
obligations agree. Fresh Bytecode and BytecodeTraced lowering snapshots also
agree. Each original unit has no Native entry and retains AuthoredSimulation.

The top-level dynamic string-length result produces genuine S100/n. Literal
length, braced list, literal integer, unknown nested result, shadowing, rename
and write trace controls stay quiet. The named-procedure variant stays quiet
and remains an explicit coverage gap outside this RootScript foundation. The
initial broader warning assertion failure is preserved. Four original #940
conditional/font programs are observed without asserting unimplemented S102
coverage. This comparison does not claim #940 is fixed.

There are 53 source and 86 Bytecode/Traced statement observations. Existing
Native query admissions (13/11) remain unchanged; no blanket absence rule is
assumed. Bytecode statement kinds include 77 generic Call and zero NativeCall,
with existing Foreach/assignment forms retained. See per-site rows and plans.

The archived driver is a standalone probe, not an installed CI test. Its link
command is in `link.json`; set `CARGO_MANIFEST_DIR` to the repository's
`rust/tcl-compiler` for the existing fixed-input matrix, whose digest is in
`runtime.json`. Full selected sources are retained in `runtime.log`.
