# KCS: feature — Document Highlight and Linked Editing

> **Audience:** User
> **Type:** Functionality

## Summary

Highlight every occurrence of the symbol under the cursor, with read/write distinction for variables, and linked editing for proc names.

## Applies to

all-editors, analyser

## Question

Why do some variable occurrences light up in a different colour when I click on them, and what is linked editing?

## How to use

### Document highlight

Place the cursor on any variable, proc, or class name. The editor highlights every occurrence in the same file:

- **Variables**: definitions are marked as **Write** (brighter background), and reads are marked as **Read** (dimmer background).
- **Procs and classes**: all occurrences are marked as **Text** (uniform highlight).

This is automatic and requires no keybinding. Toggle with `tclLsp.features.documentHighlight`.

### Linked editing

When you place the cursor on a proc name that calls itself recursively, the declaration and all recursive self-calls inside that proc's body link together so edits sync across every site. This lets you rename a recursive proc without opening the full rename dialog.

Toggle with `tclLsp.features.linkedEditingRange`.

## Example

```tcl
proc factorial {n} {              ;# Write highlight (definition)
    if {$n <= 1} { return 1 }
    return [expr {$n * [factorial [expr {$n - 1}]]}]
}                                  ;# ↑ linked editing: rename syncs
                                   ;#   both "factorial" sites
```

Placing the cursor on `$n` highlights the parameter definition and all three read sites with different intensities.

## Related

- [KCS feature index](README.md)
- [References](kcs-feature-references.md)
- [Rename](kcs-feature-rename.md) — full cross-file rename
