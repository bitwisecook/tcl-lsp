# F5 procedure-local name geometry

Status: unanswered. No BIG-IP/TMM behavior is inferred from C Tcl or Jim.

Run the fixed cases in
`experiments/name-handling/results/frame-local-name-geometry/probe.tcl` only
in an isolated test iRule/harness, with unique harness procedure names. Record
the exact adapted source and any setup refusal. A tmsh shell or Linux tclsh
answer is a separate context and cannot answer TMM procedure behavior.

Record BIG-IP release/build, interpreter context, event and rule/namespace
scope, deployment/harness channel, exact source/argv, exit/completion codes and
full output/error text. Answer these questions separately:

- Does an empty procedure-local name shadow an empty global without changing it?
- Does an unbound procedure read of `${}` ignore an existing empty global?
- Are an empty formal, `$name` formal and `name` formal accepted, and which exact
  local does each body reference read?
- Is a relative `inner::value` name activation-local or namespace-owned?
- Do interpreter and compiled iRule channels return the same observed results?

Write answers and untouched transcripts under
`experiments/name-handling/f5/answers/frame-local-name-geometry/` on this branch.
Do not substitute results from another context or treat an unavailable case as
success.
