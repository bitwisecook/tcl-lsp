# naming.try-original-compiler-topology

Kind: `native-observation`

## Problem statement

Computed handler selectors, continuation bodies, trap prefixes and finally overrides can change both compiler admission and runtime completion. A syntactic handler role must not supply a compiled-local or completion receipt.

## Question

How do the original try sources 19 through 39 select handler locals, bytecode ranges and completion results?

## Conclusion

The retained C tables distinguish each original try/finally/on/trap source, local layout, instruction/range selection and completion result. Pre-try releases retain guest command failures rather than a fabricated exception compiler. No resulting source role proves that a constructor or callback completes.

The marked Runtime consumer definition compares63 public Guest completion/result pairs and36 selected actual Operation::Try-presence assertions in cached procedure artifacts.

## Scope

C Tcl releases explicitly identified in the retained provider rows. Jim and BIG-IP are not tested unless a separate row identifies them. Cases19–39 of the unchanged40-source procedure probe, genuine compiled layouts after the call and exact result hex.

These source assertions have no attached run outcome. They cover the unchanged21 case sources19..39 only for C8.6.18/C9.0.4/C9.1.0, not C8.4/C8.5/Jim/BIG-IP. Recipe presence does not compare exact disassembled opcodes, catch ranges, local layout, result refcounts or independent effect chronology, and grants no fresh external native execution or private frame/header observation.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description. Version: tcl8.4 8.4.20; tcl8.5 8.5.19; tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0.

2 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"compile_exit": 0, "exit": 0, "observations": 40}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### jim, bigip

Status: `not-tested`.

No native observation for this precise question and provider is attached; no result is inferred from another dialect, source channel or record.

## Consumer bindings

No implementation binding is claimed by this observation record.

- [runtime/rust/src/interp/native_body_artifact/native_try.rs](../../../../runtime/rust/src/interp/native_body_artifact/native_try.rs), `Builder::try_operation`: Retain the selected original native Try recipe, preparations and actual result/options/local bindings separately from generic or rejected compilation.
- [runtime/rust/src/interp/native_body_artifact/native_try.rs](../../../../runtime/rust/src/interp/native_body_artifact/native_try.rs), `Interp::execute_body_try`: Execute the admitted software protected completion, handler and finally plan with original retained result/options owners and typed Host refusal boundaries.
- [runtime/rust/src/interp/native_body_artifact/native_try.rs](../../../../runtime/rust/src/interp/native_body_artifact/native_try.rs), `interp::native_body_artifact::native_try::tests::compiled_try_matches_original_native_handlers_fallthrough_and_finally` (linked): The unchanged case19..39 source subset compares63 public Guest completions/results across C8.6/C9.0/C9.1 software interpreters with no Host refusal; cases19..24 and34..39 additionally require36 genuine cached procedure artifacts to contain an Operation::Try.

No assertion outcome is attached to these source bindings; software outcomes retain their independently pinned command and image scope.

## Exact original-source349 software outcomes

These finite results belong to the original source349 body/question/document pairs and the owned Runtime executable. Passing, failing, aborted/incomplete and unstarted selections retain their separate meanings. They establish no outcome for another source image or external provider.

| Original selection | Passed | Failed | Aborted or incomplete | Not started |
| --- | ---: | ---: | ---: | ---: |
| `grouped` | 1 | 0 | 0 | 0 |
| `isolated` | 1 | 0 | 0 | 0 |

Exact original body hashes and process outcomes join the unchanged original receipts, streams, frozen source, own inventory/admission and complete executable bytes. The grouped abort is independent of the isolated diagnostics.

## Evidence

- [runtime/rust/tests/data/native_compiled_try](../../../../runtime/rust/tests/data/native_compiled_try)
- [rust/tcl-registry/tests/data/native_each_try_compilation](../../../../rust/tcl-registry/tests/data/native_each_try_compilation)
