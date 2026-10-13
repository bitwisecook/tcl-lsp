# How are the redacted UCS certificates recreated and verified?

`rust/bigip-report-gen/python/tests/data/certs-chain.ucs` now retains its
configuration and named placeholders for three certificates and one private
key. `rust/f5-cli/tests/fixtures/ucs-cert-sample.ucs` retains configuration and
a certificate placeholder. No original key or certificate body is stored.

The [generator](../../../../scripts/dev/generate-certificate-ucs.py) contains
the exact OpenSSL commands and configuration used by both Rust and Python
tests. It creates RSA 2048-bit keys using `openssl genpkey`, checks each with
`openssl pkey -check -noout`, creates certificates using `openssl req`,
`openssl x509` and `openssl ca`, and compares the created certificate's
`openssl x509 -pubkey -noout` output with `openssl pkey -pubout` from its key.
The key directory is private and removed automatically.

The chain fixture includes a locally generated root and intermediate valid
for two days, plus an intentionally expired leaf with dates
`20200101000000Z` to `20200102000000Z`. The exact CA configuration and serial
setup are in the generator. It runs:

```sh
openssl verify -no_check_time -CAfile "$KEY_DIR/root.crt" \
  -untrusted "$KEY_DIR/intermediate.crt" "$KEY_DIR/leaf.crt"
openssl verify -CAfile "$KEY_DIR/root.crt" \
  -untrusted "$KEY_DIR/intermediate.crt" "$KEY_DIR/leaf.crt"
```

The first must succeed, proving chain signatures against the created issuer
material. The second must fail with an expiry error. Ignoring time does not
make the leaf valid for deployment. The single-certificate fixture instead
uses `openssl verify -CAfile "$KEY_DIR/leaf.crt" "$KEY_DIR/leaf.crt"` and
requires success.

To generate either complete UCS outside the checkout:

```sh
umask 077
UCS_DIR=$(mktemp -d /tmp/tcl-certificate-ucs.XXXXXX)
trap 'rm -rf "$UCS_DIR"' EXIT
python3 scripts/dev/generate-certificate-ucs.py \
  rust/bigip-report-gen/python/tests/data/certs-chain.ucs \
  "$UCS_DIR/chain.ucs" --chain
python3 scripts/dev/generate-certificate-ucs.py \
  rust/f5-cli/tests/fixtures/ucs-cert-sample.ucs \
  "$UCS_DIR/single.ucs"
```

The output contains a fresh certificate fingerprint for test comparison.
Retain completion statuses in evidence, excluding generated key, certificate
and archive bodies. Fresh material has a new fingerprint and is not the
original certificate. Both generator configurations and both verification
boundaries passed locally during this scrub.
