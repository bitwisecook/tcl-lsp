# naming.runtime.original-nested-foreach-source-completion

Kind: `native-observation`

## Problem statement

A host implementation limit or an expectation of failure does not establish the public completion of nested original foreach source. Exact depth-bounded public controls must retain the selected provider, counted source and completion channel independently of private loop compilation or host-stack assumptions.

## Question

Do the three exact original nested foreach sources at depths 32, 128, 300 complete or raise an error on each pinned provider, and what counted public result do they return?

## Conclusion

All 18 fresh external original source processes and six actual driver compilations exit 0. Every case stderr is empty. Every C8.4.20/C8.5.19/C8.6.18/C9.0.4/C9.1.0/Jim 0.84-9-g5bac7c9 original at depth 32, 128, 300 returns completion code 0 with an empty counted result. The unchanged whole VERSION/ORIGINAL rows remain authoritative. These finite observations establish completion for these exact sources/providers/channels; they do not identify private bytecode or inline selection, command/object/instruction identity, stack layout or a general safe nesting bound. This Runtime separately enforces its explicit 128-evaluation host limit; that implementation limitation supplies no native evidence that depth 300 fails. No Rust assertion pass or BIG-IP execution follows.

## Scope

Three exact ASCII LF sources consist of a counted number of identical foreach x {1} openings, one set done 1 command, and matching closing braces. Fresh full Tcl_Init or Jim core/static-extension interpreters evaluate each complete original source directly using the unchanged native202 counted-source driver. No surrounding eval/catch/puts or stack-size override is added. Actual version and exact public counted completion remain separate rows; executable/library/header/Makefile/source and actual compiler command pins are preserved.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1, jim

Status: `observed`. Channel: Fresh external full Tcl_Init/Tcl_EvalEx(flags0) or Jim core/static extensions and counted Jim_EvalObj; unchanged independent prelude/original sources. Version: tcl8.4 8.4.20; tcl8.5 8.5.19; tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0; jim 0.84-9-g5bac7c9.

All three exact original nested sources at depths 32, 128, 300 return code0 with empty counted result. All three external case processes exit 0 with empty stderr; complete VERSION/ORIGINAL rows remain unchanged. Finite public completions do not establish private inline/bytecode/stack state, a general safe host nesting limit or a Rust pass.

### bigip

Status: `not-tested`.

No appliance execution answers these exact original nested foreach controls.

## Consumer bindings

- [runtime/rust/src/interp.rs](../../../../runtime/rust/src/interp.rs), `interp::tests::deeply_nested_foreach_reaches_the_explicit_host_evaluation_limit` (linked): The separately bounded Runtime host path expects its explicit evaluation-limit error for the identical depth300 source; this implementation limitation is independent of all-six native code0/empty observations. No executed Rust outcome is claimed.

## Exact original-source349 software outcomes

These finite results belong to the original source349 body/question/document pairs and the owned Runtime executable. Passing, failing, aborted/incomplete and unstarted selections retain their separate meanings. They establish no outcome for another source image or external provider.

| Original selection | Passed | Failed | Aborted or incomplete | Not started |
| --- | ---: | ---: | ---: | ---: |
| `grouped` | 0 | 0 | 0 | 1 |
| `isolated` | 1 | 0 | 0 | 0 |

Exact original body hashes and process outcomes join the unchanged original receipts, streams, frozen source, own inventory/admission and complete executable bytes. The grouped abort is independent of the isolated diagnostics.

## Evidence

- [rust/tcl-registry/tests/data/native_nested_foreach_source214](../../../../rust/tcl-registry/tests/data/native_nested_foreach_source214)
- [rust/tcl-registry/tests/data/native_nested_foreach_source214/8.4.20](../../../../rust/tcl-registry/tests/data/native_nested_foreach_source214/8.4.20)
- [rust/tcl-registry/tests/data/native_nested_foreach_source214/8.5.19](../../../../rust/tcl-registry/tests/data/native_nested_foreach_source214/8.5.19)
- [rust/tcl-registry/tests/data/native_nested_foreach_source214/8.6.18](../../../../rust/tcl-registry/tests/data/native_nested_foreach_source214/8.6.18)
- [rust/tcl-registry/tests/data/native_nested_foreach_source214/9.0.4](../../../../rust/tcl-registry/tests/data/native_nested_foreach_source214/9.0.4)
- [rust/tcl-registry/tests/data/native_nested_foreach_source214/9.1.0](../../../../rust/tcl-registry/tests/data/native_nested_foreach_source214/9.1.0)
- [rust/tcl-registry/tests/data](../../../../rust/tcl-registry/tests/data)
- [rust/tcl-registry/tests/data/native_nested_foreach_source214/jim](../../../../rust/tcl-registry/tests/data/native_nested_foreach_source214/jim)
