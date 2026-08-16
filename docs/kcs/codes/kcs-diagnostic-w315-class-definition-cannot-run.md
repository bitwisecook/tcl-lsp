# KCS: W315 — Why does the analyser say "this class definition cannot run"?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, analyser, command-walk

## Profiles

default

## Question

Why is a `deletemethod` or `renamemethod` inside my `oo::class create` /
`oo::define` body flagged, when the body looks fine?

## Why

`TclOO`'s retracting member words are *strict*. `deletemethod` and
`renamemethod` require the member they name to already exist **on the side of
the class they are scoped to**, and `renamemethod` requires the destination
name to be free. When either requirement is broken, Tcl does not skip the
statement and carry on — it raises an error out of the whole definition script,
so **no class is created at all**.

That is what makes this worth a diagnostic rather than a shrug: the file looks
like it declares a class, the outline shows one, and at run time
`oo::class create` throws and nothing exists.

The three shapes are byte-identical on tclsh 9.0.4 and 8.6.14:

```tcl
oo::class create ::E1 { deletemethod ghost ; method ghost {} {} }
;# -> method ghost does not exist          [info object isa class ::E1] -> 0

oo::class create ::E2 { self { method cm {} {} } ; deletemethod cm }
;# -> method cm does not exist             (`cm` is class-side; the unwrapped
;#                                          `deletemethod` looks instance-side)

oo::class create ::E3 { method a {} {} ; method b {} {} ; renamemethod a b }
;# -> method called b already exists

oo::class create ::E4 { method a {} {} ; renamemethod a a }
;# -> cannot rename method to itself
```

### The two sides

A class keeps the members it gives its *instances* and the members defined on
the class *object* itself in separate tables, and every member word acts on
exactly one of them:

| Spelling | Side | Introspection |
|---|---|---|
| `deletemethod m` | instance | `info class methods` |
| `private deletemethod m` | instance | `info class methods` |
| `self deletemethod m` | class object | `info object methods` |

Neither reaches across — which is why the `::E2` example above fails: the
unwrapped `deletemethod cm` looks for `cm` on the instance side, where it was
never declared.

### What is *not* flagged

`export` and `unexport` are the words that look similar but behave differently:
naming a member that does not exist on their side is a **silent no-op**, not an
error, so they are never reported.

```tcl
oo::class create E { method onlyinst {} {} }
oo::define E { self unexport onlyinst }   ;# succeeds, changes nothing
oo::define E { export ghost }             ;# succeeds too
```

`filter` likewise names a method without requiring it to exist
(`oo::class create H { filter nosuch }` is legal, and
`info class filters ::H` really answers `nosuch`).

## Symptoms

- A yellow squiggle on the member-name word inside a class body, with the
  message "this class definition cannot run: …".
- At run time the `oo::class create` / `oo::define` raises `method … does not
  exist` or `method called … already exists`, and the class is missing.

## Example that triggers it

```tcl
oo::class create ::Widget {
    deletemethod render
    method render {} { return "…" }
}
```

The analyser reports **`W315`** on `render` in the `deletemethod` line.

## Fix

Put the declaration before the retraction — that is the only legal order:

```tcl
oo::class create ::Widget {
    method render {} { return "…" }
    deletemethod render
}
```

For the cross-side form, scope the word to the side the member is really on:

```tcl
oo::class create ::Widget {
    self { method make {} { … } }
    self deletemethod make
}
```

For a rename onto a name already taken, retract the destination first (legal —
the check reads the table state at the point the word runs, exactly as Tcl
does):

```tcl
oo::class create ::Widget {
    method a {} { … }
    method b {} { … }
    deletemethod b
    renamemethod a b
}
```

## When it abstains

- A **dynamic** member name (`deletemethod $name`, `renamemethod $old new`)
  names nothing statically, so nothing is reported.
- A **cross-file** `oo::define` extension — a stub for a class created in
  another file — has no member tables to judge against, and a retraction
  naming a member declared elsewhere is the normal shape there, not an error:

  ```tcl
  # a.tcl
  oo::class create ::C { method m {} { … } }
  # b.tcl
  oo::define ::C { deletemethod m }     ;# legal, and not reported
  ```

  An `oo::define` extending a class created *earlier in the same file* is not a
  stub — it reuses that class's tables — so it is checked normally.
- Per-object bodies (`oo::objdefine $obj { … }`) are not checked, because the
  per-object member state has nowhere to live across blocks yet.

## Navigation still works

The partial class is still recorded, so the outline, hover, and
go-to-definition keep working on whatever the body declared — the same way a
file with a parse error still navigates. The diagnostic is the signal that the
class will not exist at run time; it does not erase the editor's model of it.

## How to suppress

Add `# noqa: W315` on the line **above** the offending command, or
`# tcl-lsp: disable=W315` at the top of the file.

## Related

- [KCS codes index](README.md)
- [W308 — unknown TclOO method](kcs-diagnostic-w308-unknown-tcloo-method.md)
- [Document symbols feature](../features/kcs-feature-document-symbols.md)
- [Completions feature](../features/kcs-feature-completions.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
