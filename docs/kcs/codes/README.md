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

- [W001 — unknown subcommand](kcs-diagnostic-w001-unknown-subcommand.md)
- [W002 — command disabled in dialect](kcs-diagnostic-w002-command-disabled-in-dialect.md)
- [W100 — unbraced expression](kcs-diagnostic-w100-unbraced-expression.md)
- [W104 — string concat for lists](kcs-diagnostic-w104-string-concat-for-lists.md)
- [W105 — unbraced code block](kcs-diagnostic-w105-unbraced-code-block.md)
- [W106 — unbraced switch body](kcs-diagnostic-w106-unbraced-switch-body.md)
- [W108 — non-ASCII characters](kcs-diagnostic-w108-non-ascii-characters.md)
- [W110 — use eq/ne for strings](kcs-diagnostic-w110-use-eq-ne-for-strings.md)
- [W111 — line too long](kcs-diagnostic-w111-line-too-long.md)
- [W112 — trailing whitespace](kcs-diagnostic-w112-trailing-whitespace.md)
- [W113 — proc shadows built-in](kcs-diagnostic-w113-proc-shadows-builtin.md)
- [W114 — redundant nested expr](kcs-diagnostic-w114-redundant-nested-expr.md)
- [W115 — backslash-newline in comment](kcs-diagnostic-w115-backslash-newline-in-comment.md)
- [W116 — stub shadows built-in](kcs-diagnostic-w116-stub-shadows-builtin.md)
- [W117 — stub expr shadows built-in](kcs-diagnostic-w117-stub-expr-shadows-builtin.md)
- [W118 — inconsistent line endings](kcs-diagnostic-w118-inconsistent-line-endings.md)
- [W120 — missing package require](kcs-diagnostic-w120-missing-package-require.md)
- [W121 — invalid subnet mask](kcs-diagnostic-w121-invalid-subnet-mask.md)
- [W122 — mistyped IPv4 address](kcs-diagnostic-w122-mistyped-ipv4-address.md)
- [W124 — invalid IP literal](kcs-diagnostic-w124-invalid-ip-literal.md)
- [W125 — orphaned control flow](kcs-diagnostic-w125-orphaned-control-flow.md)
- [W126 — non-channel argument](kcs-diagnostic-w126-non-channel-argument.md)
- [W200 — exec result not captured](kcs-diagnostic-w200-exec-result-not-captured.md)
- [W201 — manual path concatenation](kcs-diagnostic-w201-manual-path-concatenation.md)

## Security (W1xx, W3xx)

- [W101 — eval string concatenation](kcs-diagnostic-w101-eval-string-concatenation.md)
- [W102 — subst on variable](kcs-diagnostic-w102-subst-on-variable.md)
- [W103 — open pipeline](kcs-diagnostic-w103-open-pipeline.md)
- [W123 — unresolved command (opt-in)](kcs-diagnostic-w123-unresolved-command.md)
- [W300 — source with variable](kcs-diagnostic-w300-source-with-variable.md)
- [W301 — uplevel string script](kcs-diagnostic-w301-uplevel-string-script.md)
- [W302 — catch without result](kcs-diagnostic-w302-catch-without-result.md)
- [W303 — regexp ReDoS](kcs-diagnostic-w303-regexp-redos.md)
- [W304 — missing option terminator](kcs-diagnostic-w304-missing-option-terminator.md)
- [W306 — substitution in literal position](kcs-diagnostic-w306-substitution-in-literal-position.md)
- [W307 — non-literal command](kcs-diagnostic-w307-non-literal-command.md)
- [W308 — subst without -nocommands](kcs-diagnostic-w308-subst-without-nocommands.md)
- [W309 — eval with subst](kcs-diagnostic-w309-eval-with-subst.md)
- [W313 — destructive file variable path](kcs-diagnostic-w313-destructive-file-variable-path.md)

## Variables (W2xx)

- [W210 — variable read before set](kcs-diagnostic-w210-variable-read-before-set.md)
- [W211 — variable set not used](kcs-diagnostic-w211-variable-set-not-used.md)
- [W212 — variable substitution where name expected](kcs-diagnostic-w212-variable-substitution-where-name-expected.md)
- [W213 — variable may not exist](kcs-diagnostic-w213-variable-may-not-exist.md)
- [W214 — unused proc parameter](kcs-diagnostic-w214-unused-proc-parameter.md)
- [W220 — dead store](kcs-diagnostic-w220-dead-store.md)

## Shimmer (S-codes)

- [S100 — shimmer outside loop](kcs-diagnostic-s100-shimmer-outside-loop.md)
- [S101 — shimmer inside loop](kcs-diagnostic-s101-shimmer-inside-loop.md)
- [S102 — shimmer oscillation](kcs-diagnostic-s102-shimmer-oscillation.md)

## Taint (T-codes)

- [T100 — taint code execution sink](kcs-diagnostic-t100-taint-code-execution-sink.md)
- [T101 — taint output sink](kcs-diagnostic-t101-taint-output-sink.md)
- [T102 — taint option injection](kcs-diagnostic-t102-taint-option-injection.md)

## iRule security (IRULE3xxx)

- [IRULE3001 — taint HTTP response body](kcs-diagnostic-irule3001-taint-http-response-body.md)
- [IRULE3002 — taint HTTP header](kcs-diagnostic-irule3002-taint-http-header.md)

*3 more codes (IRULE3003, IRULE3101, IRULE3102) to be added in Phase 5.2.*

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
