# KCS: feature — Code Lens

> **Audience:** User
> **Type:** Functionality

## Summary

Reference counts shown above each proc and TclOO method definition. Click the lens to find all references.

## Applies to

all-editors, analyser

## Question

What are the numbers that appear above my proc definitions?

## How to use

Code lenses appear automatically above every `proc` definition — and every TclOO `method` / `classmethod` definition — in a Tcl or iRules file. Each lens shows how many references exist to that proc or method across the current file (and the workspace, if indexing is enabled). A method lens counts dispatch sites (`$obj method`, `my method`). Click the lens to open the Find References panel.

No configuration is needed. The feature can be toggled with `tclLsp.features.codeLens`.

## Example

```tcl
proc greet {name} {          ;# "2 references" appears above this line
    puts "Hello, $name"
}

greet "Alice"                 ;# reference 1
greet "Bob"                   ;# reference 2
```

The lens updates as you type. If you rename or remove a call, the count adjusts on the next keystroke.

## Related

- [KCS feature index](README.md)
- [References](kcs-feature-references.md) — the Find References provider the lens delegates to
- [Document Symbols](kcs-feature-document-symbols.md) — the outline view that also shows procs
