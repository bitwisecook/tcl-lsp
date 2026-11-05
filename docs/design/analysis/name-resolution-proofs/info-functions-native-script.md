# naming.info.functions-native-script

Kind: `native-observation`

## Problem statement

A nominal math-function table can omit caller-local functions or bypass the actual script helper selected by later Tcl info functions.

## Question

Which fixed-table, caller-local/global merge and selected helper outcomes does the exact original native info functions source program produce?

## Conclusion

All five C providers return sin for the literal pattern. Tcl 8.4 returns empty for the new caller-local, merged suffix and binary-produced opaque function controls and still returns sin after the apply helper replacement. Tcl 8.5–9.1 return caller-local local085, the sorted distinct local085/other085/shared085 names, and the opaque function with final numeric unit 255; their selected apply replacement produces the exact APPLY_HELPER085 error. Both binary-produced zero-suffix patterns return empty in all C controls. Jim explicitly rejects info functions. These are finite source/script result observations; they establish no raw resident byte representation, function lookup allocation, cache or Normal authority.

## Scope

Seven named function controls in the shared twelve-row ASCII LF source-file program, independently captured under C8.4.20/8.5.19/8.6.18/9.0.4/9.1.0 and Jim0.84-9-g5bac7c9. The result channel is decimal string units, including materialised binary-produced values. No direct object-vector API, physical header/cache, native compiler, transformed-program or Rust execution result is attached. Four independent pinned Tcl 8.5/8.6/9.0/9.1 InfoFunctionsCmd source excerpts retain only the script-construction prefix: global and caller-local mathfunc enumeration, namespace tail selection, duplicate-name exclusion and the selected current namespace. Argument append and execution are outside these windows; the excerpts supply no build association.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: ASCII LF source-file shell ingress. Runtime binary-produced raw/encoded bytes are separate; output decimal string-index scan-%c units, not original physical bytes. No direct API/object cache/CPP/Normal grant. Version: tcl8.4 8.4.20; tcl8.5 8.5.19; tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0.

All five C providers return sin for the literal pattern. Tcl 8.4 returns empty for the new caller-local, merged suffix and binary-produced opaque function controls and still returns sin after the apply helper replacement. Tcl 8.5–9.1 return caller-local local085, the sorted distinct local085/other085/shared085 names, and the opaque function with final numeric unit 255; their selected apply replacement produces the exact APPLY_HELPER085 error. Both binary-produced zero-suffix patterns return empty in all C controls. Jim explicitly rejects info functions. These are finite source/script result observations; they establish no raw resident byte representation, function lookup allocation, cache or Normal authority.

### jim

Status: `unsupported`. Channel: ASCII LF source-file shell ingress. Runtime binary-produced raw/encoded bytes are separate; output decimal string-index scan-%c units, not original physical bytes. No direct API/object cache/CPP/Normal grant. Version: jim 0.84-9-g5bac7c9.

Observed guest rejection of info functions in all seven function controls; this is not a command-cache, fixed-table or function execution answer.

### bigip

Status: `not-tested`.

This is a Rust source-consumer implementation contract. No native provider observation or executed Rust result is attached; selected source advice supplies no physical native or runtime authority.

## Source inspection

tcl8.5 8.5.19, revision `8.5.19`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclCmdIL.c`, function `InfoFunctionsCmd`, lines 1349–1362. Retained reference `functions-script-prefix-0`.

```text
    script = Tcl_NewStringObj(
"	    ::apply [::list {{pattern *}} {\n"
"		::set cmds {}\n"
"		::foreach cmd [::info commands ::tcl::mathfunc::$pattern] {\n"
"		    ::lappend cmds [::namespace tail $cmd]\n"
"		}\n"
"		::foreach cmd [::info commands tcl::mathfunc::$pattern] {\n"
"		    ::set cmd [::namespace tail $cmd]\n"
"		    ::if {$cmd ni $cmds} {\n"
"			::lappend cmds $cmd\n"
"		    }\n"
"		}\n"
"		::return $cmds\n"
"	    } [::namespace current]] ", -1);
```

tcl8.6 8.6.18, revision `8.6.18`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclCmdIL.c`, function `InfoFunctionsCmd`, lines 1495–1508. Retained reference `functions-script-prefix-1`.

```text
    script = Tcl_NewStringObj(
"	    ::apply [::list {{pattern *}} {\n"
"		::set cmds {}\n"
"		::foreach cmd [::info commands ::tcl::mathfunc::$pattern] {\n"
"		    ::lappend cmds [::namespace tail $cmd]\n"
"		}\n"
"		::foreach cmd [::info commands tcl::mathfunc::$pattern] {\n"
"		    ::set cmd [::namespace tail $cmd]\n"
"		    ::if {$cmd ni $cmds} {\n"
"			::lappend cmds $cmd\n"
"		    }\n"
"		}\n"
"		::return $cmds\n"
"	    } [::namespace current]] ", -1);
```

tcl9.0 9.0.4, revision `9.0.4`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclCmdIL.c`, function `InfoFunctionsCmd`, lines 1484–1497. Retained reference `functions-script-prefix-2`.

```text
    script = Tcl_NewStringObj(
"	    ::apply [::list {{pattern *}} {\n"
"		::set cmds {}\n"
"		::foreach cmd [::info commands ::tcl::mathfunc::$pattern] {\n"
"		    ::lappend cmds [::namespace tail $cmd]\n"
"		}\n"
"		::foreach cmd [::info commands tcl::mathfunc::$pattern] {\n"
"		    ::set cmd [::namespace tail $cmd]\n"
"		    ::if {$cmd ni $cmds} {\n"
"			::lappend cmds $cmd\n"
"		    }\n"
"		}\n"
"		::return $cmds\n"
"	    } [::namespace current]] ", -1);
```

tcl9.1 9.1.0, revision `9.1.0`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclCmdIL.c`, function `InfoFunctionsCmd`, lines 1468–1481. Retained reference `functions-script-prefix-3`.

```text
    script = Tcl_NewStringObj(
"	    ::apply [::list {{pattern *}} {\n"
"		::set cmds {}\n"
"		::foreach cmd [::info commands ::tcl::mathfunc::$pattern] {\n"
"		    ::lappend cmds [::namespace tail $cmd]\n"
"		}\n"
"		::foreach cmd [::info commands tcl::mathfunc::$pattern] {\n"
"		    ::set cmd [::namespace tail $cmd]\n"
"		    ::if {$cmd ni $cmds} {\n"
"			::lappend cmds $cmd\n"
"		    }\n"
"		}\n"
"		::return $cmds\n"
"	    } [::namespace current]] ", -1);
```

## Consumer bindings

- [rust/tcl-vm/src/cmd_info.rs](../../../../rust/tcl-vm/src/cmd_info.rs), `info_functions_impl`: Use the selected native fixed table or original script and actual helpers rather than a replacement nominal table.
- [runtime/rust/src/cmd_info.rs](../../../../runtime/rust/src/cmd_info.rs), `info_functions`: Preserve the selected function script/table and actual native helper lookup under the current interpreter.
- [rust/tcl-vm/src/cmd_info.rs](../../../../rust/tcl-vm/src/cmd_info.rs), `cmd_info::tests::original_info_functions_follow_selected_native_script_and_helpers` (linked): Compare the matched finite C source/result purposes through the actual selected VM/Runtime provider; this binding claims no Rust execution or physical byte/header/cache parity.
- [runtime/rust/src/cmd_info.rs](../../../../runtime/rust/src/cmd_info.rs), `cmd_info::tests::original_info_functions_follow_selected_native_script_and_helpers` (linked): Compare the matched finite C source/result purposes through the actual selected VM/Runtime provider; this binding claims no Rust execution or physical byte/header/cache parity.

## Exact original-source349 software outcomes

These finite results belong to the original source349 body/question/document pairs and the owned Runtime executable. Passing, failing, aborted/incomplete and unstarted selections retain their separate meanings. They establish no outcome for another source image or external provider.

| Original selection | Passed | Failed | Aborted or incomplete | Not started |
| --- | ---: | ---: | ---: | ---: |
| `grouped` | 1 | 0 | 0 | 0 |
| `isolated` | 1 | 0 | 0 | 0 |

Exact original body hashes and process outcomes join the unchanged original receipts, streams, frozen source, own inventory/admission and complete executable bytes. The grouped abort is independent of the isolated diagnostics.

## Evidence

- [rust/tcl-cmd-core/tests/data/native_info_functions_loaded](../../../../rust/tcl-cmd-core/tests/data/native_info_functions_loaded)
- [rust/tcl-cmd-core/tests/data/native_info_functions_loaded/8.4.20](../../../../rust/tcl-cmd-core/tests/data/native_info_functions_loaded/8.4.20)
- [rust/tcl-cmd-core/tests/data/native_info_functions_loaded/8.5.19](../../../../rust/tcl-cmd-core/tests/data/native_info_functions_loaded/8.5.19)
- [rust/tcl-cmd-core/tests/data/native_info_functions_loaded/8.6.18](../../../../rust/tcl-cmd-core/tests/data/native_info_functions_loaded/8.6.18)
- [rust/tcl-cmd-core/tests/data/native_info_functions_loaded/9.0.4](../../../../rust/tcl-cmd-core/tests/data/native_info_functions_loaded/9.0.4)
- [rust/tcl-cmd-core/tests/data/native_info_functions_loaded/9.1.0](../../../../rust/tcl-cmd-core/tests/data/native_info_functions_loaded/9.1.0)
- [rust/tcl-cmd-core/tests/data/native_info_functions_loaded/jim](../../../../rust/tcl-cmd-core/tests/data/native_info_functions_loaded/jim)
