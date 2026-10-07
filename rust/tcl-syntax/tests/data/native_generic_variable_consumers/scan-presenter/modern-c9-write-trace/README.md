# Native C9 scan write callbacks

These two controls execute C Tcl 9.0.4 and 9.1.0 with the supported `trace add variable first write bad` form. The callback fails with `BOOM`; scan retains its first variable diagnostic and assigns the later output to 2. The result has a String primary, resident bytes, one reference and an unknown character count before the actual length getter; the string getter preserves that header and the length getter records 23 characters.

`case.tcl` is the exact executed script. `probe.c`, compiler arguments, native/probe/compiler ELF hashes, library/header/source hashes and raw stdout/stderr/status receipts are retained. Probe binaries are omitted. `replay.py` verifies file hashes or recompiles against both explicitly supplied original engine roots, using the two selected global lock slots and a 60-second subprocess limit.

The parent controls retain their original legacy trace program. C9 rejects that trace registration before scan executes; its recorded output variable belongs to the earlier case in the same interpreter. This independently supported trace trigger measures scan callback failure and continuation.
