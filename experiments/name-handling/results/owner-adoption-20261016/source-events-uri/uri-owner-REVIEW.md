# Existing canonical_file_uri representability repair

Patch `uri-owner-representability.patch` SHA-256
`dedce74292277c41a03c174abf752662f0454881b878f2a1b064e60a356d5109`.
One exact source file, pinned before/after in manifest. Direct TMP rustfmt
edition2024/skip_children passed; only the intended owner/doc and two test hunks
changed. Live before hash matches and patch dry-run passes. No Cargo/build/test,
main or Git operation was performed.

## Cause and bounded change

Locked ls-types0.0.6 Uri::from_file_path applies to_string_lossy to the selected
Path on both hosts. Current canonical_file_uri therefore returns Some with
replacement filename units for original opaque OS names. The six separately
reviewed validation callers cannot establish original Path identity through
that owner, even after removing their own local lossy renderers.

The owner now performs the same path selection first: borrow an absolute path,
canonicalise a relative path through std::fs::canonicalize. It checks to_str()
on this selected actual path before calling the unchanged URI provider. Opaque
units return the existing None result; literal U+FFFD remains valid Unicode.
It passes the selected canonical path to the provider, so there is no second
filesystem resolution/race and no check of a mere relative alias spelling.
All existing URI percent encoding, authority/extended-prefix repairs and
canonical drive/escape casing remain in the same owner chain. No encoder,
resolver, inverse, filesystem identity claim or ASCII fixture substitute added.

Absolute paths still avoid fs access/canonicalisation and need not exist.
Relative paths still need an existing canonicalisable target. An absolute ASCII
symlink to an opaque target keeps its own ASCII path and stays representable;
the relative spelling selects the opaque canonical target and must refuse.

Windows source comparison: the dependency's private strict_canonicalize is
std::fs::canonicalize followed by removal of VerbatimDisk prefix. The new
selected canonical Path retains that prefix until the existing
repair_file_uri_from_path removes the encoded extended disk marker. Its final
URI matches the old chain (including drive/percent case); VerbatimUNC was not
stripped by the private helper and still follows the unchanged UNC repair.
No Windows result is claimed executed. wasm rooted fallback remains unchanged;
relative unrooted paths still cannot acquire a file URI from missing fs entry.

## Tests and dependency

New Unix tests create distinct actual files with byte0xff, literal U+FFFD, and
Unicode/space names. Opaque Path refuses; U+FFFD and Unicode/space paths preserve
proper percent encoding. An actual ASCII symlink is accepted as an absolute
path but its relative canonical selection to opaque bytes refuses; this catches
checking to_str on the input before canonicalisation. A separate control keeps
nonexistent absolute Unicode paths representable and equivalent nonexistent
relative paths unavailable, then verifies equality after the file exists.
Existing owner tests and all six caller sources/assertions/budgets are intact.

Owner patch is independent of d967 caller patch but is required before claiming
those callers now have honest opaque-native-path admission. Callers should keep
existing expect failures for genuinely unsupported paths; no skipped fixture or
substituted pathname. Product callers must preserve their existing None gates.

## Catalogue accounting

Existing lsp-server-path-uri-repair-policy explicitly records production
canonical_file_uri Path-to-URI owner/policy migration. This is a bounded
representability slice within that existing boundary. The recorded POSIX //
authority policy mismatch is not changed, so no full-unit completion or new
owner-facet count is claimed. The six validation IDs remain six separate
existing caller-adoption purposes; this does not automatically mark six done.
The separate native-file-uri-owner-exposure unit concerns dependency exposure
below Core and is not credited by this Server admission check.

Root-owned formatting/build/frozen acceptance remains necessary. In particular,
returning None is faithful refusal, not new support for opaque filesystem names
or a Native path/URI inverse proof.
