# KCS: feature — `f5 encrypt-secrets` / `f5 decrypt-secrets`

> **Audience:** User
> **Type:** Functionality

## Applies to

`f5` CLI — the native `f5-query` binary (`rust/f5-cli`). The behaviour
contract is the same verbs, flags, master-key resolution order, and the
byte-for-byte `$M$<salt>$<base64>` envelope.

## Summary

`f5 encrypt-secrets` and `f5 decrypt-secrets` convert the
credential-bearing values in a `bigip.conf` / SCF between clear text and
the encrypted form BIG-IP stores, using the unit master key.  The master
key is the base64 string `f5mku -K` prints on the device.  Encryption
wraps a value in the `$M$<salt>$<base64>` envelope; decryption recovers
the clear text.  Both verbs are idempotent — a value already in the
target form is left untouched.

## Question

How do I read the real passwords out of a `bigip.conf`, or seal clear-text
secrets back into the `$M$...` form, given the device master key?

## How to use

Get the master key off the device once:

```sh
f5mku -K > key.txt          # prints e.g. BHDLd0bbao1VlwpTk1sioQ==
```

Then decrypt or encrypt the secrets in a config:

```sh
# Reveal every stored secret in clear text
f5 decrypt-secrets bigip.conf --f5mku-file key.txt -o clear.conf

# Seal clear-text secrets back into the $M$ envelope
F5MKU="$(cat key.txt)" f5 encrypt-secrets clear.conf -o sealed.conf

# The key can also be passed inline
f5 decrypt-secrets bigip.conf -k BHDLd0bbao1VlwpTk1sioQ==
```

`encrypt` / `decrypt` are accepted as aliases for the two verbs.

The key is resolved from `--f5mku KEY` (`-k`), then `--f5mku-file FILE`,
then the `$F5MKU` environment variable, and finally a secure `F5 MKU Key:`
terminal prompt (no echo).  Pass `--no-key-prompt` to fail instead of
prompting in non-interactive runs.  Output goes to stdout unless
`-o FILE` is given, and `--format scf|tmsh|tmsh-delta` re-renders the
result the same way the other rewriting verbs do (`tmsh-delta` uses the
pre-rewrite config as its baseline, so existing objects emit `modify`,
not a spurious `create`).

### Which values are affected

Only the fields BIG-IP actually master-key encrypts are touched:
`passphrase`, `password`, `secret`, `shared-secret`, `auth-password`,
and `privacy-password`.  SNMP community strings and monitor receive
strings — which the device keeps in clear text and never wraps in
`$M$` — are left alone, unlike the broader
[`f5 redact`](kcs-feature-f5-cli.md) set.  The `auth user`
`encrypted-password` field is deliberately excluded: it holds an
operating-system crypt hash (`$6$…`), not an `$M$` master-key secret.
The literals `none` and `<REDACTED>`, and any value already in a
`$scheme$…` encoded form, are skipped.

### Example

```text
# before  (clear.conf)
auth radius-server /Common/rad {
    secret "my radius secret"
}

# after  f5 encrypt-secrets clear.conf --f5mku-file key.txt
auth radius-server /Common/rad {
    secret "$M$ab$2wzXs0xM6OJcV5A4DJ6zCT4fMYLjTWwOPZNT4VBBbQ0="
}
```

Running `f5 decrypt-secrets` on the output with the same key returns the
original `secret "my radius secret"`.

## Notes

- The transform is AES in ECB mode with PKCS#7 padding and a two-character
  salt — the scheme BIG-IP itself uses.  The `f5-query` front-end delegates
  the block transform to the audited [`aes`] crate already vendored for the
  encrypted-UCS path.
- A wrong master key is reported as an error (the padding or salt check
  fails) rather than producing silent garbage.
- The clear-text output holds real credentials and SSL key passphrases —
  treat `decrypt-secrets` output as sensitive and avoid writing it to a
  shared location.

[`aes`]: https://crates.io/crates/aes
