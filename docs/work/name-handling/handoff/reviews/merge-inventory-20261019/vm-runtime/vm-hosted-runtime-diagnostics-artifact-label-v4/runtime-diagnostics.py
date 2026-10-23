"""Work-only hosted original noTom16 diagnostics; never a release gate waiver."""

import argparse
import datetime
import hashlib
import importlib.util
import json
import os
import re
import resource
import shutil
import subprocess
import sys
from pathlib import Path

EXECUTOR_SHA = "88125bd5267ac897dda4b5f665872c5ae0ab9984b0f42a41edeadf718873d811"
CAP = 20 * 1024**2
FLOOR = 2 * 1024**3
ORIGINAL16 = [
    "cmd_error::tests::eight_four_catch_takes_no_options_variable",
    "cmd_oo::tests::object_command_role_walk_does_not_revisit_earlier_roles",
    "cmd_regex::tests::regsub_literal_empty_pattern_walks_the_releases_characters",
    "cmd_trace::tests::the_trace_gate_reads_the_pinned_profile_not_the_release_name",
    "codegen_abi::tests::guarded_intrinsic_string_length_identity_and_count_follow_runtime_version",
    "codegen_abi::tests::typed_value_reads_report_overflow_and_non_numbers_like_c",
    "env_trace::tests::a_nul_cuts_the_name_and_value_the_host_keeps",
    "env_trace::tests::an_element_alias_reaches_the_host_only_from_9_0",
    "env_trace::tests::child_interpreters_share_the_host_and_safe_ones_have_no_env",
    "env_trace::tests::every_kind_of_element_write_and_unset_reaches_the_host",
    "env_trace::tests::reads_refresh_from_the_host_and_fail_when_it_has_no_such_variable",
    "env_trace::tests::rebinding_the_host_moves_the_trace_to_the_new_host",
    "env_trace::tests::the_home_directory_follows_env_home",
    "env_trace::tests::the_trace_is_invisible_and_runs_after_the_arrays_script_traces",
    "env_trace::tests::variables_that_cannot_reach_env_keep_the_untraced_fast_path",
    "env_trace::tests::unsetting_the_whole_array_leaves_the_host_alone_and_detaches_it",
]


def require(x, m):
    if not x:
        raise RuntimeError(m)


def sha(p):
    with Path(p).open("rb") as f:
        return hashlib.file_digest(f, "sha256").hexdigest()


def unique(pairs):
    d = {}
    for k, v in pairs:
        require(k not in d, "duplicate JSON key")
        d[k] = v
    return d


def read(s):
    return json.loads(s, object_pairs_hook=unique)


def save(p, x):
    Path(p).write_text(json.dumps(x, indent=2, sort_keys=True) + "\n")


def git(*args):
    return subprocess.check_output(["git", *args], text=True).strip()


def snapshot():
    require(git("rev-parse", "HEAD") == args.sha, "checkout SHA changed")
    clean = git("status", "--porcelain=v1", "--untracked-files=all")
    require(not clean, "checkout dirty")
    files = {}
    for name in subprocess.check_output(["git", "ls-files", "-z"]).split(b"\0"):
        if not name:
            continue
        p = Path(os.fsdecode(name))
        files[str(p)] = {
            "sha256": sha(p),
            "symlink": os.readlink(p) if p.is_symlink() else None,
        }
    return {
        "head": args.sha,
        "tree": git("rev-parse", "HEAD^{tree}"),
        "clean": True,
        "tracked": files,
    }


a = argparse.ArgumentParser(description=__doc__)
a.add_argument("--sha", required=True)
a.add_argument("--out", required=True, type=Path)
args = a.parse_args()
require(re.fullmatch("[0-9a-f]{40}", args.sha), "full exact workSHA required")
require(
    os.environ.get("GITHUB_EVENT_NAME") == "workflow_dispatch"
    and os.environ.get("GITHUB_REF") == "refs/heads/work",
    "work-only manual diagnostic required",
)
require(
    args.sha == os.environ.get("GITHUB_SHA"),
    "requested SHA must equal dispatched work commit",
)
require(not args.out.exists(), "fresh output required")
args.out.mkdir(parents=True)
out = args.out.resolve()
root = Path.cwd().resolve()
require(not out.is_relative_to(root), "proof must be outside checkout")
resource.setrlimit(resource.RLIMIT_CORE, (0, 0))
results = []
remaining = CAP
before = None
artifact = None
exit_code = 2
try:
    require(
        os.environ.get("TCL_RUNTIME_EXPECT_NO_TOMMATH") == "1"
        and "TCL_TOMMATH_DIR" not in os.environ,
        "explicit supportedabsence environment required",
    )
    require(
        not (root / "tmp/tcl9.0.4/libtommath/tommath.h").exists(),
        "actual fallback source present",
    )
    target = Path(os.environ["CARGO_TARGET_DIR"]).resolve()
    require(not target.is_relative_to(root), "target must be outside checkout")
    require(not target.exists(), "fresh dedicated target must not exist before Cargo")
    executor = Path(__file__).with_name("runtime-diagnostics-exec.py")
    require(sha(executor) == EXECUTOR_SHA, "executor pin changed")
    cargo_path = shutil.which("cargo")
    rustc_path = shutil.which("rustc")
    if cargo_path is None or rustc_path is None:
        raise RuntimeError("Cargo or rustc is absent")
    rustup_path = shutil.which("rustup")
    if rustup_path is None:
        raise RuntimeError("hosted rustup toolchain resolver absent")
    cargo = Path(
        subprocess.check_output([rustup_path, "which", "cargo"], text=True).strip()
    ).resolve()
    rustc = Path(
        subprocess.check_output([rustup_path, "which", "rustc"], text=True).strip()
    ).resolve()
    require(
        cargo.name == "cargo" and rustc.name == "rustc",
        "actual toolchain executables required",
    )
    before = snapshot()
    save(out / "source-before.json", before)
    save(
        out / "job.json",
        {
            "utc": datetime.datetime.now(datetime.UTC).isoformat(),
            "python_version": sys.version,
            "toolchain_proxies": {
                name: {
                    "path": path,
                    "realpath": str(Path(path).resolve()),
                    "sha256": sha(Path(path)),
                }
                for name, path in [
                    ("cargo", cargo_path),
                    ("rustc", rustc_path),
                    ("rustup", rustup_path),
                ]
            },
            "diagnostic_only": True,
            "requested_sha": args.sha,
            "job": {
                k: os.environ.get(k)
                for k in [
                    "GITHUB_RUN_ID",
                    "GITHUB_RUN_ATTEMPT",
                    "GITHUB_JOB",
                    "GITHUB_REPOSITORY",
                    "GITHUB_WORKFLOW_REF",
                    "GITHUB_SHA",
                    "GITHUB_REF",
                ]
            },
            "tools": {str(p): sha(p) for p in [cargo, rustc, executor, Path(__file__)]},
            "required_environment": {
                k: os.environ.get(k)
                for k in [
                    "TCL_RUNTIME_EXPECT_NO_TOMMATH",
                    "CARGO_TARGET_DIR",
                    "CARGO_BUILD_JOBS",
                    "CARGO_INCREMENTAL",
                    "CARGO_PROFILE_DEV_DEBUG",
                ]
            },
            "selectors": ORIGINAL16,
            "limits": {
                "runtime_seconds": 180,
                "build_seconds": 3600,
                "aggregate_log_bytes": CAP,
                "disk_floor_bytes": FLOOR,
            },
        },
    )
    sp = importlib.util.spec_from_file_location("owned_executor", executor)
    if sp is None or sp.loader is None:
        raise RuntimeError("pinned executor loader absent")
    owned = importlib.util.module_from_spec(sp)
    sp.loader.exec_module(owned)

    def execute(argv, prefix, pin, seconds):
        global remaining
        require(remaining > 0, "aggregate log budget exhausted")
        r = owned.run(
            argv,
            cwd=root,
            env=os.environ.copy(),
            stdout_path=str(prefix) + ".stdout",
            stderr_path=str(prefix) + ".stderr",
            receipt_path=str(prefix) + ".process.json",
            artifact_path=pin,
            deadline_seconds=seconds,
            cap_bytes=remaining,
            disk_floor_bytes=FLOOR,
            disk_paths=(root, "/tmp", out, target),
        )
        n = r.get("retained_bytes")
        require(isinstance(n, int) and 0 <= n <= remaining, "invalid log accounting")
        remaining -= n
        save(
            out / "aggregate-log.json", {"cap_bytes": CAP, "remaining_bytes": remaining}
        )
        return r

    def lifecycle(r):
        return (
            r["complete"]
            and r["stop_reason"] is None
            and r["group_empty_before_parent_reap"]
            and r["timer_armed_before_spawn"]
            and r["artifact_unchanged"]
        )

    argv = [
        str(cargo),
        "test",
        "--locked",
        "--manifest-path",
        "runtime/rust/Cargo.toml",
        "--lib",
        "--no-run",
        "--message-format=json",
        "-j",
        "2",
    ]
    save(out / "build.argv.json", argv)
    r = execute(argv, out / "build", cargo, 3600)
    require(lifecycle(r) and r["exit_code"] == 0, "build not complete success")
    events = [
        read(x) for x in (out / "build.stdout").read_text().splitlines() if x.strip()
    ]
    save(out / "build.events.json", events)
    finished = [x for x in events if x.get("reason") == "build-finished"]
    require(
        len(finished) == 1 and finished[0]["success"] is True,
        "Cargo terminal event missing",
    )
    stderr = (out / "build.stderr").read_text(errors="replace")
    warnings = [
        x
        for x in events
        if x.get("reason") == "compiler-message"
        and x.get("message", {}).get("level") in ["warning", "error"]
    ]
    require(
        not warnings and not re.search(r"\bwarning:", stderr), "strict0W build required"
    )
    scripts = [
        x
        for x in events
        if x.get("reason") == "build-script-executed"
        and "tcl-runtime" in x.get("package_id", "")
    ]
    require(len(scripts) == 1, "actual buildscript event absent/ambiguous")
    bs = scripts[0]
    od = Path(bs["out_dir"]).resolve()
    require(
        od.is_relative_to(target) and "have_tommath" not in bs.get("cfgs", []),
        "actual backend not absent",
    )
    cap = od / "tommath-capability.json"
    require(
        cap.is_file()
        and read(cap.read_text()) == {"detected": False, "intentional_absence": True},
        "fresh intentionalabsence capability missing",
    )
    raw = (od.parent / "output").read_text()
    require(
        "tommath_capability=absent-intentional" in raw
        and "cargo:rustc-cfg=have_tommath" not in raw,
        "raw buildscript capability mismatch",
    )
    save(
        out / "capability.json",
        {
            "out_dir": str(od),
            "capability": read(cap.read_text()),
            "sha256": sha(cap),
            "build_script": bs,
        },
    )
    (out / "build-script-output.txt").write_text(raw)
    candidates = [
        x
        for x in events
        if x.get("reason") == "compiler-artifact"
        and x.get("target", {}).get("name") == "tcl_runtime"
        and set(x.get("target", {}).get("kind", [])) == {"rlib", "cdylib", "staticlib"}
        and x.get("profile", {}).get("test") is True
        and x.get("executable")
        and Path(x.get("manifest_path", "")).resolve()
        == root / "runtime/rust/Cargo.toml"
    ]
    require(len(candidates) == 1, "fresh exact CargoJSON artifact absent/ambiguous")
    event = candidates[0]
    require(
        event.get("fresh") is False,
        "fresh Cargo compilation required; cached artifact refused",
    )
    exe = Path(event["executable"]).resolve()
    require(
        exe.is_file() and exe.is_relative_to(target), "artifact outside current target"
    )
    artifact = {"path": str(exe), "sha256": sha(exe), "event": event}
    save(out / "artifact.json", artifact)
    r = execute([str(exe), "--list", "--format", "terse"], out / "roster", exe, 180)
    require(lifecycle(r) and r["exit_code"] == 0, "roster failure")
    names = re.findall(
        r"^(.+): test$", (out / "roster.stdout").read_text(), re.MULTILINE
    )
    require(
        len(names) == len(set(names)) and all(names.count(x) == 1 for x in ORIGINAL16),
        "original selectors missing/duplicate",
    )
    save(out / "selected.json", {"original16": ORIGINAL16, "actual_roster": names})
    for i, name in enumerate(ORIGINAL16, 1):
        require(sha(exe) == artifact["sha256"], "artifact changed")
        require(
            git("rev-parse", "HEAD") == args.sha
            and not git("status", "--porcelain=v1", "--untracked-files=all"),
            "source/clean guard failed",
        )
        prefix = out / (f"{i:02d}-" + name.replace(":", "_"))
        argv = [str(exe), name, "--exact", "--nocapture", "--test-threads=1"]
        save(str(prefix) + ".argv.json", argv)
        r = execute(argv, prefix, exe, 180)
        text = (
            Path(str(prefix) + ".stdout").read_text(errors="replace")
            + "\n"
            + Path(str(prefix) + ".stderr").read_text(errors="replace")
        )
        starts = [
            int(x) for x in re.findall(r"^running (\d+) tests?$", text, re.MULTILINE)
        ]
        printed = re.findall(r"^test ([^ ]+) \.\.\.", text, re.MULTILINE)
        summary = re.findall(
            r"^test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;",
            text,
            re.MULTILINE,
        )
        status = "PROTOCOL_FAILURE"
        runtime_warnings = re.findall(r"^warning:.*", text, re.MULTILINE)
        if lifecycle(r) and starts == [1] and printed == [name]:
            if r["exit_code"] in [0, 101] and len(summary) == 1:
                s, p, f, ignored = summary[0]
                if (s, int(p), int(f), int(ignored), r["exit_code"]) in [
                    ("ok", 1, 0, 0, 0),
                    ("FAILED", 0, 1, 0, 101),
                ]:
                    status = "PASS" if r["exit_code"] == 0 else "FAIL"
            elif isinstance(r["exit_code"], int) and r["exit_code"] < 0 and not summary:
                status = "SIGNAL"
        if runtime_warnings:
            status = "PROTOCOL_FAILURE"
        row = {
            "index": i,
            "selector": name,
            "status": status,
            "raw_exit_code": r["exit_code"],
            "start_counts": starts,
            "printed_names": printed,
            "raw_summary": summary,
            "runtime_warnings": runtime_warnings,
            "process": r,
            "diagnostic_only": True,
        }
        results.append(row)
        save(str(prefix) + ".result.json", row)
        save(out / "results.json", results)
        print(f"DIAGNOSTIC {i}/16 {status}", flush=True)
        require(lifecycle(r), "lifecycle stop; no unsafe continuation")
        require(remaining > 0, "aggregate log cap exhausted")
    require(len(results) == 16, "original16 incomplete")
    after = snapshot()
    save(out / "source-after.json", after)
    require(
        before == after and sha(exe) == artifact["sha256"],
        "final source/artifact drift",
    )
    save(
        out / "final.json",
        {
            "diagnostic_only": True,
            "count": 16,
            "statuses": {
                k: sum(x["status"] == k for x in results)
                for k in ["PASS", "FAIL", "SIGNAL", "PROTOCOL_FAILURE"]
            },
            "actual_source_sha": args.sha,
            "source_clean_before_after": True,
            "source_stable": True,
            "artifact_stable": True,
            "finished_utc": datetime.datetime.now(datetime.UTC).isoformat(),
            "release_admission": False,
        },
    )
    exit_code = 0 if all(x["status"] == "PASS" for x in results) else 1
except (
    RuntimeError,
    OSError,
    ValueError,
    subprocess.SubprocessError,
    ImportError,
    KeyError,
    TypeError,
) as error:
    failure = {
        "protocol_failure": True,
        "reason": repr(error),
        "results_observed": len(results),
        "diagnostic_only": True,
    }
    try:
        failure["after"] = snapshot()
        failure["source_stable"] = before is not None and before == failure["after"]
    except (RuntimeError, OSError, ValueError, subprocess.SubprocessError) as guard:
        failure["final_guard_error"] = repr(guard)
    save(out / "failure.json", failure)
    print("DIAGNOSTIC STOP " + repr(error), file=sys.stderr)
    exit_code = 2
sys.exit(exit_code)
