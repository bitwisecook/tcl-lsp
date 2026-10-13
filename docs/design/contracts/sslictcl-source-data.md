# SslicTcl source-data contract

The SslicTcl trust-store, TLS registry, and external-observation data are
shipped inside the report builder. A generated report therefore has no runtime
dependency on a web site, package manager, or source repository.

## Layout

The checked-in bundle has this shape:

~~~text
rust/tcl-sslictcl/data/
  raw/           # fetched upstream material, retained for auditability
  generated/     # normalised data embedded by the report/TLS implementation
  provenance.json
~~~

The trust snapshot comes from Trust Stores Observatory, covering Apple,
Android AOSP, Microsoft Windows, Mozilla NSS, OpenJDK, and Oracle Java. The
browser coverage additionally pins Chromium's Chrome Root Store policy metadata
from `root_store.textproto`, including SCT constraints. `provenance.json`
records each exact revision. All certificate bodies, including public vendor
roots, are omitted from the repository. Former PEM bundles, `root_store.certs`
and the certificate archive contain explicit omission notices.

`trust-seed.json` retains membership, fingerprints, subjects and purpose
provenance. It has no DER, SPKI, SKI or certificate validity material. Every
anchor is listed in `generated/trust-material-exceptions.json` with this
reason. These records cannot complete a cryptographic chain. The runtime uses
caller-supplied certificates from its certificate pool or `TrustStore` input
for that purpose. Reports must preserve the missing-material boundary.

The updater fetches membership and policy metadata only, and the generator
never imports certificate bodies. The [local verification recipe](../../../rust/tcl-sslictcl/data/README.md)
retrieves exact pinned public roots into a private temporary directory,
compares their DER fingerprints with retained metadata, and runs OpenSSL
verification. Authentic vendor roots must be obtained from their issuer;
self-signing a new local certificate does not recreate them. Test certificates
are generated locally using the [fixture recipe](../../../rust/tcl-bigip-query/tests/fixtures/certificate-generation.md),
verified against the generated key and certificate, and removed afterwards.
No certificates, private keys or resolved credentials are retained in evidence.

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

`rust/tcl-sslictcl/scripts/update-source-data.sh` is the explicit,
network-capable refresh; `rust/tcl-sslictcl/scripts/generate-source-data.sh
--check` is the deterministic generated-output check. The top-level wrappers
are:

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
