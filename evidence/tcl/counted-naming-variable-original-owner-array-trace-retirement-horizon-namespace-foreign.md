# Tcl counted-source execution evidence

## Question

> Namespace deletion and procedure return are distinct array-owner teardown operations. Public logs and final alias/namespace queries are needed to determine which originals reach their proposed trace horizon, without assuming that an explicit array-unset observation applies to either owner teardown.

> What whole completions, callback logs and final namespace/member queries do the five original owner-teardown programs return, and which frame originals reach trace setup?

This script is one recorded source in the stated question; it does not by itself establish results for other sources or environments.

## Assumptions

Recorded scope and limits:

> Five untouched ASCII LF originals each run in six fresh fully initialised provider interpreters under the unchanged counted-source CAPI driver. Thirty whole process receipts/streams and six compile receipts/ELFs retain every setup refusal and namespace result. The initial request, expanded capture request and launcher are preserved as distinct original artifacts. Only the three admitted C namespace sources supply teardown log/query observations. Neither bad-level frame input establishes frame teardown or accessible old-variable mutation; Jim supplies only the measured trace-API refusal. No private identity, Native lookup/frame/argv/header purpose, opaque/NUL/Unicode name, BIG-IP execution or executed software control is supplied. Current linked comparisons consume complete unchanged public code/result windows without host refusal, keeping admitted namespace/caller observations separate from rejected frame setups and no executed Rust pass claim. Software namespace teardown retains its actual original parent edge while callbacks run and retires only the selected root binding; same-spelled foreign state and private receiver correspondence require their independent software owners. Public logs do not prove those private identities. Namespace teardown obtains the actual selected root home directly from its retained receiver and namespace tuple, without reconstructing a second private lookup. Public namespace values/logs remain unchanged and do not certify software private root/home correspondence.

Other assumptions were not separately recorded. The retained driver evaluated this single counted Tcl source without a prelude or query source; its process exit is separate from Tcl's completion code.

## Script

```tcl-visible
set log {}; proc foreign args {lappend ::log FOREIGN}; proc member args {lappend ::log OLD}; proc root args {lappend ::log ROOT; namespace eval ::Foreign {set a(x) FRESH}; trace add variable ::Foreign::a(x) unset foreign}; namespace eval ::N {set a(x) X}; upvar #0 ::N::a(x) xx; trace add variable ::N::a(x) unset member; trace add variable ::N::a unset root; namespace delete ::N; set code [catch {set ::Foreign::a(x)} value]; list $log [namespace exists ::N] [namespace exists ::Foreign] $code $value [trace info variable ::Foreign::a(x)]
```

## Results

### tcl8.4, run 1

- Recorded version: 8.4.20
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|382e342e3230
ORIGINAL|0|7b524f4f54204f4c447d203020312030204652455348207b7b756e73657420666f726569676e7d7d
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
{ROOT OLD} 0 1 0 FRESH {{unset foreign}}⟦EOF⟧
```

### tcl8.5, run 2

- Recorded version: 8.5.19
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|382e352e3139
ORIGINAL|0|7b524f4f54204f4c447d203020312030204652455348207b7b756e73657420666f726569676e7d7d
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
{ROOT OLD} 0 1 0 FRESH {{unset foreign}}⟦EOF⟧
```

### tcl8.6, run 3

- Recorded version: 8.6.18
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|382e362e3138
ORIGINAL|0|7b524f4f54204f4c447d203020312030204652455348207b7b756e73657420666f726569676e7d7d
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
{ROOT OLD} 0 1 0 FRESH {{unset foreign}}⟦EOF⟧
```

### tcl9.0, run 4

- Recorded version: 9.0.4
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|392e302e34
ORIGINAL|0|7b524f4f54204f4c447d203020312030204652455348207b7b756e73657420666f726569676e7d7d
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
{ROOT OLD} 0 1 0 FRESH {{unset foreign}}⟦EOF⟧
```

### tcl9.1, run 5

- Recorded version: 9.1.0
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|392e312e30
ORIGINAL|0|7b524f4f54204f4c447d203020312030204652455348207b7b756e73657420666f726569676e7d7d
```

- Standard error: empty.

Tcl completion code for the original source: 0. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
{ROOT OLD} 0 1 0 FRESH {{unset foreign}}⟦EOF⟧
```

### jim, run 6

- Recorded version: 0.84-9-g5bac7c9
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|302e38342d392d6735626163376339
ORIGINAL|1|696e76616c696420636f6d6d616e64206e616d652022747261636522
```

- Standard error: empty.

Tcl completion code for the original source: 1. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
invalid command name "trace"⟦EOF⟧
```
