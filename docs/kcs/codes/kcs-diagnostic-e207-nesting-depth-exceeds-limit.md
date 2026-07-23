# KCS: E207 — Why does the analyser stop collecting diagnostics partway through deeply nested code?

> **Audience:** User
> **Type:** Issue

## Applies to

all-editors, diagnostic, analyser, tcl-lsp-cli, mcp

## Profiles

default

## Question

Why do I see an error saying "nesting depth exceeds the analysis limit", and why do diagnostics stop appearing partway through a deeply nested block?

## Why

The analyser walks nested `if` / `while` / `foreach` / `switch` / `try` / `catch` / `dict for` bodies recursively, one level at a time. Recursion this deep costs real memory on the call stack, so the walk is capped at 256 nesting levels — comfortably past anything a person would write by hand, but reachable by deeply nested generated or minified Tcl (templated iRules are the most common real-world source). Past the cap the analyser stops descending: it reports **`E207`** at the point it stopped and keeps every diagnostic it already collected for the levels above, rather than analysing the rest of the file incorrectly or not at all. This mirrors `tclsh`'s own `interp recursionlimit`, which raises a catchable "too many nested evaluations (infinite loop?)" error at a similar boundary rather than running away.

## Symptoms

- A red squiggle with the message "nesting depth exceeds the analysis limit (256 levels) — diagnostics for this body and anything nested inside it are not collected", anchored on the body where the walk stopped.
- No other diagnostics appear for code nested inside that body, even where you would expect one (e.g. an obviously unreachable branch, or a call with the wrong argument count).

## Example that triggers it

Generated or minified Tcl that nests **more than 256** levels of control flow — this only happens with machine-emitted code; 256 levels is far beyond what anyone writes by hand:

```tcl
proc deepnest {} {
  if {1} {
    if {1} {
      if {1} {
        # ... 250+ more nested `if` levels ...
      }
    }
  }
}
```

The analyser reports **`E207`** on the body at nesting level 257, where it stopped descending.

## Fix

There is nothing to fix in the sense of a bug — this is expected behaviour on input nested far past any hand-written shape. If the diagnostic is unwelcome noise on machine-generated input you don't intend to hand-edit, treat it the same as any other file you don't want analysed (exclude it from the workspace, or add it to your editor's per-language ignore list). If you believe the *generator* itself is unintentionally producing pathologically deep nesting, that is worth fixing at the source — flattening the generated structure will also make the file smaller and faster to load.

## How to suppress

`E207` cannot be suppressed with a `# noqa` comment — it fires before the analyser reaches the per-command comment scan that `# noqa` relies on. Disable it workspace-wide instead (`tclLsp.diagnostics.disabled` in your editor settings, or `--disable E207` on the `tcl` CLI) if you don't want it reported at all.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- Related codes: `E200`, `E201`, `E202`, `E203`
