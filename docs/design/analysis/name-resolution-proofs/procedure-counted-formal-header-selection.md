# naming.procedure.counted-formal-header-selection

Kind: `native-observation`

## Problem statement

An original formal object can contain args followed by counted zero/tail bytes while the compiler header uses a different naming purpose from actual formal storage. A header pointer must not be interpreted as proof that only the displayed args formal was installed or that calls will succeed.

## Question

Does procedure definition install a compiler header for the exact counted args, args00X and whitespace-wrapped args00X parameter objects with empty bodies?

## Conclusion

All five C releases complete each of the three original definitions with code0 and a non-null Command.compileProc pointer. Parameter lengths are4,6 and8, and their actual zero/tail bytes are retained in the probe. The pointer observation supplies neither accepted call arity nor formal-storage name identity, object callback closure or body execution.

## Scope

Counted original Tcl_NewStringObj parameter/body objects passed to proc via public Tcl_EvalObjv with TCL_EVAL_DIRECT, followed by private Command.compileProc inspection. Eight definitions redefine p in one native interpreter; all observed code0. Full original source/five outputs and linked-library digests are retained; runtime version queries, compiler/executable/header digests and full stderr are not in this receipt. Jim/BIG-IP are not tested. This compiler-header purpose remains independent of declaration/name/procedure-body ownership.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: compiled public Tcl_EvalObjv TCL_EVAL_DIRECT; private Command.compileProc observer. Version: tcl8.4 8.4.20 (receipt release association; tcl8.5 8.5.19 (receipt release association; tcl8.6 8.6.18 (receipt release association; tcl9.0 9.0.4 (receipt release association; tcl9.1 9.1.0 (receipt release association.

Relevant [case,definition code,compiler-header present] rows: [[0,0,1],[1,0,1],[2,0,1]]. Recorded process status0.

### jim, bigip

Status: `not-tested`.

No native observation of this exact question is attached for this provider.

## Consumer bindings

- [rust/tcl-registry/src/native_procedure.rs](../../../../rust/tcl-registry/src/native_procedure.rs), `procedure_header_compilation_bytes`: Owns the selected header byte-purpose independently of formal storage and actual entered procedure execution.
- [rust/tcl-registry/src/native_procedure.rs](../../../../rust/tcl-registry/src/native_procedure.rs), `native_procedure::tests::byte_header_selection_matches_original_native_storage` (linked): The specific counted parameter/body controls compare NoOp/Absent header selection against actual native pointer observations, without proving allocation or call acceptance.

## Evidence

- [rust/tcl-syntax/tests/data/native_procedure_headers/probe.c](../../../../rust/tcl-syntax/tests/data/native_procedure_headers/probe.c)
- [rust/tcl-syntax/tests/data/native_procedure_headers/manifest.json](../../../../rust/tcl-syntax/tests/data/native_procedure_headers/manifest.json)
- [rust/tcl-syntax/tests/data/native_procedure_headers/README.md](../../../../rust/tcl-syntax/tests/data/native_procedure_headers/README.md)
- [rust/tcl-syntax/tests/data/native_procedure_headers/8.4.20.txt](../../../../rust/tcl-syntax/tests/data/native_procedure_headers/8.4.20.txt)
- [rust/tcl-syntax/tests/data/native_procedure_headers/8.5.19.txt](../../../../rust/tcl-syntax/tests/data/native_procedure_headers/8.5.19.txt)
- [rust/tcl-syntax/tests/data/native_procedure_headers/8.6.18.txt](../../../../rust/tcl-syntax/tests/data/native_procedure_headers/8.6.18.txt)
- [rust/tcl-syntax/tests/data/native_procedure_headers/9.0.4.txt](../../../../rust/tcl-syntax/tests/data/native_procedure_headers/9.0.4.txt)
- [rust/tcl-syntax/tests/data/native_procedure_headers/9.1.0.txt](../../../../rust/tcl-syntax/tests/data/native_procedure_headers/9.1.0.txt)
