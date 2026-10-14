"""Capture the retained dictionary scripts with original interpreter inputs."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess

parser = argparse.ArgumentParser()
parser.add_argument("--repo", type=Path, required=True)
parser.add_argument("--out", type=Path, required=True)
parser.add_argument("--combined", action="store_true")
args = parser.parse_args()
args.out.mkdir(parents=True, exist_ok=False)
retained = Path(__file__).parent
scripts = [retained / "oracles/source.tcl"] if args.combined else sorted((retained / "oracles-isolated").glob("*.tcl"))
providers = [("C" + version, args.repo.resolve() / "tmp" / ("tcl" + version) / "unix/tclsh") for version in ["8.4.20", "8.5.19", "8.6.18", "9.0.4", "9.1.0"]]
providers.append(("Jim", args.repo.resolve() / "tmp/jimtcl-oracle/jimsh"))
for label, exe in providers:
    if not exe.is_file():
        (args.out / (label + ".unavailable.json")).write_text(json.dumps({"exe": str(exe), "observed": False}, indent=2) + "\n")
        continue
    library = exe.parent.parent / "library"
    for script in scripts:
        paths = [script, exe]
        paths.extend(exe.parent.glob("libtcl*.so"))
        if library.is_dir():
            paths.extend(path for path in library.rglob("*") if path.is_file())
        def snapshot():
            return {str(path): hashlib.sha256(path.read_bytes()).hexdigest() for path in paths}
        before = snapshot()
        env = os.environ.copy()
        if library.is_dir():
            env["TCL_LIBRARY"] = str(library)
        command = [str(exe), str(script)]
        result = subprocess.run(command, env=env, capture_output=True, timeout=30)
        after = snapshot()
        prefix = label + "." + script.stem
        (args.out / (prefix + ".source.tcl")).write_bytes(script.read_bytes())
        (args.out / (prefix + ".stdout")).write_bytes(result.stdout)
        (args.out / (prefix + ".stderr")).write_bytes(result.stderr)
        for kind, value in [("inputs-before", before), ("inputs-after", after)]:
            (args.out / (prefix + "." + kind + ".json")).write_text(json.dumps(value, indent=2) + "\n")
        (args.out / (prefix + ".result.json")).write_text(json.dumps({"provider": label, "argv": command, "exit_code": result.returncode, "input_drift": [path for path in before.keys() | after.keys() if before.get(path) != after.get(path)], "original_release_library": str(library) if library.is_dir() else None, "scope": "Original script, interpreter, adjacent Tcl library and original release library only. No tcltest, complete loader closure, hosted F5, compiler, Native or Normal admission."}, indent=2) + "\n")
