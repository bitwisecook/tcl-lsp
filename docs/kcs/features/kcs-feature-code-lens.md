# KCS: feature — Code Lens

> **Audience:** User
> **Type:** Functionality

## Summary

Reference counts shown above each proc, class, TclOO method, classmethod,
property, constructor, and destructor definition. Click the lens to find
all references.

## Applies to

all-editors, analyser

## Question

What are the numbers that appear above my proc, class, method, property, and constructor/destructor definitions?

## How to use

Code lenses appear automatically above every `proc` definition, every
`oo::class create` declaration, every TclOO `method` / `classmethod` inside a
class body, every `property` declaration (`oo::configurable`'s `property
name -get {...} -set {...}` form), and every explicit `constructor` /
`destructor`, in a Tcl or iRules file. Each lens shows how many references
exist to that symbol across the current file (and the workspace, for procs
and classes, if indexing is enabled). Click the lens to open the Find
References panel.

A property's count comes from a class-local scan of `my <property>` sites —
properties have no `$obj property` dispatch shape and no inheritance model,
unlike methods and classmethods.

A constructor or destructor is invoked positionally (`ClassName
new`/`create`/`destroy`), never dispatched by name, so a conventional "N
references" count has no general meaning for it the way it does for a
`method`/`classmethod`/`property`. Its lens is scoped to one specific,
name-independent relationship instead: an overriding subclass's own
constructor/destructor chaining up to this one via `next` / `nextto`. The
count resolves that chain through the full class hierarchy (skipping past
an intermediate ancestor that declares no constructor/destructor of its
own), not just the immediate superclass.

No configuration is needed. The feature can be toggled with `tclLsp.features.codeLens`.

## Example

```tcl
proc greet {name} {          ;# "2 references" appears above this line
    puts "Hello, $name"
}

greet "Alice"                 ;# reference 1
greet "Bob"                   ;# reference 2

oo::class create Base {
    constructor {} { }         ;# "1 reference" — Sub's `next` below chains to this one
}
oo::class create Factory {
    superclass Base
    constructor {} { next }     ;# "0 references" — nothing chains into Factory's own
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

- A method / classmethod / property / constructor / destructor lens above a
  TclOO member always resolves to a clickable command, the same as a proc or
  class lens — a lens that shows a count but does nothing when clicked is a
  bug (issues #724, #956, #992), not expected behaviour.
- A `method` and a `classmethod` sharing the same name on one class are
  counted and resolved independently — the method's lens never picks up the
  classmethod's `ClassName foo` dispatch sites (or vice versa), even though
  both are legal, separate members. The same independence holds for a
  `property` sharing a name with a `method` or `classmethod`.
- `oo::configurable` allows several `constructor` declarations on one class;
  only the last is ever effective. An earlier, shadowed `constructor` gets no
  lens — there is no reference story worth surfacing for a declaration that
  can never run.
- A `nextto SomeClass` inside a subclass's constructor/destructor is
  resolved to the class it actually names, not assumed to target the
  immediate superclass — a `nextto` that skips past a closer ancestor to an
  explicit target further up the chain counts only for that named target.

## Related

- [KCS feature index](README.md)
- [References](kcs-feature-references.md) — the Find References provider the lens delegates to
- [Document Symbols](kcs-feature-document-symbols.md) — the outline view that also shows procs
