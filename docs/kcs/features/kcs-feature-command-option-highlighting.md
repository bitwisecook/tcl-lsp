# KCS: feature — Command option highlighting

> **Audience:** User
> **Type:** Functionality

## Summary

Words that read as `-option` switches, and the values they take, are given
their own semantic-token colours — an option token for the switch and an
option-value token for its argument — so a command's flags stand out from its
positional strings.

## Applies to

all-editors, analyser

## How to use

Applied automatically as part of semantic highlighting.  No configuration;
toggles with `tclLsp.features.semanticTokens`.

## Example

```tcl
lsort -index 2 -unique $rows          ;# -index / -unique coloured as options, 2 as an option value
puts -foo hello                       ;# -foo stays a plain string: puts declares no such switch
set chart [ticklecharts::chart new]
$chart Xaxis -name {v} -type value    ;# -name / -type resolved from the chart class
lsearch -exact -- -inline $items      ;# -- ends option scanning, so -inline is an operand
```

## What gets resolved

There are three layers, most-precise first.

1. **Registry-declared options.** For a command the registry knows
   (`lsort -index 2`, `file delete -force …`), only the switches the command
   actually declares are highlighted, so a stray `puts -foo` stays a plain
   string.  Value-taking options colour their value too — an enum value as an
   enum member, otherwise an option value.

2. **Object-method options.** For a handle bound by a constructor
   (`set chart [ticklecharts::chart new]`, then `$chart Xaxis -name {v}`), the
   handle's class is tracked from the `new` call, the method is resolved
   against that class, and its declared options and values are coloured
   precisely.  Direct `[Class new] method …` dispatch resolves the same way.

   Resolution reaches classes you define yourself, not only registry-modelled
   ones: the method is looked up through the class hierarchy
   (`superclass` / `mixin`), and for an `oo::configurable` receiver its
   `configure` / `cget` `-property` options come from the class's declared
   properties.  The hierarchy is merged across the workspace, so a class
   defined in another file still resolves.

   Where the handle came from is tracked through four kinds of edge: a proc
   parameter bound by an object argument, a proc that returns an object, an
   assignment (`set b $a`), and an object passed into a constructor and stored
   on an instance variable.  The class flows along chains of those edges.

   A **self-call** inside a class body resolves against the enclosing class —
   `my method …` (`TclOO`), `$self method …` (snit), and `$this method …`
   (itcl); `my configure -prop` colours the property option too.  Recognising
   the enclosing type is driven by the registry's definer-family grammar, not
   a hardcoded definer list.

   A snit or itcl object bound by its named constructor
   (`set o [foo create x]`) types the variable as that class.  A snit
   **component** installed with `install NAME using TYPE …`, or created by the
   bare-word form `set NAME [TYPE inst …]`, resolves `$NAME method …` in the
   body.  The bare-word form fires only when `TYPE` is a known snit-family
   class and the first argument is not a `typemethod`, so a `set x [Type
   info …]` call is not mistaken for a construction.

   A **Tk/ttk widget's instance command** (`.t instate …`, `$w tag
   configure …`) resolves on the widget path rather than an object identity.
   Widget constructors are registry data, so both the bareword form
   (`ttk::treeview .t` then `.t instate …`) and the captured form
   (`set lb [listbox .l]` then `$lb curselection`) resolve their subcommands,
   options, and argument values precisely.

   The receiver's class can also come from an object **collection**: a dict
   filled with `[Pin new]` makes `[dict get $Pins $pin] configure -node …`
   resolve to `Pin`, and a loop over the collection (`dict for`, `dict map`,
   `foreach`, including one nested in a command substitution) binds its value
   variable the same way.  A collection holding several classes keeps the
   whole set and resolves the method against any member.

3. **Generic fallback.** For an unknown command head, a word shaped like a
   clean option — a leading `-` then a letter — is highlighted as an option
   and its following literal value as an option value.

The fallback respects Tcl's own conventions: a negative number (`-5`, `-1.6`)
or a special-float literal (`-inf`, `-nan`) is not an option; a substitution
word (`-$var`, `-[cmd]`) keeps its own highlight; and `--` stops option
scanning.

### Computed command heads are not command tokens

A command whose head word is computed — `$obj method …`,
`[dict get $Pins $pin] method …`, or a multi-fragment `chartV$node …` — has a
name that is only known at run time, so the head is not painted as a resolved
command.  It is tokenised as ordinary code instead: a `[…]` substitution
recurses into its inner script, and a `$var` head reads as a variable.  The
words after the head are still highlighted by the passes above.

## Failure modes

- A `-option` on an unmodelled object method is highlighted by shape only, so
  an invalid switch is not distinguished from a valid one.
- `installhull $win` (the already-created form, which names no widget type)
  and a bareword named-object dispatch onto a class you defined
  (`Foo create obj; obj method`) fall back to shape matching.
- A receiver whose class is bound only in another file — a cross-file instance
  variable, global, or parameter — is not tracked.  Only the class *hierarchy*
  crosses files, not the handle provenance.  This applies to widget paths too.
- A widget path built dynamically rather than captured from a literal
  constructor call (`set w .t; ttk::treeview $w; $w instate …`) is not
  tracked, and renaming a widget's instance command breaks the association.
- A negative-number argument that a command genuinely treats as a value is
  correctly *not* highlighted as an option.

## Discoverability

- [KCS feature index](README.md)
- [Command registry](../../../docs/design/compiler/command-registry.md)
- [Semantic tokens](kcs-feature-semantic-tokens.md)
- [TclOO object-type tracking — design](../../design/analysis/tcloo-object-typing.md)
- [Tk widget instance-command typing — design](../../design/analysis/tk-widget-instance-typing.md)
