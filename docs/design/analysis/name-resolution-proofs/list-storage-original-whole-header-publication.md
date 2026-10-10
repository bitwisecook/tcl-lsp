# naming.list-storage.original-whole-header-publication

Kind: `native-observation`

## Problem statement

A whole list range can retain the original header or allocate a new header that shares its backing. Result publication adds a genuine reference independently of either identity, so a single shared flag is insufficient.

## Question

Does a complete C9 private list range retain the original header/store under original owner1, a second header reference, and a separate duplicate header?

## Conclusion

Every measured whole range under original owner1 or an independently duplicated header returns the original header/store. A second reference on the same original header yields a distinct result header sharing its backing. Public Tcl_SetObjResult supplies the printed result role: reused headers have refs2 for owner+result; the newly allocated result has refs1 while original retains refs2. These original owners are real references; no compiler stack ownership is inferred.

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
C|0|100|100|0|0|99
S|0|before|1|0|0|100|100|1|0|-1|-1|0|1
R|0|1
S|0|after-original|2|0|0|100|100|1|0|-1|-1|0|1
S|0|result|2|0|0|100|100|1|0|-1|-1|0|1
C|1|100|100|0|0|99
S|1|before|1|0|0|100|100|1|0|-1|-1|0|1
R|1|1
S|1|after-original|2|0|0|100|100|1|0|-1|-1|0|1
S|1|result|2|0|0|100|100|1|0|-1|-1|0|1
C|4|100|100|1|0|99
S|4|before|2|0|0|100|100|1|0|-1|-1|0|1
R|4|0
S|4|after-original|2|0|0|100|100|2|0|-1|-1|0|1
S|4|result|1|0|0|100|100|2|0|-1|-1|0|1
C|5|100|100|1|0|99
S|5|before|2|0|0|100|100|1|0|-1|-1|0|1
R|5|0
S|5|after-original|2|0|0|100|100|2|0|-1|-1|0|1
S|5|result|1|0|0|100|100|2|0|-1|-1|0|1
C|8|100|100|2|0|99
S|8|before|1|0|0|100|100|2|0|-1|-1|0|1
R|8|1
S|8|after-original|2|0|0|100|100|2|0|-1|-1|0|1
S|8|result|2|0|0|100|100|2|0|-1|-1|0|1
C|9|100|100|2|0|99
S|9|before|1|0|0|100|100|2|0|-1|-1|0|1
R|9|1
S|9|after-original|2|0|0|100|100|2|0|-1|-1|0|1
S|9|result|2|0|0|100|100|2|0|-1|-1|0|1
C|12|100|300|0|0|99
S|12|before|1|0|0|100|300|1|0|-1|-1|0|1
R|12|1
S|12|after-original|2|0|0|100|300|1|0|-1|-1|0|1
S|12|result|2|0|0|100|300|1|0|-1|-1|0|1
C|13|100|300|0|0|99
S|13|before|1|0|0|100|300|1|0|-1|-1|0|1
R|13|1
S|13|after-original|2|0|0|100|300|1|0|-1|-1|0|1
S|13|result|2|0|0|100|300|1|0|-1|-1|0|1
C|16|100|300|1|0|99
S|16|before|2|0|0|100|300|1|0|-1|-1|0|1
R|16|0
S|16|after-original|2|0|0|100|300|2|0|-1|-1|0|1
S|16|result|1|0|0|100|300|2|0|-1|-1|0|1
C|17|100|300|1|0|99
S|17|before|2|0|0|100|300|1|0|-1|-1|0|1
R|17|0
S|17|after-original|2|0|0|100|300|2|0|-1|-1|0|1
S|17|result|1|0|0|100|300|2|0|-1|-1|0|1
C|20|100|300|2|0|99
S|20|before|1|0|0|100|300|2|0|-1|-1|0|1
R|20|1
S|20|after-original|2|0|0|100|300|2|0|-1|-1|0|1
S|20|result|2|0|0|100|300|2|0|-1|-1|0|1
C|21|100|300|2|0|99
S|21|before|1|0|0|100|300|2|0|-1|-1|0|1
R|21|1
S|21|after-original|2|0|0|100|300|2|0|-1|-1|0|1
S|21|result|2|0|0|100|300|2|0|-1|-1|0|1
C|24|101|101|0|0|100
S|24|before|1|0|0|101|101|1|0|-1|-1|0|1
R|24|1
S|24|after-original|2|0|0|101|101|1|0|-1|-1|0|1
S|24|result|2|0|0|101|101|1|0|-1|-1|0|1
C|25|101|101|0|0|100
S|25|before|1|0|0|101|101|1|0|-1|-1|0|1
R|25|1
S|25|after-original|2|0|0|101|101|1|0|-1|-1|0|1
S|25|result|2|0|0|101|101|1|0|-1|-1|0|1
C|28|101|101|1|0|100
S|28|before|2|0|0|101|101|1|0|-1|-1|0|1
R|28|0
S|28|after-original|2|0|0|101|101|2|0|-1|-1|0|1
S|28|result|1|0|0|101|101|2|0|-1|-1|0|1
C|29|101|101|1|0|100
S|29|before|2|0|0|101|101|1|0|-1|-1|0|1
R|29|0
S|29|after-original|2|0|0|101|101|2|0|-1|-1|0|1
S|29|result|1|0|0|101|101|2|0|-1|-1|0|1
C|32|101|101|2|0|100
S|32|before|1|0|0|101|101|2|0|-1|-1|0|1
R|32|1
S|32|after-original|2|0|0|101|101|2|0|-1|-1|0|1
S|32|result|2|0|0|101|101|2|0|-1|-1|0|1
C|33|101|101|2|0|100
S|33|before|1|0|0|101|101|2|0|-1|-1|0|1
R|33|1
S|33|after-original|2|0|0|101|101|2|0|-1|-1|0|1
S|33|result|2|0|0|101|101|2|0|-1|-1|0|1
C|36|101|303|0|0|100
S|36|before|1|0|0|101|303|1|0|-1|-1|0|1
R|36|1
S|36|after-original|2|0|0|101|303|1|0|-1|-1|0|1
S|36|result|2|0|0|101|303|1|0|-1|-1|0|1
C|37|101|303|0|0|100
S|37|before|1|0|0|101|303|1|0|-1|-1|0|1
R|37|1
S|37|after-original|2|0|0|101|303|1|0|-1|-1|0|1
S|37|result|2|0|0|101|303|1|0|-1|-1|0|1
C|40|101|303|1|0|100
S|40|before|2|0|0|101|303|1|0|-1|-1|0|1
R|40|0
S|40|after-original|2|0|0|101|303|2|0|-1|-1|0|1
S|40|result|1|0|0|101|303|2|0|-1|-1|0|1
C|41|101|303|1|0|100
S|41|before|2|0|0|101|303|1|0|-1|-1|0|1
R|41|0
S|41|after-original|2|0|0|101|303|2|0|-1|-1|0|1
S|41|result|1|0|0|101|303|2|0|-1|-1|0|1
C|44|101|303|2|0|100
S|44|before|1|0|0|101|303|2|0|-1|-1|0|1
R|44|1
S|44|after-original|2|0|0|101|303|2|0|-1|-1|0|1
S|44|result|2|0|0|101|303|2|0|-1|-1|0|1
C|45|101|303|2|0|100
S|45|before|1|0|0|101|303|2|0|-1|-1|0|1
R|45|1
S|45|after-original|2|0|0|101|303|2|0|-1|-1|0|1
S|45|result|2|0|0|101|303|2|0|-1|-1|0|1
C|48|202|202|0|0|201
S|48|before|1|0|0|202|202|1|0|-1|-1|0|1
R|48|1
S|48|after-original|2|0|0|202|202|1|0|-1|-1|0|1
S|48|result|2|0|0|202|202|1|0|-1|-1|0|1
C|52|202|202|1|0|201
S|52|before|2|0|0|202|202|1|0|-1|-1|0|1
R|52|0
S|52|after-original|2|0|0|202|202|2|0|-1|-1|0|1
S|52|result|1|0|0|202|202|2|0|-1|-1|0|1
C|56|202|202|2|0|201
S|56|before|1|0|0|202|202|2|0|-1|-1|0|1
R|56|1
S|56|after-original|2|0|0|202|202|2|0|-1|-1|0|1
S|56|result|2|0|0|202|202|2|0|-1|-1|0|1
C|60|202|606|0|0|201
S|60|before|1|0|0|202|606|1|0|-1|-1|0|1
R|60|1
S|60|after-original|2|0|0|202|606|1|0|-1|-1|0|1
S|60|result|2|0|0|202|606|1|0|-1|-1|0|1
C|64|202|606|1|0|201
S|64|before|2|0|0|202|606|1|0|-1|-1|0|1
R|64|0
S|64|after-original|2|0|0|202|606|2|0|-1|-1|0|1
S|64|result|1|0|0|202|606|2|0|-1|-1|0|1
C|68|202|606|2|0|201
S|68|before|1|0|0|202|606|2|0|-1|-1|0|1
R|68|1
S|68|after-original|2|0|0|202|606|2|0|-1|-1|0|1
S|68|result|2|0|0|202|606|2|0|-1|-1|0|1
C|72|203|203|0|0|202
S|72|before|1|0|0|203|203|1|0|-1|-1|0|1
R|72|1
S|72|after-original|2|0|0|203|203|1|0|-1|-1|0|1
S|72|result|2|0|0|203|203|1|0|-1|-1|0|1
C|76|203|203|1|0|202
S|76|before|2|0|0|203|203|1|0|-1|-1|0|1
R|76|0
S|76|after-original|2|0|0|203|203|2|0|-1|-1|0|1
S|76|result|1|0|0|203|203|2|0|-1|-1|0|1
C|80|203|203|2|0|202
S|80|before|1|0|0|203|203|2|0|-1|-1|0|1
R|80|1
S|80|after-original|2|0|0|203|203|2|0|-1|-1|0|1
S|80|result|2|0|0|203|203|2|0|-1|-1|0|1
C|84|203|609|0|0|202
S|84|before|1|0|0|203|609|1|0|-1|-1|0|1
R|84|1
S|84|after-original|2|0|0|203|609|1|0|-1|-1|0|1
S|84|result|2|0|0|203|609|1|0|-1|-1|0|1
C|88|203|609|1|0|202
S|88|before|2|0|0|203|609|1|0|-1|-1|0|1
R|88|0
S|88|after-original|2|0|0|203|609|2|0|-1|-1|0|1
S|88|result|1|0|0|203|609|2|0|-1|-1|0|1
C|92|203|609|2|0|202
S|92|before|1|0|0|203|609|2|0|-1|-1|0|1
R|92|1
S|92|after-original|2|0|0|203|609|2|0|-1|-1|0|1
S|92|result|2|0|0|203|609|2|0|-1|-1|0|1
C|96|300|300|0|0|299
S|96|before|1|0|0|300|300|1|0|-1|-1|0|1
R|96|1
S|96|after-original|2|0|0|300|300|1|0|-1|-1|0|1
S|96|result|2|0|0|300|300|1|0|-1|-1|0|1
C|100|300|300|1|0|299
S|100|before|2|0|0|300|300|1|0|-1|-1|0|1
R|100|0
S|100|after-original|2|0|0|300|300|2|0|-1|-1|0|1
S|100|result|1|0|0|300|300|2|0|-1|-1|0|1
C|104|300|300|2|0|299
S|104|before|1|0|0|300|300|2|0|-1|-1|0|1
R|104|1
S|104|after-original|2|0|0|300|300|2|0|-1|-1|0|1
S|104|result|2|0|0|300|300|2|0|-1|-1|0|1
C|108|300|900|0|0|299
S|108|before|1|0|0|300|900|1|0|-1|-1|0|1
R|108|1
S|108|after-original|2|0|0|300|900|1|0|-1|-1|0|1
S|108|result|2|0|0|300|900|1|0|-1|-1|0|1
C|112|300|900|1|0|299
S|112|before|2|0|0|300|900|1|0|-1|-1|0|1
R|112|0
S|112|after-original|2|0|0|300|900|2|0|-1|-1|0|1
S|112|result|1|0|0|300|900|2|0|-1|-1|0|1
C|116|300|900|2|0|299
S|116|before|1|0|0|300|900|2|0|-1|-1|0|1
R|116|1
S|116|after-original|2|0|0|300|900|2|0|-1|-1|0|1
S|116|result|2|0|0|300|900|2|0|-1|-1|0|1
C|120|301|301|0|0|300
S|120|before|1|0|0|301|301|1|0|-1|-1|0|1
R|120|1
S|120|after-original|2|0|0|301|301|1|0|-1|-1|0|1
S|120|result|2|0|0|301|301|1|0|-1|-1|0|1
C|124|301|301|1|0|300
S|124|before|2|0|0|301|301|1|0|-1|-1|0|1
R|124|0
S|124|after-original|2|0|0|301|301|2|0|-1|-1|0|1
S|124|result|1|0|0|301|301|2|0|-1|-1|0|1
C|128|301|301|2|0|300
S|128|before|1|0|0|301|301|2|0|-1|-1|0|1
R|128|1
S|128|after-original|2|0|0|301|301|2|0|-1|-1|0|1
S|128|result|2|0|0|301|301|2|0|-1|-1|0|1
C|132|301|903|0|0|300
S|132|before|1|0|0|301|903|1|0|-1|-1|0|1
R|132|1
S|132|after-original|2|0|0|301|903|1|0|-1|-1|0|1
S|132|result|2|0|0|301|903|1|0|-1|-1|0|1
C|136|301|903|1|0|300
S|136|before|2|0|0|301|903|1|0|-1|-1|0|1
R|136|0
S|136|after-original|2|0|0|301|903|2|0|-1|-1|0|1
S|136|result|1|0|0|301|903|2|0|-1|-1|0|1
C|140|301|903|2|0|300
S|140|before|1|0|0|301|903|2|0|-1|-1|0|1
R|140|1
S|140|after-original|2|0|0|301|903|2|0|-1|-1|0|1
S|140|result|2|0|0|301|903|2|0|-1|-1|0|1
```

### tcl9.1

Status: `observed`. Version: 9.1.0; exact launched patchlevel not queried by this probe. Build: native_header_sha256=fea9d99020df9aec6efebf6e3f18592ee267a9bf80d3843df0d6439efebe65cc; native_implementation_sha256=6711651457a8f8813f37c71e5e8958aca9d2e83a6fc7c7e281ae3f7ada10e81b; library_sha256=8b2dba836908287f95f26a442bb065601daac300fdeee48b15e64a5c03894e33; binary_sha256=37920f13d6672f2d491bee665d1f420d02b07f581b3296ffa53e2bb533354b0f; compile_status=0; run_status=0. Compiler/version/configuration fields absent from this original receipt remain unknown.. Channel: Original native List allocation and private TclListObjRange; public Tcl_SetObjResult publication.. Dialect: C Tcl.

Exact selected C/R/S rows. C fields are id,used,allocated,sharing,first,last. R records header==original. S records id/window,header refs,resident,firstUsed,numUsed,capacity,store refs,span present/start/length/refs,store==original.

```text
C|0|100|100|0|0|99
S|0|before|1|0|0|100|100|1|0|-1|-1|0|1
R|0|1
S|0|after-original|2|0|0|100|100|1|0|-1|-1|0|1
S|0|result|2|0|0|100|100|1|0|-1|-1|0|1
C|1|100|100|0|0|99
S|1|before|1|0|0|100|100|1|0|-1|-1|0|1
R|1|1
S|1|after-original|2|0|0|100|100|1|0|-1|-1|0|1
S|1|result|2|0|0|100|100|1|0|-1|-1|0|1
C|4|100|100|1|0|99
S|4|before|2|0|0|100|100|1|0|-1|-1|0|1
R|4|0
S|4|after-original|2|0|0|100|100|2|0|-1|-1|0|1
S|4|result|1|0|0|100|100|2|0|-1|-1|0|1
C|5|100|100|1|0|99
S|5|before|2|0|0|100|100|1|0|-1|-1|0|1
R|5|0
S|5|after-original|2|0|0|100|100|2|0|-1|-1|0|1
S|5|result|1|0|0|100|100|2|0|-1|-1|0|1
C|8|100|100|2|0|99
S|8|before|1|0|0|100|100|2|0|-1|-1|0|1
R|8|1
S|8|after-original|2|0|0|100|100|2|0|-1|-1|0|1
S|8|result|2|0|0|100|100|2|0|-1|-1|0|1
C|9|100|100|2|0|99
S|9|before|1|0|0|100|100|2|0|-1|-1|0|1
R|9|1
S|9|after-original|2|0|0|100|100|2|0|-1|-1|0|1
S|9|result|2|0|0|100|100|2|0|-1|-1|0|1
C|12|100|300|0|0|99
S|12|before|1|0|0|100|300|1|0|-1|-1|0|1
R|12|1
S|12|after-original|2|0|0|100|300|1|0|-1|-1|0|1
S|12|result|2|0|0|100|300|1|0|-1|-1|0|1
C|13|100|300|0|0|99
S|13|before|1|0|0|100|300|1|0|-1|-1|0|1
R|13|1
S|13|after-original|2|0|0|100|300|1|0|-1|-1|0|1
S|13|result|2|0|0|100|300|1|0|-1|-1|0|1
C|16|100|300|1|0|99
S|16|before|2|0|0|100|300|1|0|-1|-1|0|1
R|16|0
S|16|after-original|2|0|0|100|300|2|0|-1|-1|0|1
S|16|result|1|0|0|100|300|2|0|-1|-1|0|1
C|17|100|300|1|0|99
S|17|before|2|0|0|100|300|1|0|-1|-1|0|1
R|17|0
S|17|after-original|2|0|0|100|300|2|0|-1|-1|0|1
S|17|result|1|0|0|100|300|2|0|-1|-1|0|1
C|20|100|300|2|0|99
S|20|before|1|0|0|100|300|2|0|-1|-1|0|1
R|20|1
S|20|after-original|2|0|0|100|300|2|0|-1|-1|0|1
S|20|result|2|0|0|100|300|2|0|-1|-1|0|1
C|21|100|300|2|0|99
S|21|before|1|0|0|100|300|2|0|-1|-1|0|1
R|21|1
S|21|after-original|2|0|0|100|300|2|0|-1|-1|0|1
S|21|result|2|0|0|100|300|2|0|-1|-1|0|1
C|24|101|101|0|0|100
S|24|before|1|0|0|101|101|1|0|-1|-1|0|1
R|24|1
S|24|after-original|2|0|0|101|101|1|0|-1|-1|0|1
S|24|result|2|0|0|101|101|1|0|-1|-1|0|1
C|25|101|101|0|0|100
S|25|before|1|0|0|101|101|1|0|-1|-1|0|1
R|25|1
S|25|after-original|2|0|0|101|101|1|0|-1|-1|0|1
S|25|result|2|0|0|101|101|1|0|-1|-1|0|1
C|28|101|101|1|0|100
S|28|before|2|0|0|101|101|1|0|-1|-1|0|1
R|28|0
S|28|after-original|2|0|0|101|101|2|0|-1|-1|0|1
S|28|result|1|0|0|101|101|2|0|-1|-1|0|1
C|29|101|101|1|0|100
S|29|before|2|0|0|101|101|1|0|-1|-1|0|1
R|29|0
S|29|after-original|2|0|0|101|101|2|0|-1|-1|0|1
S|29|result|1|0|0|101|101|2|0|-1|-1|0|1
C|32|101|101|2|0|100
S|32|before|1|0|0|101|101|2|0|-1|-1|0|1
R|32|1
S|32|after-original|2|0|0|101|101|2|0|-1|-1|0|1
S|32|result|2|0|0|101|101|2|0|-1|-1|0|1
C|33|101|101|2|0|100
S|33|before|1|0|0|101|101|2|0|-1|-1|0|1
R|33|1
S|33|after-original|2|0|0|101|101|2|0|-1|-1|0|1
S|33|result|2|0|0|101|101|2|0|-1|-1|0|1
C|36|101|303|0|0|100
S|36|before|1|0|0|101|303|1|0|-1|-1|0|1
R|36|1
S|36|after-original|2|0|0|101|303|1|0|-1|-1|0|1
S|36|result|2|0|0|101|303|1|0|-1|-1|0|1
C|37|101|303|0|0|100
S|37|before|1|0|0|101|303|1|0|-1|-1|0|1
R|37|1
S|37|after-original|2|0|0|101|303|1|0|-1|-1|0|1
S|37|result|2|0|0|101|303|1|0|-1|-1|0|1
C|40|101|303|1|0|100
S|40|before|2|0|0|101|303|1|0|-1|-1|0|1
R|40|0
S|40|after-original|2|0|0|101|303|2|0|-1|-1|0|1
S|40|result|1|0|0|101|303|2|0|-1|-1|0|1
C|41|101|303|1|0|100
S|41|before|2|0|0|101|303|1|0|-1|-1|0|1
R|41|0
S|41|after-original|2|0|0|101|303|2|0|-1|-1|0|1
S|41|result|1|0|0|101|303|2|0|-1|-1|0|1
C|44|101|303|2|0|100
S|44|before|1|0|0|101|303|2|0|-1|-1|0|1
R|44|1
S|44|after-original|2|0|0|101|303|2|0|-1|-1|0|1
S|44|result|2|0|0|101|303|2|0|-1|-1|0|1
C|45|101|303|2|0|100
S|45|before|1|0|0|101|303|2|0|-1|-1|0|1
R|45|1
S|45|after-original|2|0|0|101|303|2|0|-1|-1|0|1
S|45|result|2|0|0|101|303|2|0|-1|-1|0|1
C|48|202|202|0|0|201
S|48|before|1|0|0|202|202|1|0|-1|-1|0|1
R|48|1
S|48|after-original|2|0|0|202|202|1|0|-1|-1|0|1
S|48|result|2|0|0|202|202|1|0|-1|-1|0|1
C|52|202|202|1|0|201
S|52|before|2|0|0|202|202|1|0|-1|-1|0|1
R|52|0
S|52|after-original|2|0|0|202|202|2|0|-1|-1|0|1
S|52|result|1|0|0|202|202|2|0|-1|-1|0|1
C|56|202|202|2|0|201
S|56|before|1|0|0|202|202|2|0|-1|-1|0|1
R|56|1
S|56|after-original|2|0|0|202|202|2|0|-1|-1|0|1
S|56|result|2|0|0|202|202|2|0|-1|-1|0|1
C|60|202|606|0|0|201
S|60|before|1|0|0|202|606|1|0|-1|-1|0|1
R|60|1
S|60|after-original|2|0|0|202|606|1|0|-1|-1|0|1
S|60|result|2|0|0|202|606|1|0|-1|-1|0|1
C|64|202|606|1|0|201
S|64|before|2|0|0|202|606|1|0|-1|-1|0|1
R|64|0
S|64|after-original|2|0|0|202|606|2|0|-1|-1|0|1
S|64|result|1|0|0|202|606|2|0|-1|-1|0|1
C|68|202|606|2|0|201
S|68|before|1|0|0|202|606|2|0|-1|-1|0|1
R|68|1
S|68|after-original|2|0|0|202|606|2|0|-1|-1|0|1
S|68|result|2|0|0|202|606|2|0|-1|-1|0|1
C|72|203|203|0|0|202
S|72|before|1|0|0|203|203|1|0|-1|-1|0|1
R|72|1
S|72|after-original|2|0|0|203|203|1|0|-1|-1|0|1
S|72|result|2|0|0|203|203|1|0|-1|-1|0|1
C|76|203|203|1|0|202
S|76|before|2|0|0|203|203|1|0|-1|-1|0|1
R|76|0
S|76|after-original|2|0|0|203|203|2|0|-1|-1|0|1
S|76|result|1|0|0|203|203|2|0|-1|-1|0|1
C|80|203|203|2|0|202
S|80|before|1|0|0|203|203|2|0|-1|-1|0|1
R|80|1
S|80|after-original|2|0|0|203|203|2|0|-1|-1|0|1
S|80|result|2|0|0|203|203|2|0|-1|-1|0|1
C|84|203|609|0|0|202
S|84|before|1|0|0|203|609|1|0|-1|-1|0|1
R|84|1
S|84|after-original|2|0|0|203|609|1|0|-1|-1|0|1
S|84|result|2|0|0|203|609|1|0|-1|-1|0|1
C|88|203|609|1|0|202
S|88|before|2|0|0|203|609|1|0|-1|-1|0|1
R|88|0
S|88|after-original|2|0|0|203|609|2|0|-1|-1|0|1
S|88|result|1|0|0|203|609|2|0|-1|-1|0|1
C|92|203|609|2|0|202
S|92|before|1|0|0|203|609|2|0|-1|-1|0|1
R|92|1
S|92|after-original|2|0|0|203|609|2|0|-1|-1|0|1
S|92|result|2|0|0|203|609|2|0|-1|-1|0|1
C|96|300|300|0|0|299
S|96|before|1|0|0|300|300|1|0|-1|-1|0|1
R|96|1
S|96|after-original|2|0|0|300|300|1|0|-1|-1|0|1
S|96|result|2|0|0|300|300|1|0|-1|-1|0|1
C|100|300|300|1|0|299
S|100|before|2|0|0|300|300|1|0|-1|-1|0|1
R|100|0
S|100|after-original|2|0|0|300|300|2|0|-1|-1|0|1
S|100|result|1|0|0|300|300|2|0|-1|-1|0|1
C|104|300|300|2|0|299
S|104|before|1|0|0|300|300|2|0|-1|-1|0|1
R|104|1
S|104|after-original|2|0|0|300|300|2|0|-1|-1|0|1
S|104|result|2|0|0|300|300|2|0|-1|-1|0|1
C|108|300|900|0|0|299
S|108|before|1|0|0|300|900|1|0|-1|-1|0|1
R|108|1
S|108|after-original|2|0|0|300|900|1|0|-1|-1|0|1
S|108|result|2|0|0|300|900|1|0|-1|-1|0|1
C|112|300|900|1|0|299
S|112|before|2|0|0|300|900|1|0|-1|-1|0|1
R|112|0
S|112|after-original|2|0|0|300|900|2|0|-1|-1|0|1
S|112|result|1|0|0|300|900|2|0|-1|-1|0|1
C|116|300|900|2|0|299
S|116|before|1|0|0|300|900|2|0|-1|-1|0|1
R|116|1
S|116|after-original|2|0|0|300|900|2|0|-1|-1|0|1
S|116|result|2|0|0|300|900|2|0|-1|-1|0|1
C|120|301|301|0|0|300
S|120|before|1|0|0|301|301|1|0|-1|-1|0|1
R|120|1
S|120|after-original|2|0|0|301|301|1|0|-1|-1|0|1
S|120|result|2|0|0|301|301|1|0|-1|-1|0|1
C|124|301|301|1|0|300
S|124|before|2|0|0|301|301|1|0|-1|-1|0|1
R|124|0
S|124|after-original|2|0|0|301|301|2|0|-1|-1|0|1
S|124|result|1|0|0|301|301|2|0|-1|-1|0|1
C|128|301|301|2|0|300
S|128|before|1|0|0|301|301|2|0|-1|-1|0|1
R|128|1
S|128|after-original|2|0|0|301|301|2|0|-1|-1|0|1
S|128|result|2|0|0|301|301|2|0|-1|-1|0|1
C|132|301|903|0|0|300
S|132|before|1|0|0|301|903|1|0|-1|-1|0|1
R|132|1
S|132|after-original|2|0|0|301|903|1|0|-1|-1|0|1
S|132|result|2|0|0|301|903|1|0|-1|-1|0|1
C|136|301|903|1|0|300
S|136|before|2|0|0|301|903|1|0|-1|-1|0|1
R|136|0
S|136|after-original|2|0|0|301|903|2|0|-1|-1|0|1
S|136|result|1|0|0|301|903|2|0|-1|-1|0|1
C|140|301|903|2|0|300
S|140|before|1|0|0|301|903|2|0|-1|-1|0|1
R|140|1
S|140|after-original|2|0|0|301|903|2|0|-1|-1|0|1
S|140|result|2|0|0|301|903|2|0|-1|-1|0|1
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
- `rows-tcl9.0` (observation): [rust/tcl-cmd-core/tests/data/native_list_storage/9.0.4.txt](../../../../rust/tcl-cmd-core/tests/data/native_list_storage/9.0.4.txt). SHA-256 `7ee6709535f5e14c81870e3afbb854ea03524ea69dcae9f1bb91e6db37be0121`. Complete original stream. Selected case IDs 0,1,4,5,8,9,12,13,16,17,20,21,24,25,28,29,32,33,36,37,40,41,44,45,48,52,56,60,64,68,72,76,80,84,88,92,96,100,104,108,112,116,120,124,128,132,136,140.
- `rows-tcl9.1` (observation): [rust/tcl-cmd-core/tests/data/native_list_storage/9.1.0.txt](../../../../rust/tcl-cmd-core/tests/data/native_list_storage/9.1.0.txt). SHA-256 `7ee6709535f5e14c81870e3afbb854ea03524ea69dcae9f1bb91e6db37be0121`. Complete original stream. Selected case IDs 0,1,4,5,8,9,12,13,16,17,20,21,24,25,28,29,32,33,36,37,40,41,44,45,48,52,56,60,64,68,72,76,80,84,88,92,96,100,104,108,112,116,120,124,128,132,136,140.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-cmd-core/src/native_list_storage.rs](../../../../rust/tcl-cmd-core/src/native_list_storage.rs), `range_storage_action`: Selects finite original store geometry through independent real header/store sharing and used/allocated extents.
- [rust/tcl-cmd-core/src/native_list_storage.rs](../../../../rust/tcl-cmd-core/src/native_list_storage.rs), `native_list_storage::tests::range_storage_geometry_matches_288_actual_c9_owner_windows` (linked): Compares selected result backing/offset/capacity/span fields within288 original range cases. Header/refcount/whole-publication fields are attached native observations, not asserted by this pure geometry test.

A named test is a coverage binding, not a claim that it executed.

## Replay

No new native run or Rust execution result is claimed. Replay must compile the retained exact probe against independently identified release headers/libraries and preserve separate compile/process status and raw streams before comparing the attached original hashes. Original absolute compile paths are capture metadata, not a portable executable command. No evaluator/source/parser or native allocation authority may be obtained by matching display text. Probe code is input/observer code; absent native source excerpts supply no implementation explanation.
