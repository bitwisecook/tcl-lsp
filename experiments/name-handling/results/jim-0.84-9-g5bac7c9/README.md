# Upstream Jim naming suites

The selected upstream revision is `5bac7c99ad65864c87da513e22e2f01703fa4e03`,
reported as `0.84-9-g5bac7c9`. GitHub upstream HEAD matched this revision when
checked on 12 October 2026. The repository's shared Jim manifest selects the
same revision. The provisioning script built the actual source checkout.

All 339 tests in 12 original naming-related suites passed, with zero skips,
failures, timeouts or nonzero process exits. Each original suite sources Jim's
real `tests/testing.tcl` and distribution `tcltest.tcl`. Jim's extension-initialised
shell has its own startup contract; C Tcl's `init.tcl` is not substituted for it.

[records.json](records.json) retains individual suite counters, exact source and
stream hashes, arguments and process status. [provenance.json](provenance.json)
retains binary, harness and runner hashes. Reproduce with
[run_jim_suites.py](../../run_jim_suites.py) into a new destination.

These observations execute upstream Jim only. They do not establish VM/runtime
conformance, C alias/TclOO compatibility, another Jim build's capabilities or
an unexecuted F5 result. Use Jim-specific input and capability owners when
promoting these observations to conformance tests.
