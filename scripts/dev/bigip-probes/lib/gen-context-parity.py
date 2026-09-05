#!/usr/bin/env python3
"""Generate the four-context parity probes from suites/10-context-parity.cases.

One case list, four wrappers: an iRule (TmmIRule), a cli script (TmshCliScript),
an iApp template+service (IAppImplementation), and a plain tclsh script
(HostShellTcl).  Emitting all four from one source is the point: any difference
in the transcripts is a real context difference, not a difference in what was
asked.

Usage: gen-context-parity.py <cases-file> <outdir>
"""

import sys
from pathlib import Path

cases = []
with open(sys.argv[1], encoding="utf-8") as handle:
    for line in handle:
        if line.startswith("#") or not line.strip():
            continue
        cid, cat, src = line.rstrip("\n").split("\t")
        # Un-escape here rather than in Tcl: the case list is emitted inside {...},
        # which performs no substitution, so the file must already hold real braces
        # and real newlines. $ and [ deliberately survive to eval time.
        src = (
            src.replace("\\{", "{")
            .replace("\\}", "}")
            .replace('\\"', '"')
            .replace("\\n", "\n")
        )
        cases.append((cid, cat, src))
out = Path(sys.argv[2])
out.mkdir(parents=True, exist_ok=True)

# Shared Tcl body. $EMIT is replaced per context with a one-argument logger.
BODY = r"""
set ::probe_cases [list \
%CASES%]
set ::acc {}
foreach {cid cat src} $::probe_cases {
    set ::m unset
    set rc [catch {uplevel #0 $src} v]
    regsub -all {[\n\r|]} $v " " v
    if {[string length $v] > 60} { set v "[string range $v 0 59]..." }
    lappend ::acc "$cid rc=$rc v=($v)"
}
set tpl UNSET
if {[info exists tcl_patchLevel]} { set tpl $tcl_patchLevel }
# Built dynamically: the iRule compiler rejects a literal reference to an
# undefined command at rule load, even inside catch (see ctx_unknown_cmd.conf).
set tvcmd tmsh::version
if {[catch {eval $tvcmd} tv]} { set tv n/a }
lappend ::acc "REPORTED patchlevel=[info patchlevel] tclversion=[info tclversion] tcl_patchLevel=$tpl tmshversion=$tv"
lappend ::acc "REPORTED ncommands=[llength [info commands]]"
set plat {}
foreach k [lsort [array names tcl_platform]] { lappend plat "$k=$tcl_platform($k)" }
lappend ::acc "REPORTED tcl_platform([llength [array names tcl_platform]]) [join $plat ,]"
set i 0
foreach chunkstart [list 0 8 16 24 32 40] {
    set part [lrange $::acc $chunkstart [expr {$chunkstart + 7}]]
    if {[llength $part]} { EMIT "TCLLSPPROBE|CTXNAME|$i| [join $part " ;; "]" ; incr i }
}
"""


def body_for(ctxname, emit):
    lit = "".join(f"    {{{c}}} {{{k}}} {{{s}}} \\\n" for c, k, s in cases)
    b = BODY.replace("%CASES%", lit).replace("CTXNAME", ctxname)
    return b.replace("EMIT ", emit + " ")


# 1. iRule — RULE_INIT, logs to /var/log/ltm
(out / "ctx_irule.conf").write_text(
    "ltm rule __tcl_lsp_probe_ctx_irule {\nwhen RULE_INIT {"
    + body_for("TmmIRule", "log local0.")
    + "}\n}\n",
    encoding="utf-8",
)

# 2. tmsh cli script — puts to stdout
(out / "ctx_cli.conf").write_text(
    "cli script __tcl_lsp_probe_ctx_cli {\nproc script::run {} {"
    + body_for("TmshCliScript", "puts")
    + "}\n}\n",
    encoding="utf-8",
)

# 3. iApp implementation — tmsh::log err (info level never reaches /var/log/ltm)
(out / "ctx_iapp.conf").write_text(
    "sys application template __tcl_lsp_probe_ctx_iapp {\n  actions {\n    definition {\n"
    "      implementation {"
    + body_for("IAppImplementation", "tmsh::log err")
    + "      }\n      presentation {\n      }\n    }\n  }\n}\n",
    encoding="utf-8",
)
(out / "ctx_iapp_svc.conf").write_text(
    "sys application service __tcl_lsp_probe_ctx_iapp_svc {\n"
    "  template __tcl_lsp_probe_ctx_iapp\n}\n",
    encoding="utf-8",
)

# 4. host tclsh control
(out / "ctx_host.tcl").write_text(body_for("HostShellTcl", "puts"), encoding="utf-8")
print(f"wrote 5 wrappers for {len(cases)} cases -> {out}")
