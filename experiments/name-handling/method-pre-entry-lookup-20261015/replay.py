from pathlib import Path
import argparse, hashlib, json, os, subprocess

p = argparse.ArgumentParser()
p.add_argument("--root", type=Path, required=True)
p.add_argument("--out", type=Path, required=True)
a = p.parse_args()
here = Path(__file__).resolve().parent
a.out.mkdir(parents=True, exist_ok=False)
manifest = json.loads((here / "manifest.json").read_text())
sha = lambda path: hashlib.sha256(path.read_bytes()).hexdigest()
assert sha(here / "probe.tcl") == manifest["probe_sha256"]
for original in manifest["providers"]:
    version = original["version"]
    tree = a.root / "tmp" / ("jimtcl-oracle" if version == "jim" else "tcl" + version)
    executable = tree / ("jimsh" if version == "jim" else "unix/tclsh")
    assert sha(executable) == original["executable_sha256"]
    environment = dict(os.environ)
    environment.pop("TCLLIBPATH", None)
    environment.pop("TCL_LIBRARY", None)
    if version != "jim":
        assert sha(tree / "library/init.tcl") == original["original_init_sha256"]
        environment["TCL_LIBRARY"] = str(tree / "library")
    result = subprocess.run([str(executable), str(here / "probe.tcl")], env=environment, capture_output=True, timeout=30)
    assert result.returncode == original["exit_code"]
    for channel in ["stdout", "stderr"]:
        data = getattr(result, channel)
        (a.out / (version + "." + channel)).write_bytes(data)
        assert data == (here / (version + "." + channel)).read_bytes()
        assert hashlib.sha256(data).hexdigest() == original[channel + "_sha256"]
    assert sha(executable) == original["executable_sha256"]
    if version != "jim":
        assert sha(tree / "library/init.tcl") == original["original_init_sha256"]
    print(version, "PASS")
