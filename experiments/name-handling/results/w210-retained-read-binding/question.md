# W210 retained read binding

Does forwarding the same realm-issued binding onto the already reconstructed original complete command vector restore W210 authored read advice while preserving original source/currentness/invalid-owner guards? Production change only in script_binds.rs; syntax-only producer unchanged. Run existing declaration-read owner regression and Tcltest nine-control filter once; packaged two-control consumer check only if Tcltest is green. No runtime/oracle/full-gate claim. Shared special_vars owner hash verified at freeze.
