# What trust data is retained without certificates?

All vendor certificate bundles and the retained certificate archive have been replaced with omission placeholders. `generated/trust-seed.json` retains pinned membership, fingerprints, subjects, and purpose provenance. Every anchor has absent certificate material. The runtime embeds this metadata and accepts caller-supplied certificates for chain evaluation. It cannot complete a path using a vendor root whose bytes were not supplied. `trust-material-exceptions.json` names that boundary for every anchor.

The updater fetches membership and policy metadata only. It does not restore PEM or DER certificate bodies. The generator and offline source-data check enforce the metadata projection. The source provenance manifest hashes the retained placeholders and generated metadata.

Vendor certificates are issued by their upstream certificate authorities. A local `openssl req` command cannot recreate an authentic vendor root or its private signing key. Retrieve the exact pinned public material into a temporary directory when testing against those roots:

```sh
umask 077
CA_DIR=$(mktemp -d /tmp/sslictcl-ca-input.XXXXXX)
trap 'rm -rf "$CA_DIR"' EXIT
OBSERVATORY_REV=$(head -n 1 rust/tcl-sslictcl/data/raw/observatory-revision.txt)
CHROMIUM_REV=$(head -n 1 rust/tcl-sslictcl/data/raw/chrome_root_store/revision.txt)
curl --fail --location --silent --show-error \
  "https://raw.githubusercontent.com/nabla-c0d3/trust_stores_observatory/$OBSERVATORY_REV/docs/trust_stores_as_pem.tar.gz" \
  -o "$CA_DIR/vendor-roots.tar.gz"
tar -xzf "$CA_DIR/vendor-roots.tar.gz" -C "$CA_DIR" mozilla_nss.pem
curl --fail --location --silent --show-error \
  "https://chromium.googlesource.com/chromium/src/+/$CHROMIUM_REV/net/data/ssl/chrome_root_store/root_store.certs?format=TEXT" \
  -o "$CA_DIR/chrome.b64"
openssl base64 -d -A -in "$CA_DIR/chrome.b64" -out "$CA_DIR/chrome.pem"
```

Split a bundle into individual certificates locally, then verify each against its membership fingerprint:

```sh
awk -v directory="$CA_DIR" '
  /-----BEGIN CERTIFICATE-----/ { n++; out=sprintf("%s/root-%04d.pem", directory, n) }
  out != "" { print > out }
  /-----END CERTIFICATE-----/ { close(out); out="" }
' "$CA_DIR/mozilla_nss.pem"
for certificate in "$CA_DIR"/root-*.pem; do
  openssl x509 -in "$certificate" -noout -subject -issuer -dates -fingerprint -sha256
  openssl x509 -in "$certificate" -outform DER -out "$CA_DIR/root.der"
  fingerprint=$(openssl dgst -sha256 "$CA_DIR/root.der" | awk '{print $NF}')
  jq -e --arg fingerprint "$fingerprint" \
    '.trust.anchors | any(.fingerprint_sha256 == $fingerprint)' \
    rust/tcl-sslictcl/data/generated/trust-seed.json >/dev/null
  openssl verify -CAfile "$CA_DIR/mozilla_nss.pem" "$certificate"
done
```

`openssl verify` additionally checks validity and available issuer material. An expired or cross-signed historical root can fail that check; do not label that failure a successful trust validation. Compare the DER fingerprint with the pinned metadata separately. Supply verified local roots through the caller's certificate pool or `TrustStore` input, rather than adding their bytes to this repository. See the [source-data contract](../../../docs/design/contracts/sslictcl-source-data.md).
