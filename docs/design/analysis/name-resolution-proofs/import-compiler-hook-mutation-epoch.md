# naming.import.compiler-hook-mutation-epoch

Kind: `native-observation`

## Problem statement

A compiler epoch must reflect the reached original compiler attachment and mutation, rather than the callable source alone. Over-invalidating an unhooked import or ignoring a hooked visibility mutation loses the measured release-specific deltas.

## Question

Which import/source hook and visibility mutations advance the private interpreter compileEpoch in this sequence?

## Conclusion

C8.5–9.1 source attach/import/detach/reattach each advance compileEpoch by1. Configuring unhooked source changes none; configuring hooked source advances by2 in C8.5 and by3 in C8.6–9.1. Unhooked old rename/hide/expose changes none; hooked fresh hide and expose each advance by1; source deletion advances by2. C8.4 ordinary procedure import/rename/hide/expose/delete keeps compileEpoch0. Absolute starting epochs differ and are not current runtime currency.

## Scope

Exact private C Command.compileProc and Interp.compileEpoch observer after completed public source/rename/hide/expose operations. The same source TU conditionally runs an ordinary procedure-only sequence in C8.4 and ensemble attachment mutations in C8.5–9.1; compile-out operations are not guest rejections. The observer selects currently visible command records and compares compiler function pointers only. Five original compile0/run0 captures with empty stderr and private-header/library/executable hashes are retained; runtime patchlevel and complete configure/compiler flags are unqueried. Jim and BIG-IP are not tested.

## Provider answers

### tcl8.4

Status: `observed`. Channel: Private C field observer following public Tcl_Eval, Tcl_HideCommand/Tcl_ExposeCommand and Tcl_SetEnsembleFlags mutations. Version: tcl8.4 8.4.20 (capture association; launched patchlevel unqueried).

Exact event snapshots:

event	code	compiler	source_hook	old_hook	fresh_hook	old_same	fresh_same
import_absent	0	0	0	0	-1	1	-1
rename_old	0	0	0	0	-1	1	-1
hide_old	0	0	0	-1	-1	-1	-1
expose_old	0	0	0	0	-1	1	-1
delete_source	0	0	-1	-1	-1	-1	-1. All reached guest codes are0; absent visible slots are−1. Pointer equality is compiler function equality only.

### tcl8.5

Status: `observed`. Channel: Private C field observer following public Tcl_Eval, Tcl_HideCommand/Tcl_ExposeCommand and Tcl_SetEnsembleFlags mutations. Version: tcl8.5 8.5.19 (capture association; launched patchlevel unqueried).

Exact event snapshots:

event	code	compiler	source_hook	old_hook	fresh_hook	old_same	fresh_same
import_absent	0	3	0	0	-1	1	-1
attach_source	0	4	1	0	-1	0	-1
import_present	0	5	1	0	1	0	1
detach_source	0	6	0	0	1	1	0
configure_unhooked	0	6	0	0	1	1	0
reattach_source	0	7	1	0	1	0	1
configure_hooked	0	9	1	0	1	0	1
rename_old	0	9	1	0	1	0	1
hide_old	0	9	1	-1	1	-1	1
expose_old	0	9	1	0	1	0	1
hide_fresh	0	10	1	0	-1	0	-1
expose_fresh	0	11	1	0	1	0	1
delete_source	0	13	-1	-1	-1	-1	-1. All reached guest codes are0; absent visible slots are−1. Pointer equality is compiler function equality only.

### tcl8.6

Status: `observed`. Channel: Private C field observer following public Tcl_Eval, Tcl_HideCommand/Tcl_ExposeCommand and Tcl_SetEnsembleFlags mutations. Version: tcl8.6 8.6.18 (capture association; launched patchlevel unqueried).

Exact event snapshots:

event	code	compiler	source_hook	old_hook	fresh_hook	old_same	fresh_same
import_absent	0	17	0	0	-1	1	-1
attach_source	0	18	1	0	-1	0	-1
import_present	0	19	1	0	1	0	1
detach_source	0	20	0	0	1	1	0
configure_unhooked	0	20	0	0	1	1	0
reattach_source	0	21	1	0	1	0	1
configure_hooked	0	24	1	0	1	0	1
rename_old	0	24	1	0	1	0	1
hide_old	0	24	1	-1	1	-1	1
expose_old	0	24	1	0	1	0	1
hide_fresh	0	25	1	0	-1	0	-1
expose_fresh	0	26	1	0	1	0	1
delete_source	0	28	-1	-1	-1	-1	-1. All reached guest codes are0; absent visible slots are−1. Pointer equality is compiler function equality only.

### tcl9.0

Status: `observed`. Channel: Private C field observer following public Tcl_Eval, Tcl_HideCommand/Tcl_ExposeCommand and Tcl_SetEnsembleFlags mutations. Version: tcl9.0 9.0.4 (capture association; launched patchlevel unqueried).

Exact event snapshots:

event	code	compiler	source_hook	old_hook	fresh_hook	old_same	fresh_same
import_absent	0	20	0	0	-1	1	-1
attach_source	0	21	1	0	-1	0	-1
import_present	0	22	1	0	1	0	1
detach_source	0	23	0	0	1	1	0
configure_unhooked	0	23	0	0	1	1	0
reattach_source	0	24	1	0	1	0	1
configure_hooked	0	27	1	0	1	0	1
rename_old	0	27	1	0	1	0	1
hide_old	0	27	1	-1	1	-1	1
expose_old	0	27	1	0	1	0	1
hide_fresh	0	28	1	0	-1	0	-1
expose_fresh	0	29	1	0	1	0	1
delete_source	0	31	-1	-1	-1	-1	-1. All reached guest codes are0; absent visible slots are−1. Pointer equality is compiler function equality only.

### tcl9.1

Status: `observed`. Channel: Private C field observer following public Tcl_Eval, Tcl_HideCommand/Tcl_ExposeCommand and Tcl_SetEnsembleFlags mutations. Version: tcl9.1 9.1.0 (capture association; launched patchlevel unqueried).

Exact event snapshots:

event	code	compiler	source_hook	old_hook	fresh_hook	old_same	fresh_same
import_absent	0	24	0	0	-1	1	-1
attach_source	0	25	1	0	-1	0	-1
import_present	0	26	1	0	1	0	1
detach_source	0	27	0	0	1	1	0
configure_unhooked	0	27	0	0	1	1	0
reattach_source	0	28	1	0	1	0	1
configure_hooked	0	31	1	0	1	0	1
rename_old	0	31	1	0	1	0	1
hide_old	0	31	1	-1	1	-1	1
expose_old	0	31	1	0	1	0	1
hide_fresh	0	32	1	0	-1	0	-1
expose_fresh	0	33	1	0	1	0	1
delete_source	0	35	-1	-1	-1	-1	-1. All reached guest codes are0; absent visible slots are−1. Pointer equality is compiler function equality only.

### jim, bigip

Status: `not-tested`.

No observation of this exact question is attached for this provider.

## Consumer bindings

- [rust/tcl-vm/src/interp.rs](../../../../rust/tcl-vm/src/interp.rs), `native_compiler_binding_at_lookup`: Reissues the imported compiler selection and callable implementation independently from source attachment.
- [rust/tcl-vm/src/interp/native_cache_tests.rs](../../../../rust/tcl-vm/src/interp/native_cache_tests.rs), `interp::native_cache_tests::imported_compiler_attachments_match_all_57_original_native_snapshots` (linked): Checks all 57 reached snapshots, copied attachments and relative compiler epoch deltas; Rust native token/implementation assertions remain additional implementation obligations.

## Evidence

- [rust/tcl-vm/tests/data/native_import_hook_copy/probe.c](../../../../rust/tcl-vm/tests/data/native_import_hook_copy/probe.c)
- [rust/tcl-vm/tests/data/native_import_hook_copy/manifest.json](../../../../rust/tcl-vm/tests/data/native_import_hook_copy/manifest.json)
- [rust/tcl-vm/tests/data/native_import_hook_copy/8.4.20.tsv](../../../../rust/tcl-vm/tests/data/native_import_hook_copy/8.4.20.tsv)
- [rust/tcl-vm/tests/data/native_import_hook_copy/8.5.19.tsv](../../../../rust/tcl-vm/tests/data/native_import_hook_copy/8.5.19.tsv)
- [rust/tcl-vm/tests/data/native_import_hook_copy/8.6.18.tsv](../../../../rust/tcl-vm/tests/data/native_import_hook_copy/8.6.18.tsv)
- [rust/tcl-vm/tests/data/native_import_hook_copy/9.0.4.tsv](../../../../rust/tcl-vm/tests/data/native_import_hook_copy/9.0.4.tsv)
- [rust/tcl-vm/tests/data/native_import_hook_copy/9.1.0.tsv](../../../../rust/tcl-vm/tests/data/native_import_hook_copy/9.1.0.tsv)
