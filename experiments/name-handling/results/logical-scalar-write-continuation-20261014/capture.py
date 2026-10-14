from pathlib import Path
import argparse, datetime, hashlib, json, os, subprocess
parser = argparse.ArgumentParser()
parser.add_argument("--repo", type=Path, required=True)
parser.add_argument("--out", type=Path, required=True)
parser.add_argument("--probe", choices=["modern", "legacy"], default="modern")
args = parser.parse_args()
args.out.mkdir(parents=True, exist_ok=False)
folder = "oracles-modern-trace-api" if args.probe == "modern" else "oracles"
probe = Path(__file__).with_name(folder) / "scalar-original-name-mutations.tcl"
providers = {"C" + version: args.repo / "tmp" / ("tcl" + version) / "unix/tclsh" for version in ["8.4.20", "8.5.19", "8.6.18", "9.0.4", "9.1.0"]}
jim = Path(os.environ.get("TCL_LSP_JIMSH", str(args.repo / "tmp/jimtcl-oracle/jimsh")))
providers["Jim"] = jim
records = []
for label, executable in providers.items():
    if not executable.is_file():
        records.append({"provider": label, "status": "not observed", "executable": str(executable)})
        continue
    paths = [probe, executable.resolve()]
    paths.extend(executable.parent.glob("libtcl*.so"))
    library = executable.parent.parent / "library"
    if library.is_dir():
        paths.extend(path for path in library.rglob("*") if path.is_file())
    def snapshot():
        return {str(path): hashlib.sha256(path.read_bytes()).hexdigest() for path in paths}
    before = snapshot()
    (args.out / (label + ".inputs-before.json")).write_text(json.dumps(before, indent=2) + "\n")
    env = os.environ.copy()
    if label.startswith("C") and library.is_dir():
        env["TCL_LIBRARY"] = str(library)
    result = subprocess.run([str(executable), str(probe)], cwd=args.repo, env=env, capture_output=True, timeout=30)
    (args.out / (label + ".stdout")).write_bytes(result.stdout)
    (args.out / (label + ".stderr")).write_bytes(result.stderr)
    after = snapshot()
    (args.out / (label + ".inputs-after.json")).write_text(json.dumps(after, indent=2) + "\n")
    records.append({"provider": label, "executable": str(executable), "exit_code": result.returncode, "input_drift": [path for path in before.keys() | after.keys() if before.get(path) != after.get(path)]})
(args.out / "observations.json").write_text(json.dumps({"utc": datetime.datetime.now(datetime.UTC).isoformat(), "records": records, "scope": "Direct original interpreter programs. Matching original Tcl release library selected when present; tcltest not used. No compiler/Native/F5 or complete loader closure certification."}, indent=2) + "\n")
