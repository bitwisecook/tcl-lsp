# v2.1.8

**2.x alpha — pre-release channel.**

Another pre-release on the **2.x** line, where the ongoing Python → Rust
rewrite of tcl-lsp ships its alphas. It is opt-in: install it from the VS Code
Marketplace **pre-release** channel or the JetBrains Marketplace **eap**
channel, or download the pre-release VSIX / plugin / native binaries from this
GitHub release. The stable **1.x** line stays the default for everyone who has
not opted into pre-releases, and a `2.1.x` build never becomes the "latest"
GitHub release or the default Marketplace download.

This release is mostly about the **BIG-IP report generator**: the report now
carries the f5-query and tcl-lsp marks, and — more usefully — it *prints*. A
printed report is now a linear document with a running header and footer on
every page, one section per page, and the architecture view rendered properly
alongside the manifest that defined it.

Chasing the printing work turned up a bug that was never about printing: every
device node in the **architecture diagram was invisible on screen too**. See
**Bug Fixes**.

## New Features

- **The report carries the f5-query and tcl-lsp marks.** Both are inlined as
  real `<svg>`, so they follow the report's light/dark theme and stay crisp at
  any zoom rather than blurring like a raster logo. They lead the builder page
  beside the title — the f5-query mark linking to the query quick start, the
  tcl-lsp mark to the project — and sit above the attribution in the report
  footer. A report generated without a logo of your own now uses the f5-query
  mark in the header instead of a placeholder glyph; supplying your own logo
  still overrides it.
- **A report prints as a linear document.** Printing now walks every selected
  device and section in document order rather than whatever tab happened to be
  open, with each section starting on its own page. The architecture view gets
  a page of its own — the diagram, the devices-by-tier breakdown, and the
  manifest DSL that defined the estate, rendered as a code block rather than as
  the empty box an editable textarea prints as. The f5-query manual is left out
  of the printed copy, where a reference manual is only noise.

## Bug Fixes

- **The architecture diagram now draws its devices — on screen as well as in
  print.** The diagram draws each device as an SVG node carrying
  `class="device"`, and the report's own `.device { display: none }` rule — the
  one that hides the device sections you are not looking at — was hiding those
  nodes too. The diagram rendered as edges over blank space. The same selector
  collision made the print dialog offer phantom devices to print.
- **The printed report repeats its header and footer on every page.** The
  running title and the attribution / version / copyright line were positioned
  into the page margin, which browsers hand to the *neighbouring* page: the
  title printed at the foot of the page before it, and the attribution across
  the top of the page after it, on top of the content. Both now ride along on
  every sheet, clear of the content — so a confidentiality notice set on a
  report reaches every printed page, which is the point of setting one.
- **`f5report` wheels ship the report's logo assets.** The vendored logo SVGs
  were treated as build outputs and left out of the package, so a packaged
  install would raise on first render instead of producing a report.
- **BIG-IP object references inside `switch` arms are found.** An object
  referenced only from a `switch` arm could be invisible to the reference graph
  that `f5 grep`, `f5 irule trace`, `f5 query rename` and the `bigip-cleanup`
  skill all read — where a miss is a deleted live pool, not a wrong colour. A
  single exhaustive definition of "does this argument carry executable Tcl" now
  backs the walk: a new argument role that can hold a script fails the build
  until it is classified, and generative tests assert that such arguments are
  actually walked.

## Improvements

- **The APL and BIG-IP editor grammars match what the server sees.** Both
  dialects shipped with the *Tcl* grammar, which contains no BIG-IP and no APL
  rules, so a `bigip.conf` was painted with Tcl rules until the language server
  answered and replaced the highlighting wholesale. Each dialect now has its own
  tree-sitter grammar, with `ltm rule` bodies and APL `[ … ]` expressions
  highlighted as Tcl exactly as the server walks them.
- **The Zed grammars resolve.** Both new grammars were pinned to a commit on a
  branch that no longer exists, which Zed cannot fetch; they now name a commit
  on the release line.
