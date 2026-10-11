### Source configuration before an opened-file read

The owned-directory controls distinguish opening from reading. C8.5/C8.6 UTF-8 and binary reach the directory read error, while unknown encoding fails first with `TCL LOOKUP ENCODING`. C9.0/C9.1 UTF-8 reaches `EISDIR`; unknown encoding and the removed binary configuration fail before that read. Existing UTF-8 scripts return `SOURCE_OK`, and absent paths report `ENOENT` before lookup. Selected original source opens the channel before configuring its encoding and reads afterward.

C8.4 and Jim refuse the explicit encoding grammar before lookup. Their separate plain-directory controls retain the actual file-read errors: C8.4's `POSIX EISDIR` and Jim's `failed to load file` with seeded errorCode unchanged. System queries retain separate result codes, including Jim's absent encoding command.

Six original processes each contain eight fresh initialized interpreters:48 blocks, four successful script operations and44 Guest failures. Compile/CLI/run and separate same-library diagnostics exit0; all before/after pins match. Full result, errorCode, effect and system-query bytes remain in the separate `646` data directory. No arbitrary VFS callback/refcount chronology, complete errno/close/codec/BOM/EOF parity, Rust assertion or appliance observation is established. The original246 encoding blocks retain their individual attribution.
