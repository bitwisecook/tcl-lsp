# Affected test-build capture

The compiler, Core, DB and tooling test-build preflight stopped at two compiler fixture errors: a missing enum qualification and an incomplete synthetic Module initializer. No tests ran. Exact diagnostics, argv, available artifact identity and unchanged authored source snapshots are retained. This result supplies no semantic or provider conformance acceptance.
