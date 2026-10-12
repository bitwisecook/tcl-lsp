# Packaged Tcltest regression and child format restoration

The existing packaged imported_tcltest_test_structure_recognised filter executed once:2pass/0fail, exit0. Provider fixture tested SHA6d4facc342f6fd1f99590398f90691c380d12a308a7818d1d6b09f26f4ab3161 is unchanged. No other tests/builds/traces or production changes were made in this job.

Parent rustfmt had recursively reformatted provider_fixtures/itcl.rs assertions outside the prior eight-file captured scope. Both accidental and HEAD versions produce identical rustfmt stdout under skip_children=true; itcl was restored byte-exact to HEAD before the packaged run. Exact diff, byte snapshots and restoration hashes are retained. This corrects ownership reporting without claiming whole-workspace freeze. Six named dependency hashes remain identical before/after the packaged run.

Scoped #2287 packaged regressions pass. #2286 remains open for the independent compiler cleanup warning and original-program missing-read defects. No full gate/native oracle/runtime conformance claim.
