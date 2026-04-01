# v1.4.1

## New Features
- **TclOO introspection commands**: added command definitions for `my`,
  `self`, `next`, `nextto`, and `classvariable`, providing hover
  documentation and completion for TclOO method-context commands.
- **`tcl::mathop` operator commands**: all `tcl::mathop::*` operators
  (`+`, `-`, `*`, `/`, `==`, `<`, `in`, `eq`, etc.) are now registered
  with hover and arity validation.
- **`lremove` command**: added command definition for the Tcl 9.0
  `lremove` command.
- **Dialect detection from `package require Tcl`**: source files
  containing `package require Tcl 9.0` (or other versions) now
  auto-detect the correct dialect without needing a shebang or comment
  directive.
- **Workspace-level dialect upgrade**: when any file in the workspace
  requires a higher Tcl version, the dialect automatically upgrades for
  the entire workspace.

## Improvements
- **W307 suppression in method bodies and `dict with` scopes**: the
  non-literal command name diagnostic is now suppressed inside TclOO
  method bodies and functions containing `dict with`/`dict update`,
  where `$var method` patterns are common and expected.
- **W308 external superclass tolerance**: method validation no longer
  emits false positives for classes that inherit from external
  (unresolved) superclasses.
- **External class constructor inference**: `[ClassName new]` where
  `ClassName` is not locally defined is now inferred as a TclOO
  constructor, suppressing W307 on the resulting object variable.
- **W123 alias suppression**: unknown command warnings are now
  suppressed for commands defined via `interp alias` in the workspace.
- **Return value tracking**: variables used in `return $var` are no
  longer falsely flagged as unused (W211) or dead stores (O126).
  Braced returns (`return {$var}`) are correctly identified as literals.
- **`dict with`/`dict update` SSA modelling**: variables unpacked by
  `dict with` and `dict update` are now tracked as definitions in SSA,
  eliminating false read-before-set and unused-variable warnings.
- **`dict for`/`dict map` iteration variables**: loop variables from
  `dict for` and `dict map` are now registered as SSA definitions.
- **Global variable dead store suppression**: assignments to
  namespace-qualified variables (`::var`) are no longer flagged as dead
  stores or unused, since they are consumed externally.
- **`.test` file extension**: Tcl test files (`.test`) are now
  recognised by VS Code and Zed editors.
- **Separated generated docs**: diagnostic and optimisation code tables
  are now generated as individual files alongside the combined table.
- **`package ifneeded` body role removed**: the script argument is no
  longer treated as a body form, fixing false diagnostics inside
  `package ifneeded` scripts.

## Bug Fixes
- Fixed backslash-newline mid-word parsing: `foo\<newline>{bar}` now
  correctly tokenises `{bar}` as a braced string instead of absorbing
  it into the preceding word.
- Fixed glob expansion of operator-named commands (`*`, `+`, `[`, etc.)
  in the iRule test framework TMM shim, which caused `::orch::init` to
  fail with `can't rename "::*"`.
- Fixed subprocess analysis deadlocks by switching to `forkserver`
  multiprocessing context on platforms that support it, avoiding
  fork-with-threads lock contention.
- Added 15-second timeout and thread fallback for subprocess analysis,
  preventing indefinite hangs when the process pool is unhealthy.
