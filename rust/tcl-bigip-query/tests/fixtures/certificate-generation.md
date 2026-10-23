# How is the query certificate fixture generated and verified?

The former certificate and its public-key bytes have been replaced by a placeholder. `tests/common/mod.rs` generates fresh material in a private temporary directory for each test. Its destructor removes that directory. The test compares the query result with the created certificate's OpenSSL fingerprint and public key; fixed metadata stays in `probes.json`.

To reproduce the fixture outside the checkout:

```sh
umask 077
CERT_DIR=$(mktemp -d /tmp/tcl-query-certificate.XXXXXX)
trap 'rm -rf "$CERT_DIR"' EXIT
openssl genpkey -algorithm RSA -pkeyopt rsa_keygen_bits:2048 -out "$CERT_DIR/key.pem"
openssl req -new -x509 -key "$CERT_DIR/key.pem" -out "$CERT_DIR/certificate.pem" \
  -sha256 -days 2 -set_serial 1 -subj '/C=US/O=Example Corp/CN=example.test' \
  -addext 'subjectAltName=DNS:example.test,DNS:www.example.test,IP:192.0.2.10'
openssl pkey -in "$CERT_DIR/key.pem" -check -noout
openssl verify -CAfile "$CERT_DIR/certificate.pem" "$CERT_DIR/certificate.pem"
openssl x509 -in "$CERT_DIR/certificate.pem" -noout -subject -issuer -dates -fingerprint -sha256
openssl x509 -in "$CERT_DIR/certificate.pem" -pubkey -noout > "$CERT_DIR/cert-public.pem"
openssl pkey -in "$CERT_DIR/key.pem" -pubout > "$CERT_DIR/key-public.pem"
cmp -s "$CERT_DIR/cert-public.pem" "$CERT_DIR/key-public.pem"
```

The comparison verifies that the certificate and generated private key correspond. Record completion codes and metadata, rather than certificate, public-key, or private-key contents. OpenSSL's [`req`](https://docs.openssl.org/3.4/man1/openssl-req/) and [`x509`](https://docs.openssl.org/3.4/man1/openssl-x509/) manuals describe the creation and inspection options.
