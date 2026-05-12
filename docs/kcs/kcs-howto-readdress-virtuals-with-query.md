# KCS: How do I bulk-readdress virtual servers into a new subnet?

> **Audience:** User
> **Type:** How-To

## Applies to

tcl-lsp CLI

## Question

How do I bulk-readdress virtual servers into a new subnet?

## Before you start

- A `bigip.conf` / SCF text file (e.g. extracted from a UCS with `f5 extract`).
- The target network in CIDR form, for example `192.168.9.0/24`.
- The `f5` CLI on your `PATH`; in dev, `python -m explorer.f5_cli` works the same way.

## Answer

`f5 query` ships an `ip(network, source)` builtin that rebases a source address into a new network, preserving the host bits.  Combined with the `|=` update operator, one line readdresses every virtual server's destination.

1. Preview the rewrite with a dry-run:

   ```
   f5 query '.ltm.virtual[] | .destination |= ip("192.168.9.0/24", .)' bigip.conf
   ```

   The verb prints a unified diff so you can verify the host bits and ports are preserved.

2. When the diff looks right, persist the change:

   ```
   f5 query --in-place '.ltm.virtual[] | .destination |= ip("192.168.9.0/24", .)' bigip.conf
   ```

   `--in-place` overwrites the input file.  Use `--write > new.conf` instead if you'd rather keep the original untouched.

3. To readdress only a subset, add a `select(...)`:

   ```
   f5 query --in-place '
     .ltm.virtual["~^vs_prod_"]
     | .destination |= ip("192.168.9.0/24", .)
   ' bigip.conf
   ```

   The regex subscript narrows the stream to VSes whose full-path matches `^vs_prod_`; the same `|=` rewrite then runs against each one.

The partition prefix on `destination` (`/Common/...`) and the `:port` suffix are preserved automatically — `ip()` strips and re-attaches them around the address-arithmetic.

## Related

- [BIG-IP Query DSL](features/kcs-feature-bigip-query.md)
- [BIG-IP rename](features/kcs-feature-rename.md)
- [F5 query DSL design](../design/f5-query-dsl.md)
