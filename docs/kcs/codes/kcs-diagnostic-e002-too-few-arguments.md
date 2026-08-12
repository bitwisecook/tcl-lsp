# KCS: E002 — Why does the analyser say a command has too few arguments?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, lowering

## Profiles

default

## Question

Why do I see a red squiggle saying a command was called with too few arguments?

## Why

Calling a command with fewer arguments than it requires will always raise a runtime error. Catching this statically prevents unexpected failures in production.

This check is not limited to builtin commands: it also applies to same-file `proc` calls, `interp alias` targets (shifted by any prepended arguments), `rename`d commands (which keep the original's arity — and, if the old name is later re-declared as a fresh `proc`, the *new* declaration's own arity, not the original's), TclOO methods and `forward`s (including `forward NAME my TARGET ?ARG…?`, the idiom for forwarding to a sibling or inherited method), and reachable TclOO manufacturer calls (`ClassName new ?args?` / `ClassName create name ?args?`, checked against the nearest explicit `constructor` in the class's inheritance chain — a class with no `constructor` anywhere in its hierarchy is never checked, since `TclOO`'s built-in default constructor accepts any number of arguments). `createWithNamespace` has its own registry layout but is unexported in C Tcl; ordinary class-command calls are therefore not treated as successful construction. The check also covers `next`/`nextto` calls inside a method body (checked against the resolved next-in-MRO method or `nextto`'s named target — see the TclOO section below), and direct calls to an inline `apply {{params} body} ?args?` lambda.

## Symptoms

- A red squiggle appears under the command, with a message like "Too few
  arguments for 'puts': expected at least 1, got 0 — usage: puts ?-nonewline?
  ?channelId? string".
- The " — usage: …" tail quotes the command's synopsis so the expected call
  shape is visible in the message itself. It appears only when the analyser
  has a registry signature for the command (builtins and known extension
  commands); calls to your own `proc`s, TclOO methods and constructors, and
  `apply` lambdas keep the count-only message.
- The same squiggle can appear on a call to a `proc` you defined earlier in the file, an `interp alias`, a `rename`d command, or a `$obj method` call — not just builtin commands.

## Example that triggers it

```tcl
puts
```

The analyser reports **`E002`** on the bare `puts` token.

```tcl
proc greet {name} {
    return "hello $name"
}
greet
```

The analyser reports **`E002`** on the call, since `greet` requires one argument.

## Command-prefix callback context

`E002` also fires on a **callback proc that requires more arguments than its
command prefix supplies**. When a command invokes a callback (`lsort -command
cb`, `trace add … cb`, `$graph walk … -command cb`), it appends a fixed number
of arguments; if the referenced proc has more *required* parameters than that,
the runtime call raises "too few arguments". Here the squiggle is under the
**callback proc name** (the head of the prefix), not under the calling command
— look at the proc it names.

```tcl
proc cmp {a b c} { return 0 }
lsort -command cmp {3 1 2}   ;# lsort appends only 2 → E002 on `cmp`
```

Fix by giving the extra parameters defaults (`{a b {c 0}}`) or removing them so
the callback matches the appended-argument count. (A callback whose appended
count is open-ended — `AtLeast(n)` — never draws `E002`.)

**This check also works across files by default** when the call site's exact
C Tcl resolution candidate names the proc. The server then reads the same
project-wide signature table used by navigation. `crossFileResolution` is
only needed for its broader, deliberately lossy bare-name workspace inference.
Every other E002 case on this page (same-file `proc`/alias/`rename`/TclOO
calls, `next`/`nextto`, constructors, `apply`) is likewise unconditional.
`crossFileResolution` is independent of `xcDiagnostics` (the unrelated,
f5-irules-only XC100-301 translatability diagnostics).

## TclOO `next` / `nextto` context

`next` re-invokes the current method's implementation on the next class along
the receiver's MRO; `nextto CLASS` jumps straight to `CLASS`'s implementation
instead. Both are checked against the resolved target method's own arity —
the *method* body's parameter list, not the calling method's:

```tcl
oo::class create Base { method speak {a b} { return "$a$b" } }
oo::class create Derived {
    superclass Base
    method speak {a b} { next 1 }   ;# Base::speak needs 2 → E002 on `next`
}
```

A `next`/`nextto` outside a method body, or one with no further provider
along the MRO (the chain is exhausted), draws no diagnostic — both are
runtime errors in Tcl itself ("next may only be called from inside a
method" / "no next"), not statically-checkable arity mismatches.

## Tk widget instance-dispatch context

`E002` also fires on a **Tk/ttk widget's own instance command** when the
analyser can trace the receiver back to the widget that created it:

```tcl
ttk::treeview .t
.t move onlyone   ;# `move` needs (item parent index) → E002
```

This works for a bareword receiver (`.t`, reusing the literal path text) and
for a `$var` holding a widget the constructor's return value was captured
into (`set lb [listbox .l]; $lb …`). `configure`/`cget` are never
arity-checked (every widget accepts them, but no widget spec declares their
shape).

## Fix

```tcl
puts "hello"
```

Supply the required arguments so the command can execute successfully.

## How to suppress

Add `# noqa: E002` at the end of the offending line.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [lowering](../../GLOSSARY.md#lowering)
- Related codes: `E001`, `E003`, `E005` (wrong argument-count *shape* — an
  in-range count that doesn't fit a key/value-pair or paired-argument
  command like `dict create`/`foreach`/`switch`)
