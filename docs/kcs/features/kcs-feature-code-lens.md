# KCS: feature — Code Lens

> **Audience:** User
> **Type:** Functionality

## Summary

Reference counts shown above each proc, class, TclOO method, classmethod,
and property definition. Click the lens to find all references.

## Applies to

all-editors, analyser

## Question

What are the numbers that appear above my proc, class, method, and property definitions?

## How to use

Code lenses appear automatically above every `proc` definition, every
`oo::class create` declaration, every TclOO `method` / `classmethod` inside a
class body, and every `property` declaration (`oo::configurable`'s `property
name -get {...} -set {...}` form), in a Tcl or iRules file. Each lens shows
how many references exist to that symbol across the current file (and the
workspace, for procs and classes, if indexing is enabled). Click the lens to
open the Find References panel.

A property's count comes from a class-local scan of `my <property>` sites —
properties have no `$obj property` dispatch shape and no inheritance model,
unlike methods and classmethods.

No configuration is needed. The feature can be toggled with `tclLsp.features.codeLens`.

## Example

```tcl
proc greet {name} {          ;# "2 references" appears above this line
    puts "Hello, $name"
}

greet "Alice"                 ;# reference 1
greet "Bob"                   ;# reference 2

oo::class create Factory {
    method get {} { return 1 }       ;# "1 reference" — the $f get call below
    classmethod make {} { return [Factory new] }  ;# "1 reference" — Factory make
    property size -get {return $mySize} -set {set mySize $value}  ;# "1 reference" — my size below
    method resize {} { my size }
}
set f [Factory new]
$f get
Factory make
```

The lens updates as you type. If you rename or remove a call, the count adjusts on the next keystroke.

## Failure modes

- A method / classmethod / property lens above a TclOO member always
  resolves to a clickable command, the same as a proc or class lens — a lens
  that shows a count but does nothing when clicked is a bug (issues #724,
  #956, #992), not expected behaviour.
- A `method` and a `classmethod` sharing the same name on one class are
  counted and resolved independently — the method's lens never picks up the
  classmethod's `ClassName foo` dispatch sites (or vice versa), even though
  both are legal, separate members. The same independence holds for a
  `property` sharing a name with a `method` or `classmethod`.
- A class's `constructor` and `destructor` do not get a lens: both are
  invoked positionally (`ClassName new`/`create`), never dispatched by name,
  so a conventional "N references" count has no obvious meaning for them the
  way it does for a `method`/`classmethod`/`property`.

## Related

- [KCS feature index](README.md)
- [References](kcs-feature-references.md) — the Find References provider the lens delegates to
- [Document Symbols](kcs-feature-document-symbols.md) — the outline view that also shows procs
