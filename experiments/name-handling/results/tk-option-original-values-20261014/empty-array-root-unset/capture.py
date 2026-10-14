from pathlib import Path
import argparse, datetime, hashlib, json, os, re, subprocess
parser = argparse.ArgumentParser()
parser.add_argument("--out", required=True)
args = parser.parse_args()
output = Path(args.out)
output.mkdir(parents=True, exist_ok=False)
source = Path(__file__).with_name("source.tcl")
exe = Path("/workspace/tcl-lsp/tmp/tcl8.6.18/unix/tclsh")
paths = [source, exe, Path("/usr/lib/x86_64-linux-gnu/libtk8.6.so")]
paths.extend(exe.parent.glob("libtcl*.so"))
for directory in [exe.parent.parent / "library", Path("/usr/share/tcltk/tk8.6"), Path("/usr/lib/tcltk/x86_64-linux-gnu/tk8.6")]:
    paths.extend(path for path in directory.rglob("*") if path.is_file())
def snapshot():
    return {str(path): hashlib.sha256(path.read_bytes()).hexdigest() for path in paths}
def save(name, data):
    (output / name).write_text(json.dumps(data, indent=2) + "\n")
before = snapshot()
save("inputs-before.json", before)
env = os.environ.copy()
env["PATH"] = "/tmp/name-editor-host-deps-20261014/root/usr/bin:" + env.get("PATH", "")
env["TCLLIBPATH"] = "/usr/lib/tcltk/x86_64-linux-gnu"
env["TK_LIBRARY"] = "/usr/share/tcltk/tk8.6"
command = ["xvfb-run", "-a", str(exe), str(source)]
result = subprocess.run(command, env=env, capture_output=True, timeout=45)
(output / "source.tcl").write_bytes(source.read_bytes())
(output / "stdout").write_bytes(result.stdout)
(output / "stderr").write_bytes(result.stderr)
after = snapshot()
save("inputs-after.json", after)
libraries = {}
for line in result.stdout.decode("utf-8", "replace").splitlines():
    if line.startswith("LOADED_LIBRARY "):
        path = Path(line.removeprefix("LOADED_LIBRARY "))
        if path.is_file():
            libraries[str(path)] = hashlib.sha256(path.read_bytes()).hexdigest()
save("actual-loaded-libraries-sha256.json", libraries)
save("result.json", {"utc": datetime.datetime.now(datetime.UTC).isoformat(), "argv": command, "exit_code": result.returncode, "input_drift": [path for path in before.keys() | after.keys() if before.get(path) != after.get(path)], "actual_loaded_library_count": len(libraries), "scope": "Original source-built C Tcl 8.6.18 plus independently installed system Tk. Libraries identified from actual process mappings and hashed afterwards; complete loader-before closure is not certified. No other provider or compiler admission."})
raise SystemExit(result.returncode)
