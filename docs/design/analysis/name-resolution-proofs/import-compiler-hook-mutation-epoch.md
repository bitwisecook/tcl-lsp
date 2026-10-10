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

Status: `observed`. Version: 8.4.20 (capture association; launched patchlevel unqueried). Build: binary_sha256=996d24a53c8585d75f2c0b9d532d76d1d0376d8fd8737c9610acd25d32d0749a; library_sha256=d71ed42efd90354cdc99f73ba39b30e2cdedda463768a7fd025114b50db60011; header_sha256=f51c8e66c562de424b84a35a04b6b86d9a135ef7d4ca891b35c3b0586bc0ce1a; compile_exit=0; exit=0. Compiler version, full configure flags and launched runtime patchlevel were not queried by this probe.. Channel: Private C field observer following public Tcl_Eval, Tcl_HideCommand/Tcl_ExposeCommand and Tcl_SetEnsembleFlags mutations.. Dialect: C Tcl.

Exact event snapshots:

event	code	compiler	source_hook	old_hook	fresh_hook	old_same	fresh_same
import_absent	0	0	0	0	-1	1	-1
rename_old	0	0	0	0	-1	1	-1
hide_old	0	0	0	-1	-1	-1	-1
expose_old	0	0	0	0	-1	1	-1
delete_source	0	0	-1	-1	-1	-1	-1. All reached guest codes are0; absent visible slots are−1. Pointer equality is compiler function equality only.

### tcl8.5

Status: `observed`. Version: 8.5.19 (capture association; launched patchlevel unqueried). Build: binary_sha256=31b39cad2e990f95e96678f95c71e76e09b8a702e175ace4784546364873297f; library_sha256=99e0d524e54498713498e43d6cf80fa085471913cb38a53b505e18c0469417df; header_sha256=72078ca207924e61b4fbf637eff263d280a0dbc78e6a5fbb39e2cd980d08a8aa; compile_exit=0; exit=0. Compiler version, full configure flags and launched runtime patchlevel were not queried by this probe.. Channel: Private C field observer following public Tcl_Eval, Tcl_HideCommand/Tcl_ExposeCommand and Tcl_SetEnsembleFlags mutations.. Dialect: C Tcl.

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

Status: `observed`. Version: 8.6.18 (capture association; launched patchlevel unqueried). Build: binary_sha256=e95c73f4907354ac36a87cde10f8b0a4c199ab5bda497fd6929803c72a630d97; library_sha256=a3a8abdeadd8aafa007d3bc9fa199b9cf48d43d01735ff41b3fcf33abb4019d6; header_sha256=e9e70f6461d7c4b37173173956040b647914157c28425ec859730cd8eaef4d2e; compile_exit=0; exit=0. Compiler version, full configure flags and launched runtime patchlevel were not queried by this probe.. Channel: Private C field observer following public Tcl_Eval, Tcl_HideCommand/Tcl_ExposeCommand and Tcl_SetEnsembleFlags mutations.. Dialect: C Tcl.

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

Status: `observed`. Version: 9.0.4 (capture association; launched patchlevel unqueried). Build: binary_sha256=a8713802ec9c6210dc42bd85f19dc66fc300e5edc15dd1067fcae922cb203e40; library_sha256=2ba08ecf7197e16d99e303c0d95dde29c59f391a22fd02378edc2697567e0e04; header_sha256=f000e14fdc4e00da1c5ab98a8087c41be3cbf26564a274152236e0531bb55053; compile_exit=0; exit=0. Compiler version, full configure flags and launched runtime patchlevel were not queried by this probe.. Channel: Private C field observer following public Tcl_Eval, Tcl_HideCommand/Tcl_ExposeCommand and Tcl_SetEnsembleFlags mutations.. Dialect: C Tcl.

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

Status: `observed`. Version: 9.1.0 (capture association; launched patchlevel unqueried). Build: binary_sha256=7c9fd0ed1953f8abe1c69c1a772e3834fa7cfa9e33c1b1927f11b8a58ed4b083; library_sha256=8b2dba836908287f95f26a442bb065601daac300fdeee48b15e64a5c03894e33; header_sha256=fea9d99020df9aec6efebf6e3f18592ee267a9bf80d3843df0d6439efebe65cc; compile_exit=0; exit=0. Compiler version, full configure flags and launched runtime patchlevel were not queried by this probe.. Channel: Private C field observer following public Tcl_Eval, Tcl_HideCommand/Tcl_ExposeCommand and Tcl_SetEnsembleFlags mutations.. Dialect: C Tcl.

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

### jim

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: jim.

No observation of this exact question is attached for this provider.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: bigip.

No observation of this exact question is attached for this provider.

## Exact evidence

- `probe` (input): [rust/tcl-vm/tests/data/native_import_hook_copy/probe.c](../../../../rust/tcl-vm/tests/data/native_import_hook_copy/probe.c). SHA-256 `44ba80880460fc01c2a9c0bb2df4718275b9ef2872296973333c5d5eeb72b603`. Exact private fields, conditional release-specific program and compiler pointer comparison.
- `receipt` (provider): [rust/tcl-vm/tests/data/native_import_hook_copy/manifest.json](../../../../rust/tcl-vm/tests/data/native_import_hook_copy/manifest.json). SHA-256 `e7cebed2c614dd2dfd4871f3562918a605551b5ad262ee8237eee73b981615fb`. Original full per-release observations, empty stderr, compile/run status and actual private-header/library/executable hashes.
- `rows-tcl8.4` (observation): [rust/tcl-vm/tests/data/native_import_hook_copy/8.4.20.tsv](../../../../rust/tcl-vm/tests/data/native_import_hook_copy/8.4.20.tsv). SHA-256 `565269918c18df7e1f91f4b1d196bc45207e69be2450feb7bb5420aa3700fca5`. Full 5-event sequence; columns event,code,compileEpoch,source-hook,old-hook,fresh-hook,old-pointer-equal-source,fresh-pointer-equal-source.
- `rows-tcl8.5` (observation): [rust/tcl-vm/tests/data/native_import_hook_copy/8.5.19.tsv](../../../../rust/tcl-vm/tests/data/native_import_hook_copy/8.5.19.tsv). SHA-256 `5cf8cd085069606ccd191888076f5c71631057858c185e68098bca9a30f068a9`. Full 13-event sequence; columns event,code,compileEpoch,source-hook,old-hook,fresh-hook,old-pointer-equal-source,fresh-pointer-equal-source.
- `rows-tcl8.6` (observation): [rust/tcl-vm/tests/data/native_import_hook_copy/8.6.18.tsv](../../../../rust/tcl-vm/tests/data/native_import_hook_copy/8.6.18.tsv). SHA-256 `5396fc9c593cd950945b2f77c590ca84b4eb8eae7044ca0d94c0954b1cbd6f34`. Full 13-event sequence; columns event,code,compileEpoch,source-hook,old-hook,fresh-hook,old-pointer-equal-source,fresh-pointer-equal-source.
- `rows-tcl9.0` (observation): [rust/tcl-vm/tests/data/native_import_hook_copy/9.0.4.tsv](../../../../rust/tcl-vm/tests/data/native_import_hook_copy/9.0.4.tsv). SHA-256 `a084d8dfd8d54bf77ec89a3ba9e106d19b629ce0a3ebd831f04b2ca146aef217`. Full 13-event sequence; columns event,code,compileEpoch,source-hook,old-hook,fresh-hook,old-pointer-equal-source,fresh-pointer-equal-source.
- `rows-tcl9.1` (observation): [rust/tcl-vm/tests/data/native_import_hook_copy/9.1.0.tsv](../../../../rust/tcl-vm/tests/data/native_import_hook_copy/9.1.0.tsv). SHA-256 `d7bdc8a76235e5fcf1c52c278e1cc36ea6d13366af9912c9e34f0eaaa4fcb6eb`. Full 13-event sequence; columns event,code,compileEpoch,source-hook,old-hook,fresh-hook,old-pointer-equal-source,fresh-pointer-equal-source.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-vm/src/interp.rs](../../../../rust/tcl-vm/src/interp.rs), `native_compiler_binding_at_lookup`: Reissues the imported compiler selection and callable implementation independently from source attachment.
- [rust/tcl-vm/src/interp/native_cache_tests.rs](../../../../rust/tcl-vm/src/interp/native_cache_tests.rs), `interp::native_cache_tests::imported_compiler_attachments_match_all_57_original_native_snapshots` (linked): Checks all 57 reached snapshots, copied attachments and relative compiler epoch deltas; Rust native token/implementation assertions remain additional implementation obligations.

A named test is a coverage binding, not a claim that it executed.

## Replay

No fresh native launch or executed Rust result is claimed. A new native run must compile the retained exact probe against independently identified release headers and library, preserve the counted input channel, and record separate status/stdout/stderr before comparing the retained stream. Original absolute compiler paths and binary digests are capture metadata, not a currently runnable command. No native implementation explanation is inferred from the probe. Retained run.py is an original capture writer with hardcoded external paths and a non-existing-manifest assertion, not a portable reconfirmation runner; it must not overwrite this evidence.
