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
| `namespace` | complete with typed abstentions | declared nested ensemble management dispatcher | Tcl 9.0.4 primary; Tcl 8.5.9 cross-check |
| `source` | complete with typed abstentions | corrected Tcl 9 option forms and exact option spellings | Tcl 9.0.4 primary; Tcl 8.6.18 and 8.5.9 cross-checks |
| `interp` | complete for the static core surface | retain Tcl 9's `slaves` compatibility spelling and type `target` as a Tcl list | Tcl 9.0.4 and Tcl 8.5 executable oracles; Tcl 9.0.4 `interp(n)` manpage |

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

## `namespace`

### Evidence availability

The requested source glob `/Users/jimd/src/tcl-[89]*` had no matches. The
repository's fetched Tcl-source directories were also absent, so this audit
does not claim a source-code reading of `tclNamesp.c`.

The primary executable oracle was `/opt/homebrew/bin/tclsh9.0` (Tcl 9.0.4),
with its installed `namespace(n)` manual. Version-boundary checks used
`/usr/bin/tclsh8.5` (Tcl 8.5.9). Tcl 8.4, 8.6, and 9.1 executables were not
available to this worktree.

### Verified contract

| Surface | Verdict |
| --- | --- |
| Dispatch and abbreviation | The outer dispatcher has exactly `children`, `code`, `current`, `delete`, `ensemble`, `eval`, `exists`, `export`, `forget`, `import`, `inscope`, `origin`, `parent`, `path`, `qualifiers`, `tail`, `unknown`, `upvar`, and `which`; each accepts a unique non-empty prefix. The `namespace ensemble` management dispatcher is exactly `create`, `configure`, and `exists`, also prefix-resolved. |
| Namespace identity and lifecycle | Relative namespace words resolve only below the current namespace. `eval` creates its target and missing ancestors; `inscope`, `children`, `delete`, `parent`, `path`, and `upvar` require existing namespaces. `delete` removes a namespace tree, but a currently executing frame is retained until return and is no longer name-resolvable. |
| Code execution | `eval` concatenates its script words as `eval` does; `inscope` appends later words as list elements; `code` returns an `::namespace inscope NS script` callback prefix. The registry records structural bodies, concatenation/list-append distinctions, and the wrapped-prefix fact. |
| Import/export and lookup | Export patterns are namespace-local, append-only until `-clear`, and may name commands not yet defined. Imports have snapshot semantics, demand a qualified exporting namespace, and reject collisions unless the single leading `-force` replaces the target binding. `forget` removes imported aliases only. `origin` follows an import chain, while `which` is a non-throwing command/variable probe. |
| Path and unknown dispatch | `path` replaces the current namespace's command-resolution path (Tcl 8.5+); bare command resolution is current namespace, path, global namespace, then a per-namespace unknown handler. A non-empty `unknown` handler makes failed command lookup dynamically provider-dependent; an empty handler resets it. |
| Ensembles | `create` makes the current namespace's ensemble unless `-command` supplies the command name; `configure` observes or mutates it; `exists` probes it. `-map`, `-subcommands`, `-prefixes`, `-unknown`, and `-command` date from Tcl 8.5; `-parameters` is Tcl 8.6+. Map targets are command prefixes, and a later map replacement removes stale map dispatch. |
| Names as data | `qualifiers` and `tail` are pure string splits, independent of namespace existence. `import`, `export`, and `forget` operands are patterns, not namespace symbols. `origin` is a command-name reference; `which -command` is a command-name probe; `which -variable` is a variable read; `upvar` writes only each local pair target. |
| Dynamic, child/safe, and trace cases | Computed namespace, path, handler, map, import, and ensemble operands retain the registry's conservative unknown/dynamic state rather than inventing bindings. Namespace state is interpreter-local, so child/safe interpreter and alias effects are intentionally represented through the generic interpreter and command-binding facts. `namespace delete` is destructive; trace/re-entrancy ordering is not promoted to a stronger static transition. |
| Results | Query forms return their documented scalar/list values; successful state-mutating forms normally return the empty string. Result stability is encoded where it drives consumers (`Boolean`, `List`, `String`, and const-fold descriptors), without treating stateful queries as pure. |

### Version and dialect verdict

`namespace` is core Tcl (`ALL_TCL`) and absent from the iRules profile through
ordinary registry availability. Tcl 8.5 adds `ensemble`, `path`, `unknown`,
and `upvar`; Tcl 8.5 requires an `upvar` pair, whereas Tcl 8.6+ permits the
namespace-only no-op. The Tcl 8.5 and 9.0.4 runs directly confirmed the
`upvar` split, the 8.5 ensemble/path/unknown surface, prefix dispatch, import
collision/`-force`, `code`'s generated prefix, path resolution, and deletion.

### Registry and consumer verdict

The existing command spec already held the command-level arity, roles,
effects, dynamic-evaluation, namespace declaration, import/export lifecycle,
path, unknown-handler, ensemble-mutation, and constant-fold facts. This
tranche adds the one missing static surface: `namespace ensemble`'s second
dispatcher is now `SubCommand::sub_subcommands`. Generic semantic-token,
hover, and completion consumers therefore discover `create`, `configure`, and
`exists`, including Tcl's unique-prefix rule, without a compiler or LSP
command-name branch.

### Typed abstentions

No static result can soundly predict a dynamic handler's result, computed
path/map/import target, command trace re-entrancy, or an alias/rename mutation
performed by an arbitrary evaluated script. Those cases remain typed
namespace/command-binding/interpreter-state uncertainty; the audit makes no
new diagnostic or optimisation claim for them. The Rust VM/WASM runtime's
implementation completeness is also outside this registry oracle tranche.

## `source`

### Evidence availability

The requested `/Users/jimd/src/tcl-[89]*` source glob and the repository's
fetched Tcl source trees are absent. The primary executable oracle was
`/opt/homebrew/bin/tclsh9.0` (Tcl 9.0.4), cross-checked with Tcl 8.6.18 and
8.5.9. Tcl 9.0.4's installed `source` manual was read from
`/opt/homebrew/share/man/mann/source.ntcl`. Tcl 8.4 and 9.1 executable/source
oracles remain unavailable.

### Verified contract

| Surface | Verdict |
| --- | --- |
| Forms and options | `source fileName` is universal. Tcl 8.5+ adds `-encoding encodingName fileName`; Tcl 9 adds a separate `-nopkg fileName` form. Tcl 9.0.4 rejects combining `-encoding` and `-nopkg`, and Tcl 8.5/8.6 reject `-nopkg`. The complete option spellings are required: `-enc` and `-nop` are rejected across the checked releases. The registry now exposes separate Tcl 9 forms and full-length option abbreviation floors. |
| Caller frame and namespace | The file is evaluated in the caller's current interpreter, call frame, and namespace. A top-level `set` in a sourced file can create or overwrite a local in the caller's proc. `info script` reports the path spelling supplied to `source`: a relative name remains relative and a symlink name is not canonicalised. |
| Result and completion | The result is the last command's result. A top-level `return value` ends the sourced file and returns normally from `source`; it does not unwind the caller. Errors propagate with their result and completion code. An empty file returns the empty string. |
| File reading and encoding | Reading stops at the first ASCII `^Z` byte. Tcl 8.5/8.6 default to the system encoding when `-encoding` is omitted; Tcl 9 defaults to UTF-8. A leading BOM is ignored for the documented Unicode encodings. Missing files and invalid encodings are file/read errors, not package-resolution facts. |
| Resolution and identity | Relative names resolve against the process's current working directory, not the directory of the calling file. Each invocation reads the target again; Tcl does not cache a successful `source` by path or inode. Repeated-source elimination is therefore unsound without an external proof that the file is immutable and side-effect free. |
| Safe and child interpreters | `source` is hidden in a newly created safe interpreter and remains unavailable to a child created inside it. Explicit `interp expose` makes it callable again. Package databases, command tables, namespaces, and file execution are interpreter-local. |
| Traces and dynamic execution | Execution traces can observe `source` enter/leave and can affect completion. A sourced script may call arbitrary commands, load packages, mutate namespaces, or re-enter the caller. A computed filename cannot soundly identify a target file; the existing registry Source hook records a dynamic target and generic workspace/package consumers widen. |

### Registry and consumer verdict

The existing registry correctly carries `SOURCES_FILE`, external-unit loading,
dynamic-evaluation barrier, safe-interpreter hiding, file-read effects, the
caller-frame hook, and the Tcl 8.5/9 option gates. This tranche corrects only
the high-confidence Tcl 9 form/option facts and tests them in the registry.
The `Source` analyser hook remains the generic hand-off for recording a
source target and caller namespace; no compiler or LSP path rewrite was added.
Any future source-specific quick fix must be returned by a registry-owned hook
from generic resolved invocation/context facts, with the analyser/LSP applying
only the resulting edit plan.

No safe diagnostic or optimisation is added for repeated loads, dynamic
filenames, package initialisation, or arbitrary source-file effects. A
literal source target can feed existing document-link and workspace source
graph resolution, but it does not prove the target is executed exactly once.

### Release-gate matrix

| Release | Gate result |
| --- | --- |
| Tcl 8.4 | Source/executable unavailable. Existing registry retains the classic-Mac `-rsrc`/`-rsrcid` hover forms and records the known platform-locked arity gap; no new 8.4 runtime claim is made. |
| Tcl 8.5 | Executable oracle 8.5.9 confirms caller-frame evaluation, result/return/error propagation, `-encoding`, exact option spelling, ^Z termination, and repeated-load behaviour. |
| Tcl 8.6 | Executable oracle 8.6.18 confirms the Tcl 8.5 source surface and rejection of Tcl 9 `-nopkg`. |
| Tcl 9.0 | Primary executable oracle 9.0.4 confirms UTF-8 default, separate `-nopkg`, option non-combination, safe-interpreter hiding, traces, and path identity. |
| Tcl 9.1 | Source/executable unavailable; no new 9.1-specific claim is made. |

## `interp`

### Evidence availability

The requested `/Users/jimd/tcl-[89]*` source glob remains absent. This
tranche uses `/opt/homebrew/bin/tclsh9.0` (Tcl 9.0.4) as its primary
executable oracle, `/usr/bin/tclsh8.5` (Tcl 8.5.9) for the available older
surface, and the installed Tcl 9.0.4 manual at
`/opt/homebrew/Cellar/tcl-tk/9.0.4/share/man/mann/interp.ntcl`. No source-code
or Tcl 8.4, 8.6, or 9.1 executable claim is made.

### Verified contract

| Surface | Verdict |
| --- | --- |
| Dispatch and versions | The direct dispatcher accepts unique prefixes. Tcl 8.5 has `alias`, `aliases`, `bgerror`, `create`, `debug`, `delete`, `eval`, `exists`, `expose`, `hide`, `hidden`, `invokehidden`, `issafe`, `limit`, `marktrusted`, `recursionlimit`, `slaves`, `share`, `target`, and `transfer`; Tcl 8.6 adds `cancel` and `children`, both retained by Tcl 9.0. `set` remains Tcl 9.1-only. |
| Child topology and safety | `create` makes a child command and interpreter, inheriting safe policy from a safe parent; `-safe` explicitly restricts it. Paths are relative Tcl lists, `{}` denotes the invoking interpreter, and a child cannot name an ancestor except through aliases. `delete` removes each requested child and descendants left-to-right, retaining preceding deletions if a later path errors. |
| Alias lifecycle and dispatch | `alias` queries a source token, deletes it with an empty target path, or binds it to a target command plus prepended arguments. Target commands may be absent at creation. Alias invocation crosses interpreter domains; deleting or renaming the source token controls the alias binding. `aliases` enumerates tokens, and `target` returns either `{}` or the two-element `{targetPath targetCmd}` list. |
| Visibility and hidden dispatch | `hide` and `expose` rename entries between ordinary and hidden command tables; names are global-table names, not namespace-relative lookups. `invokehidden` runs the hidden command without a second substitution pass, with `-global`, Tcl 8.5+ `-namespace`, and `--` controlling execution context and option parsing. Trusted ancestors, not untrusted child code, may use the safe interpreter's hidden commands. |
| Evaluation and result transfer | `eval` concatenates script words like `concat`, evaluates them in the target interpreter's current frame, and returns both result and completion options. It is an isolated interpreter domain, not a caller-frame body. `cancel` is Tcl 8.6+; `limit`, `recursionlimit`, `debug`, and `bgerror` have query/set forms whose policy changes are typed separately from query results. |
| Channels and policy | `share` adds a channel reference in a destination interpreter; `transfer` removes it from the source. Resource-limit callbacks and `bgerror` prefixes run later, so they are recorded as command prefixes rather than eagerly evaluated bodies. `marktrusted` changes safety policy without exposing hidden commands. |
| Compatibility, lifecycle, and result typing | Tcl 9.0's manual documents `children` but not legacy `slaves`; the Tcl 9.0.4 oracle still accepts `interp slaves`, so it remains available in every core Tcl profile. `children` appeared in Tcl 8.6, so the registry marks `slaves` deprecated from 8.6 (not retired) and owns a semantics-equivalent matched-word hook to rewrite it to `children`. The old registry wrongly gated it to Tcl 8.x. The manual and executable agree that `interp target` is Tcl-list data, correcting its old scalar-string result type. |
| Dynamic operands | Computed paths, command identities, handler prefixes, and scripts retain typed interpreter/command-binding/policy widening. No static state transition claims which child, hidden command, alias target, or callback result such operands denote. |

### Registry and consumer verdict

The existing `interp` spec already owns subcommand arities, feature gates,
options, code-evaluation roles, prefix callbacks, interpreter topology and
policy transitions, command-binding alias transitions, and dynamic widening.
This tranche corrects the two observed gaps in that same registry surface:
`slaves` now remains available under Tcl 9, with a Tcl-8.6 lifecycle warning
and safe `children` quick fix, and `target` reports `TclType::List`. The
generic W144 lifecycle consumer now resolves registry-owned typed fix hooks
against core Tcl profile versions as well as package-version axes. No compiler
or LSP consumer gained an `interp` name branch.

### Typed abstentions

Alias targets, hidden-command availability, callback outcomes, channel
ownership, cancellation catch depth, and scripts assembled dynamically remain
runtime-dependent. The registry records their affected domains and only
materialises literal transitions; it does not infer cross-interpreter state
from dynamic strings. Rust VM/WASM completeness is outside this oracle
tranche.
