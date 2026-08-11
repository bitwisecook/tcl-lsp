# Command semantic oracle audits

This queue records command-by-command checks against supported Tcl releases.
It distinguishes source evidence from executable-oracle evidence, so an
unavailable source tree cannot become an unmarked assumption in a registry
descriptor. Registry data remains the only per-command implementation site;
compiler and language-server consumers project the typed facts generically.

## Queue

| Command | Tcl 9 verdict | Registry action | Evidence status |
| --- | --- | --- | --- |
| `rename` | complete | corrected namespace-creation transition and hover text | Tcl 9.0.4 and Tcl 8.5 executable oracles; Tcl 9.0 manpage |
| `package` | complete with typed abstention | corrected Tcl 8.4/8.5 `vsatisfies` arity split | Tcl 9.0.4 primary; Tcl 8.6.18 and 8.5.9 cross-checks |

## `rename`

### Evidence availability

The user-requested source glob `/Users/jimd/src/tcl-[89]*` had no matches.
The repository-configured fetched-source directories were also absent in both
the audit worktree and the primary checkout:

```text
tmp/tcl8.4.20/
tmp/tcl8.5.19/
tmp/tcl8.6.16/
tmp/tcl9.0.4/
tmp/tcl9.1b0/
```

The completed executable checks used `/usr/bin/tclsh8.5` and
`/opt/homebrew/bin/tclsh9.0` (Tcl 9.0.4). The Tcl 9.0 `rename(n)` manpage was
read from `/opt/homebrew/Cellar/tcl-tk/9.0.4/share/man/mann/rename.ntcl`.
No Tcl 8.4, 8.6, or 9.1 source or executable claim is made by this audit.

### Verified contract

| Surface | Verdict |
| --- | --- |
| Canonical name and form | `rename oldName newName`; no alias or option form observed. The registry's exact-two-word arity remains correct. |
| Version and dialect availability | The form is present in the Tcl 8.5 and 9.0 executable oracles. `ALL_TCL` remains the registry availability group; iRules remains excluded by ordinary dialect intersection. |
| Names and namespaces | Both operands resolve as command names in the current namespace unless qualified. A non-empty qualified target creates all missing target namespaces, then moves the binding. An empty target deletes instead. |
| Result and dispatch | Successful move or deletion returns the empty string. A missing source does not invoke `unknown`; it errors directly. |
| Errors | Missing source: `can't rename "name": command doesn't exist`; Tcl 9.0 reports `TCL LOOKUP COMMAND name`, Tcl 8.5 reports `NONE`. Existing target, including a self-target: `can't rename to "name": command already exists`; Tcl 9.0 reports `TCL OPERATION RENAME TARGET_EXISTS`, Tcl 8.5 reports `NONE`. |
| Traces and mutation ordering | A command rename trace receives `oldName newName rename`; deletion receives `oldName {} delete`. A trace callback returning an error did not abort the Tcl 8.5 or 9.0 operation. Checked failures left the source binding unchanged and did not create a target namespace. |
| Aliases and interpreters | A renamed `interp alias` remains an alias at its new name. The operation succeeded in safe and ordinary child interpreters; command tables are interpreter-local. |
| Dynamic operands | A substituted source or target is not statically knowable. The registry emits an unknown command-binding transition and widens command-binding, namespace, and command-trace state rather than guessing. |

### Registry and optimiser verdict

The old registry commentary incorrectly described a non-existent target
namespace as a `bad command name` error. Tcl 8.5 and 9.0 instead create the
namespace lineage. The resolver now emits a typed `NamespaceTransition::Ensure`
before its `CommandBindingTransition::Move` for a literal qualified target. A
shared Tcl-compatible namespace-qualifier helper centralises the parsing rule
used by both `namespace qualifiers` and this resolver. Generic world-state SSA
then projects the namespace lineage; no compiler or LSP path names `rename`.

The conservative abrupt-edge policy remains: it preserves a possibly observed
command-table update across trace and callback re-entrancy without claiming
that a trace callback error itself aborts `rename`. No new diagnostic or quick
fix is sound for successful implicit namespace creation. The improved state
fact keeps later qualified resolution, optimisation, and navigation precise.

## `package`

### Evidence availability

The requested source glob and repository-fetched source trees listed above
were absent. The primary executable oracle was
`/opt/homebrew/bin/tclsh9.0` (Tcl 9.0.4), with cross-checks on Tcl 8.6.18 and
`/usr/bin/tclsh8.5` (Tcl 8.5.9). The Tcl 9.0.4 manual is installed as
`/opt/homebrew/share/man/mann/package.ntcl`. Tcl 8.4 and 9.1 were unavailable,
so those two executable gates remain explicitly unverified.

### Verified contract

| Surface | Verdict |
| --- | --- |
| Dispatch and availability | `package` is a Tcl core ensemble. Tcl 9 adds `files`; Tcl 8.5 adds `prefer` and the variadic requirement grammar. The registry gates `files` to `TCL90_PLUS`, `prefer` to `TCL85_PLUS`, and excludes iRules through `ALL_TCL`. |
| Subcommands | The registry covers `forget`, `ifneeded`, `names`, `prefer`, `present`, `provide`, `require`, `unknown`, `vcompare`, `versions`, and `vsatisfies`, plus Tcl 9's `files`. Unique-prefix dispatch follows the generic ensemble resolver. |
| Version selection | A bare requirement is bounded to the same major series; `min-` is unbounded above; `min-max` has an exclusive upper bound; equal bounds pin one version. Multiple requirement words are alternatives. `-exact` accepts one version. Stable mode prefers the highest stable match, falling back to unstable; `prefer latest` selects the highest match. |
| State and lifecycle | `provide` records one interpreter-local version and rejects a conflicting second version. `ifneeded` installs or replaces a deferred global-namespace load script. `require` checks provided state, evaluates the selected script, then invokes `package unknown` and retries. `forget` clears provided and deferred records. |
| Loading and auto-index coupling | A require script may `source` Tcl, `load` a binary extension, or require other packages. The default unknown handler searches `auto_path` and `pkgIndex.tcl`; a custom handler is a command prefix receiving the package name and requirements. The load boundary is therefore external and dynamic. |
| Results and errors | Query forms return strings or lists; mutators return the empty string. Missing `present`, unsatisfied `require`, conflicting versions, and malformed `-exact` calls error. Error-code detail differs between Tcl 8.x and 9.x and is not promoted to a version-invariant registry fact. |
| Dynamic, namespace, and safe-interpreter cases | Package databases are interpreter-local. Load scripts run at global level even when called from a namespace, and safe or child interpreters have their own package records and command visibility. Computed package values cannot select a release or prove command availability. |
| Arity and argument roles | `ifneeded` carries a deferred body; `unknown` carries a command prefix; `-exact` is a flag on `present` and `require`. `vsatisfies` has exactly two trailing arguments in Tcl 8.4 and is variadic from Tcl 8.5; this split is now registry form data. |

### Registry and optimiser verdict

The registry already held the high-confidence package facts: subcommands,
release gates, `-exact`, deferred bodies, callback arity, external-unit traits,
result types, and interpreter-state effects. This audit adds the missing Tcl
8.4 versus 8.5+ `vsatisfies` arity form and a dialect-aware registry test.

A redundant `package require` rewrite is not generally sound. An earlier call
may have run a dynamic unknown handler or load script, and package state is
mutable through `forget`, `provide`, `ifneeded`, and interpreter boundaries.
Only a proof carrying the same interpreter, requirement, completion, and
package-world versions could justify it.

### Package world-state transition decision

`WorldStateDomain::PackageState` already lets common effects version the coarse
package world. The structured transition vocabulary does not yet have a
package domain or facts for provided and deferred maps, preference mode,
unknown-handler replacement, interpreter identity, and abrupt script
completion. This audit therefore adds no guessed transition. Package and load
boundaries retain their generic external-unit and interpreter-state effects.

### Release-gate matrix

| Release | Gate result |
| --- | --- |
| Tcl 8.4 | Source and executable unavailable. The registry retains the legacy two-argument `vsatisfies` form; no new runtime claim is made. |
| Tcl 8.5 | Tcl 8.5.9 confirms `prefer`, variadic requirements, `-exact`, alternative selection, and deferred loading. |
| Tcl 8.6 | Tcl 8.6.18 confirms the same surface except Tcl 9-only `files`; error-code structure remains Tcl 8.x-specific. |
| Tcl 9.0 | Tcl 9.0.4 and its manpage confirm `files`, selection, lifecycle, global load scripts, and Tcl 9 error-code families. |
| Tcl 9.1 | Source and executable unavailable; existing registry inclusion is retained but not newly asserted. |
