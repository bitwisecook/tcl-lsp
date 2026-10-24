# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Affero General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.
#
# SPDX-License-Identifier: AGPL-3.0-or-later

"""Read-only unwind metadata for a pinned runtime test executable."""

import argparse
import bisect
import hashlib
import importlib.util
import json
import os
import re
import shutil
import subprocess
import sys
import time
from pathlib import Path
from typing import Any

EXECUTOR_SHA = "88125bd5267ac897dda4b5f665872c5ae0ab9984b0f42a41edeadf718873d811"
FLOOR = 2 * 1024**3
PROBE_CAP = 512 * 1024
CATALOGUE_CAP = 256 * 1024
LINE_CAP = 64 * 1024
ITEM_CAP = 250000
WATCH = (
    "::eval_script",
    "::eval_command",
    "::eval_words",
    "::eval_control_body",
    "::eval_generic_control_body",
    "::eval_original_body_framed",
    "::eval_shared_located_body",
    "::eval_framed",
    "::each_loop",
    "::foreach",
    "::prepare_each_loop_group",
    "::execute_body_region",
    "::execute_body_instruction",
    "::execute_body_each",
    "::execute_body_control",
    "::execute_body_switch",
    "::native_command_selection",
    "::dispatch_native",
)


def require(value, message):
    if not value:
        raise RuntimeError(message)


def sha(path):
    with Path(path).open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def save(path, value):
    Path(path).write_text(json.dumps(value, indent=2, sort_keys=True) + "\n")


def seal(paths):
    return {
        str(p): {
            "sha256": sha(p),
            "resolved": str(p.resolve()),
            "symlink": os.readlink(p) if p.is_symlink() else None,
            "mode": p.stat().st_mode,
        }
        for p in paths
    }


def stream_tool(argv, consume):
    process = subprocess.Popen(argv, stdin=subprocess.DEVNULL, stdout=subprocess.PIPE)
    if process.stdout is None:
        raise RuntimeError("inspection stdout absent")
    digest = hashlib.sha256()
    count = lines = overlong = 0
    discarding = False
    while True:
        data = process.stdout.readline(LINE_CAP)
        if not data:
            break
        digest.update(data)
        count += len(data)
        continued = not data.endswith(b"\n") and len(data) == LINE_CAP
        if continued and not discarding:
            overlong += 1
        if not continued:
            lines += 1
        if not discarding and not continued:
            consume(data)
        discarding = continued
    process.stdout.close()
    code = process.wait()
    require(code == 0, "inspection tool failed: " + repr((argv, code)))
    return {
        "argv": argv,
        "bytes": count,
        "sha256": digest.hexdigest(),
        "lines": lines,
        "overlong_lines_not_parsed": overlong,
        "exit_code": code,
        "raw_stream_retained": False,
    }


def inspect_frames(binary, nm, objdump, expected_sha):
    require(sha(binary) == expected_sha, "ELF changed before frame inspection")
    versions = {}
    for label, tool in [("nm", nm), ("objdump", objdump)]:
        text = bytearray()

        def retain_version(line, target=text):
            if len(target) < 4096:
                target.extend(line[: 4096 - len(target)])

        observed = stream_tool([str(tool), "--version"], retain_version)
        versions[label] = {
            "text_prefix": text.decode(errors="replace"),
            "stream": observed,
            "text_truncated": observed["bytes"] > len(text),
        }
    symbols: list[dict[str, Any]] = []
    symbol_overflow = 0
    unparsed_symbol_rows = 0
    non_function_symbol_rows = 0

    def symbol_line(line):
        nonlocal symbol_overflow, unparsed_symbol_rows, non_function_symbol_rows
        match = re.fullmatch(
            rb"([0-9a-fA-F]+)\s+([0-9a-fA-F]+)\s+[tTwW]\s+(.+)\n?", line
        )
        if match is None:
            if line.strip():
                ordinary = re.fullmatch(
                    rb"[0-9a-fA-F]+(?:\s+[0-9a-fA-F]+)?\s+([A-Za-z?])\s+.+\n?", line
                )
                if ordinary is not None and ordinary[1] not in (b"t", b"T", b"w", b"W"):
                    non_function_symbol_rows += 1
                else:
                    unparsed_symbol_rows += 1
            return
        if len(symbols) == ITEM_CAP:
            symbol_overflow += 1
            return
        name = match[3].rstrip(b"\n")
        symbols.append(
            {
                "start": int(match[1], 16),
                "size": int(match[2], 16),
                "name": name[:512].decode(errors="replace"),
                "name_sha256": hashlib.sha256(name).hexdigest(),
                "name_bytes": len(name),
                "name_truncated": len(name) > 512,
                "watched": any(word.encode() in name for word in WATCH),
            }
        )

    symbol_stream = stream_tool(
        [
            str(nm),
            "--defined-only",
            "--print-size",
            "--numeric-sort",
            "--demangle=rust",
            str(binary),
        ],
        symbol_line,
    )
    symbols.sort(key=lambda item: item["start"])
    starts = [item["start"] for item in symbols]
    frames: list[dict[str, Any]] = []
    current: dict[str, Any] | None = None
    frame_overflow = 0
    unparsed_FDE_headers = 0
    unparsed_CFA_rows = 0

    def finish_frame():
        nonlocal current, frame_overflow
        if current is None:
            return
        if len(frames) == ITEM_CAP:
            frame_overflow += 1
        else:
            frames.append(current)
        current = None

    def frame_line(line):
        nonlocal current, unparsed_FDE_headers, unparsed_CFA_rows
        header = re.search(rb"\bFDE\b.*\bpc=([0-9a-fA-F]+)\.\.([0-9a-fA-F]+)", line)
        if header is not None and int(header[2], 16) > int(header[1], 16):
            finish_frame()
            current = {
                "pc_start": int(header[1], 16),
                "pc_end": int(header[2], 16),
                "cfa_rows": 0,
                "max_rsp_cfa_offset": None,
                "other_cfa_rows": 0,
                "other_cfa_examples": [],
            }
            return
        if re.search(rb"\bFDE\b", line):
            finish_frame()
            unparsed_FDE_headers += 1
            return
        if re.search(rb"\bCIE\b", line):
            finish_frame()
            return
        row = re.match(rb"\s*[0-9a-fA-F]{8,16}\s+(\S+)", line)
        if row is None:
            if re.match(rb"\s*[0-9a-fA-F]{8,16}(?:\s|$)", line):
                unparsed_CFA_rows += 1
                if current is not None:
                    current["other_cfa_rows"] += 1
            return
        if current is None:
            return
        current["cfa_rows"] += 1
        cfa = row[1]
        offset = re.fullmatch(rb"rsp\+([0-9]+)", cfa)
        if offset is not None:
            value = int(offset[1])
            previous = current["max_rsp_cfa_offset"]
            current["max_rsp_cfa_offset"] = (
                value if previous is None else max(previous, value)
            )
        else:
            current["other_cfa_rows"] += 1
            if len(current["other_cfa_examples"]) < 4:
                current["other_cfa_examples"].append(cfa.decode(errors="replace"))

    frame_stream = stream_tool(
        [str(objdump), "--wide", "--dwarf=frames-interp", str(binary)], frame_line
    )
    finish_frame()
    unmatched = unknown = large = relevant_symbol_gaps = 0
    rows: list[dict[str, Any]] = []
    for frame in frames:
        start = frame["pc_start"]
        left = bisect.bisect_left(starts, start)
        right = bisect.bisect_right(starts, start)
        if left != right:
            matches = symbols[left:right]
        elif left:
            candidate = symbols[left - 1]
            matches = (
                [candidate] if start < candidate["start"] + candidate["size"] else []
            )
        else:
            matches = []
        unmatched += not bool(matches)
        uncertain = not matches or frame["other_cfa_rows"] > 0 or frame["cfa_rows"] == 0
        unknown += uncertain
        offset = frame["max_rsp_cfa_offset"]
        lower_bound = None if offset is None else max(0, offset - 8)
        is_large = lower_bound is not None and lower_bound >= 1024
        large += is_large
        watched = any(item["watched"] for item in matches)
        if (is_large or watched or uncertain) and (
            not matches
            or len(matches) > 4
            or any(item["name_truncated"] for item in matches)
        ):
            relevant_symbol_gaps += 1
        rows.append(
            {
                **frame,
                "stack_bytes_lower_bound_from_rsp_cfa": lower_bound,
                "sizing_incomplete": uncertain,
                "large_lower_bound": is_large,
                "watched": watched,
                "symbols": matches[:4],
                "symbol_alias_count": len(matches),
                "symbol_names_incomplete": len(matches) > 4
                or any(item["name_truncated"] for item in matches),
            }
        )
    # Prefer the actual source recursion path, then every known large frame,
    # then unknown unwind forms. Omitted rows are counted and never treated as small.
    rows.sort(
        key=lambda item: (
            not item["watched"],
            not item["large_lower_bound"],
            -(item["stack_bytes_lower_bound_from_rsp_cfa"] or 0),
        )
    )
    kept = []
    retained = omitted = 0
    for row in rows:
        if not (row["watched"] or row["large_lower_bound"] or row["sizing_incomplete"]):
            continue
        size = len(json.dumps(row).encode())
        if retained + size > CATALOGUE_CAP:
            omitted += 1
        else:
            kept.append(row)
            retained += size
    require(sha(binary) == expected_sha, "ELF changed during frame inspection")
    return {
        "status": "CAPTURED",
        "elf_sha256": expected_sha,
        "tool_versions": versions,
        "symbol_stream": symbol_stream,
        "frame_stream": frame_stream,
        "defined_function_symbols": len(symbols),
        "no_function_symbols": not bool(symbols),
        "unparsed_symbol_rows": unparsed_symbol_rows,
        "valid_non_function_symbol_rows": non_function_symbol_rows,
        "FDE_records": len(frames),
        "no_unwind_frames": not bool(frames),
        "unparsed_FDE_headers": unparsed_FDE_headers,
        "unparsed_CFA_rows": unparsed_CFA_rows,
        "unmatched_FDE_records": unmatched,
        "relevant_frame_symbol_gaps": relevant_symbol_gaps,
        "unknown_CFA_frame_records": unknown,
        "large_rsp_lower_bound_frame_records": large,
        "catalogue": kept,
        "catalogue_bytes": retained,
        "relevant_rows_omitted": omitted,
        "symbol_records_omitted": symbol_overflow,
        "frame_records_omitted": frame_overflow,
        "capture_incomplete": bool(
            omitted
            or not symbols
            or not frames
            or unmatched
            or unparsed_symbol_rows
            or unparsed_FDE_headers
            or unparsed_CFA_rows
            or relevant_symbol_gaps
            or symbol_overflow
            or frame_overflow
            or unknown
            or symbol_stream["overlong_lines_not_parsed"]
            or frame_stream["overlong_lines_not_parsed"]
        ),
        "limits": [
            "Static unwind metadata for this exact ELF; no test or ELF execution, active stack pointer, depth reached, backtrace or overflow-cause proof.",
            "rsp CFA offset minus the 8-byte return address is a lower bound at recorded PCs, not an exact maximum frame measurement. Red-zone locals, dynamic CFA expressions and non-rsp/frame-pointer rows remain unknown.",
            "All tool stdout is streamed and hashed; raw symbol/unwind text and bounded catalogue tail are not retained. Counts and incomplete flags prevent omitted/unknown records from proving small frames.",
        ],
    }


def capture_stack_frames(
    *,
    out,
    binary,
    source,
    expected_elf_sha256,
    environment,
    absolute_deadline,
    remaining_log_bytes,
):
    directory = Path(out) / "stack-frames"
    require(not directory.exists(), "fresh stack capture directory required")
    directory.mkdir()
    binary = Path(binary).resolve()
    record: dict[str, Any] = {
        "source": source,
        "binary": str(binary),
        "expected_elf_sha256": expected_elf_sha256,
        "status": "UNRUN",
        "retained_bytes": 0,
        "remaining_log_bytes": remaining_log_bytes,
        "capture_incomplete": True,
        "error": None,
    }
    tools: list[Path] = []
    pins = None
    try:
        require(re.fullmatch(r"[0-9a-f]{40}", source), "full source SHA required")
        require(
            re.fullmatch(r"[0-9a-f]{64}", expected_elf_sha256), "full ELF SHA required"
        )
        require(
            binary.is_file() and sha(binary) == expected_elf_sha256,
            "fresh selected ELF differs from prepared identity",
        )
        with binary.open("rb") as stream:
            header = stream.read(20)
        require(
            header[:6] == b"\x7fELF\x02\x01"
            and int.from_bytes(header[18:20], "little") == 62,
            "x86_64 little-endian ELF required for rsp CFA interpretation",
        )
        require(
            environment.get("GITHUB_SHA") == source
            and environment.get("GITHUB_REF") == "refs/heads/work",
            "exact WORK source environment required",
        )
        require(
            60 + 25 + 205 <= absolute_deadline - time.time() <= 3600,
            "insufficient or invalid full probe/cleanup/original-case reserve",
        )
        require(
            0 <= remaining_log_bytes <= 20 * 1024**2,
            "invalid remaining aggregate log budget",
        )
        cap = min(PROBE_CAP, remaining_log_bytes - 20000000 - 65536)
        require(cap > 0, "insufficient log reserve for optional stack inspection")
        helper = Path(__file__).resolve()
        executor = helper.parent / "runtime-diagnostics-exec.py"
        require(sha(executor) == EXECUTOR_SHA, "owned process executor changed")
        found = []
        for name in ["nm", "objdump"]:
            path = shutil.which(name, path=environment.get("PATH"))
            if path is None:
                raise RuntimeError("inspection tool unavailable: " + name)
            found.append(Path(path).absolute())
        nm, objdump = found
        python = Path(sys.executable).resolve()
        tools = [helper, executor, python, nm, objdump]
        pins = seal(tools)
        record["tools_before"] = pins
        spec = importlib.util.spec_from_file_location("stack_capture_owner", executor)
        if spec is None or spec.loader is None:
            raise RuntimeError("owned process loader unavailable")
        owner = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(owner)
        argv = [
            str(python),
            "-B",
            str(helper),
            "--worker",
            "--elf",
            str(binary),
            "--elf-sha",
            expected_elf_sha256,
            "--nm",
            str(nm),
            "--objdump",
            str(objdump),
        ]
        require(
            absolute_deadline - time.time() >= 60 + 25 + 205,
            "full probe reserve exhausted before spawn",
        )
        record["status"] = "FAILED"
        receipt = owner.run(
            argv,
            cwd=Path.cwd(),
            env=environment,
            stdout_path=directory / "worker.stdout",
            stderr_path=directory / "worker.stderr",
            receipt_path=directory / "worker.process.json",
            artifact_path=python,
            deadline_seconds=60,
            cap_bytes=cap,
            disk_floor_bytes=FLOOR,
            disk_paths=(
                binary.parent,
                directory,
                Path(environment.get("RUNNER_TEMP", str(directory.parent))),
            ),
        )
        record["process"] = receipt
        observed_bytes = sum(
            (directory / name).stat().st_size
            for name in ["worker.stdout", "worker.stderr"]
        )
        record["retained_bytes"] = observed_bytes
        record["remaining_log_bytes"] = remaining_log_bytes - observed_bytes
        require(
            0 <= observed_bytes == receipt["retained_bytes"] <= cap,
            "inspection retained log census differs or exceeds cap",
        )
        after = seal(tools)
        record["tools_after"] = after
        record["elf_sha256_after"] = sha(binary)
        require(
            pins == after and record["elf_sha256_after"] == expected_elf_sha256,
            "inspection tools or ELF changed",
        )
        require(
            all(
                receipt.get(key) is True
                for key in [
                    "complete",
                    "timer_armed_before_spawn",
                    "artifact_unchanged",
                    "group_empty_before_parent_reap",
                ]
            )
            and receipt.get("stop_reason") is None
            and receipt["exit_code"] == 0,
            "inspection process failed or incomplete",
        )
        require(
            receipt["consecutive_complete_empty_group_scans"] >= 2
            and not receipt["owned_live_group_members"],
            "inspection owned group was not proved empty",
        )
        require(
            receipt["timer_armed_monotonic_ns"] <= receipt["spawn_return_monotonic_ns"],
            "inspection timer armed after spawn",
        )
        require(
            receipt["artifact_sha256_before"]
            == receipt["artifact_sha256_after"]
            == pins[str(python)]["sha256"],
            "inspection Python artifact differs",
        )
        require(
            receipt["argv"] == argv and receipt["artifact"] == str(python),
            "inspection owner argv or artifact differs",
        )
        report = json.loads((directory / "worker.stdout").read_text())
        require(
            report["status"] == "CAPTURED"
            and report["elf_sha256"] == expected_elf_sha256,
            "inspection report identity differs",
        )
        record["report"] = report
        record["status"] = "CAPTURED"
        record["tool_stderr_bytes"] = (directory / "worker.stderr").stat().st_size
        record["capture_incomplete"] = (
            report["capture_incomplete"] or record["tool_stderr_bytes"] > 0
        )
    except (
        RuntimeError,
        OSError,
        ValueError,
        TypeError,
        KeyError,
        ImportError,
    ) as error:
        record["error"] = repr(error)
    finally:
        record["scope"] = (
            "Read-only diagnostic metadata. Existing original3 stdout-only cap and separate stderr cap are unchanged; this component does not prove a retroactive global capture cap. Caller must include retained_bytes in existing combined original/supplement log accounting. Incomplete frame data never admits a stack fix."
        )
        save(directory / "summary.json", record)
    return record


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--worker", action="store_true", required=True)
    parser.add_argument("--elf", type=Path, required=True)
    parser.add_argument("--elf-sha", required=True)
    parser.add_argument("--nm", type=Path, required=True)
    parser.add_argument("--objdump", type=Path, required=True)
    args = parser.parse_args()
    try:
        result = inspect_frames(args.elf, args.nm, args.objdump, args.elf_sha)
        print(json.dumps(result, sort_keys=True))
        return 0
    except (RuntimeError, OSError, ValueError, TypeError, KeyError) as error:
        print(
            json.dumps(
                {"status": "FAILED", "error": repr(error), "capture_incomplete": True}
            )
        )
        return 1


if __name__ == "__main__":
    sys.exit(main())
