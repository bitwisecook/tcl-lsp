# KCS: Sticky scroll shows nothing in VS Code

> **Audience:** User
> **Type:** Issue

## Applies to

VS Code

## Question

Sticky scroll pins nothing at the top of the editor for my Tcl files
while the extension is enabled — it worked before I installed the
extension — how do I get it back?

## Symptoms

- Scrolling inside a proc, class, namespace, or `when` handler pins no
  header line at the top of the editor, at any scroll position.
- Disabling the extension (and reloading) makes sticky scroll work
  again via VS Code's indentation heuristic.
- Everything else looks healthy: breadcrumbs show the enclosing proc or
  class, the Outline view is populated, and diagnostics and hovers work.

## Answer

Sticky scroll for Tcl files uses the extension's folding ranges (the
extension sets `editor.stickyScroll.defaultModel` to
`foldingProviderModel` for the Tcl languages), so anything that leaves
the folding provider without data blanks sticky scroll while the rest
of the extension keeps working.

1. Update the extension to 2.1.16 or later. Older versions could answer
   a folding request with an empty list — for example when
   `editor.folding` was switched off — and VS Code treats an empty
   folding answer as final: sticky scroll shows nothing rather than
   falling back to its indentation heuristic.
2. Check you have not set `tclLsp.features.folding` to `false`
   (**File > Preferences > Settings**, search for `tclLsp folding`).
   That switch turns off the server's folding ranges entirely; sticky
   scroll then falls back to VS Code's indentation model, which pins on
   indent depth rather than real block boundaries. Leave it unset (or
   `true`) for brace-accurate sticky scroll. `editor.folding` only
   hides the fold arrows in the gutter — it no longer affects sticky
   scroll.
3. Open the Output panel (**View > Output**), select **Tcl Language
   Server**, and look for a line mentioning
   `editor.stickyScroll.defaultModel`. The extension logs it when
   another installed extension has caused VS Code to drop our
   sticky-scroll default for Tcl. If you see it, add the default back
   yourself in your settings JSON:

   ```json
   "[tcl]": { "editor.stickyScroll.defaultModel": "foldingProviderModel" }
   ```

4. Success signal: scroll into the middle of any multi-line proc or
   class body — its header line stays pinned at the top of the editor.

If the steps above do not fix the problem, collect the output channel log
and open an issue.

## Related

- [KCS index](README.md)
- [Glossary](../GLOSSARY.md)
- [feature — Folding](features/kcs-feature-folding.md)
- [LSP features are missing in VS Code](kcs-issue-lsp-features-are-missing.md)
