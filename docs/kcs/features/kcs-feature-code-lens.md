# KCS: feature — Code Lens

> **Audience:** User
> **Type:** Functionality

## Summary

Reference counts shown above each proc, class, TclOO method, and classmethod
definition. Click the lens to find all references.

## Applies to

all-editors, analyser

## Question

What are the numbers that appear above my proc, class, and method definitions?

## How to use

Code lenses appear automatically above every `proc` definition, every
`oo::class create` declaration, and every TclOO `method` / `classmethod`
inside a class body, in a Tcl or iRules file. Each lens shows how many
references exist to that symbol across the current file (and the workspace,
for procs and classes, if indexing is enabled). Click the lens to open the
Find References panel.

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
}
set f [Factory new]
$f get
Factory make
```

The lens updates as you type. If you rename or remove a call, the count adjusts on the next keystroke.

## Failure modes

- A method / classmethod lens above a TclOO member always resolves to a
  clickable command, the same as a proc or class lens — a lens that shows a
  count but does nothing when clicked is a bug (issues #724, #956), not
  expected behaviour.

## Related

- [KCS feature index](README.md)
- [References](kcs-feature-references.md) — the Find References provider the lens delegates to
- [Document Symbols](kcs-feature-document-symbols.md) — the outline view that also shows procs
