# KCS: O107 — Eliminate unreachable dead code

> **Audience:** User
> **Type:** Functionality

## Applies to

all-editors, optimisation, dce

## Profiles

full

## Question

What does O107 rewrite, and when does it fire?

## Why

Code that can never execute wastes reader attention and source size; removing it clarifies intent and shrinks the script.

## Before

```tcl
return $x
puts "never reached"
```

## After

```tcl
return $x
```

## Safety conditions

- Skipped when the analyser cannot prove the code is unreachable on all paths.
- Skipped at top level when removal would change observable script results.
- A branch guarded by a variable trace or a frame-aliased variable
  (`upvar`/`global`/`variable`) is never treated as provably
  unreachable, even when the condition otherwise looks constant — the
  compiler cannot assume the traced/aliased value at runtime, so both
  arms stay live and O107 does not fire on either one:

  ```tcl
  proc setup {} { trace add variable ::x read onread }
  set x 1
  setup
  if {$x} {
      puts yes
  } else {
      puts no
  }
  ```

  Here `puts no` survives even though `$x` is `1` at every call —
  eliding it would silently drop the read that fires the trace.

## How to disable

Toggle the optimiser profile in your editor settings. See the [optimiser feature](../features/kcs-feature-optimiser.md) for profile options.

## Related

- [KCS codes index](README.md)
- [Optimiser feature](../features/kcs-feature-optimiser.md)
- [DCE](../../GLOSSARY.md#dce)
- Related codes: `O108`, `O109`
