# naming.package.authored-index-written-head-projections

Kind: `implementation-contract`

## Problem statement

The authored package-index advisory scan receives written command heads. Removing pairs of leading colons can leave a spurious literal colon and miss selected registration, version-guard or termination metadata.

## Question

How does the authored package-index scan project complete written heads to Registry keys while retaining symbolic control conditions and independent original-source authority?

## Conclusion

The advisory scan uses canonical_written_command on the complete written head, then removes one optional root from that constructed key. Registry descriptors select registration, body and termination roles. Symbolic version guards retain their Tcl-package subject and requirements; uncertain or dynamic guards remain conditional. Literal single colons, relative qualifiers, trailing empty names and dynamic heads cannot select an unrelated stock command. This authored convention supplies no C/Jim lookup, actual package publication, loader execution, rewrite or Native authority. Analysis-backed index advice keeps its independent complete source, grammar, Registry, entry and selected-handler owner.

## Scope

The five neutral package-index head projections in scan_with_config and its existing authored control/guard helpers. Software controls exercise the actual scanner and its symbolic availability output. They measure no interpreter execution, native name conversion, command lookup or package registration. Original scan_with_analysis and its SourceStructure gates are independent and unchanged.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not applicable to this authored advisory projection. Build: No external provider process is used.. Channel: Fixed authored index source and software advisory scan.. Dialect: tcl8.4.

The scan asserts an authored written-name convention and symbolic Registry advice. It supplies no native lookup, callback chronology, loader execution or provider equivalence.

### tcl8.5

Status: `not-tested`. Version: not applicable to this authored advisory projection. Build: No external provider process is used.. Channel: Fixed authored index source and software advisory scan.. Dialect: tcl8.5.

The scan asserts an authored written-name convention and symbolic Registry advice. It supplies no native lookup, callback chronology, loader execution or provider equivalence.

### tcl8.6

Status: `not-tested`. Version: not applicable to this authored advisory projection. Build: No external provider process is used.. Channel: Fixed authored index source and software advisory scan.. Dialect: tcl8.6.

The scan asserts an authored written-name convention and symbolic Registry advice. It supplies no native lookup, callback chronology, loader execution or provider equivalence.

### tcl9.0

Status: `not-tested`. Version: not applicable to this authored advisory projection. Build: No external provider process is used.. Channel: Fixed authored index source and software advisory scan.. Dialect: tcl9.0.

The scan asserts an authored written-name convention and symbolic Registry advice. It supplies no native lookup, callback chronology, loader execution or provider equivalence.

### tcl9.1

Status: `not-tested`. Version: not applicable to this authored advisory projection. Build: No external provider process is used.. Channel: Fixed authored index source and software advisory scan.. Dialect: tcl9.1.

The scan asserts an authored written-name convention and symbolic Registry advice. It supplies no native lookup, callback chronology, loader execution or provider equivalence.

### jim

Status: `not-tested`. Version: not applicable to this authored advisory projection. Build: No external provider process is used.. Channel: Fixed authored index source and software advisory scan.. Dialect: jim.

The scan asserts an authored written-name convention and symbolic Registry advice. It supplies no native lookup, callback chronology, loader execution or provider equivalence.

### bigip

Status: `not-tested`. Version: not applicable to this authored advisory projection. Build: No external provider process is used.. Channel: Fixed authored index source and software advisory scan.. Dialect: bigip.

The scan asserts an authored written-name convention and symbolic Registry advice. It supplies no native lookup, callback chronology, loader execution or provider equivalence.

## Exact evidence

- `authored-index-source608-0` (implementation): [rust/tcl-lsp-core/src/package_resolver/reachability.rs](../../../../rust/tcl-lsp-core/src/package_resolver/reachability.rs). SHA-256 `3b007f580b998d60e76a5bb8c107f93ae5c650d0dd4c670272f460ff22265753`. Current authored neutral written-head projection or actual advisory scanner control. No Native lookup, package publication, loader execution or assertion outcome is attached.
- `authored-index-source608-1` (implementation): [rust/tcl-lsp-core/src/package_resolver/reachability/tests/written_head_tests.rs](../../../../rust/tcl-lsp-core/src/package_resolver/reachability/tests/written_head_tests.rs). SHA-256 `bcd6de2ef35808cd617344efddb62af8608ac6f980a59ee5f83b2e641133fb71`. Current authored neutral written-head projection or actual advisory scanner control. No Native lookup, package publication, loader execution or assertion outcome is attached.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-lsp-core/src/package_resolver/reachability.rs](../../../../rust/tcl-lsp-core/src/package_resolver/reachability.rs), `scan_with_config`: Retain the supplied whole source grammar and explicit authored advisory convention; original analysis-backed source advice is independent.
- [rust/tcl-lsp-core/src/package_resolver/registration.rs](../../../../rust/tcl-lsp-core/src/package_resolver/registration.rs), `select_in_plan`: Validate the exact complete original command vector and supplied grammar, then project its canonical written head before the selected Registry package registration/body-role query.
- [rust/tcl-lsp-core/src/package_resolver/reachability.rs](../../../../rust/tcl-lsp-core/src/package_resolver/reachability.rs), `ScriptWalker::walk_script`: Reuse selected Registry control/body metadata after written-head projection, retaining existing symbolic branch and completion conditions.
- [rust/tcl-lsp-core/src/package_resolver/reachability.rs](../../../../rust/tcl-lsp-core/src/package_resolver/reachability.rs), `body_may_terminate`: Use the same written-head projection before the existing Registry may-termination query; no native handler or completion follows.
- [rust/tcl-lsp-core/src/package_resolver/reachability.rs](../../../../rust/tcl-lsp-core/src/package_resolver/reachability.rs), `parse_vsatisfies`: Require the existing Tcl-package subject and actual version-requirement geometry under authored written-head projection.
- [rust/tcl-lsp-core/src/package_resolver/reachability.rs](../../../../rust/tcl-lsp-core/src/package_resolver/reachability.rs), `is_tcl_version_subject`: Project only the complete written package head in the existing exact provide/require Tcl subject; other packages or malformed/dynamic subjects decline this guard.
- [rust/tcl-lsp-core/src/package_resolver/reachability/tests/written_head_tests.rs](../../../../rust/tcl-lsp-core/src/package_resolver/reachability/tests/written_head_tests.rs), `package_resolver::reachability::tests::written_head_tests::authored_index_written_heads_keep_complete_colon_runs_and_literal_boundaries` (linked): The actual authored advisory scanner recognises complete odd/even leading colon runs and retains literal Unicode package names. Single-colon, relative-qualified, trailing-empty, dynamic, malformed and case-divergent heads cannot borrow the stock package row. This is a neutral source convention, not a C/Jim lookup or package-registration observation.
- [rust/tcl-lsp-core/src/package_resolver/reachability/tests/written_head_tests.rs](../../../../rust/tcl-lsp-core/src/package_resolver/reachability/tests/written_head_tests.rs), `package_resolver::reachability::tests::written_head_tests::authored_index_written_heads_keep_version_and_termination_conditions` (linked): Actual authored version and termination paths retain selected Registry roles: the Tcl subject and known requirements preserve conditional release advice; unguarded returns stop scanning, loop termination remains conditional and foreign-package/dynamic requirements cannot settle the guard. No loader execution or native completion is inferred.

A named test is a coverage binding, not a claim that it executed.

## Replay

The exact named Rust bodies are source-defined coverage. No executed assertion receipt or external provider observation is attached.
