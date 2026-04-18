# KCS: feature — Type Navigation (TclOO)

> **Audience:** User
> **Type:** Functionality

## Summary

Go to Type Definition, Go to Implementation, and Type Hierarchy browsing for TclOO classes and methods.

## Applies to

all-editors, analyser

## Question

How do I navigate TclOO class hierarchies — jump to a variable's class, find method overrides, or browse superclasses?

## How to use

Three standard LSP navigation features work together for TclOO code:

### Go to Type Definition

Place the cursor on a variable whose type the compiler can infer (e.g., a `my` method call) and run **Go to Type Definition**. The editor jumps to the class definition.

### Go to Implementation

- On a **class name**: shows all subclasses.
- On a **method name**: shows all overrides of that method in subclasses.

### Type Hierarchy

Right-click a class or method and choose **Show Type Hierarchy**, or use the keyboard shortcut. The hierarchy view shows:

- **Supertypes**: the parent class chain plus mixins.
- **Subtypes**: every direct subclass.

## Example

```tcl
oo::class create Animal {
    method speak {} { return "..." }
}

oo::class create Dog {
    superclass Animal
    method speak {} { return "Woof" }   ;# ← Go to Implementation on
}                                        ;#   Animal::speak finds this

set pet [Dog new]
$pet speak                               ;# ← Go to Type Definition
                                         ;#   jumps to Dog class
```

## Related

- [KCS feature index](README.md)
- [Definition](kcs-feature-definition.md) — Go to Definition for procs, variables, and commands
- [Call Hierarchy](kcs-feature-call-hierarchy.md) — browse caller/callee trees
