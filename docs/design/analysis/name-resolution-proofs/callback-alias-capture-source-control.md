# naming.callback.alias-capture-source-control

Kind: `native-observation`

## Problem statement

A readonly callback source producer can retain prefix words, but alias target-name relookup, nested capture order, moves and later replacement require independent public native controls. Error labels also differ by release and cannot identify a source declaration.

## Question

What exact public comparator arguments, alias moves, target-name relookup and replacement-arity results do the retained lsort callback alias controls produce?

## Conclusion

All six providers expose FIXED 2 1, FIXED BAKED 2 1 and INNER OUTER BAKED 2 1 in their public comparator procedure args. Moving the alias preserves FIXED. Moving its target away leaves invalid command name target and no comparator observation; creating a four-formal replacement at target fails without BAKED and succeeds with BAKED. C8.4 and Jim report the failing signature as target a b c d; C8.5+ report moved_cb b c d. Jim uses its authored alias command while C Tcl uses interp alias. Every process exits zero with empty stderr and eight retained stdout rows. These finite public args and target-name lookup controls do not supply internal native argv/object/header/cache, installation currentness, a runtime frame or source edit authority.

## Scope

Original fixed ASCII LF CLI file on five pinned C releases and current Jim0.84-9-g5bac7c9. Twenty original input/receipt/stream files retain exact release-specific outcomes and independently checked executable/library/header/Makefile/source pins. The two-item lsort vectors retain the exact recorded comparison order. BIG-IP is not tested.

## Provider answers

### tcl8.4

Status: `observed`. Channel: Original fixed ASCII LF CLI file; lsort comparator public args and caught result/error; no internal object/header/native argv/cache/frame or installation-currentness claim. Version: tcl8.4 8.4.20.

Exact stdout: version 8.4.20 alias_command {} | single 0 {2 1} {{FIXED 2 1}} | baked 0 {2 1} {{FIXED BAKED 2 1}} | nested 0 {2 1} {{INNER OUTER BAKED 2 1}} | moved_alias 0 {2 1} {{FIXED 2 1}} | moved_target 1 {invalid command name "target"} {} | replacement_four 1 {wrong # args: should be "target a b c d"} | replacement_baked_four 0 {2 1} . Captured words precede the selected two comparator inputs; nested INNER precedes OUTER and BAKED. Alias movement retains capture, target movement fails lookup, and four-formal replacement accepts only the added BAKED case.

### tcl8.5

Status: `observed`. Channel: Original fixed ASCII LF CLI file; lsort comparator public args and caught result/error; no internal object/header/native argv/cache/frame or installation-currentness claim. Version: tcl8.5 8.5.19.

Exact stdout: version 8.5.19 alias_command {} | single 0 {2 1} {{FIXED 2 1}} | baked 0 {2 1} {{FIXED BAKED 2 1}} | nested 0 {2 1} {{INNER OUTER BAKED 2 1}} | moved_alias 0 {2 1} {{FIXED 2 1}} | moved_target 1 {invalid command name "target"} {} | replacement_four 1 {wrong # args: should be "moved_cb b c d"} | replacement_baked_four 0 {2 1} . Captured words precede the selected two comparator inputs; nested INNER precedes OUTER and BAKED. Alias movement retains capture, target movement fails lookup, and four-formal replacement accepts only the added BAKED case.

### tcl8.6

Status: `observed`. Channel: Original fixed ASCII LF CLI file; lsort comparator public args and caught result/error; no internal object/header/native argv/cache/frame or installation-currentness claim. Version: tcl8.6 8.6.18.

Exact stdout: version 8.6.18 alias_command {} | single 0 {2 1} {{FIXED 2 1}} | baked 0 {2 1} {{FIXED BAKED 2 1}} | nested 0 {2 1} {{INNER OUTER BAKED 2 1}} | moved_alias 0 {2 1} {{FIXED 2 1}} | moved_target 1 {invalid command name "target"} {} | replacement_four 1 {wrong # args: should be "moved_cb b c d"} | replacement_baked_four 0 {2 1} . Captured words precede the selected two comparator inputs; nested INNER precedes OUTER and BAKED. Alias movement retains capture, target movement fails lookup, and four-formal replacement accepts only the added BAKED case.

### tcl9.0

Status: `observed`. Channel: Original fixed ASCII LF CLI file; lsort comparator public args and caught result/error; no internal object/header/native argv/cache/frame or installation-currentness claim. Version: tcl9.0 9.0.4.

Exact stdout: version 9.0.4 alias_command {} | single 0 {2 1} {{FIXED 2 1}} | baked 0 {2 1} {{FIXED BAKED 2 1}} | nested 0 {2 1} {{INNER OUTER BAKED 2 1}} | moved_alias 0 {2 1} {{FIXED 2 1}} | moved_target 1 {invalid command name "target"} {} | replacement_four 1 {wrong # args: should be "moved_cb b c d"} | replacement_baked_four 0 {2 1} . Captured words precede the selected two comparator inputs; nested INNER precedes OUTER and BAKED. Alias movement retains capture, target movement fails lookup, and four-formal replacement accepts only the added BAKED case.

### tcl9.1

Status: `observed`. Channel: Original fixed ASCII LF CLI file; lsort comparator public args and caught result/error; no internal object/header/native argv/cache/frame or installation-currentness claim. Version: tcl9.1 9.1.0.

Exact stdout: version 9.1.0 alias_command {} | single 0 {2 1} {{FIXED 2 1}} | baked 0 {2 1} {{FIXED BAKED 2 1}} | nested 0 {2 1} {{INNER OUTER BAKED 2 1}} | moved_alias 0 {2 1} {{FIXED 2 1}} | moved_target 1 {invalid command name "target"} {} | replacement_four 1 {wrong # args: should be "moved_cb b c d"} | replacement_baked_four 0 {2 1} . Captured words precede the selected two comparator inputs; nested INNER precedes OUTER and BAKED. Alias movement retains capture, target movement fails lookup, and four-formal replacement accepts only the added BAKED case.

### jim

Status: `observed`. Channel: Original fixed ASCII LF CLI file; lsort comparator public args and caught result/error; no internal object/header/native argv/cache/frame or installation-currentness claim. Version: jim 0.84-9-g5bac7c9.

Exact stdout: version 0.84-9-g5bac7c9 alias_command alias | single 0 {2 1} {{FIXED 2 1}} | baked 0 {2 1} {{FIXED BAKED 2 1}} | nested 0 {2 1} {{INNER OUTER BAKED 2 1}} | moved_alias 0 {2 1} {{FIXED 2 1}} | moved_target 1 {invalid command name "target"} {} | replacement_four 1 {wrong # args: should be "target a b c d"} | replacement_baked_four 0 {2 1} . Captured words precede the selected two comparator inputs; nested INNER precedes OUTER and BAKED. Alias movement retains capture, target movement fails lookup, and four-formal replacement accepts only the added BAKED case.

### bigip

Status: `not-tested`.

No appliance process or event callback observation is retained for this question.

## Consumer bindings

No implementation binding is claimed by this observation record.

## Evidence

- [rust/tcl-registry/tests/data/native_callback_alias_source255/8.4.20](../../../../rust/tcl-registry/tests/data/native_callback_alias_source255/8.4.20)
- [rust/tcl-registry/tests/data/native_callback_alias_source255/8.5.19](../../../../rust/tcl-registry/tests/data/native_callback_alias_source255/8.5.19)
- [rust/tcl-registry/tests/data/native_callback_alias_source255/8.6.18](../../../../rust/tcl-registry/tests/data/native_callback_alias_source255/8.6.18)
- [rust/tcl-registry/tests/data/native_callback_alias_source255/9.0.4](../../../../rust/tcl-registry/tests/data/native_callback_alias_source255/9.0.4)
- [rust/tcl-registry/tests/data/native_callback_alias_source255/9.1.0](../../../../rust/tcl-registry/tests/data/native_callback_alias_source255/9.1.0)
- [rust/tcl-registry/tests/data/native_callback_alias_source255/jim](../../../../rust/tcl-registry/tests/data/native_callback_alias_source255/jim)
- [rust/tcl-registry/tests/data/native_callback_alias_source255](../../../../rust/tcl-registry/tests/data/native_callback_alias_source255)
