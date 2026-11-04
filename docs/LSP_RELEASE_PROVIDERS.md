# LSP provider release issue evidence

This page records the source and test evidence for the provider issues in the
release sweep. The linked issue descriptions are historical reports; the status
below refers to the current `work` branch.

| Issue | Status and evidence |
| --- | --- |
| [#2265](https://github.com/bitwisecook/tcl-lsp/issues/2265) | Fixed before this sweep. `collect_expr` uses the lexer's `expression_term` range to descend into a nested command. `nested_command_head_substitution_in_an_expression_keeps_word_extents_2265` passed and checks the exact `length` and `abcdef` spans in the reported source, with no token overlap. |
| [#2497](https://github.com/bitwisecook/tcl-lsp/issues/2497) | Fixed before this sweep. `lens_counts_superclass_my_dispatch_to_a_subclass_method_2497` passed and asserts one reference for `YAxis::CalculateAxisPosition` from `BaseAxis::preparePosition`; this sweep also checks the Find References result against that count. The shared method-reference result is also used by Rename. The call returns `reached` in Tcl 8.6, 9.0 and 9.1. |
| [#2143](https://github.com/bitwisecook/tcl-lsp/issues/2143) | Under verification. Existing tests checked the W214 line and absence of W123 for `port`; this sweep adds the exact factory-call range and a W123 control with that call removed. The reported fixture prints `8080` under Tcl 8.4–9.1 and JimTcl, while the no-call control raises `invalid command name "port"` under the same matrix. The strengthened Rust assertions await the shared compiler build slot. |
| [#2254](https://github.com/bitwisecook/tcl-lsp/issues/2254) | Fixed in this sweep. Removed dead, test-only whitespace recognisers and their `#[allow(dead_code)]` attributes from `hover.rs`. Existing production hover tests cover clock, sprintf, binary, regsub, glob and regex; added quoted format, binary scan and unrelated-quoted-word cases through the production entry point. All added tests passed. |
| [#1956](https://github.com/bitwisecook/tcl-lsp/issues/1956) | Open. The fixed Tcllib 2.0 source creates `::clay::define` in `modules/clay/clay.tcl` through `::clay::dialect::create`; `modules/httpd/httpd.tcl` uses it to declare `::httpd::server` and `method connect`. Loading Tcllib 2.0 under Tcl 9.0 returned Clay 0.8.8 and HTTPD 4.3.6; `info class methods ::httpd::server -all` includes `connect`. The registry has no Clay definition grammar, so the analyser records no class or methods for this form. A fix needs a registry-owned member grammar and a definer-family path that preserves source identity. |
| [#1655](https://github.com/bitwisecook/tcl-lsp/issues/1655) | Open. The current Tcllib corpus test documents two abstention limits in the `::clay::dialect::create` prefix: conditional `return` through the live command table and `eval $ninja` without caller argument binding. Under Tcl 9.0 with exact Tcllib 2.0, `::clay::class` exists, inherits `::clay::dialect::MotherOfAllMetaClasses`, and answers `create` and `new`. Lifting the limits requires source-derived completion and argument-binding proofs; a command-name shortcut would not establish the factory's execution. |

Fresh validation on `0f847a7ca991` passed the exact #2265 token-span test and
the #2497 lens/reference-agreement test. The #2265 nested-closer and #2497
unrelated-class callback controls also passed. Two other inherited-method lens
tests fail their old exact-title assertions because current partial coverage
labels are `at least 1 reference` and `at least 2 references`; neither is the
#2497 fixture, whose title is `1 reference`. This is an open test-expectation
gap, so the whole inherited-lens suite is not claimed green.

The broader `hover::tests` run had 150 passes and 30 failures in existing
production assertions; the failure log is `/tmp/lsp-providers-hover.log`. The
removed recognisers existed only under `#[cfg(test)]` and were never called by
production hover. The focused #2265 and #2497 regressions and the added #2254
tests passed. The strengthened #2143 Rust regressions and repository gates remain pending a
compiler build slot.
