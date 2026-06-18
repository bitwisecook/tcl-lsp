# KCS: Code lens says "0 references" but the proc is used

> **Audience:** User
> **Type:** Issue

## Applies to

VS Code

## Question

Why does the reference-count code lens above a proc read "0 references"
when the same proc is clearly called, and clicking the lens (or running
**Find All References**) lists the call site correctly?

## Symptoms

- The lens above a proc definition reads `0 references`.
- Clicking the lens, or running **Find All References** on the proc name,
  opens a peek that lists one or more real call sites.
- The two views disagree for the same proc: the count says zero, the
  reference list does not.
- Most reproducible when the proc is **called before it is defined**
  (a forward reference), or is called from a **different file**.

## Answer

Upgrade to a build that includes the fix for this issue, then reload the
window:

1. Update the **Tcl LSP** extension to the latest version.
2. Run **Developer: Reload Window** from the Command Palette.
3. Hover the proc definition — the lens count now matches the number of
   entries shown when you click it.

The success signal is that the lens count and the peek list always agree.

### What was happening

The count and the reference list came from two separate code paths. The
count was taken from a workspace tally keyed by each call's *resolved*
proc name. A call written before the proc's definition — or in another
file — has no resolved name at analysis time, so it was missing from the
tally even though the reference resolver still matched it by name. The
count now derives from the same resolution that backs the peek, so the two
can no longer drift.

If the count and the reference list still disagree after updating, collect
the **Tcl LSP** output channel log and open an issue.

## Related

- [KCS index](README.md)
- [Glossary](../GLOSSARY.md)
- [kcs-issue-lsp-features-are-missing.md](kcs-issue-lsp-features-are-missing.md)
