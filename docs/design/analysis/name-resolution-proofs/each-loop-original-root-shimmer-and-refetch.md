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

Status: `observed`. Channel: Original List object-vector evaluator plus actual first-body character getter; no document source rewrite. Version: tcl8.4 tcl8.4.

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

Status: `observed`. Channel: Original List object-vector evaluator plus actual first-body character getter; no document source rewrite. Version: tcl8.5 tcl8.5.

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

### tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Original List object-vector evaluator plus actual first-body character getter; no document source rewrite. Version: tcl8.6 tcl8.6; tcl9.0 tcl9.0; tcl9.1 tcl9.1.

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

Status: `observed`. Channel: Original List object-vector evaluator plus actual first-body character getter; no document source rewrite. Version: jim Jim capture.

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

Status: `not-tested`.

No original observation for this exact question is attached for this provider.

## Consumer bindings

- [rust/tcl-vm/src/exec.rs](../../../../rust/tcl-vm/src/exec.rs), `refresh_each_loop_group`: Reissues current group views through the selected live/refetch protocol rather than caching a displayed list.
- [rust/tcl-vm/src/cmd_control/native_each_loop_tests.rs](../../../../rust/tcl-vm/src/cmd_control/native_each_loop_tests.rs), `cmd_control::native_each_loop_tests::generic_each_loops_match_all_406_original_native_physical_windows` (linked): Includes all217 distinct shimmer snapshots and120 combined completions; real external pins and entered first-body getter are recreated separately.

## Evidence

- [rust/tcl-cmd-core/tests/data/native_each_loop/shimmer/probe.c](../../../../rust/tcl-cmd-core/tests/data/native_each_loop/shimmer/probe.c)
- [rust/tcl-cmd-core/tests/data/native_each_loop/shimmer/manifest.json](../../../../rust/tcl-cmd-core/tests/data/native_each_loop/shimmer/manifest.json)
- [rust/tcl-cmd-core/tests/data/native_each_loop/shimmer/tcl8.4.txt](../../../../rust/tcl-cmd-core/tests/data/native_each_loop/shimmer/tcl8.4.txt)
- [rust/tcl-cmd-core/tests/data/native_each_loop/shimmer/tcl8.5.txt](../../../../rust/tcl-cmd-core/tests/data/native_each_loop/shimmer/tcl8.5.txt)
- [rust/tcl-cmd-core/tests/data/native_each_loop/shimmer/tcl8.6.txt](../../../../rust/tcl-cmd-core/tests/data/native_each_loop/shimmer/tcl8.6.txt)
- [rust/tcl-cmd-core/tests/data/native_each_loop/shimmer/tcl9.0.txt](../../../../rust/tcl-cmd-core/tests/data/native_each_loop/shimmer/tcl9.0.txt)
- [rust/tcl-cmd-core/tests/data/native_each_loop/shimmer/tcl9.1.txt](../../../../rust/tcl-cmd-core/tests/data/native_each_loop/shimmer/tcl9.1.txt)
- [rust/tcl-cmd-core/tests/data/native_each_loop/shimmer/jim.txt](../../../../rust/tcl-cmd-core/tests/data/native_each_loop/shimmer/jim.txt)
