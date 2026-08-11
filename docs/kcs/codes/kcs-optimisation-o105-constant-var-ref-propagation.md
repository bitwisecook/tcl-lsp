# KCS: O105 — Propagate constants into variable references (GVN/CSE)

> **Audience:** User
> **Type:** Optimisation

## Applies to

all-editors, optimisation, gvn

## Profiles

standard, full

## Question

What does O105 rewrite, and when does it fire?

## Why

Replacing `$var` with its known value reduces runtime work and memory traffic.
The same optimiser also identifies common expressions, but it reuses a command
result only when dispatch and observation are proven safe.

## Before

```tcl
set endpoint /health
set copiedEndpoint $endpoint
```

## After

```tcl
set endpoint /health
set copiedEndpoint /health
```

## Safety conditions

- Skipped when the variable may be modified between definition and use.
- Registry-declared stable list and formatting operations can become common
  expression candidates. The optimiser still withholds call reuse until it can
  prove that command rebinding, aliases, `unknown`, and live command or
  execution traces cannot change or observe dispatch at that program point.
- Volatile results, such as the current clock time, are never candidates.
  Results that read package, environment, locale, or other versioned
  interpreter state require matching world versions before they can be reused.
- Skipped when the duplicated command has side effects that must execute twice.
- Skipped when the value contains metacharacters unsafe for the target context.

O105 is an optimiser report, not an automatic editor quick fix. The optimiser
keeps the original expression whenever any required proof is unavailable.

## How to disable

Toggle the optimiser profile in your editor settings. See the [optimiser feature](../features/kcs-feature-optimiser.md) for profile options.

## Related

- [KCS codes index](README.md)
- [Optimiser feature](../features/kcs-feature-optimiser.md)
- [GVN](../../GLOSSARY.md#gvn)
- Related codes: `O100`, `O106`
