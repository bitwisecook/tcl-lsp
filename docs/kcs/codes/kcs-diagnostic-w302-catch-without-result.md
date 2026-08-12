# KCS: W302 — Does catch without a result variable hide errors?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, lowering

## Profiles

default

## Question

Why does the analyser warn when `catch` is used without a result variable?

## Why

Errors are silently discarded, hiding bugs and security issues that would otherwise surface during execution.

## Symptoms

- A blue squiggle (hint) appears under the `catch` call, with the message "catch without result variable".

## Example that triggers it

```tcl
catch {risky_command}
```

The analyser reports **`W302`** on the `catch` call.

## Fix

```tcl
catch {risky_command} result
```

Capture the result so that errors can be logged, inspected, or re-raised.

## The quick fixes

Two quick fixes are offered on the diagnostic. Both append words to the end
of the `catch` call, after the body's closing brace:

| Action | `catch {risky_command}` becomes |
|---|---|
| **Add catch result variable(s)** | `catch {risky_command} result` |
| **Add catch result + options variable(s)** | `catch {risky_command} result options` |

The second action is offered only where the active dialect documents the
options dictionary. Tcl 8.4 and F5 iRules have `catch script ?varName?` and
no options word, so under those dialects a single action is offered and it
appends ` var`.

The insertion point is the end of the **body**, wherever that is: a
multi-line body, a body followed by a trailing `;# comment`, an empty
`catch {}`, and a body containing nested `[...]` substitutions all place the
new word after the body's last character. Anchoring at the `catch` keyword —
where the squiggle sits — would write `catch result {risky_command}`, which
Tcl reads as a `catch` of the script `result`.

## Limits

- The variable names are `result` and `options` (or `var` under Tcl 8.4 and
  iRules), taken from the command's documented argument names. If the
  surrounding code already uses a variable of that name, applying the fix
  overwrites it — rename the inserted variable after applying.
- Because the fix writes a variable into the caller's frame, it is
  classified **behaviour-hardening**, not semantics-preserving, so
  **Fix All Safe Issues** never applies it. See
  [the fix-safety taxonomy](../kcs-qa-what-does-fix-all-safe-issues-apply.md).
- A `catch` whose body is not a literal script (`catch $body`) gets no
  diagnostic and no fix: there is nothing to prove about a script the
  analyser cannot see.

## How to suppress

Add `# noqa: W302` on the line **above** the offending command.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [lowering](../../GLOSSARY.md#lowering)
- Related codes: `W125`, `W200`
