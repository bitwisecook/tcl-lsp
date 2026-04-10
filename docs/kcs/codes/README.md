# KCS codes index

Every diagnostic, warning, security, taint, iRule, and optimisation
code the analyser and optimiser emit has a page in this directory.
The filename shape is:

```
kcs-diagnostic-<code>-<plain-words>.md
kcs-optimisation-<code>-<plain-words>.md
```

Each page follows the diagnostic template
([`../templates/kcs-template-diagnostic.md`](../templates/kcs-template-diagnostic.md))
or the optimisation template
([`../templates/kcs-template-optimisation.md`](../templates/kcs-template-optimisation.md)),
and tags the producing compiler pass so readers can jump to the
relevant entry in the [glossary](../../GLOSSARY.md) and from there to
the [compiler design docs](../../design/compiler/README.md).

The [diagnostics feature page](../features/kcs-feature-diagnostics.md)
is the user-facing entry point for the whole family; it links here
for per-code details.

This index is filled in as the KCS completeness work progresses. See
[`docs/design/kcs-completeness-plan.md`](../../design/kcs-completeness-plan.md)
for the phase plan. Phase 0 scaffolds the directory; Phases 4-6 fill
in the pages; Phase 7 cross-links.

## Errors (E-codes)

- [E001 — missing subcommand](kcs-diagnostic-e001-missing-subcommand.md)
- [E002 — too few arguments](kcs-diagnostic-e002-too-few-arguments.md)
- [E003 — too many arguments](kcs-diagnostic-e003-too-many-arguments.md)
- [E004 — invalid argument count](kcs-diagnostic-e004-invalid-argument-count.md)
- [E100 — unmatched close bracket](kcs-diagnostic-e100-unmatched-close-bracket.md)
- [E101 — missing open brace](kcs-diagnostic-e101-missing-open-brace.md)
- [E102 — unmatched close brace](kcs-diagnostic-e102-unmatched-close-brace.md)
- [E103 — stolen close brace](kcs-diagnostic-e103-stolen-close-brace.md)
- [E200 — parse error](kcs-diagnostic-e200-parse-error.md)
- [E201 — unclosed bracket](kcs-diagnostic-e201-unclosed-bracket.md)
- [E202 — unclosed quote](kcs-diagnostic-e202-unclosed-quote.md)
- [E203 — unclosed brace](kcs-diagnostic-e203-unclosed-brace.md)

## Warnings and style (W-codes)

*~20 codes — style, idiom, and correctness warnings. Populated in
Phase 4.2.*

## Security (W1xx, W3xx)

*13 codes — code and command injection, ReDoS, path traversal, and
other security-related warnings. Populated in Phase 4.3.*

## Variables (W2xx)

*6 codes — unused, read-before-set, dead-store, and related variable
warnings. Populated in Phase 4.4.*

## Shimmer (S-codes)

*3 codes — shimmer detection over the type lattice. Populated in
Phase 4.4.*

## Taint (T-codes)

*5 codes — source-to-sink taint propagation. Populated in Phase 4.4.*

## iRule events and commands (IRULE1xxx, IRULE2xxx)

*16 codes — iRule event-context checks, deprecated commands, and
event-flow errors. Populated in Phase 5.1.*

## iRule security (IRULE3xxx)

*5 codes — iRule-specific taint and HTTP normalisation warnings.
Populated in Phase 5.2.*

## iRule variables (IRULE4xxx)

*5 codes — `static::` variable scoping and race conditions.
Populated in Phase 5.3.*

## iRule flow (IRULE5xxx)

*6 codes — top-level and nested context, `drop`/`return`, and related
control-flow warnings. Populated in Phase 5.4.*

## Optimisations (O-codes)

*28 codes — rewrites performed by the optimiser, grouped by category
(constant folding, code motion, DCE, pattern, readability, recursion,
code sinking). Populated in Phases 6.1-6.4.*

## Internal codes

Some taint propagation codes (`T103`, `T106`) are internal and never
surface to users — they exist so the propagation engine can emit
structured records the analyser later resolves into a T100/T101/T102
finding. These codes do not get their own page; see the
[taint analysis glossary entry](../../GLOSSARY.md#taint-analysis) for
the data-flow model.
