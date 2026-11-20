# What a tmsh cli script can call under each user role

## Question

Does a tmsh `cli script` see the same Tcl command surface and tmsh access whatever the role of the user running it? In particular, does `exec` exist for non-administrator roles, and are tmsh commands refused at authorisation time or hidden at syntax level? The analyser treats the tmsh script context as one surface with no role annotation.

## Assumptions

Run on 2026-11-20 UTC on BIG-IP 21.1.0.1 build 0.0.26. `systemauth.disablebash` was `false`. The script below was loaded as `cli script tcllsp_oq_role`. For each role, a disposable local user `tcllsp_oq_<role>` was created with that role on all partitions and shell `tmsh`. Each user got a random password it never used, the expired-password flag was cleared with `chage`, and a public key was installed for an SSH login. The script ran with `ssh <user>@host 'run cli script tcllsp_oq_role'`, a real login. `su` from root is not a substitute, because tmsh then still grants root's access. Users, rules the script created and the script itself were deleted afterwards, and nothing was saved. The root and admin rows used the same loop body as a separate script, `tcllsp_oq_execctl`, run as root from bash and as admin over SSH. That run recorded only `exec id -un` and the command count.

## Script

```tcl-visible
proc script::run {} {
    foreach {k cmd} {
        list_rule {tmsh::list ltm rule tcllsp_oq_role_absent}
        show_version {tmsh::show sys version}
        create_rule {tmsh::create ltm rule tcllsp_oq_role_script_made}
        delete_rule {tmsh::delete ltm rule tcllsp_oq_role_script_made}
        exec_id {exec id}
    } {
        set c [catch {eval $cmd} r]
        puts "ROLE-SCRIPT $k $c [string range [string map {"\n" " "} $r] 0 100]"
    }
}
```

## Results

### BIG-IP 21.1.0.1 build 0.0.26, role guest

- Exit status: 0
- Standard output:

```text-visible
ROLE-SCRIPT list_rule 1 01020036:3: The requested iRule (tcllsp_oq_role_absent) was not found.
ROLE-SCRIPT show_version 0 Sys::Version Main Package Product BIG-IP Version 21.1.0.1 Build 0.0.26 Edi
ROLE-SCRIPT create_rule 1 "rule" unexpected argument
ROLE-SCRIPT delete_rule 1 user tcllsp_oq_guest with role guest doesn't have access to "rule"
ROLE-SCRIPT exec_id 1 invalid command name "exec"
```

- Standard error: Not recorded

### BIG-IP 21.1.0.1 build 0.0.26, role operator

- Exit status: 0
- Standard output:

```text-visible
ROLE-SCRIPT list_rule 1 01020036:3: The requested iRule (tcllsp_oq_role_absent) was not found.
ROLE-SCRIPT show_version 0 Sys::Version Main Package Product BIG-IP Version 21.1.0.1 Build 0.0.26 Edi
ROLE-SCRIPT create_rule 1 "rule" unexpected argument
ROLE-SCRIPT delete_rule 1 user tcllsp_oq_operator with role operator doesn't have access to "rule"
ROLE-SCRIPT exec_id 1 invalid command name "exec"
```

- Standard error: Not recorded

### BIG-IP 21.1.0.1 build 0.0.26, role manager

- Exit status: 0
- Standard output:

```text-visible
ROLE-SCRIPT list_rule 1 01020036:3: The requested iRule (tcllsp_oq_role_absent) was not found.
ROLE-SCRIPT show_version 0 Sys::Version Main Package Product BIG-IP Version 21.1.0.1 Build 0.0.26 Edi
ROLE-SCRIPT create_rule 0
ROLE-SCRIPT delete_rule 0
ROLE-SCRIPT exec_id 1 invalid command name "exec"
```

- Standard error: Not recorded

### BIG-IP 21.1.0.1 build 0.0.26, role resource-admin

- Exit status: 0
- Standard output:

```text-visible
ROLE-SCRIPT list_rule 1 01020036:3: The requested iRule (tcllsp_oq_role_absent) was not found.
ROLE-SCRIPT show_version 0 Sys::Version Main Package Product BIG-IP Version 21.1.0.1 Build 0.0.26 Edi
ROLE-SCRIPT create_rule 0
ROLE-SCRIPT delete_rule 0
ROLE-SCRIPT exec_id 1 invalid command name "exec"
```

- Standard error: Not recorded

### BIG-IP 21.1.0.1 build 0.0.26, root and admin (administrator)

- Channel: the `tcllsp_oq_execctl` control described in Assumptions.
- Exit status: Not recorded
- Standard output: Not recorded
- Standard error: Not recorded
- Recorded result:

```text-visible
root: EXEC 0 root INFO 95 1
admin: EXEC 0 root INFO 95 1
```

`EXEC <catch code> <exec id -un>`; `INFO <llength [info commands]> <exec present>`.

`exec` is absent from the cli-script interpreter for guest, operator, manager and resource-admin. For root and admin it is present: 95 commands, and it runs as root. Guest and operator get a syntax-level `"rule" unexpected argument` for `create ltm rule`, so the command is hidden rather than refused, while `delete` reports an authorisation error. The same split appeared in direct `tmsh` commands over SSH. In the run that also issued `run util bash`, every one of these roles got `Syntax Error: "bash" unexpected argument`.
