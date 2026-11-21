# Review: the `overrides` package against tcl-lsp

[`Dechcaudron/overrides`](https://github.com/Dechcaudron/overrides) at `fe18ee3` (version 1.0, MIT) temporarily replaces commands for the duration of a script: `overrides::withCmdOverrides DICT SCRIPT`, where `DICT` maps command names to `{lambda LAMBDA}` or `{command NAME}`, and `overrides::nextOverride` inside a lambda calls the previous definition. It renames the original to a mangled name, installs a dispatcher, and while any override is active it also overrides `::rename` and `::proc` to protect the dispatchers.

Its 34 tests pass on tclsh 8.6.18, 9.0.4 and 9.1.0. Fixtures and outputs for everything below are in [`oo-dispatch-probe/external-overrides/`](handover/oo-dispatch-probe/external-overrides/); the package itself is not copied.

## Faults in the package

Measured on tclsh 8.6.18 and 9.0.4 with the package loaded through `TCLLIBPATH`. These are upstream's to fix; they matter here because code that uses the package will hit them.

| Case | Result | Cause |
| --- | --- | --- |
| Override a global command, a built-in such as `clock`, or a command reached by `namespace path`, from code inside a namespace | `can't rename "::app::greet": command doesn't exist` | `ResolveCmdName` takes the unqualified name `info commands` returns and prefixes the caller's namespace, but `info commands` also matches global commands |
| `proc helper …` or `rename ::bar baz` inside an overridden script | creates `::overrides::helper` and `::overrides::baz`; `helper` is then `invalid command name` | the protective `proc`/`rename` lambdas run in namespace `overrides`, and `nextOverride` evaluates one level up, inside that namespace |
| A command name containing a space or `[` | `wrong # args: should be "info commands ?pattern?"` | `uplevel $level info commands NAME` concatenates its arguments into a script and re-splits the name (our W301 flags each of these sites) |
| `return` inside the script | returns from `withCmdOverrides`, not from the calling procedure | `try { uplevel … }` without `-level` handling; unlike `if`, `catch` or `try` bodies |
| `break` inside the script, within a loop | `invoked "break" outside of a loop` | same |
| The README's first example | `can't read "fileName"` | the lambda declares `{filename}`; the example also uses `import` instead of `namespace import` |

## What tcl-lsp needs to support code that uses it

Measured on `0de6cc6d` with go to definition, hover and the analyser's diagnostics, on a usage file alone and with the package's source inlined (as a workspace containing it would have).

1. **The script argument is not code to us.** Inside `withCmdOverrides … { set result [load a.tcl] }`, go to definition and hover on `load` return nothing, with or without the package's source present. The package runs the script with `uplevel 1 eval [list $script]`, so it is a script in the caller's frame: its commands should be checked and navigable, and its variable writes (`result`) visible after the call. This needs the evaluation-context declarations of the [command lookup plan](command-lookup-centralisation.md) step 2, either declared for the package in SpecTcl or derived from the procedure body: a parameter passed to `uplevel 1` (directly, or through `eval [list …]`) is a caller-frame script.
2. **The override dictionary is code inside data.** The lambdas are never walked, so nothing in them is checked, and `nextOverride` inside them has no definition or hover. This needs the value grammars of [root cause 6](oo-tk-root-causes.md): "a dictionary whose keys are command names and whose values are `lambda LAMBDA` or `command NAME`", where a lambda is an `apply` body and a name is a command reference (navigation, rename, unknown-command checks).
3. **A helper that exists only inside particular bodies.** `nextOverride` is valid only inside an override lambda and calls the previous definition with that definition's arguments, as TclOO's `next` does inside a method. The object-system profile's receiver-helper role ([object systems plan](object-systems-plan.md)) should be usable for non-object definers too.
4. **Command meaning changes for the extent of a script.** Inside the script, `load` dispatches to the lambda, so arity, definition and hover should use the override (or present both); after it, the original. This is a scoped command-world transition: the [fine-grained transitions](oo-tk-root-causes.md) and the lookup frame, not a whole-world reset.

With these, a SpecTcl declaration for the package would read roughly (proposed syntax):

```tcl
command overrides::withCmdOverrides {
    arg 0 -role Value -grammar {dict -keys CommandName -values {choice {lambda Lambda -helpers {nextOverride -chains-to key}} {command CommandName}}}
    arg 1 -role Body -frame caller -command-overrides-from 0
}
```

## Our own faults it exposed

- **False W214 and a lost W301 in `overrides::ResolveCmdName`.** Both parameters are reported unused although both are read, and the W301 on its first `uplevel` disappears. Removing the line `set resolvedName "${callerNamespace}::$resolvedName"`, or writing it `"$callerNamespace\::$resolvedName"`, removes both effects; that line in a small procedure does not reproduce it on its own, so it depends on the rest of the body. Reproducer: `external-overrides/w214-reproducer.tcl`, with `w214-reproducer-without-line-10.tcl` and `w214-reproducer-unbraced-name.tcl` as the passing variants. The `${name}::suffix` form is the [naming guide](naming-api-guide.md)'s own example of a reference whose boundary matters.
- **Whole-file specialisation of library procedures.** In a file with one call `f x 0`, I230 reports `if !$b` inside `f` as constant. For a package whose procedures are called from other files this reports facts about the one in-file caller as facts about the procedure.
- **Correctly reported:** W301 at the `uplevel` calls that concatenate a script (the cause of the space and bracket failures above), apart from the one lost in `ResolveCmdName`, W100 on `if ![…]` and `if !$x`, W110 on `==`/`!=` string comparisons, W113 on a user `load` shadowing the built-in.
