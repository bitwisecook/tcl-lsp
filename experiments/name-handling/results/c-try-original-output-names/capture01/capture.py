"""Capture original C try handler output-name headers in a fresh destination."""
import argparse
import datetime
import hashlib
import json
import subprocess
from pathlib import Path

def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()
def now():
    return datetime.datetime.now(datetime.timezone.utc).isoformat()

parser = argparse.ArgumentParser()
parser.add_argument("--out", required=True, type=Path)
args = parser.parse_args()
here = Path(__file__).resolve().parent
repo = here.parents[3]
out = args.out.resolve()
out.mkdir(parents=True, exist_ok=False)
for name in ["probe.c", "capture.py"]:
    (out / name).write_bytes((here / name).read_bytes())

def command(argv, label):
    started = now()
    try:
        result = subprocess.run(argv, capture_output=True, timeout=30)
        code, timed_out, stdout, stderr = result.returncode, False, result.stdout, result.stderr
    except subprocess.TimeoutExpired as error:
        code, timed_out, stdout, stderr = None, True, error.stdout or b"", error.stderr or b""
    (out / (label + ".stdout")).write_bytes(stdout)
    (out / (label + ".stderr")).write_bytes(stderr)
    return dict(argv=argv, started_utc=started, finished_utc=now(), exit_code=code,
        timed_out=timed_out, stdout_sha256=sha(out / (label + ".stdout")),
        stderr_sha256=sha(out / (label + ".stderr")))

def provider(tree):
    return {path.relative_to(tree).as_posix(): sha(path)
        for directory in ["generic", "unix", "library"]
        for path in sorted((tree / directory).rglob("*"))
        if path.is_file() and (path.suffix in [".c", ".h", ".tcl", ".a", ".so", ".sh"]
            or path.name in ["tclsh", "Makefile", "configure", "config.log"])}

manifest = dict(started_utc=now(), source_sha256=sha(here / "probe.c"),
    driver_sha256=sha(here / "capture.py"), versions={},
    question="Does original C try retain and assign original handler list members on empty names, failed stores, fall-through and malformed lists?",
    scope="Original public C API Tcl_CreateInterp core only; no Tcl_Init or tcltest harness executed; no Runtime/VM.")
manifest["compiler"] = command(["cc", "--version"], "compiler")
for version in ["8.6.18", "9.0.4", "9.1.0"]:
    tree = repo / ("tmp/tcl" + version)
    major_minor = ".".join(version.split(".")[:2])
    executable = out / ("probe-" + version)
    row = dict(tree=str(tree), provider_before_sha256=provider(tree),
        harness="Core-only Tcl_CreateInterp; no init.tcl/tcltest execution. Original library/init.tcl retained in provider closure.")
    row["compile"] = command(["cc", "-std=c99", "-I" + str(tree / "generic"),
        "-I" + str(tree / "unix"), str(here / "probe.c"),
        str(tree / ("unix/libtcl" + major_minor + ".a")), "-lm", "-ldl", "-lpthread",
        "-lz", "-o", str(executable)], version + "-compile")
    if row["compile"]["exit_code"] == 0:
        row["executable_sha256"] = sha(executable)
        row["execution"] = command([str(executable)], version + "-execution")
        row["linked_libraries"] = command(["ldd", str(executable)], version + "-libraries")
        row["linked_library_sha256"] = {str(path.resolve()): sha(path.resolve())
            for word in (out / (version + "-libraries.stdout")).read_text().split()
            if word.startswith("/") and (path := Path(word)).is_file()}
    row["provider_after_sha256"] = provider(tree)
    row["provider_unchanged"] = row["provider_before_sha256"] == row["provider_after_sha256"]
    manifest["versions"][version] = row
    print(version, row["compile"]["exit_code"], row.get("execution", {}).get("exit_code"), row["provider_unchanged"])
manifest["finished_utc"] = now()
(out / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")
