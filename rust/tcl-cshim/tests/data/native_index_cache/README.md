# Native Index table caches

The probe exercises the same original Tcl object through a prefix lookup, a
same-table exact lookup, a different-table exact lookup, string regeneration,
and duplication. C9 also exercises temporary tables. The observations cover
all five pinned C releases; `manifest.json` records source, library and output
hashes plus the exact compiler commands.

The same table and stride reuse the cached index even when a later call asks
for an exact match. A different table does not reuse that cache. Regenerating
the string, including on a duplicate, reads the current original table entry.
Temporary tables do not install an Index cache.

To reproduce, run the compiler command recorded for a release, replacing its
probe input and output paths with local paths, then execute the output binary.
Compare its JSON lines with the corresponding release file. The observer only
inspects the object representation and resident bytes; it does not query error
globals or alter interpreter completion state.

The shim's `load_static` caller guarantees native code, persistent tables and
their entry strings outlive every retained and duplicated object cache. A
private extension receipt retains that promise alongside the table reader.
Removing a command does not revoke an existing table cache. A dynamic loader
must retain its real library and table ownership before it can issue the same
capability. An arbitrary pointer cannot supply that lifetime authority.
