# naming.info.loaded-original-interpreter-path

Kind: `native-observation`

## Problem statement

Decoding an interpreter path into a display name can lose its original list element boundaries or choose a different child; a setup failure also cannot answer the later loaded query.

## Question

How do the exact source controls resolve root and nested original interpreter list paths, materialised binary child names and malformed/missing paths for info loaded?

## Conclusion

All five C providers return an empty loaded list for the root, the existing two-element nested child path, and a child created and queried with the same binary-produced runtime value. A malformed list produces unmatched open brace in list, while the missing child produces could not find interpreter "MissingInfo085". Jim explicitly rejects direct info loaded queries. Its nested and binary-child controls fail at interp create before reaching info loaded and provide no loaded-path result. The runtime binary values undergo string conversion; successful lookup supplies no raw-zero argv, physical child-key/cache law or interpreter lifetime grant.

## Scope

Five loaded-path controls in the exact shared twelve-row ASCII LF source-file program, independently captured under C8.4.20/8.5.19/8.6.18/9.0.4/9.1.0 and Jim0.84-9-g5bac7c9. Empty loaded lists do not establish general library enumeration, raw counted storage, cache, native compiler or Normal capability. No Rust result is attached.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: ASCII LF source-file shell ingress. Runtime binary-produced raw/encoded bytes are separate; output decimal string-index scan-%c units, not original physical bytes. No direct API/object cache/CPP/Normal grant. Version: tcl8.4 8.4.20; tcl8.5 8.5.19; tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0.

All five C providers return an empty loaded list for the root, the existing two-element nested child path, and a child created and queried with the same binary-produced runtime value. A malformed list produces unmatched open brace in list, while the missing child produces could not find interpreter "MissingInfo085". Jim explicitly rejects direct info loaded queries. Its nested and binary-child controls fail at interp create before reaching info loaded and provide no loaded-path result. The runtime binary values undergo string conversion; successful lookup supplies no raw-zero argv, physical child-key/cache law or interpreter lifetime grant.

### jim

Status: `unsupported`. Channel: ASCII LF source-file shell ingress. Runtime binary-produced raw/encoded bytes are separate; output decimal string-index scan-%c units, not original physical bytes. No direct API/object cache/CPP/Normal grant. Version: jim 0.84-9-g5bac7c9.

Direct loaded-root/malformed-list/missing-child controls reach info loaded and reject its unknown subcommand. Nested and binary-child controls fail at interp create with wrong # args before loaded; those two later queries are unattempted.

### bigip

Status: `not-tested`.

Observed guest rejection of info functions in all seven function controls; this is not a command-cache, fixed-table or function execution answer.

## Consumer bindings

- [rust/tcl-vm/src/cmd_info.rs](../../../../rust/tcl-vm/src/cmd_info.rs), `cmd_info`: Retain original interpreter list path elements for loaded queries independently of reporting names and source labels.
- [runtime/rust/src/cmd_info.rs](../../../../runtime/rust/src/cmd_info.rs), `stock_loaded`: Pass the original path object to the independently selected live child-path owner before enumerating loaded entries.
- [rust/tcl-vm/src/cmd_info.rs](../../../../rust/tcl-vm/src/cmd_info.rs), `cmd_info::tests::original_info_loaded_uses_counted_interpreter_path_elements` (linked): Compare the matched finite C source/result purposes through the actual selected VM/Runtime provider; this binding claims no Rust execution or physical byte/header/cache parity.
- [runtime/rust/src/cmd_info.rs](../../../../runtime/rust/src/cmd_info.rs), `cmd_info::tests::original_info_loaded_uses_counted_interpreter_path_elements` (linked): Compare the matched finite C source/result purposes through the actual selected VM/Runtime provider; this binding claims no Rust execution or physical byte/header/cache parity.
- [rust/tcl-vm/src/cmd_info.rs](../../../../rust/tcl-vm/src/cmd_info.rs), `cmd_info::tests::original_info_loaded_empty_result_uses_actual_c_constructor_without_list_cache` (linked): Five genuine software core fixtures select the actual empty info-loaded result constructor and assert resident empty String with no List cache. The fixture selections are not external C header measurements or new executed results.
- [rust/tcl-vm/src/cmd_info.rs](../../../../rust/tcl-vm/src/cmd_info.rs), `cmd_info::tests::original_info_loaded_inventory_follows_selected_child_and_refuses_retired_id` (linked): Actual root and resolved child inventory remain distinct; deleting the selected child withdraws its inventory without changing root libraries, and the original public child path yields a Guest error. This software owner/path definition grants no fallback inventory or passing outcome.
- [rust/tcl-vm/src/cmd_info.rs](../../../../rust/tcl-vm/src/cmd_info.rs), `cmd_info::tests::original_info_loaded_keeps_physical_result_owner_separate_from_source_labels` (linked): An actual C9 software core retains its empty result owner independently of C84 source policy. An authored actual-profile label without a genuine core has no physical String protocol and refuses; this definition adds no Native header measurement or executed outcome.

## Evidence

- [rust/tcl-cmd-core/tests/data/native_info_functions_loaded](../../../../rust/tcl-cmd-core/tests/data/native_info_functions_loaded)
- [rust/tcl-cmd-core/tests/data/native_info_functions_loaded/8.4.20](../../../../rust/tcl-cmd-core/tests/data/native_info_functions_loaded/8.4.20)
- [rust/tcl-cmd-core/tests/data/native_info_functions_loaded/8.5.19](../../../../rust/tcl-cmd-core/tests/data/native_info_functions_loaded/8.5.19)
- [rust/tcl-cmd-core/tests/data/native_info_functions_loaded/8.6.18](../../../../rust/tcl-cmd-core/tests/data/native_info_functions_loaded/8.6.18)
- [rust/tcl-cmd-core/tests/data/native_info_functions_loaded/9.0.4](../../../../rust/tcl-cmd-core/tests/data/native_info_functions_loaded/9.0.4)
- [rust/tcl-cmd-core/tests/data/native_info_functions_loaded/9.1.0](../../../../rust/tcl-cmd-core/tests/data/native_info_functions_loaded/9.1.0)
- [rust/tcl-cmd-core/tests/data/native_info_functions_loaded/jim](../../../../rust/tcl-cmd-core/tests/data/native_info_functions_loaded/jim)
