# Do repeated callback arguments and a nested callback retain the same Tcl_Obj, and what original primary/resident storage does each reached getter publish?

Proof ID: `naming.native-abi.original-object-callback-getters`

## Problem statement

Converting callback argv to new Rust values can lose repeated argument identity and hide representation mutations made by successful or failing C getters. Native binary backing also differs from its lazily produced Tcl string. This question concerns original objects passed through public C callbacks, not equivalent argument text.

## Question

Do repeated callback arguments and a nested callback retain the same Tcl_Obj, and what original primary/resident storage does each reached getter publish?

## Scope

Six fixed public C callback controls per C8.4.20–9.1.0 release; scalar getter and ByteArray string updates on original Tcl_Obj headers. No Jim or BIG-IP physical ABI observation.

## Measured answers

| Provider | Status | Answer |
|---|---|---|
| tcl8.4 8.4.20 | observed | Six controls, native exit0; repeated pointer alias1, nested alias1 on nested-wide, original getter changes retained. |
| tcl8.5 8.5.19 | observed | Six controls, native exit0; repeated pointer alias1, nested alias1 on nested-wide, original getter changes retained. |
| tcl8.6 8.6.18 | observed | Six controls, native exit0; repeated pointer alias1, nested alias1 on nested-wide, original getter changes retained. |
| tcl9.0 9.0.4 | observed | Six controls, native exit0; repeated pointer alias1, nested alias1 on nested-wide, original getter changes retained. |
| tcl9.1 9.1.0 | observed | Six controls, native exit0; repeated pointer alias1, nested alias1 on nested-wide, original getter changes retained. |
| jim not recorded | not-tested | No observation for this precise question. |
| bigip not recorded | not-tested | No observation for this precise question. |

## Conclusion

All five C captures preserve repeated and nested original pointer identity. Reached integer and string getters publish the retained primary/resident changes on that same object; failed integer conversion may still install a Double or Bignum primary depending on the release. Binary backing00FF materializes as C080C3BF while retaining ByteArray primary. Equal text alone establishes none of these object facts.

## Source anchors

No interpreter-source anchor is asserted for this question; the retained probe and outcomes establish the narrow observation.

## Shared owners and tests

- `rust/tcl-cshim/src/obj.rs`: `Obj` — Original object/shim mirror lifetime and getter changes.
- `rust/tcl-cshim/src/state.rs`: `InterpState` — Callback original-object graph.
- `rust/tcl-cshim/tests/factory.rs`: `callback_getters_preserve_aliases_and_publish_original_failure_caches`. Verify original callback aliases and failure cache publication under the shim selected ABI.

Native output is evidence for the interpreter operation. Rust tests must independently pass to establish implementation correspondence.

## Reconfirmation

```text
cc -I<version-source>/generic rust/tcl-cshim/tests/data/native_original_objects/probe.c <version-source>/unix/libtcl<major.minor>.a -lm -ldl -lpthread -lz -o /tmp/native-original-object-probe
```

Build for each exact C release, run resulting executable and compare every JSON row with the release-specific capture. Guest conversion codes0/1 are expected data. This native program does not prove Rust shim equivalence; the named Rust test checks that separate contract.
