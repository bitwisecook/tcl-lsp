# BIG-IP questions: XC translation source owners

Status: unanswered. Generic C Tcl and Jim mock-getter results establish word grammar only. They do not establish these appliance behaviors. Record original appliance results in answer documents on the `work` branch, with exact version, rule/profile/traffic context, full source, commands, stdout/logs/errors, worker identity and capture file checksums. Preserve failed install and execution attempts. Do not replace missing evidence with C/Jim expectations.

The translator supplies conditional authored source advice. Its planned shared owners require complete original source and selected descriptors; separate appliance entry, command holder and successful effects remain obligations. These questions determine where an appliance-specific naming policy can later be admitted.

## Command heads and replacement

For a lab HTTP virtual server, compare an ordinary `[HTTP::path]` getter with static quoted, braced, escaped and rooted command heads: `["HTTP::path"]`, `[{HTTP::path}]`, `[HTTP\x3a\x3apath]`, `[::HTTP::path]`. Install each rule independently; record configuration acceptance separately from traffic-time result. Preserve the request path and response/log output for each case. Do not assume static lexical equivalence establishes TMM dispatch equivalence.

In a separate disposable rule, determine whether declaring `proc HTTP::path {} {return /replacement}` is accepted and where the procedure is published. Compare defining it before an HTTP_REQUEST handler, inside RULE_INIT, and inside HTTP_REQUEST before a subsequent getter. Record rule namespace, worker context, lookup outcome and exact errors. If a declaration is rejected, that result is the answer; do not substitute another mechanism and mark it equivalent.

Where supported, compare the same getter source text before and after an actual replacement, and from two different iRule namespaces. Identify whether rule/profile attachment order changes lookup. Changes to reported labels alone are not replacement evidence.

## Wrapper and original operand semantics

Compare `[string tolower [HTTP::path]]`, `[string {tolower} [HTTP::path]]`, and `[string tolower {[HTTP::path]}]` under the same HTTP request. The third case is a separate literal-data control; record its original output instead of predicting it. Compare a getter plus an extra argument and a bracket body containing two commands; preserve exact failure stage and completion/error tuple if available.

For a `switch` subject, compare an executable `[HTTP::path]`, quoted `"[HTTP::path]"`, and braced `{[HTTP::path]}`. Use distinguishable arm outputs so the observed match identifies whether the subject substituted. Keep rule install acceptance, selected arm and traffic-time getter availability separate.

## Nested namespace and source availability

Determine whether an iRule procedure can access HTTP getters when invoked from HTTP_REQUEST, from RULE_INIT, and from an event where the getter is unavailable. Keep the procedure source unchanged and record actual invocation namespace and context. Repeat a method/host getter with the applicable HTTP profile present and absent; report load/compile availability separately from runtime command presence.

## Class operands

With one disposable string data group, compare `class match [HTTP::method] equals group_name`, `class match [IP::client_addr] equals group_name`, a literal `{[HTTP::method]}` operand, and a group whose authored name contains `HTTP::method`. Observe the actual operand and result. A substring elsewhere in a command cannot establish a getter invocation.

## Result format

For every question include a unique case id, exact original source and source checksum, BIG-IP version/build, iRule and virtual/profile names, install acceptance, traffic input, observed worker/log output, return/error details and cleanup commands. Attach raw files and retain them on the branch. State unsupported commands and unobserved contexts explicitly. Run no `tcltest` here unless the actual appliance interpreter and matching original `tcltest.tcl`/`init.tcl` are available and identified.
