<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
# registry-audit

Generators / provenance for the BIG-IP registry data under
`rust/tcl-registry/src/`.

## Profile defaults (`gen_profile_defaults.py`)

Turns a real F5 `profile_base.conf` — the read-only default-profile base every
BIG-IP ships in `/config/profile_base.conf` — into
`rust/tcl-registry/src/profile_defaults/generated.rs`
(`PROFILE_DEFAULTS_GENERATED`). SCF exports and `tmsh list … one-line` omit any
field left at its default, so the report / `f5 query` need this table to
reconstruct a base profile's effective configuration.

For each `ltm profile <type> <name> { … }` object the *root* default of a type
is the block with no `defaults-from` (or `defaults-from none`); its fields become
the type's defaults. Scalars map to `field value`; single-level `{ a b c }`
lists to `a b c`; nested `{ k { … } }` blocks to a flattened `{ … }` string.

A single snapshot can only express one version, so all entries are
version-unbounded. Cross-release changes a snapshot can't capture (e.g. the base
client/server-ssl `options` gaining `no-tlsv1.3` — TLS 1.3 off by default — at
14.0) are listed in the script's `OVERRIDES` table and emitted as adjacent
half-open `VersionRange`s; the script asserts the snapshot still matches the
current override so drift is caught.

### Regenerate

```sh
python3 scripts/registry-audit/gen_profile_defaults.py \
  scripts/registry-audit/data/profile_base.conf \
  rust/tcl-registry/src/profile_defaults/generated.rs
```

To refresh from a newer TMOS release, drop its `profile_base.conf` in `data/`
and re-run; add an `OVERRIDES` entry (with the release boundary) for any field
whose default changed between snapshots.

### Provenance

`data/profile_base.conf` is a verbatim TMOS default-profile base taken from the
[f5-corkscrew](https://github.com/f5devcentral/f5-corkscrew) project's test
fixtures (`tests/archive_generator/archive1/config/profile_base.conf`),
distributed by F5 DevCentral under the Apache License 2.0. It is retained here
solely as the input for the generator above. See `DUAL-LICENSING.md`.
