"""WORK-only separate autoload structural Core2 controls; no startup admission."""

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
import time
from pathlib import Path

CAP = 20 * 1024**2
FLOOR = 2 * 1024**3
EXECUTOR_SHA = "88125bd5267ac897dda4b5f665872c5ae0ab9984b0f42a41edeadf718873d811"
ROSTER_SHA = "41a2a5252ea8e4b0b25b3bb897081e4e685e9f2628779a02e212de1a6ba099ec"
RESULT_COMPLETION_CONTROLS = ('original_command_rename::tests::authored_result_completion_coverage_retains_nested_call_without_normal_grant', 'original_command_rename::tests::authored_result_completion_coverage_refuses_unknown_shadowed_error_and_observer_inputs', 'original_command_rename::tests::authored_result_completion_never_skips_nested_name_write_or_opaque_channel')
SOURCE_LOAD_CONTROLS = ('original_command_rename::source_coverage::tests::single_cut_source_coverage_surveys_actual_child_and_issued_suffix', 'original_command_rename::source_coverage::tests::source_coverage_refuses_missing_child_reference_and_foreign_membership', 'original_command_rename::source_coverage::tests::source_coverage_keeps_selected_owner_and_opaque_body_refusals', 'original_command_rename::source_coverage::tests::source_coverage_never_omits_extra_cuts_or_namespace_frames')
TK_PREREQUISITE_CONTROLS = ('hover::original_registry::tests::tk_source_hover_keeps_declared_prerequisite_and_unresolved_availability', 'hover::original_registry::tests::tk_source_prerequisite_refuses_stale_source_and_foreign_input', 'hover::original_registry::tests::declared_prerequisite_uses_required_package_not_owning_package')
BASE = Path(__file__).resolve().parent


def require(value, message):
    if not value:
        raise RuntimeError(message)


def sha(path):
    with Path(path).open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def save(path, value):
    Path(path).write_text(json.dumps(value, indent=2, sort_keys=True) + "\n")


def git(*argv):
    return subprocess.check_output(["git", *argv], text=True).strip()


def files(paths):
    result = {}
    for root in paths:
        require(root.exists(), "required provider absent: " + str(root))
        names = [root] if root.is_file() else sorted(root.rglob("*"))
        for path in names:
            if path.is_file():
                result[str(path)] = {
                    "sha256": sha(path),
                    "symlink": os.readlink(path) if path.is_symlink() else None,
                }
    return result


def source():
    require(git("rev-parse", "HEAD") == args.sha, "source commit changed")
    require(
        not git("status", "--porcelain=v1", "--untracked-files=all"), "dirty source"
    )
    paths = [
        root / os.fsdecode(name)
        for name in subprocess.check_output(["git", "ls-files", "-z"]).split(b"\0")
        if name
    ]
    return {
        "head": args.sha,
        "tree": git("rev-parse", "HEAD^{tree}"),
        "files": files(paths),
    }


def lifecycle(receipt):
    return (
        all(
            receipt.get(key) is True
            for key in [
                "complete",
                "group_empty_before_parent_reap",
                "timer_armed_before_spawn",
                "artifact_unchanged",
            ]
        )
        and receipt.get("stop_reason") is None
    )


def original_result(stdout, exit_code):
    terminal = re.findall(
        r"^test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured; (\d+) filtered out;",
        stdout,
        re.MULTILINE,
    )
    require(len(terminal) == 1, "selected test terminal missing/ambiguous")
    label, passed, failed, ignored, measured, _ = terminal[0]
    require(
        "running 1 test" in stdout and int(ignored) == int(measured) == 0,
        "selected test counts mismatch",
    )
    require(
        (label, int(passed), int(failed), exit_code)
        in [("ok", 1, 0, 0), ("FAILED", 0, 1, 101)],
        "exit/terminal mismatch",
    )
    return "PASS" if int(passed) == 1 else "FAIL"


def execute(argv, prefix, artifact, seconds, observation_environment=None):
    global remaining
    require(remaining > 0, "aggregate log budget exhausted")
    require(
        time.time() + seconds <= float(os.environ["DIAGNOSTIC_DEADLINE_EPOCH"]),
        "insufficient full process budget",
    )
    receipt = owner.run(
        argv,
        cwd=root,
        env=environment if observation_environment is None else {**environment, **observation_environment},
        stdout_path=out / (prefix + ".stdout"),
        stderr_path=out / (prefix + ".stderr"),
        receipt_path=out / (prefix + ".process.json"),
        artifact_path=artifact,
        deadline_seconds=seconds,
        cap_bytes=remaining,
        disk_floor_bytes=FLOOR,
        disk_paths=(root, out, target, Path(os.environ["RUNNER_TEMP"])),
    )
    processes.append(receipt)
    remaining -= receipt["retained_bytes"]
    require(lifecycle(receipt) and remaining >= 0, "process capture incomplete")
    return receipt


def validate_result_completion_controls(roster):
    require(
        roster.get("result_completion_control_selectors") == list(RESULT_COMPLETION_CONTROLS),
        "exact bounded result completion controls mismatch",
    )


def run_result_completion_controls(roster, executable, artifact, out, cases):
    validate_result_completion_controls(roster)
    for index, selector in enumerate(roster["result_completion_control_selectors"]):
        require(sha(executable) == artifact["sha256"], "selected executable changed")
        prefix = "result-completion-control-" + str(index + 1)
        listed = execute(
            [str(executable), selector, "--exact", "--list", "--format", "terse"],
            prefix + "-list", executable, 180,
        )
        require(
            listed["exit_code"] == 0
            and (out / (prefix + "-list.stdout")).read_text().splitlines()
                == [selector + ": test"],
            "exact result completion control missing/ambiguous",
        )
        receipt = execute(
            [str(executable), selector, "--exact", "--nocapture", "--test-threads=1"],
            prefix, executable, 180,
        )
        result = original_result((out / (prefix + ".stdout")).read_text(), receipt["exit_code"])
        cases.append({"selector": selector, "result": result, "exit_code": receipt["exit_code"]})
        save(out / "result-completion-control-cases.json", cases)


def validate_source_load_controls(roster):
    require(
        roster.get("source_load_control_selectors") == list(SOURCE_LOAD_CONTROLS),
        "exact bounded source load controls mismatch",
    )


def run_source_load_controls(roster, executable, artifact, out, cases):
    validate_source_load_controls(roster)
    for index, selector in enumerate(roster["source_load_control_selectors"]):
        require(sha(executable) == artifact["sha256"], "selected executable changed")
        prefix = "source-load-control-" + str(index + 1)
        listed = execute(
            [str(executable), selector, "--exact", "--list", "--format", "terse"],
            prefix + "-list", executable, 180,
        )
        require(
            listed["exit_code"] == 0
            and (out / (prefix + "-list.stdout")).read_text().splitlines()
                == [selector + ": test"],
            "exact source load control missing/ambiguous",
        )
        receipt = execute(
            [str(executable), selector, "--exact", "--nocapture", "--test-threads=1"],
            prefix, executable, 180,
        )
        result = original_result((out / (prefix + ".stdout")).read_text(), receipt["exit_code"])
        cases.append({"selector": selector, "result": result, "exit_code": receipt["exit_code"]})
        save(out / "source-load-control-cases.json", cases)


def validate_tk_prerequisite_controls(roster):
    require(
        roster.get("tk_prerequisite_control_selectors") == list(TK_PREREQUISITE_CONTROLS),
        "exact bounded Tk prerequisite controls mismatch",
    )


def run_tk_prerequisite_controls(roster, executable, artifact, out, cases):
    validate_tk_prerequisite_controls(roster)
    for index, selector in enumerate(roster["tk_prerequisite_control_selectors"]):
        require(sha(executable) == artifact["sha256"], "selected executable changed")
        prefix = "tk-prerequisite-control-" + str(index + 1)
        listed = execute(
            [str(executable), selector, "--exact", "--list", "--format", "terse"],
            prefix + "-list", executable, 180,
        )
        require(
            listed["exit_code"] == 0
            and (out / (prefix + "-list.stdout")).read_text().splitlines()
                == [selector + ": test"],
            "exact Tk prerequisite control missing/ambiguous",
        )
        receipt = execute(
            [str(executable), selector, "--exact", "--nocapture", "--test-threads=1"],
            prefix, executable, 180,
        )
        result = original_result((out / (prefix + ".stdout")).read_text(), receipt["exit_code"])
        cases.append({"selector": selector, "result": result, "exit_code": receipt["exit_code"]})
        save(out / "tk-prerequisite-control-cases.json", cases)


parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--sha", required=True)
parser.add_argument("--out", type=Path, required=True)
args = parser.parse_args()
require(re.fullmatch("[0-9a-f]{40}", args.sha), "full SHA required")
require(os.environ.get("GITHUB_SHA") == args.sha, "workflow/source SHA mismatch")
require(os.environ.get("GITHUB_REF") == "refs/heads/work", "WORK only")
root = Path.cwd().resolve()
out = args.out.resolve()
target = Path(os.environ["CARGO_TARGET_DIR"]).resolve()
require(
    not out.exists() and not out.is_relative_to(root), "fresh external output required"
)
require(
    not target.exists() and not target.is_relative_to(root),
    "fresh external target required",
)
out.mkdir(parents=True)
resource.setrlimit(resource.RLIMIT_CORE, (0, 0))
remaining = CAP
processes = []
cases = []
seals = {}
originals = {}
providers = []
dependency_roots = []
tools = {}
artifact = None
integration_artifact = None
integration_cases = []
quiet_store_cases = []
result_completion_cases = []
source_load_cases = []
tk_prerequisite_cases = []
error = None
status = 2
environment = os.environ.copy()
try:
    executor = BASE / "runtime-diagnostics-exec.py"
    roster_path = BASE / "autoload-core2-inputs.json"
    require(sha(executor) == EXECUTOR_SHA, "process owner pin changed")
    require(sha(roster_path) == ROSTER_SHA, "Core2 roster pin changed")
    roster = json.loads(roster_path.read_text())
    validate_source_load_controls(roster)
    validate_result_completion_controls(roster)
    validate_tk_prerequisite_controls(roster)
    require(
        roster["executor_sha256"] == EXECUTOR_SHA
        and len(roster["selectors"]) == 2
        and len(set(roster["selectors"])) == 2
        and roster["integration_target"] == "lsp_edit_workspace"
        and roster["integration_selectors"] == ["minify_strips_comments_and_collapses_whitespace_factorial", "minify_preserves_string_and_expr_results"],
        "Core2 roster mismatch",
    )
    require(roster["build_observation_environment"] == {"TCL_LSP_TRACE_COARSE_SOURCE": "1"}
        and roster["integration_observation_environment"] == {"TCL_LSP_TRACE_BODY_SYNTAX": "1"},
        "exact minify observation environment required")
    require(roster["quiet_store_control_selectors"] == ['original_command_rename::tests::quiet_store_coverage_requires_actual_points_and_refuses_traced_foreign_source'], "quiet store control roster mismatch")
    require(
        all(sha(root / n) == pin for n, pin in roster["source_pins"].items()),
        "reviewed structural source changed",
    )
    spec = importlib.util.spec_from_file_location("core2_owner", executor)
    require(spec is not None and spec.loader is not None, "owner loader absent")
    owner = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(owner)
    rustup = shutil.which("rustup")
    require(rustup is not None, "rustup absent")
    tools = {"python": Path(sys.executable).resolve(), "executor": executor}
    for name in ["cargo", "rustc"]:
        tools[name] = Path(
            subprocess.check_output([rustup, "which", name], text=True).strip()
        ).resolve()
    for name in ["cc", "ar", "ld"]:
        found = shutil.which(name)
        require(found is not None, "native tool absent: " + name)
        tools[name] = Path(found).resolve()
    overrides = [
        "RUSTC_WRAPPER",
        "RUSTC_WORKSPACE_WRAPPER",
        "CARGO_ENCODED_RUSTFLAGS",
        "RUSTC_BOOTSTRAP",
        "TCL_RUNTIME_EXPECT_NO_TOMMATH",
        "CARGO_BUILD_TARGET",
        "CARGO_BUILD_RUSTC",
        "CARGO_BUILD_RUSTFLAGS",
        "CARGO_BUILD_RUSTC_WRAPPER",
        "CARGO_BUILD_RUSTC_WORKSPACE_WRAPPER",
    ]
    require(not any(environment.get(n) for n in overrides), "unexpected build override")
    require(
        not any(
            n.startswith("CARGO_TARGET_") and n.endswith("_RUSTFLAGS") and v
            for n, v in environment.items()
        ),
        "target Rust flag override",
    )
    require(environment.get("RUSTFLAGS") == "-Dwarnings", "strict warnings required")
    environment["RUSTC"] = str(tools["rustc"])
    sysroot = Path(
        subprocess.check_output(
            [str(tools["rustc"]), "--print", "sysroot"], text=True
        ).strip()
    )
    providers = [Path(environment["TCL_TOMMATH_DIR"]).parent, sysroot / "lib"]
    require((providers[0] / "libtommath/tommath.h").is_file(), "full TomMath absent")
    originals = {
        "source": source(),
        "providers": files(providers),
        "tools": files(list(tools.values())),
    }
    for name, value in originals.items():
        save(out / (name + "-before.json"), value)
    save(
        out / "job.json",
        {
            "requested_sha": args.sha,
            "job": {
                n: environment.get(n)
                for n in [
                    "GITHUB_RUN_ID",
                    "GITHUB_RUN_ATTEMPT",
                    "GITHUB_JOB",
                    "GITHUB_WORKFLOW_REF",
                ]
            },
            "limits": {
                "fetch_seconds": 600,
                "build_seconds": 2400,
                "each_list_or_test_seconds": 180,
                "aggregate_log_bytes": CAP,
                "disk_floor_bytes": FLOOR,
            },
            "roster": roster,
        },
    )
    fetched = execute(
        [
            str(tools["cargo"]),
            "fetch",
            "--locked",
            "--target",
            "x86_64-unknown-linux-gnu",
        ],
        "fetch",
        tools["cargo"],
        600,
    )
    require(fetched["exit_code"] == 0, "dependency fetch failed")
    cargo_home = Path(environment.get("CARGO_HOME", str(Path.home() / ".cargo")))
    dependency_roots = [
        p
        for p in [
            cargo_home / "registry/src",
            cargo_home / "registry/cache",
            cargo_home / "git/checkouts",
        ]
        if p.exists()
    ]
    require(bool(dependency_roots), "fetched dependency inputs absent")
    originals["dependencies"] = files(dependency_roots)
    save(out / "dependencies-before.json", originals["dependencies"])
    built = execute(
        [
            str(tools["cargo"]),
            "test",
            "--locked",
            "--profile",
            "ci",
            "-p",
            "tcl-lsp-core",
            "--lib",
            "--test",
            roster["integration_target"],
            "--no-run",
            "--message-format=json-render-diagnostics",
            "-j",
            "2",
        ],
        "build",
        tools["cargo"],
        2400,
        roster["build_observation_environment"],
    )
    require(built["exit_code"] == 0, "fresh Core libtest build failed")
    rows = [
        json.loads(line)
        for line in (out / "build.stdout").read_text().splitlines()
        if line.startswith("{")
    ]
    require(
        not any(
            r.get("reason") == "compiler-message"
            and r.get("message", {}).get("level") in ["warning", "error"]
            for r in rows
        ),
        "Rust diagnostic observed",
    )
    require(
        sum(
            r.get("reason") == "build-finished" and r.get("success") is True
            for r in rows
        )
        == 1,
        "Cargo success terminal absent",
    )
    selected = [
        r
        for r in rows
        if r.get("reason") == "compiler-artifact"
        and r.get("target", {}).get("name") == roster["target"]
        and r.get("target", {}).get("kind") == ["lib"]
        and r.get("manifest_path") == str(root / "rust/tcl-lsp-core/Cargo.toml")
        and r.get("profile", {}).get("test") is True
        and r.get("executable")
    ]
    require(
        len(selected) == 1
        and selected[0].get("fresh") is False
        and str(selected[0].get("profile", {}).get("opt_level")) == "3",
        "fresh CI Core libtest event absent/ambiguous",
    )
    executable = Path(selected[0]["executable"]).resolve()
    require(
        executable.is_relative_to(target) and executable.is_file(),
        "executable outside fresh target",
    )
    with executable.open("rb") as stream:
        require(stream.read(4) == b"\x7fELF", "native ELF required")
    artifact = {
        "path": str(executable),
        "sha256": sha(executable),
        "cargo_event": selected[0],
    }
    save(out / "core-artifact.json", artifact)
    integration_selected = [r for r in rows
        if r.get("reason") == "compiler-artifact"
        and r.get("target", {}).get("name") == roster["integration_target"]
        and r.get("target", {}).get("kind") == ["test"]
        and r.get("manifest_path") == str(root / "rust/tcl-lsp-core/Cargo.toml")
        and r.get("profile", {}).get("test") is True and r.get("executable")]
    require(len(integration_selected) == 1
        and integration_selected[0].get("fresh") is False
        and str(integration_selected[0].get("profile", {}).get("opt_level")) == "3",
        "fresh CI exact Default integration event absent/ambiguous")
    integration_executable = Path(integration_selected[0]["executable"]).resolve()
    require(integration_executable.is_relative_to(target) and integration_executable.is_file()
        and integration_executable != executable, "distinct fresh integration executable required")
    with integration_executable.open("rb") as stream:
        require(stream.read(4) == b"\x7fELF", "native integration ELF required")
    integration_artifact = {"path": str(integration_executable), "sha256": sha(integration_executable),
        "cargo_event": integration_selected[0]}
    save(out / "default-minify-integration-artifact.json", integration_artifact)
    require(
        source() == originals["source"]
        and files(providers) == originals["providers"]
        and files(dependency_roots) == originals["dependencies"]
        and files(list(tools.values())) == originals["tools"],
        "pre-test input seals changed",
    )
    for index, selector in enumerate(roster["selectors"]):
        require(sha(executable) == artifact["sha256"], "selected executable changed")
        prefix = "core2-" + str(index + 1)
        listed = execute(
            [str(executable), selector, "--exact", "--list", "--format", "terse"],
            prefix + "-list",
            executable,
            180,
        )
        require(
            listed["exit_code"] == 0
            and (out / (prefix + "-list.stdout")).read_text().splitlines()
            == [selector + ": test"],
            "exact selected test missing/ambiguous",
        )
        receipt = execute(
            [str(executable), selector, "--exact", "--nocapture", "--test-threads=1"],
            prefix,
            executable,
            180,
        )
        result = original_result(
            (out / (prefix + ".stdout")).read_text(), receipt["exit_code"]
        )
        cases.append(
            {"selector": selector, "result": result, "exit_code": receipt["exit_code"]}
        )
        save(out / "cases.json", cases)
    status = 0 if len(cases) == 2 and all(c["result"] == "PASS" for c in cases) else 1
    for index, selector in enumerate(roster["integration_selectors"]):
        require(sha(integration_executable) == integration_artifact["sha256"], "selected executable changed")
        prefix = "default-minify-integration-" + str(index + 1)
        listed = execute(
            [str(integration_executable), selector, "--exact", "--list", "--format", "terse"],
            prefix + "-list",
            integration_executable,
            180,
            roster["integration_observation_environment"],
        )
        require(
            listed["exit_code"] == 0
            and (out / (prefix + "-list.stdout")).read_text().splitlines()
            == [selector + ": test"],
            "exact selected test missing/ambiguous",
        )
        receipt = execute(
            [str(integration_executable), selector, "--exact", "--nocapture", "--test-threads=1"],
            prefix,
            integration_executable,
            180,
            roster["integration_observation_environment"],
        )
        result = original_result(
            (out / (prefix + ".stdout")).read_text(), receipt["exit_code"]
        )
        integration_cases.append(
            {"selector": selector, "result": result, "exit_code": receipt["exit_code"]}
        )
        save(out / "default-minify-integration-cases.json", integration_cases)
    status = 0 if status == 0 and len(integration_cases) == 2 and all(c["result"] == "PASS" for c in integration_cases) else 1


    old_cohort_status = status
    status = 2
    for index, selector in enumerate(roster["quiet_store_control_selectors"]):
        require(sha(executable) == artifact["sha256"], "selected executable changed")
        prefix = "quiet-store-control-" + str(index + 1)
        listed = execute(
            [str(executable), selector, "--exact", "--list", "--format", "terse"],
            prefix + "-list", executable, 180,
        )
        require(
            listed["exit_code"] == 0
            and (out / (prefix + "-list.stdout")).read_text().splitlines()
            == [selector + ": test"],
            "exact selected quiet store control missing/ambiguous",
        )
        receipt = execute(
            [str(executable), selector, "--exact", "--nocapture", "--test-threads=1"],
            prefix, executable, 180,
        )
        result = original_result(
            (out / (prefix + ".stdout")).read_text(), receipt["exit_code"]
        )
        quiet_store_cases.append(
            {"selector": selector, "result": result, "exit_code": receipt["exit_code"],
             "same_fresh_core_library_sha256": artifact["sha256"]}
        )
        save(out / "quiet-store-control-cases.json", quiet_store_cases)
    status = 0 if old_cohort_status == 0 and len(quiet_store_cases) == 1 and all(c["result"] == "PASS" for c in quiet_store_cases) else 1

    prior_result_completion_status = status
    status = 2
    run_result_completion_controls(roster, executable, artifact, out, result_completion_cases)
    status = (
        0 if prior_result_completion_status == 0
        and len(result_completion_cases) == 3
        and all(case["result"] == "PASS" for case in result_completion_cases)
        else 1
    )

    prior_source_load_status = status
    status = 2
    run_source_load_controls(roster, executable, artifact, out, source_load_cases)
    status = (
        0 if prior_source_load_status == 0
        and len(source_load_cases) == 4
        and all(case["result"] == "PASS" for case in source_load_cases)
        else 1
    )

    prior_tk_prerequisite_status = status
    status = 2
    run_tk_prerequisite_controls(roster, executable, artifact, out, tk_prerequisite_cases)
    status = (
        0 if prior_tk_prerequisite_status == 0
        and len(tk_prerequisite_cases) == 3
        and all(case["result"] == "PASS" for case in tk_prerequisite_cases)
        else 1
    )

except (
    RuntimeError,
    OSError,
    ValueError,
    subprocess.SubprocessError,
    ImportError,
    KeyError,
    TypeError,
    AssertionError,
    KeyboardInterrupt,
) as failure:
    status = 2
    error = repr(failure)
    print("AUTOLOAD CORE2 STOP " + error, file=sys.stderr, flush=True)
finally:
    captures = {
        "source": source,
        "providers": lambda: files(providers),
        "tools": lambda: files(list(tools.values())),
        "dependencies": lambda: files(dependency_roots),
    }
    for name, capture in captures.items():
        try:
            current = capture()
            save(out / (name + "-after.json"), current)
            seals[name] = name in originals and current == originals[name]
        except (
            RuntimeError,
            OSError,
            ValueError,
            subprocess.SubprocessError,
            KeyError,
            TypeError,
        ):
            seals[name] = False
    seals["artifact"] = (
        artifact is not None
        and Path(artifact["path"]).is_file()
        and sha(artifact["path"]) == artifact["sha256"]
    )
    seals["integration_artifact"] = (
        integration_artifact is not None and Path(integration_artifact["path"]).is_file()
        and sha(integration_artifact["path"]) == integration_artifact["sha256"])
    if not all(seals.values()):
        status = 2
    save(
        out / "final.json",
        {
            "finished_utc": datetime.datetime.now(datetime.UTC).isoformat(),
            "overall_exit": status,
            "error": error,
            "seals": seals,
            "cases": cases,
            "separate_supplemental_total": len(cases),
            "exact_default_minify_integration_cases": integration_cases,
            "exact_default_minify_integration_total": len(integration_cases),
            "quiet_store_control_cases": quiet_store_cases,
            "quiet_store_control_total": len(quiet_store_cases),
            "result_completion_control_cases": result_completion_cases,
            "result_completion_control_total": len(result_completion_cases),
            "source_load_control_cases": source_load_cases,
            "source_load_control_total": len(source_load_cases),
            "tk_prerequisite_control_cases": tk_prerequisite_cases,
            "tk_prerequisite_control_total": len(tk_prerequisite_cases),
            "minify_observation_qualification": "WORK coarse-instrumented build; BODY_SYNTAX runtime enabled only exact Default integration list/test rows; old2 runtime environment unchanged. No timing/performance acceptance or new source queries.",
            "integration_qualification": "Distinct fresh integration ELF, same single Cargo invocation and shared original process/phase/aggregate limits; old Core2 selectors remain separate and unchanged.",
            "processes": processes,
            "remaining_log_bytes": remaining,
            "startup_acceptance": False,
            "performance_admission": False,
        },
    )
sys.exit(status)
