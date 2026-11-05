# IMAP4 core-package requirement

The exact Tcllib 2.0 IMAP4 header contains `package require Tcl 8.5 9` before
`package provide imap4 0.5.5` and the namespace body. The separate
[compiler coverage audit](../lsp-2243-coverage/README.md) loses command-table
knowledge after that first invocation. These native captures measure the
package behaviour; they do not establish a compiler fix for #2243.

| Input | Measured result |
| --- | --- |
| Exact header | C Tcl 8.5.19, 8.6.18, 9.0.4 and 9.1.0 return their actual provided `Tcl` version. Tcl 8.4.20 and current Jim reject the two requirement words for argument count. The measured global command-name snapshot is unchanged in all rows. |
| Provided and forgotten packages | All five C releases run no `ifneeded` loader for an already provided package, both for a compatible request and an incompatible request that errors. An external package and `Tcl` after explicit `package forget` each execute the registered loader and create a command. |
| Jim boundary | Exact checkout `82fb4d0caf90240d94352a31fcbd16adf52658a7` reports `0.84-10-g82fb4d0`, supplies no initial `Tcl` provision and rejects `ifneeded`. The external/forgotten cases are not executed there; provided-package results use Jim's dummy `1.0` version. |

The standalone execution evidence retains each exact script once, all six
actual environments, launch channel, process statuses and streams:

- [Exact core-package header](../../../evidence/tcl/lsp-2243-core-package-header.md).
- [Provided-package loader boundary](../../../evidence/tcl/lsp-2243-provided-package-loader-boundary.md).

The adjacent `*.results.json` files preserve the original scripts and process
captures. Each row was a new process receiving the script on standard input,
with no arguments and `LC_ALL=C`. Errors under test were caught by the script;
the process exit status is a separate field. No Tcllib package body or F5
provider was executed.

Current implementation owners are independent from these native observations:

- `tcl-registry::commands::tcl::package_` supplies the typed `PackageTransition::Require`, argument layout and version/requirement validation.
- `tcl-compiler::command_binding::apply_package_transition` preserves source package state when every retained provision is present; absent or mixed provisions keep loader uncertainty.
- `TclVersion::core_provided_packages` describes the authored release survey. Its documented contract does not authenticate a current interpreter's package database or patch version.
- `NativeCompilationEntry` explicitly does not assert package providers. A command/namespace entry cannot supply a package provision.

The C 9.0.4 source in `generic/tclPkg.c`, `PkgRequireCore`, sends an already
provided version directly to requirement validation and bypasses loader
selection. `package forget` removes that provision and its registered scripts.
Jim's `jim-package.c` instead uses a dummy package version and ignores requested
versions. This source inspection is development material, separate from the
captured execution evidence above.
