"""Observe exact command-substitution programs in original provisioned C Tcl."""

from pathlib import Path
import datetime
import hashlib
import json
import os
import subprocess


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


root = Path(__file__).resolve().parents[4]
out = Path(__file__).resolve().parent / "capture01"
out.mkdir(exist_ok=False)
cases = [
    ("braced-close", "set observed [list {]}]"),
    ("quoted-close", 'set observed [list "]"]'),
    ("comment-close", "set observed [list a; # ] ignored\nlist b]"),
    ("braced-open", "set observed [list {[}]"),
    ("concatenated", "set observed [list {]}]:[list z]"),
    ("quoted-semicolon", 'set observed [list ";"]'),
    ("multiple", "set observed [list a; list b]"),
    ("incomplete", "set observed [list {]"),
]
script = "puts [list PROVIDER [info patchlevel] [info library] [info nameofexecutable]]\n"
for label, body in cases:
    script += "set script [binary format H* " + body.encode().hex() + "]\n"
    script += "set code [catch {uplevel #0 $script} observed]\n"
    script += "binary scan $observed H* observedHex\n"
    script += 'puts "' + label + '|$code|$observedHex"\n'
(out / "input.tcl").write_text(script)
(out / "cases.json").write_text(json.dumps(cases, indent=2) + "\n")
(out / "capture.py").write_bytes(Path(__file__).read_bytes())
manifest = {
    "utc": datetime.datetime.now(datetime.timezone.utc).isoformat(),
    "question": "Which original brackets and separators belong to each substituted script?",
    "harness": "Direct original tclsh; exact input.tcl, no tcltest or replacement harness.",
    "driver_sha256": sha(Path(__file__)),
    "input_sha256": sha(out / "input.tcl"),
    "providers": [],
    "limitations": [
        "Scoped parser/result observations do not grant compiler Native entry or binding.",
        "No new Jim or F5 observation is claimed.",
    ],
}
for version in ["8.4.20", "8.5.19", "8.6.18", "9.0.4", "9.1.0"]:
    tree = root / "tmp" / ("tcl" + version)
    binary = tree / "unix/tclsh"
    paths = [p for p in tree.rglob("*") if p.is_file() and (
        p.suffix in (".c", ".h", ".tcl")
        or p.name in ("configure", "Makefile", "tclsh")
        or p.name.startswith("libtcl") and p.suffix in (".so", ".a", ".zip")
    )]
    before = {str(p.relative_to(tree)): sha(p) for p in paths}
    env = os.environ.copy()
    env.pop("TCL_LIBRARY", None)
    argv = [str(binary), str(out / "input.tcl")]
    start = datetime.datetime.now(datetime.timezone.utc).isoformat()
    result = subprocess.run(argv, env=env, capture_output=True, timeout=20)
    (out / (version + ".stdout")).write_bytes(result.stdout)
    (out / (version + ".stderr")).write_bytes(result.stderr)
    after = {str(p.relative_to(tree)): sha(p) for p in paths}
    manifest["providers"].append({
        "version": version,
        "started_utc": start,
        "binary": str(binary),
        "binary_sha256": sha(binary),
        "argv": argv,
        "exit_code": result.returncode,
        "completion": "process-exit",
        "source_before_sha256": before,
        "source_after_sha256": after,
        "changed_inputs": {n: [before.get(n), after.get(n)]
                           for n in before.keys() | after.keys() if before.get(n) != after.get(n)},
        "stdout_sha256": hashlib.sha256(result.stdout).hexdigest(),
        "stderr_sha256": hashlib.sha256(result.stderr).hexdigest(),
    })
    (out / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")
    assert result.returncode == 0 and not result.stderr and before == after
    print(version, result.stdout.decode().splitlines()[1:])
files = [p for p in out.iterdir() if p.is_file()]
(out / "SHA256SUMS").write_text("".join(sha(p) + "  " + p.name + "\n" for p in sorted(files)))
