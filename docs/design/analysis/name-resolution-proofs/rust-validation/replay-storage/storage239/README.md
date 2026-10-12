# Exact inactive Rust executable storage

The closed storage239 journal preserves nine original pinned Rust executable paths, full byte lengths and SHA-256 digests alongside their gzip storage paths and digests. The [independent verification report](verification.json) checks every compressed stream, complete decompressed executable and unchanged published executable pin. Storage verification executes no Rust assertion, native process or recorded replay.

| Closed journal | Executables | Original bytes | Compressed bytes |
| --- | ---: | ---: | ---: |
| [Inactive pinned Rust executables](lossless-inactive-rust-elves239.json) | 9 | 755,435,136 | 150,701,865 |

The nine gzip streams restore the complete original executable bytes. Their recorded original digests match the published Compiler224/232, Registry228 and Runtime223/225/227/229/233/234 pins. Compiler236 and Runtime238 are outside this journal. The journal records byte identity and contains no permission-mode field.

To restore an executable, check the compressed digest and length against its journal row, decompress into a temporary file, check the original digest and length, then atomically replace the recorded original_path. Apply executable permission after byte verification when a recorded command requires execution. Restore the complete original file; rebuilding from current source cannot reproduce the recorded pin by assumption.

The absolute /workspace paths describe local replay artifacts. This exact storage operation changes no original pin, command receipt, source association or measured outcome. Native captures, original requests, source files and evidence streams remain separate immutable artifacts. Restoration does not supply a successful build or test result. Disposable Cargo cache storage is independent of these executable streams and replay requirements.

| Original executable | Published pin |
| --- | --- |
| pinned-compiler224.elf | [pinned-compiler224.json](../../frozen224/pinned-compiler224.json) |
| pinned-compiler232.elf | [pinned-compiler232.json](../../frozen232/pinned-compiler232.json) |
| pinned-runtime223.elf | [pinned-runtime223.json](../../frozen223/pinned-runtime223.json) |
| pinned-runtime225.elf | [pinned-runtime225.json](../../frozen225/pinned-runtime225.json) |
| pinned-runtime227.elf | [pinned-runtime227.json](../../frozen227/pinned-runtime227.json) |
| pinned-runtime229.elf | [pinned-runtime229.json](../../frozen229/pinned-runtime229.json) |
| pinned-runtime233.elf | [pinned-runtime233.json](../../frozen233/pinned-runtime233.json) |
| pinned-runtime234.elf | [pinned-runtime234.json](../../frozen234/pinned-runtime234.json) |
| pinned-registry228.elf | [pinned-registry228.json](../../frozen228/pinned-registry228.json) |
