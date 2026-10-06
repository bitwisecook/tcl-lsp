# Evaluated body reference fixtures

`body_execution_conformance.rs` runs these fixed scripts through
`tcl-test-support` discovery and isolated source-file execution. It pins both
C Tcl source patchlevel and the independently selected tcltest package version.

| Core | Package | Observations |
| --- | --- | --- |
| 8.4.20 | 2.2.11 | 64 |
| 8.5.19 | 2.3.8 | 64 |
| 8.6.18 | 2.5.11 | 64 |
| 9.0.4 | 2.5.11 | 64 |
| 9.1.0 | 2.6.0 | 69 |

The 160 C observations cover lifecycle option order and duplicates, setup/body/
cleanup errors and returns, skip paths, caller visibility, legacy and one-list
arguments, constraints, output capture, hooks, nested catch and frame selection,
iteration, matcher ordering, and hidden private-variable observers. Earlier
packages' absent iteration support is asserted explicitly; the Option(-iterations)
observer case belongs to the package that actually implements that option.

Five variable-link cases per C release add 25 observations of alias retargeting,
missing-element materialisation, writes after array deletion, and self cycles.
Together these lifecycle and variable-link cases contribute 185 observations.

The current upstream Jim reference has 43 independent observations. Its tcltest
package advertises 1.0 and defines global `test`; it runs the body after a setup
error. It does not supply C's `::tcltest::test` command, and its variable-trace
operation is absent. Capability expectations are checked against measured
`tcl-test-support::JimCapability` results. These cases never select C's captured
lifecycle implementation contract for Jim.
Five of these observations cover its name-following variable-link policy.
The lifecycle and variable-link subset contributes 203 observations across both engine families.

Expected files retain literal interpreter output, with per-package changes
expressed as separate files. Test reports go into the isolated fixture root,
never a repository working directory. A source or capability update must change
and remeasure expectations rather than silently omit unsupported operations.

Run `scripts/dev/run-resolution-oracles.sh` for the strict full reference suite.

`return-routes.tcl` adds eighteen fixed observations per engine for pending
return levels, the legacy 8.4 option grammar, custom modern C options, Jim's
retained `-code return`, and raw versus configured loop completions across
two procedure boundaries. The complete corpus now contains 381 C observations
(71/71/78/78/83) and 43 independent Jim observations, 424 in total.

Seven native numeric-selector observations per engine distinguish C 8.x signed
UINT_MAX magnitudes, the stricter C 9.x lower bound, and Jim wide-to-int
conversion. Past-wide Jim literals retain explicit unknown static routes while
the reference fixture records their actual interpreter completion.

Ten native variable-completion observations per C release establish the authored
`set`/`incr`/`global`/`upvar` OK-or-error domains. Non-OK variable-trace callbacks
are normalised to error; native error alternatives stay abrupt across procedure
boundaries. Tcl 9's removed legacy trace grammar uses its documented replacement,
while 8.4 exercises the legacy operation it actually supplies.
