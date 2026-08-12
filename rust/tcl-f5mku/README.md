# tcl-f5mku — F5 master-key secret crypto

The `f5mku` SecureVault envelope: BIG-IP stores credential-bearing values (SSL
key passphrases, monitor passwords, RADIUS / TACACS / SNMP secrets) in its
configuration encrypted under the unit master key as
`$M$<salt>$<base64-ciphertext>`. On a device that key is what `f5mku -K` prints —
a base64 AES key.

The transform is AES-ECB with PKCS#7 padding over a salt-prefixed plaintext; the
block cipher comes from the audited pure-Rust [`aes`] crate and this crate adds
only the thin salt / padding / base64 envelope.

```rust
use tcl_f5mku::{decrypt, encrypt, extract_salt};
let key = "BHDLd0bbao1VlwpTk1sioQ==";
assert_eq!(decrypt("$M$iP$rr0su9oHn9J9p1t3nRzydA==", key)?, "KEY45678");
```

`decrypt`, `is_ciphertext` and `extract_salt` need no entropy and are always
available (so the crate builds for `wasm32-unknown-unknown`); `encrypt` with a
random salt needs the default `rand` feature (drop it — `default-features =
false` — for a decrypt-only, entropy-free build).

Shared by the `f5 encrypt-secrets` / `decrypt-secrets` CLI verbs
([`f5-cli`](../f5-cli)) and the in-browser BIG-IP report generator
([`tcl-bigip-report`](../bigip-report-gen/rust) / [`bigip-report-wasm`](../bigip-report-gen/wasm)),
which uses it to reveal encrypted secrets in a report.
