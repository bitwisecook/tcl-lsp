# Public NUL-containing alias names

`input.tcl` is the unchanged original ASCII input run on C8.4.20–9.1.0 and pinned latest Jim 0.84-9-g5bac7c9. It constructs the original NUL-containing names `::alias\0old` and `::alias\0new` using `binary format`. It reports actual creation results, alias queries/inventory, direct calls and origins as hex, without inferring a token from the result.

All C references retain these as distinct alias registrations: after moving the first command to `::moved`, registering the second leaves the moved command callable and both registrations queryable. All C records retain whole original creation names. Latest Jim does not support these original `interp alias`/`interp aliases` forms; its actual errors and subsequent missing-command observations are retained, with no C alias policy transfer.

Guest NUL characters do not establish an internal native raw00 C-string extent. The pure shared coordinate control accepts explicit native byte inputs and remains a geometry test. These public observations must not be used to truncate guest strings or to claim Native header/allocation/cache identity.

Each provider's `record.json` retains input/raw output hashes and actual commands, outcomes and version authority. The executable and complete matching C library inventory were independently hashed before and after this capture, and all inputs remained stable. No tcltest was used. The existing source and provisioning authority is linked from the original origin records; no new source-build closure is inferred.
