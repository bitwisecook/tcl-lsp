# naming.variable.original-increment-representation-frontiers

Kind: `native-observation`

## Problem statement

Incrementing a shared original variable value can convert the amount/current before or after deciding to duplicate it. A correct integer result alone cannot establish the original object mutation and reference frontier.

## Question

What original current/amount type, residency, reference and identity windows do the fifteen shapes with two sharing modes retain?

## Conclusion

The 180 original observations keep sharing and conversion order separate. Tcl 8.4 uses its Long amount getter and duplication-before-current-conversion; Jim uses its independent safe-expression amount getter and current conversion before sharing. Modern C follows its distinct full-number path. Trace/return-options effects were not measured.

## Scope

Only the original retained programs, capture windows and provider labels in the attached artifacts are covered. Version labels are original receipt metadata, not a fresh patchlevel query. Missing command/API doors and aborted processes stay distinct from successful guest completion. Source files/probes explain the observer protocol; no pinned engine implementation excerpt or executed Rust assertion is claimed. Different attempts and sources are retained separately, never treated as byte-identical duplicates.

## Provider answers

### tcl8.4

Status: `observed`. Channel: Compiled retained C probe; original evaluator/API calls: Jim_EvalObjVector, Tcl_EvalObjv. Exact lengths, flags and input constructors remain in the source. Version: tcl8.4 8.4.20.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`manifest.json` (`file-6-row-7`):

```json
{
  "version": "8.4.20",
  "compile_exit": 0,
  "exit": 0,
  "stderr": "",
  "rows": 30
}
```

`8.4.20.tsv` (`file-0`):

```json
{
  "original_table": "0\t0\tbefore-current=none,1,1\tbefore-amount=int,0,1,1\tafter-current=int,0,2,5\tafter-amount=int,0,1,1\tidentity=1\tcode=0\tresult=35\n0\t1\tbefore-current=none,1,2\tbefore-amount=int,0,1,1\tafter-current=int,0,2,5\tafter-amount=int,0,1,1\toriginal-current=none,1,1\tidentity=0\tcode=0\tresult=35\n1\t0\tbefore-current=int,0,1,4\tbefore-amount=int,0,1,1\tafter-current=int,0,2,5\tafter-amount=int,0,1,1\tidentity=1\tcode=0\tresult=35\n1\t1\tbefore-current=int,0,2,4\tbefore-amount=int,0,1,1\tafter-current=int,0,2,5\tafter-amount=int,0,1,1\toriginal-current=int,0,1,4\tidentity=0\tcode=0\tresult=35\n2\t0\tbefore-current=wideInt,0,1,4\tbefore-amount=int,0,1,1\tafter-current=wideInt,0,2,5\tafter-amount=int,0,1,1\tidentity=1\tcode=0\tresult=35\n2\t1\tbefore-current=wideInt,0,2,4\tbefore-amount=int,0,1,1\tafter-current=wideInt,0,2,5\tafter-amount=int,0,1,1\toriginal-current=wideInt,0,1,4\tidentity=0\tcode=0\tresult=35\n3\t0\tbefore-current=double,0,1\tbefore-amount=int,0,1,1\tafter-current=double,1,1\tafter-amount=int,1,1,1\tidentity=1\tcode=1\tresult=657870656374656420696e74656765722062757420676f742022342e3022\n3\t1\tbefore-current=double,0,2\tbefore-amount=int,0,1,1\tafter-current=double,0,2\tafter-amount=int,1,1,1\toriginal-current=double,0,2\tidentity=1\tcode=1\tresult=657870656374656420696e74656765722062757420676f742022342e3022\n4\t0\tbefore-current=none,1,1\tbefore-amount=int,0,1,1\tafter-current=none,1,1\tafter-amount=int,1,1,1\tidentity=1\tcode=1\tresult=657870656374656420696e74656765722062757420676f742022312e3522\n4\t1\tbefore-current=none,1,2\tbefore-amount=int,0,1,1\tafter-current=none,1,2\tafter-amount=int,1,1,1\toriginal-current=none,1,2\tidentity=1\tcode=1\tresult=657870656374656420696e74656765722062757420676f742022312e3522\n5\t0\tbefore-current=wideInt,0,1,9223372036854775807\tbefore-amount=int,0,1,1\tafter-current=wideInt,0,2,-9223372036854775808\tafter-amount=int,0,1,1\tidentity=1\tcode=0\tresult=2d38\n5\t1\tbefore-current=wideInt,0,2,9223372036854775807\tbefore-amount=int,0,1,1\tafter-current=wideInt,0,2,-9223372036854775808\tafter-amount=int,0,1,1\toriginal-current=wideInt,0,1,9223372036854775807\tidentity=0\tcode=0\tresult=2d38\n6\t0\tbefore-current=none,1,1\tbefore-amount=double,0,1\tafter-current=none,1,1\tafter-amount=double,1,1\tidentity=1\tcode=1\tresult=657870656374656420696e74656765722062757420676f742022312e3022\n6\t1\tbefore-current=none,1,2\tbefore-amount=double,0,1\tafter-current=none,1,2\tafter-amount=double,1,1\toriginal-current=none,1,2\tidentity=1\tcode=1\tresult=657870656374656420696e74656765722062757420676f742022312e3022\n7\t0\tbefore-current=none,1,1\tbefore-amount=double,0,1\tafter-current=none,1,1\tafter-amount=double,1,1\tidentity=1\tcode=1\tresult=657870656374656420696e74656765722062757420676f742022312e3522\n7\t1\tbefore-current=none,1,2\tbefore-amount=double,0,1\tafter-current=none,1,2\tafter-amount=double,1,1\toriginal-current=none,1,2\tidentity=1\tcode=1\tresult=657870656374656420696e74656765722062757420676f742022312e3522\n8\t0\tbefore-current=none,1,1\tbefore-amount=none,1,1\tafter-current=none,1,1\tafter-amount=none,1,1\tidentity=1\tcode=1\tresult=657870656374656420696e74656765722062757420676f74202231202b203222\n8\t1\tbefore-current=none,1,2\tbefore-amount=none,1,1\tafter-current=none,1,2\tafter-amount=none,1,1\toriginal-current=none,1,2\tidentity=1\tcode=1\tresult=657870656374656420696e74656765722062757420676f74202231202b203222\n9\t0\tbefore-current=none,1,1\tbefore-amount=none,1,1\tafter-current=none,1,1\tafter-amount=none,1,1\tidentity=1\tcode=1\tresult=657870656374656420696e74656765722062757420676f74202242414422\n9\t1\tbefore-current=none,1,2\tbefore-amount=none,1,1\tafter-current=none,1,2\tafter-amount=none,1,1\toriginal-current=none,1,2\tidentity=1\tcode=1\tresult=657870656374656420696e74656765722062757420676f74202242414422\n10\t0\tbefore-current=none,1,1\tbefore-amount=none,1,1\tafter-current=none,1,1\tafter-amount=none,1,1\tidentity=1\tcode=1\tresult=657870656374656420696e74656765722062757420676f74202242414422\n10\t1\tbefore-current=none,1,2\tbefore-amount=none,1,1\tafter-current=none,1,2\tafter-amount=none,1,1\toriginal-current=none,1,2\tidentity=1\tcode=1\tresult=657870656374656420696e74656765722062757420676f74202242414422\n11\t0\tbefore-current=missing,0,0\tbefore-amount=none,1,1\tafter-current=missing,0,0\tafter-amount=none,1,1\tidentity=0\tcode=1\tresult=657870656374656420696e74656765722062757420676f74202242414422\n11\t1\tbefore-current=missing,0,0\tbefore-amount=none,1,1\tafter-current=missing,0,0\tafter-amount=none,1,1\toriginal-current=missing,0,0\tidentity=0\tcode=1\tresult=657870656374656420696e74656765722062757420676f74202242414422\n12\t0\tbefore-current=missing,0,0\tbefore-amount=int,0,1,1\tafter-current=missing,0,0\tafter-amount=int,1,1,1\tidentity=0\tcode=1\tresult=63616e27742072656164202278223a206e6f2073756368207661726961626c65\n12\t1\tbefore-current=missing,0,0\tbefore-amount=int,0,1,1\tafter-current=missing,0,0\tafter-amount=int,1,1,1\toriginal-current=missing,0,0\tidentity=0\tcode=1\tresult=63616e27742072656164202278223a206e6f2073756368207661726961626c65\n13\t0\tbefore-current=none,1,1\tbefore-amount=none,1,1\tafter-current=none,1,1\tafter-amount=none,1,1\tidentity=1\tcode=1\tresult=657870656374656420696e74656765722062757420676f7420223122\n13\t1\tbefore-current=none,1,2\tbefore-amount=none,1,1\tafter-current=none,1,2\tafter-amount=none,1,1\toriginal-current=none,1,2\tidentity=1\tcode=1\tresult=657870656374656420696e74656765722062757420676f7420223122\n14\t0\tbefore-current=none,1,1\tbefore-amount=int,0,1,1\tafter-current=none,1,1\tafter-amount=int,1,1,1\tidentity=1\tcode=1\tresult=657870656374656420696e74656765722062757420676f74202242414422\n14\t1\tbefore-current=none,1,2\tbefore-amount=int,0,1,1\tafter-current=none,1,2\tafter-amount=int,1,1,1\toriginal-current=none,1,2\tidentity=1\tcode=1\tresult=657870656374656420696e74656765722062757420676f74202242414422\n"
}
```

### tcl8.5

Status: `observed`. Channel: Compiled retained C probe; original evaluator/API calls: Jim_EvalObjVector, Tcl_EvalObjv. Exact lengths, flags and input constructors remain in the source. Version: tcl8.5 8.5.19.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`manifest.json` (`file-6-row-8`):

```json
{
  "version": "8.5.19",
  "compile_exit": 0,
  "exit": 0,
  "stderr": "",
  "rows": 30
}
```

`8.5.19.tsv` (`file-1`):

```json
{
  "original_table": "0\t0\tbefore-current=none,1,1\tbefore-amount=int,0,1,1\tafter-current=int,0,2,5\tafter-amount=int,0,1,1\tidentity=1\tcode=0\tresult=35\n0\t1\tbefore-current=none,1,2\tbefore-amount=int,0,1,1\tafter-current=int,0,2,5\tafter-amount=int,0,1,1\toriginal-current=none,1,1\tidentity=0\tcode=0\tresult=35\n1\t0\tbefore-current=int,0,1,4\tbefore-amount=int,0,1,1\tafter-current=int,0,2,5\tafter-amount=int,0,1,1\tidentity=1\tcode=0\tresult=35\n1\t1\tbefore-current=int,0,2,4\tbefore-amount=int,0,1,1\tafter-current=int,0,2,5\tafter-amount=int,0,1,1\toriginal-current=int,0,1,4\tidentity=0\tcode=0\tresult=35\n2\t0\tbefore-current=int,0,1,4\tbefore-amount=int,0,1,1\tafter-current=int,0,2,5\tafter-amount=int,0,1,1\tidentity=1\tcode=0\tresult=35\n2\t1\tbefore-current=int,0,2,4\tbefore-amount=int,0,1,1\tafter-current=int,0,2,5\tafter-amount=int,0,1,1\toriginal-current=int,0,1,4\tidentity=0\tcode=0\tresult=35\n3\t0\tbefore-current=double,0,1\tbefore-amount=int,0,1,1\tafter-current=double,1,1\tafter-amount=int,1,1,1\tidentity=1\tcode=1\tresult=657870656374656420696e74656765722062757420676f742022342e3022\n3\t1\tbefore-current=double,0,2\tbefore-amount=int,0,1,1\tafter-current=double,0,2\tafter-amount=int,1,1,1\toriginal-current=double,0,2\tidentity=1\tcode=1\tresult=657870656374656420696e74656765722062757420676f742022342e3022\n4\t0\tbefore-current=none,1,1\tbefore-amount=int,0,1,1\tafter-current=double,1,1\tafter-amount=int,1,1,1\tidentity=1\tcode=1\tresult=657870656374656420696e74656765722062757420676f742022312e3522\n4\t1\tbefore-current=none,1,2\tbefore-amount=int,0,1,1\tafter-current=none,1,2\tafter-amount=int,1,1,1\toriginal-current=none,1,2\tidentity=1\tcode=1\tresult=657870656374656420696e74656765722062757420676f742022312e3522\n5\t0\tbefore-current=int,0,1,9223372036854775807\tbefore-amount=int,0,1,1\tafter-current=bignum,0,2\tafter-amount=int,0,1,1\tidentity=1\tcode=0\tresult=39323233333732303336383534373735383038\n5\t1\tbefore-current=int,0,2,9223372036854775807\tbefore-amount=int,0,1,1\tafter-current=bignum,0,2\tafter-amount=int,0,1,1\toriginal-current=int,0,1,9223372036854775807\tidentity=0\tcode=0\tresult=39323233333732303336383534373735383038\n6\t0\tbefore-current=none,1,1\tbefore-amount=double,0,1\tafter-current=int,1,1,4\tafter-amount=double,1,1\tidentity=1\tcode=1\tresult=657870656374656420696e74656765722062757420676f742022312e3022\n6\t1\tbefore-current=none,1,2\tbefore-amount=double,0,1\tafter-current=none,1,2\tafter-amount=double,1,1\toriginal-current=none,1,2\tidentity=1\tcode=1\tresult=657870656374656420696e74656765722062757420676f742022312e3022\n7\t0\tbefore-current=none,1,1\tbefore-amount=double,0,1\tafter-current=int,1,1,4\tafter-amount=double,1,1\tidentity=1\tcode=1\tresult=657870656374656420696e74656765722062757420676f742022312e3522\n7\t1\tbefore-current=none,1,2\tbefore-amount=double,0,1\tafter-current=none,1,2\tafter-amount=double,1,1\toriginal-current=none,1,2\tidentity=1\tcode=1\tresult=657870656374656420696e74656765722062757420676f742022312e3522\n8\t0\tbefore-current=none,1,1\tbefore-amount=none,1,1\tafter-current=int,1,1,4\tafter-amount=none,1,1\tidentity=1\tcode=1\tresult=657870656374656420696e74656765722062757420676f74202231202b203222\n8\t1\tbefore-current=none,1,2\tbefore-amount=none,1,1\tafter-current=none,1,2\tafter-amount=none,1,1\toriginal-current=none,1,2\tidentity=1\tcode=1\tresult=657870656374656420696e74656765722062757420676f74202231202b203222\n9\t0\tbefore-current=none,1,1\tbefore-amount=none,1,1\tafter-current=int,1,1,4\tafter-amount=none,1,1\tidentity=1\tcode=1\tresult=657870656374656420696e74656765722062757420676f74202242414422\n9\t1\tbefore-current=none,1,2\tbefore-amount=none,1,1\tafter-current=none,1,2\tafter-amount=none,1,1\toriginal-current=none,1,2\tidentity=1\tcode=1\tresult=657870656374656420696e74656765722062757420676f74202242414422\n10\t0\tbefore-current=none,1,1\tbefore-amount=none,1,1\tafter-current=none,1,1\tafter-amount=none,1,1\tidentity=1\tcode=1\tresult=657870656374656420696e74656765722062757420676f74202242414422\n10\t1\tbefore-current=none,1,2\tbefore-amount=none,1,1\tafter-current=none,1,2\tafter-amount=none,1,1\toriginal-current=none,1,2\tidentity=1\tcode=1\tresult=657870656374656420696e74656765722062757420676f74202242414422\n11\t0\tbefore-current=missing,0,0\tbefore-amount=none,1,1\tafter-current=missing,0,0\tafter-amount=none,1,1\tidentity=0\tcode=1\tresult=657870656374656420696e74656765722062757420676f74202242414422\n11\t1\tbefore-current=missing,0,0\tbefore-amount=none,1,1\tafter-current=missing,0,0\tafter-amount=none,1,1\toriginal-current=missing,0,0\tidentity=0\tcode=1\tresult=657870656374656420696e74656765722062757420676f74202242414422\n12\t0\tbefore-current=missing,0,0\tbefore-amount=int,0,1,1\tafter-current=int,0,2,1\tafter-amount=int,0,1,1\tidentity=0\tcode=0\tresult=31\n12\t1\tbefore-current=missing,0,0\tbefore-amount=int,0,1,1\tafter-current=int,0,2,1\tafter-amount=int,0,1,1\toriginal-current=missing,0,0\tidentity=0\tcode=0\tresult=31\n13\t0\tbefore-current=none,1,1\tbefore-amount=none,1,1\tafter-current=int,0,2,5\tafter-amount=int,1,1,1\tidentity=1\tcode=0\tresult=35\n13\t1\tbefore-current=none,1,2\tbefore-amount=none,1,1\tafter-current=int,0,2,5\tafter-amount=int,1,1,1\toriginal-current=none,1,1\tidentity=0\tcode=0\tresult=35\n14\t0\tbefore-current=none,1,1\tbefore-amount=int,0,1,1\tafter-current=none,1,1\tafter-amount=int,1,1,1\tidentity=1\tcode=1\tresult=657870656374656420696e74656765722062757420676f74202242414422\n14\t1\tbefore-current=none,1,2\tbefore-amount=int,0,1,1\tafter-current=none,1,2\tafter-amount=int,1,1,1\toriginal-current=none,1,2\tidentity=1\tcode=1\tresult=657870656374656420696e74656765722062757420676f74202242414422\n"
}
```

### tcl8.6

Status: `observed`. Channel: Compiled retained C probe; original evaluator/API calls: Jim_EvalObjVector, Tcl_EvalObjv. Exact lengths, flags and input constructors remain in the source. Version: tcl8.6 8.6.18.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`manifest.json` (`file-6-row-9`):

```json
{
  "version": "8.6.18",
  "compile_exit": 0,
  "exit": 0,
  "stderr": "",
  "rows": 30
}
```

`8.6.18.tsv` (`file-2`):

```json
{
  "original_table": "0\t0\tbefore-current=none,1,1\tbefore-amount=int,0,1,1\tafter-current=int,0,2,5\tafter-amount=int,0,1,1\tidentity=1\tcode=0\tresult=35\n0\t1\tbefore-current=none,1,2\tbefore-amount=int,0,1,1\tafter-current=int,0,2,5\tafter-amount=int,0,1,1\toriginal-current=none,1,1\tidentity=0\tcode=0\tresult=35\n1\t0\tbefore-current=int,0,1,4\tbefore-amount=int,0,1,1\tafter-current=int,0,2,5\tafter-amount=int,0,1,1\tidentity=1\tcode=0\tresult=35\n1\t1\tbefore-current=int,0,2,4\tbefore-amount=int,0,1,1\tafter-current=int,0,2,5\tafter-amount=int,0,1,1\toriginal-current=int,0,1,4\tidentity=0\tcode=0\tresult=35\n2\t0\tbefore-current=int,0,1,4\tbefore-amount=int,0,1,1\tafter-current=int,0,2,5\tafter-amount=int,0,1,1\tidentity=1\tcode=0\tresult=35\n2\t1\tbefore-current=int,0,2,4\tbefore-amount=int,0,1,1\tafter-current=int,0,2,5\tafter-amount=int,0,1,1\toriginal-current=int,0,1,4\tidentity=0\tcode=0\tresult=35\n3\t0\tbefore-current=double,0,1\tbefore-amount=int,0,1,1\tafter-current=double,1,1\tafter-amount=int,1,1,1\tidentity=1\tcode=1\tresult=657870656374656420696e74656765722062757420676f742022342e3022\n3\t1\tbefore-current=double,0,2\tbefore-amount=int,0,1,1\tafter-current=double,0,2\tafter-amount=int,1,1,1\toriginal-current=double,0,2\tidentity=1\tcode=1\tresult=657870656374656420696e74656765722062757420676f742022342e3022\n4\t0\tbefore-current=none,1,1\tbefore-amount=int,0,1,1\tafter-current=double,1,1\tafter-amount=int,1,1,1\tidentity=1\tcode=1\tresult=657870656374656420696e74656765722062757420676f742022312e3522\n4\t1\tbefore-current=none,1,2\tbefore-amount=int,0,1,1\tafter-current=none,1,2\tafter-amount=int,1,1,1\toriginal-current=none,1,2\tidentity=1\tcode=1\tresult=657870656374656420696e74656765722062757420676f742022312e3522\n5\t0\tbefore-current=int,0,1,9223372036854775807\tbefore-amount=int,0,1,1\tafter-current=bignum,0,2\tafter-amount=int,0,1,1\tidentity=1\tcode=0\tresult=39323233333732303336383534373735383038\n5\t1\tbefore-current=int,0,2,9223372036854775807\tbefore-amount=int,0,1,1\tafter-current=bignum,0,2\tafter-amount=int,0,1,1\toriginal-current=int,0,1,9223372036854775807\tidentity=0\tcode=0\tresult=39323233333732303336383534373735383038\n6\t0\tbefore-current=none,1,1\tbefore-amount=double,0,1\tafter-current=int,1,1,4\tafter-amount=double,1,1\tidentity=1\tcode=1\tresult=657870656374656420696e74656765722062757420676f742022312e3022\n6\t1\tbefore-current=none,1,2\tbefore-amount=double,0,1\tafter-current=none,1,2\tafter-amount=double,1,1\toriginal-current=none,1,2\tidentity=1\tcode=1\tresult=657870656374656420696e74656765722062757420676f742022312e3022\n7\t0\tbefore-current=none,1,1\tbefore-amount=double,0,1\tafter-current=int,1,1,4\tafter-amount=double,1,1\tidentity=1\tcode=1\tresult=657870656374656420696e74656765722062757420676f742022312e3522\n7\t1\tbefore-current=none,1,2\tbefore-amount=double,0,1\tafter-current=none,1,2\tafter-amount=double,1,1\toriginal-current=none,1,2\tidentity=1\tcode=1\tresult=657870656374656420696e74656765722062757420676f742022312e3522\n8\t0\tbefore-current=none,1,1\tbefore-amount=none,1,1\tafter-current=int,1,1,4\tafter-amount=none,1,1\tidentity=1\tcode=1\tresult=657870656374656420696e74656765722062757420676f74202231202b203222\n8\t1\tbefore-current=none,1,2\tbefore-amount=none,1,1\tafter-current=none,1,2\tafter-amount=none,1,1\toriginal-current=none,1,2\tidentity=1\tcode=1\tresult=657870656374656420696e74656765722062757420676f74202231202b203222\n9\t0\tbefore-current=none,1,1\tbefore-amount=none,1,1\tafter-current=int,1,1,4\tafter-amount=none,1,1\tidentity=1\tcode=1\tresult=657870656374656420696e74656765722062757420676f74202242414422\n9\t1\tbefore-current=none,1,2\tbefore-amount=none,1,1\tafter-current=none,1,2\tafter-amount=none,1,1\toriginal-current=none,1,2\tidentity=1\tcode=1\tresult=657870656374656420696e74656765722062757420676f74202242414422\n10\t0\tbefore-current=none,1,1\tbefore-amount=none,1,1\tafter-current=none,1,1\tafter-amount=none,1,1\tidentity=1\tcode=1\tresult=657870656374656420696e74656765722062757420676f74202242414422\n10\t1\tbefore-current=none,1,2\tbefore-amount=none,1,1\tafter-current=none,1,2\tafter-amount=none,1,1\toriginal-current=none,1,2\tidentity=1\tcode=1\tresult=657870656374656420696e74656765722062757420676f74202242414422\n11\t0\tbefore-current=missing,0,0\tbefore-amount=none,1,1\tafter-current=missing,0,0\tafter-amount=none,1,1\tidentity=0\tcode=1\tresult=657870656374656420696e74656765722062757420676f74202242414422\n11\t1\tbefore-current=missing,0,0\tbefore-amount=none,1,1\tafter-current=missing,0,0\tafter-amount=none,1,1\toriginal-current=missing,0,0\tidentity=0\tcode=1\tresult=657870656374656420696e74656765722062757420676f74202242414422\n12\t0\tbefore-current=missing,0,0\tbefore-amount=int,0,1,1\tafter-current=int,0,2,1\tafter-amount=int,0,1,1\tidentity=0\tcode=0\tresult=31\n12\t1\tbefore-current=missing,0,0\tbefore-amount=int,0,1,1\tafter-current=int,0,2,1\tafter-amount=int,0,1,1\toriginal-current=missing,0,0\tidentity=0\tcode=0\tresult=31\n13\t0\tbefore-current=none,1,1\tbefore-amount=none,1,1\tafter-current=int,0,2,5\tafter-amount=int,1,1,1\tidentity=1\tcode=0\tresult=35\n13\t1\tbefore-current=none,1,2\tbefore-amount=none,1,1\tafter-current=int,0,2,5\tafter-amount=int,1,1,1\toriginal-current=none,1,1\tidentity=0\tcode=0\tresult=35\n14\t0\tbefore-current=none,1,1\tbefore-amount=int,0,1,1\tafter-current=none,1,1\tafter-amount=int,1,1,1\tidentity=1\tcode=1\tresult=657870656374656420696e74656765722062757420676f74202242414422\n14\t1\tbefore-current=none,1,2\tbefore-amount=int,0,1,1\tafter-current=none,1,2\tafter-amount=int,1,1,1\toriginal-current=none,1,2\tidentity=1\tcode=1\tresult=657870656374656420696e74656765722062757420676f74202242414422\n"
}
```

### tcl9.0

Status: `observed`. Channel: Compiled retained C probe; original evaluator/API calls: Jim_EvalObjVector, Tcl_EvalObjv. Exact lengths, flags and input constructors remain in the source. Version: tcl9.0 9.0.4.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`manifest.json` (`file-6-row-10`):

```json
{
  "version": "9.0.4",
  "compile_exit": 0,
  "exit": 0,
  "stderr": "",
  "rows": 30
}
```

`9.0.4.tsv` (`file-3`):

```json
{
  "original_table": "0\t0\tbefore-current=none,1,1\tbefore-amount=int,0,1,1\tafter-current=int,0,2,5\tafter-amount=int,0,1,1\tidentity=1\tcode=0\tresult=35\n0\t1\tbefore-current=none,1,2\tbefore-amount=int,0,1,1\tafter-current=int,0,2,5\tafter-amount=int,0,1,1\toriginal-current=none,1,1\tidentity=0\tcode=0\tresult=35\n1\t0\tbefore-current=int,0,1,4\tbefore-amount=int,0,1,1\tafter-current=int,0,2,5\tafter-amount=int,0,1,1\tidentity=1\tcode=0\tresult=35\n1\t1\tbefore-current=int,0,2,4\tbefore-amount=int,0,1,1\tafter-current=int,0,2,5\tafter-amount=int,0,1,1\toriginal-current=int,0,1,4\tidentity=0\tcode=0\tresult=35\n2\t0\tbefore-current=int,0,1,4\tbefore-amount=int,0,1,1\tafter-current=int,0,2,5\tafter-amount=int,0,1,1\tidentity=1\tcode=0\tresult=35\n2\t1\tbefore-current=int,0,2,4\tbefore-amount=int,0,1,1\tafter-current=int,0,2,5\tafter-amount=int,0,1,1\toriginal-current=int,0,1,4\tidentity=0\tcode=0\tresult=35\n3\t0\tbefore-current=double,0,1\tbefore-amount=int,0,1,1\tafter-current=double,1,1\tafter-amount=int,1,1,1\tidentity=1\tcode=1\tresult=657870656374656420696e74656765722062757420676f742022342e3022\n3\t1\tbefore-current=double,0,2\tbefore-amount=int,0,1,1\tafter-current=double,0,2\tafter-amount=int,1,1,1\toriginal-current=double,0,2\tidentity=1\tcode=1\tresult=657870656374656420696e74656765722062757420676f742022342e3022\n4\t0\tbefore-current=none,1,1\tbefore-amount=int,0,1,1\tafter-current=double,1,1\tafter-amount=int,1,1,1\tidentity=1\tcode=1\tresult=657870656374656420696e74656765722062757420676f742022312e3522\n4\t1\tbefore-current=none,1,2\tbefore-amount=int,0,1,1\tafter-current=none,1,2\tafter-amount=int,1,1,1\toriginal-current=none,1,2\tidentity=1\tcode=1\tresult=657870656374656420696e74656765722062757420676f742022312e3522\n5\t0\tbefore-current=int,0,1,9223372036854775807\tbefore-amount=int,0,1,1\tafter-current=bignum,0,2\tafter-amount=int,0,1,1\tidentity=1\tcode=0\tresult=39323233333732303336383534373735383038\n5\t1\tbefore-current=int,0,2,9223372036854775807\tbefore-amount=int,0,1,1\tafter-current=bignum,0,2\tafter-amount=int,0,1,1\toriginal-current=int,0,1,9223372036854775807\tidentity=0\tcode=0\tresult=39323233333732303336383534373735383038\n6\t0\tbefore-current=none,1,1\tbefore-amount=double,0,1\tafter-current=int,1,1,4\tafter-amount=double,1,1\tidentity=1\tcode=1\tresult=657870656374656420696e74656765722062757420676f742022312e3022\n6\t1\tbefore-current=none,1,2\tbefore-amount=double,0,1\tafter-current=none,1,2\tafter-amount=double,1,1\toriginal-current=none,1,2\tidentity=1\tcode=1\tresult=657870656374656420696e74656765722062757420676f742022312e3022\n7\t0\tbefore-current=none,1,1\tbefore-amount=double,0,1\tafter-current=int,1,1,4\tafter-amount=double,1,1\tidentity=1\tcode=1\tresult=657870656374656420696e74656765722062757420676f742022312e3522\n7\t1\tbefore-current=none,1,2\tbefore-amount=double,0,1\tafter-current=none,1,2\tafter-amount=double,1,1\toriginal-current=none,1,2\tidentity=1\tcode=1\tresult=657870656374656420696e74656765722062757420676f742022312e3522\n8\t0\tbefore-current=none,1,1\tbefore-amount=none,1,1\tafter-current=int,1,1,4\tafter-amount=none,1,1\tidentity=1\tcode=1\tresult=657870656374656420696e74656765722062757420676f742061206c697374\n8\t1\tbefore-current=none,1,2\tbefore-amount=none,1,1\tafter-current=none,1,2\tafter-amount=none,1,1\toriginal-current=none,1,2\tidentity=1\tcode=1\tresult=657870656374656420696e74656765722062757420676f742061206c697374\n9\t0\tbefore-current=none,1,1\tbefore-amount=none,1,1\tafter-current=int,1,1,4\tafter-amount=none,1,1\tidentity=1\tcode=1\tresult=657870656374656420696e74656765722062757420676f74202242414422\n9\t1\tbefore-current=none,1,2\tbefore-amount=none,1,1\tafter-current=none,1,2\tafter-amount=none,1,1\toriginal-current=none,1,2\tidentity=1\tcode=1\tresult=657870656374656420696e74656765722062757420676f74202242414422\n10\t0\tbefore-current=none,1,1\tbefore-amount=none,1,1\tafter-current=none,1,1\tafter-amount=none,1,1\tidentity=1\tcode=1\tresult=657870656374656420696e74656765722062757420676f74202242414422\n10\t1\tbefore-current=none,1,2\tbefore-amount=none,1,1\tafter-current=none,1,2\tafter-amount=none,1,1\toriginal-current=none,1,2\tidentity=1\tcode=1\tresult=657870656374656420696e74656765722062757420676f74202242414422\n11\t0\tbefore-current=missing,0,0\tbefore-amount=none,1,1\tafter-current=missing,0,0\tafter-amount=none,1,1\tidentity=0\tcode=1\tresult=657870656374656420696e74656765722062757420676f74202242414422\n11\t1\tbefore-current=missing,0,0\tbefore-amount=none,1,1\tafter-current=missing,0,0\tafter-amount=none,1,1\toriginal-current=missing,0,0\tidentity=0\tcode=1\tresult=657870656374656420696e74656765722062757420676f74202242414422\n12\t0\tbefore-current=missing,0,0\tbefore-amount=int,0,1,1\tafter-current=int,0,2,1\tafter-amount=int,0,1,1\tidentity=0\tcode=0\tresult=31\n12\t1\tbefore-current=missing,0,0\tbefore-amount=int,0,1,1\tafter-current=int,0,2,1\tafter-amount=int,0,1,1\toriginal-current=missing,0,0\tidentity=0\tcode=0\tresult=31\n13\t0\tbefore-current=none,1,1\tbefore-amount=none,1,1\tafter-current=int,0,2,5\tafter-amount=int,1,1,1\tidentity=1\tcode=0\tresult=35\n13\t1\tbefore-current=none,1,2\tbefore-amount=none,1,1\tafter-current=int,0,2,5\tafter-amount=int,1,1,1\toriginal-current=none,1,1\tidentity=0\tcode=0\tresult=35\n14\t0\tbefore-current=none,1,1\tbefore-amount=int,0,1,1\tafter-current=none,1,1\tafter-amount=int,1,1,1\tidentity=1\tcode=1\tresult=657870656374656420696e74656765722062757420676f74202242414422\n14\t1\tbefore-current=none,1,2\tbefore-amount=int,0,1,1\tafter-current=none,1,2\tafter-amount=int,1,1,1\toriginal-current=none,1,2\tidentity=1\tcode=1\tresult=657870656374656420696e74656765722062757420676f74202242414422\n"
}
```

### tcl9.1

Status: `observed`. Channel: Compiled retained C probe; original evaluator/API calls: Jim_EvalObjVector, Tcl_EvalObjv. Exact lengths, flags and input constructors remain in the source. Version: tcl9.1 9.1.0.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`manifest.json` (`file-6-row-11`):

```json
{
  "version": "9.1.0",
  "compile_exit": 0,
  "exit": 0,
  "stderr": "",
  "rows": 30
}
```

`9.1.0.tsv` (`file-4`):

```json
{
  "original_table": "0\t0\tbefore-current=none,1,1\tbefore-amount=int,0,1,1\tafter-current=int,0,2,5\tafter-amount=int,0,1,1\tidentity=1\tcode=0\tresult=35\n0\t1\tbefore-current=none,1,2\tbefore-amount=int,0,1,1\tafter-current=int,0,2,5\tafter-amount=int,0,1,1\toriginal-current=none,1,1\tidentity=0\tcode=0\tresult=35\n1\t0\tbefore-current=int,0,1,4\tbefore-amount=int,0,1,1\tafter-current=int,0,2,5\tafter-amount=int,0,1,1\tidentity=1\tcode=0\tresult=35\n1\t1\tbefore-current=int,0,2,4\tbefore-amount=int,0,1,1\tafter-current=int,0,2,5\tafter-amount=int,0,1,1\toriginal-current=int,0,1,4\tidentity=0\tcode=0\tresult=35\n2\t0\tbefore-current=int,0,1,4\tbefore-amount=int,0,1,1\tafter-current=int,0,2,5\tafter-amount=int,0,1,1\tidentity=1\tcode=0\tresult=35\n2\t1\tbefore-current=int,0,2,4\tbefore-amount=int,0,1,1\tafter-current=int,0,2,5\tafter-amount=int,0,1,1\toriginal-current=int,0,1,4\tidentity=0\tcode=0\tresult=35\n3\t0\tbefore-current=double,0,1\tbefore-amount=int,0,1,1\tafter-current=double,1,1\tafter-amount=int,1,1,1\tidentity=1\tcode=1\tresult=657870656374656420696e74656765722062757420676f742022342e3022\n3\t1\tbefore-current=double,0,2\tbefore-amount=int,0,1,1\tafter-current=double,0,2\tafter-amount=int,1,1,1\toriginal-current=double,0,2\tidentity=1\tcode=1\tresult=657870656374656420696e74656765722062757420676f742022342e3022\n4\t0\tbefore-current=none,1,1\tbefore-amount=int,0,1,1\tafter-current=double,1,1\tafter-amount=int,1,1,1\tidentity=1\tcode=1\tresult=657870656374656420696e74656765722062757420676f742022312e3522\n4\t1\tbefore-current=none,1,2\tbefore-amount=int,0,1,1\tafter-current=none,1,2\tafter-amount=int,1,1,1\toriginal-current=none,1,2\tidentity=1\tcode=1\tresult=657870656374656420696e74656765722062757420676f742022312e3522\n5\t0\tbefore-current=int,0,1,9223372036854775807\tbefore-amount=int,0,1,1\tafter-current=bignum,0,2\tafter-amount=int,0,1,1\tidentity=1\tcode=0\tresult=39323233333732303336383534373735383038\n5\t1\tbefore-current=int,0,2,9223372036854775807\tbefore-amount=int,0,1,1\tafter-current=bignum,0,2\tafter-amount=int,0,1,1\toriginal-current=int,0,1,9223372036854775807\tidentity=0\tcode=0\tresult=39323233333732303336383534373735383038\n6\t0\tbefore-current=none,1,1\tbefore-amount=double,0,1\tafter-current=int,1,1,4\tafter-amount=double,1,1\tidentity=1\tcode=1\tresult=657870656374656420696e74656765722062757420676f742022312e3022\n6\t1\tbefore-current=none,1,2\tbefore-amount=double,0,1\tafter-current=none,1,2\tafter-amount=double,1,1\toriginal-current=none,1,2\tidentity=1\tcode=1\tresult=657870656374656420696e74656765722062757420676f742022312e3022\n7\t0\tbefore-current=none,1,1\tbefore-amount=double,0,1\tafter-current=int,1,1,4\tafter-amount=double,1,1\tidentity=1\tcode=1\tresult=657870656374656420696e74656765722062757420676f742022312e3522\n7\t1\tbefore-current=none,1,2\tbefore-amount=double,0,1\tafter-current=none,1,2\tafter-amount=double,1,1\toriginal-current=none,1,2\tidentity=1\tcode=1\tresult=657870656374656420696e74656765722062757420676f742022312e3522\n8\t0\tbefore-current=none,1,1\tbefore-amount=none,1,1\tafter-current=int,1,1,4\tafter-amount=none,1,1\tidentity=1\tcode=1\tresult=657870656374656420696e74656765722062757420676f742061206c697374\n8\t1\tbefore-current=none,1,2\tbefore-amount=none,1,1\tafter-current=none,1,2\tafter-amount=none,1,1\toriginal-current=none,1,2\tidentity=1\tcode=1\tresult=657870656374656420696e74656765722062757420676f742061206c697374\n9\t0\tbefore-current=none,1,1\tbefore-amount=none,1,1\tafter-current=int,1,1,4\tafter-amount=none,1,1\tidentity=1\tcode=1\tresult=657870656374656420696e74656765722062757420676f74202242414422\n9\t1\tbefore-current=none,1,2\tbefore-amount=none,1,1\tafter-current=none,1,2\tafter-amount=none,1,1\toriginal-current=none,1,2\tidentity=1\tcode=1\tresult=657870656374656420696e74656765722062757420676f74202242414422\n10\t0\tbefore-current=none,1,1\tbefore-amount=none,1,1\tafter-current=none,1,1\tafter-amount=none,1,1\tidentity=1\tcode=1\tresult=657870656374656420696e74656765722062757420676f74202242414422\n10\t1\tbefore-current=none,1,2\tbefore-amount=none,1,1\tafter-current=none,1,2\tafter-amount=none,1,1\toriginal-current=none,1,2\tidentity=1\tcode=1\tresult=657870656374656420696e74656765722062757420676f74202242414422\n11\t0\tbefore-current=missing,0,0\tbefore-amount=none,1,1\tafter-current=missing,0,0\tafter-amount=none,1,1\tidentity=0\tcode=1\tresult=657870656374656420696e74656765722062757420676f74202242414422\n11\t1\tbefore-current=missing,0,0\tbefore-amount=none,1,1\tafter-current=missing,0,0\tafter-amount=none,1,1\toriginal-current=missing,0,0\tidentity=0\tcode=1\tresult=657870656374656420696e74656765722062757420676f74202242414422\n12\t0\tbefore-current=missing,0,0\tbefore-amount=int,0,1,1\tafter-current=int,0,2,1\tafter-amount=int,0,1,1\tidentity=0\tcode=0\tresult=31\n12\t1\tbefore-current=missing,0,0\tbefore-amount=int,0,1,1\tafter-current=int,0,2,1\tafter-amount=int,0,1,1\toriginal-current=missing,0,0\tidentity=0\tcode=0\tresult=31\n13\t0\tbefore-current=none,1,1\tbefore-amount=none,1,1\tafter-current=int,0,2,5\tafter-amount=int,1,1,1\tidentity=1\tcode=0\tresult=35\n13\t1\tbefore-current=none,1,2\tbefore-amount=none,1,1\tafter-current=int,0,2,5\tafter-amount=int,1,1,1\toriginal-current=none,1,1\tidentity=0\tcode=0\tresult=35\n14\t0\tbefore-current=none,1,1\tbefore-amount=int,0,1,1\tafter-current=none,1,1\tafter-amount=int,1,1,1\tidentity=1\tcode=1\tresult=657870656374656420696e74656765722062757420676f74202242414422\n14\t1\tbefore-current=none,1,2\tbefore-amount=int,0,1,1\tafter-current=none,1,2\tafter-amount=int,1,1,1\toriginal-current=none,1,2\tidentity=1\tcode=1\tresult=657870656374656420696e74656765722062757420676f74202242414422\n"
}
```

### jim

Status: `observed`. Channel: Compiled retained C probe; original evaluator/API calls: Jim_EvalObjVector, Tcl_EvalObjv. Exact lengths, flags and input constructors remain in the source. Version: jim Jim.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`manifest.json` (`file-6-row-12`):

```json
{
  "version": "Jim",
  "compile_exit": 0,
  "exit": 0,
  "stderr": "",
  "rows": 30
}
```

`Jim.tsv` (`file-5`):

```json
{
  "original_table": "0\t0\tbefore-current=none,1,1\tbefore-amount=int,0,1,1\tafter-current=int,0,2,5\tafter-amount=int,0,1,1\tidentity=1\tcode=0\tresult=35\n0\t1\tbefore-current=none,1,2\tbefore-amount=int,0,1,1\tafter-current=int,0,2,5\tafter-amount=int,0,1,1\toriginal-current=int,1,1,4\tidentity=0\tcode=0\tresult=35\n1\t0\tbefore-current=int,0,1,4\tbefore-amount=int,0,1,1\tafter-current=int,0,2,5\tafter-amount=int,0,1,1\tidentity=1\tcode=0\tresult=35\n1\t1\tbefore-current=int,0,2,4\tbefore-amount=int,0,1,1\tafter-current=int,0,2,5\tafter-amount=int,0,1,1\toriginal-current=int,0,1,4\tidentity=0\tcode=0\tresult=35\n2\t0\tbefore-current=int,0,1,4\tbefore-amount=int,0,1,1\tafter-current=int,0,2,5\tafter-amount=int,0,1,1\tidentity=1\tcode=0\tresult=35\n2\t1\tbefore-current=int,0,2,4\tbefore-amount=int,0,1,1\tafter-current=int,0,2,5\tafter-amount=int,0,1,1\toriginal-current=int,0,1,4\tidentity=0\tcode=0\tresult=35\n3\t0\tbefore-current=double,0,1\tbefore-amount=int,0,1,1\tafter-current=double,1,1\tafter-amount=int,0,2,1\tidentity=1\tcode=1\tresult=657870656374656420696e74656765722062757420676f742022342e3022\n3\t1\tbefore-current=double,0,2\tbefore-amount=int,0,1,1\tafter-current=double,1,2\tafter-amount=int,0,2,1\toriginal-current=double,1,2\tidentity=1\tcode=1\tresult=657870656374656420696e74656765722062757420676f742022342e3022\n4\t0\tbefore-current=none,1,1\tbefore-amount=int,0,1,1\tafter-current=none,1,1\tafter-amount=int,0,2,1\tidentity=1\tcode=1\tresult=657870656374656420696e74656765722062757420676f742022312e3522\n4\t1\tbefore-current=none,1,2\tbefore-amount=int,0,1,1\tafter-current=none,1,2\tafter-amount=int,0,2,1\toriginal-current=none,1,2\tidentity=1\tcode=1\tresult=657870656374656420696e74656765722062757420676f742022312e3522\n5\t0\tbefore-current=int,0,1,9223372036854775807\tbefore-amount=int,0,1,1\tafter-current=int,0,2,-9223372036854775808\tafter-amount=int,0,1,1\tidentity=1\tcode=0\tresult=2d39323233333732303336383534373735383038\n5\t1\tbefore-current=int,0,2,9223372036854775807\tbefore-amount=int,0,1,1\tafter-current=int,0,2,-9223372036854775808\tafter-amount=int,0,1,1\toriginal-current=int,0,1,9223372036854775807\tidentity=0\tcode=0\tresult=2d39323233333732303336383534373735383038\n6\t0\tbefore-current=none,1,1\tbefore-amount=double,0,1\tafter-current=none,1,1\tafter-amount=expression,1,2\tidentity=1\tcode=1\tresult=657870656374656420696e74656765722065787072657373696f6e2062757420676f742022312e3022\n6\t1\tbefore-current=none,1,2\tbefore-amount=double,0,1\tafter-current=none,1,2\tafter-amount=expression,1,2\toriginal-current=none,1,2\tidentity=1\tcode=1\tresult=657870656374656420696e74656765722065787072657373696f6e2062757420676f742022312e3022\n7\t0\tbefore-current=none,1,1\tbefore-amount=double,0,1\tafter-current=none,1,1\tafter-amount=expression,1,2\tidentity=1\tcode=1\tresult=657870656374656420696e74656765722065787072657373696f6e2062757420676f742022312e3522\n7\t1\tbefore-current=none,1,2\tbefore-amount=double,0,1\tafter-current=none,1,2\tafter-amount=expression,1,2\toriginal-current=none,1,2\tidentity=1\tcode=1\tresult=657870656374656420696e74656765722065787072657373696f6e2062757420676f742022312e3522\n8\t0\tbefore-current=none,1,1\tbefore-amount=none,1,1\tafter-current=int,0,2,7\tafter-amount=expression,1,1\tidentity=1\tcode=0\tresult=37\n8\t1\tbefore-current=none,1,2\tbefore-amount=none,1,1\tafter-current=int,0,2,7\tafter-amount=expression,1,1\toriginal-current=int,1,1,4\tidentity=0\tcode=0\tresult=37\n9\t0\tbefore-current=none,1,1\tbefore-amount=none,1,1\tafter-current=none,1,1\tafter-amount=expression,1,2\tidentity=1\tcode=1\tresult=657870656374656420696e74656765722065787072657373696f6e2062757420676f74202242414422\n9\t1\tbefore-current=none,1,2\tbefore-amount=none,1,1\tafter-current=none,1,2\tafter-amount=expression,1,2\toriginal-current=none,1,2\tidentity=1\tcode=1\tresult=657870656374656420696e74656765722065787072657373696f6e2062757420676f74202242414422\n10\t0\tbefore-current=none,1,1\tbefore-amount=none,1,1\tafter-current=none,1,1\tafter-amount=expression,1,2\tidentity=1\tcode=1\tresult=657870656374656420696e74656765722065787072657373696f6e2062757420676f74202242414422\n10\t1\tbefore-current=none,1,2\tbefore-amount=none,1,1\tafter-current=none,1,2\tafter-amount=expression,1,2\toriginal-current=none,1,2\tidentity=1\tcode=1\tresult=657870656374656420696e74656765722065787072657373696f6e2062757420676f74202242414422\n11\t0\tbefore-current=missing,0,0\tbefore-amount=none,1,1\tafter-current=missing,0,0\tafter-amount=expression,1,2\tidentity=0\tcode=1\tresult=657870656374656420696e74656765722065787072657373696f6e2062757420676f74202242414422\n11\t1\tbefore-current=missing,0,0\tbefore-amount=none,1,1\tafter-current=missing,0,0\tafter-amount=expression,1,2\toriginal-current=missing,0,0\tidentity=0\tcode=1\tresult=657870656374656420696e74656765722065787072657373696f6e2062757420676f74202242414422\n12\t0\tbefore-current=missing,0,0\tbefore-amount=int,0,1,1\tafter-current=int,0,2,1\tafter-amount=int,0,1,1\tidentity=0\tcode=0\tresult=31\n12\t1\tbefore-current=missing,0,0\tbefore-amount=int,0,1,1\tafter-current=int,0,2,1\tafter-amount=int,0,1,1\toriginal-current=missing,0,0\tidentity=0\tcode=0\tresult=31\n13\t0\tbefore-current=none,1,1\tbefore-amount=none,1,1\tafter-current=int,0,2,5\tafter-amount=int,1,1,1\tidentity=1\tcode=0\tresult=35\n13\t1\tbefore-current=none,1,2\tbefore-amount=none,1,1\tafter-current=int,0,2,5\tafter-amount=int,1,1,1\toriginal-current=int,1,1,4\tidentity=0\tcode=0\tresult=35\n14\t0\tbefore-current=none,1,1\tbefore-amount=int,0,1,1\tafter-current=none,1,1\tafter-amount=int,0,2,1\tidentity=1\tcode=1\tresult=657870656374656420696e74656765722062757420676f74202242414422\n14\t1\tbefore-current=none,1,2\tbefore-amount=int,0,1,1\tafter-current=none,1,2\tafter-amount=int,0,2,1\toriginal-current=none,1,2\tidentity=1\tcode=1\tresult=657870656374656420696e74656765722062757420676f74202242414422\n"
}
```

### bigip

Status: `not-tested`.

No execution of this question is attached for this provider. An API or command observed elsewhere supplies no answer here.

## Consumer bindings

No implementation binding is claimed by this observation record.

## Evidence

- [rust/tcl-vm/tests/data/native_legacy_increment/8.4.20.tsv](../../../../rust/tcl-vm/tests/data/native_legacy_increment/8.4.20.tsv)
- [rust/tcl-vm/tests/data/native_legacy_increment/8.5.19.tsv](../../../../rust/tcl-vm/tests/data/native_legacy_increment/8.5.19.tsv)
- [rust/tcl-vm/tests/data/native_legacy_increment/8.6.18.tsv](../../../../rust/tcl-vm/tests/data/native_legacy_increment/8.6.18.tsv)
- [rust/tcl-vm/tests/data/native_legacy_increment/9.0.4.tsv](../../../../rust/tcl-vm/tests/data/native_legacy_increment/9.0.4.tsv)
- [rust/tcl-vm/tests/data/native_legacy_increment/9.1.0.tsv](../../../../rust/tcl-vm/tests/data/native_legacy_increment/9.1.0.tsv)
- [rust/tcl-vm/tests/data/native_legacy_increment/Jim.tsv](../../../../rust/tcl-vm/tests/data/native_legacy_increment/Jim.tsv)
- [rust/tcl-vm/tests/data/native_legacy_increment/manifest.json](../../../../rust/tcl-vm/tests/data/native_legacy_increment/manifest.json)
- [rust/tcl-vm/tests/data/native_legacy_increment/probe.c](../../../../rust/tcl-vm/tests/data/native_legacy_increment/probe.c)
