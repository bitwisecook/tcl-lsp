# Command semantic oracle audits

This queue records command-by-command checks against supported Tcl releases.
It distinguishes source evidence from executable-oracle evidence, so an
unavailable source tree cannot become an unmarked assumption in a registry
descriptor.  Registry data remains the only per-command implementation site;
compiler and language-server consumers project the typed facts generically.

## Queue

| Command | Tcl 9 verdict | Registry action | Evidence status |
| --- | --- | --- | --- |
| `rename` | complete | corrected namespace-creation transition and hover text | Tcl 9.0.4 and Tcl 8.5 executable oracles; Tcl 9.0 manpage |

## `rename`

### Evidence availability

The user-requested source glob `/Users/jimd/src/tcl-[89]*` had no matches.
The repository-configured fetched-source directories were also absent in both
the audit worktree and the primary checkout:

```
tmp/tcl8.4.20/
tmp/tcl8.5.19/
tmp/tcl8.6.16/
tmp/tcl9.0.4/
tmp/tcl9.1b0/
```

The completed executable checks used `/usr/bin/tclsh8.5` and
`/opt/homebrew/bin/tclsh9.0` (Tcl 9.0.4).  The Tcl 9.0 `rename(n)` manpage was
read from `/opt/homebrew/Cellar/tcl-tk/9.0.4/share/man/mann/rename.ntcl`.
No Tcl 8.4, 8.6, or 9.1 source or executable claim is made by this audit.

### Verified contract

| Surface | Verdict |
| --- | --- |
| Canonical name and form | `rename oldName newName`; no alias or option form observed.  The registry's exact-two-word arity remains correct. |
| Version and dialect availability | The form is present in the Tcl 8.5 and 9.0 executable oracles.  `ALL_TCL` remains the registry availability group; the separate iRules surface remains excluded by ordinary dialect intersection. |
| Names and namespaces | Both operands resolve as command names in the current namespace unless qualified.  A non-empty qualified target creates all missing target namespaces, then moves the binding.  An empty target deletes instead. |
| Result and dispatch | Successful move or deletion returns the empty string.  A missing source does not invoke `unknown`; it errors directly. |
| Errors | Missing source: `can't rename "name": command doesn't exist`; Tcl 9.0 reports `TCL LOOKUP COMMAND name`, Tcl 8.5 reports `NONE`.  Existing target, including a self-target: `can't rename to "name": command already exists`; Tcl 9.0 reports `TCL OPERATION RENAME TARGET_EXISTS`, Tcl 8.5 reports `NONE`. |
| Traces and mutation ordering | A command rename trace receives `oldName newName rename`; deletion receives `oldName {} delete`.  A trace callback returning an error did not abort the Tcl 8.5 or 9.0 operation.  The checked missing-source and target-collision errors left the source binding unchanged and did not create a target namespace. |
| Aliases and interpreters | A renamed `interp alias` remains an alias at its new name.  The operation succeeded in a safe child interpreter and in an ordinary child interpreter; command tables are interpreter-local. |
| Dynamic operands | A substituted source or target is not statically knowable.  The registry emits an unknown command-binding transition and widens command-binding, namespace, and command-trace state rather than guessing. |

### Registry and optimiser verdict

The old registry commentary incorrectly described a non-existent target
namespace as a `bad command name` error.  Tcl 8.5 and 9.0 instead create the
namespace lineage.  The `rename` transition resolver now emits a typed
`NamespaceTransition::Ensure` before its `CommandBindingTransition::Move` for
a literal qualified target.  A shared Tcl-compatible namespace-qualifier
helper centralises the parsing rule used by both `namespace qualifiers` and
this resolver.  Generic world-state SSA then projects the namespace lineage;
no compiler or LSP path names `rename`.

The existing conservative abrupt-edge policy is retained: it preserves a
possibly observed command-table update across trace and callback re-entrancy,
without claiming that a trace callback error itself aborts `rename`.

No new user diagnostic or quick fix is sound for the successful implicit
namespace creation.  The improved state fact can, however, keep later
namespace-qualified command resolution, optimisation, and navigation precise
after a literal rename.
