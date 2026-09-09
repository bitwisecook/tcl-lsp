# KCS: W123 — Is this command unresolved?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, command-walk

## Profiles

default

## Question

Why does the analyser flag a command it cannot resolve?

## Why

A command the analyser cannot find in the registry, user procs, or unknown handler will likely fail at runtime.

## Symptoms

- A hint underline under the command name, with the message "Unknown command
  'unknownCmd'; did you mean 'unknown'?" — the "did you mean" tail appears only
  when a close match exists.

## Example that triggers it

```tcl
unknownCmd $arg
```

The analyser reports **`W123`** on `unknownCmd`.

A proc, class, `rename` target, or `interp alias` that was renamed or
deleted away, with no later re-establishment under the same name, is also
unresolved — calling it fails `invalid command name` at runtime just like
a name that was never defined:

```tcl
proc helper {} { return 1 }
rename helper {}
proc caller {} { helper }
```

The analyser reports **`W123`** on `helper` inside `caller`. Defining a
fresh `helper` (or `rename`-ing a different command to that name) after
the deletion re-establishes it and clears the warning.

## What does not trigger it

A call the file demonstrably makes *before* the deletion is fine, even when it
is written inside a proc body. The analyser follows the chain of enclosing
definitions: if a top-level call reaches the proc before the deletion runs,
the nested call resolved when it ran.

```tcl
proc helper {} { return hi }
proc inner {} { helper }
proc outer {} { inner }
outer
rename helper {}
```

Nothing is flagged: `outer` runs on the fourth line, which runs `inner`, which
runs `helper` — all before the rename.

The chain only follows call sites guaranteed to run. A call inside an `if`, a
loop, or a `switch` arm proves nothing about whether the enclosing proc
reaches it, so it does not silence a warning on a later call:

```tcl
proc helper {} { return hi }
proc b {} { helper }
proc a {} { if {0} { b } }
a
rename helper {}
b
```

The analyser reports **`W123`** on `helper` inside `b`: the last line really
does fail with `invalid command name "helper"`.

A `rename` or deletion inside an `if`, a loop, or a `switch` arm is treated
the same way in the other direction. Because it might never run, it is not
proof that the command is gone:

```tcl
oo::class create Dog {
    method bark {} { return woof }
}
if {0} { rename Dog {} }
Dog new
```

Nothing is flagged on `Dog`. The rule is about where the `rename` is written,
not whether the branch is taken: `if {1} { rename Dog {} }` behaves the same.

A name vacated by a `rename` is still flagged even though the command survives
under its new name. `rename Dog Cat` moves the class to `Cat`; `Dog new`
afterwards fails with `invalid command name "Dog"`.

## Commands defined in another file

The check reads one file at a time, so a `proc` that lives in a sibling
file is not something it can see on its own. Three things make it visible.

Always on: a command an installed library auto-loads (`tclIndex`), and a
command defined by a package the file `package require`s. The package's
`pkgIndex.tcl` is found on the search path, including one the file builds for
itself with `lappend auto_path [file dirname [file dirname [info script]]]`.
That search follows Tcl's own rules: `set auto_path {…}` contributes one
directory per **list element** (`lappend` one per argument word), paths use
Tcl slash form so a Windows `[info script]` resolves against its own
directory, and `package require NAME 2.0` reads the release `package
vsatisfies` would pick.

Also always on: an `expr` math function the **workspace** defines. Nothing
below is reported:

```tcl
# helper.tcl
namespace eval tcl::mathfunc {
    proc Pi {} { return 3.141592653589793 }
}

# vector.tcl — no source, no package require
proc angle {dp} { return [expr {Pi()}] }
```

`expr` looks a function up in `::tcl::mathfunc`, one table per interpreter, so
a workspace that defines `Pi` there defines the one `Pi` every `expr` in it
can call — hence no setting. The match is exact for the same reason: a `proc`
named `Pi` in another namespace is a different command that `expr {Pi()}`
cannot reach, and the report stands.

Off by default, opt in with `tclLsp.features.crossFileResolution`: every
`proc` and class the **workspace** defines, whether or not anything links the
two files. It is a setting because a workspace is not always one program — two
unrelated scripts in the same folder each have their own `helper`.

A name nothing in the workspace defines is still reported either way.

## The quick fix and the report always agree

The "did you mean …?" quick fix is offered only where the report itself is.
If any rule above resolved the name, both are gone — accepting one would
otherwise rewrite working code, turning `expr {Pi()}` into `expr {ni()}`.

## What a `proc unknown` changes

Defining your own `unknown` handler at **global** scope switches the check off
for the whole file: Tcl sends every unresolved command word to that handler,
so no name can be proved unresolved.

```tcl
proc unknown {cmd args} { exec $cmd {*}$args }
totallyBogusCommand
```

Nothing is flagged here.

A `proc unknown` written **inside a namespace** is an ordinary procedure
that happens to share the name, and changes nothing:

```tcl
namespace eval ::mylib {
    proc unknown {cmd args} { exec $cmd {*}$args }
}
totallyBogusCommand
```

The analyser still reports **`W123`** on `totallyBogusCommand`, and running it
really does fail: Tcl consults `::unknown` for a bare unresolved word whatever
the calling namespace. For a per-namespace handler use `namespace unknown
NAME`, which the analyser models separately.

## Commands that only resolve inside a TclOO method

`link`, `my`, `next`, `nextto`, `self`, and `classvariable` are reachable only
from inside a method body — a `method`, `constructor`, `destructor`,
class-side method, or `oo::objdefine method`. Anywhere else they really are
unresolved:

```tcl
# tcl-dialect: tcl9.0
link foo
```

The analyser reports **`W123`** on `link`, and running it fails with `invalid
command name "link"`. The same call inside a method body is fine, as is the
fully qualified `::oo::Helpers::link`, a real command everywhere.

An `apply` lambda inside a method body does **not** count: `apply` runs its
body in the global namespace, so the object context is gone.

A Tcl 9 class `initialise` / `initialize` body is **not** flagged, even though
only `my` works there. That body runs in the class object's own namespace, so
the words are found — calling one fails with `self may only be called from
inside a method`, a different error. Completion and hover still decline to
offer them there.

For the full rule, see
[Where can I call `my`, `next`, `self`, and `link`?](../kcs-qa-where-can-i-call-my-next-self-and-link.md).

A built-in `expr` math function (`sin`, `max`, `abs`, …) called with
function-call syntax inside `expr` resolves to the command it dispatches to
(`::tcl::mathfunc::<name>`) and never draws `W123`, whether or not it has
also been overridden by a `proc ::tcl::mathfunc::<name>` in the file:

```tcl
set x [expr {sin(1.0) + max(1, 2, 3)}]
```

## Fix

```tcl
package require mypackage
mypackage::knownCmd $arg
```

Define the command, or use `package require` to load the package that provides it.

## How to suppress

Add `# noqa: W123` on the line **above** the offending command.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [command walk](../../GLOSSARY.md#command-walk)
- Related codes: `W120`, `W307`
