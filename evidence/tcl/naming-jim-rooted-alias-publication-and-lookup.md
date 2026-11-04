# Tcl execution evidence

## Question

> An alias command can retain its original returned spelling while replacing a procedure and supporting multiple rooted call spellings. Those public distinctions require independent measurements before assigning source or port comparison keys, and namespace-local competition remains a separate lookup condition.

> What original alias spellings, caught call results and body-query statuses do rooted, multiword and extra-root publications produce, and how do independently authored namespace-local and global commands compete?

## Assumptions

Assumptions were not separately recorded in the retained capture.

Recorded scope and limits:

> One fixed ASCII LF CLI file per fresh provider process uses the original standalone alias command, returned public naming spelling, caught ordinary call results and info-body status. Four exact controls cover rooted, multiword and extra-root spellings plus independent namespace/global competition. No NUL, non-ASCII, continuation/control escape, alias enumeration, arbitrary namespace path or class/package branch is tested. Exact input/command/executable/library/header/build/source associations are checked independently. Source hash/registration excerpts do not prove compiled-source identity or a cache/object lifetime law. Runtime API and VM consumers require their separate supported ingress and source requirements; matching caller projection cannot attest an unavailable standalone port alias command. A marked Runtime control compares all four unchanged Jim original public rows under actual core alias registration. Its linked definition is implementation coverage, not an executed Rust pass, new native run or public header/pointer observer. Actual source-distribution bootstrap and C interp/child alias APIs retain their separate purposes.

## Script

```tcl-visible
puts [list version [info patchlevel] alias_available [info commands alias]]
if {[llength [info commands alias]] == 0} {
    puts [list alias_publication_scope not_applicable]
    exit 0
}
proc ::alias_slot {} {return ORIGINAL}
set original [alias ::alias_slot list ROOT_ALIAS]
set bare_code [catch {alias_slot} bare_result]
set rooted_code [catch {::alias_slot} rooted_result]
set body_code [catch {info body ::alias_slot} body_result]
puts [list rooted_replacement $original $bare_code $bare_result $rooted_code $rooted_result $body_code]
proc {::AliasProbe slot} {} {return ORIGINAL_MULTIWORD}
set original [alias {::AliasProbe slot} list MULTIWORD_ALIAS]
set bare_code [catch {{AliasProbe slot}} bare_result]
set rooted_code [catch {{::AliasProbe slot}} rooted_result]
set body_code [catch {info body {::AliasProbe slot}} body_result]
puts [list multiword_replacement $original $bare_code $bare_result $rooted_code $rooted_result $body_code]
set original [alias ::::colon_slot list COLON_ALIAS]
set bare_code [catch {colon_slot} bare_result]
set rooted_code [catch {::colon_slot} rooted_result]
set extra_root_code [catch {::::colon_slot} extra_root_result]
puts [list extra_root_extent $original $bare_code $bare_result $rooted_code $rooted_result $extra_root_code $extra_root_result]
proc competition_slot {} {return ORIGINAL_GLOBAL}
set original [namespace eval AliasContext {
    proc competition_slot {} {return LOCAL_PROC}
    alias ::competition_slot list GLOBAL_ALIAS
}]
set global_code [catch {competition_slot} global_result]
set local_code [catch {namespace eval AliasContext {competition_slot}} local_result]
set rooted_code [catch {namespace eval AliasContext {::competition_slot}} rooted_result]
puts [list namespace_competition $original $global_code $global_result $local_code $local_result $rooted_code $rooted_result]
```

## Results

### tcl8.4

- Recorded version: 8.4.20
- Recorded input channel: One fixed ASCII LF CLI source file. Public alias return spelling, caught command invocation values and info-body success code only. No NUL, non-ASCII, private table/frame/object identity, alias-query enumeration, or class/package scope claims.
- Exit status: 0
- Standard output:

```text-visible
version 8.4.20 alias_available {}
alias_publication_scope not_applicable
```

- Standard error: empty.

### tcl8.5

- Recorded version: 8.5.19
- Recorded input channel: One fixed ASCII LF CLI source file. Public alias return spelling, caught command invocation values and info-body success code only. No NUL, non-ASCII, private table/frame/object identity, alias-query enumeration, or class/package scope claims.
- Exit status: 0
- Standard output:

```text-visible
version 8.5.19 alias_available {}
alias_publication_scope not_applicable
```

- Standard error: empty.

### tcl8.6

- Recorded version: 8.6.18
- Recorded input channel: One fixed ASCII LF CLI source file. Public alias return spelling, caught command invocation values and info-body success code only. No NUL, non-ASCII, private table/frame/object identity, alias-query enumeration, or class/package scope claims.
- Exit status: 0
- Standard output:

```text-visible
version 8.6.18 alias_available {}
alias_publication_scope not_applicable
```

- Standard error: empty.

### tcl9.0

- Recorded version: 9.0.4
- Recorded input channel: One fixed ASCII LF CLI source file. Public alias return spelling, caught command invocation values and info-body success code only. No NUL, non-ASCII, private table/frame/object identity, alias-query enumeration, or class/package scope claims.
- Exit status: 0
- Standard output:

```text-visible
version 9.0.4 alias_available {}
alias_publication_scope not_applicable
```

- Standard error: empty.

### tcl9.1

- Recorded version: 9.1.0
- Recorded input channel: One fixed ASCII LF CLI source file. Public alias return spelling, caught command invocation values and info-body success code only. No NUL, non-ASCII, private table/frame/object identity, alias-query enumeration, or class/package scope claims.
- Exit status: 0
- Standard output:

```text-visible
version 9.1.0 alias_available {}
alias_publication_scope not_applicable
```

- Standard error: empty.

### jim

- Recorded version: 0.84-9-g5bac7c9
- Recorded input channel: One fixed ASCII LF CLI source file. Public alias return spelling, caught command invocation values and info-body success code only. No NUL, non-ASCII, private table/frame/object identity, alias-query enumeration, or class/package scope claims.
- Exit status: 0
- Standard output:

```text-visible
version 0.84-9-g5bac7c9 alias_available alias
rooted_replacement ::alias_slot 0 ROOT_ALIAS 0 ROOT_ALIAS 1
multiword_replacement {::AliasProbe slot} 0 MULTIWORD_ALIAS 0 MULTIWORD_ALIAS 1
extra_root_extent ::::colon_slot 0 COLON_ALIAS 0 COLON_ALIAS 0 COLON_ALIAS
namespace_competition ::competition_slot 0 GLOBAL_ALIAS 0 LOCAL_PROC 0 GLOBAL_ALIAS
```

- Standard error: empty.
