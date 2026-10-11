# naming.source.original-counted-filename-filesystem-extent

Kind: `native-observation`

## Problem statement

Source filenames have separate original counted String bytes, external filesystem bytes and diagnostic C-string extents. Conflating those extents, borrowing source script codecs or forgiving strict conversion units changes the selected file and winning result.

## Question

How do the original C and Jim source commands convert counted filename bytes, select real byte-named files and report literal/modified-zero and surrogate-unit failures under actual UTF-8, ISO-8859-1 and identity system encoding selections? What exact results, errorCode, effects, selection results and original headers remain?

## Conclusion

The observed Unix C8.4 worker applies C-string extent after actual external conversion, while C8.5 and later refuse converted external zero with EINVAL; reporting retains original String C-string extent. UTF-8 maps resident raw FF to C3 BF. ISO-8859-1 selects raw E9/FF and substitutes question mark for euro on C8, while C9 reports strict EILSEQ. Named identity copies bytes on C8 but is unknown on the measured C9 catalogue. C8.4/C8.5 preserve supplied CESU pairs; C8.6 combines them to four-byte UTF-8; C9 rejects both unpaired units and supplied CESU pairs with EILSEQ. Jim independently supplies raw C-string filenames and preserves seeded errorCode on its observed failures. Real byte-named file effects establish the selected backend. Every reached source operation preserves its original counted header.

## Scope

Eighteen separately compiled original public-object probe processes provide 193 fresh initialized interpreter blocks: filename-extent controls contribute36 (nine source successes,27 Guest errors), filesystem-codec controls contribute133 (60 source successes,52 source Guest errors,21 failed selections that never call source), and strict-UTF8 controls contribute24 (ten source successes,14 Guest errors). There are172 actual source operations and21 selection-only failures. Each collection retains six CLI and six independent same-static-library version diagnostics, exact full input pins, original streams/receipts/probes/runners and lossless driver/version executable bytes. The selected Unix source and output-conversion windows join whole input payloads. Invalid OS byte filenames are retained through raw path hex or lossless JSON. Query result state, operation RETURN and later observers remain distinct. These finite cases establish no arbitrary codec or Windows behavior, callback/updater/free chronology, Native sharing/reference neutrality, same-owner encoding-setting mutation timing, complete historical toolchain/dynamic load closure, source read/configuration precedence, Rust assertion/build result or BIG-IP outcome.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20. Build: Three original compiled driver processes per provider, actual static-library/header/source/tool fingerprints and full before/after input pins; separate CLI and same-static-library diagnostics. Historical library build commands and complete dynamic load closure are unrecorded.. Channel: Original counted public String constructors and object-vector source operations, with explicit argv and result/errorCode/effect observers. Filename extent uses six fresh interpreters; codec selection uses25 C interpreters or eight Jim interpreters; strict UTF-8 uses four. Failed codec selection never calls source. Raw input is observed before getter; the post-operation header is a separate observer.. Dialect: tcl8.4.

Filename extents: Under the explicitly selected UTF-8 system encoding, plain existing, literal-zero-tail existing and modified-zero-tail existing all execute the exact file and return SOURCE_OK. All three absent variants report POSIX ENOENT. The literal-zero diagnostic ends at the original zero; modified-zero C080 plus TAIL remains in the original diagnostic even though external conversion terminates filesystem lookup. Success preserves the seeded errorCode and all original path headers remain byte-exact.

Filesystem codecs: The actual UTF-8 selection maps internal é to the UTF-8 file, resident raw FF to C3 BF rather than replacement, and euro to its UTF-8 file. ISO-8859-1 selects raw E9/FF files and substitutes question mark for euro, selecting QUESTION_EXTERNAL. Identity selects the original byte files without Unicode decoding. Literal zero truncates all three backend codecs; modified zero truncates after UTF-8/ISO conversion but remains distinct absent bytes with identity. Ordinary absent names report POSIX ENOENT. Source success preserves seeded errorCode; original counted headers remain exact. Unknown selection fails and source is not called.

Strict UTF-8 filenames: ASCII executes its file. The supplied CESU pair selects the original CESU byte-named file, not the four-byte UTF-8 file. Both unpaired units remain distinct absent external names and report POSIX ENOENT. Success preserves the seeded errorCode and every original path header remains exact.

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Three original compiled driver processes per provider, actual static-library/header/source/tool fingerprints and full before/after input pins; separate CLI and same-static-library diagnostics. Historical library build commands and complete dynamic load closure are unrecorded.. Channel: Original counted public String constructors and object-vector source operations, with explicit argv and result/errorCode/effect observers. Filename extent uses six fresh interpreters; codec selection uses25 C interpreters or eight Jim interpreters; strict UTF-8 uses four. Failed codec selection never calls source. Raw input is observed before getter; the post-operation header is a separate observer.. Dialect: tcl8.5.

Filename extents: Plain existing runs the file; plain absent reports POSIX ENOENT. All four literal/modified-zero existing/absent variants report POSIX EINVAL without the source effect. Literal zero truncates the original diagnostic while modified C080 plus TAIL remains visible. Original counted path headers remain byte-exact.

Filesystem codecs: UTF-8 selects the é, C3 BF and euro UTF-8 files. ISO-8859-1 selects raw E9/FF and QUESTION_EXTERNAL for euro. Identity is available and selects raw original bytes. Literal external zero refuses with POSIX EINVAL; modified zero refuses under UTF-8/ISO but remains distinct absent bytes under identity and reports ENOENT. Unknown selection fails before source. Source success preserves seeded errorCode and every reached source header is exact afterward.

Strict UTF-8 filenames: ASCII executes its file. The supplied pair selects PAIR_CESU rather than PAIR_UTF8; both unpaired original units report POSIX ENOENT on the guaranteed absent names. Every original counted header remains exact and success preserves seeded errorCode.

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Three original compiled driver processes per provider, actual static-library/header/source/tool fingerprints and full before/after input pins; separate CLI and same-static-library diagnostics. Historical library build commands and complete dynamic load closure are unrecorded.. Channel: Original counted public String constructors and object-vector source operations, with explicit argv and result/errorCode/effect observers. Filename extent uses six fresh interpreters; codec selection uses25 C interpreters or eight Jim interpreters; strict UTF-8 uses four. Failed codec selection never calls source. Raw input is observed before getter; the post-operation header is a separate observer.. Dialect: tcl8.6.

Filename extents: Plain existing runs the file; plain absent reports POSIX ENOENT. All four literal/modified-zero existing/absent variants report POSIX EINVAL without the source effect. Literal zero truncates the diagnostic and modified C080 plus TAIL is preserved. Original counted path headers remain byte-exact.

Filesystem codecs: UTF-8 selects é, C3 BF for raw FF, and euro UTF-8 files. ISO-8859-1 selects raw E9/FF and substitutes question mark for euro. Identity selects original byte names. Literal external zero and converted modified zero refuse with POSIX EINVAL; identity modified C080 remains an absent distinct path and reports ENOENT. Unknown selection fails without a source call. Reached source path headers remain exact and success preserves seeded errorCode.

Strict UTF-8 filenames: ASCII executes its file. The supplied CESU pair combines to the real four-byte external UTF-8 filename and selects PAIR_UTF8. Both unpaired units remain absent names and report POSIX ENOENT. Original counted headers remain exact and success preserves seeded errorCode.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Three original compiled driver processes per provider, actual static-library/header/source/tool fingerprints and full before/after input pins; separate CLI and same-static-library diagnostics. Historical library build commands and complete dynamic load closure are unrecorded.. Channel: Original counted public String constructors and object-vector source operations, with explicit argv and result/errorCode/effect observers. Filename extent uses six fresh interpreters; codec selection uses25 C interpreters or eight Jim interpreters; strict UTF-8 uses four. Failed codec selection never calls source. Raw input is observed before getter; the post-operation header is a separate observer.. Dialect: tcl9.0.

Filename extents: Plain existing runs the file; plain absent reports POSIX ENOENT. All four literal/modified-zero variants report POSIX EINVAL before any source effect; diagnostics use the original String CString extent, not the converted backend extent. Every original counted path header remains byte-exact.

Filesystem codecs: UTF-8 selects é, raw-FF-as-C3-BF and euro UTF-8 files. ISO-8859-1 selects raw E9/FF, but strict euro conversion reports POSIX EILSEQ with no source effect. Converted zero refuses with POSIX EINVAL. All eight identity selections are unknown and never call source, as does the separate unknown codec selection. Source success preserves seeded errorCode and all reached source headers remain exact.

Strict UTF-8 filenames: ASCII executes its file. Unpaired high, unpaired low and the supplied CESU pair each report POSIX EILSEQ with the original filename diagnostic and no source effect. Neither pair file is executed. All original path headers remain exact; ASCII success preserves seeded errorCode.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Three original compiled driver processes per provider, actual static-library/header/source/tool fingerprints and full before/after input pins; separate CLI and same-static-library diagnostics. Historical library build commands and complete dynamic load closure are unrecorded.. Channel: Original counted public String constructors and object-vector source operations, with explicit argv and result/errorCode/effect observers. Filename extent uses six fresh interpreters; codec selection uses25 C interpreters or eight Jim interpreters; strict UTF-8 uses four. Failed codec selection never calls source. Raw input is observed before getter; the post-operation header is a separate observer.. Dialect: tcl9.1.

Filename extents: Plain existing runs the file; plain absent reports POSIX ENOENT. Literal and modified zero variants each report POSIX EINVAL without source effects, retaining the distinct original diagnostic extents. Every original counted path header remains byte-exact.

Filesystem codecs: UTF-8 selects é, raw-FF-as-C3-BF and euro UTF-8 files. ISO-8859-1 selects raw E9/FF but euro reports strict POSIX EILSEQ without source effects. Literal and modified external zero report POSIX EINVAL. Named identity is unavailable: eight failed selections never call source; the separate unknown selection also never calls source. Reached source headers remain exact and success preserves seeded errorCode.

Strict UTF-8 filenames: ASCII executes its file. Each unpaired unit and the supplied CESU pair fail strict conversion with POSIX EILSEQ before any source effect. Both actual pair files remain unexecuted. Every original counted header remains exact and ASCII success preserves seeded errorCode.

### jim

Status: `observed`. Version: 0.84-9-g5bac7c9. Build: Three original compiled driver processes per provider, actual static-library/header/source/tool fingerprints and full before/after input pins; separate CLI and same-static-library diagnostics. Historical library build commands and complete dynamic load closure are unrecorded.. Channel: Original counted public String constructors and object-vector source operations, with explicit argv and result/errorCode/effect observers. Filename extent uses six fresh interpreters; codec selection uses25 C interpreters or eight Jim interpreters; strict UTF-8 uses four. Failed codec selection never calls source. Raw input is observed before getter; the post-operation header is a separate observer.. Dialect: jim.

Filename extents: Plain existing and literal-zero-tail existing run the file and return SOURCE_OK. Plain absent, literal-zero-tail absent and both modified-zero variants report the original capitalized No such file or directory and preserve the seeded errorCode. The raw modified-zero filename remains a different path; Jim does not use C external filename conversion. Original path headers remain byte-exact.

Filesystem codecs: The independent raw-source worker has no encoding command or C system selection. It selects original UTF-8 é/euro bytes and raw FF. Literal zero truncates to the ASCII prefix; modified C080 remains a distinct absent path. Ordinary absent cases and modified-zero failure retain the original capitalized No such file or directory and seeded errorCode. All eight reached source headers remain exact.

Strict UTF-8 filenames: The independent raw-source worker executes ASCII and the raw CESU pair file. Both unpaired raw names are absent and report capitalized No such file or directory while preserving seeded errorCode. No C UTF-8 system converter is used, and all four original headers remain exact.

### bigip

Status: `not-tested`. Version: not tested. Build: No appliance process or library measured.. Channel: No appliance filename input executed.. Dialect: bigip.

No original BIG-IP filename or filesystem codec observation is available.

## Exact evidence

- `whole-input-map` (provider): [runtime/rust/tests/data/native_source_original_counted_filename_extent/pinned-input-map.json](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/pinned-input-map.json). SHA-256 `f5ea0b148600f7dea1cff4c3cecbe4a3fb01bff2af69ee00ccfbac023424d5a2`. Every measured original path joins its complete digest-keyed input archive member, including exact OS byte paths.
- `whole-input-bytes` (provider): [runtime/rust/tests/data/native_source_original_counted_filename_extent/pinned-input-bytes.tar.gz](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/pinned-input-bytes.tar.gz). SHA-256 `58a40b82635d9bbb613f82343d01ec8eead991b10cdda38edbb89b0149962043`. One complete lossless archive of measured source/header/configuration/resources/library/compiler/CLI/input bytes shared across the three selected purposes.
- `original-retention` (provider): [runtime/rust/tests/data/native_source_original_counted_filename_extent/retention.json](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/retention.json). SHA-256 `e358967451964218e8cc70aad49af8177b9dfb5add7dbc1bfdadc1baadb3d050`. All original streams, receipts, source files and36 driver/version ELFs are losslessly retained; raw filenames have exact byte mappings.
- `complete-operation-projection` (observation): [runtime/rust/tests/data/native_source_original_counted_filename_extent/interpretation.json](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/interpretation.json). SHA-256 `09aec731915d016496155bca7cf538538b9d01801125ff5099fc78b852545d00`. All193 original interpreter blocks retain independent source and selection/query results;21 failed selections never call source.
- `replay-path-adaptation` (input): [runtime/rust/tests/data/native_source_original_counted_filename_extent/REPLAY-PATH-ADAPTATION.json](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/REPLAY-PATH-ADAPTATION.json). SHA-256 `308a1b053ca5cc421d14f2404395465f1b70603aa8119c2ba3b489745f95ade5`. Explicit path-only replay adaptation; original launch algorithms and unmodified protocol scripts remain retained.
- `filename-648-probe-c` (input): [runtime/rust/tests/data/native_source_original_counted_filename_extent/648/probe.c](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/648/probe.c). SHA-256 `b1d303979d1cb26b37606f275403cb9a875cede5b326030fbfc3376bdf427de3`. filename extents: Exact public object constructors, original argv and separate observer schedule.
- `filename-648-request-json` (input): [runtime/rust/tests/data/native_source_original_counted_filename_extent/648/request.json](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/648/request.json). SHA-256 `60e5b2dca4ad0e992c6a496ea2d2fb79188c2ea27b8ae37c3af9fe420eed969a`. filename extents: Original selected inputs and purpose; no answer is prescribed.
- `filename-648-input-manifest-json` (input): [runtime/rust/tests/data/native_source_original_counted_filename_extent/648/input-manifest.json](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/648/input-manifest.json). SHA-256 `9cf480d10cad5d4b4d5a1391c8d291eb871137215f77d44154fdb5f12b9213dc`. filename extents: Exact original input manifest bound by the actual receipts.
- `filename-648-results-json` (observation): [runtime/rust/tests/data/native_source_original_counted_filename_extent/648/results.json](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/648/results.json). SHA-256 `834785fdddcb87c0e3830e6327a31c2ebf61c7bee4c689db460785bf7146d35c`. filename extents: Original six closed process summaries and immutable receipt hashes.
- `filename-648-compiler-json` (provider): [runtime/rust/tests/data/native_source_original_counted_filename_extent/648/compiler.json](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/648/compiler.json). SHA-256 `dc7ff8306f2c9de4db66d38470841b2d9aaf67ee6c367c83646523f13965dc21`. filename extents: Measured compiler executable/version and exact launch metadata.
- `filename-648-library-version-results-json` (observation): [runtime/rust/tests/data/native_source_original_counted_filename_extent/648/library-version-results.json](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/648/library-version-results.json). SHA-256 `087a279f5ea7cede3f176d7063777139cd581de75e8709a1bd9b35e8184d8fa5`. filename extents: Six independent same-static-library diagnostic receipt joins.
- `filename-650-probe-c` (input): [runtime/rust/tests/data/native_source_original_counted_filename_extent/650/probe.c](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/650/probe.c). SHA-256 `ebee73cb221c29e32cf7a89a73ca01b0bd8d24f6ccbbfe9f155d92b8f1e80db1`. filesystem codecs: Exact public object constructors, original argv and separate observer schedule.
- `filename-650-request-json` (input): [runtime/rust/tests/data/native_source_original_counted_filename_extent/650/request.json](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/650/request.json). SHA-256 `0cce4f33471da6e7b5f6d243bfd60555f963bc58b2eb2cc46c1d7c7cf8e6a27d`. filesystem codecs: Original selected inputs and purpose; no answer is prescribed.
- `filename-650-input-manifest-json` (input): [runtime/rust/tests/data/native_source_original_counted_filename_extent/650/input-manifest.json](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/650/input-manifest.json). SHA-256 `08699022a2be0a7ad47069ec65755a7c3637e8eb51250698737fb2ff18bad2a0`. filesystem codecs: Exact original input manifest bound by the actual receipts.
- `filename-650-results-json` (observation): [runtime/rust/tests/data/native_source_original_counted_filename_extent/650/results.json](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/650/results.json). SHA-256 `9243e48dc3e2872beaa9d4d1f2f0227ad97760ec618d8160bfc82963e50e2ce0`. filesystem codecs: Original six closed process summaries and immutable receipt hashes.
- `filename-650-compiler-json` (provider): [runtime/rust/tests/data/native_source_original_counted_filename_extent/650/compiler.json](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/650/compiler.json). SHA-256 `fffd32ab175a9787b5cb28e2fe99f9719b59572d2199e914b1ff8931971e085b`. filesystem codecs: Measured compiler executable/version and exact launch metadata.
- `filename-650-library-version-results-json` (observation): [runtime/rust/tests/data/native_source_original_counted_filename_extent/650/library-version-results.json](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/650/library-version-results.json). SHA-256 `988b0824d0fb321c5e7523b64a3cad5bcdb5020c4c5be51f3e21b59801d73fe5`. filesystem codecs: Six independent same-static-library diagnostic receipt joins.
- `filename-651-probe-c` (input): [runtime/rust/tests/data/native_source_original_counted_filename_extent/651/probe.c](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/651/probe.c). SHA-256 `f27f6fd9d54e39a73482a51c1f10641d25f21b9607316edac0eadfdae5d00612`. strict UTF-8 filenames: Exact public object constructors, original argv and separate observer schedule.
- `filename-651-request-json` (input): [runtime/rust/tests/data/native_source_original_counted_filename_extent/651/request.json](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/651/request.json). SHA-256 `81bc1708a51928b502994bfd4ac0ff868b49c034da56567bf0191fbee7d786bf`. strict UTF-8 filenames: Original selected inputs and purpose; no answer is prescribed.
- `filename-651-input-manifest-json` (input): [runtime/rust/tests/data/native_source_original_counted_filename_extent/651/input-manifest.json](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/651/input-manifest.json). SHA-256 `2cb438e9a3d5a818514bc446bd935fc8ba5f632762b33a5366cc4ca334316f85`. strict UTF-8 filenames: Exact original input manifest bound by the actual receipts.
- `filename-651-results-json` (observation): [runtime/rust/tests/data/native_source_original_counted_filename_extent/651/results.json](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/651/results.json). SHA-256 `144cc60446b981329d6d5ba36f6e40adecc94014fbd667adfefcf4e6eae27a85`. strict UTF-8 filenames: Original six closed process summaries and immutable receipt hashes.
- `filename-651-compiler-json` (provider): [runtime/rust/tests/data/native_source_original_counted_filename_extent/651/compiler.json](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/651/compiler.json). SHA-256 `603e32bc18a3a8df74bd23721412d1baaeb1040e662bebfd8eeb3caf7c1c9fdb`. strict UTF-8 filenames: Measured compiler executable/version and exact launch metadata.
- `filename-651-library-version-results-json` (observation): [runtime/rust/tests/data/native_source_original_counted_filename_extent/651/library-version-results.json](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/651/library-version-results.json). SHA-256 `16e0cb88baa0aa54ff689097be3badd4bc697c96c03a341b70f0944574cdcfb5`. strict UTF-8 filenames: Six independent same-static-library diagnostic receipt joins.
- `source-window-0` (source-anchor): [runtime/rust/tests/data/native_source_original_counted_filename_extent/source-windows.json](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/source-windows.json). SHA-256 `e664c795bf35c879345d6c4f30da1a763abd75edfed8bfd48081e6c563619e97`. JSON pointer `/anchors/0/snippet`. NativeCreateNativeRep: complete physical-LF snippet joined to whole measured source and archive bytes.
- `source-window-1` (source-anchor): [runtime/rust/tests/data/native_source_original_counted_filename_extent/source-windows.json](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/source-windows.json). SHA-256 `e664c795bf35c879345d6c4f30da1a763abd75edfed8bfd48081e6c563619e97`. JSON pointer `/anchors/1/snippet`. Tcl_FSEvalFile: complete physical-LF snippet joined to whole measured source and archive bytes.
- `source-window-2` (source-anchor): [runtime/rust/tests/data/native_source_original_counted_filename_extent/source-windows.json](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/source-windows.json). SHA-256 `e664c795bf35c879345d6c4f30da1a763abd75edfed8bfd48081e6c563619e97`. JSON pointer `/anchors/2/snippet`. TclNativeCreateNativeRep: complete physical-LF snippet joined to whole measured source and archive bytes.
- `source-window-3` (source-anchor): [runtime/rust/tests/data/native_source_original_counted_filename_extent/source-windows.json](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/source-windows.json). SHA-256 `e664c795bf35c879345d6c4f30da1a763abd75edfed8bfd48081e6c563619e97`. JSON pointer `/anchors/3/snippet`. Tcl_FSEvalFileEx: complete physical-LF snippet joined to whole measured source and archive bytes.
- `source-window-4` (source-anchor): [runtime/rust/tests/data/native_source_original_counted_filename_extent/source-windows.json](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/source-windows.json). SHA-256 `e664c795bf35c879345d6c4f30da1a763abd75edfed8bfd48081e6c563619e97`. JSON pointer `/anchors/4/snippet`. TclNativeCreateNativeRep: complete physical-LF snippet joined to whole measured source and archive bytes.
- `source-window-5` (source-anchor): [runtime/rust/tests/data/native_source_original_counted_filename_extent/source-windows.json](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/source-windows.json). SHA-256 `e664c795bf35c879345d6c4f30da1a763abd75edfed8bfd48081e6c563619e97`. JSON pointer `/anchors/5/snippet`. TclNREvalFile: complete physical-LF snippet joined to whole measured source and archive bytes.
- `source-window-6` (source-anchor): [runtime/rust/tests/data/native_source_original_counted_filename_extent/source-windows.json](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/source-windows.json). SHA-256 `e664c795bf35c879345d6c4f30da1a763abd75edfed8bfd48081e6c563619e97`. JSON pointer `/anchors/6/snippet`. TclNativeCreateNativeRep: complete physical-LF snippet joined to whole measured source and archive bytes.
- `source-window-7` (source-anchor): [runtime/rust/tests/data/native_source_original_counted_filename_extent/source-windows.json](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/source-windows.json). SHA-256 `e664c795bf35c879345d6c4f30da1a763abd75edfed8bfd48081e6c563619e97`. JSON pointer `/anchors/7/snippet`. TclNREvalFile: complete physical-LF snippet joined to whole measured source and archive bytes.
- `source-window-8` (source-anchor): [runtime/rust/tests/data/native_source_original_counted_filename_extent/source-windows.json](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/source-windows.json). SHA-256 `e664c795bf35c879345d6c4f30da1a763abd75edfed8bfd48081e6c563619e97`. JSON pointer `/anchors/8/snippet`. TclNativeCreateNativeRep: complete physical-LF snippet joined to whole measured source and archive bytes.
- `source-window-9` (source-anchor): [runtime/rust/tests/data/native_source_original_counted_filename_extent/source-windows.json](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/source-windows.json). SHA-256 `e664c795bf35c879345d6c4f30da1a763abd75edfed8bfd48081e6c563619e97`. JSON pointer `/anchors/9/snippet`. TclNREvalFile: complete physical-LF snippet joined to whole measured source and archive bytes.
- `source-window-10` (source-anchor): [runtime/rust/tests/data/native_source_original_counted_filename_extent/source-windows.json](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/source-windows.json). SHA-256 `e664c795bf35c879345d6c4f30da1a763abd75edfed8bfd48081e6c563619e97`. JSON pointer `/anchors/10/snippet`. Jim_SourceCoreCommand: complete physical-LF snippet joined to whole measured source and archive bytes.
- `source-window-11` (source-anchor): [runtime/rust/tests/data/native_source_original_counted_filename_extent/source-windows.json](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/source-windows.json). SHA-256 `e664c795bf35c879345d6c4f30da1a763abd75edfed8bfd48081e6c563619e97`. JSON pointer `/anchors/11/snippet`. Jim_EvalFile: complete physical-LF snippet joined to whole measured source and archive bytes.
- `source-window-12` (source-anchor): [runtime/rust/tests/data/native_source_original_counted_filename_extent/source-windows.json](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/source-windows.json). SHA-256 `e664c795bf35c879345d6c4f30da1a763abd75edfed8bfd48081e6c563619e97`. JSON pointer `/anchors/12/snippet`. BinaryProc: complete physical-LF snippet joined to whole measured source and archive bytes.
- `source-window-13` (source-anchor): [runtime/rust/tests/data/native_source_original_counted_filename_extent/source-windows.json](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/source-windows.json). SHA-256 `e664c795bf35c879345d6c4f30da1a763abd75edfed8bfd48081e6c563619e97`. JSON pointer `/anchors/13/snippet`. TableFromUtfProc: complete physical-LF snippet joined to whole measured source and archive bytes.
- `source-window-14` (source-anchor): [runtime/rust/tests/data/native_source_original_counted_filename_extent/source-windows.json](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/source-windows.json). SHA-256 `e664c795bf35c879345d6c4f30da1a763abd75edfed8bfd48081e6c563619e97`. JSON pointer `/anchors/14/snippet`. BinaryProc: complete physical-LF snippet joined to whole measured source and archive bytes.
- `source-window-15` (source-anchor): [runtime/rust/tests/data/native_source_original_counted_filename_extent/source-windows.json](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/source-windows.json). SHA-256 `e664c795bf35c879345d6c4f30da1a763abd75edfed8bfd48081e6c563619e97`. JSON pointer `/anchors/15/snippet`. Iso88591FromUtfProc: complete physical-LF snippet joined to whole measured source and archive bytes.
- `source-window-16` (source-anchor): [runtime/rust/tests/data/native_source_original_counted_filename_extent/source-windows.json](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/source-windows.json). SHA-256 `e664c795bf35c879345d6c4f30da1a763abd75edfed8bfd48081e6c563619e97`. JSON pointer `/anchors/16/snippet`. BinaryProc: complete physical-LF snippet joined to whole measured source and archive bytes.
- `source-window-17` (source-anchor): [runtime/rust/tests/data/native_source_original_counted_filename_extent/source-windows.json](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/source-windows.json). SHA-256 `e664c795bf35c879345d6c4f30da1a763abd75edfed8bfd48081e6c563619e97`. JSON pointer `/anchors/17/snippet`. Iso88591FromUtfProc: complete physical-LF snippet joined to whole measured source and archive bytes.
- `source-window-18` (source-anchor): [runtime/rust/tests/data/native_source_original_counted_filename_extent/source-windows.json](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/source-windows.json). SHA-256 `e664c795bf35c879345d6c4f30da1a763abd75edfed8bfd48081e6c563619e97`. JSON pointer `/anchors/18/snippet`. BinaryProc: complete physical-LF snippet joined to whole measured source and archive bytes.
- `source-window-19` (source-anchor): [runtime/rust/tests/data/native_source_original_counted_filename_extent/source-windows.json](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/source-windows.json). SHA-256 `e664c795bf35c879345d6c4f30da1a763abd75edfed8bfd48081e6c563619e97`. JSON pointer `/anchors/19/snippet`. Iso88591FromUtfProc: complete physical-LF snippet joined to whole measured source and archive bytes.
- `source-window-20` (source-anchor): [runtime/rust/tests/data/native_source_original_counted_filename_extent/source-windows.json](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/source-windows.json). SHA-256 `e664c795bf35c879345d6c4f30da1a763abd75edfed8bfd48081e6c563619e97`. JSON pointer `/anchors/20/snippet`. BinaryProc: complete physical-LF snippet joined to whole measured source and archive bytes.
- `source-window-21` (source-anchor): [runtime/rust/tests/data/native_source_original_counted_filename_extent/source-windows.json](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/source-windows.json). SHA-256 `e664c795bf35c879345d6c4f30da1a763abd75edfed8bfd48081e6c563619e97`. JSON pointer `/anchors/21/snippet`. Iso88591FromUtfProc: complete physical-LF snippet joined to whole measured source and archive bytes.
- `source-window-22` (source-anchor): [runtime/rust/tests/data/native_source_original_counted_filename_extent/source-windows.json](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/source-windows.json). SHA-256 `e664c795bf35c879345d6c4f30da1a763abd75edfed8bfd48081e6c563619e97`. JSON pointer `/anchors/22/snippet`. TclInitEncodingSubsystem: complete physical-LF snippet joined to whole measured source and archive bytes.
- `source-window-23` (source-anchor): [runtime/rust/tests/data/native_source_original_counted_filename_extent/source-windows.json](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/source-windows.json). SHA-256 `e664c795bf35c879345d6c4f30da1a763abd75edfed8bfd48081e6c563619e97`. JSON pointer `/anchors/23/snippet`. TclInitEncodingSubsystem: complete physical-LF snippet joined to whole measured source and archive bytes.
- `source-window-24` (source-anchor): [runtime/rust/tests/data/native_source_original_counted_filename_extent/source-windows.json](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/source-windows.json). SHA-256 `e664c795bf35c879345d6c4f30da1a763abd75edfed8bfd48081e6c563619e97`. JSON pointer `/anchors/24/snippet`. TclInitEncodingSubsystem: complete physical-LF snippet joined to whole measured source and archive bytes.
- `source-window-25` (source-anchor): [runtime/rust/tests/data/native_source_original_counted_filename_extent/source-windows.json](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/source-windows.json). SHA-256 `e664c795bf35c879345d6c4f30da1a763abd75edfed8bfd48081e6c563619e97`. JSON pointer `/anchors/25/snippet`. Tcl_UtfToExternalDStringEx: complete physical-LF snippet joined to whole measured source and archive bytes.
- `source-window-26` (source-anchor): [runtime/rust/tests/data/native_source_original_counted_filename_extent/source-windows.json](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/source-windows.json). SHA-256 `e664c795bf35c879345d6c4f30da1a763abd75edfed8bfd48081e6c563619e97`. JSON pointer `/anchors/26/snippet`. Tcl_UtfToExternalDStringEx: complete physical-LF snippet joined to whole measured source and archive bytes.
- `source-window-27` (source-anchor): [runtime/rust/tests/data/native_source_original_counted_filename_extent/source-windows.json](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/source-windows.json). SHA-256 `e664c795bf35c879345d6c4f30da1a763abd75edfed8bfd48081e6c563619e97`. JSON pointer `/anchors/27/snippet`. UtfToUtfProc: complete physical-LF snippet joined to whole measured source and archive bytes.
- `source-window-28` (source-anchor): [runtime/rust/tests/data/native_source_original_counted_filename_extent/source-windows.json](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/source-windows.json). SHA-256 `e664c795bf35c879345d6c4f30da1a763abd75edfed8bfd48081e6c563619e97`. JSON pointer `/anchors/28/snippet`. UtfToUtfProc: complete physical-LF snippet joined to whole measured source and archive bytes.
- `source-window-29` (source-anchor): [runtime/rust/tests/data/native_source_original_counted_filename_extent/source-windows.json](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/source-windows.json). SHA-256 `e664c795bf35c879345d6c4f30da1a763abd75edfed8bfd48081e6c563619e97`. JSON pointer `/anchors/29/snippet`. UtfToUtfProc: complete physical-LF snippet joined to whole measured source and archive bytes.
- `source-window-30` (source-anchor): [runtime/rust/tests/data/native_source_original_counted_filename_extent/source-windows.json](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/source-windows.json). SHA-256 `e664c795bf35c879345d6c4f30da1a763abd75edfed8bfd48081e6c563619e97`. JSON pointer `/anchors/30/snippet`. UtfToUtfProc: complete physical-LF snippet joined to whole measured source and archive bytes.
- `source-window-31` (source-anchor): [runtime/rust/tests/data/native_source_original_counted_filename_extent/source-windows.json](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/source-windows.json). SHA-256 `e664c795bf35c879345d6c4f30da1a763abd75edfed8bfd48081e6c563619e97`. JSON pointer `/anchors/31/snippet`. UtfToUtfProc: complete physical-LF snippet joined to whole measured source and archive bytes.
- `tcl8.4-648-receipt-json` (provider): [runtime/rust/tests/data/native_source_original_counted_filename_extent/648/8.4.20/receipt.json](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/648/8.4.20/receipt.json). SHA-256 `17d37016e2a89bd913a7244a49e4887202e521e2eece1642732a9298203c7240`. filename extents: Exact compile/link/launch/environment, whole input pins and actual executable identity.
- `tcl8.4-648-execute-stdout` (observation): [runtime/rust/tests/data/native_source_original_counted_filename_extent/648/8.4.20/execute.stdout](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/648/8.4.20/execute.stdout). SHA-256 `4bcde5b45869ed8455cea8cc461edfe819b04384adbc30b7232cb9d0dbd59034`. filename extents: Unmodified full operation/query/header stream; selection-only errors provide no source completion.
- `tcl8.4-648-execute-stderr` (observation): [runtime/rust/tests/data/native_source_original_counted_filename_extent/648/8.4.20/execute.stderr](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/648/8.4.20/execute.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. filename extents: Unmodified original process stderr.
- `tcl8.4-648-observations-json` (observation): [runtime/rust/tests/data/native_source_original_counted_filename_extent/648/8.4.20/observations.json](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/648/8.4.20/observations.json). SHA-256 `aea99902f5081915f1b6e4bc129b9aa0c45ee56679049ecf45d208e0761bdd7e`. filename extents: Exact original projection of raw counted-byte output.
- `tcl8.4-648-cli-version-stdout` (observation): [runtime/rust/tests/data/native_source_original_counted_filename_extent/648/8.4.20/cli-version.stdout](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/648/8.4.20/cli-version.stdout). SHA-256 `c4a268983bec029286482df6caa3aaae81571c33d467c0a13f258e335da5954f`. filename extents: Actual independently receipt-bound CLI patchlevel.
- `tcl8.4-648-same-library-version-receipt-json` (provider): [runtime/rust/tests/data/native_source_original_counted_filename_extent/648/8.4.20/same-library-version/receipt.json](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/648/8.4.20/same-library-version/receipt.json). SHA-256 `12b9754bf31c10a259f22403149107acba2ab9c49e636d5d453a86e56c85a370`. filename extents: Actual independent diagnostic bound to the identical measured static library and full pins.
- `tcl8.4-648-same-library-version-execute-stdout` (observation): [runtime/rust/tests/data/native_source_original_counted_filename_extent/648/8.4.20/same-library-version/execute.stdout](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/648/8.4.20/same-library-version/execute.stdout). SHA-256 `ef18d09813e9c05fbd397154e6f786ea0221f2ce8981597665e917a08a5c5d7f`. filename extents: Linked-library patchlevel and C public version fields.
- `tcl8.4-650-receipt-json` (provider): [runtime/rust/tests/data/native_source_original_counted_filename_extent/650/8.4.20/receipt.json](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/650/8.4.20/receipt.json). SHA-256 `d59f6a78d0b82a0527b62eb809b14945a77754b2bf3d77d77b9414e706747192`. filesystem codecs: Exact compile/link/launch/environment, whole input pins and actual executable identity.
- `tcl8.4-650-execute-stdout` (observation): [runtime/rust/tests/data/native_source_original_counted_filename_extent/650/8.4.20/execute.stdout](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/650/8.4.20/execute.stdout). SHA-256 `0510e4bfe2558fd55d3922ef7c5551298976b0c7b22692b3dcd93d68cdf09f6a`. filesystem codecs: Unmodified full operation/query/header stream; selection-only errors provide no source completion.
- `tcl8.4-650-execute-stderr` (observation): [runtime/rust/tests/data/native_source_original_counted_filename_extent/650/8.4.20/execute.stderr](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/650/8.4.20/execute.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. filesystem codecs: Unmodified original process stderr.
- `tcl8.4-650-observations-json` (observation): [runtime/rust/tests/data/native_source_original_counted_filename_extent/650/8.4.20/observations.json](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/650/8.4.20/observations.json). SHA-256 `106d25701405f1b67e0baadddf4d9922f7f425b2a1b4442f751c0c561df151da`. filesystem codecs: Exact original projection of raw counted-byte output.
- `tcl8.4-650-cli-version-stdout` (observation): [runtime/rust/tests/data/native_source_original_counted_filename_extent/650/8.4.20/cli-version.stdout](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/650/8.4.20/cli-version.stdout). SHA-256 `c4a268983bec029286482df6caa3aaae81571c33d467c0a13f258e335da5954f`. filesystem codecs: Actual independently receipt-bound CLI patchlevel.
- `tcl8.4-650-same-library-version-receipt-json` (provider): [runtime/rust/tests/data/native_source_original_counted_filename_extent/650/8.4.20/same-library-version/receipt.json](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/650/8.4.20/same-library-version/receipt.json). SHA-256 `3898eed4c8a9eaa61b7a0b0591d423d172f1279dc24a9a4c27d9b181ff05c27d`. filesystem codecs: Actual independent diagnostic bound to the identical measured static library and full pins.
- `tcl8.4-650-same-library-version-execute-stdout` (observation): [runtime/rust/tests/data/native_source_original_counted_filename_extent/650/8.4.20/same-library-version/execute.stdout](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/650/8.4.20/same-library-version/execute.stdout). SHA-256 `ef18d09813e9c05fbd397154e6f786ea0221f2ce8981597665e917a08a5c5d7f`. filesystem codecs: Linked-library patchlevel and C public version fields.
- `tcl8.4-651-receipt-json` (provider): [runtime/rust/tests/data/native_source_original_counted_filename_extent/651/8.4.20/receipt.json](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/651/8.4.20/receipt.json). SHA-256 `90df3bfa661cbf6a74e407058e14e9fb3eeeec8d653b2cbc9db9c8ebaa10ddfe`. strict UTF-8 filenames: Exact compile/link/launch/environment, whole input pins and actual executable identity.
- `tcl8.4-651-execute-stdout` (observation): [runtime/rust/tests/data/native_source_original_counted_filename_extent/651/8.4.20/execute.stdout](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/651/8.4.20/execute.stdout). SHA-256 `5b0a8865a241cdb1e8f863038d1d3561b4887c4bb18fafd66334c6ce6d79ea46`. strict UTF-8 filenames: Unmodified full operation/query/header stream; selection-only errors provide no source completion.
- `tcl8.4-651-execute-stderr` (observation): [runtime/rust/tests/data/native_source_original_counted_filename_extent/651/8.4.20/execute.stderr](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/651/8.4.20/execute.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. strict UTF-8 filenames: Unmodified original process stderr.
- `tcl8.4-651-observations-json` (observation): [runtime/rust/tests/data/native_source_original_counted_filename_extent/651/8.4.20/observations.json](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/651/8.4.20/observations.json). SHA-256 `0537e74ee9876eeb2415167e26cca0df598ec365caf1b51dea4ed1225bbedfd8`. strict UTF-8 filenames: Exact original projection of raw counted-byte output.
- `tcl8.4-651-cli-version-stdout` (observation): [runtime/rust/tests/data/native_source_original_counted_filename_extent/651/8.4.20/cli-version.stdout](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/651/8.4.20/cli-version.stdout). SHA-256 `c4a268983bec029286482df6caa3aaae81571c33d467c0a13f258e335da5954f`. strict UTF-8 filenames: Actual independently receipt-bound CLI patchlevel.
- `tcl8.4-651-same-library-version-receipt-json` (provider): [runtime/rust/tests/data/native_source_original_counted_filename_extent/651/8.4.20/same-library-version/receipt.json](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/651/8.4.20/same-library-version/receipt.json). SHA-256 `64e1e3f6dbf4562a03ca040e62bd0e1ddb8a6e25b2b706583f795844e7efddfc`. strict UTF-8 filenames: Actual independent diagnostic bound to the identical measured static library and full pins.
- `tcl8.4-651-same-library-version-execute-stdout` (observation): [runtime/rust/tests/data/native_source_original_counted_filename_extent/651/8.4.20/same-library-version/execute.stdout](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/651/8.4.20/same-library-version/execute.stdout). SHA-256 `ef18d09813e9c05fbd397154e6f786ea0221f2ce8981597665e917a08a5c5d7f`. strict UTF-8 filenames: Linked-library patchlevel and C public version fields.
- `tcl8.5-648-receipt-json` (provider): [runtime/rust/tests/data/native_source_original_counted_filename_extent/648/8.5.19/receipt.json](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/648/8.5.19/receipt.json). SHA-256 `3d1c756c6a65dcbd853e938cb524c4a051c981f7cf00dfab330569d3e211ef12`. filename extents: Exact compile/link/launch/environment, whole input pins and actual executable identity.
- `tcl8.5-648-execute-stdout` (observation): [runtime/rust/tests/data/native_source_original_counted_filename_extent/648/8.5.19/execute.stdout](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/648/8.5.19/execute.stdout). SHA-256 `31ad7f2e14a07cb170a0c0ed2c6328932df9825a3767a3a9f693fa3b0e7c9282`. filename extents: Unmodified full operation/query/header stream; selection-only errors provide no source completion.
- `tcl8.5-648-execute-stderr` (observation): [runtime/rust/tests/data/native_source_original_counted_filename_extent/648/8.5.19/execute.stderr](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/648/8.5.19/execute.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. filename extents: Unmodified original process stderr.
- `tcl8.5-648-observations-json` (observation): [runtime/rust/tests/data/native_source_original_counted_filename_extent/648/8.5.19/observations.json](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/648/8.5.19/observations.json). SHA-256 `7d5efc749762e6b064e5473befe295187d67e5805dec4996fe1dbc4fd773cee4`. filename extents: Exact original projection of raw counted-byte output.
- `tcl8.5-648-cli-version-stdout` (observation): [runtime/rust/tests/data/native_source_original_counted_filename_extent/648/8.5.19/cli-version.stdout](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/648/8.5.19/cli-version.stdout). SHA-256 `6b58793c92a3db92926bdacbe0a52a520d417fe876aaa7cf8c29fed54a1fea71`. filename extents: Actual independently receipt-bound CLI patchlevel.
- `tcl8.5-648-same-library-version-receipt-json` (provider): [runtime/rust/tests/data/native_source_original_counted_filename_extent/648/8.5.19/same-library-version/receipt.json](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/648/8.5.19/same-library-version/receipt.json). SHA-256 `b0f5cd9730021ba64d54fefb590d5138a551bf7be8774eaad0955e0a72d66141`. filename extents: Actual independent diagnostic bound to the identical measured static library and full pins.
- `tcl8.5-648-same-library-version-execute-stdout` (observation): [runtime/rust/tests/data/native_source_original_counted_filename_extent/648/8.5.19/same-library-version/execute.stdout](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/648/8.5.19/same-library-version/execute.stdout). SHA-256 `1bd253489464d3ed255f286efdc8deb3768100a5b873e0a062f5026291fcf9bb`. filename extents: Linked-library patchlevel and C public version fields.
- `tcl8.5-650-receipt-json` (provider): [runtime/rust/tests/data/native_source_original_counted_filename_extent/650/8.5.19/receipt.json](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/650/8.5.19/receipt.json). SHA-256 `cb9e0552023c209accaf8a60bafd315c9d0f550067c850572d338dbd5b91fb80`. filesystem codecs: Exact compile/link/launch/environment, whole input pins and actual executable identity.
- `tcl8.5-650-execute-stdout` (observation): [runtime/rust/tests/data/native_source_original_counted_filename_extent/650/8.5.19/execute.stdout](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/650/8.5.19/execute.stdout). SHA-256 `4c9a756aa26955ab57bc2df298e01537ebb7629aef7a096f0d65500e6603bdd7`. filesystem codecs: Unmodified full operation/query/header stream; selection-only errors provide no source completion.
- `tcl8.5-650-execute-stderr` (observation): [runtime/rust/tests/data/native_source_original_counted_filename_extent/650/8.5.19/execute.stderr](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/650/8.5.19/execute.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. filesystem codecs: Unmodified original process stderr.
- `tcl8.5-650-observations-json` (observation): [runtime/rust/tests/data/native_source_original_counted_filename_extent/650/8.5.19/observations.json](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/650/8.5.19/observations.json). SHA-256 `d0d6e634ec53715127473378316a7e219a24af746edcf580c311405ffe17e641`. filesystem codecs: Exact original projection of raw counted-byte output.
- `tcl8.5-650-cli-version-stdout` (observation): [runtime/rust/tests/data/native_source_original_counted_filename_extent/650/8.5.19/cli-version.stdout](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/650/8.5.19/cli-version.stdout). SHA-256 `6b58793c92a3db92926bdacbe0a52a520d417fe876aaa7cf8c29fed54a1fea71`. filesystem codecs: Actual independently receipt-bound CLI patchlevel.
- `tcl8.5-650-same-library-version-receipt-json` (provider): [runtime/rust/tests/data/native_source_original_counted_filename_extent/650/8.5.19/same-library-version/receipt.json](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/650/8.5.19/same-library-version/receipt.json). SHA-256 `bf7b3dd50bed7a4b4129f18d20555df758894ccc8fe050eb454bb72b7ab8c9e5`. filesystem codecs: Actual independent diagnostic bound to the identical measured static library and full pins.
- `tcl8.5-650-same-library-version-execute-stdout` (observation): [runtime/rust/tests/data/native_source_original_counted_filename_extent/650/8.5.19/same-library-version/execute.stdout](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/650/8.5.19/same-library-version/execute.stdout). SHA-256 `1bd253489464d3ed255f286efdc8deb3768100a5b873e0a062f5026291fcf9bb`. filesystem codecs: Linked-library patchlevel and C public version fields.
- `tcl8.5-651-receipt-json` (provider): [runtime/rust/tests/data/native_source_original_counted_filename_extent/651/8.5.19/receipt.json](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/651/8.5.19/receipt.json). SHA-256 `a0a2638c1633d71a157ee4842cda6670e9426908ec39073d2f4b0ca6635a35be`. strict UTF-8 filenames: Exact compile/link/launch/environment, whole input pins and actual executable identity.
- `tcl8.5-651-execute-stdout` (observation): [runtime/rust/tests/data/native_source_original_counted_filename_extent/651/8.5.19/execute.stdout](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/651/8.5.19/execute.stdout). SHA-256 `a911a629eb85de1377732720a47f3a2782427bde22b68e35b05e566d160bea97`. strict UTF-8 filenames: Unmodified full operation/query/header stream; selection-only errors provide no source completion.
- `tcl8.5-651-execute-stderr` (observation): [runtime/rust/tests/data/native_source_original_counted_filename_extent/651/8.5.19/execute.stderr](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/651/8.5.19/execute.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. strict UTF-8 filenames: Unmodified original process stderr.
- `tcl8.5-651-observations-json` (observation): [runtime/rust/tests/data/native_source_original_counted_filename_extent/651/8.5.19/observations.json](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/651/8.5.19/observations.json). SHA-256 `f4ea5e4031026a1c18621d39b276ddc491843bf3a62eae392b890cbb12ef9502`. strict UTF-8 filenames: Exact original projection of raw counted-byte output.
- `tcl8.5-651-cli-version-stdout` (observation): [runtime/rust/tests/data/native_source_original_counted_filename_extent/651/8.5.19/cli-version.stdout](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/651/8.5.19/cli-version.stdout). SHA-256 `6b58793c92a3db92926bdacbe0a52a520d417fe876aaa7cf8c29fed54a1fea71`. strict UTF-8 filenames: Actual independently receipt-bound CLI patchlevel.
- `tcl8.5-651-same-library-version-receipt-json` (provider): [runtime/rust/tests/data/native_source_original_counted_filename_extent/651/8.5.19/same-library-version/receipt.json](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/651/8.5.19/same-library-version/receipt.json). SHA-256 `9860a0f4fc054957f65ecf1ad4cb38fc7c61e051f058c192d6f27cec3f5ba8c4`. strict UTF-8 filenames: Actual independent diagnostic bound to the identical measured static library and full pins.
- `tcl8.5-651-same-library-version-execute-stdout` (observation): [runtime/rust/tests/data/native_source_original_counted_filename_extent/651/8.5.19/same-library-version/execute.stdout](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/651/8.5.19/same-library-version/execute.stdout). SHA-256 `1bd253489464d3ed255f286efdc8deb3768100a5b873e0a062f5026291fcf9bb`. strict UTF-8 filenames: Linked-library patchlevel and C public version fields.
- `tcl8.6-648-receipt-json` (provider): [runtime/rust/tests/data/native_source_original_counted_filename_extent/648/8.6.18/receipt.json](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/648/8.6.18/receipt.json). SHA-256 `7f3fdf2c84a93106323d8ee8358e3eae54c19a86d7f0517454563a5b07885553`. filename extents: Exact compile/link/launch/environment, whole input pins and actual executable identity.
- `tcl8.6-648-execute-stdout` (observation): [runtime/rust/tests/data/native_source_original_counted_filename_extent/648/8.6.18/execute.stdout](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/648/8.6.18/execute.stdout). SHA-256 `61ec81465e80b4cb06911915fa2c60cf736b4d596f2de50bc540ef1d6e43f2ef`. filename extents: Unmodified full operation/query/header stream; selection-only errors provide no source completion.
- `tcl8.6-648-execute-stderr` (observation): [runtime/rust/tests/data/native_source_original_counted_filename_extent/648/8.6.18/execute.stderr](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/648/8.6.18/execute.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. filename extents: Unmodified original process stderr.
- `tcl8.6-648-observations-json` (observation): [runtime/rust/tests/data/native_source_original_counted_filename_extent/648/8.6.18/observations.json](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/648/8.6.18/observations.json). SHA-256 `92885463bd82b653623f867ba5ffb2b01b3de7daa60bfd722205bcf6dc396874`. filename extents: Exact original projection of raw counted-byte output.
- `tcl8.6-648-cli-version-stdout` (observation): [runtime/rust/tests/data/native_source_original_counted_filename_extent/648/8.6.18/cli-version.stdout](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/648/8.6.18/cli-version.stdout). SHA-256 `7523e113df6c321e18099a10ec22086db746e2fcc31a879e16d961e5d3878d01`. filename extents: Actual independently receipt-bound CLI patchlevel.
- `tcl8.6-648-same-library-version-receipt-json` (provider): [runtime/rust/tests/data/native_source_original_counted_filename_extent/648/8.6.18/same-library-version/receipt.json](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/648/8.6.18/same-library-version/receipt.json). SHA-256 `2c1e0914427025dc14ec163104b4a6747fe2065012b628139a05329d4d5984dd`. filename extents: Actual independent diagnostic bound to the identical measured static library and full pins.
- `tcl8.6-648-same-library-version-execute-stdout` (observation): [runtime/rust/tests/data/native_source_original_counted_filename_extent/648/8.6.18/same-library-version/execute.stdout](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/648/8.6.18/same-library-version/execute.stdout). SHA-256 `dd12d5dc76bfc80ae3033ac87b1932541f275a888adf9e03dabbcb5878b0df8c`. filename extents: Linked-library patchlevel and C public version fields.
- `tcl8.6-650-receipt-json` (provider): [runtime/rust/tests/data/native_source_original_counted_filename_extent/650/8.6.18/receipt.json](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/650/8.6.18/receipt.json). SHA-256 `9ff6206f51cc597c83cd4bb4315e86dbb9e641f2fc66a2867144a5d5c5ea97ae`. filesystem codecs: Exact compile/link/launch/environment, whole input pins and actual executable identity.
- `tcl8.6-650-execute-stdout` (observation): [runtime/rust/tests/data/native_source_original_counted_filename_extent/650/8.6.18/execute.stdout](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/650/8.6.18/execute.stdout). SHA-256 `06da2d9794c5761ee4e4d183c79be374391d15b6c448023039aeb8e8cd98594d`. filesystem codecs: Unmodified full operation/query/header stream; selection-only errors provide no source completion.
- `tcl8.6-650-execute-stderr` (observation): [runtime/rust/tests/data/native_source_original_counted_filename_extent/650/8.6.18/execute.stderr](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/650/8.6.18/execute.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. filesystem codecs: Unmodified original process stderr.
- `tcl8.6-650-observations-json` (observation): [runtime/rust/tests/data/native_source_original_counted_filename_extent/650/8.6.18/observations.json](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/650/8.6.18/observations.json). SHA-256 `4b1fffb25daf57819166b0d06c65e06a91fcc03d203fcb93160a6230413bd944`. filesystem codecs: Exact original projection of raw counted-byte output.
- `tcl8.6-650-cli-version-stdout` (observation): [runtime/rust/tests/data/native_source_original_counted_filename_extent/650/8.6.18/cli-version.stdout](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/650/8.6.18/cli-version.stdout). SHA-256 `7523e113df6c321e18099a10ec22086db746e2fcc31a879e16d961e5d3878d01`. filesystem codecs: Actual independently receipt-bound CLI patchlevel.
- `tcl8.6-650-same-library-version-receipt-json` (provider): [runtime/rust/tests/data/native_source_original_counted_filename_extent/650/8.6.18/same-library-version/receipt.json](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/650/8.6.18/same-library-version/receipt.json). SHA-256 `f1823499e29f94ec35e8e55a6b0388a2c503cfa4ea5f9efdcc781d520ca87825`. filesystem codecs: Actual independent diagnostic bound to the identical measured static library and full pins.
- `tcl8.6-650-same-library-version-execute-stdout` (observation): [runtime/rust/tests/data/native_source_original_counted_filename_extent/650/8.6.18/same-library-version/execute.stdout](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/650/8.6.18/same-library-version/execute.stdout). SHA-256 `dd12d5dc76bfc80ae3033ac87b1932541f275a888adf9e03dabbcb5878b0df8c`. filesystem codecs: Linked-library patchlevel and C public version fields.
- `tcl8.6-651-receipt-json` (provider): [runtime/rust/tests/data/native_source_original_counted_filename_extent/651/8.6.18/receipt.json](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/651/8.6.18/receipt.json). SHA-256 `fe728cd58a9163e9705fca31ed67cccd0b5652ffee14b37e3c0832ee59d33e38`. strict UTF-8 filenames: Exact compile/link/launch/environment, whole input pins and actual executable identity.
- `tcl8.6-651-execute-stdout` (observation): [runtime/rust/tests/data/native_source_original_counted_filename_extent/651/8.6.18/execute.stdout](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/651/8.6.18/execute.stdout). SHA-256 `986752b17729d0ceaa005e0f13e99ecdb24cee9d39b16a944f76ffab92eecf05`. strict UTF-8 filenames: Unmodified full operation/query/header stream; selection-only errors provide no source completion.
- `tcl8.6-651-execute-stderr` (observation): [runtime/rust/tests/data/native_source_original_counted_filename_extent/651/8.6.18/execute.stderr](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/651/8.6.18/execute.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. strict UTF-8 filenames: Unmodified original process stderr.
- `tcl8.6-651-observations-json` (observation): [runtime/rust/tests/data/native_source_original_counted_filename_extent/651/8.6.18/observations.json](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/651/8.6.18/observations.json). SHA-256 `fa3db8e6c7b1191f1892ffe580d9fc1e9a3848a1d2cc58a5f0995cdb7ee570e1`. strict UTF-8 filenames: Exact original projection of raw counted-byte output.
- `tcl8.6-651-cli-version-stdout` (observation): [runtime/rust/tests/data/native_source_original_counted_filename_extent/651/8.6.18/cli-version.stdout](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/651/8.6.18/cli-version.stdout). SHA-256 `7523e113df6c321e18099a10ec22086db746e2fcc31a879e16d961e5d3878d01`. strict UTF-8 filenames: Actual independently receipt-bound CLI patchlevel.
- `tcl8.6-651-same-library-version-receipt-json` (provider): [runtime/rust/tests/data/native_source_original_counted_filename_extent/651/8.6.18/same-library-version/receipt.json](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/651/8.6.18/same-library-version/receipt.json). SHA-256 `5504033477049dda81d8e93139583547b365c46dbfffd7e0719de08aa83ad6fd`. strict UTF-8 filenames: Actual independent diagnostic bound to the identical measured static library and full pins.
- `tcl8.6-651-same-library-version-execute-stdout` (observation): [runtime/rust/tests/data/native_source_original_counted_filename_extent/651/8.6.18/same-library-version/execute.stdout](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/651/8.6.18/same-library-version/execute.stdout). SHA-256 `dd12d5dc76bfc80ae3033ac87b1932541f275a888adf9e03dabbcb5878b0df8c`. strict UTF-8 filenames: Linked-library patchlevel and C public version fields.
- `tcl9.0-648-receipt-json` (provider): [runtime/rust/tests/data/native_source_original_counted_filename_extent/648/9.0.4/receipt.json](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/648/9.0.4/receipt.json). SHA-256 `aeb35fddd81d46855997fd5f4dcb19f40d0e7e3b3db3082a3efcc2a22ba8b657`. filename extents: Exact compile/link/launch/environment, whole input pins and actual executable identity.
- `tcl9.0-648-execute-stdout` (observation): [runtime/rust/tests/data/native_source_original_counted_filename_extent/648/9.0.4/execute.stdout](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/648/9.0.4/execute.stdout). SHA-256 `90e8576af641deea99ba292b49655782ac7cbb2f81a13361567a34a0c619e443`. filename extents: Unmodified full operation/query/header stream; selection-only errors provide no source completion.
- `tcl9.0-648-execute-stderr` (observation): [runtime/rust/tests/data/native_source_original_counted_filename_extent/648/9.0.4/execute.stderr](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/648/9.0.4/execute.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. filename extents: Unmodified original process stderr.
- `tcl9.0-648-observations-json` (observation): [runtime/rust/tests/data/native_source_original_counted_filename_extent/648/9.0.4/observations.json](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/648/9.0.4/observations.json). SHA-256 `7f0e3e602e2711163edbf77b96a6bc252cad468037a825487306ffd71154ef5c`. filename extents: Exact original projection of raw counted-byte output.
- `tcl9.0-648-cli-version-stdout` (observation): [runtime/rust/tests/data/native_source_original_counted_filename_extent/648/9.0.4/cli-version.stdout](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/648/9.0.4/cli-version.stdout). SHA-256 `b83920bac1b2d47c3840f588f95bee9dc69b85e26117f471541cdf85d7da52a2`. filename extents: Actual independently receipt-bound CLI patchlevel.
- `tcl9.0-648-same-library-version-receipt-json` (provider): [runtime/rust/tests/data/native_source_original_counted_filename_extent/648/9.0.4/same-library-version/receipt.json](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/648/9.0.4/same-library-version/receipt.json). SHA-256 `117525df0fe09717d0375277ebb2aa1be18725a191a353b25444ba20fb191f76`. filename extents: Actual independent diagnostic bound to the identical measured static library and full pins.
- `tcl9.0-648-same-library-version-execute-stdout` (observation): [runtime/rust/tests/data/native_source_original_counted_filename_extent/648/9.0.4/same-library-version/execute.stdout](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/648/9.0.4/same-library-version/execute.stdout). SHA-256 `d016aed0da5636f83bfe6d76f7c26b06e30ce7951203b5d3543001780eb51a2c`. filename extents: Linked-library patchlevel and C public version fields.
- `tcl9.0-650-receipt-json` (provider): [runtime/rust/tests/data/native_source_original_counted_filename_extent/650/9.0.4/receipt.json](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/650/9.0.4/receipt.json). SHA-256 `338eb013cdd2ec834d6c040d22483c9f85208fa5523a2f04500c35cd6148ad48`. filesystem codecs: Exact compile/link/launch/environment, whole input pins and actual executable identity.
- `tcl9.0-650-execute-stdout` (observation): [runtime/rust/tests/data/native_source_original_counted_filename_extent/650/9.0.4/execute.stdout](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/650/9.0.4/execute.stdout). SHA-256 `94dc6065e55165b87a69d1e3459dee0a1ac9676cc98dc54b9a52fc6e9915ba95`. filesystem codecs: Unmodified full operation/query/header stream; selection-only errors provide no source completion.
- `tcl9.0-650-execute-stderr` (observation): [runtime/rust/tests/data/native_source_original_counted_filename_extent/650/9.0.4/execute.stderr](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/650/9.0.4/execute.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. filesystem codecs: Unmodified original process stderr.
- `tcl9.0-650-observations-json` (observation): [runtime/rust/tests/data/native_source_original_counted_filename_extent/650/9.0.4/observations.json](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/650/9.0.4/observations.json). SHA-256 `145087fef387fd6fd1e4b4f126ff6f710986d883549b23fc4ebf956b0dd208d8`. filesystem codecs: Exact original projection of raw counted-byte output.
- `tcl9.0-650-cli-version-stdout` (observation): [runtime/rust/tests/data/native_source_original_counted_filename_extent/650/9.0.4/cli-version.stdout](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/650/9.0.4/cli-version.stdout). SHA-256 `b83920bac1b2d47c3840f588f95bee9dc69b85e26117f471541cdf85d7da52a2`. filesystem codecs: Actual independently receipt-bound CLI patchlevel.
- `tcl9.0-650-same-library-version-receipt-json` (provider): [runtime/rust/tests/data/native_source_original_counted_filename_extent/650/9.0.4/same-library-version/receipt.json](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/650/9.0.4/same-library-version/receipt.json). SHA-256 `19b4c52725f4b2de474016a13360bbcfa9da3d5e89ee6e9720f7e33b9acda642`. filesystem codecs: Actual independent diagnostic bound to the identical measured static library and full pins.
- `tcl9.0-650-same-library-version-execute-stdout` (observation): [runtime/rust/tests/data/native_source_original_counted_filename_extent/650/9.0.4/same-library-version/execute.stdout](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/650/9.0.4/same-library-version/execute.stdout). SHA-256 `d016aed0da5636f83bfe6d76f7c26b06e30ce7951203b5d3543001780eb51a2c`. filesystem codecs: Linked-library patchlevel and C public version fields.
- `tcl9.0-651-receipt-json` (provider): [runtime/rust/tests/data/native_source_original_counted_filename_extent/651/9.0.4/receipt.json](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/651/9.0.4/receipt.json). SHA-256 `4109633ae8f434d50b01a8c925f74a203ceafa8c26429dfd3f5e108b27148308`. strict UTF-8 filenames: Exact compile/link/launch/environment, whole input pins and actual executable identity.
- `tcl9.0-651-execute-stdout` (observation): [runtime/rust/tests/data/native_source_original_counted_filename_extent/651/9.0.4/execute.stdout](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/651/9.0.4/execute.stdout). SHA-256 `13ae65b442be83c3c8d8725a2c7ba8088aa967e8fdf2badd863cc0484c6919f6`. strict UTF-8 filenames: Unmodified full operation/query/header stream; selection-only errors provide no source completion.
- `tcl9.0-651-execute-stderr` (observation): [runtime/rust/tests/data/native_source_original_counted_filename_extent/651/9.0.4/execute.stderr](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/651/9.0.4/execute.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. strict UTF-8 filenames: Unmodified original process stderr.
- `tcl9.0-651-observations-json` (observation): [runtime/rust/tests/data/native_source_original_counted_filename_extent/651/9.0.4/observations.json](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/651/9.0.4/observations.json). SHA-256 `3b95be806347b1249ab80afdda18621b8c149465eecfa55afbb1abc94add58da`. strict UTF-8 filenames: Exact original projection of raw counted-byte output.
- `tcl9.0-651-cli-version-stdout` (observation): [runtime/rust/tests/data/native_source_original_counted_filename_extent/651/9.0.4/cli-version.stdout](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/651/9.0.4/cli-version.stdout). SHA-256 `b83920bac1b2d47c3840f588f95bee9dc69b85e26117f471541cdf85d7da52a2`. strict UTF-8 filenames: Actual independently receipt-bound CLI patchlevel.
- `tcl9.0-651-same-library-version-receipt-json` (provider): [runtime/rust/tests/data/native_source_original_counted_filename_extent/651/9.0.4/same-library-version/receipt.json](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/651/9.0.4/same-library-version/receipt.json). SHA-256 `5b6e0ab3b941850f87655fcfcaa5861857dac03d0df1b59921e37a1a5fbdf77d`. strict UTF-8 filenames: Actual independent diagnostic bound to the identical measured static library and full pins.
- `tcl9.0-651-same-library-version-execute-stdout` (observation): [runtime/rust/tests/data/native_source_original_counted_filename_extent/651/9.0.4/same-library-version/execute.stdout](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/651/9.0.4/same-library-version/execute.stdout). SHA-256 `d016aed0da5636f83bfe6d76f7c26b06e30ce7951203b5d3543001780eb51a2c`. strict UTF-8 filenames: Linked-library patchlevel and C public version fields.
- `tcl9.1-648-receipt-json` (provider): [runtime/rust/tests/data/native_source_original_counted_filename_extent/648/9.1.0/receipt.json](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/648/9.1.0/receipt.json). SHA-256 `f8e6fd94a47c461cdfc5fc0918e47e845428d5ccc470f940c8477acd0211ac9d`. filename extents: Exact compile/link/launch/environment, whole input pins and actual executable identity.
- `tcl9.1-648-execute-stdout` (observation): [runtime/rust/tests/data/native_source_original_counted_filename_extent/648/9.1.0/execute.stdout](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/648/9.1.0/execute.stdout). SHA-256 `2ab9ab6fc52e2196857291878465f8e1672bd08c197b816733c8780440bba2c3`. filename extents: Unmodified full operation/query/header stream; selection-only errors provide no source completion.
- `tcl9.1-648-execute-stderr` (observation): [runtime/rust/tests/data/native_source_original_counted_filename_extent/648/9.1.0/execute.stderr](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/648/9.1.0/execute.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. filename extents: Unmodified original process stderr.
- `tcl9.1-648-observations-json` (observation): [runtime/rust/tests/data/native_source_original_counted_filename_extent/648/9.1.0/observations.json](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/648/9.1.0/observations.json). SHA-256 `9bb03feecdda9f66686ecce3a7c7a9910844f920fb46f90b9336b715b8030724`. filename extents: Exact original projection of raw counted-byte output.
- `tcl9.1-648-cli-version-stdout` (observation): [runtime/rust/tests/data/native_source_original_counted_filename_extent/648/9.1.0/cli-version.stdout](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/648/9.1.0/cli-version.stdout). SHA-256 `077ee5b3a3a7fc2622ceff6466fea0c28f3fb08b24653dcc7f4baab29b9aef36`. filename extents: Actual independently receipt-bound CLI patchlevel.
- `tcl9.1-648-same-library-version-receipt-json` (provider): [runtime/rust/tests/data/native_source_original_counted_filename_extent/648/9.1.0/same-library-version/receipt.json](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/648/9.1.0/same-library-version/receipt.json). SHA-256 `cdc91356e46d0e7e4d830c283866f21c920c9571bdc40786ed03fbfd7058605a`. filename extents: Actual independent diagnostic bound to the identical measured static library and full pins.
- `tcl9.1-648-same-library-version-execute-stdout` (observation): [runtime/rust/tests/data/native_source_original_counted_filename_extent/648/9.1.0/same-library-version/execute.stdout](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/648/9.1.0/same-library-version/execute.stdout). SHA-256 `b225f15b75641002abfa4f29d4d1be66051123c85bc897b9a7234b22317ea071`. filename extents: Linked-library patchlevel and C public version fields.
- `tcl9.1-650-receipt-json` (provider): [runtime/rust/tests/data/native_source_original_counted_filename_extent/650/9.1.0/receipt.json](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/650/9.1.0/receipt.json). SHA-256 `919916557d6b9b8815a24e60ff4ef5d7da92664070f0825f10d45d1ec7348b9b`. filesystem codecs: Exact compile/link/launch/environment, whole input pins and actual executable identity.
- `tcl9.1-650-execute-stdout` (observation): [runtime/rust/tests/data/native_source_original_counted_filename_extent/650/9.1.0/execute.stdout](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/650/9.1.0/execute.stdout). SHA-256 `1cb28b22b41f64b347973569de55bf77c7a177b24196fcb6e234abb96f97d902`. filesystem codecs: Unmodified full operation/query/header stream; selection-only errors provide no source completion.
- `tcl9.1-650-execute-stderr` (observation): [runtime/rust/tests/data/native_source_original_counted_filename_extent/650/9.1.0/execute.stderr](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/650/9.1.0/execute.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. filesystem codecs: Unmodified original process stderr.
- `tcl9.1-650-observations-json` (observation): [runtime/rust/tests/data/native_source_original_counted_filename_extent/650/9.1.0/observations.json](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/650/9.1.0/observations.json). SHA-256 `ad832036a233126dad9251fc814d59534f1f8dddf5a87abb5f323015c921e84c`. filesystem codecs: Exact original projection of raw counted-byte output.
- `tcl9.1-650-cli-version-stdout` (observation): [runtime/rust/tests/data/native_source_original_counted_filename_extent/650/9.1.0/cli-version.stdout](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/650/9.1.0/cli-version.stdout). SHA-256 `077ee5b3a3a7fc2622ceff6466fea0c28f3fb08b24653dcc7f4baab29b9aef36`. filesystem codecs: Actual independently receipt-bound CLI patchlevel.
- `tcl9.1-650-same-library-version-receipt-json` (provider): [runtime/rust/tests/data/native_source_original_counted_filename_extent/650/9.1.0/same-library-version/receipt.json](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/650/9.1.0/same-library-version/receipt.json). SHA-256 `744843c36d86f98a17d50a1e05282149169f4bc4446dd64192c59095af68301e`. filesystem codecs: Actual independent diagnostic bound to the identical measured static library and full pins.
- `tcl9.1-650-same-library-version-execute-stdout` (observation): [runtime/rust/tests/data/native_source_original_counted_filename_extent/650/9.1.0/same-library-version/execute.stdout](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/650/9.1.0/same-library-version/execute.stdout). SHA-256 `b225f15b75641002abfa4f29d4d1be66051123c85bc897b9a7234b22317ea071`. filesystem codecs: Linked-library patchlevel and C public version fields.
- `tcl9.1-651-receipt-json` (provider): [runtime/rust/tests/data/native_source_original_counted_filename_extent/651/9.1.0/receipt.json](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/651/9.1.0/receipt.json). SHA-256 `80ec60552a5d6885ef5c81277d93deb56479f1acc50bce5bb4e8959282306308`. strict UTF-8 filenames: Exact compile/link/launch/environment, whole input pins and actual executable identity.
- `tcl9.1-651-execute-stdout` (observation): [runtime/rust/tests/data/native_source_original_counted_filename_extent/651/9.1.0/execute.stdout](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/651/9.1.0/execute.stdout). SHA-256 `8d0b7cc828a7b4521f658fb90b8394dbd26b430921dbdaa4c0f65b9387a3d888`. strict UTF-8 filenames: Unmodified full operation/query/header stream; selection-only errors provide no source completion.
- `tcl9.1-651-execute-stderr` (observation): [runtime/rust/tests/data/native_source_original_counted_filename_extent/651/9.1.0/execute.stderr](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/651/9.1.0/execute.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. strict UTF-8 filenames: Unmodified original process stderr.
- `tcl9.1-651-observations-json` (observation): [runtime/rust/tests/data/native_source_original_counted_filename_extent/651/9.1.0/observations.json](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/651/9.1.0/observations.json). SHA-256 `90fe1b80929b63b0a3060815c5bcf64b8a4e0e3d920432740424ee2589b178e9`. strict UTF-8 filenames: Exact original projection of raw counted-byte output.
- `tcl9.1-651-cli-version-stdout` (observation): [runtime/rust/tests/data/native_source_original_counted_filename_extent/651/9.1.0/cli-version.stdout](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/651/9.1.0/cli-version.stdout). SHA-256 `077ee5b3a3a7fc2622ceff6466fea0c28f3fb08b24653dcc7f4baab29b9aef36`. strict UTF-8 filenames: Actual independently receipt-bound CLI patchlevel.
- `tcl9.1-651-same-library-version-receipt-json` (provider): [runtime/rust/tests/data/native_source_original_counted_filename_extent/651/9.1.0/same-library-version/receipt.json](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/651/9.1.0/same-library-version/receipt.json). SHA-256 `ba9d5e3b3af26b6d7f2ecbddcbc49c14b0b7108567c9aa951627f1bed57e99d8`. strict UTF-8 filenames: Actual independent diagnostic bound to the identical measured static library and full pins.
- `tcl9.1-651-same-library-version-execute-stdout` (observation): [runtime/rust/tests/data/native_source_original_counted_filename_extent/651/9.1.0/same-library-version/execute.stdout](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/651/9.1.0/same-library-version/execute.stdout). SHA-256 `b225f15b75641002abfa4f29d4d1be66051123c85bc897b9a7234b22317ea071`. strict UTF-8 filenames: Linked-library patchlevel and C public version fields.
- `jim-648-receipt-json` (provider): [runtime/rust/tests/data/native_source_original_counted_filename_extent/648/Jim/receipt.json](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/648/Jim/receipt.json). SHA-256 `31abeddaf61cc407be1534e1b00e31760aaf2d5a28c63934924d3e7dd276d42b`. filename extents: Exact compile/link/launch/environment, whole input pins and actual executable identity.
- `jim-648-execute-stdout` (observation): [runtime/rust/tests/data/native_source_original_counted_filename_extent/648/Jim/execute.stdout](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/648/Jim/execute.stdout). SHA-256 `041b931dc26ce9b25306f9a4482557cfeaa9f40a0edbc8a6dabc57b6b70eb0c4`. filename extents: Unmodified full operation/query/header stream; selection-only errors provide no source completion.
- `jim-648-execute-stderr` (observation): [runtime/rust/tests/data/native_source_original_counted_filename_extent/648/Jim/execute.stderr](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/648/Jim/execute.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. filename extents: Unmodified original process stderr.
- `jim-648-observations-json` (observation): [runtime/rust/tests/data/native_source_original_counted_filename_extent/648/Jim/observations.json](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/648/Jim/observations.json). SHA-256 `2ba1b7c7cea33e4a5de74c25da79fbc2c479561a21a682e66552f62494d538ff`. filename extents: Exact original projection of raw counted-byte output.
- `jim-648-cli-version-stdout` (observation): [runtime/rust/tests/data/native_source_original_counted_filename_extent/648/Jim/cli-version.stdout](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/648/Jim/cli-version.stdout). SHA-256 `f9752899fa012cf994a528e0250402609067a6f67a144a1172644a9329386803`. filename extents: Actual independently receipt-bound CLI patchlevel.
- `jim-648-same-library-version-receipt-json` (provider): [runtime/rust/tests/data/native_source_original_counted_filename_extent/648/Jim/same-library-version/receipt.json](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/648/Jim/same-library-version/receipt.json). SHA-256 `c864c963e4e643b00175f9cdbee64b1fc529d3f8419b6f20aa0ff5fbd2095a34`. filename extents: Actual independent diagnostic bound to the identical measured static library and full pins.
- `jim-648-same-library-version-execute-stdout` (observation): [runtime/rust/tests/data/native_source_original_counted_filename_extent/648/Jim/same-library-version/execute.stdout](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/648/Jim/same-library-version/execute.stdout). SHA-256 `d7ce62ac8c979a247ad4edf0a43c9133dc8b547cdfc8e5b540fccd9bbe848166`. filename extents: Linked-library patchlevel and C public version fields.
- `jim-650-receipt-json` (provider): [runtime/rust/tests/data/native_source_original_counted_filename_extent/650/Jim/receipt.json](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/650/Jim/receipt.json). SHA-256 `77faeceb1c5c38b541663de976fa05079a5db9994269d2d35084395d637d14fc`. filesystem codecs: Exact compile/link/launch/environment, whole input pins and actual executable identity.
- `jim-650-execute-stdout` (observation): [runtime/rust/tests/data/native_source_original_counted_filename_extent/650/Jim/execute.stdout](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/650/Jim/execute.stdout). SHA-256 `140a7cc48fe70bef1be4c51c6cb46308a000806d0bc2e8eea776bb9cbd0d3aac`. filesystem codecs: Unmodified full operation/query/header stream; selection-only errors provide no source completion.
- `jim-650-execute-stderr` (observation): [runtime/rust/tests/data/native_source_original_counted_filename_extent/650/Jim/execute.stderr](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/650/Jim/execute.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. filesystem codecs: Unmodified original process stderr.
- `jim-650-observations-json` (observation): [runtime/rust/tests/data/native_source_original_counted_filename_extent/650/Jim/observations.json](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/650/Jim/observations.json). SHA-256 `da4812d1b4a5a4fad53c529d92b54ad3215d16dad737df09c29e23ab77696ddf`. filesystem codecs: Exact original projection of raw counted-byte output.
- `jim-650-cli-version-stdout` (observation): [runtime/rust/tests/data/native_source_original_counted_filename_extent/650/Jim/cli-version.stdout](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/650/Jim/cli-version.stdout). SHA-256 `f9752899fa012cf994a528e0250402609067a6f67a144a1172644a9329386803`. filesystem codecs: Actual independently receipt-bound CLI patchlevel.
- `jim-650-same-library-version-receipt-json` (provider): [runtime/rust/tests/data/native_source_original_counted_filename_extent/650/Jim/same-library-version/receipt.json](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/650/Jim/same-library-version/receipt.json). SHA-256 `9b7e3829fa6023070ccd7745ad1d0da5498bd2e15125796fbbb4dec4bb70cb55`. filesystem codecs: Actual independent diagnostic bound to the identical measured static library and full pins.
- `jim-650-same-library-version-execute-stdout` (observation): [runtime/rust/tests/data/native_source_original_counted_filename_extent/650/Jim/same-library-version/execute.stdout](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/650/Jim/same-library-version/execute.stdout). SHA-256 `d7ce62ac8c979a247ad4edf0a43c9133dc8b547cdfc8e5b540fccd9bbe848166`. filesystem codecs: Linked-library patchlevel and C public version fields.
- `jim-651-receipt-json` (provider): [runtime/rust/tests/data/native_source_original_counted_filename_extent/651/Jim/receipt.json](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/651/Jim/receipt.json). SHA-256 `0f2dfdfe6e22f73929dda8ab8aeddfaaaeb31515924240e48fd8159fb5078238`. strict UTF-8 filenames: Exact compile/link/launch/environment, whole input pins and actual executable identity.
- `jim-651-execute-stdout` (observation): [runtime/rust/tests/data/native_source_original_counted_filename_extent/651/Jim/execute.stdout](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/651/Jim/execute.stdout). SHA-256 `0f3f5e02141ed1908fa77ed3304be1b335bae4fbb4b3ff727e89968cd998e4c0`. strict UTF-8 filenames: Unmodified full operation/query/header stream; selection-only errors provide no source completion.
- `jim-651-execute-stderr` (observation): [runtime/rust/tests/data/native_source_original_counted_filename_extent/651/Jim/execute.stderr](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/651/Jim/execute.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. strict UTF-8 filenames: Unmodified original process stderr.
- `jim-651-observations-json` (observation): [runtime/rust/tests/data/native_source_original_counted_filename_extent/651/Jim/observations.json](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/651/Jim/observations.json). SHA-256 `762a466ff3fb53eea1806f2a55d61a6911be11d43440bffdea901d7dddc7f66c`. strict UTF-8 filenames: Exact original projection of raw counted-byte output.
- `jim-651-cli-version-stdout` (observation): [runtime/rust/tests/data/native_source_original_counted_filename_extent/651/Jim/cli-version.stdout](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/651/Jim/cli-version.stdout). SHA-256 `f9752899fa012cf994a528e0250402609067a6f67a144a1172644a9329386803`. strict UTF-8 filenames: Actual independently receipt-bound CLI patchlevel.
- `jim-651-same-library-version-receipt-json` (provider): [runtime/rust/tests/data/native_source_original_counted_filename_extent/651/Jim/same-library-version/receipt.json](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/651/Jim/same-library-version/receipt.json). SHA-256 `c9831a63bb2238faa45311dbef29ca5557e539873058c70bd9f8f73fc18e7c05`. strict UTF-8 filenames: Actual independent diagnostic bound to the identical measured static library and full pins.
- `jim-651-same-library-version-execute-stdout` (observation): [runtime/rust/tests/data/native_source_original_counted_filename_extent/651/Jim/same-library-version/execute.stdout](../../../../runtime/rust/tests/data/native_source_original_counted_filename_extent/651/Jim/same-library-version/execute.stdout). SHA-256 `d7ce62ac8c979a247ad4edf0a43c9133dc8b547cdfc8e5b540fccd9bbe848166`. strict UTF-8 filenames: Linked-library patchlevel and C public version fields.
- `original-filename-owner-8bc5a15bd80b` (implementation): [runtime/rust/src/interp/native_encoding_source.rs](../../../../runtime/rust/src/interp/native_encoding_source.rs). SHA-256 `e7e4cd199a0cef00c7b95e892dc7b2c3cb80f15304b8a3b8a1907f3f71b076cd`. Current filename source/software controls and selected source/codec owner. Original operation/header checks and filesystem byte/reporting extents retain their independent purposes; source declarations provide no execution or Native callback/reference outcome.
- `original-filename-owner-5df1d6a30071` (implementation): [runtime/rust/src/interp/native_encoding_source/tests.rs](../../../../runtime/rust/src/interp/native_encoding_source/tests.rs). SHA-256 `22684b4ff81a77b6f19c5e2d1249acc6213a0b21541a5bed8a50c5dc57521590`. Current filename source/software controls and selected source/codec owner. Original operation/header checks and filesystem byte/reporting extents retain their independent purposes; source declarations provide no execution or Native callback/reference outcome.
- `original-filename-owner-3ea64916c589` (implementation): [rust/tcl-cmd-core/src/encoding/original_names.rs](../../../../rust/tcl-cmd-core/src/encoding/original_names.rs). SHA-256 `ba4a0f01d7367660cce90007243c09ce44f4cf073b18f65686b6cd8a3fa5ad4f`. Current filename source/software controls and selected source/codec owner. Original operation/header checks and filesystem byte/reporting extents retain their independent purposes; source declarations provide no execution or Native callback/reference outcome.
- `original-filename-owner-195e5af98323` (implementation): [rust/tcl-cmd-core/src/encoding/original_names/tests.rs](../../../../rust/tcl-cmd-core/src/encoding/original_names/tests.rs). SHA-256 `3ec19e0baf511c4ae882cae2aab07bbc8baec504a54eb4efbd92de788ae87dd7`. Current filename source/software controls and selected source/codec owner. Original operation/header checks and filesystem byte/reporting extents retain their independent purposes; source declarations provide no execution or Native callback/reference outcome.
- `original-filename-owner-4065d34b637e` (implementation): [rust/tcl-platform/src/lib.rs](../../../../rust/tcl-platform/src/lib.rs). SHA-256 `0a371c35d83bc12de2f3cdf07928d446d5e381d7d7b7e32f3da9be75badf9805`. Current filename source/software controls and selected source/codec owner. Original operation/header checks and filesystem byte/reporting extents retain their independent purposes; source declarations provide no execution or Native callback/reference outcome.
- `original-filename-owner-3622975b48c3` (implementation): [rust/tcl-vm/src/command/native_encoding_source.rs](../../../../rust/tcl-vm/src/command/native_encoding_source.rs). SHA-256 `521bc00029729c055b2153539078dc89aca3824328a20efde56d5ee7fba43996`. Current filename source/software controls and selected source/codec owner. Original operation/header checks and filesystem byte/reporting extents retain their independent purposes; source declarations provide no execution or Native callback/reference outcome.
- `original-filename-owner-03a384feb151` (implementation): [rust/tcl-vm/src/command/native_encoding_source/tests.rs](../../../../rust/tcl-vm/src/command/native_encoding_source/tests.rs). SHA-256 `d926a79cec5b25c30c330df8d9c7474bbd2b94441145f5f780a67f9cfd6364dd`. Current filename source/software controls and selected source/codec owner. Original operation/header checks and filesystem byte/reporting extents retain their independent purposes; source declarations provide no execution or Native callback/reference outcome.

## Source inspection

tcl8.4 8.4.20, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclIOUtil.c`, function `NativeCreateNativeRep`, lines 4024–4057. Line unit: physical LF only. Original byte range [121309, 122218). Full-source SHA-256 `fa8dbfd1c3a77b0538cece015df9efa56354f3d58272073bb91837537479c73d`; snippet SHA-256 `cd7e95d6ace8eb2f050037b99f7b71b55a5b6b2be0090a40c582a0d458039bd4`; retained evidence `source-window-0`.

```text
static ClientData 
NativeCreateNativeRep(pathObjPtr)
    Tcl_Obj* pathObjPtr;
{
    char *nativePathPtr;
    Tcl_DString ds;
    Tcl_Obj* validPathObjPtr;
    int len;
    char *str;

    /* Make sure the normalized path is set */
    validPathObjPtr = Tcl_FSGetNormalizedPath(NULL, pathObjPtr);
    if (validPathObjPtr == NULL) {
	return NULL;
    }

    str = Tcl_GetStringFromObj(validPathObjPtr, &len);
#ifdef __WIN32__
    Tcl_WinUtfToTChar(str, len, &ds);
    if (tclWinProcs->useWide) {
	len = Tcl_DStringLength(&ds) + sizeof(WCHAR);
    } else {
	len = Tcl_DStringLength(&ds) + sizeof(char);
    }
#else
    Tcl_UtfToExternalDString(NULL, str, len, &ds);
    len = Tcl_DStringLength(&ds) + sizeof(char);
#endif
    nativePathPtr = ckalloc((unsigned) len);
    memcpy((VOID*)nativePathPtr, (VOID*)Tcl_DStringValue(&ds), (size_t) len);

    Tcl_DStringFree(&ds);
    return (ClientData)nativePathPtr;
}

```

tcl8.4 8.4.20, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclIOUtil.c`, function `Tcl_FSEvalFile`, lines 1703–1810. Line unit: physical LF only. Original byte range [52363, 55548). Full-source SHA-256 `fa8dbfd1c3a77b0538cece015df9efa56354f3d58272073bb91837537479c73d`; snippet SHA-256 `958674b83a4f12dbc2102d4c19b097d0c4fc1b873ce0cbcd010736ac979b192e`; retained evidence `source-window-1`.

```text
int
Tcl_FSEvalFile(interp, pathPtr)
    Tcl_Interp *interp;		/* Interpreter in which to process file. */
    Tcl_Obj *pathPtr;		/* Path of file to process.  Tilde-substitution
				 * will be performed on this name. */
{
    int result, length;
    Tcl_StatBuf statBuf;
    Tcl_Obj *oldScriptFile;
    Interp *iPtr;
    char *string;
    Tcl_Channel chan;
    Tcl_Obj *objPtr;

    if (Tcl_FSGetNormalizedPath(interp, pathPtr) == NULL) {
	return TCL_ERROR;
    }

    result = TCL_ERROR;
    objPtr = Tcl_NewObj();
    Tcl_IncrRefCount(objPtr);

    if (Tcl_FSStat(pathPtr, &statBuf) == -1) {
        Tcl_SetErrno(errno);
	Tcl_AppendResult(interp, "couldn't read file \"", 
		Tcl_GetString(pathPtr),
		"\": ", Tcl_PosixError(interp), (char *) NULL);
	goto end;
    }
    chan = Tcl_FSOpenFileChannel(interp, pathPtr, "r", 0644);
    if (chan == (Tcl_Channel) NULL) {
        Tcl_ResetResult(interp);
	Tcl_AppendResult(interp, "couldn't read file \"", 
		Tcl_GetString(pathPtr),
		"\": ", Tcl_PosixError(interp), (char *) NULL);
	goto end;
    }
    /*
     * The eofchar is \32 (^Z).  This is the usual on Windows, but we
     * effect this cross-platform to allow for scripted documents.
     * [Bug: 2040]
     */
    Tcl_SetChannelOption(interp, chan, "-eofchar", "\32");
    /* Try to read first character of stream, so we can
     * check for utf-8 BOM to be handled especially.
     */
    if (Tcl_ReadChars(chan, objPtr, 1, 0) < 0) {
	Tcl_Close(interp, chan);
	Tcl_AppendResult(interp, "couldn't read file \"", 
		Tcl_GetString(pathPtr),	"\": ", Tcl_PosixError(interp), NULL);
	goto end;
    }
    string = Tcl_GetString(objPtr);
    /*
     * If first character is not a BOM, append the remaining characters,
     * otherwise replace them [Bug 3466099].
     */
    if (Tcl_ReadChars(chan, objPtr, -1,
	    memcmp(string, "\xef\xbb\xbf", 3)) < 0) {
	Tcl_Close(interp, chan);
	Tcl_AppendResult(interp, "couldn't read file \"",
		Tcl_GetString(pathPtr), "\": ", Tcl_PosixError(interp), NULL);
	goto end;
    }
    if (Tcl_Close(interp, chan) != TCL_OK) {
        goto end;
    }

    iPtr = (Interp *) interp;
    oldScriptFile = iPtr->scriptFile;
    iPtr->scriptFile = pathPtr;
    Tcl_IncrRefCount(iPtr->scriptFile);
    string = Tcl_GetStringFromObj(objPtr, &length);

#ifdef TCL_TIP280
    /* TIP #280 Force the evaluator to open a frame for a sourced
     * file. */
    iPtr->evalFlags |= TCL_EVAL_FILE;
#endif
    result = Tcl_EvalEx(interp, string, length, 0);
    /* 
     * Now we have to be careful; the script may have changed the
     * iPtr->scriptFile value, so we must reset it without
     * assuming it still points to 'pathPtr'.
     */
    if (iPtr->scriptFile != NULL) {
	Tcl_DecrRefCount(iPtr->scriptFile);
    }
    iPtr->scriptFile = oldScriptFile;

    if (result == TCL_RETURN) {
	result = TclUpdateReturnInfo(iPtr);
    } else if (result == TCL_ERROR) {
	char msg[200 + TCL_INTEGER_SPACE];

	/*
	 * Record information telling where the error occurred.
	 */

	sprintf(msg, "\n    (file \"%.150s\" line %d)", Tcl_GetString(pathPtr),
		interp->errorLine);
	Tcl_AddErrorInfo(interp, msg);
    }

    end:
    Tcl_DecrRefCount(objPtr);
    return result;
}

```

tcl8.5 8.5.19, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.5.19/unix/tclUnixFile.c`, function `TclNativeCreateNativeRep`, lines 1081–1129. Line unit: physical LF only. Original byte range [26796, 28080). Full-source SHA-256 `cd4ed1281e47f50f91d7fdec1f62ea003374a96a660d21d47219dae910f677ce`; snippet SHA-256 `70af7b6e7d88324078de2f3cb80f0ba00e29753156dc953f029793766c67e1e4`; retained evidence `source-window-2`.

```text
ClientData
TclNativeCreateNativeRep(
    Tcl_Obj *pathPtr)
{
    char *nativePathPtr;
    Tcl_DString ds;
    Tcl_Obj *validPathPtr;
    int len;
    char *str;

    if (TclFSCwdIsNative()) {
	/*
	 * The cwd is native, which means we can use the translated path
	 * without worrying about normalization (this will also usually be
	 * shorter so the utf-to-external conversion will be somewhat faster).
	 */

	validPathPtr = Tcl_FSGetTranslatedPath(NULL, pathPtr);
	if (validPathPtr == NULL) {
	    return NULL;
	}
    } else {
	/*
	 * Make sure the normalized path is set.
	 */

	validPathPtr = Tcl_FSGetNormalizedPath(NULL, pathPtr);
	if (validPathPtr == NULL) {
	    return NULL;
	}
	Tcl_IncrRefCount(validPathPtr);
    }

    str = Tcl_GetStringFromObj(validPathPtr, &len);
    Tcl_UtfToExternalDString(NULL, str, len, &ds);
    len = Tcl_DStringLength(&ds) + sizeof(char);
    if (strlen(Tcl_DStringValue(&ds)) < len - sizeof(char)) {
	/* See bug [3118489]: NUL in filenames */
	Tcl_DecrRefCount(validPathPtr);
	Tcl_DStringFree(&ds);
	return NULL;
    }
    Tcl_DecrRefCount(validPathPtr);
    nativePathPtr = ckalloc((unsigned) len);
    memcpy((void*)nativePathPtr, (void*)Tcl_DStringValue(&ds), (size_t) len);

    Tcl_DStringFree(&ds);
    return (ClientData)nativePathPtr;
}

```

tcl8.5 8.5.19, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclIOUtil.c`, function `Tcl_FSEvalFileEx`, lines 1769–1892. Line unit: physical LF only. Original byte range [50140, 53829). Full-source SHA-256 `24e13e612b8207a7109d6cb6175a8972d4b5233ec1088b7fbfbf632bf47b9b73`; snippet SHA-256 `f2b8aeeb4ce28b6e4970c0359e6fccf280c8522e627a33189a4bf4d0ffe799b2`; retained evidence `source-window-3`.

```text
int
Tcl_FSEvalFileEx(
    Tcl_Interp *interp,		/* Interpreter in which to process file. */
    Tcl_Obj *pathPtr,		/* Path of file to process. Tilde-substitution
				 * will be performed on this name. */
    const char *encodingName)	/* If non-NULL, then use this encoding for the
				 * file. NULL means use the system encoding. */
{
    int length, result = TCL_ERROR;
    Tcl_StatBuf statBuf;
    Tcl_Obj *oldScriptFile;
    Interp *iPtr;
    char *string;
    Tcl_Channel chan;
    Tcl_Obj *objPtr;

    if (Tcl_FSGetNormalizedPath(interp, pathPtr) == NULL) {
	return result;
    }

    if (Tcl_FSStat(pathPtr, &statBuf) == -1) {
	Tcl_SetErrno(errno);
	Tcl_AppendResult(interp, "couldn't read file \"",
		Tcl_GetString(pathPtr), "\": ", Tcl_PosixError(interp), NULL);
	return result;
    }
    chan = Tcl_FSOpenFileChannel(interp, pathPtr, "r", 0644);
    if (chan == (Tcl_Channel) NULL) {
	Tcl_ResetResult(interp);
	Tcl_AppendResult(interp, "couldn't read file \"",
		Tcl_GetString(pathPtr), "\": ", Tcl_PosixError(interp), NULL);
	return result;
    }

    /*
     * The eofchar is \32 (^Z). This is the usual on Windows, but we effect
     * this cross-platform to allow for scripted documents. [Bug: 2040]
     */

    Tcl_SetChannelOption(interp, chan, "-eofchar", "\32");

    /*
     * If the encoding is specified, set it for the channel. Else don't touch
     * it (and use the system encoding) Report error on unknown encoding.
     */

    if (encodingName != NULL) {
	if (Tcl_SetChannelOption(interp, chan, "-encoding", encodingName)
		!= TCL_OK) {
	    Tcl_Close(interp,chan);
	    return result;
	}
    }

    objPtr = Tcl_NewObj();
    Tcl_IncrRefCount(objPtr);
    /* Try to read first character of stream, so we can
     * check for utf-8 BOM to be handled especially.
     */
    if (Tcl_ReadChars(chan, objPtr, 1, 0) < 0) {
	Tcl_Close(interp, chan);
	Tcl_AppendResult(interp, "couldn't read file \"",
		Tcl_GetString(pathPtr), "\": ", Tcl_PosixError(interp), NULL);
	goto end;
    }
    string = Tcl_GetString(objPtr);
    /*
     * If first character is not a BOM, append the remaining characters,
     * otherwise replace them [Bug 3466099].
     */
    if (Tcl_ReadChars(chan, objPtr, -1,
	    memcmp(string, "\xef\xbb\xbf", 3)) < 0) {
	Tcl_Close(interp, chan);
	Tcl_AppendResult(interp, "couldn't read file \"",
		Tcl_GetString(pathPtr), "\": ", Tcl_PosixError(interp), NULL);
	goto end;
    }

    if (Tcl_Close(interp, chan) != TCL_OK) {
	goto end;
    }

    iPtr = (Interp *) interp;
    oldScriptFile = iPtr->scriptFile;
    iPtr->scriptFile = pathPtr;
    Tcl_IncrRefCount(iPtr->scriptFile);
    string = Tcl_GetStringFromObj(objPtr, &length);
    /* TIP #280 Force the evaluator to open a frame for a sourced
     * file. */
    iPtr->evalFlags |= TCL_EVAL_FILE;
    result = Tcl_EvalEx(interp, string, length, 0);

    /*
     * Now we have to be careful; the script may have changed the
     * iPtr->scriptFile value, so we must reset it without assuming it still
     * points to 'pathPtr'.
     */

    if (iPtr->scriptFile != NULL) {
	Tcl_DecrRefCount(iPtr->scriptFile);
    }
    iPtr->scriptFile = oldScriptFile;

    if (result == TCL_RETURN) {
	result = TclUpdateReturnInfo(iPtr);
    } else if (result == TCL_ERROR) {
	/*
	 * Record information telling where the error occurred.
	 */

	const char *pathString = Tcl_GetStringFromObj(pathPtr, &length);
	int limit = 150;
	int overflow = (length > limit);

	Tcl_AppendObjToErrorInfo(interp, Tcl_ObjPrintf(
		"\n    (file \"%.*s%s\" line %d)",
		(overflow ? limit : length), pathString,
		(overflow ? "..." : ""), interp->errorLine));
    }

  end:
    Tcl_DecrRefCount(objPtr);
    return result;
}

```

tcl8.6 8.6.18, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.6.18/unix/tclUnixFile.c`, function `TclNativeCreateNativeRep`, lines 1069–1117. Line unit: physical LF only. Original byte range [26403, 27705). Full-source SHA-256 `ac18cab881aec0b5699e70f165e49d1afc22c97b61e040ada7da31b7d53c1c40`; snippet SHA-256 `e0a8df86568d3a58b69fb121d83e8b446cbd2aaa16ba8c0fb922c7f5d4662654`; retained evidence `source-window-4`.

```text
void *
TclNativeCreateNativeRep(
    Tcl_Obj *pathPtr)
{
    char *nativePathPtr;
    const char *str;
    Tcl_DString ds;
    Tcl_Obj *validPathPtr;
    int len;

    if (TclFSCwdIsNative() || Tcl_FSGetPathType(pathPtr) == TCL_PATH_ABSOLUTE) {
	/*
	 * The cwd is native (or path is absolute), use the translated path
	 * without worrying about normalization (this will also usually be
	 * shorter so the utf-to-external conversion will be somewhat faster).
	 */

	validPathPtr = Tcl_FSGetTranslatedPath(NULL, pathPtr);
	if (validPathPtr == NULL) {
	    return NULL;
	}
    } else {
	/*
	 * Make sure the normalized path is set.
	 */

	validPathPtr = Tcl_FSGetNormalizedPath(NULL, pathPtr);
	if (validPathPtr == NULL) {
	    return NULL;
	}
	Tcl_IncrRefCount(validPathPtr);
    }

    str = Tcl_GetStringFromObj(validPathPtr, &len);
    Tcl_UtfToExternalDString(NULL, str, len, &ds);
    len = Tcl_DStringLength(&ds) + sizeof(char);
    if (strlen(Tcl_DStringValue(&ds)) < len - sizeof(char)) {
	/* See bug [3118489]: NUL in filenames */
	Tcl_DecrRefCount(validPathPtr);
	Tcl_DStringFree(&ds);
	return NULL;
    }
    Tcl_DecrRefCount(validPathPtr);
    nativePathPtr = (char *)ckalloc(len);
    memcpy(nativePathPtr, Tcl_DStringValue(&ds), len);

    Tcl_DStringFree(&ds);
    return nativePathPtr;
}

```

tcl8.6 8.6.18, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclIOUtil.c`, function `TclNREvalFile`, lines 1862–1966. Line unit: physical LF only. Original byte range [52139, 55130). Full-source SHA-256 `efc2579a7e21c6e4490116ddee386d3ea83819a3bcad76215127b480f5934f00`; snippet SHA-256 `0e9073e0356e3ce30863f04f86d3826c90b746f27a4d199c633997d19b1fcef9`; retained evidence `source-window-5`.

```text
int
TclNREvalFile(
    Tcl_Interp *interp,		/* Interpreter in which to process file. */
    Tcl_Obj *pathPtr,		/* Path of file to process. Tilde-substitution
				 * will be performed on this name. */
    const char *encodingName)	/* If non-NULL, then use this encoding for the
				 * file. NULL means use the system encoding. */
{
    Tcl_StatBuf statBuf;
    Tcl_Obj *oldScriptFile, *objPtr;
    Interp *iPtr;
    Tcl_Channel chan;
    const char *string;

    if (Tcl_FSGetNormalizedPath(interp, pathPtr) == NULL) {
	return TCL_ERROR;
    }

    if (Tcl_FSStat(pathPtr, &statBuf) == -1) {
	Tcl_SetErrno(errno);
	Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		"couldn't read file \"%s\": %s",
		TclGetString(pathPtr), Tcl_PosixError(interp)));
	return TCL_ERROR;
    }
    chan = Tcl_FSOpenFileChannel(interp, pathPtr, "r", 0644);
    if (chan == NULL) {
	Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		"couldn't read file \"%s\": %s",
		TclGetString(pathPtr), Tcl_PosixError(interp)));
	return TCL_ERROR;
    }

    /*
     * The eofchar is \32 (^Z). This is the usual on Windows, but we effect
     * this cross-platform to allow for scripted documents. [Bug: 2040]
     */

    Tcl_SetChannelOption(interp, chan, "-eofchar", "\32 {}");

    /*
     * If the encoding is specified, set it for the channel. Else don't touch
     * it (and use the system encoding) Report error on unknown encoding.
     */

    if (encodingName != NULL) {
	if (Tcl_SetChannelOption(interp, chan, "-encoding", encodingName)
		!= TCL_OK) {
	    Tcl_Close(interp,chan);
	    return TCL_ERROR;
	}
    }

    TclNewObj(objPtr);
    Tcl_IncrRefCount(objPtr);

    /*
     * Try to read first character of stream, so we can check for utf-8 BOM to
     * be handled especially.
     */

    if (Tcl_ReadChars(chan, objPtr, 1, 0) < 0) {
	Tcl_Close(interp, chan);
	Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		"couldn't read file \"%s\": %s",
		TclGetString(pathPtr), Tcl_PosixError(interp)));
	Tcl_DecrRefCount(objPtr);
	return TCL_ERROR;
    }
    string = TclGetString(objPtr);

    /*
     * If first character is not a BOM, append the remaining characters,
     * otherwise replace them. [Bug 3466099]
     */

    if (Tcl_ReadChars(chan, objPtr, -1,
	    memcmp(string, "\xEF\xBB\xBF", 3)) < 0) {
	Tcl_Close(interp, chan);
	Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		"couldn't read file \"%s\": %s",
		TclGetString(pathPtr), Tcl_PosixError(interp)));
	Tcl_DecrRefCount(objPtr);
	return TCL_ERROR;
    }

    if (Tcl_Close(interp, chan) != TCL_OK) {
	Tcl_DecrRefCount(objPtr);
	return TCL_ERROR;
    }

    iPtr = (Interp *) interp;
    oldScriptFile = iPtr->scriptFile;
    iPtr->scriptFile = pathPtr;
    Tcl_IncrRefCount(iPtr->scriptFile);

    /*
     * TIP #280: Force the evaluator to open a frame for a sourced file.
     */

    iPtr->evalFlags |= TCL_EVAL_FILE;
    TclNRAddCallback(interp, EvalFileCallback, oldScriptFile, pathPtr, objPtr,
	    NULL);
    return TclNREvalObjEx(interp, objPtr, 0, NULL, INT_MIN);
}

```

tcl9.0 9.0.4, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl9.0.4/unix/tclUnixFile.c`, function `TclNativeCreateNativeRep`, lines 1141–1193. Line unit: physical LF only. Original byte range [28566, 29983). Full-source SHA-256 `379cbe7ee97ea75e484f9c7cd7e354bdf86fa2644f6c4f77d3a106871d4bed43`; snippet SHA-256 `3087dc7e12cbef24ea1bd633064a72fc5503d6fefa8dc6e18fa42656a71f407d`; retained evidence `source-window-6`.

```text
void *
TclNativeCreateNativeRep(
    Tcl_Obj *pathPtr)
{
    char *nativePathPtr;
    const char *str;
    Tcl_DString ds;
    Tcl_Obj *validPathPtr;
    Tcl_Size len;

    if (TclFSCwdIsNative() || Tcl_FSGetPathType(pathPtr) == TCL_PATH_ABSOLUTE) {
	/*
	 * The cwd is native (or path is absolute), use the translated path
	 * without worrying about normalization (this will also usually be
	 * shorter so the utf-to-external conversion will be somewhat faster).
	 */

	validPathPtr = Tcl_FSGetTranslatedPath(NULL, pathPtr);
	if (validPathPtr == NULL) {
	    return NULL;
	}
    } else {
	/*
	 * Make sure the normalized path is set.
	 */

	validPathPtr = Tcl_FSGetNormalizedPath(NULL, pathPtr);
	if (validPathPtr == NULL) {
	    return NULL;
	}
	Tcl_IncrRefCount(validPathPtr);
    }

    str = TclGetStringFromObj(validPathPtr, &len);
    if (Tcl_UtfToExternalDStringEx(NULL, NULL, str, len, 0, &ds, NULL) != TCL_OK) {
	Tcl_DecrRefCount(validPathPtr);
	Tcl_DStringFree(&ds);
	return NULL;
    }
    len = Tcl_DStringLength(&ds) + sizeof(char);
    if (strlen(Tcl_DStringValue(&ds)) < len - sizeof(char)) {
	/* See bug [3118489]: NUL in filenames */
	Tcl_DecrRefCount(validPathPtr);
	Tcl_DStringFree(&ds);
	return NULL;
    }
    Tcl_DecrRefCount(validPathPtr);
    nativePathPtr = (char *)Tcl_Alloc(len);
    memcpy(nativePathPtr, Tcl_DStringValue(&ds), len);

    Tcl_DStringFree(&ds);
    return nativePathPtr;
}

```

tcl9.0 9.0.4, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclIOUtil.c`, function `TclNREvalFile`, lines 1828–1934. Line unit: physical LF only. Original byte range [48128, 51181). Full-source SHA-256 `a92cd056226140169ef4789870156962b7219d99cb72218a999ca5b8d9ce8770`; snippet SHA-256 `e1eb07ce63ba159b83319be60e50a5b27a4f6a0080d439141aa25e6bc7f45738`; retained evidence `source-window-7`.

```text
int
TclNREvalFile(
    Tcl_Interp *interp,		/* Interpreter in which to evaluate the script. */
    Tcl_Obj *pathPtr,		/* Pathname of a file containing the script to
				 * evaluate. Tilde-substitution is performed on
				 * this pathname. */
    const char *encodingName)	/* The name of an encoding to use, or NULL to
				 *  use the utf-8 encoding. */
{
    Tcl_StatBuf statBuf;
    Tcl_Obj *oldScriptFile, *objPtr;
    Interp *iPtr;
    Tcl_Channel chan;
    const char *string;

    if (Tcl_FSGetNormalizedPath(interp, pathPtr) == NULL) {
	return TCL_ERROR;
    }

    if (Tcl_FSStat(pathPtr, &statBuf) == -1) {
	Tcl_SetErrno(errno);
	Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		"couldn't read file \"%s\": %s",
		TclGetString(pathPtr), Tcl_PosixError(interp)));
	return TCL_ERROR;
    }
    chan = Tcl_FSOpenFileChannel(interp, pathPtr, "r", 0644);
    if (chan == NULL) {
	Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		"couldn't read file \"%s\": %s",
		TclGetString(pathPtr), Tcl_PosixError(interp)));
	return TCL_ERROR;
    }
    TclPkgFileSeen(interp, TclGetString(pathPtr));

    /*
     * The eof character is \x1A (^Z). Tcl uses it on every platform to allow
     * for scripted documents. [Bug: 2040]
     */

    Tcl_SetChannelOption(interp, chan, "-eofchar", "\x1A");

    /*
     * If the encoding is specified, set the channel to that encoding.
     * Otherwise use utf-8.  If the encoding is unknown report an error.
     */

    if (encodingName == NULL) {
	encodingName = "utf-8";
    }
    if (Tcl_SetChannelOption(interp, chan, "-encoding", encodingName)
	    != TCL_OK) {
	Tcl_CloseEx(interp, chan, 0);
	return TCL_ERROR;
    }

    TclNewObj(objPtr);
    Tcl_IncrRefCount(objPtr);

    /*
     * Read first character of stream to check for utf-8 BOM
     */

    if (Tcl_ReadChars(chan, objPtr, 1, 0) == TCL_IO_FAILURE) {
	Tcl_CloseEx(interp, chan, 0);
	Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		"couldn't read file \"%s\": %s",
		TclGetString(pathPtr), Tcl_PosixError(interp)));
	Tcl_DecrRefCount(objPtr);
	return TCL_ERROR;
    }
    string = TclGetString(objPtr);

    /*
     * If first character is not a BOM, append the remaining characters.
     * Otherwise, replace them. [Bug 3466099]
     */

    if (Tcl_ReadChars(chan, objPtr, TCL_INDEX_NONE,
	    memcmp(string, "\xEF\xBB\xBF", 3)) == TCL_IO_FAILURE) {
	Tcl_CloseEx(interp, chan, 0);
	Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		"couldn't read file \"%s\": %s",
		TclGetString(pathPtr), Tcl_PosixError(interp)));
	Tcl_DecrRefCount(objPtr);
	return TCL_ERROR;
    }

    if (Tcl_CloseEx(interp, chan, 0) != TCL_OK) {
	Tcl_DecrRefCount(objPtr);
	return TCL_ERROR;
    }

    iPtr = (Interp *) interp;
    oldScriptFile = iPtr->scriptFile;
    iPtr->scriptFile = pathPtr;
    Tcl_IncrRefCount(iPtr->scriptFile);

    /*
     * TIP #280:  Open a frame for the evaluated script.
     */

    iPtr->evalFlags |= TCL_EVAL_FILE;
    TclNRAddCallback(interp, EvalFileCallback, oldScriptFile, pathPtr, objPtr,
	    NULL);
    return TclNREvalObjEx(interp, objPtr, 0, NULL, INT_MIN);
}

```

tcl9.1 9.1.0, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl9.1.0/unix/tclUnixFile.c`, function `TclNativeCreateNativeRep`, lines 1143–1195. Line unit: physical LF only. Original byte range [28657, 30074). Full-source SHA-256 `d1d2c45a5a443d49c4b04baa55bc09b9e2c70a625db801947d8dcb112f4fc736`; snippet SHA-256 `3087dc7e12cbef24ea1bd633064a72fc5503d6fefa8dc6e18fa42656a71f407d`; retained evidence `source-window-8`.

```text
void *
TclNativeCreateNativeRep(
    Tcl_Obj *pathPtr)
{
    char *nativePathPtr;
    const char *str;
    Tcl_DString ds;
    Tcl_Obj *validPathPtr;
    Tcl_Size len;

    if (TclFSCwdIsNative() || Tcl_FSGetPathType(pathPtr) == TCL_PATH_ABSOLUTE) {
	/*
	 * The cwd is native (or path is absolute), use the translated path
	 * without worrying about normalization (this will also usually be
	 * shorter so the utf-to-external conversion will be somewhat faster).
	 */

	validPathPtr = Tcl_FSGetTranslatedPath(NULL, pathPtr);
	if (validPathPtr == NULL) {
	    return NULL;
	}
    } else {
	/*
	 * Make sure the normalized path is set.
	 */

	validPathPtr = Tcl_FSGetNormalizedPath(NULL, pathPtr);
	if (validPathPtr == NULL) {
	    return NULL;
	}
	Tcl_IncrRefCount(validPathPtr);
    }

    str = TclGetStringFromObj(validPathPtr, &len);
    if (Tcl_UtfToExternalDStringEx(NULL, NULL, str, len, 0, &ds, NULL) != TCL_OK) {
	Tcl_DecrRefCount(validPathPtr);
	Tcl_DStringFree(&ds);
	return NULL;
    }
    len = Tcl_DStringLength(&ds) + sizeof(char);
    if (strlen(Tcl_DStringValue(&ds)) < len - sizeof(char)) {
	/* See bug [3118489]: NUL in filenames */
	Tcl_DecrRefCount(validPathPtr);
	Tcl_DStringFree(&ds);
	return NULL;
    }
    Tcl_DecrRefCount(validPathPtr);
    nativePathPtr = (char *)Tcl_Alloc(len);
    memcpy(nativePathPtr, Tcl_DStringValue(&ds), len);

    Tcl_DStringFree(&ds);
    return nativePathPtr;
}

```

tcl9.1 9.1.0, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclIOUtil.c`, function `TclNREvalFile`, lines 1827–1933. Line unit: physical LF only. Original byte range [48199, 51251). Full-source SHA-256 `0e7e89671b5840ab90267b9ef5a4a9cac35b702c1a5b62a8a42de066dff514fd`; snippet SHA-256 `66cf0318dd71ffccf5daf7827173078e6c13fb1a78d25f61987db2eace82803f`; retained evidence `source-window-9`.

```text
int
TclNREvalFile(
    Tcl_Interp *interp,		/* Interpreter in which to evaluate the script. */
    Tcl_Obj *pathPtr,		/* Pathname of a file containing the script to
				 * evaluate. Tilde-substitution is performed on
				 * this pathname. */
    const char *encodingName)	/* The name of an encoding to use, or NULL to
				 * use the utf-8 encoding. */
{
    Tcl_StatBuf statBuf;
    Tcl_Obj *oldScriptFile, *objPtr;
    Interp *iPtr;
    Tcl_Channel chan;
    const char *string;

    if (Tcl_FSGetNormalizedPath(interp, pathPtr) == NULL) {
	return TCL_ERROR;
    }

    if (Tcl_FSStat(pathPtr, &statBuf) == -1) {
	Tcl_SetErrno(errno);
	Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		"couldn't read file \"%s\": %s",
		TclGetString(pathPtr), Tcl_PosixError(interp)));
	return TCL_ERROR;
    }
    chan = Tcl_FSOpenFileChannel(interp, pathPtr, "r", 0644);
    if (chan == NULL) {
	Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		"couldn't read file \"%s\": %s",
		TclGetString(pathPtr), Tcl_PosixError(interp)));
	return TCL_ERROR;
    }
    TclPkgFileSeen(interp, TclGetString(pathPtr));

    /*
     * The eof character is \x1A (^Z). Tcl uses it on every platform to allow
     * for scripted documents. [Bug: 2040]
     */

    Tcl_SetChannelOption(interp, chan, "-eofchar", "\x1A");

    /*
     * If the encoding is specified, set the channel to that encoding.
     * Otherwise use utf-8.  If the encoding is unknown report an error.
     */

    if (encodingName == NULL) {
	encodingName = "utf-8";
    }
    if (Tcl_SetChannelOption(interp, chan, "-encoding", encodingName)
	    != TCL_OK) {
	Tcl_CloseEx(interp, chan, 0);
	return TCL_ERROR;
    }

    TclNewObj(objPtr);
    Tcl_IncrRefCount(objPtr);

    /*
     * Read first character of stream to check for utf-8 BOM
     */

    if (Tcl_ReadChars(chan, objPtr, 1, 0) == TCL_IO_FAILURE) {
	Tcl_CloseEx(interp, chan, 0);
	Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		"couldn't read file \"%s\": %s",
		TclGetString(pathPtr), Tcl_PosixError(interp)));
	Tcl_DecrRefCount(objPtr);
	return TCL_ERROR;
    }
    string = TclGetString(objPtr);

    /*
     * If first character is not a BOM, append the remaining characters.
     * Otherwise, replace them. [Bug 3466099]
     */

    if (Tcl_ReadChars(chan, objPtr, TCL_INDEX_NONE,
	    memcmp(string, "\xEF\xBB\xBF", 3)) == TCL_IO_FAILURE) {
	Tcl_CloseEx(interp, chan, 0);
	Tcl_SetObjResult(interp, Tcl_ObjPrintf(
		"couldn't read file \"%s\": %s",
		TclGetString(pathPtr), Tcl_PosixError(interp)));
	Tcl_DecrRefCount(objPtr);
	return TCL_ERROR;
    }

    if (Tcl_CloseEx(interp, chan, 0) != TCL_OK) {
	Tcl_DecrRefCount(objPtr);
	return TCL_ERROR;
    }

    iPtr = (Interp *) interp;
    oldScriptFile = iPtr->scriptFile;
    iPtr->scriptFile = pathPtr;
    Tcl_IncrRefCount(iPtr->scriptFile);

    /*
     * TIP #280:  Open a frame for the evaluated script.
     */

    iPtr->evalFlags |= TCL_EVAL_FILE;
    TclNRAddCallback(interp, EvalFileCallback, oldScriptFile, pathPtr, objPtr,
	    NULL);
    return TclNREvalObjEx(interp, objPtr, 0, NULL, INT_MIN);
}

```

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `Jim_SourceCoreCommand`, lines 16711–16716. Line unit: physical LF only. Original byte range [529211, 529412). Full-source SHA-256 `fe5e3bea157b898e91e6c8e50c8f9eb0e89a6aba8c6a26b5d4b7b2b03cba3867`; snippet SHA-256 `06f50ddd7d96c6cf52b7de221ead820052a94493441f3e1f3d016bb7708981b5`; retained evidence `source-window-10`.

```text
static int Jim_SourceCoreCommand(Jim_Interp *interp, int argc, Jim_Obj *const *argv)
{
    int retval = Jim_EvalFile(interp, Jim_String(argv[1]));

    return retval == JIM_RETURN ? JIM_OK : retval;
}

```

jim 0.84-9-g5bac7c9, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03`, `/workspace/.proofs/native-providers/jimtcl/jim.c`, function `Jim_EvalFile`, lines 12120–12168. Line unit: physical LF only. Original byte range [384075, 385640). Full-source SHA-256 `fe5e3bea157b898e91e6c8e50c8f9eb0e89a6aba8c6a26b5d4b7b2b03cba3867`; snippet SHA-256 `4dcbe0c82d53aff2760380ea9daed26891e9400aea9c272d8510940734f8df8b`; retained evidence `source-window-11`.

```text
int Jim_EvalFile(Jim_Interp *interp, const char *filename)
{
    FILE *fp;
    char *buf;
    Jim_Obj *scriptObjPtr;
    Jim_Obj *filenameObj, *oldFilenameObj;
    int retcode = JIM_ERR;
    int readlen;
#define READ_BUF_SIZE 256

   if ((fp = fopen(filename, "rt")) == NULL) {
        Jim_SetResultFormatted(interp, "couldn't read file \"%s\": %s", filename, strerror(errno));
        return JIM_ERR;
    }
    scriptObjPtr = Jim_NewStringObj(interp, NULL, 0);

    buf = Jim_Alloc(READ_BUF_SIZE);
    while ((readlen = fread(buf, 1, READ_BUF_SIZE, fp)) > 0) {
        Jim_AppendString(interp, scriptObjPtr, buf, readlen);
    }
    Jim_Free(buf);
    if (ferror(fp)) {
        fclose(fp);
        Jim_SetResultFormatted(interp, "failed to load file \"%s\": %s", filename, strerror(errno));
        Jim_FreeNewObj(interp, scriptObjPtr);
        return retcode;
    }
    fclose(fp);

    /* Convert the stringObjType to a sourceObjType with filename and line */
    filenameObj = Jim_NewStringObj(interp, filename, -1);
    Jim_SetSourceInfo(interp, scriptObjPtr, filenameObj, 1);
    oldFilenameObj = JimPushInterpObj(interp->currentFilenameObj, filenameObj);

    retcode = Jim_EvalObj(interp, scriptObjPtr);

    JimPopInterpObj(interp, interp->currentFilenameObj, oldFilenameObj);

    /* Handle the JIM_RETURN return code */
    if (retcode == JIM_RETURN) {
        if (--interp->returnLevel <= 0) {
            retcode = interp->returnCode;
            interp->returnCode = JIM_OK;
            interp->returnLevel = 0;
        }
    }

    return retcode;
}

```

tcl8.4 8.4.20, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclEncoding.c`, function `BinaryProc`, lines 1846–1888. Line unit: physical LF only. Original byte range [55178, 56671). Full-source SHA-256 `b52a3a4d8385133479cd25a815f3dbb50f66848df7353be164ac2108556240a6`; snippet SHA-256 `6dac413b89d94e12b17a694b405608f92063deed1c50454997ddef2dc9bd22de`; retained evidence `source-window-12`.

```text
static int
BinaryProc(clientData, src, srcLen, flags, statePtr, dst, dstLen,
	srcReadPtr, dstWrotePtr, dstCharsPtr)
    ClientData clientData;	/* Not used. */
    CONST char *src;		/* Source string (unknown encoding). */
    int srcLen;			/* Source string length in bytes. */
    int flags;			/* Conversion control flags. */
    Tcl_EncodingState *statePtr;/* Place for conversion routine to store
				 * state information used during a piecewise
				 * conversion.  Contents of statePtr are
				 * initialized and/or reset by conversion
				 * routine under control of flags argument. */
    char *dst;			/* Output buffer in which converted string
				 * is stored. */
    int dstLen;			/* The maximum length of output buffer in
				 * bytes. */
    int *srcReadPtr;		/* Filled with the number of bytes from the
				 * source string that were converted. */
    int *dstWrotePtr;		/* Filled with the number of bytes that were
				 * stored in the output buffer as a result of
				 * the conversion. */
    int *dstCharsPtr;		/* Filled with the number of characters that
				 * correspond to the bytes stored in the
				 * output buffer. */
{
    int result;

    result = TCL_OK;
    dstLen -= TCL_UTF_MAX - 1;
    if (dstLen < 0) {
	dstLen = 0;
    }
    if (srcLen > dstLen) {
	srcLen = dstLen;
	result = TCL_CONVERT_NOSPACE;
    }

    *srcReadPtr = srcLen;
    *dstWrotePtr = srcLen;
    *dstCharsPtr = srcLen;
    memcpy((void *) dst, (void *) src, (size_t) srcLen);
    return result;
}

```

tcl8.4 8.4.20, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclEncoding.c`, function `TableFromUtfProc`, lines 2412–2516. Line unit: physical LF only. Original byte range [72899, 76015). Full-source SHA-256 `b52a3a4d8385133479cd25a815f3dbb50f66848df7353be164ac2108556240a6`; snippet SHA-256 `319dd2e40a44c47783ace438abc8be8312e9342c4e7354a4000155e97c89e5d0`; retained evidence `source-window-13`.

```text
static int 
TableFromUtfProc(clientData, src, srcLen, flags, statePtr, dst, dstLen,
	srcReadPtr, dstWrotePtr, dstCharsPtr)
    ClientData clientData;	/* TableEncodingData that specifies
				 * encoding. */
    CONST char *src;		/* Source string in UTF-8. */
    int srcLen;			/* Source string length in bytes. */
    int flags;			/* Conversion control flags. */
    Tcl_EncodingState *statePtr;/* Place for conversion routine to store
				 * state information used during a piecewise
				 * conversion.  Contents of statePtr are
				 * initialized and/or reset by conversion
				 * routine under control of flags argument. */
    char *dst;			/* Output buffer in which converted string
				 * is stored. */
    int dstLen;			/* The maximum length of output buffer in
				 * bytes. */
    int *srcReadPtr;		/* Filled with the number of bytes from the
				 * source string that were converted.  This
				 * may be less than the original source length
				 * if there was a problem converting some
				 * source characters. */
    int *dstWrotePtr;		/* Filled with the number of bytes that were
				 * stored in the output buffer as a result of
				 * the conversion. */
    int *dstCharsPtr;		/* Filled with the number of characters that
				 * correspond to the bytes stored in the
				 * output buffer. */
{
    CONST char *srcStart, *srcEnd, *srcClose;
    char *dstStart, *dstEnd, *prefixBytes;
    Tcl_UniChar ch;
    int result, len, word, numChars;
    TableEncodingData *dataPtr;
    unsigned short **fromUnicode;
    
    result = TCL_OK;    

    dataPtr = (TableEncodingData *) clientData;
    prefixBytes = dataPtr->prefixBytes;
    fromUnicode = dataPtr->fromUnicode;
    
    srcStart = src;
    srcEnd = src + srcLen;
    srcClose = srcEnd;
    if ((flags & TCL_ENCODING_END) == 0) {
	srcClose -= TCL_UTF_MAX;
    }

    dstStart = dst;
    dstEnd = dst + dstLen - 1;

    for (numChars = 0; src < srcEnd; numChars++) {
	if ((src > srcClose) && (!Tcl_UtfCharComplete(src, srcEnd - src))) {
	    /*
	     * If there is more string to follow, this will ensure that the
	     * last UTF-8 character in the source buffer hasn't been cut off.
	     */

	    result = TCL_CONVERT_MULTIBYTE;
	    break;
	}
	len = TclUtfToUniChar(src, &ch);

#if TCL_UTF_MAX > 3
	/*
	 * This prevents a crash condition.  More evaluation is required
	 * for full support of int Tcl_UniChar. [Bug 1004065]
	 */
	if (ch & 0xffff0000) {
	    word = 0;
	} else
#endif
	    word = fromUnicode[(ch >> 8)][ch & 0xff];

	if ((word == 0) && (ch != 0)) {
	    if (flags & TCL_ENCODING_STOPONERROR) {
		result = TCL_CONVERT_UNKNOWN;
		break;
	    }
	    word = dataPtr->fallback; 
	}
	if (prefixBytes[(word >> 8)] != 0) {
	    if (dst + 1 > dstEnd) {
		result = TCL_CONVERT_NOSPACE;
		break;
	    }
	    dst[0] = (char) (word >> 8);
	    dst[1] = (char) word;
	    dst += 2;
	} else {
	    if (dst > dstEnd) {
		result = TCL_CONVERT_NOSPACE;
		break;
	    }
	    dst[0] = (char) word;
	    dst++;
	} 
	src += len;
    }
    *srcReadPtr = src - srcStart;
    *dstWrotePtr = dst - dstStart;
    *dstCharsPtr = numChars;
    return result;
}

```

tcl8.5 8.5.19, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclEncoding.c`, function `BinaryProc`, lines 2059–2100. Line unit: physical LF only. Original byte range [60808, 62189). Full-source SHA-256 `b100161e6fbcdc9b4055b28308071fa123000526abcb51dc1a65eeb83e94b292`; snippet SHA-256 `00b08ff0265596e311ccf77b5568a1ff3b6d6e6b902168d6482e5adc15862454`; retained evidence `source-window-14`.

```text
static int
BinaryProc(
    ClientData clientData,	/* Not used. */
    const char *src,		/* Source string (unknown encoding). */
    int srcLen,			/* Source string length in bytes. */
    int flags,			/* Conversion control flags. */
    Tcl_EncodingState *statePtr,/* Place for conversion routine to store state
				 * information used during a piecewise
				 * conversion. Contents of statePtr are
				 * initialized and/or reset by conversion
				 * routine under control of flags argument. */
    char *dst,			/* Output buffer in which converted string is
				 * stored. */
    int dstLen,			/* The maximum length of output buffer in
				 * bytes. */
    int *srcReadPtr,		/* Filled with the number of bytes from the
				 * source string that were converted. */
    int *dstWrotePtr,		/* Filled with the number of bytes that were
				 * stored in the output buffer as a result of
				 * the conversion. */
    int *dstCharsPtr)		/* Filled with the number of characters that
				 * correspond to the bytes stored in the
				 * output buffer. */
{
    int result;

    result = TCL_OK;
    dstLen -= TCL_UTF_MAX - 1;
    if (dstLen < 0) {
	dstLen = 0;
    }
    if (srcLen > dstLen) {
	srcLen = dstLen;
	result = TCL_CONVERT_NOSPACE;
    }

    *srcReadPtr = srcLen;
    *dstWrotePtr = srcLen;
    *dstCharsPtr = srcLen;
    memcpy(dst, src, (size_t) srcLen);
    return result;
}

```

tcl8.5 8.5.19, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclEncoding.c`, function `Iso88591FromUtfProc`, lines 2821–2908. Line unit: physical LF only. Original byte range [83361, 85745). Full-source SHA-256 `b100161e6fbcdc9b4055b28308071fa123000526abcb51dc1a65eeb83e94b292`; snippet SHA-256 `2985077a739b5487790178e1d0d6f86dcaf6235687120f493463caffb0c03f8c`; retained evidence `source-window-15`.

```text
static int
Iso88591FromUtfProc(
    ClientData clientData,	/* Ignored. */
    const char *src,		/* Source string in UTF-8. */
    int srcLen,			/* Source string length in bytes. */
    int flags,			/* Conversion control flags. */
    Tcl_EncodingState *statePtr,/* Place for conversion routine to store state
				 * information used during a piecewise
				 * conversion. Contents of statePtr are
				 * initialized and/or reset by conversion
				 * routine under control of flags argument. */
    char *dst,			/* Output buffer in which converted string is
				 * stored. */
    int dstLen,			/* The maximum length of output buffer in
				 * bytes. */
    int *srcReadPtr,		/* Filled with the number of bytes from the
				 * source string that were converted. This may
				 * be less than the original source length if
				 * there was a problem converting some source
				 * characters. */
    int *dstWrotePtr,		/* Filled with the number of bytes that were
				 * stored in the output buffer as a result of
				 * the conversion. */
    int *dstCharsPtr)		/* Filled with the number of characters that
				 * correspond to the bytes stored in the
				 * output buffer. */
{
    const char *srcStart, *srcEnd, *srcClose;
    char *dstStart, *dstEnd;
    int result, numChars;

    result = TCL_OK;

    srcStart = src;
    srcEnd = src + srcLen;
    srcClose = srcEnd;
    if ((flags & TCL_ENCODING_END) == 0) {
	srcClose -= TCL_UTF_MAX;
    }

    dstStart = dst;
    dstEnd = dst + dstLen - 1;

    for (numChars = 0; src < srcEnd; numChars++) {
	Tcl_UniChar ch;
	int len;

	if ((src > srcClose) && (!Tcl_UtfCharComplete(src, srcEnd - src))) {
	    /*
	     * If there is more string to follow, this will ensure that the
	     * last UTF-8 character in the source buffer hasn't been cut off.
	     */

	    result = TCL_CONVERT_MULTIBYTE;
	    break;
	}
	len = TclUtfToUniChar(src, &ch);

	/*
	 * Check for illegal characters.
	 */

	if (ch > 0xff) {
	    if (flags & TCL_ENCODING_STOPONERROR) {
		result = TCL_CONVERT_UNKNOWN;
		break;
	    }

	    /*
	     * Plunge on, using '?' as a fallback character.
	     */

	    ch = (Tcl_UniChar) '?';
	}

	if (dst > dstEnd) {
	    result = TCL_CONVERT_NOSPACE;
	    break;
	}
	*(dst++) = (char) ch;
	src += len;
    }

    *srcReadPtr = src - srcStart;
    *dstWrotePtr = dst - dstStart;
    *dstCharsPtr = numChars;
    return result;
}

```

tcl8.6 8.6.18, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclEncoding.c`, function `BinaryProc`, lines 2113–2157. Line unit: physical LF only. Original byte range [61425, 62897). Full-source SHA-256 `359bfa493e7aba998e524978aca4345422e2e265dde93e73f76cb5fc8784e4c6`; snippet SHA-256 `f5fea9f144ff24bb4d3ecd05b216b1c41ba0ac465685ab3c7ee8b9f97e7237ff`; retained evidence `source-window-16`.

```text
static int
BinaryProc(
    ClientData clientData,	/* Not used. */
    const char *src,		/* Source string (unknown encoding). */
    int srcLen,			/* Source string length in bytes. */
    int flags,			/* Conversion control flags. */
    Tcl_EncodingState *statePtr,/* Place for conversion routine to store state
				 * information used during a piecewise
				 * conversion. Contents of statePtr are
				 * initialized and/or reset by conversion
				 * routine under control of flags argument. */
    char *dst,			/* Output buffer in which converted string is
				 * stored. */
    int dstLen,			/* The maximum length of output buffer in
				 * bytes. */
    int *srcReadPtr,		/* Filled with the number of bytes from the
				 * source string that were converted. */
    int *dstWrotePtr,		/* Filled with the number of bytes that were
				 * stored in the output buffer as a result of
				 * the conversion. */
    int *dstCharsPtr)		/* Filled with the number of characters that
				 * correspond to the bytes stored in the
				 * output buffer. */
{
    int result;

    result = TCL_OK;
    dstLen -= TCL_UTF_MAX - 1;
    if (dstLen < 0) {
	dstLen = 0;
    }
    if ((flags & TCL_ENCODING_CHAR_LIMIT) && srcLen > *dstCharsPtr) {
	srcLen = *dstCharsPtr;
    }
    if (srcLen > dstLen) {
	srcLen = dstLen;
	result = TCL_CONVERT_NOSPACE;
    }

    *srcReadPtr = srcLen;
    *dstWrotePtr = srcLen;
    *dstCharsPtr = srcLen;
    memcpy(dst, src, srcLen);
    return result;
}

```

tcl8.6 8.6.18, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclEncoding.c`, function `Iso88591FromUtfProc`, lines 3006–3099. Line unit: physical LF only. Original byte range [88094, 90629). Full-source SHA-256 `359bfa493e7aba998e524978aca4345422e2e265dde93e73f76cb5fc8784e4c6`; snippet SHA-256 `9bba61b14ff19fe76492fa9a7d4aa704d8c1275091f4c943af9409e1d9fe2daa`; retained evidence `source-window-17`.

```text
static int
Iso88591FromUtfProc(
    ClientData clientData,	/* Ignored. */
    const char *src,		/* Source string in UTF-8. */
    int srcLen,			/* Source string length in bytes. */
    int flags,			/* Conversion control flags. */
    Tcl_EncodingState *statePtr,/* Place for conversion routine to store state
				 * information used during a piecewise
				 * conversion. Contents of statePtr are
				 * initialized and/or reset by conversion
				 * routine under control of flags argument. */
    char *dst,			/* Output buffer in which converted string is
				 * stored. */
    int dstLen,			/* The maximum length of output buffer in
				 * bytes. */
    int *srcReadPtr,		/* Filled with the number of bytes from the
				 * source string that were converted. This may
				 * be less than the original source length if
				 * there was a problem converting some source
				 * characters. */
    int *dstWrotePtr,		/* Filled with the number of bytes that were
				 * stored in the output buffer as a result of
				 * the conversion. */
    int *dstCharsPtr)		/* Filled with the number of characters that
				 * correspond to the bytes stored in the
				 * output buffer. */
{
    const char *srcStart, *srcEnd, *srcClose;
    const char *dstStart, *dstEnd;
    int result = TCL_OK, numChars;
    Tcl_UniChar ch = 0;

    srcStart = src;
    srcEnd = src + srcLen;
    srcClose = srcEnd;
    if ((flags & TCL_ENCODING_END) == 0) {
	srcClose -= TCL_UTF_MAX;
    }

    dstStart = dst;
    dstEnd = dst + dstLen - 1;

    for (numChars = 0; src < srcEnd; numChars++) {
	int len;

	if ((src > srcClose) && (!Tcl_UtfCharComplete(src, srcEnd - src))) {
	    /*
	     * If there is more string to follow, this will ensure that the
	     * last UTF-8 character in the source buffer hasn't been cut off.
	     */

	    result = TCL_CONVERT_MULTIBYTE;
	    break;
	}
	len = TclUtfToUniChar(src, &ch);

	/*
	 * Check for illegal characters.
	 */

	if (ch > 0xFF
#if TCL_UTF_MAX == 4
		|| ((ch >= 0xD800) && (len < 3))
#endif
		) {
	    if (flags & TCL_ENCODING_STOPONERROR) {
		result = TCL_CONVERT_UNKNOWN;
		break;
	    }
#if TCL_UTF_MAX == 4
	    if ((ch >= 0xD800) && (len < 3)) {
		len = 4;
	    }
#endif
	    /*
	     * Plunge on, using '?' as a fallback character.
	     */

	    ch = (Tcl_UniChar) '?';
	}

	if (dst > dstEnd) {
	    result = TCL_CONVERT_NOSPACE;
	    break;
	}
	*(dst++) = (char) ch;
	src += len;
    }

    *srcReadPtr = src - srcStart;
    *dstWrotePtr = dst - dstStart;
    *dstCharsPtr = numChars;
    return result;
}

```

tcl9.0 9.0.4, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclEncoding.c`, function `BinaryProc`, lines 2380–2420. Line unit: physical LF only. Original byte range [71239, 72466). Full-source SHA-256 `b2d923ff94e392ddbfcb6daf474e58cff1b880d574d6b70ee3153a196f9f7531`; snippet SHA-256 `d4723d95992ac4a75f322b542ca10c788cca3a0a8c2a9621ae7bc7b43fc49fa8`; retained evidence `source-window-18`.

```text
static int
BinaryProc(
    TCL_UNUSED(void *),
    const char *src,		/* Source string (unknown encoding). */
    int srcLen,			/* Source string length in bytes. */
    int flags,			/* Conversion control flags. */
    TCL_UNUSED(Tcl_EncodingState *),
    char *dst,			/* Output buffer in which converted string is
				 * stored. */
    int dstLen,			/* The maximum length of output buffer in
				 * bytes. */
    int *srcReadPtr,		/* Filled with the number of bytes from the
				 * source string that were converted. */
    int *dstWrotePtr,		/* Filled with the number of bytes that were
				 * stored in the output buffer as a result of
				 * the conversion. */
    int *dstCharsPtr)		/* Filled with the number of characters that
				 * correspond to the bytes stored in the
				 * output buffer. */
{
    int result;

    result = TCL_OK;
    dstLen -= TCL_UTF_MAX - 1;
    if (dstLen < 0) {
	dstLen = 0;
    }
    if ((flags & TCL_ENCODING_CHAR_LIMIT) && srcLen > *dstCharsPtr) {
	srcLen = *dstCharsPtr;
    }
    if (srcLen > dstLen) {
	srcLen = dstLen;
	result = TCL_CONVERT_NOSPACE;
    }

    *srcReadPtr = srcLen;
    *dstWrotePtr = srcLen;
    *dstCharsPtr = srcLen;
    memcpy(dst, src, srcLen);
    return result;
}

```

tcl9.0 9.0.4, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclEncoding.c`, function `Iso88591FromUtfProc`, lines 3768–3848. Line unit: physical LF only. Original byte range [111142, 113288). Full-source SHA-256 `b2d923ff94e392ddbfcb6daf474e58cff1b880d574d6b70ee3153a196f9f7531`; snippet SHA-256 `f96bdfc7d38dfdfba7474be9cb206bca17b4729432fdba9e21bdac814e6d500a`; retained evidence `source-window-19`.

```text
static int
Iso88591FromUtfProc(
    TCL_UNUSED(void *),
    const char *src,		/* Source string in UTF-8. */
    int srcLen,			/* Source string length in bytes. */
    int flags,			/* Conversion control flags. */
    TCL_UNUSED(Tcl_EncodingState *),
    char *dst,			/* Output buffer in which converted string is
				 * stored. */
    int dstLen,			/* The maximum length of output buffer in
				 * bytes. */
    int *srcReadPtr,		/* Filled with the number of bytes from the
				 * source string that were converted. This may
				 * be less than the original source length if
				 * there was a problem converting some source
				 * characters. */
    int *dstWrotePtr,		/* Filled with the number of bytes that were
				 * stored in the output buffer as a result of
				 * the conversion. */
    int *dstCharsPtr)		/* Filled with the number of characters that
				 * correspond to the bytes stored in the
				 * output buffer. */
{
    const char *srcStart, *srcEnd, *srcClose;
    const char *dstStart, *dstEnd;
    int result = TCL_OK, numChars;
    Tcl_UniChar ch = 0;

    srcStart = src;
    srcEnd = src + srcLen;
    srcClose = srcEnd;
    if ((flags & TCL_ENCODING_END) == 0) {
	srcClose -= TCL_UTF_MAX;
    }

    dstStart = dst;
    dstEnd = dst + dstLen - 1;

    for (numChars = 0; src < srcEnd; numChars++) {
	int len;

	if ((src > srcClose) && (!Tcl_UtfCharComplete(src, srcEnd - src))) {
	    /*
	     * If there is more string to follow, this will ensure that the
	     * last UTF-8 character in the source buffer hasn't been cut off.
	     */

	    result = TCL_CONVERT_MULTIBYTE;
	    break;
	}
	len = TclUtfToUniChar(src, &ch);

	/*
	 * Check for illegal characters.
	 */

	if (ch > 0xFF) {
	    if (PROFILE_STRICT(flags)) {
		result = TCL_CONVERT_UNKNOWN;
		break;
	    }
	    /*
	     * Plunge on, using '?' as a fallback character.
	     */

	    ch = '?'; /* Profiles TCL8 and REPLACE */
	}

	if (dst > dstEnd) {
	    result = TCL_CONVERT_NOSPACE;
	    break;
	}
	*(dst++) = (char) ch;
	src += len;
    }

    *srcReadPtr = src - srcStart;
    *dstWrotePtr = dst - dstStart;
    *dstCharsPtr = numChars;
    return result;
}

```

tcl9.1 9.1.0, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclEncoding.c`, function `BinaryProc`, lines 2922–2962. Line unit: physical LF only. Original byte range [90136, 91363). Full-source SHA-256 `a8e40415c4e984eb2d8b888f79a3fc09fe1074233f8ee3d6b6bdb0b270d8b770`; snippet SHA-256 `d4723d95992ac4a75f322b542ca10c788cca3a0a8c2a9621ae7bc7b43fc49fa8`; retained evidence `source-window-20`.

```text
static int
BinaryProc(
    TCL_UNUSED(void *),
    const char *src,		/* Source string (unknown encoding). */
    int srcLen,			/* Source string length in bytes. */
    int flags,			/* Conversion control flags. */
    TCL_UNUSED(Tcl_EncodingState *),
    char *dst,			/* Output buffer in which converted string is
				 * stored. */
    int dstLen,			/* The maximum length of output buffer in
				 * bytes. */
    int *srcReadPtr,		/* Filled with the number of bytes from the
				 * source string that were converted. */
    int *dstWrotePtr,		/* Filled with the number of bytes that were
				 * stored in the output buffer as a result of
				 * the conversion. */
    int *dstCharsPtr)		/* Filled with the number of characters that
				 * correspond to the bytes stored in the
				 * output buffer. */
{
    int result;

    result = TCL_OK;
    dstLen -= TCL_UTF_MAX - 1;
    if (dstLen < 0) {
	dstLen = 0;
    }
    if ((flags & TCL_ENCODING_CHAR_LIMIT) && srcLen > *dstCharsPtr) {
	srcLen = *dstCharsPtr;
    }
    if (srcLen > dstLen) {
	srcLen = dstLen;
	result = TCL_CONVERT_NOSPACE;
    }

    *srcReadPtr = srcLen;
    *dstWrotePtr = srcLen;
    *dstCharsPtr = srcLen;
    memcpy(dst, src, srcLen);
    return result;
}

```

tcl9.1 9.1.0, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclEncoding.c`, function `Iso88591FromUtfProc`, lines 4573–4653. Line unit: physical LF only. Original byte range [136492, 138638). Full-source SHA-256 `a8e40415c4e984eb2d8b888f79a3fc09fe1074233f8ee3d6b6bdb0b270d8b770`; snippet SHA-256 `f96bdfc7d38dfdfba7474be9cb206bca17b4729432fdba9e21bdac814e6d500a`; retained evidence `source-window-21`.

```text
static int
Iso88591FromUtfProc(
    TCL_UNUSED(void *),
    const char *src,		/* Source string in UTF-8. */
    int srcLen,			/* Source string length in bytes. */
    int flags,			/* Conversion control flags. */
    TCL_UNUSED(Tcl_EncodingState *),
    char *dst,			/* Output buffer in which converted string is
				 * stored. */
    int dstLen,			/* The maximum length of output buffer in
				 * bytes. */
    int *srcReadPtr,		/* Filled with the number of bytes from the
				 * source string that were converted. This may
				 * be less than the original source length if
				 * there was a problem converting some source
				 * characters. */
    int *dstWrotePtr,		/* Filled with the number of bytes that were
				 * stored in the output buffer as a result of
				 * the conversion. */
    int *dstCharsPtr)		/* Filled with the number of characters that
				 * correspond to the bytes stored in the
				 * output buffer. */
{
    const char *srcStart, *srcEnd, *srcClose;
    const char *dstStart, *dstEnd;
    int result = TCL_OK, numChars;
    Tcl_UniChar ch = 0;

    srcStart = src;
    srcEnd = src + srcLen;
    srcClose = srcEnd;
    if ((flags & TCL_ENCODING_END) == 0) {
	srcClose -= TCL_UTF_MAX;
    }

    dstStart = dst;
    dstEnd = dst + dstLen - 1;

    for (numChars = 0; src < srcEnd; numChars++) {
	int len;

	if ((src > srcClose) && (!Tcl_UtfCharComplete(src, srcEnd - src))) {
	    /*
	     * If there is more string to follow, this will ensure that the
	     * last UTF-8 character in the source buffer hasn't been cut off.
	     */

	    result = TCL_CONVERT_MULTIBYTE;
	    break;
	}
	len = TclUtfToUniChar(src, &ch);

	/*
	 * Check for illegal characters.
	 */

	if (ch > 0xFF) {
	    if (PROFILE_STRICT(flags)) {
		result = TCL_CONVERT_UNKNOWN;
		break;
	    }
	    /*
	     * Plunge on, using '?' as a fallback character.
	     */

	    ch = '?'; /* Profiles TCL8 and REPLACE */
	}

	if (dst > dstEnd) {
	    result = TCL_CONVERT_NOSPACE;
	    break;
	}
	*(dst++) = (char) ch;
	src += len;
    }

    *srcReadPtr = src - srcStart;
    *dstWrotePtr = dst - dstStart;
    *dstCharsPtr = numChars;
    return result;
}

```

tcl8.4 8.4.20, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclEncoding.c`, function `TclInitEncodingSubsystem`, lines 346–386. Line unit: physical LF only. Original byte range [12276, 13438). Full-source SHA-256 `b52a3a4d8385133479cd25a815f3dbb50f66848df7353be164ac2108556240a6`; snippet SHA-256 `6a0a088c3ee2c50b93ba9f1bea5a07fab9828296d21deb1138c7fed41fe6b74e`; retained evidence `source-window-22`.

```text
void
TclInitEncodingSubsystem()
{
    Tcl_EncodingType type;

    Tcl_MutexLock(&encodingMutex);
    Tcl_InitHashTable(&encodingTable, TCL_STRING_KEYS);
    Tcl_MutexUnlock(&encodingMutex);
    
    /*
     * Create a few initial encodings.  Note that the UTF-8 to UTF-8 
     * translation is not a no-op, because it will turn a stream of
     * improperly formed UTF-8 into a properly formed stream.
     */

    type.encodingName	= "identity";
    type.toUtfProc	= BinaryProc;
    type.fromUtfProc	= BinaryProc;
    type.freeProc	= NULL;
    type.nullSize	= 1;
    type.clientData	= NULL;

    defaultEncoding	= Tcl_CreateEncoding(&type);
    systemEncoding	= Tcl_GetEncoding(NULL, type.encodingName);

    type.encodingName	= "utf-8";
    type.toUtfProc	= UtfExtToUtfIntProc;
    type.fromUtfProc	= UtfIntToUtfExtProc;
    type.freeProc	= NULL;
    type.nullSize	= 1;
    type.clientData	= NULL;
    Tcl_CreateEncoding(&type);

    type.encodingName   = "unicode";
    type.toUtfProc	= UnicodeToUtfProc;
    type.fromUtfProc    = UtfToUnicodeProc;
    type.freeProc	= NULL;
    type.nullSize	= 2;
    type.clientData	= NULL;
    Tcl_CreateEncoding(&type);
}

```

tcl9.0 9.0.4, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclEncoding.c`, function `TclInitEncodingSubsystem`, lines 523–659. Line unit: physical LF only. Original byte range [16674, 20842). Full-source SHA-256 `b2d923ff94e392ddbfcb6daf474e58cff1b880d574d6b70ee3153a196f9f7531`; snippet SHA-256 `931e5e68141d7aa16eed37ffd0bf6865f74bab56633f81e932af8431c62608f8`; retained evidence `source-window-23`.

```text
void
TclInitEncodingSubsystem(void)
{
    Tcl_EncodingType type;
    TableEncodingData *dataPtr;
    unsigned size;
    unsigned short i;
    union {
	char c;
	short s;
    } isLe;
    int leFlags;

    if (encodingsInitialized) {
	return;
    }

    /* Note: This DEPENDS on TCL_ENCODING_LE being defined in least sig byte */
    isLe.s = 1;
    leFlags = isLe.c ? TCL_ENCODING_LE : 0;

    Tcl_MutexLock(&encodingMutex);
    Tcl_InitHashTable(&encodingTable, TCL_STRING_KEYS);
    Tcl_MutexUnlock(&encodingMutex);

    /*
     * Create a few initial encodings.  UTF-8 to UTF-8 translation is not a
     * no-op because it turns a stream of improperly formed UTF-8 into a
     * properly formed stream.
     */

    type.encodingName	= NULL;
    type.toUtfProc	= BinaryProc;
    type.fromUtfProc	= BinaryProc;
    type.freeProc	= NULL;
    type.nullSize	= 1;
    type.clientData	= NULL;
    tclIdentityEncoding = Tcl_CreateEncoding(&type);

    type.encodingName	= "utf-8";
    type.toUtfProc	= UtfToUtfProc;
    type.fromUtfProc	= UtfToUtfProc;
    type.freeProc	= NULL;
    type.nullSize	= 1;
    type.clientData	= INT2PTR(ENCODING_UTF);
    tclUtf8Encoding = Tcl_CreateEncoding(&type);
    type.clientData	= NULL;
    type.encodingName	= "cesu-8";
    Tcl_CreateEncoding(&type);

    type.toUtfProc	= Utf16ToUtfProc;
    type.fromUtfProc	= UtfToUcs2Proc;
    type.freeProc	= NULL;
    type.nullSize	= 2;
    type.encodingName	= "ucs-2le";
    type.clientData	= INT2PTR(TCL_ENCODING_LE);
    Tcl_CreateEncoding(&type);
    type.encodingName	= "ucs-2be";
    type.clientData	= NULL;
    Tcl_CreateEncoding(&type);
    type.encodingName	= "ucs-2";
    type.clientData	= INT2PTR(leFlags);
    Tcl_CreateEncoding(&type);

    type.toUtfProc	= Utf32ToUtfProc;
    type.fromUtfProc	= UtfToUtf32Proc;
    type.freeProc	= NULL;
    type.nullSize	= 4;
    type.encodingName	= "utf-32le";
    type.clientData	= INT2PTR(TCL_ENCODING_LE);
    Tcl_CreateEncoding(&type);
    type.encodingName	= "utf-32be";
    type.clientData	= NULL;
    Tcl_CreateEncoding(&type);
    type.encodingName	= "utf-32";
    type.clientData	= INT2PTR(leFlags);
    Tcl_CreateEncoding(&type);

    type.toUtfProc	= Utf16ToUtfProc;
    type.fromUtfProc    = UtfToUtf16Proc;
    type.freeProc	= NULL;
    type.nullSize	= 2;
    type.encodingName	= "utf-16le";
    type.clientData	= INT2PTR(TCL_ENCODING_LE);
    Tcl_CreateEncoding(&type);
    type.encodingName	= "utf-16be";
    type.clientData	= NULL;
    Tcl_CreateEncoding(&type);
    type.encodingName	= "utf-16";
    type.clientData	= INT2PTR(leFlags);
    Tcl_CreateEncoding(&type);

#ifndef TCL_NO_DEPRECATED
    type.encodingName	= "unicode";
    Tcl_CreateEncoding(&type);
#endif

    /*
     * Need the iso8859-1 encoding in order to process binary data, so force
     * it to always be embedded. Note that this encoding *must* be a proper
     * table encoding or some of the escape encodings crash! Hence the ugly
     * code to duplicate the structure of a table encoding here.
     */

    dataPtr = (TableEncodingData *)Tcl_Alloc(sizeof(TableEncodingData));
    memset(dataPtr, 0, sizeof(TableEncodingData));
    dataPtr->fallback = '?';

    size = 256*(sizeof(unsigned short *) + sizeof(unsigned short));
    dataPtr->toUnicode = (unsigned short **)Tcl_Alloc(size);
    memset(dataPtr->toUnicode, 0, size);
    dataPtr->fromUnicode = (unsigned short **)Tcl_Alloc(size);
    memset(dataPtr->fromUnicode, 0, size);

    dataPtr->toUnicode[0] = (unsigned short *) (dataPtr->toUnicode + 256);
    dataPtr->fromUnicode[0] = (unsigned short *) (dataPtr->fromUnicode + 256);
    for (i=1 ; i<256 ; i++) {
	dataPtr->toUnicode[i] = emptyPage;
	dataPtr->fromUnicode[i] = emptyPage;
    }

    for (i=0 ; i<256 ; i++) {
	dataPtr->toUnicode[0][i] = i;
	dataPtr->fromUnicode[0][i] = i;
    }

    type.encodingName	= "iso8859-1";
    type.toUtfProc	= Iso88591ToUtfProc;
    type.fromUtfProc	= Iso88591FromUtfProc;
    type.freeProc	= TableFreeProc;
    type.nullSize	= 1;
    type.clientData	= dataPtr;
    defaultEncoding	= Tcl_CreateEncoding(&type);
    systemEncoding	= Tcl_GetEncoding(NULL, type.encodingName);

    encodingsInitialized = 1;
}

```

tcl9.1 9.1.0, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclEncoding.c`, function `TclInitEncodingSubsystem`, lines 526–667. Line unit: physical LF only. Original byte range [16862, 21139). Full-source SHA-256 `a8e40415c4e984eb2d8b888f79a3fc09fe1074233f8ee3d6b6bdb0b270d8b770`; snippet SHA-256 `b40aa62bbadee3fcf43d51427adcbc4d0063a892fe52a6df05068bfceca41ffe`; retained evidence `source-window-24`.

```text
void
TclInitEncodingSubsystem(void)
{
    Tcl_EncodingType type;
    TableEncodingData *dataPtr;
    unsigned size;
    unsigned short i;
    union {
	char c;
	short s;
    } isLe;
    int leFlags;

    if (encodingsInitialized) {
	return;
    }

    /* Note: This DEPENDS on TCL_ENCODING_LE being defined in least sig byte */
    isLe.s = 1;
    leFlags = isLe.c ? TCL_ENCODING_LE : 0;

    Tcl_MutexLock(&encodingMutex);
    Tcl_InitHashTable(&encodingTable, TCL_STRING_KEYS);
    Tcl_MutexUnlock(&encodingMutex);

    /*
     * Create a few initial encodings.  UTF-8 to UTF-8 translation is not a
     * no-op because it turns a stream of improperly formed UTF-8 into a
     * properly formed stream.
     */

    type.encodingName	= NULL;
    type.toUtfProc	= BinaryProc;
    type.fromUtfProc	= BinaryProc;
    type.freeProc	= NULL;
    type.nullSize	= 1;
    type.clientData	= NULL;
    tclIdentityEncoding = Tcl_CreateEncoding(&type);

    type.encodingName	= "utf-8";
    type.toUtfProc	= UtfToUtfProc;
    type.fromUtfProc	= UtfToUtfProc;
    type.freeProc	= NULL;
    type.nullSize	= 1;
    type.clientData	= NULL;
    tclUtf8Encoding = Tcl_CreateEncoding(&type);

    type.encodingName	= "cesu-8";
    type.toUtfProc	= Cesu8ToUtfProc;
    type.fromUtfProc	= UtfToCesu8Proc;
    type.freeProc	= NULL;
    type.nullSize	= 1;
    type.clientData	= NULL;
    Tcl_CreateEncoding(&type);

    type.toUtfProc	= Utf16ToUtfProc;
    type.fromUtfProc	= UtfToUcs2Proc;
    type.freeProc	= NULL;
    type.nullSize	= 2;
    type.encodingName	= "ucs-2le";
    type.clientData	= INT2PTR(TCL_ENCODING_LE);
    Tcl_CreateEncoding(&type);
    type.encodingName	= "ucs-2be";
    type.clientData	= NULL;
    Tcl_CreateEncoding(&type);
    type.encodingName	= "ucs-2";
    type.clientData	= INT2PTR(leFlags);
    Tcl_CreateEncoding(&type);

    type.toUtfProc	= Utf32ToUtfProc;
    type.fromUtfProc	= UtfToUtf32Proc;
    type.freeProc	= NULL;
    type.nullSize	= 4;
    type.encodingName	= "utf-32le";
    type.clientData	= INT2PTR(TCL_ENCODING_LE);
    Tcl_CreateEncoding(&type);
    type.encodingName	= "utf-32be";
    type.clientData	= NULL;
    Tcl_CreateEncoding(&type);
    type.encodingName	= "utf-32";
    type.clientData	= INT2PTR(leFlags);
    Tcl_CreateEncoding(&type);

    type.toUtfProc	= Utf16ToUtfProc;
    type.fromUtfProc    = UtfToUtf16Proc;
    type.freeProc	= NULL;
    type.nullSize	= 2;
    type.encodingName	= "utf-16le";
    type.clientData	= INT2PTR(TCL_ENCODING_LE);
    Tcl_CreateEncoding(&type);
    type.encodingName	= "utf-16be";
    type.clientData	= NULL;
    Tcl_CreateEncoding(&type);
    type.encodingName	= "utf-16";
    type.clientData	= INT2PTR(leFlags);
    Tcl_CreateEncoding(&type);

#ifndef TCL_NO_DEPRECATED
    type.encodingName	= "unicode";
    Tcl_CreateEncoding(&type);
#endif

    /*
     * Need the iso8859-1 encoding in order to process binary data, so force
     * it to always be embedded. Note that this encoding *must* be a proper
     * table encoding or some of the escape encodings crash! Hence the ugly
     * code to duplicate the structure of a table encoding here.
     */

    dataPtr = (TableEncodingData *)Tcl_Alloc(sizeof(TableEncodingData));
    memset(dataPtr, 0, sizeof(TableEncodingData));
    dataPtr->fallback = '?';

    size = 256*(sizeof(unsigned short *) + sizeof(unsigned short));
    dataPtr->toUnicode = (unsigned short **)Tcl_Alloc(size);
    memset(dataPtr->toUnicode, 0, size);
    dataPtr->fromUnicode = (unsigned short **)Tcl_Alloc(size);
    memset(dataPtr->fromUnicode, 0, size);

    dataPtr->toUnicode[0] = (unsigned short *) (dataPtr->toUnicode + 256);
    dataPtr->fromUnicode[0] = (unsigned short *) (dataPtr->fromUnicode + 256);
    for (i=1 ; i<256 ; i++) {
	dataPtr->toUnicode[i] = emptyPage;
	dataPtr->fromUnicode[i] = emptyPage;
    }

    for (i=0 ; i<256 ; i++) {
	dataPtr->toUnicode[0][i] = i;
	dataPtr->fromUnicode[0][i] = i;
    }

    type.encodingName	= "iso8859-1";
    type.toUtfProc	= Iso88591ToUtfProc;
    type.fromUtfProc	= Iso88591FromUtfProc;
    type.freeProc	= TableFreeProc;
    type.nullSize	= 1;
    type.clientData	= dataPtr;
    defaultEncoding	= Tcl_CreateEncoding(&type);
    systemEncoding	= Tcl_GetEncoding(NULL, type.encodingName);

    encodingsInitialized = 1;
}

```

tcl9.0 9.0.4, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclEncoding.c`, function `Tcl_UtfToExternalDStringEx`, lines 1494–1613. Line unit: physical LF only. Original byte range [45751, 49495). Full-source SHA-256 `b2d923ff94e392ddbfcb6daf474e58cff1b880d574d6b70ee3153a196f9f7531`; snippet SHA-256 `4fbfc390445266168e291b31c87bcec9e6d6d1672185e42e85acce099c9d80f7`; retained evidence `source-window-25`.

```text
int
Tcl_UtfToExternalDStringEx(
    Tcl_Interp *interp,		/* For error messages. May be NULL. */
    Tcl_Encoding encoding,	/* The encoding for the converted string, or
				 * NULL for the default system encoding. */
    const char *src,		/* Source string in UTF-8. */
    Tcl_Size srcLen,		/* Source string length in bytes, or < 0 for
				 * strlen(). */
    int flags,			/* Conversion control flags. */
    Tcl_DString *dstPtr,	/* Uninitialized or free DString in which the
				 * converted string is stored. */
    Tcl_Size *errorLocPtr)	/* Where to store the error location
				 * (or TCL_INDEX_NONE if no error). May
				 * be NULL. */
{
    char *dst;
    Tcl_EncodingState state;
    const Encoding *encodingPtr;
    int result;
    const char *srcStart = src;
    Tcl_Size dstLen, soFar;

    /* DO FIRST - must always be initialized on return */
    Tcl_DStringInit(dstPtr);

    dst = Tcl_DStringValue(dstPtr);
    dstLen = dstPtr->spaceAvl - 1;

    if (encoding == NULL) {
	encoding = systemEncoding;
    }
    encodingPtr = (Encoding *) encoding;

    if (src == NULL) {
	srcLen = 0;
	src = "";
    } else if (srcLen < 0) {
	srcLen = strlen(src);
    }

    flags &= ~TCL_ENCODING_END;
    flags |= TCL_ENCODING_START;
    while (1) {
	int srcChunkLen, srcChunkRead;
	int dstChunkLen, dstChunkWrote, dstChunkChars;

	if (srcLen > INT_MAX) {
	    srcChunkLen = INT_MAX;
	} else {
	    srcChunkLen = srcLen;
	    flags |= TCL_ENCODING_END; /* Last chunk */
	}
	dstChunkLen = dstLen > INT_MAX ? INT_MAX : dstLen;

	result = encodingPtr->fromUtfProc(encodingPtr->clientData, src,
		srcChunkLen, flags, &state, dst, dstChunkLen,
		&srcChunkRead, &dstChunkWrote, &dstChunkChars);
	soFar = dst + dstChunkWrote - Tcl_DStringValue(dstPtr);

	/* Move past the part processed in this go around */
	src += srcChunkRead;

	/*
	 * Keep looping in two case -
	 *   - our destination buffer did not have enough room
	 *   - we had not passed in all the data and error indicated fragment
	 *     of a multibyte character
	 * In both cases we have to grow buffer, move the input source pointer
	 * and loop. Otherwise, return the result we got.
	 */
	if ((result != TCL_CONVERT_NOSPACE) &&
		(result != TCL_CONVERT_MULTIBYTE || (flags & TCL_ENCODING_END))) {
	    Tcl_Size nBytesProcessed = (src - srcStart);
	    Tcl_Size i = soFar + encodingPtr->nullSize - 1;
	    /* Loop as DStringSetLength only stores one nul byte at a time */
	    while (i >= soFar) {
		Tcl_DStringSetLength(dstPtr, i--);
	    }
	    if (errorLocPtr) {
		/*
		 * Do not write error message into interpreter if caller
		 * wants to know error location.
		 */
		*errorLocPtr = result == TCL_OK
			? TCL_INDEX_NONE : nBytesProcessed;
	    } else {
		/* Caller wants error message on failure */
		if (result != TCL_OK && interp != NULL) {
		    Tcl_Size pos = Tcl_NumUtfChars(srcStart, nBytesProcessed);
		    int ucs4;
		    char buf[TCL_INTEGER_SPACE];

		    TclUtfToUniChar(&srcStart[nBytesProcessed], &ucs4);
		    snprintf(buf, sizeof(buf), "%" TCL_SIZE_MODIFIER "d",
			    nBytesProcessed);
		    Tcl_SetObjResult(interp, Tcl_ObjPrintf(
			    "unexpected character at index %" TCL_SIZE_MODIFIER
			    "u: 'U+%06X'",
			    pos, ucs4));
		    Tcl_SetErrorCode(interp, "TCL", "ENCODING", "ILLEGALSEQUENCE",
			    buf, (char *)NULL);
		}
	    }
	    if (result != TCL_OK) {
		errno = (result == TCL_CONVERT_NOSPACE) ? ENOMEM : EILSEQ;
	    }
	    return result;
	}

	flags &= ~TCL_ENCODING_START;
	srcLen -= srcChunkRead;

	if (Tcl_DStringLength(dstPtr) == 0) {
	    Tcl_DStringSetLength(dstPtr, dstLen);
	}
	Tcl_DStringSetLength(dstPtr, 2 * Tcl_DStringLength(dstPtr) + 1);
	dst = Tcl_DStringValue(dstPtr) + soFar;
	dstLen = Tcl_DStringLength(dstPtr) - soFar - 1;
    }
}

```

tcl9.1 9.1.0, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclEncoding.c`, function `Tcl_UtfToExternalDStringEx`, lines 1744–1863. Line unit: physical LF only. Original byte range [54661, 58459). Full-source SHA-256 `a8e40415c4e984eb2d8b888f79a3fc09fe1074233f8ee3d6b6bdb0b270d8b770`; snippet SHA-256 `2372667acfbd14860cf3d2407f7a20b9531509cdf3d193424594b68b9f92a7f7`; retained evidence `source-window-26`.

```text
int
Tcl_UtfToExternalDStringEx(
    Tcl_Interp *interp,		/* For error messages. May be NULL. */
    Tcl_Encoding encoding,	/* The encoding for the converted string, or
				 * NULL for the default system encoding. */
    const char *src,		/* Source string in UTF-8. */
    Tcl_Size srcLen,		/* Source string length in bytes, or < 0 for
				 * strlen(). */
    int flags,			/* Conversion control flags. */
    Tcl_DString *dstPtr,	/* Uninitialized or free DString in which the
				 * converted string is stored. */
    Tcl_Size *errorLocPtr)	/* Where to store the error location
				 * (or TCL_INDEX_NONE if no error). May
				 * be NULL. */
{
    char *dst;
    Tcl_EncodingState state;
    const Encoding *encodingPtr;
    int result;
    const char *srcStart = src;
    Tcl_Size dstLen, soFar;

    /* DO FIRST - must always be initialized on return */
    Tcl_DStringInit(dstPtr);

    dst = Tcl_DStringValue(dstPtr);
    dstLen = dstPtr->spaceAvl - 1;

    if (encoding == NULL) {
	encoding = systemEncoding;
    }
    encodingPtr = (Encoding *) encoding;

    if (src == NULL) {
	srcLen = 0;
	src = "";	/* Avoid ABSAN warnings about ops on NULL pointers */
    } else if (srcLen < 0) {
	srcLen = strlen(src);
    }

    flags &= ~TCL_ENCODING_END;
    flags |= TCL_ENCODING_START;
    while (1) {
	int srcChunkLen, srcChunkRead;
	int dstChunkLen, dstChunkWrote, dstChunkChars;

	if (srcLen > INT_MAX) {
	    srcChunkLen = INT_MAX;
	} else {
	    srcChunkLen = srcLen;
	    flags |= TCL_ENCODING_END; /* Last chunk */
	}
	dstChunkLen = dstLen > INT_MAX ? INT_MAX : dstLen;

	result = encodingPtr->fromUtfProc(encodingPtr->clientData, src,
		srcChunkLen, flags, &state, dst, dstChunkLen,
		&srcChunkRead, &dstChunkWrote, &dstChunkChars);
	soFar = dst + dstChunkWrote - Tcl_DStringValue(dstPtr);

	/* Move past the part processed in this go around */
	src += srcChunkRead;

	/*
	 * Keep looping in two case -
	 *   - our destination buffer did not have enough room
	 *   - we had not passed in all the data and error indicated fragment
	 *     of a multibyte character
	 * In both cases we have to grow buffer, move the input source pointer
	 * and loop. Otherwise, return the result we got.
	 */
	if ((result != TCL_CONVERT_NOSPACE) &&
		(result != TCL_CONVERT_MULTIBYTE || (flags & TCL_ENCODING_END))) {
	    Tcl_Size nBytesProcessed = (src - srcStart);
	    Tcl_Size i = soFar + encodingPtr->nullSize - 1;
	    /* Loop as DStringSetLength only stores one nul byte at a time */
	    while (i >= soFar) {
		Tcl_DStringSetLength(dstPtr, i--);
	    }
	    if (errorLocPtr) {
		/*
		 * Do not write error message into interpreter if caller
		 * wants to know error location.
		 */
		*errorLocPtr = result == TCL_OK
			? TCL_INDEX_NONE : nBytesProcessed;
	    } else {
		/* Caller wants error message on failure */
		if (result != TCL_OK && interp != NULL) {
		    Tcl_Size pos = Tcl_NumUtfChars(srcStart, nBytesProcessed);
		    int ucs4;
		    char buf[TCL_INTEGER_SPACE];

		    TclUtfToUniChar(&srcStart[nBytesProcessed], &ucs4);
		    snprintf(buf, sizeof(buf), "%" TCL_SIZE_MODIFIER "d",
			    nBytesProcessed);
		    Tcl_SetObjResult(interp, Tcl_ObjPrintf(
			    "unexpected character at index %" TCL_SIZE_MODIFIER
			    "u: 'U+%06X'",
			    pos, ucs4));
		    Tcl_SetErrorCode(interp, "TCL", "ENCODING", "ILLEGALSEQUENCE",
			    buf, (char *)NULL);
		}
	    }
	    if (result != TCL_OK) {
		errno = (result == TCL_CONVERT_NOSPACE) ? ENOMEM : EILSEQ;
	    }
	    return result;
	}

	flags &= ~TCL_ENCODING_START;
	srcLen -= srcChunkRead;

	if (Tcl_DStringLength(dstPtr) == 0) {
	    Tcl_DStringSetLength(dstPtr, dstLen);
	}
	Tcl_DStringSetLength(dstPtr, 2 * Tcl_DStringLength(dstPtr) + 1);
	dst = Tcl_DStringValue(dstPtr) + soFar;
	dstLen = Tcl_DStringLength(dstPtr) - soFar - 1;
    }
}

```

tcl8.4 8.4.20, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclEncoding.c`, function `UtfToUtfProc`, lines 2007–2105. Line unit: physical LF only. Original byte range [60882, 64085). Full-source SHA-256 `b52a3a4d8385133479cd25a815f3dbb50f66848df7353be164ac2108556240a6`; snippet SHA-256 `f93d119d803238db3b634cd8a76abe094c2d754b0acfda0325e1519575152fdc`; retained evidence `source-window-27`.

```text
static int 
UtfToUtfProc(clientData, src, srcLen, flags, statePtr, dst, dstLen,
	     srcReadPtr, dstWrotePtr, dstCharsPtr, pureNullMode)
    ClientData clientData;	/* Not used. */
    CONST char *src;		/* Source string in UTF-8. */
    int srcLen;			/* Source string length in bytes. */
    int flags;			/* Conversion control flags. */
    Tcl_EncodingState *statePtr;/* Place for conversion routine to store
				 * state information used during a piecewise
				 * conversion.  Contents of statePtr are
				 * initialized and/or reset by conversion
				 * routine under control of flags argument. */
    char *dst;			/* Output buffer in which converted string
				 * is stored. */
    int dstLen;			/* The maximum length of output buffer in
				 * bytes. */
    int *srcReadPtr;		/* Filled with the number of bytes from the
				 * source string that were converted.  This
				 * may be less than the original source length
				 * if there was a problem converting some
				 * source characters. */
    int *dstWrotePtr;		/* Filled with the number of bytes that were
				 * stored in the output buffer as a result of
				 * the conversion. */
    int *dstCharsPtr;		/* Filled with the number of characters that
				 * correspond to the bytes stored in the
				 * output buffer. */
    int pureNullMode;		/* Convert embedded nulls from
				 * internal representation to real
				 * null-bytes or vice versa */

{
    CONST char *srcStart, *srcEnd, *srcClose;
    char *dstStart, *dstEnd;
    int result, numChars;
    Tcl_UniChar ch;

    result = TCL_OK;
    
    srcStart = src;
    srcEnd = src + srcLen;
    srcClose = srcEnd;
    if ((flags & TCL_ENCODING_END) == 0) {
	srcClose -= TCL_UTF_MAX;
    }

    dstStart = dst;
    dstEnd = dst + dstLen - TCL_UTF_MAX;

    for (numChars = 0; src < srcEnd; numChars++) {
	if ((src > srcClose) && (!Tcl_UtfCharComplete(src, srcEnd - src))) {
	    /*
	     * If there is more string to follow, this will ensure that the
	     * last UTF-8 character in the source buffer hasn't been cut off.
	     */

	    result = TCL_CONVERT_MULTIBYTE;
	    break;
	}
	if (dst > dstEnd) {
	    result = TCL_CONVERT_NOSPACE;
	    break;
	}
	if (UCHAR(*src) < 0x80 &&
	    !(UCHAR(*src) == 0 && pureNullMode == 0)) {
	    /*
	     * Copy 7bit chatacters, but skip null-bytes when we are
	     * in input mode, so that they get converted to 0xc080.
	     */
	    *dst++ = *src++;
	} else if (pureNullMode == 1 &&
		   UCHAR(*src) == 0xc0 &&
		   UCHAR(*(src+1)) == 0x80) {
	    /* 
	     * Convert 0xc080 to real nulls when we are in output mode.
	     */
	    *dst++ = 0;
	    src += 2;
	} else if (!Tcl_UtfCharComplete(src, srcEnd - src)) {
	    /* Always check before using Tcl_UtfToUniChar. Not doing
	     * can so cause it run beyond the endof the buffer!  If we
	     * happen such an incomplete char its bytes are made to
	     * represent themselves.
	     */

	    ch = (unsigned char) *src;
	    src += 1;
	    dst += Tcl_UniCharToUtf(ch, dst);
	} else {
	    src += Tcl_UtfToUniChar(src, &ch);
	    dst += Tcl_UniCharToUtf(ch, dst);
	}
    }

    *srcReadPtr  = src - srcStart;
    *dstWrotePtr = dst - dstStart;
    *dstCharsPtr = numChars;
    return result;
}

```

tcl8.5 8.5.19, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclEncoding.c`, function `UtfToUtfProc`, lines 2217–2313. Line unit: physical LF only. Original byte range [66200, 69291). Full-source SHA-256 `b100161e6fbcdc9b4055b28308071fa123000526abcb51dc1a65eeb83e94b292`; snippet SHA-256 `0959214c9bea8bb05df0e9acbee4fb4ec3f5bb135cdc8eb17f8a5ae43aff03fe`; retained evidence `source-window-28`.

```text
static int
UtfToUtfProc(
    ClientData clientData,	/* Not used. */
    const char *src,		/* Source string in UTF-8. */
    int srcLen,			/* Source string length in bytes. */
    int flags,			/* Conversion control flags. */
    Tcl_EncodingState *statePtr,/* Place for conversion routine to store state
				 * information used during a piecewise
				 * conversion. Contents of statePtr are
				 * initialized and/or reset by conversion
				 * routine under control of flags argument. */
    char *dst,			/* Output buffer in which converted string is
				 * stored. */
    int dstLen,			/* The maximum length of output buffer in
				 * bytes. */
    int *srcReadPtr,		/* Filled with the number of bytes from the
				 * source string that were converted. This may
				 * be less than the original source length if
				 * there was a problem converting some source
				 * characters. */
    int *dstWrotePtr,		/* Filled with the number of bytes that were
				 * stored in the output buffer as a result of
				 * the conversion. */
    int *dstCharsPtr,		/* Filled with the number of characters that
				 * correspond to the bytes stored in the
				 * output buffer. */
    int pureNullMode)		/* Convert embedded nulls from internal
				 * representation to real null-bytes or vice
				 * versa. */
{
    const char *srcStart, *srcEnd, *srcClose;
    char *dstStart, *dstEnd;
    int result, numChars;
    Tcl_UniChar ch;

    result = TCL_OK;

    srcStart = src;
    srcEnd = src + srcLen;
    srcClose = srcEnd;
    if ((flags & TCL_ENCODING_END) == 0) {
	srcClose -= TCL_UTF_MAX;
    }

    dstStart = dst;
    dstEnd = dst + dstLen - TCL_UTF_MAX;

    for (numChars = 0; src < srcEnd; numChars++) {
	if ((src > srcClose) && (!Tcl_UtfCharComplete(src, srcEnd - src))) {
	    /*
	     * If there is more string to follow, this will ensure that the
	     * last UTF-8 character in the source buffer hasn't been cut off.
	     */

	    result = TCL_CONVERT_MULTIBYTE;
	    break;
	}
	if (dst > dstEnd) {
	    result = TCL_CONVERT_NOSPACE;
	    break;
	}
	if (UCHAR(*src) < 0x80 && !(UCHAR(*src) == 0 && pureNullMode == 0)) {
	    /*
	     * Copy 7bit chatacters, but skip null-bytes when we are in input
	     * mode, so that they get converted to 0xc080.
	     */

	    *dst++ = *src++;
	} else if (pureNullMode == 1 && UCHAR(*src) == 0xc0 &&
		(src + 1 < srcEnd) && UCHAR(*(src+1)) == 0x80) {
	    /*
	     * Convert 0xc080 to real nulls when we are in output mode.
	     */

	    *dst++ = 0;
	    src += 2;
	} else if (!Tcl_UtfCharComplete(src, srcEnd - src)) {
	    /*
	     * Always check before using Tcl_UtfToUniChar. Not doing can so
	     * cause it run beyond the endof the buffer! If we happen such an
	     * incomplete char its byts are made to represent themselves.
	     */

	    ch = (unsigned char) *src;
	    src += 1;
	    dst += Tcl_UniCharToUtf(ch, dst);
	} else {
	    src += Tcl_UtfToUniChar(src, &ch);
	    dst += Tcl_UniCharToUtf(ch, dst);
	}
    }

    *srcReadPtr = src - srcStart;
    *dstWrotePtr = dst - dstStart;
    *dstCharsPtr = numChars;
    return result;
}

```

tcl8.6 8.6.18, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclEncoding.c`, function `UtfToUtfProc`, lines 2275–2420. Line unit: physical LF only. Original byte range [66906, 71888). Full-source SHA-256 `359bfa493e7aba998e524978aca4345422e2e265dde93e73f76cb5fc8784e4c6`; snippet SHA-256 `daf9a4a731ba652ebc67b4dc02a45d2784caf8398bc430596df49fdd3cf4251d`; retained evidence `source-window-29`.

```text
static int
UtfToUtfProc(
    ClientData clientData,	/* Not used. */
    const char *src,		/* Source string in UTF-8. */
    int srcLen,			/* Source string length in bytes. */
    int flags,			/* Conversion control flags. */
    Tcl_EncodingState *statePtr,/* Place for conversion routine to store state
				 * information used during a piecewise
				 * conversion. Contents of statePtr are
				 * initialized and/or reset by conversion
				 * routine under control of flags argument. */
    char *dst,			/* Output buffer in which converted string is
				 * stored. */
    int dstLen,			/* The maximum length of output buffer in
				 * bytes. */
    int *srcReadPtr,		/* Filled with the number of bytes from the
				 * source string that were converted. This may
				 * be less than the original source length if
				 * there was a problem converting some source
				 * characters. */
    int *dstWrotePtr,		/* Filled with the number of bytes that were
				 * stored in the output buffer as a result of
				 * the conversion. */
    int *dstCharsPtr,		/* Filled with the number of characters that
				 * correspond to the bytes stored in the
				 * output buffer. */
    int pureNullMode)		/* Convert embedded nulls from internal
				 * representation to real null-bytes or vice
				 * versa. Also combine or separate surrogate pairs */
{
    const char *srcStart, *srcEnd, *srcClose;
    const char *dstStart, *dstEnd;
    int result, numChars, charLimit = INT_MAX;
    Tcl_UniChar *chPtr = (Tcl_UniChar *) statePtr;

    if (flags & TCL_ENCODING_START) {
    	*statePtr = 0;
    }
    result = TCL_OK;

    srcStart = src;
    srcEnd = src + srcLen;
    srcClose = srcEnd;
    if ((flags & TCL_ENCODING_END) == 0) {
	srcClose -= 6;
    }
    if (flags & TCL_ENCODING_CHAR_LIMIT) {
	charLimit = *dstCharsPtr;
    }

    dstStart = dst;
    dstEnd = dst + dstLen - ((pureNullMode == 1) ? 4 : TCL_UTF_MAX);

    for (numChars = 0; src < srcEnd && numChars <= charLimit; numChars++) {
	if ((src > srcClose) && (!Tcl_UtfCharComplete(src, srcEnd - src))) {
	    /*
	     * If there is more string to follow, this will ensure that the
	     * last UTF-8 character in the source buffer hasn't been cut off.
	     */

	    result = TCL_CONVERT_MULTIBYTE;
	    break;
	}
	if (dst > dstEnd) {
	    result = TCL_CONVERT_NOSPACE;
	    break;
	}
	if (UCHAR(*src) < 0x80 && !((UCHAR(*src) == 0) && (pureNullMode == 0))) {
	    /*
	     * Copy 7bit characters, but skip null-bytes when we are in input
	     * mode, so that they get converted to 0xC080.
	     */

	    *dst++ = *src++;
	    *chPtr = 0; /* reset surrogate handling */
	} else if ((UCHAR(*src) == 0xC0) && (src + 1 < srcEnd)
		&& (UCHAR(src[1]) == 0x80) && (pureNullMode == 1)) {
	    /*
	     * Convert 0xC080 to real nulls when we are in output mode.
	     */

	    *dst++ = 0;
	    *chPtr = 0; /* reset surrogate handling */
	    src += 2;
	} else if (!Tcl_UtfCharComplete(src, srcEnd - src)) {
	    /*
	     * Always check before using TclUtfToUniChar. Not doing can so
	     * cause it run beyond the end of the buffer! If we happen such an
	     * incomplete char its bytes are made to represent themselves
	     * unless the user has explicitly asked to be told.
	     */

	    if ((flags & TCL_ENCODING_STOPONERROR) && (pureNullMode == 0)) {
		result = TCL_CONVERT_MULTIBYTE;
		break;
	    }
	    *chPtr = UCHAR(*src);
	    src += 1;
	    dst += Tcl_UniCharToUtf(*chPtr, dst);
	} else {
	    size_t len = TclUtfToUniChar(src, chPtr);
	    if ((len < 2) && (*chPtr != 0) && (flags & TCL_ENCODING_STOPONERROR)
		    && ((*chPtr & ~0x7FF) != 0xD800) && (pureNullMode == 0)) {
		result = TCL_CONVERT_SYNTAX;
		break;
	    }
	    src += len;
	    if ((*chPtr & ~0x7FF) == 0xD800) {
		Tcl_UniChar low;
		/* A surrogate character is detected, handle especially */
#if TCL_UTF_MAX <= 4
	    if ((len < 3) && ((src[3 - len] & 0xC0) != 0x80)) {
	    /* It's invalid. See [ed29806ba] */
		*chPtr = UCHAR(src[-1]);
		dst += Tcl_UniCharToUtf(*chPtr, dst);
		continue;
	    }
#endif
		low = *chPtr;
		len = (src <= srcEnd-3) ? Tcl_UtfToUniChar(src, &low) : 0;
		if (((low & ~0x3FF) != 0xDC00) || (*chPtr & 0x400)) {
		    *dst++ = (char) (((*chPtr >> 12) | 0xE0) & 0xEF);
		    *dst++ = (char) (((*chPtr >> 6) | 0x80) & 0xBF);
		    *dst++ = (char) ((*chPtr | 0x80) & 0xBF);
		    *chPtr = 0; /* reset surrogate handling */
		    continue;
		} else if ((TCL_UTF_MAX > 3) || (pureNullMode == 1)) {
		    int full = (((*chPtr & 0x3FF) << 10) | (low & 0x3FF)) + 0x10000;
		    *dst++ = (char) (((full >> 18) | 0xF0) & 0xF7);
		    *dst++ = (char) (((full >> 12) | 0x80) & 0xBF);
		    *dst++ = (char) (((full >> 6) | 0x80) & 0xBF);
		    *dst++ = (char) ((full | 0x80) & 0xBF);
			*chPtr = 0; /* reset surrogate handling */
		    src += len;
		    continue;
		}
	    }
	    dst += Tcl_UniCharToUtf(*chPtr, dst);
	}
    }

    *srcReadPtr = src - srcStart;
    *dstWrotePtr = dst - dstStart;
    *dstCharsPtr = numChars;
    return result;
}

```

tcl9.0 9.0.4, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclEncoding.c`, function `UtfToUtfProc`, lines 2440–2743. Line unit: physical LF only. Original byte range [72928, 82701). Full-source SHA-256 `b2d923ff94e392ddbfcb6daf474e58cff1b880d574d6b70ee3153a196f9f7531`; snippet SHA-256 `5f7a8b0ac8c514bb18db617835d3038482ff55136cb8a0f2f550996facfe2078`; retained evidence `source-window-30`.

```text
static int
UtfToUtfProc(
    void *clientData,		/* additional flags */
    const char *src,		/* Source string in UTF-8. */
    int srcLen,			/* Source string length in bytes. */
    int flags,			/* TCL_ENCODING_* conversion control flags. */
    Tcl_EncodingState *statePtr,/* Place for conversion routine to store state
				 * information used during a piecewise
				 * conversion. Contents of statePtr are
				 * initialized and/or reset by conversion
				 * routine under control of flags argument. */
    char *dst,			/* Output buffer in which converted string is
				 * stored. */
    int dstLen,			/* The maximum length of output buffer in
				 * bytes. */
    int *srcReadPtr,		/* Filled with the number of bytes from the
				 * source string that were converted. This may
				 * be less than the original source length if
				 * there was a problem converting some source
				 * characters. */
    int *dstWrotePtr,		/* Filled with the number of bytes that were
				 * stored in the output buffer as a result of
				 * the conversion. */
    int *dstCharsPtr)		/* Filled with the number of characters that
				 * correspond to the bytes stored in the
				 * output buffer. */
{
    const char *srcStart, *srcEnd, *srcClose;
    const char *dstStart, *dstEnd;
    int result, numChars, charLimit = INT_MAX;
    int ch;
    int profile;

    if (flags & TCL_ENCODING_START) {
	/* *statePtr will hold high surrogate in a split surrogate pair */
	*statePtr = 0;
    }
    result = TCL_OK;

    srcStart = src;
    srcEnd = src + srcLen;
    srcClose = srcEnd;
    if ((flags & TCL_ENCODING_END) == 0) {
	srcClose -= 6;
    }
    if (flags & TCL_ENCODING_CHAR_LIMIT) {
	charLimit = *dstCharsPtr;
    }

    dstStart = dst;
    flags |= PTR2INT(clientData);

    /*
     * If output is UTF-8 or encoding for Tcl's internal encoding,
     * max space needed is TCL_UTF_MAX. Otherwise, need 6 bytes (CESU-8)
     */
    dstEnd = dst + dstLen - ((flags & (ENCODING_INPUT|ENCODING_UTF)) ? TCL_UTF_MAX : 6);

    /*
     * Macro to output an isolated high surrogate when it is not followed
     * by a low surrogate. NOT to be called for strict profile since
     * that should raise an error.
     */
#define OUTPUT_ISOLATEDSURROGATE                                \
    do {                                                        \
	Tcl_UniChar high;                                       \
	if (PROFILE_REPLACE(profile)) {                         \
	    high = UNICODE_REPLACE_CHAR;                        \
	} else {                                                \
	    high = (Tcl_UniChar)(ptrdiff_t) *statePtr;          \
	}                                                       \
	assert(!(flags & ENCODING_UTF)); /* Must be CESU-8 */   \
	assert(HIGH_SURROGATE(high));                           \
	assert(!PROFILE_STRICT(profile));                       \
	dst += Tcl_UniCharToUtf(high, dst);                     \
	*statePtr = 0; /* Reset state */                        \
    } while (0)

    /*
     * Macro to check for isolated surrogate and either break with
     * an error if profile is strict, or output an appropriate
     * character for replace and tcl8 profiles and continue.
     */
#define CHECK_ISOLATEDSURROGATE                                         \
    if (*statePtr) {                                                    \
	if (PROFILE_STRICT(profile)) {                                  \
	    result = TCL_CONVERT_SYNTAX;                                \
	    break;                                                      \
	}                                                               \
	OUTPUT_ISOLATEDSURROGATE;                                       \
	continue; /* Rerun loop so length checks etc. repeated */       \
    } else                                                              \
	(void) 0

    profile = ENCODING_PROFILE_GET(flags);
    for (numChars = 0; src < srcEnd && numChars <= charLimit; numChars++) {

	if ((src > srcClose) && (!Tcl_UtfCharComplete(src, srcEnd - src))) {
	    /*
	     * If there is more string to follow, this will ensure that the
	     * last UTF-8 character in the source buffer hasn't been cut off.
	     */

	    result = TCL_CONVERT_MULTIBYTE;
	    break;
	}
	if (dst > dstEnd) {
	    result = TCL_CONVERT_NOSPACE;
	    break;
	}
	if (UCHAR(*src) < 0x80 && !((UCHAR(*src) == 0) && (flags & ENCODING_INPUT))) {

	    CHECK_ISOLATEDSURROGATE;
	    /*
	     * Copy 7bit characters, but skip null-bytes when we are in input
	     * mode, so that they get converted to \xC0\x80.
	     */
	    *dst++ = *src++;
	} else if ((UCHAR(*src) == 0xC0) && (src + 1 < srcEnd) &&
		 (UCHAR(src[1]) == 0x80) &&
		 (!(flags & ENCODING_INPUT) || !PROFILE_TCL8(profile))) {
	    /* Special sequence \xC0\x80 */

	    CHECK_ISOLATEDSURROGATE;
	    if (!PROFILE_TCL8(profile) && (flags & ENCODING_INPUT)) {
		if (PROFILE_REPLACE(profile)) {
		    dst += Tcl_UniCharToUtf(UNICODE_REPLACE_CHAR, dst);
		    src += 2;
		} else {
		    /* PROFILE_STRICT */
		    result = TCL_CONVERT_SYNTAX;
		    break;
		}
	    } else {
		/*
		 * Convert 0xC080 to real nulls when we are in output mode,
		 * irrespective of the profile.
		 */
		*dst++ = 0;
		src += 2;
	    }

	} else if (!Tcl_UtfCharComplete(src, srcEnd - src)) {
	    /*
	     * Incomplete byte sequence not because there are insufficient
	     * bytes in source buffer (have already checked that above) but
	     * because the UTF-8 sequence is truncated.
	     */

	    CHECK_ISOLATEDSURROGATE;

	    if (flags & ENCODING_INPUT) {
		/* Incomplete bytes for modified UTF-8 target */
		if (PROFILE_STRICT(profile)) {
		    result = (flags & TCL_ENCODING_CHAR_LIMIT)
			    ? TCL_CONVERT_MULTIBYTE
			    : TCL_CONVERT_SYNTAX;
		    break;
		}
	    }
	    if (PROFILE_REPLACE(profile)) {
		ch = UNICODE_REPLACE_CHAR;
		++src;
	    } else {
		/* TCL_ENCODING_PROFILE_TCL8 */
		char chbuf[2];
		chbuf[0] = UCHAR(*src++);
		chbuf[1] = 0;
		TclUtfToUniChar(chbuf, &ch);
	    }
	    dst += Tcl_UniCharToUtf(ch, dst);
	} else {
	    /* Have a complete character */
	    size_t len = TclUtfToUniChar(src, &ch);

	    Tcl_UniChar savedSurrogate = (Tcl_UniChar) (ptrdiff_t)*statePtr;
	    *statePtr = 0; /* Reset surrogate */

	    if (flags & ENCODING_INPUT) {
		if (((len < 2) && (ch != 0)) || ((ch > 0xFFFF) && !(flags & ENCODING_UTF))) {
		    if (PROFILE_STRICT(profile)) {
			result = TCL_CONVERT_SYNTAX;
			break;
		    } else if (PROFILE_REPLACE(profile)) {
			ch = UNICODE_REPLACE_CHAR;
		    }
		}
	    }

	    const char *saveSrc = src;
	    src += len;
	    if (!(flags & ENCODING_UTF) && !(flags & ENCODING_INPUT)
		    && (ch > 0x7FF)) {
		assert(savedSurrogate == 0); /* Since this flag combo
						will never set *statePtr */
		if (ch > 0xFFFF) {
		    /* CESU-8 6-byte sequence for chars > U+FFFF */
		    ch -= 0x10000;
		    *dst++ = 0xED;
		    *dst++ = (char) (((ch >> 16) & 0x0F) | 0xA0);
		    *dst++ = (char) (((ch >> 10) & 0x3F) | 0x80);
		    ch = (ch & 0x03FF) | 0xDC00;
		}
		*dst++ = (char)(((ch >> 12) | 0xE0) & 0xEF);
		*dst++ = (char)(((ch >> 6) | 0x80) & 0xBF);
		*dst++ = (char)((ch | 0x80) & 0xBF);
		continue;
	    } else if (SURROGATE(ch)) {
		if ((flags & ENCODING_UTF)) {
		    /* UTF-8, not CESU-8, so surrogates should not appear */
		    if (PROFILE_STRICT(profile)) {
			result = (flags & ENCODING_INPUT)
			    ? TCL_CONVERT_SYNTAX : TCL_CONVERT_UNKNOWN;
			src = saveSrc;
			break;
		    } else if (PROFILE_REPLACE(profile)) {
			ch = UNICODE_REPLACE_CHAR;
		    } else {
			/* PROFILE_TCL8 - output as is */
		    }
		} else {
		    /* CESU-8 */
		    if (LOW_SURROGATE(ch)) {
			if (savedSurrogate) {
			    assert(HIGH_SURROGATE(savedSurrogate));
			    ch = 0x10000 + ((savedSurrogate - 0xd800) << 10) + (ch - 0xdc00);
			} else {
			    /* Isolated low surrogate */
			    if (PROFILE_STRICT(profile)) {
				result = (flags & ENCODING_INPUT)
				    ? TCL_CONVERT_SYNTAX : TCL_CONVERT_UNKNOWN;
				src = saveSrc;
				break;
			    } else if (PROFILE_REPLACE(profile)) {
				ch = UNICODE_REPLACE_CHAR;
			    } else {
				/* Tcl8 profile. Output low surrogate as is */
			    }
			}
		    } else {
			assert(HIGH_SURROGATE(ch));
			/* Save the high surrogate */
			*statePtr = (Tcl_EncodingState) (ptrdiff_t) ch;
			if (savedSurrogate) {
			    assert(HIGH_SURROGATE(savedSurrogate));
			    if (PROFILE_STRICT(profile)) {
				result = (flags & ENCODING_INPUT)
				    ? TCL_CONVERT_SYNTAX : TCL_CONVERT_UNKNOWN;
				src = saveSrc;
				break;
			    } else if (PROFILE_REPLACE(profile)) {
				ch = UNICODE_REPLACE_CHAR;
			    } else {
				/* Output the isolated high surrogate */
				ch = savedSurrogate;
			    }
			} else {
			    /* High surrogate saved in *statePtr. Do not output anything just yet. */
			    --numChars; /* Cancel the increment at end of loop */
			    continue;
			}
		    }
		}
	    } else {
		/* Normal character */
		CHECK_ISOLATEDSURROGATE;
	    }

	    dst += Tcl_UniCharToUtf(ch, dst);
	}
    }

    /* Check if an high surrogate left over */
    if (*statePtr) {
	assert(!(flags & ENCODING_UTF)); /* CESU-8, Not UTF-8 */
	if (!(flags & TCL_ENCODING_END)) {
	    /* More data coming */
	} else {
	    /* No more data coming */
	    if (PROFILE_STRICT(profile)) {
		result = (flags & ENCODING_INPUT)
		    ? TCL_CONVERT_SYNTAX : TCL_CONVERT_UNKNOWN;
	    } else {
		if (PROFILE_REPLACE(profile)) {
		    ch = UNICODE_REPLACE_CHAR;
		} else {
		    ch = (Tcl_UniChar) (ptrdiff_t) *statePtr;
		}
		if (dst < dstEnd) {
		    dst += Tcl_UniCharToUtf(ch, dst);
		    ++numChars;
		} else {
		    /* No room in destination */
		    result = TCL_CONVERT_NOSPACE;
		}
	    }
	}
    }

    *srcReadPtr = src - srcStart;
    *dstWrotePtr = dst - dstStart;
    *dstCharsPtr = numChars;
    return result;
}

```

tcl9.1 9.1.0, revision `retained released source bytes`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclEncoding.c`, function `UtfToUtfProc`, lines 2964–3152. Line unit: physical LF only. Original byte range [91365, 96533). Full-source SHA-256 `a8e40415c4e984eb2d8b888f79a3fc09fe1074233f8ee3d6b6bdb0b270d8b770`; snippet SHA-256 `cc111a825092173de7b0cd8fc21474445b32e15dfe1c680899edc905e39c391b`; retained evidence `source-window-31`.

```text
static int
UtfToUtfProc(
    void *clientData,
    const char *src,
    int srcLen,
    int flags,
    Tcl_EncodingState *statePtr,
    char *dst,
    int dstLen,
    int *srcReadPtr,
    int *dstWrotePtr,
    int *dstCharsPtr)
{
    const char *srcStart, *srcEnd, *srcClose;
    const char *dstStart, *dstEnd;
    int result, numChars = 0, charLimit = INT_MAX;
    int ch;
    int profile;

    *statePtr = 0;
    result = TCL_OK;

    srcStart = src;
    srcEnd = src + srcLen;
    srcClose = srcEnd;
    if ((flags & TCL_ENCODING_END) == 0) {
	srcClose -= TCL_UTF_MAX;
    }
    if (flags & TCL_ENCODING_CHAR_LIMIT) {
	charLimit = *dstCharsPtr;
    }

    dstStart = dst;
    flags |= PTR2INT(clientData);

    dstEnd = dst + dstLen - TCL_UTF_MAX;

    /* Checks if a source byte can be copied directly to destination */
#define BYTE_COPYABLE(byte_) \
    (UCHAR(byte_) < 0x80 && ((UCHAR(byte_) != 0)))

    profile = ENCODING_PROFILE_GET(flags);

    for (numChars = 0; src < srcEnd && numChars < charLimit; numChars++) {
	if ((src > srcClose) && (!Tcl_UtfCharComplete(src, srcEnd - src))) {
	    result = TCL_CONVERT_MULTIBYTE;
	    break;
	}
	if (dst > dstEnd) {
	    result = TCL_CONVERT_NOSPACE;
	    break;
	}

	if (BYTE_COPYABLE(*src)) {
	    /*
	     * Common case fast path for non-nul ASCII bytes.
	     *
	     * Because we resolved any pending surrogate above (before the
	     * loop), *statePtr is guaranteed to be 0 here, so no
	     * CHECK_ISOLATEDSURROGATE() call is needed.
	     */
	    const char *p = src;
	    ptrdiff_t copyCount = srcEnd - src;
	    if (charLimit != INT_MAX) {
		if (copyCount > (charLimit - numChars)) {
		    copyCount = charLimit - numChars;
		}
	    }
	    if (copyCount > (dstEnd - dst + 1)) {
		copyCount = dstEnd - dst + 1;
	    }
	    const char *srcStop = src + copyCount;
	    while (p < srcStop && BYTE_COPYABLE(*p)) {
		*dst++ = *p++;
	    }
	    numChars += (int)(p - src) - 1;
	    src = p;
	} else if ((UCHAR(*src) == 0xC0) && (src + 1 < srcEnd) &&
		(UCHAR(src[1]) == 0x80) &&
		(!(flags & ENCODING_INPUT) || !PROFILE_TCL8(profile))) {
	    /* Special sequence \xC0\x80 */
	    if (!PROFILE_TCL8(profile) && (flags & ENCODING_INPUT)) {
		if (PROFILE_REPLACE(profile)) {
		    dst += Tcl_UniCharToUtf(UNICODE_REPLACE_CHAR, dst);
		    src += 2;
		} else {
		    /* PROFILE_STRICT */
		    result = TCL_CONVERT_SYNTAX;
		    break;
		}
	    } else {
		*dst++ = 0;
		src += 2;
	    }
	} else if (!Tcl_UtfCharComplete(src, srcEnd - src)) {
	    /*
	     * Incomplete byte sequence (truncated UTF-8, not just end of
	     * source buffer — that is caught by the srcClose check above).
	     */
	    if (flags & ENCODING_INPUT) {
		if (PROFILE_STRICT(profile)) {
		    result = (flags & TCL_ENCODING_CHAR_LIMIT)
			    ? TCL_CONVERT_MULTIBYTE
			    : TCL_CONVERT_SYNTAX;
		    break;
		}
	    }
	    if (PROFILE_REPLACE(profile)) {
		ch = UNICODE_REPLACE_CHAR;
		++src;
	    } else {
		/* TCL_ENCODING_PROFILE_TCL8 */
		char chbuf[2];
		chbuf[0] = UCHAR(*src++);
		chbuf[1] = 0;
		TclUtfToUniChar(chbuf, &ch);
	    }
	    dst += Tcl_UniCharToUtf(ch, dst);
	} else {
	    /* Have a complete character */
	    size_t len = TclUtfToUniChar(src, &ch);

	    /*
	     * For invalid inputs, Tcl_UtfToUniChar will return the byte
	     * itself and len == 1. We know this is invalid because valid
	     * single byte ASCII is already handled above except for 0.
	     * For the output case, Tcl should not have invalid byte sequences
	     * internally but if it does, garbage in, garbage out. This is
	     * historical behavior that should be changed imho. See
	     * ticket [b69e00ecf6]. TODO.
	     */
	    if (flags & ENCODING_INPUT) {
		if ((len < 2) && (ch != 0)) {
		    if (PROFILE_STRICT(profile)) {
			result = TCL_CONVERT_SYNTAX;
			break;
		    } else if (PROFILE_REPLACE(profile)) {
			ch = UNICODE_REPLACE_CHAR;
		    }
		}
	    }

	    const char *saveSrc = src;
	    src += len;

	    if (SURROGATE(ch)) {
		if (PROFILE_STRICT(profile)) {
		    result = (flags & ENCODING_INPUT)
			    ? TCL_CONVERT_SYNTAX : TCL_CONVERT_UNKNOWN;
		    src = saveSrc;
		    break;
		} else if (PROFILE_REPLACE(profile)) {
		    ch = UNICODE_REPLACE_CHAR;
		}
		/* PROFILE_TCL8: fall through and output as-is */
	    }
	    /* Normal character (or surrogate resolved to replacement/as-is) */
	    assert(ch >= 0 && ch <= 0x10FFFF);
	    if (ch == 0) {
		if (flags & ENCODING_INPUT) {
		    *dst++ = (char)0xC0;
		    *dst++ = (char)0x80;
		} else {
		    *dst++ = 0;
		}
	    } else if ((unsigned)ch < 0x800) {
		assert(ch >= 0x80);
		*dst++ = (char)(0xC0 | (ch >> 6));
		*dst++ = (char)(0x80 | (ch & 0x3F));
	    } else if ((unsigned)ch < 0x10000) {
		*dst++ = (char)(0xE0 | (ch >> 12));
		*dst++ = (char)(0x80 | ((ch >> 6) & 0x3F));
		*dst++ = (char)(0x80 | (ch & 0x3F));
	    } else {
		*dst++ = (char)(0xF0 | (ch >> 18));
		*dst++ = (char)(0x80 | ((ch >> 12) & 0x3F));
		*dst++ = (char)(0x80 | ((ch >> 6)  & 0x3F));
		*dst++ = (char)(0x80 | (ch & 0x3F));
	    }
	}
    }

    *srcReadPtr  = src - srcStart;
    *dstWrotePtr = dst - dstStart;
    *dstCharsPtr = numChars;
    return result;

#undef BYTE_COPYABLE
}

```


## Consumer bindings

- [rust/tcl-cmd-core/src/encoding/original_names.rs](../../../../rust/tcl-cmd-core/src/encoding/original_names.rs), `prepare_source_filename`: Keep selected external backend bytes, original diagnostic extent and counted source label separate under actual C/Jim filename facts.
- [rust/tcl-cmd-core/src/encoding/original_names.rs](../../../../rust/tcl-cmd-core/src/encoding/original_names.rs), `external_iso88591_filename`: Reuse selected original unit decoding for actual ISO external bytes and C8 fallback/C9 strict filesystem error.
- [rust/tcl-cmd-core/src/encoding/original_names.rs](../../../../rust/tcl-cmd-core/src/encoding/original_names.rs), `filename_encoding_error`: Present reached strict external-conversion failure as filesystem EILSEQ, independently of converter capability absence.
- [rust/tcl-platform/src/lib.rs](../../../../rust/tcl-platform/src/lib.rs), `Filesystem::source_filename_bytes`: Supply an independent actual filesystem converter from already selected native units; no raw dialect decoder or source script codec donation.
- [rust/tcl-vm/src/command/native_encoding_source.rs](../../../../rust/tcl-vm/src/command/native_encoding_source.rs), `SourceOps::system_encoding`: Read actual mutable system setting at reached filesystem conversion, with original Name/operand/currency purpose retained separately.
- [runtime/rust/src/interp/native_encoding_source.rs](../../../../runtime/rust/src/interp/native_encoding_source.rs), `SourceOps::system_encoding`: Read current Runtime system encoding at reached source filename conversion under genuine original argv and command currency.
- [rust/tcl-cmd-core/src/encoding/original_names/tests.rs](../../../../rust/tcl-cmd-core/src/encoding/original_names/tests.rs), `encoding::original_names::tests::original_filename_external_zero_has_separate_backend_and_report_extents` (linked): Keep selected C external conversion/C-string or EINVAL backend extent separate from original diagnostic bytes and counted source label; Jim retains raw CString purpose.
- [rust/tcl-cmd-core/src/encoding/original_names/tests.rs](../../../../rust/tcl-cmd-core/src/encoding/original_names/tests.rs), `encoding::original_names::tests::original_filename_iso88591_and_identity_select_actual_external_bytes` (linked): The existing selected native decoder supplies ISO units; C8 fallback and C9 strict EILSEQ remain separate from byte-preserving identity and source labels.
- [rust/tcl-cmd-core/src/encoding/original_names/tests.rs](../../../../rust/tcl-cmd-core/src/encoding/original_names/tests.rs), `encoding::original_names::tests::original_c9_identity_name_uses_actual_catalogue_instead_of_builtin_codec` (linked): C9 named identity uses actual catalogue knowledge: genuine miss yields the selected unknown diagnostic, while supplied installed identity remains possible.
- [rust/tcl-cmd-core/src/encoding/original_names/tests.rs](../../../../rust/tcl-cmd-core/src/encoding/original_names/tests.rs), `encoding::original_names::tests::original_filename_utf8_surrogates_keep_selected_conversion_and_error_owner` (linked): Preserve C84/85 supplied pair bytes, C86 pair combination and C9 strict unpaired/pair EILSEQ through the same external worker, without a missing-converter or Host classification.
- [rust/tcl-vm/src/command/native_encoding_source/tests.rs](../../../../rust/tcl-vm/src/command/native_encoding_source/tests.rs), `command::native_encoding_source::tests::original_filename_utf8_conversion_preserves_counted_header_and_lookup_effects` (linked): Original VM source uses selected external filename bytes and preserves the complete counted operand header while distinguishing C84 prefix effects from later converted-zero EINVAL.
- [rust/tcl-vm/src/command/native_encoding_source/tests.rs](../../../../rust/tcl-vm/src/command/native_encoding_source/tests.rs), `command::native_encoding_source::tests::original_filename_iso88591_host_default_keeps_plain_source_working` (linked): Actual supplied Host ISO-8859-1 default permits ordinary source through the selected filename worker without an additional custom converter query.
- [runtime/rust/src/interp/native_encoding_source/tests.rs](../../../../runtime/rust/src/interp/native_encoding_source/tests.rs), `interp::native_encoding_source::tests::original_filename_external_zero_refuses_before_open_and_keeps_report_bytes` (linked): Runtime original argv uses external zero refusal before backend open, preserves complete original counted header and presents original CString report bytes.
- [runtime/rust/src/interp/native_encoding_source/tests.rs](../../../../runtime/rust/src/interp/native_encoding_source/tests.rs), `interp::native_encoding_source::tests::original_filename_iso88591_selected_system_keeps_plain_source_working` (linked): Runtime consumes the actual selected ISO-8859-1 system setting for an ordinary source filename and preserves exact file/evaluation effects without custom converter queries.

A named test is a coverage binding, not a claim that it executed.

## Replay

Recorded argument vector:

```json
[
  "python3",
  "runtime/rust/tests/data/native_source_original_counted_filename_extent/run.py",
  "--output",
  "<fresh-absolute-directory>"
]
```

The canonical adapter verifies every recorded original input first and redirects only request/output and identical retained version-source constants to a fresh directory. Restore original absolute provider/tool/source paths separately from the digest archive; raw filenames require exact OS byte mappings. Rebuilt/replayed outputs are new evidence, never replacements or executable donors for these original captures. No replay has been launched by publication.
