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

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1, jim, bigip

Status: `not-tested`. Channel: Fixed authored index source and software advisory scan. Version: tcl8.4 not applicable to this authored advisory projection; tcl8.5 not applicable to this authored advisory projection; tcl8.6 not applicable to this authored advisory projection; tcl9.0 not applicable to this authored advisory projection; tcl9.1 not applicable to this authored advisory projection; jim not applicable to this authored advisory projection; bigip not applicable to this authored advisory projection.

The scan asserts an authored written-name convention and symbolic Registry advice. It supplies no native lookup, callback chronology, loader execution or provider equivalence.

## Consumer bindings

- [rust/tcl-lsp-core/src/package_resolver/reachability.rs](../../../../rust/tcl-lsp-core/src/package_resolver/reachability.rs), `scan_with_config`: Retain the supplied whole source grammar and explicit authored advisory convention; original analysis-backed source advice is independent.
- [rust/tcl-lsp-core/src/package_resolver/registration.rs](../../../../rust/tcl-lsp-core/src/package_resolver/registration.rs), `select_in_plan`: Validate the exact complete original command vector and supplied grammar, then project its canonical written head before the selected Registry package registration/body-role query.
- [rust/tcl-lsp-core/src/package_resolver/reachability.rs](../../../../rust/tcl-lsp-core/src/package_resolver/reachability.rs), `ScriptWalker::walk_script`: Reuse selected Registry control/body metadata after written-head projection, retaining existing symbolic branch and completion conditions.
- [rust/tcl-lsp-core/src/package_resolver/reachability.rs](../../../../rust/tcl-lsp-core/src/package_resolver/reachability.rs), `body_may_terminate`: Use the same written-head projection before the existing Registry may-termination query; no native handler or completion follows.
- [rust/tcl-lsp-core/src/package_resolver/reachability/tests/written_head_tests.rs](../../../../rust/tcl-lsp-core/src/package_resolver/reachability/tests/written_head_tests.rs), `package_resolver::reachability::tests::written_head_tests::authored_index_written_heads_keep_complete_colon_runs_and_literal_boundaries` (linked): The actual authored advisory scanner recognises complete odd/even leading colon runs and retains literal Unicode package names. Single-colon, relative-qualified, trailing-empty, dynamic, malformed and case-divergent heads cannot borrow the stock package row. This is a neutral source convention, not a C/Jim lookup or package-registration observation.
- [rust/tcl-lsp-core/src/package_resolver/reachability/tests/written_head_tests.rs](../../../../rust/tcl-lsp-core/src/package_resolver/reachability/tests/written_head_tests.rs), `package_resolver::reachability::tests::written_head_tests::authored_index_written_heads_keep_version_and_termination_conditions` (linked): Actual authored version and termination paths retain selected Registry roles: the Tcl subject and known requirements preserve conditional release advice; unguarded returns stop scanning, loop termination remains conditional and foreign-package/dynamic requirements cannot settle the guard. No loader execution or native completion is inferred.

## Evidence

- [rust/tcl-lsp-core/src/package_resolver/reachability.rs](../../../../rust/tcl-lsp-core/src/package_resolver/reachability.rs)
- [rust/tcl-lsp-core/src/package_resolver/reachability/tests/written_head_tests.rs](../../../../rust/tcl-lsp-core/src/package_resolver/reachability/tests/written_head_tests.rs)
