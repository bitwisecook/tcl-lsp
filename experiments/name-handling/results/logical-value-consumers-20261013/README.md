Value-consumer implementation and measured controls
===================================================

The patch touches four analyser owners only and applies cleanly to Main when
checked. It migrates the audited dynamic-apply, defined-symbol/constant-word,
namespace constant lookup and DictWith lookup sites to one Logical value
selector. The old recursive namespace-constant walker is removed. Exact
source/input/declaration/value provenance is private and survives snapshots.
Formatting and whitespace checks pass. The ten exact Rust selectors in
`test-selectors.json` are UNEXECUTED; no build or Rust result is claimed.

The original C8.4.20, C8.5.19, C8.6.18, C9.0.4, C9.1.0 and latest Jim
0.84-9-g5bac7c9 binaries executed `oracles/probe.tcl`. Every final invocation
completed with exit 0 and unchanged before/after executable and source-library
hash inventories. No Tcltest helper was used. C library directories are
version-matching originals. Jim's binary and checkout Tcl sources are sealed;
these are source-library inventories, not a claim about every OS dependency.

The final probe explicitly publishes namespace variables. All C versions find
`child::x` in ::N::child; Jim reports a missing relative variable although
`namespace current` returns ::N::child and the rooted variable exists. All
providers return LOCAL for uplevel 0, ROOT for uplevel #0 and NAMESPACE for the
actual caller. Apply and dict are unavailable on C8.4; the other providers
execute the lambda and dictionary body, while a procedure's unlinked `d`
lookup fails. All providers distinguish literal array element `$which` from
the substituted `live` element and preserve `${scalar(open}` as a scalar.

The initial uncaught relative query stops and the subsequent bare-set/fallback
probe are retained in separate directories. Bare namespace `set x` with an
existing root x updates the root on C8.4–8.6, whereas C9 creates the namespace
cell. That publication difference must not be described as evidence of a C
relative-variable lookup difference. Jim's distinct relative lookup is
measured only after explicit publication and rooted existence checks.

The Logical-only Rust tests deliberately claim no Native or Jim value admission.
Their expected values are readonly source advice. They test actual source
frames, qualified declaration geometry, stale images, lexer configuration,
foreign stores, missing inputs, snapshots, altered value maps, links, arrays,
method namespaces and non-dominating/future writes.

Remaining boundaries
--------------------

DictWith still records computed key variable metadata with its existing
argument token. That token is not an editable original key occurrence. Precise
key editing requires actual source-list-element provenance and a distinct
computed-key declaration issuer; this patch supplies no such permission.

The path-only generic constant readers remain for dominating interpolation,
regex metadata and interpreter-path metadata. Their purpose-specific value
adoption is separate from the audited four sites. This patch does not certify
all value consumers or all Native value handling.

`resolve_expansion_count` has zero production callers and six private tests.
It is dead production API, not a migrated consumer. Its removal and the
implementation-mirroring tests should be handled as cleanup, without claiming
new conformance coverage.
