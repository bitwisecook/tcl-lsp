# naming.list-storage.original-prefix-and-span-geometry

Kind: `native-observation`

## Problem statement

Range length alone does not choose a native span: an unshared prefix trim can run first, and unused allocated capacity changes the interior threshold. Reusing a header does not guarantee unchanged owning slots.

## Question

Which original store offsets, used/allocated extents and spans follow unshared prefix/interior C9 ranges at the measured allocation thresholds?

## Conclusion

Every selected unshared range reuses the original header/store. Prefix trims remain span-free with firstUsed0. Tight interior ranges at used202/203/300/301 selecting at least101 eligible members can retain a span; a100-member interior remains span-free. With allocated capacity three times used, the same sampled interior ranges are span-free and retire to their selected numUsed. Exact cases retain full firstUsed/numUsed/capacity/span coordinates; these finite outcomes are not a complete arbitrary-range allocator proof.

## Scope

Private TclListObjRange on original native lists for C9.0.4 and9.1.0 only. Inputs span used100/101/202/203/300/301, allocated used or3×used, real sharing modes0/1/2 and four ranges,144 cases per release. S rows sample original header refs/resident, store firstUsed/numUsed/capacity/refs, span present/start/length/refs and same original store. R compares actual returned header. Public result publication precedes result snapshots, and no member observer pins exist. This is not a compiled LIST_RANGE_IMM/opcode probe; neither C8 nor Jim was attempted.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, jim, bigip

Status: `not-tested`.

No original observation for this exact question is attached for this provider.

### tcl9.0, tcl9.1

Status: `observed`. Channel: Original native List allocation and private TclListObjRange; public Tcl_SetObjResult publication. Version: tcl9.0 9.0.4; tcl9.1 9.1.0.

Exact selected C/R/S rows. C fields are id,used,allocated,sharing,first,last. R records header==original. S records id/window,header refs,resident,firstUsed,numUsed,capacity,store refs,span present/start/length/refs,store==original.

```text
C|2|100|100|0|25|74
S|2|before|1|0|0|100|100|1|0|-1|-1|0|1
R|2|1
S|2|after-original|2|0|0|50|100|1|0|-1|-1|0|1
S|2|result|2|0|0|50|100|1|0|-1|-1|0|1
C|3|100|100|0|33|99
S|3|before|1|0|0|100|100|1|0|-1|-1|0|1
R|3|1
S|3|after-original|2|0|0|67|100|1|0|-1|-1|0|1
S|3|result|2|0|0|67|100|1|0|-1|-1|0|1
C|14|100|300|0|25|74
S|14|before|1|0|0|100|300|1|0|-1|-1|0|1
R|14|1
S|14|after-original|2|0|0|50|300|1|0|-1|-1|0|1
S|14|result|2|0|0|50|300|1|0|-1|-1|0|1
C|15|100|300|0|33|99
S|15|before|1|0|0|100|300|1|0|-1|-1|0|1
R|15|1
S|15|after-original|2|0|0|67|300|1|0|-1|-1|0|1
S|15|result|2|0|0|67|300|1|0|-1|-1|0|1
C|26|101|101|0|25|74
S|26|before|1|0|0|101|101|1|0|-1|-1|0|1
R|26|1
S|26|after-original|2|0|0|50|101|1|0|-1|-1|0|1
S|26|result|2|0|0|50|101|1|0|-1|-1|0|1
C|27|101|101|0|33|100
S|27|before|1|0|0|101|101|1|0|-1|-1|0|1
R|27|1
S|27|after-original|2|0|0|68|101|1|0|-1|-1|0|1
S|27|result|2|0|0|68|101|1|0|-1|-1|0|1
C|38|101|303|0|25|74
S|38|before|1|0|0|101|303|1|0|-1|-1|0|1
R|38|1
S|38|after-original|2|0|0|50|303|1|0|-1|-1|0|1
S|38|result|2|0|0|50|303|1|0|-1|-1|0|1
C|39|101|303|0|33|100
S|39|before|1|0|0|101|303|1|0|-1|-1|0|1
R|39|1
S|39|after-original|2|0|0|68|303|1|0|-1|-1|0|1
S|39|result|2|0|0|68|303|1|0|-1|-1|0|1
C|49|202|202|0|0|100
S|49|before|1|0|0|202|202|1|0|-1|-1|0|1
R|49|1
S|49|after-original|2|0|0|101|202|1|0|-1|-1|0|1
S|49|result|2|0|0|101|202|1|0|-1|-1|0|1
C|50|202|202|0|50|150
S|50|before|1|0|0|202|202|1|0|-1|-1|0|1
R|50|1
S|50|after-original|2|0|50|101|202|1|1|50|101|1|1
S|50|result|2|0|50|101|202|1|1|50|101|1|1
C|51|202|202|0|67|166
S|51|before|1|0|0|202|202|1|0|-1|-1|0|1
R|51|1
S|51|after-original|2|0|0|100|202|1|0|-1|-1|0|1
S|51|result|2|0|0|100|202|1|0|-1|-1|0|1
C|61|202|606|0|0|100
S|61|before|1|0|0|202|606|1|0|-1|-1|0|1
R|61|1
S|61|after-original|2|0|0|101|606|1|0|-1|-1|0|1
S|61|result|2|0|0|101|606|1|0|-1|-1|0|1
C|62|202|606|0|50|150
S|62|before|1|0|0|202|606|1|0|-1|-1|0|1
R|62|1
S|62|after-original|2|0|0|101|606|1|0|-1|-1|0|1
S|62|result|2|0|0|101|606|1|0|-1|-1|0|1
C|63|202|606|0|67|166
S|63|before|1|0|0|202|606|1|0|-1|-1|0|1
R|63|1
S|63|after-original|2|0|0|100|606|1|0|-1|-1|0|1
S|63|result|2|0|0|100|606|1|0|-1|-1|0|1
C|73|203|203|0|0|100
S|73|before|1|0|0|203|203|1|0|-1|-1|0|1
R|73|1
S|73|after-original|2|0|0|101|203|1|0|-1|-1|0|1
S|73|result|2|0|0|101|203|1|0|-1|-1|0|1
C|74|203|203|0|50|151
S|74|before|1|0|0|203|203|1|0|-1|-1|0|1
R|74|1
S|74|after-original|2|0|50|102|203|1|1|50|102|1|1
S|74|result|2|0|50|102|203|1|1|50|102|1|1
C|75|203|203|0|67|166
S|75|before|1|0|0|203|203|1|0|-1|-1|0|1
R|75|1
S|75|after-original|2|0|0|100|203|1|0|-1|-1|0|1
S|75|result|2|0|0|100|203|1|0|-1|-1|0|1
C|85|203|609|0|0|100
S|85|before|1|0|0|203|609|1|0|-1|-1|0|1
R|85|1
S|85|after-original|2|0|0|101|609|1|0|-1|-1|0|1
S|85|result|2|0|0|101|609|1|0|-1|-1|0|1
C|86|203|609|0|50|151
S|86|before|1|0|0|203|609|1|0|-1|-1|0|1
R|86|1
S|86|after-original|2|0|0|102|609|1|0|-1|-1|0|1
S|86|result|2|0|0|102|609|1|0|-1|-1|0|1
C|87|203|609|0|67|166
S|87|before|1|0|0|203|609|1|0|-1|-1|0|1
R|87|1
S|87|after-original|2|0|0|100|609|1|0|-1|-1|0|1
S|87|result|2|0|0|100|609|1|0|-1|-1|0|1
C|97|300|300|0|0|100
S|97|before|1|0|0|300|300|1|0|-1|-1|0|1
R|97|1
S|97|after-original|2|0|0|101|300|1|0|-1|-1|0|1
S|97|result|2|0|0|101|300|1|0|-1|-1|0|1
C|98|300|300|0|75|224
S|98|before|1|0|0|300|300|1|0|-1|-1|0|1
R|98|1
S|98|after-original|2|0|75|150|300|1|1|75|150|1|1
S|98|result|2|0|75|150|300|1|1|75|150|1|1
C|99|300|300|0|100|199
S|99|before|1|0|0|300|300|1|0|-1|-1|0|1
R|99|1
S|99|after-original|2|0|0|100|300|1|0|-1|-1|0|1
S|99|result|2|0|0|100|300|1|0|-1|-1|0|1
C|109|300|900|0|0|100
S|109|before|1|0|0|300|900|1|0|-1|-1|0|1
R|109|1
S|109|after-original|2|0|0|101|900|1|0|-1|-1|0|1
S|109|result|2|0|0|101|900|1|0|-1|-1|0|1
C|110|300|900|0|75|224
S|110|before|1|0|0|300|900|1|0|-1|-1|0|1
R|110|1
S|110|after-original|2|0|0|150|900|1|0|-1|-1|0|1
S|110|result|2|0|0|150|900|1|0|-1|-1|0|1
C|111|300|900|0|100|199
S|111|before|1|0|0|300|900|1|0|-1|-1|0|1
R|111|1
S|111|after-original|2|0|0|100|900|1|0|-1|-1|0|1
S|111|result|2|0|0|100|900|1|0|-1|-1|0|1
C|121|301|301|0|0|100
S|121|before|1|0|0|301|301|1|0|-1|-1|0|1
R|121|1
S|121|after-original|2|0|0|101|301|1|0|-1|-1|0|1
S|121|result|2|0|0|101|301|1|0|-1|-1|0|1
C|122|301|301|0|75|224
S|122|before|1|0|0|301|301|1|0|-1|-1|0|1
R|122|1
S|122|after-original|2|0|75|150|301|1|1|75|150|1|1
S|122|result|2|0|75|150|301|1|1|75|150|1|1
C|123|301|301|0|100|199
S|123|before|1|0|0|301|301|1|0|-1|-1|0|1
R|123|1
S|123|after-original|2|0|0|100|301|1|0|-1|-1|0|1
S|123|result|2|0|0|100|301|1|0|-1|-1|0|1
C|133|301|903|0|0|100
S|133|before|1|0|0|301|903|1|0|-1|-1|0|1
R|133|1
S|133|after-original|2|0|0|101|903|1|0|-1|-1|0|1
S|133|result|2|0|0|101|903|1|0|-1|-1|0|1
C|134|301|903|0|75|224
S|134|before|1|0|0|301|903|1|0|-1|-1|0|1
R|134|1
S|134|after-original|2|0|0|150|903|1|0|-1|-1|0|1
S|134|result|2|0|0|150|903|1|0|-1|-1|0|1
C|135|301|903|0|100|199
S|135|before|1|0|0|301|903|1|0|-1|-1|0|1
R|135|1
S|135|after-original|2|0|0|100|903|1|0|-1|-1|0|1
S|135|result|2|0|0|100|903|1|0|-1|-1|0|1
```

## Consumer bindings

- [rust/tcl-cmd-core/src/native_list_storage.rs](../../../../rust/tcl-cmd-core/src/native_list_storage.rs), `range_storage_action`: Selects finite original store geometry through independent real header/store sharing and used/allocated extents.
- [rust/tcl-cmd-core/src/native_list_storage.rs](../../../../rust/tcl-cmd-core/src/native_list_storage.rs), `native_list_storage::tests::range_storage_geometry_matches_288_actual_c9_owner_windows` (linked): Compares selected result backing/offset/capacity/span fields within288 original range cases. Header/refcount/whole-publication fields are attached native observations, not asserted by this pure geometry test.

## Evidence

- [rust/tcl-cmd-core/tests/data/native_list_storage/probe.c](../../../../rust/tcl-cmd-core/tests/data/native_list_storage/probe.c)
- [rust/tcl-cmd-core/tests/data/native_list_storage/manifest.json](../../../../rust/tcl-cmd-core/tests/data/native_list_storage/manifest.json)
- [rust/tcl-cmd-core/tests/data/native_list_storage/9.0.4.txt](../../../../rust/tcl-cmd-core/tests/data/native_list_storage/9.0.4.txt)
- [rust/tcl-cmd-core/tests/data/native_list_storage/9.1.0.txt](../../../../rust/tcl-cmd-core/tests/data/native_list_storage/9.1.0.txt)
