# naming.each-loop.original-root-shimmer-and-refetch

Kind: `native-observation`

## Problem statement

Character counting on an original list root can retire its member storage while a loop is active. One declared external observer reference prevents dereferencing retired children but must not be mistaken for a loop-owned role; resident spelling affects the next native conversion.

## Question

What original root/child/header state and later body refetch follows a first-body character-length conversion with explicit observer pins?

## Conclusion

The first reached body invokes native character length on both original roots. C8.4/8.5 and Jim convert both roots to String even when their spelling was already resident; C8.6–9.1 retain already-resident List roots in case1 but convert absent-string roots in cases0/2/5/8 to String. C8.4 drops the original name/member list role immediately; later C retains their entered role through the current borrowed backing then releases it after. Jim retires the list role and re-fetches a List in the later body of cases2/5, with the later cell carrying a source primary. The explicit name/member observer refs account for surviving children. Exact unsupported C84/85 lmap and nonentered validation cases remain included without shimmer claims.

## Scope

This is the distinct shimmer probe: ten exact cases ×six providers,60 guest completions and217 sampled windows. It adds one real reference each to the original first name/member, calls Tcl_GetCharLength or Jim_Utf8Length only in the first actually reached body, and records before/body/shimmer/after prior to result rendering. Original189-window data is a separate capture and is not substituted. No future mutation, arbitrary backing identity, compiled-loop or Rust execution claim.

## Provider answers

### tcl8.4

Status: `observed`. Version: tcl8.4; exact launched patchlevel not queried by this probe. Build: library_sha256=d71ed42efd90354cdc99f73ba39b30e2cdedda463768a7fd025114b50db60011; binary_sha256=81df49251726e26129fe4af3d6e8f1b908b38a89b98edad3d04d5073aa2c931c; compile_exit=0; exit=0. Compiler/version/configuration fields absent from this original receipt remain unknown.. Channel: Original List object-vector evaluator plus actual first-body character getter; no document source rewrite.. Dialect: C Tcl.

Exact original shimmer stream with the same S/R column layout as the original member-role record. Explicit observer pins are included in refcounts.

```tsv
S	0	0	before	list,1,0,-1	list,1,0,-1	none,2,1,-1	none,2,1,-1	none,1,1,-1	absent,-1,0,-1	0	none,1,1,-1
S	0	1	write	list,1,0,-1	list,1,0,-1	parsedVarName,2,1,-1	none,4,1,-1	none,1,1,-1	none,4,1,-1	1	none,1,1,-1
S	0	2	body	list,1,0,-1	list,1,0,-1	parsedVarName,2,1,-1	none,3,1,-1	bytecode,2,1,-1	none,3,1,-1	1	none,1,1,-1
S	0	3	shimmer	string,1,1,-1	string,1,1,-1	parsedVarName,1,1,-1	none,2,1,-1	bytecode,2,1,-1	none,2,1,-1	1	none,1,1,-1
S	0	4	after	string,1,1,-1	string,1,1,-1	parsedVarName,1,1,-1	none,2,1,-1	bytecode,1,1,-1	absent,-1,0,-1	0	none,1,1,-1
R	0	0	
S	1	0	before	list,1,1,-1	list,1,1,-1	none,2,1,-1	none,2,1,-1	none,1,1,-1	absent,-1,0,-1	0	none,1,1,-1
S	1	1	write	list,1,1,-1	list,1,1,-1	parsedVarName,2,1,-1	none,4,1,-1	none,1,1,-1	none,4,1,-1	1	none,1,1,-1
S	1	2	body	list,1,1,-1	list,1,1,-1	parsedVarName,2,1,-1	none,3,1,-1	bytecode,2,1,-1	none,3,1,-1	1	none,1,1,-1
S	1	3	shimmer	string,1,1,-1	string,1,1,-1	parsedVarName,1,1,-1	none,2,1,-1	bytecode,2,1,-1	none,2,1,-1	1	none,1,1,-1
S	1	4	after	string,1,1,-1	string,1,1,-1	parsedVarName,1,1,-1	none,2,1,-1	bytecode,1,1,-1	absent,-1,0,-1	0	none,1,1,-1
R	1	0	
S	2	0	before	list,1,0,-1	list,1,0,-1	none,2,1,-1	none,2,1,-1	none,1,1,-1	absent,-1,0,-1	0	none,1,1,-1
S	2	1	write	list,1,0,-1	list,1,0,-1	parsedVarName,2,1,-1	none,4,1,-1	none,1,1,-1	none,4,1,-1	1	none,1,1,-1
S	2	2	body	list,1,0,-1	list,1,0,-1	parsedVarName,2,1,-1	none,3,1,-1	bytecode,2,1,-1	none,3,1,-1	1	none,1,1,-1
S	2	3	shimmer	string,1,1,-1	string,1,1,-1	parsedVarName,1,1,-1	none,2,1,-1	bytecode,2,1,-1	none,2,1,-1	1	none,1,1,-1
S	2	4	write	list,1,1,-1	list,1,1,-1	parsedVarName,1,1,-1	none,1,1,-1	bytecode,1,1,-1	none,3,1,-1	0	none,1,1,-1
S	2	5	body	list,1,1,-1	list,1,1,-1	parsedVarName,1,1,-1	none,1,1,-1	bytecode,2,1,-1	none,2,1,-1	0	none,1,1,-1
S	2	6	after	list,1,1,-1	list,1,1,-1	parsedVarName,1,1,-1	none,1,1,-1	bytecode,1,1,-1	absent,-1,0,-1	0	none,1,1,-1
R	2	0	
S	3	0	before	list,1,0,-1	none,1,1,-1	none,2,1,-1	absent,-1,0,-1	none,1,1,-1	absent,-1,0,-1	0	none,1,1,-1
S	3	1	after	list,1,0,-1	list,1,1,-1	none,2,1,-1	absent,-1,0,-1	none,1,1,-1	absent,-1,0,-1	0	none,1,1,-1
R	3	0	
S	4	0	before	list,1,0,-1	none,1,1,-1	none,2,1,-1	absent,-1,0,-1	none,1,1,-1	absent,-1,0,-1	0	none,1,1,-1
S	4	1	after	list,1,1,-1	none,1,1,-1	none,2,1,-1	absent,-1,0,-1	none,1,1,-1	absent,-1,0,-1	0	string,1,1,-1
R	4	1	696e76616c696420636f6d6d616e64206e616d6520226c6d617022
S	5	0	before	list,1,0,-1	list,1,0,-1	none,2,1,-1	none,2,1,-1	none,1,1,-1	absent,-1,0,-1	0	none,1,1,-1
S	5	1	after	list,1,1,-1	list,1,1,-1	none,2,1,-1	none,2,1,-1	none,1,1,-1	absent,-1,0,-1	0	string,1,1,-1
R	5	1	696e76616c696420636f6d6d616e64206e616d6520226c6d617022
S	6	0	before	list,1,0,-1	none,1,1,-1	none,2,1,-1	absent,-1,0,-1	none,1,1,-1	absent,-1,0,-1	0	none,1,1,-1
S	6	1	after	list,1,1,-1	none,1,1,-1	none,2,1,-1	absent,-1,0,-1	none,1,1,-1	absent,-1,0,-1	0	none,1,1,-1
R	6	1	756e6d617463686564206f70656e20627261636520696e206c697374
S	7	0	before	none,1,1,-1	none,1,1,-1	absent,-1,0,-1	absent,-1,0,-1	none,1,1,-1	absent,-1,0,-1	0	none,1,1,-1
S	7	1	after	list,1,1,-1	none,1,1,-1	absent,-1,0,-1	absent,-1,0,-1	none,1,1,-1	absent,-1,0,-1	0	string,1,1,-1
R	7	1	666f7265616368207661726c69737420697320656d707479
S	8	0	before	list,1,0,-1	list,1,0,-1	none,2,1,-1	list,2,0,-1	none,1,1,-1	absent,-1,0,-1	0	none,1,1,-1
S	8	1	write	list,1,0,-1	list,1,0,-1	parsedVarName,2,1,-1	list,4,0,-1	none,1,1,-1	list,4,0,-1	1	none,1,1,-1
S	8	2	body	list,1,0,-1	list,1,0,-1	parsedVarName,2,1,-1	list,3,0,-1	bytecode,2,1,-1	list,3,0,-1	1	none,1,1,-1
S	8	3	shimmer	string,1,1,-1	string,1,1,-1	parsedVarName,1,1,-1	list,2,1,-1	bytecode,2,1,-1	list,2,1,-1	1	none,1,1,-1
S	8	4	after	string,1,1,-1	string,1,1,-1	parsedVarName,1,1,-1	list,2,1,-1	bytecode,1,1,-1	absent,-1,0,-1	0	none,1,1,-1
R	8	0	
S	9	0	before	none,1,1,-1	list,1,0,-1	absent,-1,0,-1	none,2,1,-1	none,1,1,-1	absent,-1,0,-1	0	none,1,1,-1
S	9	1	after	list,1,1,-1	list,1,1,-1	absent,-1,0,-1	none,2,1,-1	none,1,1,-1	absent,-1,0,-1	0	string,1,1,-1
R	9	1	666f7265616368207661726c69737420697320656d707479
```

### tcl8.5

Status: `observed`. Version: tcl8.5; exact launched patchlevel not queried by this probe. Build: library_sha256=99e0d524e54498713498e43d6cf80fa085471913cb38a53b505e18c0469417df; binary_sha256=f5f1b08cecec7167b2dd7e24282e40d44f3cf43cf73be15e94407842bedc5585; compile_exit=0; exit=0. Compiler/version/configuration fields absent from this original receipt remain unknown.. Channel: Original List object-vector evaluator plus actual first-body character getter; no document source rewrite.. Dialect: C Tcl.

Exact original shimmer stream with the same S/R column layout as the original member-role record. Explicit observer pins are included in refcounts.

```tsv
S	0	0	before	list,1,0,1	list,1,0,1	none,2,1,-1	none,2,1,-1	none,1,1,-1	absent,-1,0,-1	0	none,1,1,-1
S	0	1	write	list,1,0,2	list,1,0,2	parsedVarName,2,1,-1	none,3,1,-1	none,1,1,-1	none,3,1,-1	1	none,2,1,-1
S	0	2	body	list,1,0,2	list,1,0,2	parsedVarName,2,1,-1	none,3,1,-1	bytecode,2,1,-1	none,3,1,-1	1	none,1,1,-1
S	0	3	shimmer	string,1,1,-1	string,1,1,-1	parsedVarName,2,1,-1	none,3,1,-1	bytecode,2,1,-1	none,3,1,-1	1	none,1,1,-1
S	0	4	after	string,1,1,-1	string,1,1,-1	parsedVarName,1,1,-1	none,2,1,-1	bytecode,1,1,-1	absent,-1,0,-1	0	none,1,1,-1
R	0	0	
S	1	0	before	list,1,1,1	list,1,1,1	none,2,1,-1	none,2,1,-1	none,1,1,-1	absent,-1,0,-1	0	none,1,1,-1
S	1	1	write	list,1,1,2	list,1,1,2	parsedVarName,2,1,-1	none,3,1,-1	none,1,1,-1	none,3,1,-1	1	none,2,1,-1
S	1	2	body	list,1,1,2	list,1,1,2	parsedVarName,2,1,-1	none,3,1,-1	bytecode,2,1,-1	none,3,1,-1	1	none,1,1,-1
S	1	3	shimmer	string,1,1,-1	string,1,1,-1	parsedVarName,2,1,-1	none,3,1,-1	bytecode,2,1,-1	none,3,1,-1	1	none,1,1,-1
S	1	4	after	string,1,1,-1	string,1,1,-1	parsedVarName,1,1,-1	none,2,1,-1	bytecode,1,1,-1	absent,-1,0,-1	0	none,1,1,-1
R	1	0	
S	2	0	before	list,1,0,1	list,1,0,1	none,2,1,-1	none,2,1,-1	none,1,1,-1	absent,-1,0,-1	0	none,1,1,-1
S	2	1	write	list,1,0,2	list,1,0,2	parsedVarName,2,1,-1	none,3,1,-1	none,1,1,-1	none,3,1,-1	1	none,2,1,-1
S	2	2	body	list,1,0,2	list,1,0,2	parsedVarName,2,1,-1	none,3,1,-1	bytecode,2,1,-1	none,3,1,-1	1	none,1,1,-1
S	2	3	shimmer	string,1,1,-1	string,1,1,-1	parsedVarName,2,1,-1	none,3,1,-1	bytecode,2,1,-1	none,3,1,-1	1	none,1,1,-1
S	2	4	write	string,1,1,-1	string,1,1,-1	parsedVarName,2,1,-1	none,2,1,-1	bytecode,1,1,-1	none,2,1,-1	0	none,2,1,-1
S	2	5	body	string,1,1,-1	string,1,1,-1	parsedVarName,2,1,-1	none,2,1,-1	bytecode,2,1,-1	none,2,1,-1	0	none,1,1,-1
S	2	6	after	string,1,1,-1	string,1,1,-1	parsedVarName,1,1,-1	none,1,1,-1	bytecode,1,1,-1	absent,-1,0,-1	0	none,1,1,-1
R	2	0	
S	3	0	before	list,1,0,1	none,1,1,-1	none,2,1,-1	absent,-1,0,-1	none,1,1,-1	absent,-1,0,-1	0	none,1,1,-1
S	3	1	after	list,1,0,1	list,1,1,1	none,2,1,-1	absent,-1,0,-1	none,1,1,-1	absent,-1,0,-1	0	none,1,1,-1
R	3	0	
S	4	0	before	list,1,0,1	none,1,1,-1	none,2,1,-1	absent,-1,0,-1	none,1,1,-1	absent,-1,0,-1	0	none,1,1,-1
S	4	1	after	list,1,1,1	none,1,1,-1	none,2,1,-1	absent,-1,0,-1	none,1,1,-1	absent,-1,0,-1	0	string,1,1,-1
R	4	1	696e76616c696420636f6d6d616e64206e616d6520226c6d617022
S	5	0	before	list,1,0,1	list,1,0,1	none,2,1,-1	none,2,1,-1	none,1,1,-1	absent,-1,0,-1	0	none,1,1,-1
S	5	1	after	list,1,1,1	list,1,1,1	none,2,1,-1	none,2,1,-1	none,1,1,-1	absent,-1,0,-1	0	string,1,1,-1
R	5	1	696e76616c696420636f6d6d616e64206e616d6520226c6d617022
S	6	0	before	list,1,0,1	none,1,1,-1	none,2,1,-1	absent,-1,0,-1	none,1,1,-1	absent,-1,0,-1	0	none,1,1,-1
S	6	1	after	list,1,1,1	none,1,1,-1	none,2,1,-1	absent,-1,0,-1	none,1,1,-1	absent,-1,0,-1	0	none,1,1,-1
R	6	1	756e6d617463686564206f70656e20627261636520696e206c697374
S	7	0	before	none,1,1,-1	none,1,1,-1	absent,-1,0,-1	absent,-1,0,-1	none,1,1,-1	absent,-1,0,-1	0	none,1,1,-1
S	7	1	after	list,1,1,1	none,1,1,-1	absent,-1,0,-1	absent,-1,0,-1	none,1,1,-1	absent,-1,0,-1	0	string,1,1,-1
R	7	1	666f7265616368207661726c69737420697320656d707479
S	8	0	before	list,1,0,1	list,1,0,1	none,2,1,-1	list,2,0,1	none,1,1,-1	absent,-1,0,-1	0	none,1,1,-1
S	8	1	write	list,1,0,2	list,1,0,2	parsedVarName,2,1,-1	list,3,0,1	none,1,1,-1	list,3,0,1	1	none,2,1,-1
S	8	2	body	list,1,0,2	list,1,0,2	parsedVarName,2,1,-1	list,3,0,1	bytecode,2,1,-1	list,3,0,1	1	none,1,1,-1
S	8	3	shimmer	string,1,1,-1	string,1,1,-1	parsedVarName,2,1,-1	list,3,1,1	bytecode,2,1,-1	list,3,1,1	1	none,1,1,-1
S	8	4	after	string,1,1,-1	string,1,1,-1	parsedVarName,1,1,-1	list,2,1,1	bytecode,1,1,-1	absent,-1,0,-1	0	none,1,1,-1
R	8	0	
S	9	0	before	none,1,1,-1	list,1,0,1	absent,-1,0,-1	none,2,1,-1	none,1,1,-1	absent,-1,0,-1	0	none,1,1,-1
S	9	1	after	list,1,1,1	list,1,1,1	absent,-1,0,-1	none,2,1,-1	none,1,1,-1	absent,-1,0,-1	0	string,1,1,-1
R	9	1	666f7265616368207661726c69737420697320656d707479
```

### tcl8.6

Status: `observed`. Version: tcl8.6; exact launched patchlevel not queried by this probe. Build: library_sha256=a3a8abdeadd8aafa007d3bc9fa199b9cf48d43d01735ff41b3fcf33abb4019d6; binary_sha256=13e4f7ed80c544affe5ac18fa45cc8a50147b6de9664d46716a9ee8b29d8cf49; compile_exit=0; exit=0. Compiler/version/configuration fields absent from this original receipt remain unknown.. Channel: Original List object-vector evaluator plus actual first-body character getter; no document source rewrite.. Dialect: C Tcl.

Exact original shimmer stream with the same S/R column layout as the original member-role record. Explicit observer pins are included in refcounts.

```tsv
S	0	0	before	list,1,0,1	list,1,0,1	none,2,1,-1	none,2,1,-1	none,1,1,-1	absent,-1,0,-1	0	none,1,1,-1
S	0	1	write	list,1,0,2	list,1,0,2	parsedVarName,2,1,-1	none,3,1,-1	none,1,1,-1	none,3,1,-1	1	none,2,1,-1
S	0	2	body	list,1,0,2	list,1,0,2	parsedVarName,2,1,-1	none,3,1,-1	bytecode,2,1,-1	none,3,1,-1	1	none,1,1,-1
S	0	3	shimmer	string,1,1,-1	string,1,1,-1	parsedVarName,2,1,-1	none,3,1,-1	bytecode,2,1,-1	none,3,1,-1	1	none,1,1,-1
S	0	4	after	string,1,1,-1	string,1,1,-1	parsedVarName,1,1,-1	none,2,1,-1	bytecode,1,1,-1	absent,-1,0,-1	0	none,1,1,-1
R	0	0	
S	1	0	before	list,1,1,1	list,1,1,1	none,2,1,-1	none,2,1,-1	none,1,1,-1	absent,-1,0,-1	0	none,1,1,-1
S	1	1	write	list,1,1,2	list,1,1,2	parsedVarName,2,1,-1	none,3,1,-1	none,1,1,-1	none,3,1,-1	1	none,2,1,-1
S	1	2	body	list,1,1,2	list,1,1,2	parsedVarName,2,1,-1	none,3,1,-1	bytecode,2,1,-1	none,3,1,-1	1	none,1,1,-1
S	1	3	shimmer	list,1,1,2	list,1,1,2	parsedVarName,2,1,-1	none,3,1,-1	bytecode,2,1,-1	none,3,1,-1	1	none,1,1,-1
S	1	4	after	list,1,1,1	list,1,1,1	parsedVarName,2,1,-1	none,3,1,-1	bytecode,1,1,-1	absent,-1,0,-1	0	none,1,1,-1
R	1	0	
S	2	0	before	list,1,0,1	list,1,0,1	none,2,1,-1	none,2,1,-1	none,1,1,-1	absent,-1,0,-1	0	none,1,1,-1
S	2	1	write	list,1,0,2	list,1,0,2	parsedVarName,2,1,-1	none,3,1,-1	none,1,1,-1	none,3,1,-1	1	none,2,1,-1
S	2	2	body	list,1,0,2	list,1,0,2	parsedVarName,2,1,-1	none,3,1,-1	bytecode,2,1,-1	none,3,1,-1	1	none,1,1,-1
S	2	3	shimmer	string,1,1,-1	string,1,1,-1	parsedVarName,2,1,-1	none,3,1,-1	bytecode,2,1,-1	none,3,1,-1	1	none,1,1,-1
S	2	4	write	string,1,1,-1	string,1,1,-1	parsedVarName,2,1,-1	none,2,1,-1	bytecode,1,1,-1	none,2,1,-1	0	none,2,1,-1
S	2	5	body	string,1,1,-1	string,1,1,-1	parsedVarName,2,1,-1	none,2,1,-1	bytecode,2,1,-1	none,2,1,-1	0	none,1,1,-1
S	2	6	after	string,1,1,-1	string,1,1,-1	parsedVarName,1,1,-1	none,1,1,-1	bytecode,1,1,-1	absent,-1,0,-1	0	none,1,1,-1
R	2	0	
S	3	0	before	list,1,0,1	none,1,1,-1	none,2,1,-1	absent,-1,0,-1	none,1,1,-1	absent,-1,0,-1	0	none,1,1,-1
S	3	1	after	list,1,0,1	list,1,1,1	none,2,1,-1	absent,-1,0,-1	none,1,1,-1	absent,-1,0,-1	0	none,1,1,-1
R	3	0	
S	4	0	before	list,1,0,1	none,1,1,-1	none,2,1,-1	absent,-1,0,-1	none,1,1,-1	absent,-1,0,-1	0	none,1,1,-1
S	4	1	after	list,1,0,1	list,1,1,1	none,2,1,-1	absent,-1,0,-1	none,1,1,-1	absent,-1,0,-1	0	none,1,1,-1
R	4	0	
S	5	0	before	list,1,0,1	list,1,0,1	none,2,1,-1	none,2,1,-1	none,1,1,-1	absent,-1,0,-1	0	none,1,1,-1
S	5	1	write	list,1,0,2	list,1,0,2	parsedVarName,2,1,-1	none,3,1,-1	none,1,1,-1	none,3,1,-1	1	none,2,1,-1
S	5	2	body	list,1,0,2	list,1,0,2	parsedVarName,2,1,-1	none,3,1,-1	bytecode,2,1,-1	none,3,1,-1	1	none,1,1,-1
S	5	3	shimmer	string,1,1,-1	string,1,1,-1	parsedVarName,2,1,-1	none,3,1,-1	bytecode,2,1,-1	none,3,1,-1	1	none,1,1,-1
S	5	4	write	string,1,1,-1	string,1,1,-1	parsedVarName,2,1,-1	none,2,1,-1	bytecode,1,1,-1	none,2,1,-1	0	none,3,1,-1
S	5	5	body	string,1,1,-1	string,1,1,-1	parsedVarName,2,1,-1	none,2,1,-1	bytecode,2,1,-1	none,2,1,-1	0	none,1,1,-1
S	5	6	after	string,1,1,-1	string,1,1,-1	parsedVarName,1,1,-1	none,1,1,-1	bytecode,1,1,-1	absent,-1,0,-1	0	list,1,0,1
R	5	0	424f445920424f4459
S	6	0	before	list,1,0,1	none,1,1,-1	none,2,1,-1	absent,-1,0,-1	none,1,1,-1	absent,-1,0,-1	0	none,1,1,-1
S	6	1	after	list,1,1,1	none,1,1,-1	none,2,1,-1	absent,-1,0,-1	none,1,1,-1	absent,-1,0,-1	0	string,1,1,-1
R	6	1	756e6d617463686564206f70656e20627261636520696e206c697374
S	7	0	before	none,1,1,-1	none,1,1,-1	absent,-1,0,-1	absent,-1,0,-1	none,1,1,-1	absent,-1,0,-1	0	none,1,1,-1
S	7	1	after	list,1,1,1	none,1,1,-1	absent,-1,0,-1	absent,-1,0,-1	none,1,1,-1	absent,-1,0,-1	0	string,1,1,-1
R	7	1	666f7265616368207661726c69737420697320656d707479
S	8	0	before	list,1,0,1	list,1,0,1	none,2,1,-1	list,2,0,1	none,1,1,-1	absent,-1,0,-1	0	none,1,1,-1
S	8	1	write	list,1,0,2	list,1,0,2	parsedVarName,2,1,-1	list,3,0,1	none,1,1,-1	list,3,0,1	1	none,2,1,-1
S	8	2	body	list,1,0,2	list,1,0,2	parsedVarName,2,1,-1	list,3,0,1	bytecode,2,1,-1	list,3,0,1	1	none,1,1,-1
S	8	3	shimmer	string,1,1,-1	string,1,1,-1	parsedVarName,2,1,-1	list,3,1,1	bytecode,2,1,-1	list,3,1,1	1	none,1,1,-1
S	8	4	after	string,1,1,-1	string,1,1,-1	parsedVarName,1,1,-1	list,2,1,1	bytecode,1,1,-1	absent,-1,0,-1	0	none,1,1,-1
R	8	0	
S	9	0	before	none,1,1,-1	list,1,0,1	absent,-1,0,-1	none,2,1,-1	none,1,1,-1	absent,-1,0,-1	0	none,1,1,-1
S	9	1	after	list,1,1,1	list,1,1,1	absent,-1,0,-1	none,2,1,-1	none,1,1,-1	absent,-1,0,-1	0	string,1,1,-1
R	9	1	666f7265616368207661726c69737420697320656d707479
```

### tcl9.0

Status: `observed`. Version: tcl9.0; exact launched patchlevel not queried by this probe. Build: library_sha256=2ba08ecf7197e16d99e303c0d95dde29c59f391a22fd02378edc2697567e0e04; binary_sha256=ed4d1f429f4e96e39ff969b03c47a0e43a0d7fa5719c55a2e2a1304a87bdb208; compile_exit=0; exit=0. Compiler/version/configuration fields absent from this original receipt remain unknown.. Channel: Original List object-vector evaluator plus actual first-body character getter; no document source rewrite.. Dialect: C Tcl.

Exact original shimmer stream with the same S/R column layout as the original member-role record. Explicit observer pins are included in refcounts.

```tsv
S	0	0	before	list,1,0,1	list,1,0,1	none,2,1,-1	none,2,1,-1	none,1,1,-1	absent,-1,0,-1	0	none,1,1,-1
S	0	1	write	list,1,0,2	list,1,0,2	parsedVarName,2,1,-1	none,3,1,-1	none,1,1,-1	none,3,1,-1	1	none,2,1,-1
S	0	2	body	list,1,0,2	list,1,0,2	parsedVarName,2,1,-1	none,3,1,-1	bytecode,2,1,-1	none,3,1,-1	1	none,1,1,-1
S	0	3	shimmer	string,1,1,-1	string,1,1,-1	parsedVarName,2,1,-1	none,3,1,-1	bytecode,2,1,-1	none,3,1,-1	1	none,1,1,-1
S	0	4	after	string,1,1,-1	string,1,1,-1	parsedVarName,1,1,-1	none,2,1,-1	bytecode,1,1,-1	absent,-1,0,-1	0	none,1,1,-1
R	0	0	
S	1	0	before	list,1,1,1	list,1,1,1	none,2,1,-1	none,2,1,-1	none,1,1,-1	absent,-1,0,-1	0	none,1,1,-1
S	1	1	write	list,1,1,2	list,1,1,2	parsedVarName,2,1,-1	none,3,1,-1	none,1,1,-1	none,3,1,-1	1	none,2,1,-1
S	1	2	body	list,1,1,2	list,1,1,2	parsedVarName,2,1,-1	none,3,1,-1	bytecode,2,1,-1	none,3,1,-1	1	none,1,1,-1
S	1	3	shimmer	list,1,1,2	list,1,1,2	parsedVarName,2,1,-1	none,3,1,-1	bytecode,2,1,-1	none,3,1,-1	1	none,1,1,-1
S	1	4	after	list,1,1,1	list,1,1,1	parsedVarName,2,1,-1	none,3,1,-1	bytecode,1,1,-1	absent,-1,0,-1	0	none,1,1,-1
R	1	0	
S	2	0	before	list,1,0,1	list,1,0,1	none,2,1,-1	none,2,1,-1	none,1,1,-1	absent,-1,0,-1	0	none,1,1,-1
S	2	1	write	list,1,0,2	list,1,0,2	parsedVarName,2,1,-1	none,3,1,-1	none,1,1,-1	none,3,1,-1	1	none,2,1,-1
S	2	2	body	list,1,0,2	list,1,0,2	parsedVarName,2,1,-1	none,3,1,-1	bytecode,2,1,-1	none,3,1,-1	1	none,1,1,-1
S	2	3	shimmer	string,1,1,-1	string,1,1,-1	parsedVarName,2,1,-1	none,3,1,-1	bytecode,2,1,-1	none,3,1,-1	1	none,1,1,-1
S	2	4	write	string,1,1,-1	string,1,1,-1	parsedVarName,2,1,-1	none,2,1,-1	bytecode,1,1,-1	none,2,1,-1	0	none,2,1,-1
S	2	5	body	string,1,1,-1	string,1,1,-1	parsedVarName,2,1,-1	none,2,1,-1	bytecode,2,1,-1	none,2,1,-1	0	none,1,1,-1
S	2	6	after	string,1,1,-1	string,1,1,-1	parsedVarName,1,1,-1	none,1,1,-1	bytecode,1,1,-1	absent,-1,0,-1	0	none,1,1,-1
R	2	0	
S	3	0	before	list,1,0,1	none,1,1,-1	none,2,1,-1	absent,-1,0,-1	none,1,1,-1	absent,-1,0,-1	0	none,1,1,-1
S	3	1	after	list,1,0,1	list,1,1,1	none,2,1,-1	absent,-1,0,-1	none,1,1,-1	absent,-1,0,-1	0	none,1,1,-1
R	3	0	
S	4	0	before	list,1,0,1	none,1,1,-1	none,2,1,-1	absent,-1,0,-1	none,1,1,-1	absent,-1,0,-1	0	none,1,1,-1
S	4	1	after	list,1,0,1	list,1,1,1	none,2,1,-1	absent,-1,0,-1	none,1,1,-1	absent,-1,0,-1	0	none,1,1,-1
R	4	0	
S	5	0	before	list,1,0,1	list,1,0,1	none,2,1,-1	none,2,1,-1	none,1,1,-1	absent,-1,0,-1	0	none,1,1,-1
S	5	1	write	list,1,0,2	list,1,0,2	parsedVarName,2,1,-1	none,3,1,-1	none,1,1,-1	none,3,1,-1	1	none,2,1,-1
S	5	2	body	list,1,0,2	list,1,0,2	parsedVarName,2,1,-1	none,3,1,-1	bytecode,2,1,-1	none,3,1,-1	1	none,1,1,-1
S	5	3	shimmer	string,1,1,-1	string,1,1,-1	parsedVarName,2,1,-1	none,3,1,-1	bytecode,2,1,-1	none,3,1,-1	1	none,1,1,-1
S	5	4	write	string,1,1,-1	string,1,1,-1	parsedVarName,2,1,-1	none,2,1,-1	bytecode,1,1,-1	none,2,1,-1	0	none,3,1,-1
S	5	5	body	string,1,1,-1	string,1,1,-1	parsedVarName,2,1,-1	none,2,1,-1	bytecode,2,1,-1	none,2,1,-1	0	none,1,1,-1
S	5	6	after	string,1,1,-1	string,1,1,-1	parsedVarName,1,1,-1	none,1,1,-1	bytecode,1,1,-1	absent,-1,0,-1	0	list,1,0,1
R	5	0	424f445920424f4459
S	6	0	before	list,1,0,1	none,1,1,-1	none,2,1,-1	absent,-1,0,-1	none,1,1,-1	absent,-1,0,-1	0	none,1,1,-1
S	6	1	after	list,1,1,1	none,1,1,-1	none,2,1,-1	absent,-1,0,-1	none,1,1,-1	absent,-1,0,-1	0	string,1,1,-1
R	6	1	756e6d617463686564206f70656e20627261636520696e206c697374
S	7	0	before	none,1,1,-1	none,1,1,-1	absent,-1,0,-1	absent,-1,0,-1	none,1,1,-1	absent,-1,0,-1	0	none,1,1,-1
S	7	1	after	list,1,1,1	none,1,1,-1	absent,-1,0,-1	absent,-1,0,-1	none,1,1,-1	absent,-1,0,-1	0	string,1,1,-1
R	7	1	666f7265616368207661726c69737420697320656d707479
S	8	0	before	list,1,0,1	list,1,0,1	none,2,1,-1	list,2,0,1	none,1,1,-1	absent,-1,0,-1	0	none,1,1,-1
S	8	1	write	list,1,0,2	list,1,0,2	parsedVarName,2,1,-1	list,3,0,1	none,1,1,-1	list,3,0,1	1	none,2,1,-1
S	8	2	body	list,1,0,2	list,1,0,2	parsedVarName,2,1,-1	list,3,0,1	bytecode,2,1,-1	list,3,0,1	1	none,1,1,-1
S	8	3	shimmer	string,1,1,-1	string,1,1,-1	parsedVarName,2,1,-1	list,3,1,1	bytecode,2,1,-1	list,3,1,1	1	none,1,1,-1
S	8	4	after	string,1,1,-1	string,1,1,-1	parsedVarName,1,1,-1	list,2,1,1	bytecode,1,1,-1	absent,-1,0,-1	0	none,1,1,-1
R	8	0	
S	9	0	before	none,1,1,-1	list,1,0,1	absent,-1,0,-1	none,2,1,-1	none,1,1,-1	absent,-1,0,-1	0	none,1,1,-1
S	9	1	after	list,1,1,1	list,1,1,1	absent,-1,0,-1	none,2,1,-1	none,1,1,-1	absent,-1,0,-1	0	string,1,1,-1
R	9	1	666f7265616368207661726c69737420697320656d707479
```

### tcl9.1

Status: `observed`. Version: tcl9.1; exact launched patchlevel not queried by this probe. Build: library_sha256=8b2dba836908287f95f26a442bb065601daac300fdeee48b15e64a5c03894e33; binary_sha256=959f318cf5581fd3de9d4c9cc85ae3089d3a63d1e89cc56549ae502b0f80b99e; compile_exit=0; exit=0. Compiler/version/configuration fields absent from this original receipt remain unknown.. Channel: Original List object-vector evaluator plus actual first-body character getter; no document source rewrite.. Dialect: C Tcl.

Exact original shimmer stream with the same S/R column layout as the original member-role record. Explicit observer pins are included in refcounts.

```tsv
S	0	0	before	list,1,0,1	list,1,0,1	none,2,1,-1	none,2,1,-1	none,1,1,-1	absent,-1,0,-1	0	none,1,1,-1
S	0	1	write	list,1,0,2	list,1,0,2	parsedVarName,2,1,-1	none,3,1,-1	none,1,1,-1	none,3,1,-1	1	none,2,1,-1
S	0	2	body	list,1,0,2	list,1,0,2	parsedVarName,2,1,-1	none,3,1,-1	bytecode,2,1,-1	none,3,1,-1	1	none,1,1,-1
S	0	3	shimmer	string,1,1,-1	string,1,1,-1	parsedVarName,2,1,-1	none,3,1,-1	bytecode,2,1,-1	none,3,1,-1	1	none,1,1,-1
S	0	4	after	string,1,1,-1	string,1,1,-1	parsedVarName,1,1,-1	none,2,1,-1	bytecode,1,1,-1	absent,-1,0,-1	0	none,1,1,-1
R	0	0	
S	1	0	before	list,1,1,1	list,1,1,1	none,2,1,-1	none,2,1,-1	none,1,1,-1	absent,-1,0,-1	0	none,1,1,-1
S	1	1	write	list,1,1,2	list,1,1,2	parsedVarName,2,1,-1	none,3,1,-1	none,1,1,-1	none,3,1,-1	1	none,2,1,-1
S	1	2	body	list,1,1,2	list,1,1,2	parsedVarName,2,1,-1	none,3,1,-1	bytecode,2,1,-1	none,3,1,-1	1	none,1,1,-1
S	1	3	shimmer	list,1,1,2	list,1,1,2	parsedVarName,2,1,-1	none,3,1,-1	bytecode,2,1,-1	none,3,1,-1	1	none,1,1,-1
S	1	4	after	list,1,1,1	list,1,1,1	parsedVarName,2,1,-1	none,3,1,-1	bytecode,1,1,-1	absent,-1,0,-1	0	none,1,1,-1
R	1	0	
S	2	0	before	list,1,0,1	list,1,0,1	none,2,1,-1	none,2,1,-1	none,1,1,-1	absent,-1,0,-1	0	none,1,1,-1
S	2	1	write	list,1,0,2	list,1,0,2	parsedVarName,2,1,-1	none,3,1,-1	none,1,1,-1	none,3,1,-1	1	none,2,1,-1
S	2	2	body	list,1,0,2	list,1,0,2	parsedVarName,2,1,-1	none,3,1,-1	bytecode,2,1,-1	none,3,1,-1	1	none,1,1,-1
S	2	3	shimmer	string,1,1,-1	string,1,1,-1	parsedVarName,2,1,-1	none,3,1,-1	bytecode,2,1,-1	none,3,1,-1	1	none,1,1,-1
S	2	4	write	string,1,1,-1	string,1,1,-1	parsedVarName,2,1,-1	none,2,1,-1	bytecode,1,1,-1	none,2,1,-1	0	none,2,1,-1
S	2	5	body	string,1,1,-1	string,1,1,-1	parsedVarName,2,1,-1	none,2,1,-1	bytecode,2,1,-1	none,2,1,-1	0	none,1,1,-1
S	2	6	after	string,1,1,-1	string,1,1,-1	parsedVarName,1,1,-1	none,1,1,-1	bytecode,1,1,-1	absent,-1,0,-1	0	none,1,1,-1
R	2	0	
S	3	0	before	list,1,0,1	none,1,1,-1	none,2,1,-1	absent,-1,0,-1	none,1,1,-1	absent,-1,0,-1	0	none,1,1,-1
S	3	1	after	list,1,0,1	list,1,1,1	none,2,1,-1	absent,-1,0,-1	none,1,1,-1	absent,-1,0,-1	0	none,1,1,-1
R	3	0	
S	4	0	before	list,1,0,1	none,1,1,-1	none,2,1,-1	absent,-1,0,-1	none,1,1,-1	absent,-1,0,-1	0	none,1,1,-1
S	4	1	after	list,1,0,1	list,1,1,1	none,2,1,-1	absent,-1,0,-1	none,1,1,-1	absent,-1,0,-1	0	none,1,1,-1
R	4	0	
S	5	0	before	list,1,0,1	list,1,0,1	none,2,1,-1	none,2,1,-1	none,1,1,-1	absent,-1,0,-1	0	none,1,1,-1
S	5	1	write	list,1,0,2	list,1,0,2	parsedVarName,2,1,-1	none,3,1,-1	none,1,1,-1	none,3,1,-1	1	none,2,1,-1
S	5	2	body	list,1,0,2	list,1,0,2	parsedVarName,2,1,-1	none,3,1,-1	bytecode,2,1,-1	none,3,1,-1	1	none,1,1,-1
S	5	3	shimmer	string,1,1,-1	string,1,1,-1	parsedVarName,2,1,-1	none,3,1,-1	bytecode,2,1,-1	none,3,1,-1	1	none,1,1,-1
S	5	4	write	string,1,1,-1	string,1,1,-1	parsedVarName,2,1,-1	none,2,1,-1	bytecode,1,1,-1	none,2,1,-1	0	none,3,1,-1
S	5	5	body	string,1,1,-1	string,1,1,-1	parsedVarName,2,1,-1	none,2,1,-1	bytecode,2,1,-1	none,2,1,-1	0	none,1,1,-1
S	5	6	after	string,1,1,-1	string,1,1,-1	parsedVarName,1,1,-1	none,1,1,-1	bytecode,1,1,-1	absent,-1,0,-1	0	list,1,0,1
R	5	0	424f445920424f4459
S	6	0	before	list,1,0,1	none,1,1,-1	none,2,1,-1	absent,-1,0,-1	none,1,1,-1	absent,-1,0,-1	0	none,1,1,-1
S	6	1	after	list,1,1,1	none,1,1,-1	none,2,1,-1	absent,-1,0,-1	none,1,1,-1	absent,-1,0,-1	0	string,1,1,-1
R	6	1	756e6d617463686564206f70656e20627261636520696e206c697374
S	7	0	before	none,1,1,-1	none,1,1,-1	absent,-1,0,-1	absent,-1,0,-1	none,1,1,-1	absent,-1,0,-1	0	none,1,1,-1
S	7	1	after	list,1,1,1	none,1,1,-1	absent,-1,0,-1	absent,-1,0,-1	none,1,1,-1	absent,-1,0,-1	0	string,1,1,-1
R	7	1	666f7265616368207661726c69737420697320656d707479
S	8	0	before	list,1,0,1	list,1,0,1	none,2,1,-1	list,2,0,1	none,1,1,-1	absent,-1,0,-1	0	none,1,1,-1
S	8	1	write	list,1,0,2	list,1,0,2	parsedVarName,2,1,-1	list,3,0,1	none,1,1,-1	list,3,0,1	1	none,2,1,-1
S	8	2	body	list,1,0,2	list,1,0,2	parsedVarName,2,1,-1	list,3,0,1	bytecode,2,1,-1	list,3,0,1	1	none,1,1,-1
S	8	3	shimmer	string,1,1,-1	string,1,1,-1	parsedVarName,2,1,-1	list,3,1,1	bytecode,2,1,-1	list,3,1,1	1	none,1,1,-1
S	8	4	after	string,1,1,-1	string,1,1,-1	parsedVarName,1,1,-1	list,2,1,1	bytecode,1,1,-1	absent,-1,0,-1	0	none,1,1,-1
R	8	0	
S	9	0	before	none,1,1,-1	list,1,0,1	absent,-1,0,-1	none,2,1,-1	none,1,1,-1	absent,-1,0,-1	0	none,1,1,-1
S	9	1	after	list,1,1,1	list,1,1,1	absent,-1,0,-1	none,2,1,-1	none,1,1,-1	absent,-1,0,-1	0	string,1,1,-1
R	9	1	666f7265616368207661726c69737420697320656d707479
```

### jim

Status: `observed`. Version: Jim capture; patchlevel, revision and UTF configuration unrecorded. Build: library_sha256=75c74eef0c9d1ff2ff8972e6339678564a3439d6332f66032eb4db79004937ff; binary_sha256=7c1ba177b86ca8486a749f996f78aa1c0a3a55dd06d636bdc82439dc18c6cd40; compile_exit=0; exit=0. Compiler/version/configuration fields absent from this original receipt remain unknown.. Channel: Original List object-vector evaluator plus actual first-body character getter; no document source rewrite.. Dialect: Jim Tcl.

Exact original shimmer stream with the same S/R column layout as the original member-role record. Explicit observer pins are included in refcounts.

```tsv
S	0	0	before	list,1,0,-1	list,1,0,-1	none,2,1,-1	none,2,1,-1	none,1,1,-1	absent,-1,0,-1	0	none,4,1,-1
S	0	1	body	list,2,0,-1	list,2,0,-1	variable,3,1,-1	none,3,1,-1	script,3,1,-1	none,3,1,-1	1	none,8,1,-1
S	0	2	shimmer	string,2,1,-1	string,2,1,-1	variable,2,1,-1	none,2,1,-1	script,3,1,-1	none,2,1,-1	1	none,8,1,-1
S	0	3	after	string,1,1,-1	list,1,1,-1	variable,2,1,-1	none,2,1,-1	script,1,1,-1	absent,-1,0,-1	0	none,8,1,-1
R	0	0	
S	1	0	before	list,1,1,-1	list,1,1,-1	none,2,1,-1	none,2,1,-1	none,1,1,-1	absent,-1,0,-1	0	none,4,1,-1
S	1	1	body	list,2,1,-1	list,2,1,-1	variable,3,1,-1	none,3,1,-1	script,3,1,-1	none,3,1,-1	1	none,8,1,-1
S	1	2	shimmer	string,2,1,-1	string,2,1,-1	variable,2,1,-1	none,2,1,-1	script,3,1,-1	none,2,1,-1	1	none,8,1,-1
S	1	3	after	string,1,1,-1	list,1,1,-1	variable,2,1,-1	none,2,1,-1	script,1,1,-1	absent,-1,0,-1	0	none,8,1,-1
R	1	0	
S	2	0	before	list,1,0,-1	list,1,0,-1	none,2,1,-1	none,2,1,-1	none,1,1,-1	absent,-1,0,-1	0	none,4,1,-1
S	2	1	body	list,2,0,-1	list,2,0,-1	variable,3,1,-1	none,3,1,-1	script,3,1,-1	none,3,1,-1	1	none,8,1,-1
S	2	2	shimmer	string,2,1,-1	string,2,1,-1	variable,2,1,-1	none,2,1,-1	script,3,1,-1	none,2,1,-1	1	none,8,1,-1
S	2	3	body	list,2,1,-1	list,2,1,-1	variable,2,1,-1	none,1,1,-1	script,3,1,-1	source,2,1,-1	0	none,12,1,-1
S	2	4	after	list,1,1,-1	list,1,1,-1	variable,2,1,-1	none,1,1,-1	script,1,1,-1	absent,-1,0,-1	0	none,11,1,-1
R	2	0	
S	3	0	before	list,1,0,-1	list,1,0,-1	none,2,1,-1	absent,-1,0,-1	none,1,1,-1	absent,-1,0,-1	0	none,4,1,-1
S	3	1	after	list,1,0,-1	list,1,0,-1	none,2,1,-1	absent,-1,0,-1	none,1,1,-1	absent,-1,0,-1	0	none,5,1,-1
R	3	0	
S	4	0	before	list,1,0,-1	list,1,0,-1	none,2,1,-1	absent,-1,0,-1	none,1,1,-1	absent,-1,0,-1	0	none,4,1,-1
S	4	1	after	list,1,0,-1	list,1,0,-1	none,2,1,-1	absent,-1,0,-1	none,1,1,-1	absent,-1,0,-1	0	list,1,0,-1
R	4	0	
S	5	0	before	list,1,0,-1	list,1,0,-1	none,2,1,-1	none,2,1,-1	none,1,1,-1	absent,-1,0,-1	0	none,4,1,-1
S	5	1	body	list,2,0,-1	list,2,0,-1	variable,3,1,-1	none,3,1,-1	script,3,1,-1	none,3,1,-1	1	none,7,1,-1
S	5	2	shimmer	string,2,1,-1	string,2,1,-1	variable,2,1,-1	none,2,1,-1	script,3,1,-1	none,2,1,-1	1	none,7,1,-1
S	5	3	body	list,2,1,-1	list,2,1,-1	variable,2,1,-1	none,1,1,-1	script,3,1,-1	source,2,1,-1	0	none,9,1,-1
S	5	4	after	list,1,1,-1	list,1,1,-1	variable,2,1,-1	none,1,1,-1	script,1,1,-1	absent,-1,0,-1	0	list,1,0,-1
R	5	0	424f445920424f4459
S	6	0	before	list,1,0,-1	none,1,1,-1	none,2,1,-1	absent,-1,0,-1	none,1,1,-1	absent,-1,0,-1	0	none,4,1,-1
S	6	1	after	list,2,0,-1	none,2,1,-1	none,2,1,-1	absent,-1,0,-1	none,2,1,-1	absent,-1,0,-1	0	none,1,1,-1
R	6	1	666f7265616368207661726c69737420697320656d707479
S	7	0	before	list,1,0,-1	none,1,1,-1	absent,-1,0,-1	absent,-1,0,-1	none,1,1,-1	absent,-1,0,-1	0	none,4,1,-1
S	7	1	after	list,2,0,-1	none,2,1,-1	absent,-1,0,-1	absent,-1,0,-1	none,2,1,-1	absent,-1,0,-1	0	none,1,1,-1
R	7	1	666f7265616368207661726c69737420697320656d707479
S	8	0	before	list,1,0,-1	list,1,0,-1	none,2,1,-1	list,2,0,-1	none,1,1,-1	absent,-1,0,-1	0	none,4,1,-1
S	8	1	body	list,2,0,-1	list,2,0,-1	variable,3,1,-1	list,3,0,-1	script,3,1,-1	list,3,0,-1	1	none,8,1,-1
S	8	2	shimmer	string,2,1,-1	string,2,1,-1	variable,2,1,-1	list,2,1,-1	script,3,1,-1	list,2,1,-1	1	none,8,1,-1
S	8	3	after	string,1,1,-1	list,1,1,-1	variable,2,1,-1	list,2,1,-1	script,1,1,-1	absent,-1,0,-1	0	none,8,1,-1
R	8	0	
S	9	0	before	list,1,0,-1	list,1,0,-1	absent,-1,0,-1	none,2,1,-1	none,1,1,-1	absent,-1,0,-1	0	none,4,1,-1
S	9	1	after	list,2,0,-1	list,2,0,-1	absent,-1,0,-1	none,2,1,-1	none,2,1,-1	absent,-1,0,-1	0	none,1,1,-1
R	9	1	666f7265616368207661726c69737420697320656d707479
```

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: F5 iRules.

No original observation for this exact question is attached for this provider.

## Exact evidence

- `probe` (input): [rust/tcl-cmd-core/tests/data/native_each_loop/shimmer/probe.c](../../../../rust/tcl-cmd-core/tests/data/native_each_loop/shimmer/probe.c). SHA-256 `b963decd6c72dba84586f3ad06c930a6d362da0d13d1c1bae34ceaa7d90b3907`. Exact additional observer pins, first-body original-root character getters and entered guard.
- `receipt` (provider): [rust/tcl-cmd-core/tests/data/native_each_loop/shimmer/manifest.json](../../../../rust/tcl-cmd-core/tests/data/native_each_loop/shimmer/manifest.json). SHA-256 `b71dc5bb81e1570ebe54fd6b383e5d530d1e3e49e5e7a3301bdd94baa218b868`. Six actual217-window original streams/source/build/process digests.
- `rows-tcl8.4` (observation): [rust/tcl-cmd-core/tests/data/native_each_loop/shimmer/tcl8.4.txt](../../../../rust/tcl-cmd-core/tests/data/native_each_loop/shimmer/tcl8.4.txt). SHA-256 `350465844580ea21c9691f7c23f004d9aa8dfecff1ead93932a5cec64b64280c`. Entire distinct shimmer stream, including nonentered cases and guest errors.
- `rows-tcl8.5` (observation): [rust/tcl-cmd-core/tests/data/native_each_loop/shimmer/tcl8.5.txt](../../../../rust/tcl-cmd-core/tests/data/native_each_loop/shimmer/tcl8.5.txt). SHA-256 `c72abf5c47a426b6d0d11e46fa95683b68fded0733ef63e6fb7828ec08400619`. Entire distinct shimmer stream, including nonentered cases and guest errors.
- `rows-tcl8.6` (observation): [rust/tcl-cmd-core/tests/data/native_each_loop/shimmer/tcl8.6.txt](../../../../rust/tcl-cmd-core/tests/data/native_each_loop/shimmer/tcl8.6.txt). SHA-256 `c70bca73a4aee3b90beb995c9d66e3d80cf6d85e37e6967ad69bc039a74ea685`. Entire distinct shimmer stream, including nonentered cases and guest errors.
- `rows-tcl9.0` (observation): [rust/tcl-cmd-core/tests/data/native_each_loop/shimmer/tcl9.0.txt](../../../../rust/tcl-cmd-core/tests/data/native_each_loop/shimmer/tcl9.0.txt). SHA-256 `c70bca73a4aee3b90beb995c9d66e3d80cf6d85e37e6967ad69bc039a74ea685`. Entire distinct shimmer stream, including nonentered cases and guest errors.
- `rows-tcl9.1` (observation): [rust/tcl-cmd-core/tests/data/native_each_loop/shimmer/tcl9.1.txt](../../../../rust/tcl-cmd-core/tests/data/native_each_loop/shimmer/tcl9.1.txt). SHA-256 `c70bca73a4aee3b90beb995c9d66e3d80cf6d85e37e6967ad69bc039a74ea685`. Entire distinct shimmer stream, including nonentered cases and guest errors.
- `rows-jim` (observation): [rust/tcl-cmd-core/tests/data/native_each_loop/shimmer/jim.txt](../../../../rust/tcl-cmd-core/tests/data/native_each_loop/shimmer/jim.txt). SHA-256 `bfc7ec2f9f9b78d4a6d9a7c65ea66619541bfddb9891c8f217c1238d62602bf2`. Entire distinct shimmer stream, including nonentered cases and guest errors.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-vm/src/exec.rs](../../../../rust/tcl-vm/src/exec.rs), `refresh_each_loop_group`: Reissues current group views through the selected live/refetch protocol rather than caching a displayed list.
- [rust/tcl-vm/src/cmd_control/native_each_loop_tests.rs](../../../../rust/tcl-vm/src/cmd_control/native_each_loop_tests.rs), `cmd_control::native_each_loop_tests::generic_each_loops_match_all_406_original_native_physical_windows` (linked): Includes all217 distinct shimmer snapshots and120 combined completions; real external pins and entered first-body getter are recreated separately.

A named test is a coverage binding, not a claim that it executed.

## Replay

No new native run or Rust execution result is claimed. Replay must compile the retained exact probe against independently identified release headers/libraries and preserve separate compile/process status and raw streams before comparing the attached original hashes. Original absolute compile paths are capture metadata, not a portable executable command. No evaluator/source/parser or native allocation authority may be obtained by matching display text. Probe code is input/observer code; absent native source excerpts supply no implementation explanation.
