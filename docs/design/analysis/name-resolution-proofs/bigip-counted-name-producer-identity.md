# naming.bigip.counted-name-producer-identity

Kind: `native-observation`

## Problem statement

NUL-bearing and visually related names may look equal in display while arising from different byte producers. Truncating at NUL or normalizing format/binary products would merge distinct cells and commands.

## Question

Do the exact plain/NUL, format/binary and braced/unbraced name controls select the same variable or command?

## Conclusion

Plain and NUL-bearing scalar/array/index names coexist and mutate independently; traces retain counted arguments. The four format/binary spelling products remain distinct. Unbraced scanning stops on the measured non-ASCII bytes, while braced reads reach full names before a second set lookup can fail on the returned value. Dynamic command creation/rename preserves producer separation.

## Scope

Recorded hexadecimal dynamic sources and all four TMMs; no general Unicode normalization or source-literal acceptance claim. BIG-IP21.1.0.1 build0.0.26 Point Release1 only; appliance implementation source is unavailable.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No independent C Tcl or Jim execution of this appliance question is claimed.

### tcl8.5

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No independent C Tcl or Jim execution of this appliance question is claimed.

### tcl8.6

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No independent C Tcl or Jim execution of this appliance question is claimed.

### tcl9.0

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No independent C Tcl or Jim execution of this appliance question is claimed.

### tcl9.1

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No independent C Tcl or Jim execution of this appliance question is claimed.

### jim

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Jim Tcl.

No independent C Tcl or Jim execution of this appliance question is claimed.

### bigip

Status: `observed`. Version: 21.1.0.1. Build: 0.0.26, Point Release1; no separate hotfix field reported. Channel: Recorded hexadecimal dynamic sources and all four TMMs; no general Unicode normalization or source-literal acceptance claim.. Dialect: F5 iRules and the explicitly recorded hosted contexts.

Plain and NUL-bearing scalar/array/index names coexist and mutate independently; traces retain counted arguments. The four format/binary spelling products remain distinct. Unbraced scanning stops on the measured non-ASCII bytes, while braced reads reach full names before a second set lookup can fail on the returned value. Dynamic command creation/rename preserves producer separation.

The measured report rows and their limits are:


All listed wrappers loaded successfully and returned outer catch status 0 on all four TMMs. Inner catch statuses are stated explicitly.

| Case | Payload SHA-256 | Wrapper SHA-256 | Measured TMM result |
| --- | --- | --- | --- |
| `nul_counted_last` | `cba6f800a9c3e80c17cc08434200fbba2828c413460c91682138879562393df6` | `1d5a364e3c8e48e0d4e3825b3df00298aff1e6b4071a96a569c573a9e3a47b05` | Plain and `A 00 B` scalar names were distinct; `upvar`, read and unset addressed only the counted name |
| `nul_plain_last` | `0d946243ff3331524c0a55a41cc48462fbcdf5b81762025dc75fd499776ce50c` | `9921f01c557a7917e0e93975bc9daf5a84b3e40f6e1c3bd3b47ad80c20d5e80b` | Reversing write order did not alias or overwrite either name |
| `nul_plain_unset_first` | `fce2466c347362948b78530de8786a7d27111c739b300dac27e3240c312ca04a` | `421f200e8ff47ce5480ee96178c5c483d76bb3d7c54857978a21d1ac6f37ac96` | Unsetting the plain name left the counted name and its linked value intact |
| `nul_array_root` | `ac81e5dbd1ba443199d88d87221fe55f5db9f7474e1c42ef1f2ad174af551fdf` | `32931180ef11228bac2b8a8922000a83482c44f9994991cb81b375895061ffa7` | Plain and NUL-bearing array roots were distinct; root unset and `upvar` affected only the selected array |
| `nul_array_index` | `aeaac8f74458ac99a752be410304584703a8a13a071bd4f13fdedd82f4d3b070` | `8b01c6367d9d90bab44d43d1044d1311267d16cb025e7ce09e06ef7267a30b0a` | Index `41` and index `41 00 42` were distinct elements; element unset and `upvar` affected only the counted index |
| `unicode_format_forward` | `3aa03c3acdd0dc093f25f8868e1306b8fd519eb8cda7532a1df3db5362a0a2a3` | `99b72528d0e6eca1727cdff238443f2b67ef5fc3e6d916486580e298414ddc92` | `%c 233` produced raw suffix `e9`; `%c 769` produced `65 01`; names remained distinct |
| `unicode_format_reverse` | `813837e4613d89e36194dbd330d1b81d0b320706e30a73facd39147ec49ec3a0` | `35d00a28bd3e3c3d3d1735d391166cb3d50a5c43e00ecdf3c763621e80c84f1d` | Reverse write/unset order did not change identity |
| `unicode_bytes_forward` | `65f2f9c46f497a38602168109806652a7415040fbc67959c82fa002ead7ae4b7` | `57f57492859bbe116e493555a1b2fc6a677bb31c5db97612bb41d31cc6aa0a75` | Byte suffixes `c3 a9` and `65 cc 81` remained distinct |
| `unicode_bytes_reverse` | `0286dad4d8f16ee0963772c5bac720f2254539fe856c3d84e5d31a63c9df3a84` | `0de395804c938f4f97c1a3d6a1c75810e540303652a7f070652cb67e657a8e3b` | Reverse write/unset order did not change identity |
| `unicode_cross_producer` | `091fcff7300e9c3a1695a8dc575d922a25b2dbb2f459df085a0e42162d2e1bea` | `7713e3b1f8e68c19603e386b03a10a8d8e3012a58e1358ddc5c103f9aa1379e1` | All four raw spellings (`e9`, `c3a9`, `6501`, `65cc81`) were separate variable names; mutation and unset never crossed producers |
| `lexical_format_unbraced` | `c55f583a23fd2e0eae66654e8c54d1f0b0b176aecf44867e5b9809bd648379bf` | `8494917f99809892c986f78dd72cf08749c980604d1bb3a54dc8623fbecdb1ec` | Dynamic writes succeeded; evaluated unbraced tokens stopped before `e9` or `01` and failed while reading the shorter lexical name |
| `lexical_format_braced` | `6e43ed8c0e18f80195c9fd962d5e424e42a8ea21670dbfe77b650f03a6b9c803` | `e1ca6c5ccf4e89653457477da4e3f7cbb2d5c821fad28b1274d8faf6f4096787` | The full braced raw name resolved to its value; the surrounding `set ${name}` script then attempted to read a variable named `PRE_VALUE` or `DECOMPOSED_VALUE` and failed |
| `lexical_bytes_unbraced` | `fba2cd37d875209f86cb6f2d83cf315002445076379cfb34928d92fcc3184e35` | `9a4ad39c0a030674c09db5654412907b04d7578bbecfc9fb9c39e819e8881adc` | Unbraced token scanning stopped before `c3` or `cc`, producing a shorter-name read failure |
| `lexical_bytes_braced` | `611248007d36153d4c6e6559766bcdb9dae130971ee5f2a1e3b8d7089b775643` | `c6f12dede3aedd37b1c031c1c3ec50882948ee34205cb62f6ee8157a4be1919d` | The full `c3a9` and `65cc81` braced names resolved; the second `set` lookup failed on the returned value name |
| `commands_format` | `0486fe6b1bb3c748a9f23ba1f4e9d302590794d3a5cda179ea1ec184a3940b69` | `ef598faed0951bd6fe81f794b254481a84e9232d5a5f1b3df280d8fd3364ab61` | Dynamic namespace/proc creation, lookup, call and rename succeeded for `e9` and `6501` names; the old command disappeared after rename |
| `commands_bytes` | `4007963e00029f6263be93956b8f7baabf506039a3eec0072573e50c42436804` | `38e64e308931db4f6bff577a4df5cb3de2e448fdc1d50392e882c8d026d686f5` | The same operations succeeded independently for `c3a9` and `65cc81` names |
| `commands_cross_producer` | `99712a42104c7411ff853e405b1d057cacba271bd50e3ea571fe649834d664ed` | `beccb39495c06705244537d4f1f963967b683ea92f1e1a116db6e77e51bda51e` | Format- and byte-produced namespaces/procs coexisted and returned different values; renaming a byte-produced proc did not affect its format-produced counterpart |
| `expressions` | `041daa50f939c68691a98d5a138d54f7256edf8ded0765d151e8077108af58bc` | `df4e3532d0f52c3374e2ed99c7c96740ecc3e59c589037d352dcde9ca979b820` | Exact nine-row result is reported below |

### Embedded NUL and trace arguments

The NUL-bearing scalar had length 29 versus 27 for its plain control. The array-root variable references had lengths 38 versus 36, and the NUL-bearing index had length 3 versus 1. In every case the names could coexist, be read independently, be targeted by `upvar`, and be unset independently. This is a counterexample to C-string truncation or name canonicalisation at the embedded NUL.

Variable traces accepted dynamically and fired on every actual TMM. For the index case, callbacks received the ASCII root unchanged and index hex `41` or `410042`, including the NUL. For the root case, callbacks received the full root hex with or without `0042` and index `6b`; the `upvar` write callback reported root name hex `616c696173` (`alias`) while mutating the linked counted array. The raw log contains 3,200 `R2286TRACE` rows and preserves the operation letters and TMM identities in [ltm-raw.log](../../../../scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261006m/appliance/raw/ltm-raw.log).

### Unicode producer identity and lexical resolution

The TMM runtime is byte-oriented for these controls. `format %c 233` returned one byte `e9`, not UTF-8 `c3a9`; `format %c 769` returned byte `01`, so the constructed suffix was `6501`, not UTF-8 combining acute `65cc81`. `binary format H*` preserved the supplied UTF-8 bytes. All four byte sequences remained distinct as variable, namespace and procedure names. No NFC/NFD equivalence was observed. The `encoding` command was unavailable in TMM and returned `invalid command name "encoding"`.

Unbraced `$name` parsing did not consume the non-ASCII/control bytes into the lexical variable token. Braced `${name}` parsing did consume the entire raw name and reached its stored value. The subsequent error naming `PRE_VALUE` or `DECOMPOSED_VALUE` is therefore positive evidence that the full braced name resolved before `set` performed its second variable-name lookup; it is not evidence that the braced raw name was absent.

Dynamic `namespace`, `proc`, `namespace which -command`, invocation, `rename` and exact namespace cleanup all succeeded for both producer families. Cross-producer values prove that similarly rendered names were not aliases.


## Exact evidence

- `report-section` (observation): [scripts/dev/bigip-probes/resolution-2286/FOLLOWUP_APPLIANCE_RESULTS.md](../../../../scripts/dev/bigip-probes/resolution-2286/FOLLOWUP_APPLIANCE_RESULTS.md). SHA-256 `1f1324bcead38249ef875389a21ca63319c3481f529c857cc31c3fd2a69c0f21`. Lines 61–99. Exact build-specific measured report section; all rows and limits remain in the retained report.
- `provider-version` (provider): [scripts/dev/bigip-probes/resolution-2286/FOLLOWUP_APPLIANCE_RESULTS.md](../../../../scripts/dev/bigip-probes/resolution-2286/FOLLOWUP_APPLIANCE_RESULTS.md). SHA-256 `1f1324bcead38249ef875389a21ca63319c3481f529c857cc31c3fd2a69c0f21`. Lines 9–9. Actual appliance version attribution for this report; no separately named hotfix inferred.
- `raw-0` (observation): [scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261006m/appliance/raw/ltm-raw.log](../../../../scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261006m/appliance/raw/ltm-raw.log). SHA-256 `44bbb0669b671ae651de3ca1f191f2788a18805ad5526ce094f7816d43b54ca4`. Exact source or raw evidence explicitly linked by the measured section; its own context/channel remains authoritative.
- `aggregate-3` (observation): [scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261006m/decoded-results.json](../../../../scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261006m/decoded-results.json). SHA-256 `4dc32dddeb5a60b758a03b04d0a9271e83bdf37c92662d9c7e7b4e519f1d89a9`. Inspected exact decoded aggregate, including reached, failed, unreached and cleanup fields; associated report questions preserve the source/context variants.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

No implementation binding is claimed by this observation record.

A named test is a coverage binding, not a claim that it executed.

## Replay

The pinned report contains the exact source-generation, load and traffic protocol. This record claims no new appliance execution; rerun requires the recorded isolated appliance context, exact bytes and independently captured load/event outcomes. Closed implementation source and unmeasured releases are unavailable.
