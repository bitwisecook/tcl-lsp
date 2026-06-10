"""Capture a profile by running a script under tclsh with execution tracing.

This is the *precise* profile source: it drives a real ``tclsh`` with
``trace add execution source enterstep`` — the same passive-observation
hook the BIG-IP rule-profiler is modelled on, and the one this repo's
debugger already uses — and recovers the source **line** of every executed
command via ``info frame``.  That yields per-line counts the branch
reorderer can attribute exactly (unlike an F5 log, which only has command
names).

The capture is best-effort: it returns an empty :class:`ProfileData` (and
logs at debug level) if no ``tclsh`` is found or the script cannot run.
Line attribution is most reliable for top-level / event-handler code; the
trace records whatever line ``info frame`` reports for each command.

Note: a raw iRule (``when EVENT { … }``) is not directly runnable under a
stock ``tclsh`` — ``when`` is not a core command.  Use this on plain Tcl /
proc bodies, or stub the iRule commands; use the F5-log importer for live
iRule traffic.
"""

from __future__ import annotations

import json
import logging
import shutil
import subprocess
import tempfile
from pathlib import Path

from .profile_data import ProfileData

log = logging.getLogger(__name__)

#: tclsh executables to try, in order of preference.
_TCLSH_CANDIDATES = ("tclsh", "tclsh9.0", "tclsh8.6", "tclsh8.5")

# Tcl-side harness.  Installs an ``enterstep`` trace on ``source``, tallies
# per-line and per-command counts, then prints them as JSON.  Its own procs
# are namespaced ``::__pgo*`` and skipped when walking the frame stack.
_HARNESS = r"""
array set ::__pgo_lines {}
array set ::__pgo_cmds {}

proc ::__pgo_step {args} {
    set depth [info frame]
    for {set i $depth} {$i >= 1} {incr i -1} {
        if {[catch {info frame $i} fi]} continue
        if {[dict exists $fi proc]} {
            set pn [dict get $fi proc]
            if {[string match "::__pgo*" $pn]} continue
        }
        if {[dict exists $fi type] && [dict get $fi type] eq "source"
            && [dict exists $fi line]} {
            incr ::__pgo_lines([dict get $fi line])
            if {[dict exists $fi cmd]} {
                set c [lindex [dict get $fi cmd] 0]
                if {$c ne ""} { incr ::__pgo_cmds($c) }
            }
            break
        }
    }
}

set ::__pgo_script [lindex $argv 0]
trace add execution source enterstep ::__pgo_step
catch {source $::__pgo_script}
trace remove execution source enterstep ::__pgo_step

proc ::__pgo_emit {} {
    set lparts {}
    foreach {k v} [array get ::__pgo_lines] { lappend lparts "\"$k\":$v" }
    set cparts {}
    foreach {k v} [array get ::__pgo_cmds] {
        set ek [string map [list \\ \\\\ \" \\\"] $k]
        lappend cparts "\"$ek\":$v"
    }
    puts "{\"lines\":{[join $lparts ,]},\"commands\":{[join $cparts ,]}}"
}
::__pgo_emit
"""


def find_tclsh() -> str | None:
    """Return the path to a usable ``tclsh``, or ``None`` if none is found."""
    for name in _TCLSH_CANDIDATES:
        path = shutil.which(name)
        if path:
            return path
    return None


def capture_profile(
    script_path: str | Path,
    *,
    tclsh: str | None = None,
    timeout: float = 30.0,
) -> ProfileData:
    """Run *script_path* under tclsh and return a :class:`ProfileData`.

    Returns an empty profile (logged at debug level) when tracing is not
    possible — no ``tclsh``, a run error, or unparseable output.
    """
    exe = tclsh or find_tclsh()
    if exe is None:
        log.debug("pgo capture: no tclsh found on PATH")
        return ProfileData(source="tclsh-trace")

    script_path = Path(script_path)
    with tempfile.NamedTemporaryFile("w", suffix=".tcl", delete=False, encoding="utf-8") as fh:
        fh.write(_HARNESS)
        harness_path = fh.name
    try:
        proc = subprocess.run(
            [exe, harness_path, str(script_path)],
            capture_output=True,
            text=True,
            timeout=timeout,
            check=False,
        )
    except (OSError, subprocess.SubprocessError) as exc:
        log.debug("pgo capture: tclsh run failed: %s", exc)
        return ProfileData(source="tclsh-trace")
    finally:
        try:
            Path(harness_path).unlink()
        except OSError:
            pass  # best-effort temp-file cleanup; ignore permission/FS errors

    # The JSON line is the harness' final stdout line.
    payload = proc.stdout.strip().splitlines()
    for line in reversed(payload):
        line = line.strip()
        if not line.startswith("{"):
            continue
        try:
            data = json.loads(line)
        except json.JSONDecodeError:
            continue
        line_counts = {int(k): int(v) for k, v in data.get("lines", {}).items()}
        command_counts = {str(k): int(v) for k, v in data.get("commands", {}).items()}
        return ProfileData(
            line_counts=line_counts,
            command_counts=command_counts,
            source="tclsh-trace",
        )
    log.debug("pgo capture: no parseable profile output from tclsh")
    return ProfileData(source="tclsh-trace")
