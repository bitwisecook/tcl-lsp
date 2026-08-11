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

This index lists every per-code KCS page.  The original phased plan
that set out the coverage targets is archived at
[`docs/archive/kcs-completeness-plan-2026.md`](../../archive/kcs-completeness-plan-2026.md)
for historical reference.

## Errors (E-codes)

- [E001 — missing subcommand](kcs-diagnostic-e001-missing-subcommand.md)
- [E002 — too few arguments](kcs-diagnostic-e002-too-few-arguments.md)
- [E003 — too many arguments](kcs-diagnostic-e003-too-many-arguments.md)
- [E004 — invalid argument count](kcs-diagnostic-e004-invalid-argument-count.md)
- [E005 — wrong argument-count shape](kcs-diagnostic-e005-wrong-argument-count-shape.md)
- [E006 — invalid formal-parameter list](kcs-diagnostic-e006-invalid-formal-parameter-list.md)
- [E100 — unmatched close bracket](kcs-diagnostic-e100-unmatched-close-bracket.md)
- [E101 — missing open brace](kcs-diagnostic-e101-missing-open-brace.md)
- [E102 — unmatched close brace](kcs-diagnostic-e102-unmatched-close-brace.md)
- [E103 — stolen close brace](kcs-diagnostic-e103-stolen-close-brace.md)
- [E200 — parse error](kcs-diagnostic-e200-parse-error.md)
- [E201 — unclosed bracket](kcs-diagnostic-e201-unclosed-bracket.md)
- [E202 — unclosed quote](kcs-diagnostic-e202-unclosed-quote.md)
- [E203 — unclosed brace](kcs-diagnostic-e203-unclosed-brace.md)
- [E207 — nesting depth exceeds limit](kcs-diagnostic-e207-nesting-depth-exceeds-limit.md)

## Warnings and style (W-codes)

- [W001 — unknown subcommand](kcs-diagnostic-w001-unknown-subcommand.md)
- [W002 — command disabled in dialect](kcs-diagnostic-w002-command-disabled-in-dialect.md)
- [W003 — dialect-invalid expression operator](kcs-diagnostic-w003-dialect-invalid-expr-operator.md)
- [W004 — dialect-invalid command option](kcs-diagnostic-w004-dialect-invalid-option.md)
- [W100 — unbraced expression](kcs-diagnostic-w100-unbraced-expression.md)
- [W104 — string concat for lists](kcs-diagnostic-w104-string-concat-for-lists.md)
- [W105 — unbraced code block](kcs-diagnostic-w105-unbraced-code-block.md)
- [W106 — unbraced switch body](kcs-diagnostic-w106-unbraced-switch-body.md)
- [W107 — source is not valid UTF-8](kcs-diagnostic-w107-source-not-valid-utf8.md)
- [W108 — non-ASCII characters](kcs-diagnostic-w108-non-ascii-characters.md)
- [W109 — source is not UTF-8 text](kcs-diagnostic-w109-source-not-utf8-text.md)
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
- [W124 — invalid IP literal](kcs-diagnostic-w124-invalid-ip-literal.md)
- [W125 — orphaned control flow](kcs-diagnostic-w125-orphaned-control-flow.md)
- [W126 — non-channel argument](kcs-diagnostic-w126-non-channel-argument.md)
- [W127 — value not in allowed set](kcs-diagnostic-w127-value-not-in-allowed-set.md)
- [W128 — renamed or deleted command call](kcs-diagnostic-w128-renamed-command-call.md)
- [W129 — command hidden in a safe interpreter](kcs-diagnostic-w129-command-hidden-in-safe-interpreter.md)
- [W130 — package not in lockfile](kcs-diagnostic-w130-package-not-in-lockfile.md)
- [W131 — lockfile out of sync](kcs-diagnostic-w131-lockfile-out-of-sync.md)
- [W132 — integrity mismatch](kcs-diagnostic-w132-integrity-mismatch.md)
- [W133 — safe-mode violation](kcs-diagnostic-w133-safe-mode-violation.md)
- [W134 — missing pkgIndex.tcl](kcs-diagnostic-w134-missing-pkgindex.md)
- [W135 — command needs newer package version](kcs-diagnostic-w135-command-needs-newer-package.md)
- [W136 — option needs newer package version](kcs-diagnostic-w136-option-needs-newer-package.md)
- [W140 — interpreter never created in this file](kcs-diagnostic-w140-interpreter-never-created.md)
- [W143 — private Tcl implementation namespace](kcs-diagnostic-w143-private-tcl-namespace.md)
- [W144 — deprecated at the resolved version](kcs-diagnostic-w144-deprecated-at-resolved-version.md)
- [W145 — ambiguous keyword abbreviation](kcs-diagnostic-w145-ambiguous-abbreviation.md)
- [W146 — invalid literal argument members](kcs-diagnostic-w146-invalid-literal-argument-members.md)
- [W147 — mutually exclusive options](kcs-diagnostic-w147-mutually-exclusive-options.md)
- [W200 — exec result not captured](kcs-diagnostic-w200-exec-result-not-captured.md)
- [W201 — manual path concatenation](kcs-diagnostic-w201-manual-path-concatenation.md)
- [W230 — list index out of range](kcs-diagnostic-w230-list-index-out-of-range.md)
- [W231 — lset index out of range](kcs-diagnostic-w231-lset-index-out-of-range.md)
- [W232 — string index out of range](kcs-diagnostic-w232-string-index-out-of-range.md)
- [W233 — divide or modulo by zero](kcs-diagnostic-w233-divide-by-zero.md)
- [W240 — loop condition is constant false](kcs-diagnostic-w240-loop-constant-false.md)
- [W241 — loop is provably infinite](kcs-diagnostic-w241-loop-provably-infinite.md)
- [W242 — loop termination not provable (opt-in)](kcs-diagnostic-w242-loop-termination-unprovable.md)
- [W250 — instantiating an abstract class](kcs-diagnostic-w250-abstract-instantiation.md)

## Security (W1xx, W3xx)

- [W101 — eval string concatenation](kcs-diagnostic-w101-eval-string-concatenation.md)
- [W102 — subst on variable](kcs-diagnostic-w102-subst-on-variable.md)
- [W103 — open pipeline](kcs-diagnostic-w103-open-pipeline.md)
- [W123 — unresolved command](kcs-diagnostic-w123-unresolved-command.md)
- [W300 — source with variable](kcs-diagnostic-w300-source-with-variable.md)
- [W301 — uplevel string script](kcs-diagnostic-w301-uplevel-string-script.md)
- [W302 — catch without result](kcs-diagnostic-w302-catch-without-result.md)
- [W303 — regexp ReDoS](kcs-diagnostic-w303-regexp-redos.md)
- [W304 — missing option terminator](kcs-diagnostic-w304-missing-option-terminator.md)
- [W305 — bidirectional control character](kcs-diagnostic-w305-bidi-control-character.md)
- [W306 — substitution in literal position](kcs-diagnostic-w306-substitution-in-literal-position.md)
- [W307 — non-literal command](kcs-diagnostic-w307-non-literal-command.md)
- [W308 — unknown TclOO method](kcs-diagnostic-w308-unknown-tcloo-method.md)
- [W309 — eval with subst](kcs-diagnostic-w309-eval-with-subst.md)
- [W313 — destructive file variable path](kcs-diagnostic-w313-destructive-file-variable-path.md)
- [W314 — no absolute name](kcs-diagnostic-w314-no-absolute-name.md)
- [W315 — class definition cannot run](kcs-diagnostic-w315-class-definition-cannot-run.md)

## Variables (W2xx)

- [W210 — variable read before set](kcs-diagnostic-w210-variable-read-before-set.md)
- [W211 — variable set not used](kcs-diagnostic-w211-variable-set-not-used.md)
- [W212 — variable substitution where name expected](kcs-diagnostic-w212-variable-substitution-where-name-expected.md)
- [W213 — variable may not exist](kcs-diagnostic-w213-variable-may-not-exist.md)
- [W214 — unused proc parameter](kcs-diagnostic-w214-unused-proc-parameter.md)
- [W215 — variable name unreachable via $-substitution](kcs-diagnostic-w215-variable-name-unreachable-via-substitution.md)
- [W216 — broken brace-form array element reference](kcs-diagnostic-w216-broken-brace-array-element-reference.md)
- [W217 — unset unsets nothing](kcs-diagnostic-w217-unset-unsets-nothing.md)
- [W218 — args in a non-final parameter position](kcs-diagnostic-w218-args-not-final-parameter.md)
- [W220 — dead store](kcs-diagnostic-w220-dead-store.md)

## Hints (H-codes)

- [H300 — repeated assignment to same variable with same value](kcs-diagnostic-h300-repeated-assignment-same-value.md)

## Information (I-codes)

- [I230 — constant existence check / unreachable branch](kcs-diagnostic-i230-constant-existence-check.md)
- [I231 — constant switch arm / unreachable case](kcs-diagnostic-i231-constant-switch-arm.md)

## Shimmer (S-codes)

- [S100 — shimmer outside loop](kcs-diagnostic-s100-shimmer-outside-loop.md)
- [S101 — shimmer inside loop](kcs-diagnostic-s101-shimmer-inside-loop.md)
- [S102 — shimmer oscillation](kcs-diagnostic-s102-shimmer-oscillation.md)
- [S103 — shared-value copy-on-write](kcs-diagnostic-s103-shared-value-copy.md)

## Taint (T-codes)

- [T100 — taint code execution sink](kcs-diagnostic-t100-taint-code-execution-sink.md)
- [T101 — taint output sink](kcs-diagnostic-t101-taint-output-sink.md)
- [T102 — taint option injection](kcs-diagnostic-t102-taint-option-injection.md)

## iRule security (IRULE3xxx)

- [IRULE3001 — taint HTTP response body](kcs-diagnostic-irule3001-taint-http-response-body.md)
- [IRULE3002 — taint HTTP header](kcs-diagnostic-irule3002-taint-http-header.md)
- [IRULE3003 — taint log injection](kcs-diagnostic-irule3003-taint-log-injection.md)
- [IRULE3101 — URI/path without leading slash](kcs-diagnostic-irule3101-uri-path-without-leading-slash.md)
- [IRULE3102 — HTTP getter without -normalized](kcs-diagnostic-irule3102-http-getter-without-normalized.md)

## iRule events and commands (IRULE1xxx, IRULE2xxx)

- [IRULE1001 — command invalid in event](kcs-diagnostic-irule1001-command-invalid-in-event.md)
- [IRULE1002 — unknown event](kcs-diagnostic-irule1002-unknown-event.md)
- [IRULE1003 — deprecated event](kcs-diagnostic-irule1003-deprecated-event.md)
- [IRULE1004 — missing priority](kcs-diagnostic-irule1004-missing-priority.md)
- [IRULE1005 — data event without collect](kcs-diagnostic-irule1005-data-event-without-collect.md)
- [IRULE1006 — payload without collect](kcs-diagnostic-irule1006-payload-without-collect.md)
- [IRULE1007 — collect without release](kcs-diagnostic-irule1007-collect-without-release.md)
- [IRULE1008 — release without collect](kcs-diagnostic-irule1008-release-without-collect.md)
- [IRULE1201 — HTTP command after respond](kcs-diagnostic-irule1201-http-command-after-respond.md)
- [IRULE1202 — multiple respond/redirect](kcs-diagnostic-irule1202-multiple-respond-redirect.md)
- [IRULE2001 — deprecated matchclass](kcs-diagnostic-irule2001-deprecated-matchclass.md)
- [IRULE2002 — deprecated command](kcs-diagnostic-irule2002-deprecated-command.md)
- [IRULE2003 — unsafe command](kcs-diagnostic-irule2003-unsafe-command.md)
- [IRULE2101 — heavy regexp in hot event](kcs-diagnostic-irule2101-heavy-regexp-in-hot-event.md)

## iRule variables (IRULE4xxx)

- [IRULE4001 — static write outside RULE_INIT](kcs-diagnostic-irule4001-static-write-outside-rule-init.md)
- [IRULE4002 — generic static name](kcs-diagnostic-irule4002-generic-static-name.md)
- [IRULE4003 — variable scope across events](kcs-diagnostic-irule4003-variable-scope-across-events.md)
- [IRULE4004 — hoistable constant set](kcs-diagnostic-irule4004-hoistable-constant-set.md)
- [IRULE4005 — static variable race](kcs-diagnostic-irule4005-static-variable-race.md)

## iRule flow (IRULE5xxx)

- [IRULE5001 — ungated log in hot event](kcs-diagnostic-irule5001-ungated-log-in-hot-event.md)
- [IRULE5002 — drop without event disable](kcs-diagnostic-irule5002-drop-without-event-disable.md)
- [IRULE5004 — DNS::return without return](kcs-diagnostic-irule5004-dns-return-without-return.md)
- [IRULE5005 — direct proc without call](kcs-diagnostic-irule5005-direct-proc-without-call.md)
- [IRULE5006 — top-level in nested body](kcs-diagnostic-irule5006-top-level-in-nested-body.md)
- [IRULE5007 — event command outside when](kcs-diagnostic-irule5007-event-command-outside-when.md)

## Optimisations (O-codes)

### Constant folding and propagation

- [O100 — constant propagation](kcs-optimisation-o100-constant-propagation.md)
- [O101 — integer expression folding](kcs-optimisation-o101-integer-expression-folding.md)
- [O102 — load forwarding](kcs-optimisation-o102-load-forwarding.md)
- [O103 — static proc folding](kcs-optimisation-o103-static-proc-folding.md)
- [O104 — string build chain folding](kcs-optimisation-o104-string-build-chain-folding.md)
- [O105 — constant var-ref propagation / GVN/CSE](kcs-optimisation-o105-constant-var-ref-propagation.md)
- [O129 — builtin command substitution folding](kcs-optimisation-o129-builtin-command-substitution-folding.md)

### Code motion and dead-code elimination

- [O106 — loop-invariant code motion](kcs-optimisation-o106-loop-invariant-code-motion.md)
- [O107 — unreachable dead code](kcs-optimisation-o107-unreachable-dead-code.md)
- [O108 — transitive dead code](kcs-optimisation-o108-transitive-dead-code.md)
- [O109 — dead store](kcs-optimisation-o109-dead-store.md)
- [O110 — expression canonicalisation](kcs-optimisation-o110-expression-canonicalisation.md)
- [O111 — brace expression hints](kcs-optimisation-o111-brace-expression-hints.md)
- [O112 — constant-condition elimination](kcs-optimisation-o112-constant-condition-elimination.md)

### Pattern recognition and readability

- [O113 — strength reduction](kcs-optimisation-o113-strength-reduction.md)
- [O114 — incr idiom](kcs-optimisation-o114-incr-idiom.md)
- [O115 — redundant nested expr](kcs-optimisation-o115-redundant-nested-expr.md)
- [O116 — list literal folding](kcs-optimisation-o116-list-literal-folding.md)
- [O117 — string length simplification](kcs-optimisation-o117-string-length-simplification.md)
- [O118 — lindex folding](kcs-optimisation-o118-lindex-folding.md)
- [O119 — multi-set packing](kcs-optimisation-o119-multi-set-packing.md)
- [O120 — eq/ne for strings](kcs-optimisation-o120-eq-ne-for-strings.md)

### Recursion, sinking, and elimination

- [O121 — tailcall rewrite](kcs-optimisation-o121-tailcall-rewrite.md)
- [O122 — tail-recursion to while](kcs-optimisation-o122-tail-recursion-to-while.md)
- [O123 — accumulator hint](kcs-optimisation-o123-accumulator-hint.md)
- [O124 — unused iRule procs](kcs-optimisation-o124-unused-irule-procs.md)
- [O125 — code sinking](kcs-optimisation-o125-code-sinking.md)
- [O126 — unused variable removal](kcs-optimisation-o126-unused-variable-removal.md)
- [O127 — single-use inline](kcs-optimisation-o127-single-use-inline.md)
- [O128 — end-offset index rewrite](kcs-optimisation-o128-end-offset-index.md)
- [O130 — lappend list build chain folding](kcs-optimisation-o130-lappend-list-build-chain-folding.md)

## Internal codes

Some taint propagation codes (`T103`, `T106`) are internal and never
surface to users — they exist so the propagation engine can emit
structured records the analyser later resolves into a T100/T101/T102
finding. These codes do not get their own page; see the
[taint analysis glossary entry](../../GLOSSARY.md#taint-analysis) for
the data-flow model.
