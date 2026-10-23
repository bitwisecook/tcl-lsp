# KCS: How do I verify certificates without storing them in the repository?

> **Audience:** Contributor
> **Type:** How-To

Generate test certificates and keys in a private temporary directory. Run
OpenSSL key checks, certificate verification, and comparisons against the
created key before recording the result. Keep the commands, metadata checks,
and result in evidence; replace certificate bodies, private keys, and resolved
credentials with named placeholders. Remove the temporary files afterwards.

The [test certificate recipe](../../rust/tcl-bigip-query/tests/fixtures/certificate-generation.md)
creates a local self-signed certificate and verifies it against its generated
key. The query tests use that procedure and compare their output with OpenSSL
at test time. Their certificate and key are never checked in.

Vendor root certificates, used as [trust anchors](../GLOSSARY.md#trust-anchor),
must come from the upstream issuer. Use the
[pinned vendor verification recipe](../../rust/tcl-sslictcl/data/README.md)
to fetch them locally, compare their fingerprints with retained membership
metadata, and verify them with OpenSSL. The repository retains the membership
metadata and an explicit missing-material record for each root. Supply local
certificates when evaluating a chain; membership metadata alone cannot complete
it.

For passwords, API keys, SSH keys, and passkeys, follow the
creation and verification procedures.
Record successful fresh authentication or the exact failure boundary. Keep
actual values, session cookies, key bodies, and recovery material out of
captured output.
