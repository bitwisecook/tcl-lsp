#!/bin/bash
# Four-context parity probe, implementing the §E4 contract from
# docs/design/dialect-and-package-registry-redesign-bigip-evidence-review.md.
#
# Runs ON the appliance. Expects these files in /var/tmp (see gen-context-parity.py):
#   ctx_irule.conf  ctx_cli.conf  ctx_iapp.conf  ctx_iapp_svc.conf  ctx_host.tcl
#
# Contract points honoured:
#   - every create is preceded by an exact-name absence check; a collision
#     ABORTS rather than modifying an existing object
#   - an EXIT trap deletes only the exact probe names
#   - every delete is followed by an absence proof
#   - no probe object is attached to a virtual server
#   - `save sys config` is never run
set -u

R_IRULE=__tcl_lsp_probe_ctx_irule
S_CLI=__tcl_lsp_probe_ctx_cli
T_IAPP=__tcl_lsp_probe_ctx_iapp
V_IAPP=__tcl_lsp_probe_ctx_iapp_svc
MARK=TCLLSPPROBE

cleanup() {
  tmsh delete sys application service $V_IAPP >/dev/null 2>&1
  tmsh delete sys application template $T_IAPP >/dev/null 2>&1
  tmsh delete cli script $S_CLI            >/dev/null 2>&1
  tmsh delete ltm rule $R_IRULE            >/dev/null 2>&1
  tmsh delete ltm rule __tcl_lsp_probe_ctx_unknown >/dev/null 2>&1
}
trap cleanup EXIT

exists() { # exists <tmsh-object-path>
  tmsh list $1 >/dev/null 2>&1
}
require_absent() { # require_absent <path> <label>
  if exists "$1"; then
    echo "ABORT: $2 already exists ($1) — refusing to modify an existing object"
    exit 2
  fi
}
prove_absent() { # prove_absent <path> <label>
  if exists "$1"; then echo "CLEANUP-PROOF $2 = STILL PRESENT (FAIL)"
  else echo "CLEANUP-PROOF $2 = absent (ok)"; fi
}

echo "======== E4.1 inventory (no configuration change) ========"
tmsh show sys version 2>&1 | sed -n '1,12p'
echo "-- tmsh binary --"; tmsh --version 2>&1 | head -2
echo "-- host tcl binaries --"; ls -l /usr/bin/tclsh* 2>&1
echo "-- policy/system --"
echo "shell user: $(id -un) (uid $(id -u))"
tmsh list sys db systemauth.disablebash 2>&1 | tr -d '\n' | sed 's/  */ /g'; echo

echo
echo "======== E4.2 HostShellTcl (HOST ONLY — not a BIG-IP execution context) ========"
for t in tclsh8.4 tclsh8.5; do
  echo "-- /usr/bin/$t --"
  $t /var/tmp/ctx_host.tcl 2>&1
done

echo
echo "======== E4.3 TmshCliScript ========"
require_absent "cli script $S_CLI" "cli script"
tmsh load sys config merge file /var/tmp/ctx_cli.conf 2>&1 | grep -v '^Loading configuration\|^  /var/tmp'
if exists "cli script $S_CLI"; then
  tmsh run cli script $S_CLI 2>&1
  tmsh delete cli script $S_CLI >/dev/null 2>&1
else
  echo "CREATE FAILED"
fi
prove_absent "cli script $S_CLI" "cli script"

echo
echo "======== E4.4 TmmIRule (unattached rule; RULE_INIT runs once per TMM) ========"
require_absent "ltm rule $R_IRULE" "ltm rule"
SINCE=$(wc -l < /var/log/ltm)
tmsh load sys config merge file /var/tmp/ctx_irule.conf 2>&1 | grep -v '^Loading configuration\|^  /var/tmp'
if exists "ltm rule $R_IRULE"; then
  sleep 1
  echo "-- attached to any virtual server? (E4.7) --"
  if tmsh list ltm virtual one-line 2>/dev/null | grep -q "$R_IRULE"; then
    echo "ATTACHED (contract violation)"; else echo "not attached (ok)"; fi
  tail -n +$SINCE /var/log/ltm | grep -o "$MARK|.*" | sort -u
  tmsh delete ltm rule $R_IRULE >/dev/null 2>&1
else
  echo "CREATE FAILED"
fi
prove_absent "ltm rule $R_IRULE" "ltm rule"

echo
echo "======== E4.4b command resolution time (TmmIRule) ========"
U=__tcl_lsp_probe_ctx_unknown
require_absent "ltm rule $U" "unknown-cmd rule"
OUT=$(tmsh load sys config merge file /var/tmp/ctx_unknown_cmd.conf 2>&1)
if exists "ltm rule $U"; then
  echo "LOADED — undefined command inside catch is tolerated at load"
  tmsh delete ltm rule $U >/dev/null 2>&1
else
  echo "REJECTED at load: $(echo "$OUT" | grep -oE 'error: \[[^]]*\]' | head -1)"
  echo "=> iRules resolve command names at rule load, even inside catch."
fi
prove_absent "ltm rule $U" "unknown-cmd rule"

echo
echo "======== E4.5 IAppImplementation ========"
require_absent "sys application template $T_IAPP" "iApp template"
require_absent "sys application service $V_IAPP" "iApp service"
SINCE=$(wc -l < /var/log/ltm)
tmsh load sys config merge file /var/tmp/ctx_iapp.conf 2>&1 | grep -v '^Loading configuration\|^  /var/tmp'
tmsh load sys config merge file /var/tmp/ctx_iapp_svc.conf 2>&1 | grep -v '^Loading configuration\|^  /var/tmp'
if exists "sys application service $V_IAPP"; then
  # a merge creates the service without running its implementation
  tmsh modify sys application service $V_IAPP execute-action definition 2>&1 | head -5
  sleep 2
  tail -n +$SINCE /var/log/ltm | grep -o "$MARK|.*" | sort -u
  tmsh delete sys application service $V_IAPP  >/dev/null 2>&1
  tmsh delete sys application template $T_IAPP >/dev/null 2>&1
else
  echo "CREATE FAILED"
fi
prove_absent "sys application service $V_IAPP" "iApp service"
prove_absent "sys application template $T_IAPP" "iApp template"

echo
echo "======== E4.6 APL presentation contexts ========"
if command -v tmsh >/dev/null && tmsh help sys application 2>/dev/null | grep -qi 'render'; then
  echo "a presentation renderer appears available — NOT exercised by this run"
else
  echo "IAppPresentationApl          = Unknown (no non-interactive renderer exercised)"
  echo "IAppPresentationTclCallback  = Unknown (no non-interactive renderer exercised)"
fi
echo "(the IAppImplementation result above must NOT be copied into either row)"

echo
echo "======== E4.7 final state ========"
echo "residual probe objects:"
echo "  ltm rule            : $(tmsh list ltm rule one-line 2>/dev/null | grep -c __tcl_lsp_probe_)"
echo "  cli script          : $(tmsh list cli script one-line 2>/dev/null | grep -c __tcl_lsp_probe_)"
echo "  application template: $(tmsh list sys application template one-line 2>/dev/null | grep -c __tcl_lsp_probe_)"
echo "  application service : $(tmsh list sys application service one-line 2>/dev/null | grep -c __tcl_lsp_probe_)"
echo "save sys config: NOT RUN"
