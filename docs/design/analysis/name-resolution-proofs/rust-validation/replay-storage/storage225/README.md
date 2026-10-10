# Exact replay artifact storage

The three closed storage225 journals retain the original artifact paths, SHA-256 digests, byte lengths and permissions alongside the compressed storage paths and digests. Independent streaming verification checks every compressed stream and decompressed original against the journals. The [verification report](verification.json) records that check; it executes no Rust test, native process or recorded replay.

| Closed journal | Entries | Original bytes | Compressed bytes |
| --- | ---: | ---: | ---: |
| [Pinned Rust executables](rust-elf-lossless-storage225.jsonl) | 3 | 294,777,560 | 58,976,477 |
| [Inactive command receipts and logs](inactive-run-receipt-storage225.jsonl) | 591 | 1,345,411,143 | 230,356,966 |
| [Archived artifact directories](inactive-publication-staging225.jsonl) | 85 | 4,037,367,360 | 549,665,054 |

The gzip journals identify each original file and its mode. The tar journal identifies all 8,923 relative archive members: 6,349 files and 2,574 directories. Each file has its exact original byte length and digest; every member has its original mode. Tar hard links are checked through their decompressed file contents and an in-archive relative target. Storage hard links establish no interpreter object or native pointer identity.

Restore the exact artifact to its recorded original path before attempting a command that requires it. Check the compressed digest and length first, decompress into a temporary destination, check the original digest and length, apply the recorded permission mode and atomically replace the original path. For a tar archive, first check its archive digest and length, then validate every relative path, member kind, file digest, byte length and mode against the complete per_leaf map. Reject absolute paths, parent traversal and unlisted members; hard-link targets must remain within the same recorded archive namespace. Extract only that verified namespace to its recorded original_directory and verify the restored leaves before using them.

The absolute /workspace paths describe local replay artifacts. The journals preserve the original command receipts, executable pins and source associations; compressed storage does not select a different executable or rebuild a pinned image. Restoration does not execute an assertion, supply a successful build or change a measured outcome. Native captures, original requests and the published provider evidence are separate immutable files.

Disposable Cargo debug/tmp cache removal is separate from these journals. Such a cache is neither pinned proof storage nor a prerequisite for restoring a recorded receipt, log, executable or artifact directory.
