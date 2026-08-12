# KCS: How do I read an encrypted (passphrase-protected) UCS with the `f5` CLI?

> **Audience:** User
> **Type:** How-To

## Applies to

tcl-lsp CLI

## Question

A teammate saved a BIG-IP archive with `tmsh save sys ucs prod.ucs
passphrase <secret>`, so the `.ucs` is encrypted. How do I run
`f5 query`, `f5 extract`, `f5 diff`, and the other verbs against it
without decrypting it by hand first?

## Before you start

- The encrypted `.ucs` file on disk.
- The passphrase it was saved with.
- Nothing else. Decryption is built in, so no GnuPG install is needed.

## Answer

Every `f5` verb that reads a `.ucs` decrypts it **transparently** —
`query`, `extract`, `convert ucs2scf`, `diff`, `grep`, `cleanup`, and
`irule`. You only have to supply the passphrase. The cleartext is held
in memory and never written to disk, because a UCS contains the
device's SSL private keys.

There is no flag that takes the passphrase value on the command line —
that would leak it into your shell history and the process list. The
passphrase is resolved in this order:

1. the `F5_UCS_PASSPHRASE` environment variable — or a variable you
   name with `--passphrase-env VAR` (on `extract` / `convert`);
2. a secure terminal prompt, shown only when the session is
   interactive (suppress it with `--no-passphrase-prompt`).

### Pass the passphrase by environment variable (works on every verb)

```
$ F5_UCS_PASSPHRASE='s3cret!' f5 query --raw '.ltm.virtual[].name' prod.ucs
app_http_vs
app_https_vs
```

The same variable unlocks `f5 diff old.ucs new.ucs`, `f5 extract`,
`f5 grep`, and the rest — set it once for the command.

### Let it prompt you (interactive sessions)

Run any verb with no passphrase set and, on a terminal, it asks:

```
$ f5 extract prod.ucs -o prod.scf
Passphrase for prod.ucs:
```

The prompt never echoes, and it never blocks a non-interactive run —
in a pipeline or CI job with no passphrase available the verb fails
fast with a clear message instead of hanging.

### Scripts and CI — name your own variable, forbid the prompt

`f5 extract` and `f5 convert` add two flags for unattended use:

```
$ UCS_PW='s3cret!' f5 extract --passphrase-env UCS_PW prod.ucs -o prod.scf
$ f5 extract --no-passphrase-prompt prod.ucs        # fails fast, never hangs
error: this UCS archive is encrypted and requires a passphrase; set the
F5_UCS_PASSPHRASE environment variable or run in an interactive terminal
to be prompted
```

`--passphrase-env VAR` reads the passphrase from the variable you
name; `--no-passphrase-prompt` makes the verb require the variable and
refuse to prompt, which is what you want in a scheduled job.

## How to tell it worked

The verb produces normal output (a query result, an extracted SCF, a
diff) instead of a binary blob or a decode error. A wrong passphrase
fails cleanly:

```
$ F5_UCS_PASSPHRASE='wrong' f5 query --raw '.ltm.virtual[].name' prod.ucs
error: failed to decrypt UCS archive: gpg: decryption failed: Bad session key
```

## Operational context

### What `tmsh save sys ucs ... passphrase` actually produces

Per F5 KB **K5437** the encrypted archive is a GnuPG **symmetric**
(AES) OpenPGP message that wraps the ordinary gzip tar of `/config`.
BIG-IP itself uses AES-128; the reader accepts AES-128, AES-192, and
AES-256.

### No GnuPG needed

The decryptor is built into the tool and has no external dependency, so
a UCS opens on a host with no GnuPG installed. Supply the passphrase in
the `F5_UCS_PASSPHRASE` environment variable to avoid putting it on the
command line:

```
$ F5_UCS_PASSPHRASE='s3cret!' f5 extract prod.ucs -o prod.scf
```

### The keys never touch disk

Both the decryption and the gzip/tar extraction run entirely in
memory. No verb ever writes or re-encrypts a UCS — reading is
decrypt-only — so the cleartext that holds the SSL private keys is
never staged on disk.

### Plain UCS still just works

A plain (unencrypted) `.ucs` needs no passphrase and no flags; the
reader detects the OpenPGP wrapper and only asks for a passphrase when
the archive is actually encrypted.

## Related

- [KCS index](README.md)
- [kcs-howto-verify-migration-before-after-with-query.md](kcs-howto-verify-migration-before-after-with-query.md)
  — verify a migration before/after, straight from (encrypted) UCS files.
- [kcs-howto-audit-server-certs-with-query.md](kcs-howto-audit-server-certs-with-query.md)
  — audit the certs a UCS carries against the live endpoints.
- F5 KB **K5437** — Saving and restoring an encrypted UCS archive.
