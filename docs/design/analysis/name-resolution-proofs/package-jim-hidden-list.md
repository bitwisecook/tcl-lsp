# naming.package.jim-hidden-list

Kind: `native-observation`

## Problem statement

The Jim package unknown-member error names forget, names, provide and require. A catalogue rebuilt only from that report could remove a supported hidden list operation, even though an error message is a presentation roster rather than the complete operation inventory.

## Question

Does current Jim package list succeed with the same result as package names even though the exact unknown-member error omits list from its advertised operations?

## Conclusion

Jim 0.84-9-g5bac7c9 accepts both list and names in this fresh process with catch code 0 and the same recorded package sequence. The deliberate absent operation returns catch code 1 and an error roster containing forget, names, provide and require, omitting the successfully executed list operation. This one native control proves that this error roster is incomplete for supported package operations; it supplies no C or BIG-IP answer and no guarantee about arbitrary prefixes or other hidden commands.

## Scope

One fixed ASCII script-file program in a fresh current Jim native process. Actual patchlevel, complete input/executable/stream digests, three caught outcomes and outer process status are recorded. Dynamic enumeration content belongs to this process, not a universal installed-package set.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1, bigip

Status: `not-tested`.

No observation for this exact question.

### jim

Status: `observed`. Channel: Fixed ASCII script-file argument in a fresh native shell process. Version: jim 0.84-9-g5bac7c9.

Exact stdout:

```text
VERSION|0.84-9-g5bac7c9
RESULT list 0 {file glob clock eventloop signal tclcompat array tree tclprefix history ensemble binary initjimsh jsonencode zlib syslog aio nshelper stdlib json posix namespace oo interp regexp readdir exec pack}
RESULT names 0 {file glob clock eventloop signal tclcompat array tree tclprefix history ensemble binary initjimsh jsonencode zlib syslog aio nshelper stdlib json posix namespace oo interp regexp readdir exec pack}
RESULT __r2286_absent__ 1 {package, unknown command "__r2286_absent__": should be forget, names, provide, require}
```

The list/names catch results are both 0 and contain the identical printed package sequence. The absent-member catch result is 1 and omits list from the help roster. Outer process status is 0 and stderr is empty.

## Consumer bindings

- [rust/tcl-registry/src/commands/tcl/package_.rs](../../../../rust/tcl-registry/src/commands/tcl/package_.rs), `commands::tcl::package_::tests::jim_package_inventory_keeps_hidden_list_and_excludes_ifneeded` (linked): Authored genuine Jim-point inventory keeps list and names with zero-argument List metadata; the native error text and returned package sequence are evidence-only, not reproduced by this Rust test.

## Evidence

- [rust/tcl-registry/tests/data/native_jim_package_hidden_list/probe.tcl](../../../../rust/tcl-registry/tests/data/native_jim_package_hidden_list/probe.tcl)
- [rust/tcl-registry/tests/data/native_jim_package_hidden_list/stdout](../../../../rust/tcl-registry/tests/data/native_jim_package_hidden_list/stdout)
- [rust/tcl-registry/tests/data/native_jim_package_hidden_list/stderr](../../../../rust/tcl-registry/tests/data/native_jim_package_hidden_list/stderr)
