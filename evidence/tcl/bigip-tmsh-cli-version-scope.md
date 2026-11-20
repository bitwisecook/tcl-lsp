# Scope of `tmsh::modify cli version active` in a cli script

## Question

When a tmsh `cli script` runs `tmsh::modify cli version active <v>`, how far does the selected command-syntax version reach: the rest of the script only, the calling tmsh process or session, child processes, or the whole system? `rust/tcl-registry/src/f5/tmsh_syntax.rs` keeps `scope_is_measured = false` and assumes script-local scope until this is measured. Only scope is measured here; which commands each schema version accepts is not.

## Assumptions

Run on 2026-11-20 UTC on BIG-IP 21.1.0.1 build 0.0.26 (VE, single NIC) as root over SSH. The script below was loaded as `cli script tcllsp_oq_m` with `tmsh load sys config merge file`, after checking that the name was absent, and deleted afterwards. The configuration was never saved. Supported versions on this build are 14.0.0 through 21.1.0.1, and the active version before and after the run was 21.1.0.1. The 11.5.0 and 11.6.0 versions in F5's published multi-version example are refused on this build (`Invalid version`). Each numbered run is one invocation of the `tmsh` binary from bash. `tmsh -c "a; b; c"` runs its commands in one tmsh process. `run util bash -c` starts a child process from that tmsh. The version is read with `show cli version`. Inside a cli script, `tmsh::show cli version` returned an empty string, so the script cannot report the active version itself. Its `Loading schema version: …` line shows when a selection takes effect.

## Script

```tcl-visible
proc script::run {} {
    tmsh::modify cli version active 14.0.0
    puts "M-modified"
}
```

## Results

### BIG-IP 21.1.0.1 build 0.0.26, tmsh as root

- Channel: four `tmsh -c` invocations from one bash shell over SSH, each filtered to its `active`, `Loading` and `M-` lines; then a fresh `tmsh show cli version`.
- Exit status: Not recorded
- Standard output: Not recorded
- Standard error: Not recorded
- Recorded result:

```text-visible
== S1 session: show; modify 17.5.0; show
    active 21.1.0.1
Loading schema version: 17.5.0
    active 17.5.0
== S2 session: run script that modifies to 14.0.0; show
Loading schema version: 14.0.0
M-modified
    active 21.1.0.1
== S3 session: modify 17.5.0; run script (14.0.0); show
Loading schema version: 17.5.0
Loading schema version: 14.0.0
M-modified
    active 17.5.0
== S4 session: modify 17.5.0; child bash runs tmsh show
Loading schema version: 17.5.0
    active 21.1.0.1
== fresh
    active 21.1.0.1
```

S1 is `tmsh -c "show cli version; modify cli version active 17.5.0; show cli version"`. S2 is `tmsh -c "run cli script tcllsp_oq_m; show cli version"`. S3 is `tmsh -c "modify cli version active 17.5.0; run cli script tcllsp_oq_m; show cli version"`. S4 is `tmsh -c "modify cli version active 17.5.0; run util bash -c \"tmsh show cli version\""`. "fresh" is a separate `tmsh show cli version` afterwards.

A selection made inside a cli script takes effect for the rest of that script, and the caller's version is restored when the script returns (S2, S3). A selection made directly in a tmsh session persists for the later commands of that process (S1). A child process (S4), a new process and other sessions do not see it, and it is not saved to configuration.
