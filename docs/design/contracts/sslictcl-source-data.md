# SslicTcl source-data contract

The SslicTcl trust-store, TLS registry, and external-observation data are
shipped inside the report builder. A generated report therefore has no runtime
dependency on a web site, package manager, or source repository.

## Layout

When the data crate is present, its checked-in bundle has this shape:

~~~text
rust/tcl-sslictcl/data/
  raw/           # fetched upstream material, retained for auditability
  generated/     # normalised data embedded by the report/TLS implementation
  provenance.json
~~~

The checked-in trust snapshot currently comes from Trust Stores Observatory at
commit `4497c0a43a810c9ddd2c249ae61fecc10ce4c7a6`, covering Apple, Android
AOSP, Microsoft Windows, Mozilla NSS, OpenJDK, and Oracle Java snapshots. The
browser coverage additionally pins Chromium Chrome Root Store commit
`d8639ab8e5fa06c9353560b15afe1c9a8b5c4bc4` and parses its `root_store.certs`
and `root_store.textproto` (including SCT constraints). The raw PEM bundles
are parsed during generation; available roots carry complete
DER, SPKI digest, SKI, and validity metadata in `trust-seed.json`. Roots that
the YAML lists but the pinned PEM archive does not contain remain visible as
memberships and are listed, with a reason, in
`generated/trust-material-exceptions.json`. They must not be treated as
cryptographically complete chain inputs.

The source snapshots publish root membership and retrieval dates, but do not
publish a uniform EKU/purpose classification. Generated anchors therefore do
not assert `server-auth`, `Any`, or another purpose by default. The Mozilla
collector is an explicit server-auth snapshot, and Chromium's root store is an
explicit TLS store; those are the only generated server-auth assertions.
Consumers must report an unknown purpose conservatively until a source with an
explicit purpose assertion is imported. Snapshot dates and Chrome's root-store
version are provenance, not browser product-version ranges; they must not be
used as `decision_for_version` ranges. Membership records carry these as
`snapshot_version`/`snapshot_date` when available, while
`first_version`/`last_version` are reserved for authoritative product-version
ranges.

provenance.json is schema version 1:

~~~json
{
  "schema_version": 1,
  "sources": [{
    "id": "mozilla-nss",
    "url": "https://...",
    "revision": "commit, tag, or release",
    "retrieved_at": "2026-08-16T00:00:00Z",
    "license": "MPL-2.0"
  }],
  "files": [{
    "path": "rust/tcl-sslictcl/data/raw/example.json",
    "kind": "raw",
    "sha256": "64 lowercase hexadecimal characters"
  }]
}
~~~

Every file under raw/ and generated/ must occur exactly once in files. The
kind and path prefix must agree, and the recorded SHA-256 must match the
checked-in bytes. Paths are repository-relative and cannot escape the data
tree.

The data crate may provide
rust/tcl-sslictcl/scripts/update-source-data.sh for the explicit,
network-capable refresh and
rust/tcl-sslictcl/scripts/generate-source-data.sh --check for a deterministic
generated-output check. The top-level wrappers are:

~~~text
make update-source-data   # network-capable; refresh, normalise, and hash
make check-source-data    # offline; verify manifest, hashes, and generators
~~~

Normal builds, report generation, CI drift checks, and report viewing never
run the updater. make rust-check includes check-source-data.

## Release freshness

Before a release, run make update-source-data and then
make check-source-data SOURCE_DATA_MAX_AGE_DAYS=180. If an upstream source
cannot be refreshed, the release may use SSLICTCL_SOURCE_DATA_WAIVER only with
a documented reason recorded in the release preparation notes. The release
preflight performs the same check.

The report data manifest should be surfaced by the report builder so each
report records source identifiers, revisions, retrieval dates, and the
generator version alongside its embedded data.
