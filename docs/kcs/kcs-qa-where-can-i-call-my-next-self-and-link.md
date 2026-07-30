# KCS: Where can I call `my`, `next`, `self`, and `link`?

> **Audience:** User
> **Type:** Q&A

## Applies to

all-editors, diagnostic, analyser

## Question

Why does `link foo` (or `my`, `next`, `nextto`, `self`, `classvariable`)
report an unknown command when I write it outside a class method?

## Answer

Because it really is unknown there. These six words are not global Tcl
commands. A TclOO method body runs with the object's own namespace
current, and that namespace searches `::oo::Helpers` before the global
namespace — so the words resolve inside a method body and nowhere else.
Checked against Tcl 9.0:

```tcl
% link foo
invalid command name "link"
% info commands ::link
                            ;# empty
```

Inside a method the same word resolves:

```tcl
oo::class create Counter {
    method bump {} { return 1 }
    method run {} {
        link bump          ;# fine — creates a bareword `bump`
        return [bump]
    }
}
```

The editor follows the same rule everywhere. Outside a method body you
get a **`W123`** hint, no hover, and no completion entry for these words;
inside one you get all three, plus completion of the barewords a `link`
call installed.

### Which bodies count

All of these are method bodies for this purpose:

- `method NAME {args} { ... }`
- `constructor { ... }` and `destructor { ... }`
- a class-side method — `self method NAME { ... }` or `classmethod NAME { ... }`
- `oo::objdefine $obj { method NAME { ... } }`

### Which bodies do not

- The top level of a script.
- An ordinary `proc` body, however deeply nested.
- An `apply` lambda — even one written *inside* a method. `apply` runs its
  body in the global namespace, so the object context is gone:

```tcl
oo::class create Counter {
    method bump {} { return 1 }
    method run {} {
        apply {{} { link bump }}   ;# invalid command name "link"
    }
}
```

### The fully qualified spelling

`::oo::Helpers::link` (and `::oo::Helpers::next`, `…::nextto`, `…::self`,
`…::classvariable`) *are* real commands in the global command table, so
writing one of them draws no `W123` and hovers normally. Calling one
outside a method still fails at run time, with a different message: `link
may only be called from inside a method`.

`my` has no qualified spelling: each object gets its own `my` command in
its own namespace (`::oo::Obj22::my`), which no name written in a script
can reach.

### Per-Tcl-version differences

- Tcl 8.6 and 8.7 core ship `my`, `next`, `nextto`, and `self` only.
  `link` comes from the Tcllib `ooutil` package there — add `package
  require ooutil` and the editor stops reporting a missing package
  (`W120`). `classvariable` is Tcl 9.0+.
- Tcl 9.0 ships all six in the core, no package needed.

Even with `ooutil` loaded, a top-level bare `link` is still an unknown
command under 8.6 — the package installs `::oo::Helpers::link`, which
only a method body can reach by its bare name.

## Related

- [W123 — Is this command unresolved?](codes/kcs-diagnostic-w123-unresolved-command.md)
- [W120 — missing `package require`](codes/kcs-diagnostic-w120-missing-package-require.md)
- [How are command names resolved?](kcs-qa-how-are-command-names-resolved.md)
- [Command registry design](../design/compiler/command-registry.md)
