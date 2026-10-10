# naming.list-storage.original-shared-header-versus-store

Kind: `native-observation`

## Problem statement

Sharing a header and sharing backing through a separate duplicate are different native owner arrangements. One forces a new header while the other can reuse it; either can still allocate a fresh store for a small window.

## Question

Do partial C9 ranges distinguish a second original-header reference from a separately duplicated header/backing owner?

## Conclusion

Every measured partial range with a second original-header reference returns a new result header; with a separate duplicate header it reuses the original header. In both arrangements, the exact small/ineligible windows allocate fresh backing while the eligible span windows share the original backing and retain the printed span geometry. Original duplicate state is not separately printed, and no arbitrary child-reference or compiler opcode result is claimed.

## Scope

Private TclListObjRange on original native lists for C9.0.4 and9.1.0 only. Inputs span used100/101/202/203/300/301, allocated used or3×used, real sharing modes0/1/2 and four ranges,144 cases per release. S rows sample original header refs/resident, store firstUsed/numUsed/capacity/refs, span present/start/length/refs and same original store. R compares actual returned header. Public result publication precedes result snapshots, and no member observer pins exist. This is not a compiled LIST_RANGE_IMM/opcode probe; neither C8 nor Jim was attempted.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: C Tcl.

No original observation for this exact question is attached for this provider.

### tcl8.5

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: C Tcl.

No original observation for this exact question is attached for this provider.

### tcl8.6

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: C Tcl.

No original observation for this exact question is attached for this provider.

### tcl9.0

Status: `observed`. Version: 9.0.4; exact launched patchlevel not queried by this probe. Build: native_header_sha256=f000e14fdc4e00da1c5ab98a8087c41be3cbf26564a274152236e0531bb55053; native_implementation_sha256=15d49e9df1a0e799eb199ab5ceb58e77e0b3587d7200156ddf82344102069d95; library_sha256=2ba08ecf7197e16d99e303c0d95dde29c59f391a22fd02378edc2697567e0e04; binary_sha256=17d28fe5d575fbd266dbcbc5c02773b3401fdc71fd35c558e3ff86484cf96b87; compile_status=0; run_status=0. Compiler/version/configuration fields absent from this original receipt remain unknown.. Channel: Original native List allocation and private TclListObjRange; public Tcl_SetObjResult publication.. Dialect: C Tcl.

Exact selected C/R/S rows. C fields are id,used,allocated,sharing,first,last. R records header==original. S records id/window,header refs,resident,firstUsed,numUsed,capacity,store refs,span present/start/length/refs,store==original.

```text
C|6|100|100|1|25|74
S|6|before|2|0|0|100|100|1|0|-1|-1|0|1
R|6|0
S|6|after-original|2|0|0|100|100|1|0|-1|-1|0|1
S|6|result|1|0|0|50|50|1|0|-1|-1|0|0
C|7|100|100|1|33|99
S|7|before|2|0|0|100|100|1|0|-1|-1|0|1
R|7|0
S|7|after-original|2|0|0|100|100|1|0|-1|-1|0|1
S|7|result|1|0|0|67|67|1|0|-1|-1|0|0
C|10|100|100|2|25|74
S|10|before|1|0|0|100|100|2|0|-1|-1|0|1
R|10|1
S|10|after-original|2|0|0|50|50|1|0|-1|-1|0|0
S|10|result|2|0|0|50|50|1|0|-1|-1|0|0
C|11|100|100|2|33|99
S|11|before|1|0|0|100|100|2|0|-1|-1|0|1
R|11|1
S|11|after-original|2|0|0|67|67|1|0|-1|-1|0|0
S|11|result|2|0|0|67|67|1|0|-1|-1|0|0
C|18|100|300|1|25|74
S|18|before|2|0|0|100|300|1|0|-1|-1|0|1
R|18|0
S|18|after-original|2|0|0|100|300|1|0|-1|-1|0|1
S|18|result|1|0|0|50|50|1|0|-1|-1|0|0
C|19|100|300|1|33|99
S|19|before|2|0|0|100|300|1|0|-1|-1|0|1
R|19|0
S|19|after-original|2|0|0|100|300|1|0|-1|-1|0|1
S|19|result|1|0|0|67|67|1|0|-1|-1|0|0
C|22|100|300|2|25|74
S|22|before|1|0|0|100|300|2|0|-1|-1|0|1
R|22|1
S|22|after-original|2|0|0|50|50|1|0|-1|-1|0|0
S|22|result|2|0|0|50|50|1|0|-1|-1|0|0
C|23|100|300|2|33|99
S|23|before|1|0|0|100|300|2|0|-1|-1|0|1
R|23|1
S|23|after-original|2|0|0|67|67|1|0|-1|-1|0|0
S|23|result|2|0|0|67|67|1|0|-1|-1|0|0
C|30|101|101|1|25|74
S|30|before|2|0|0|101|101|1|0|-1|-1|0|1
R|30|0
S|30|after-original|2|0|0|101|101|1|0|-1|-1|0|1
S|30|result|1|0|0|50|50|1|0|-1|-1|0|0
C|31|101|101|1|33|100
S|31|before|2|0|0|101|101|1|0|-1|-1|0|1
R|31|0
S|31|after-original|2|0|0|101|101|1|0|-1|-1|0|1
S|31|result|1|0|0|68|68|1|0|-1|-1|0|0
C|34|101|101|2|25|74
S|34|before|1|0|0|101|101|2|0|-1|-1|0|1
R|34|1
S|34|after-original|2|0|0|50|50|1|0|-1|-1|0|0
S|34|result|2|0|0|50|50|1|0|-1|-1|0|0
C|35|101|101|2|33|100
S|35|before|1|0|0|101|101|2|0|-1|-1|0|1
R|35|1
S|35|after-original|2|0|0|68|68|1|0|-1|-1|0|0
S|35|result|2|0|0|68|68|1|0|-1|-1|0|0
C|42|101|303|1|25|74
S|42|before|2|0|0|101|303|1|0|-1|-1|0|1
R|42|0
S|42|after-original|2|0|0|101|303|1|0|-1|-1|0|1
S|42|result|1|0|0|50|50|1|0|-1|-1|0|0
C|43|101|303|1|33|100
S|43|before|2|0|0|101|303|1|0|-1|-1|0|1
R|43|0
S|43|after-original|2|0|0|101|303|1|0|-1|-1|0|1
S|43|result|1|0|0|68|68|1|0|-1|-1|0|0
C|46|101|303|2|25|74
S|46|before|1|0|0|101|303|2|0|-1|-1|0|1
R|46|1
S|46|after-original|2|0|0|50|50|1|0|-1|-1|0|0
S|46|result|2|0|0|50|50|1|0|-1|-1|0|0
C|47|101|303|2|33|100
S|47|before|1|0|0|101|303|2|0|-1|-1|0|1
R|47|1
S|47|after-original|2|0|0|68|68|1|0|-1|-1|0|0
S|47|result|2|0|0|68|68|1|0|-1|-1|0|0
C|53|202|202|1|0|100
S|53|before|2|0|0|202|202|1|0|-1|-1|0|1
R|53|0
S|53|after-original|2|0|0|202|202|2|0|-1|-1|0|1
S|53|result|1|0|0|202|202|2|1|0|101|1|1
C|54|202|202|1|50|150
S|54|before|2|0|0|202|202|1|0|-1|-1|0|1
R|54|0
S|54|after-original|2|0|0|202|202|2|0|-1|-1|0|1
S|54|result|1|0|0|202|202|2|1|50|101|1|1
C|55|202|202|1|67|166
S|55|before|2|0|0|202|202|1|0|-1|-1|0|1
R|55|0
S|55|after-original|2|0|0|202|202|1|0|-1|-1|0|1
S|55|result|1|0|0|100|100|1|0|-1|-1|0|0
C|57|202|202|2|0|100
S|57|before|1|0|0|202|202|2|0|-1|-1|0|1
R|57|1
S|57|after-original|2|0|0|202|202|2|1|0|101|1|1
S|57|result|2|0|0|202|202|2|1|0|101|1|1
C|58|202|202|2|50|150
S|58|before|1|0|0|202|202|2|0|-1|-1|0|1
R|58|1
S|58|after-original|2|0|0|202|202|2|1|50|101|1|1
S|58|result|2|0|0|202|202|2|1|50|101|1|1
C|59|202|202|2|67|166
S|59|before|1|0|0|202|202|2|0|-1|-1|0|1
R|59|1
S|59|after-original|2|0|0|100|100|1|0|-1|-1|0|0
S|59|result|2|0|0|100|100|1|0|-1|-1|0|0
C|65|202|606|1|0|100
S|65|before|2|0|0|202|606|1|0|-1|-1|0|1
R|65|0
S|65|after-original|2|0|0|202|606|1|0|-1|-1|0|1
S|65|result|1|0|0|101|101|1|0|-1|-1|0|0
C|66|202|606|1|50|150
S|66|before|2|0|0|202|606|1|0|-1|-1|0|1
R|66|0
S|66|after-original|2|0|0|202|606|1|0|-1|-1|0|1
S|66|result|1|0|0|101|101|1|0|-1|-1|0|0
C|67|202|606|1|67|166
S|67|before|2|0|0|202|606|1|0|-1|-1|0|1
R|67|0
S|67|after-original|2|0|0|202|606|1|0|-1|-1|0|1
S|67|result|1|0|0|100|100|1|0|-1|-1|0|0
C|69|202|606|2|0|100
S|69|before|1|0|0|202|606|2|0|-1|-1|0|1
R|69|1
S|69|after-original|2|0|0|101|101|1|0|-1|-1|0|0
S|69|result|2|0|0|101|101|1|0|-1|-1|0|0
C|70|202|606|2|50|150
S|70|before|1|0|0|202|606|2|0|-1|-1|0|1
R|70|1
S|70|after-original|2|0|0|101|101|1|0|-1|-1|0|0
S|70|result|2|0|0|101|101|1|0|-1|-1|0|0
C|71|202|606|2|67|166
S|71|before|1|0|0|202|606|2|0|-1|-1|0|1
R|71|1
S|71|after-original|2|0|0|100|100|1|0|-1|-1|0|0
S|71|result|2|0|0|100|100|1|0|-1|-1|0|0
C|77|203|203|1|0|100
S|77|before|2|0|0|203|203|1|0|-1|-1|0|1
R|77|0
S|77|after-original|2|0|0|203|203|2|0|-1|-1|0|1
S|77|result|1|0|0|203|203|2|1|0|101|1|1
C|78|203|203|1|50|151
S|78|before|2|0|0|203|203|1|0|-1|-1|0|1
R|78|0
S|78|after-original|2|0|0|203|203|2|0|-1|-1|0|1
S|78|result|1|0|0|203|203|2|1|50|102|1|1
C|79|203|203|1|67|166
S|79|before|2|0|0|203|203|1|0|-1|-1|0|1
R|79|0
S|79|after-original|2|0|0|203|203|1|0|-1|-1|0|1
S|79|result|1|0|0|100|100|1|0|-1|-1|0|0
C|81|203|203|2|0|100
S|81|before|1|0|0|203|203|2|0|-1|-1|0|1
R|81|1
S|81|after-original|2|0|0|203|203|2|1|0|101|1|1
S|81|result|2|0|0|203|203|2|1|0|101|1|1
C|82|203|203|2|50|151
S|82|before|1|0|0|203|203|2|0|-1|-1|0|1
R|82|1
S|82|after-original|2|0|0|203|203|2|1|50|102|1|1
S|82|result|2|0|0|203|203|2|1|50|102|1|1
C|83|203|203|2|67|166
S|83|before|1|0|0|203|203|2|0|-1|-1|0|1
R|83|1
S|83|after-original|2|0|0|100|100|1|0|-1|-1|0|0
S|83|result|2|0|0|100|100|1|0|-1|-1|0|0
C|89|203|609|1|0|100
S|89|before|2|0|0|203|609|1|0|-1|-1|0|1
R|89|0
S|89|after-original|2|0|0|203|609|1|0|-1|-1|0|1
S|89|result|1|0|0|101|101|1|0|-1|-1|0|0
C|90|203|609|1|50|151
S|90|before|2|0|0|203|609|1|0|-1|-1|0|1
R|90|0
S|90|after-original|2|0|0|203|609|1|0|-1|-1|0|1
S|90|result|1|0|0|102|102|1|0|-1|-1|0|0
C|91|203|609|1|67|166
S|91|before|2|0|0|203|609|1|0|-1|-1|0|1
R|91|0
S|91|after-original|2|0|0|203|609|1|0|-1|-1|0|1
S|91|result|1|0|0|100|100|1|0|-1|-1|0|0
C|93|203|609|2|0|100
S|93|before|1|0|0|203|609|2|0|-1|-1|0|1
R|93|1
S|93|after-original|2|0|0|101|101|1|0|-1|-1|0|0
S|93|result|2|0|0|101|101|1|0|-1|-1|0|0
C|94|203|609|2|50|151
S|94|before|1|0|0|203|609|2|0|-1|-1|0|1
R|94|1
S|94|after-original|2|0|0|102|102|1|0|-1|-1|0|0
S|94|result|2|0|0|102|102|1|0|-1|-1|0|0
C|95|203|609|2|67|166
S|95|before|1|0|0|203|609|2|0|-1|-1|0|1
R|95|1
S|95|after-original|2|0|0|100|100|1|0|-1|-1|0|0
S|95|result|2|0|0|100|100|1|0|-1|-1|0|0
C|101|300|300|1|0|100
S|101|before|2|0|0|300|300|1|0|-1|-1|0|1
R|101|0
S|101|after-original|2|0|0|300|300|1|0|-1|-1|0|1
S|101|result|1|0|0|101|101|1|0|-1|-1|0|0
C|102|300|300|1|75|224
S|102|before|2|0|0|300|300|1|0|-1|-1|0|1
R|102|0
S|102|after-original|2|0|0|300|300|2|0|-1|-1|0|1
S|102|result|1|0|0|300|300|2|1|75|150|1|1
C|103|300|300|1|100|199
S|103|before|2|0|0|300|300|1|0|-1|-1|0|1
R|103|0
S|103|after-original|2|0|0|300|300|1|0|-1|-1|0|1
S|103|result|1|0|0|100|100|1|0|-1|-1|0|0
C|105|300|300|2|0|100
S|105|before|1|0|0|300|300|2|0|-1|-1|0|1
R|105|1
S|105|after-original|2|0|0|101|101|1|0|-1|-1|0|0
S|105|result|2|0|0|101|101|1|0|-1|-1|0|0
C|106|300|300|2|75|224
S|106|before|1|0|0|300|300|2|0|-1|-1|0|1
R|106|1
S|106|after-original|2|0|0|300|300|2|1|75|150|1|1
S|106|result|2|0|0|300|300|2|1|75|150|1|1
C|107|300|300|2|100|199
S|107|before|1|0|0|300|300|2|0|-1|-1|0|1
R|107|1
S|107|after-original|2|0|0|100|100|1|0|-1|-1|0|0
S|107|result|2|0|0|100|100|1|0|-1|-1|0|0
C|113|300|900|1|0|100
S|113|before|2|0|0|300|900|1|0|-1|-1|0|1
R|113|0
S|113|after-original|2|0|0|300|900|1|0|-1|-1|0|1
S|113|result|1|0|0|101|101|1|0|-1|-1|0|0
C|114|300|900|1|75|224
S|114|before|2|0|0|300|900|1|0|-1|-1|0|1
R|114|0
S|114|after-original|2|0|0|300|900|1|0|-1|-1|0|1
S|114|result|1|0|0|150|150|1|0|-1|-1|0|0
C|115|300|900|1|100|199
S|115|before|2|0|0|300|900|1|0|-1|-1|0|1
R|115|0
S|115|after-original|2|0|0|300|900|1|0|-1|-1|0|1
S|115|result|1|0|0|100|100|1|0|-1|-1|0|0
C|117|300|900|2|0|100
S|117|before|1|0|0|300|900|2|0|-1|-1|0|1
R|117|1
S|117|after-original|2|0|0|101|101|1|0|-1|-1|0|0
S|117|result|2|0|0|101|101|1|0|-1|-1|0|0
C|118|300|900|2|75|224
S|118|before|1|0|0|300|900|2|0|-1|-1|0|1
R|118|1
S|118|after-original|2|0|0|150|150|1|0|-1|-1|0|0
S|118|result|2|0|0|150|150|1|0|-1|-1|0|0
C|119|300|900|2|100|199
S|119|before|1|0|0|300|900|2|0|-1|-1|0|1
R|119|1
S|119|after-original|2|0|0|100|100|1|0|-1|-1|0|0
S|119|result|2|0|0|100|100|1|0|-1|-1|0|0
C|125|301|301|1|0|100
S|125|before|2|0|0|301|301|1|0|-1|-1|0|1
R|125|0
S|125|after-original|2|0|0|301|301|1|0|-1|-1|0|1
S|125|result|1|0|0|101|101|1|0|-1|-1|0|0
C|126|301|301|1|75|224
S|126|before|2|0|0|301|301|1|0|-1|-1|0|1
R|126|0
S|126|after-original|2|0|0|301|301|2|0|-1|-1|0|1
S|126|result|1|0|0|301|301|2|1|75|150|1|1
C|127|301|301|1|100|199
S|127|before|2|0|0|301|301|1|0|-1|-1|0|1
R|127|0
S|127|after-original|2|0|0|301|301|1|0|-1|-1|0|1
S|127|result|1|0|0|100|100|1|0|-1|-1|0|0
C|129|301|301|2|0|100
S|129|before|1|0|0|301|301|2|0|-1|-1|0|1
R|129|1
S|129|after-original|2|0|0|101|101|1|0|-1|-1|0|0
S|129|result|2|0|0|101|101|1|0|-1|-1|0|0
C|130|301|301|2|75|224
S|130|before|1|0|0|301|301|2|0|-1|-1|0|1
R|130|1
S|130|after-original|2|0|0|301|301|2|1|75|150|1|1
S|130|result|2|0|0|301|301|2|1|75|150|1|1
C|131|301|301|2|100|199
S|131|before|1|0|0|301|301|2|0|-1|-1|0|1
R|131|1
S|131|after-original|2|0|0|100|100|1|0|-1|-1|0|0
S|131|result|2|0|0|100|100|1|0|-1|-1|0|0
C|137|301|903|1|0|100
S|137|before|2|0|0|301|903|1|0|-1|-1|0|1
R|137|0
S|137|after-original|2|0|0|301|903|1|0|-1|-1|0|1
S|137|result|1|0|0|101|101|1|0|-1|-1|0|0
C|138|301|903|1|75|224
S|138|before|2|0|0|301|903|1|0|-1|-1|0|1
R|138|0
S|138|after-original|2|0|0|301|903|1|0|-1|-1|0|1
S|138|result|1|0|0|150|150|1|0|-1|-1|0|0
C|139|301|903|1|100|199
S|139|before|2|0|0|301|903|1|0|-1|-1|0|1
R|139|0
S|139|after-original|2|0|0|301|903|1|0|-1|-1|0|1
S|139|result|1|0|0|100|100|1|0|-1|-1|0|0
C|141|301|903|2|0|100
S|141|before|1|0|0|301|903|2|0|-1|-1|0|1
R|141|1
S|141|after-original|2|0|0|101|101|1|0|-1|-1|0|0
S|141|result|2|0|0|101|101|1|0|-1|-1|0|0
C|142|301|903|2|75|224
S|142|before|1|0|0|301|903|2|0|-1|-1|0|1
R|142|1
S|142|after-original|2|0|0|150|150|1|0|-1|-1|0|0
S|142|result|2|0|0|150|150|1|0|-1|-1|0|0
C|143|301|903|2|100|199
S|143|before|1|0|0|301|903|2|0|-1|-1|0|1
R|143|1
S|143|after-original|2|0|0|100|100|1|0|-1|-1|0|0
S|143|result|2|0|0|100|100|1|0|-1|-1|0|0
```

### tcl9.1

Status: `observed`. Version: 9.1.0; exact launched patchlevel not queried by this probe. Build: native_header_sha256=fea9d99020df9aec6efebf6e3f18592ee267a9bf80d3843df0d6439efebe65cc; native_implementation_sha256=6711651457a8f8813f37c71e5e8958aca9d2e83a6fc7c7e281ae3f7ada10e81b; library_sha256=8b2dba836908287f95f26a442bb065601daac300fdeee48b15e64a5c03894e33; binary_sha256=37920f13d6672f2d491bee665d1f420d02b07f581b3296ffa53e2bb533354b0f; compile_status=0; run_status=0. Compiler/version/configuration fields absent from this original receipt remain unknown.. Channel: Original native List allocation and private TclListObjRange; public Tcl_SetObjResult publication.. Dialect: C Tcl.

Exact selected C/R/S rows. C fields are id,used,allocated,sharing,first,last. R records header==original. S records id/window,header refs,resident,firstUsed,numUsed,capacity,store refs,span present/start/length/refs,store==original.

```text
C|6|100|100|1|25|74
S|6|before|2|0|0|100|100|1|0|-1|-1|0|1
R|6|0
S|6|after-original|2|0|0|100|100|1|0|-1|-1|0|1
S|6|result|1|0|0|50|50|1|0|-1|-1|0|0
C|7|100|100|1|33|99
S|7|before|2|0|0|100|100|1|0|-1|-1|0|1
R|7|0
S|7|after-original|2|0|0|100|100|1|0|-1|-1|0|1
S|7|result|1|0|0|67|67|1|0|-1|-1|0|0
C|10|100|100|2|25|74
S|10|before|1|0|0|100|100|2|0|-1|-1|0|1
R|10|1
S|10|after-original|2|0|0|50|50|1|0|-1|-1|0|0
S|10|result|2|0|0|50|50|1|0|-1|-1|0|0
C|11|100|100|2|33|99
S|11|before|1|0|0|100|100|2|0|-1|-1|0|1
R|11|1
S|11|after-original|2|0|0|67|67|1|0|-1|-1|0|0
S|11|result|2|0|0|67|67|1|0|-1|-1|0|0
C|18|100|300|1|25|74
S|18|before|2|0|0|100|300|1|0|-1|-1|0|1
R|18|0
S|18|after-original|2|0|0|100|300|1|0|-1|-1|0|1
S|18|result|1|0|0|50|50|1|0|-1|-1|0|0
C|19|100|300|1|33|99
S|19|before|2|0|0|100|300|1|0|-1|-1|0|1
R|19|0
S|19|after-original|2|0|0|100|300|1|0|-1|-1|0|1
S|19|result|1|0|0|67|67|1|0|-1|-1|0|0
C|22|100|300|2|25|74
S|22|before|1|0|0|100|300|2|0|-1|-1|0|1
R|22|1
S|22|after-original|2|0|0|50|50|1|0|-1|-1|0|0
S|22|result|2|0|0|50|50|1|0|-1|-1|0|0
C|23|100|300|2|33|99
S|23|before|1|0|0|100|300|2|0|-1|-1|0|1
R|23|1
S|23|after-original|2|0|0|67|67|1|0|-1|-1|0|0
S|23|result|2|0|0|67|67|1|0|-1|-1|0|0
C|30|101|101|1|25|74
S|30|before|2|0|0|101|101|1|0|-1|-1|0|1
R|30|0
S|30|after-original|2|0|0|101|101|1|0|-1|-1|0|1
S|30|result|1|0|0|50|50|1|0|-1|-1|0|0
C|31|101|101|1|33|100
S|31|before|2|0|0|101|101|1|0|-1|-1|0|1
R|31|0
S|31|after-original|2|0|0|101|101|1|0|-1|-1|0|1
S|31|result|1|0|0|68|68|1|0|-1|-1|0|0
C|34|101|101|2|25|74
S|34|before|1|0|0|101|101|2|0|-1|-1|0|1
R|34|1
S|34|after-original|2|0|0|50|50|1|0|-1|-1|0|0
S|34|result|2|0|0|50|50|1|0|-1|-1|0|0
C|35|101|101|2|33|100
S|35|before|1|0|0|101|101|2|0|-1|-1|0|1
R|35|1
S|35|after-original|2|0|0|68|68|1|0|-1|-1|0|0
S|35|result|2|0|0|68|68|1|0|-1|-1|0|0
C|42|101|303|1|25|74
S|42|before|2|0|0|101|303|1|0|-1|-1|0|1
R|42|0
S|42|after-original|2|0|0|101|303|1|0|-1|-1|0|1
S|42|result|1|0|0|50|50|1|0|-1|-1|0|0
C|43|101|303|1|33|100
S|43|before|2|0|0|101|303|1|0|-1|-1|0|1
R|43|0
S|43|after-original|2|0|0|101|303|1|0|-1|-1|0|1
S|43|result|1|0|0|68|68|1|0|-1|-1|0|0
C|46|101|303|2|25|74
S|46|before|1|0|0|101|303|2|0|-1|-1|0|1
R|46|1
S|46|after-original|2|0|0|50|50|1|0|-1|-1|0|0
S|46|result|2|0|0|50|50|1|0|-1|-1|0|0
C|47|101|303|2|33|100
S|47|before|1|0|0|101|303|2|0|-1|-1|0|1
R|47|1
S|47|after-original|2|0|0|68|68|1|0|-1|-1|0|0
S|47|result|2|0|0|68|68|1|0|-1|-1|0|0
C|53|202|202|1|0|100
S|53|before|2|0|0|202|202|1|0|-1|-1|0|1
R|53|0
S|53|after-original|2|0|0|202|202|2|0|-1|-1|0|1
S|53|result|1|0|0|202|202|2|1|0|101|1|1
C|54|202|202|1|50|150
S|54|before|2|0|0|202|202|1|0|-1|-1|0|1
R|54|0
S|54|after-original|2|0|0|202|202|2|0|-1|-1|0|1
S|54|result|1|0|0|202|202|2|1|50|101|1|1
C|55|202|202|1|67|166
S|55|before|2|0|0|202|202|1|0|-1|-1|0|1
R|55|0
S|55|after-original|2|0|0|202|202|1|0|-1|-1|0|1
S|55|result|1|0|0|100|100|1|0|-1|-1|0|0
C|57|202|202|2|0|100
S|57|before|1|0|0|202|202|2|0|-1|-1|0|1
R|57|1
S|57|after-original|2|0|0|202|202|2|1|0|101|1|1
S|57|result|2|0|0|202|202|2|1|0|101|1|1
C|58|202|202|2|50|150
S|58|before|1|0|0|202|202|2|0|-1|-1|0|1
R|58|1
S|58|after-original|2|0|0|202|202|2|1|50|101|1|1
S|58|result|2|0|0|202|202|2|1|50|101|1|1
C|59|202|202|2|67|166
S|59|before|1|0|0|202|202|2|0|-1|-1|0|1
R|59|1
S|59|after-original|2|0|0|100|100|1|0|-1|-1|0|0
S|59|result|2|0|0|100|100|1|0|-1|-1|0|0
C|65|202|606|1|0|100
S|65|before|2|0|0|202|606|1|0|-1|-1|0|1
R|65|0
S|65|after-original|2|0|0|202|606|1|0|-1|-1|0|1
S|65|result|1|0|0|101|101|1|0|-1|-1|0|0
C|66|202|606|1|50|150
S|66|before|2|0|0|202|606|1|0|-1|-1|0|1
R|66|0
S|66|after-original|2|0|0|202|606|1|0|-1|-1|0|1
S|66|result|1|0|0|101|101|1|0|-1|-1|0|0
C|67|202|606|1|67|166
S|67|before|2|0|0|202|606|1|0|-1|-1|0|1
R|67|0
S|67|after-original|2|0|0|202|606|1|0|-1|-1|0|1
S|67|result|1|0|0|100|100|1|0|-1|-1|0|0
C|69|202|606|2|0|100
S|69|before|1|0|0|202|606|2|0|-1|-1|0|1
R|69|1
S|69|after-original|2|0|0|101|101|1|0|-1|-1|0|0
S|69|result|2|0|0|101|101|1|0|-1|-1|0|0
C|70|202|606|2|50|150
S|70|before|1|0|0|202|606|2|0|-1|-1|0|1
R|70|1
S|70|after-original|2|0|0|101|101|1|0|-1|-1|0|0
S|70|result|2|0|0|101|101|1|0|-1|-1|0|0
C|71|202|606|2|67|166
S|71|before|1|0|0|202|606|2|0|-1|-1|0|1
R|71|1
S|71|after-original|2|0|0|100|100|1|0|-1|-1|0|0
S|71|result|2|0|0|100|100|1|0|-1|-1|0|0
C|77|203|203|1|0|100
S|77|before|2|0|0|203|203|1|0|-1|-1|0|1
R|77|0
S|77|after-original|2|0|0|203|203|2|0|-1|-1|0|1
S|77|result|1|0|0|203|203|2|1|0|101|1|1
C|78|203|203|1|50|151
S|78|before|2|0|0|203|203|1|0|-1|-1|0|1
R|78|0
S|78|after-original|2|0|0|203|203|2|0|-1|-1|0|1
S|78|result|1|0|0|203|203|2|1|50|102|1|1
C|79|203|203|1|67|166
S|79|before|2|0|0|203|203|1|0|-1|-1|0|1
R|79|0
S|79|after-original|2|0|0|203|203|1|0|-1|-1|0|1
S|79|result|1|0|0|100|100|1|0|-1|-1|0|0
C|81|203|203|2|0|100
S|81|before|1|0|0|203|203|2|0|-1|-1|0|1
R|81|1
S|81|after-original|2|0|0|203|203|2|1|0|101|1|1
S|81|result|2|0|0|203|203|2|1|0|101|1|1
C|82|203|203|2|50|151
S|82|before|1|0|0|203|203|2|0|-1|-1|0|1
R|82|1
S|82|after-original|2|0|0|203|203|2|1|50|102|1|1
S|82|result|2|0|0|203|203|2|1|50|102|1|1
C|83|203|203|2|67|166
S|83|before|1|0|0|203|203|2|0|-1|-1|0|1
R|83|1
S|83|after-original|2|0|0|100|100|1|0|-1|-1|0|0
S|83|result|2|0|0|100|100|1|0|-1|-1|0|0
C|89|203|609|1|0|100
S|89|before|2|0|0|203|609|1|0|-1|-1|0|1
R|89|0
S|89|after-original|2|0|0|203|609|1|0|-1|-1|0|1
S|89|result|1|0|0|101|101|1|0|-1|-1|0|0
C|90|203|609|1|50|151
S|90|before|2|0|0|203|609|1|0|-1|-1|0|1
R|90|0
S|90|after-original|2|0|0|203|609|1|0|-1|-1|0|1
S|90|result|1|0|0|102|102|1|0|-1|-1|0|0
C|91|203|609|1|67|166
S|91|before|2|0|0|203|609|1|0|-1|-1|0|1
R|91|0
S|91|after-original|2|0|0|203|609|1|0|-1|-1|0|1
S|91|result|1|0|0|100|100|1|0|-1|-1|0|0
C|93|203|609|2|0|100
S|93|before|1|0|0|203|609|2|0|-1|-1|0|1
R|93|1
S|93|after-original|2|0|0|101|101|1|0|-1|-1|0|0
S|93|result|2|0|0|101|101|1|0|-1|-1|0|0
C|94|203|609|2|50|151
S|94|before|1|0|0|203|609|2|0|-1|-1|0|1
R|94|1
S|94|after-original|2|0|0|102|102|1|0|-1|-1|0|0
S|94|result|2|0|0|102|102|1|0|-1|-1|0|0
C|95|203|609|2|67|166
S|95|before|1|0|0|203|609|2|0|-1|-1|0|1
R|95|1
S|95|after-original|2|0|0|100|100|1|0|-1|-1|0|0
S|95|result|2|0|0|100|100|1|0|-1|-1|0|0
C|101|300|300|1|0|100
S|101|before|2|0|0|300|300|1|0|-1|-1|0|1
R|101|0
S|101|after-original|2|0|0|300|300|1|0|-1|-1|0|1
S|101|result|1|0|0|101|101|1|0|-1|-1|0|0
C|102|300|300|1|75|224
S|102|before|2|0|0|300|300|1|0|-1|-1|0|1
R|102|0
S|102|after-original|2|0|0|300|300|2|0|-1|-1|0|1
S|102|result|1|0|0|300|300|2|1|75|150|1|1
C|103|300|300|1|100|199
S|103|before|2|0|0|300|300|1|0|-1|-1|0|1
R|103|0
S|103|after-original|2|0|0|300|300|1|0|-1|-1|0|1
S|103|result|1|0|0|100|100|1|0|-1|-1|0|0
C|105|300|300|2|0|100
S|105|before|1|0|0|300|300|2|0|-1|-1|0|1
R|105|1
S|105|after-original|2|0|0|101|101|1|0|-1|-1|0|0
S|105|result|2|0|0|101|101|1|0|-1|-1|0|0
C|106|300|300|2|75|224
S|106|before|1|0|0|300|300|2|0|-1|-1|0|1
R|106|1
S|106|after-original|2|0|0|300|300|2|1|75|150|1|1
S|106|result|2|0|0|300|300|2|1|75|150|1|1
C|107|300|300|2|100|199
S|107|before|1|0|0|300|300|2|0|-1|-1|0|1
R|107|1
S|107|after-original|2|0|0|100|100|1|0|-1|-1|0|0
S|107|result|2|0|0|100|100|1|0|-1|-1|0|0
C|113|300|900|1|0|100
S|113|before|2|0|0|300|900|1|0|-1|-1|0|1
R|113|0
S|113|after-original|2|0|0|300|900|1|0|-1|-1|0|1
S|113|result|1|0|0|101|101|1|0|-1|-1|0|0
C|114|300|900|1|75|224
S|114|before|2|0|0|300|900|1|0|-1|-1|0|1
R|114|0
S|114|after-original|2|0|0|300|900|1|0|-1|-1|0|1
S|114|result|1|0|0|150|150|1|0|-1|-1|0|0
C|115|300|900|1|100|199
S|115|before|2|0|0|300|900|1|0|-1|-1|0|1
R|115|0
S|115|after-original|2|0|0|300|900|1|0|-1|-1|0|1
S|115|result|1|0|0|100|100|1|0|-1|-1|0|0
C|117|300|900|2|0|100
S|117|before|1|0|0|300|900|2|0|-1|-1|0|1
R|117|1
S|117|after-original|2|0|0|101|101|1|0|-1|-1|0|0
S|117|result|2|0|0|101|101|1|0|-1|-1|0|0
C|118|300|900|2|75|224
S|118|before|1|0|0|300|900|2|0|-1|-1|0|1
R|118|1
S|118|after-original|2|0|0|150|150|1|0|-1|-1|0|0
S|118|result|2|0|0|150|150|1|0|-1|-1|0|0
C|119|300|900|2|100|199
S|119|before|1|0|0|300|900|2|0|-1|-1|0|1
R|119|1
S|119|after-original|2|0|0|100|100|1|0|-1|-1|0|0
S|119|result|2|0|0|100|100|1|0|-1|-1|0|0
C|125|301|301|1|0|100
S|125|before|2|0|0|301|301|1|0|-1|-1|0|1
R|125|0
S|125|after-original|2|0|0|301|301|1|0|-1|-1|0|1
S|125|result|1|0|0|101|101|1|0|-1|-1|0|0
C|126|301|301|1|75|224
S|126|before|2|0|0|301|301|1|0|-1|-1|0|1
R|126|0
S|126|after-original|2|0|0|301|301|2|0|-1|-1|0|1
S|126|result|1|0|0|301|301|2|1|75|150|1|1
C|127|301|301|1|100|199
S|127|before|2|0|0|301|301|1|0|-1|-1|0|1
R|127|0
S|127|after-original|2|0|0|301|301|1|0|-1|-1|0|1
S|127|result|1|0|0|100|100|1|0|-1|-1|0|0
C|129|301|301|2|0|100
S|129|before|1|0|0|301|301|2|0|-1|-1|0|1
R|129|1
S|129|after-original|2|0|0|101|101|1|0|-1|-1|0|0
S|129|result|2|0|0|101|101|1|0|-1|-1|0|0
C|130|301|301|2|75|224
S|130|before|1|0|0|301|301|2|0|-1|-1|0|1
R|130|1
S|130|after-original|2|0|0|301|301|2|1|75|150|1|1
S|130|result|2|0|0|301|301|2|1|75|150|1|1
C|131|301|301|2|100|199
S|131|before|1|0|0|301|301|2|0|-1|-1|0|1
R|131|1
S|131|after-original|2|0|0|100|100|1|0|-1|-1|0|0
S|131|result|2|0|0|100|100|1|0|-1|-1|0|0
C|137|301|903|1|0|100
S|137|before|2|0|0|301|903|1|0|-1|-1|0|1
R|137|0
S|137|after-original|2|0|0|301|903|1|0|-1|-1|0|1
S|137|result|1|0|0|101|101|1|0|-1|-1|0|0
C|138|301|903|1|75|224
S|138|before|2|0|0|301|903|1|0|-1|-1|0|1
R|138|0
S|138|after-original|2|0|0|301|903|1|0|-1|-1|0|1
S|138|result|1|0|0|150|150|1|0|-1|-1|0|0
C|139|301|903|1|100|199
S|139|before|2|0|0|301|903|1|0|-1|-1|0|1
R|139|0
S|139|after-original|2|0|0|301|903|1|0|-1|-1|0|1
S|139|result|1|0|0|100|100|1|0|-1|-1|0|0
C|141|301|903|2|0|100
S|141|before|1|0|0|301|903|2|0|-1|-1|0|1
R|141|1
S|141|after-original|2|0|0|101|101|1|0|-1|-1|0|0
S|141|result|2|0|0|101|101|1|0|-1|-1|0|0
C|142|301|903|2|75|224
S|142|before|1|0|0|301|903|2|0|-1|-1|0|1
R|142|1
S|142|after-original|2|0|0|150|150|1|0|-1|-1|0|0
S|142|result|2|0|0|150|150|1|0|-1|-1|0|0
C|143|301|903|2|100|199
S|143|before|1|0|0|301|903|2|0|-1|-1|0|1
R|143|1
S|143|after-original|2|0|0|100|100|1|0|-1|-1|0|0
S|143|result|2|0|0|100|100|1|0|-1|-1|0|0
```

### jim

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Jim Tcl.

No original observation for this exact question is attached for this provider.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: F5 iRules.

No original observation for this exact question is attached for this provider.

## Exact evidence

- `probe` (input): [rust/tcl-cmd-core/tests/data/native_list_storage/probe.c](../../../../rust/tcl-cmd-core/tests/data/native_list_storage/probe.c). SHA-256 `25da341b2f213aa31a35d5999018265d0f3beecb5a20ccd81ea8947e8b732bfa`. Exact native allocation, true owners, private range call and public result publication observer.
- `receipt` (provider): [rust/tcl-cmd-core/tests/data/native_list_storage/manifest.json](../../../../rust/tcl-cmd-core/tests/data/native_list_storage/manifest.json). SHA-256 `9f06be1c13a3cb37d724ef82ad5082f0c4a882e18c93cebb528163e6de409a60`. Original C9 compile/status/source/header/lib/executable/output closure; native source hashes alone are not retained excerpts.
- `rows-tcl9.0` (observation): [rust/tcl-cmd-core/tests/data/native_list_storage/9.0.4.txt](../../../../rust/tcl-cmd-core/tests/data/native_list_storage/9.0.4.txt). SHA-256 `7ee6709535f5e14c81870e3afbb854ea03524ea69dcae9f1bb91e6db37be0121`. Complete original stream. Selected case IDs 6,7,10,11,18,19,22,23,30,31,34,35,42,43,46,47,53,54,55,57,58,59,65,66,67,69,70,71,77,78,79,81,82,83,89,90,91,93,94,95,101,102,103,105,106,107,113,114,115,117,118,119,125,126,127,129,130,131,137,138,139,141,142,143.
- `rows-tcl9.1` (observation): [rust/tcl-cmd-core/tests/data/native_list_storage/9.1.0.txt](../../../../rust/tcl-cmd-core/tests/data/native_list_storage/9.1.0.txt). SHA-256 `7ee6709535f5e14c81870e3afbb854ea03524ea69dcae9f1bb91e6db37be0121`. Complete original stream. Selected case IDs 6,7,10,11,18,19,22,23,30,31,34,35,42,43,46,47,53,54,55,57,58,59,65,66,67,69,70,71,77,78,79,81,82,83,89,90,91,93,94,95,101,102,103,105,106,107,113,114,115,117,118,119,125,126,127,129,130,131,137,138,139,141,142,143.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-cmd-core/src/native_list_storage.rs](../../../../rust/tcl-cmd-core/src/native_list_storage.rs), `range_storage_action`: Selects finite original store geometry through independent real header/store sharing and used/allocated extents.
- [rust/tcl-cmd-core/src/native_list_storage.rs](../../../../rust/tcl-cmd-core/src/native_list_storage.rs), `native_list_storage::tests::range_storage_geometry_matches_288_actual_c9_owner_windows` (linked): Compares selected result backing/offset/capacity/span fields within288 original range cases. Header/refcount/whole-publication fields are attached native observations, not asserted by this pure geometry test.

A named test is a coverage binding, not a claim that it executed.

## Replay

No new native run or Rust execution result is claimed. Replay must compile the retained exact probe against independently identified release headers/libraries and preserve separate compile/process status and raw streams before comparing the attached original hashes. Original absolute compile paths are capture metadata, not a portable executable command. No evaluator/source/parser or native allocation authority may be obtained by matching display text. Probe code is input/observer code; absent native source excerpts supply no implementation explanation.
