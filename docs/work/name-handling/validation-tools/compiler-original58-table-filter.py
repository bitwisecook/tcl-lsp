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
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
# GNU Affero General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public License
# along with this program. If not, see <https://www.gnu.org/licenses/>.
#
# SPDX-License-Identifier: AGPL-3.0-or-later

"""Bounded table summaries and lossless repeated source diagnostic records."""

import argparse
import hashlib
import json
import os
import re
import signal
import subprocess
import sys
from pathlib import Path

PREFIX = b"ORIGINAL_COMMAND_INSTALLED offset="
TABLE = b" bindings=SourceCommandTable("
HEADER = re.compile(
    rb"ORIGINAL_COMMAND_INSTALLED offset=([0-9]{1,10}) bindings=SourceCommandTable\("
)


def possible_header(value):
    if PREFIX.startswith(value):
        return True
    if not value.startswith(PREFIX):
        return False
    suffix = value[len(PREFIX) :]
    count = 0
    while count < len(suffix) and 48 <= suffix[count] <= 57:
        count += 1
    return 1 <= count <= 10 and TABLE.startswith(suffix[count:])


class TableFilter:
    def __init__(self, emit):
        self.emit = emit
        self.prefix = bytearray()
        self.state = "prefix"
        self.line_hash = hashlib.sha256()
        self.line_bytes = 0
        self.line_offset = b""
        self.input_hash = hashlib.sha256()
        self.table_hash = hashlib.sha256()
        self.input_bytes = 0
        self.table_bytes = 0
        self.table_lines = 0
        self.output_bytes = 0

    def output(self, value):
        self.emit(value)
        self.output_bytes += len(value)

    def finish_table(self, newline):
        digest = self.line_hash.hexdigest()
        self.table_lines += 1
        self.table_bytes += self.line_bytes
        self.table_hash.update(
            str(self.line_bytes).encode() + b":" + digest.encode() + b"\n"
        )
        self.output(
            b"ORIGINAL_COMMAND_INSTALLED offset="
            + self.line_offset
            + b" bindings=SourceCommandTable(SUMMARY count=1 bytes="
            + str(self.line_bytes).encode()
            + b" sha256="
            + digest.encode()
            + b" newline="
            + str(int(newline)).encode()
            + b")\n"
        )
        self.line_hash = hashlib.sha256()
        self.line_bytes = 0
        self.line_offset = b""

    def feed(self, chunk):
        self.input_hash.update(chunk)
        self.input_bytes += len(chunk)
        # Byte-wise only while recognising the bounded header; full table and
        # ordinary data are transferred or hashed by chunks until newline.
        while chunk:
            if self.state in ("table", "ordinary"):
                at = chunk.find(b"\n")
                take = chunk if at < 0 else chunk[: at + 1]
                if self.state == "table":
                    self.line_hash.update(take)
                    self.line_bytes += len(take)
                    if at >= 0:
                        self.finish_table(True)
                else:
                    self.output(take)
                chunk = chunk[len(take) :]
                if at >= 0:
                    self.state = "prefix"
                continue
            self.prefix.append(chunk[0])
            chunk = chunk[1:]
            value = bytes(self.prefix)
            match = HEADER.fullmatch(value)
            if match is not None:
                self.state = "table"
                self.line_offset = match[1]
                self.line_hash.update(value)
                self.line_bytes = len(value)
                self.prefix.clear()
            elif not possible_header(value):
                self.output(value)
                self.state = "prefix" if value.endswith(b"\n") else "ordinary"
                self.prefix.clear()

    def finish(self):
        if self.state == "table":
            self.finish_table(False)
        elif self.prefix:
            self.output(bytes(self.prefix))
            self.prefix.clear()
        return {
            "filter": "exact-original-installed-command-table-only-v1",
            "raw_stderr_bytes": self.input_bytes,
            "raw_stderr_sha256": self.input_hash.hexdigest(),
            "summarised_lines": self.table_lines,
            "summarised_input_bytes": self.table_bytes,
            "ordered_line_length_and_sha256_digest": self.table_hash.hexdigest(),
            "retained_stderr_bytes": self.output_bytes,
            "ordinary_stderr_verbatim": True,
            "stdout_verbatim": True,
        }


REPEAT_MAGIC = b"@TCLDIAG1 "
REPEAT_PREFIXES = (b"ORIGINAL_WORD_CONFIG ", b"REMOTE_SOURCE_WORDS ")
REPEAT_LINE_LIMIT = 64 * 1024
REPEAT_DICTIONARY_LIMIT = 2 * 1024 * 1024
REPEAT_ENTRY_LIMIT = 4096


class DiagnosticRepeatLines:
    """Lossless bounded records for two verbose diagnostic line families."""

    def __init__(self, emit):
        self.emit = emit
        self.buffer = bytearray()
        self.dictionary = {}
        self.dictionary_bytes = 0
        self.runs = []
        self.long_line = False
        self.quoted_long_line = False
        self.input_bytes = 0
        self.output_bytes = 0
        self.input_hash = hashlib.sha256()
        self.output_hash = hashlib.sha256()
        self.selected_lines = 0
        self.repeated_lines = 0

    def output(self, value):
        if value:
            self.emit(value)
            self.output_bytes += len(value)
            self.output_hash.update(value)

    def flush_runs(self):
        if self.runs:
            pairs = ",".join(f"{ident}:{count}" for ident, count in self.runs)
            self.output(REPEAT_MAGIC + b"R " + pairs.encode("ascii") + b"\n")
            self.runs.clear()

    def literal(self, value, quoted=False):
        self.flush_runs()
        if value and (quoted or value.startswith(REPEAT_MAGIC)):
            self.output(REPEAT_MAGIC + b"L " + str(len(value)).encode("ascii") + b"\n")
        self.output(value)

    def line(self, value):
        if not value.startswith(REPEAT_PREFIXES):
            self.literal(value)
            return
        self.selected_lines += 1
        ident = self.dictionary.get(value)
        if ident is not None:
            self.repeated_lines += 1
            if self.runs and self.runs[-1][0] == ident:
                self.runs[-1][1] += 1
            else:
                if len(self.runs) == 64:
                    self.flush_runs()
                self.runs.append([ident, 1])
            return
        self.flush_runs()
        if (
            len(self.dictionary) >= REPEAT_ENTRY_LIMIT
            or self.dictionary_bytes + len(value) > REPEAT_DICTIONARY_LIMIT
        ):
            self.literal(value)
            return
        ident = len(self.dictionary) + 1
        self.dictionary[value] = ident
        self.dictionary_bytes += len(value)
        header = f"D {ident} {len(value)} {hashlib.sha256(value).hexdigest()}\n"
        self.output(REPEAT_MAGIC + header.encode("ascii"))
        self.output(value)

    def feed(self, chunk):
        self.input_bytes += len(chunk)
        self.input_hash.update(chunk)
        pos = 0
        while pos < len(chunk):
            newline = chunk.find(b"\n", pos)
            end = len(chunk) if newline < 0 else newline + 1
            part = chunk[pos:end]
            pos = end
            if self.long_line:
                self.literal(part, self.quoted_long_line)
            elif len(self.buffer) + len(part) > REPEAT_LINE_LIMIT:
                prefix = (bytes(self.buffer) + part[: len(REPEAT_MAGIC)])[
                    : len(REPEAT_MAGIC)
                ]
                self.quoted_long_line = prefix.startswith(REPEAT_MAGIC)
                self.literal(bytes(self.buffer), self.quoted_long_line)
                self.buffer.clear()
                self.literal(part, self.quoted_long_line)
                self.long_line = True
            else:
                self.buffer.extend(part)
                if newline >= 0:
                    self.line(bytes(self.buffer))
                    self.buffer.clear()
            if newline >= 0:
                self.long_line = False
                self.quoted_long_line = False

    def finish(self):
        self.literal(bytes(self.buffer))
        self.buffer.clear()
        self.flush_runs()
        return {
            "format": "two-family-lossless-line-dictionary-rle-v1",
            "input_bytes": self.input_bytes,
            "input_sha256": self.input_hash.hexdigest(),
            "output_bytes": self.output_bytes,
            "output_sha256": self.output_hash.hexdigest(),
            "dictionary_entries": len(self.dictionary),
            "dictionary_bytes": self.dictionary_bytes,
            "selected_lines": self.selected_lines,
            "repeated_lines": self.repeated_lines,
        }


def decode_diagnostic_repeats(value, emit, expected_bytes):
    """Decode counted records; reject malformed or over-budget expansions."""
    dictionary = {}
    dictionary_bytes = 0
    count = 0
    digest = hashlib.sha256()
    pos = 0

    def output(data):
        nonlocal count
        if len(data) > expected_bytes - count:
            raise ValueError("diagnostic repeat expansion exceeds expected byte count")
        emit(data)
        digest.update(data)
        count += len(data)

    while pos < len(value):
        end = value.find(b"\n", pos)
        end = len(value) if end < 0 else end + 1
        line = value[pos:end]
        pos = end
        if not line.startswith(REPEAT_MAGIC):
            output(line)
            continue
        record = line[len(REPEAT_MAGIC) :]
        definition = re.fullmatch(
            rb"D ([1-9][0-9]*) ([1-9][0-9]*) ([0-9a-f]{64})\n", record
        )
        literal = re.fullmatch(rb"L ([1-9][0-9]*)\n", record)
        repeat = re.fullmatch(
            rb"R ([1-9][0-9]*:[1-9][0-9]*(?:,[1-9][0-9]*:[1-9][0-9]*)*)\n", record
        )
        if definition is not None:
            ident, size = map(int, definition.group(1, 2))
            data = value[pos : pos + size]
            if (
                ident != len(dictionary) + 1
                or ident > REPEAT_ENTRY_LIMIT
                or size > REPEAT_LINE_LIMIT
                or dictionary_bytes + size > REPEAT_DICTIONARY_LIMIT
                or len(data) != size
                or not data.endswith(b"\n")
                or not data.startswith(REPEAT_PREFIXES)
                or hashlib.sha256(data).hexdigest().encode("ascii") != definition[3]
            ):
                raise ValueError("invalid diagnostic dictionary definition")
            dictionary[ident] = data
            dictionary_bytes += size
            pos += size
            output(data)
        elif literal is not None:
            size = int(literal[1])
            data = value[pos : pos + size]
            if len(data) != size:
                raise ValueError("incomplete diagnostic literal record")
            pos += size
            output(data)
        elif repeat is not None:
            pairs = repeat[1].split(b",")
            if len(pairs) > 64:
                raise ValueError("oversized diagnostic repeat record")
            for pair in pairs:
                ident, repetitions = map(int, pair.split(b":"))
                data = dictionary.get(ident)
                if data is None or repetitions > (expected_bytes - count) // len(data):
                    raise ValueError("unknown or excessive diagnostic repeat")
                for _ in range(repetitions):
                    output(data)
        else:
            raise ValueError("unknown diagnostic repeat record")
    if count != expected_bytes:
        raise ValueError("diagnostic repeat expansion has wrong byte count")
    return {"bytes": count, "sha256": digest.hexdigest()}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--summary-path", type=Path, required=True)
    parser.add_argument("command", nargs=argparse.REMAINDER)
    args = parser.parse_args()
    command = args.command
    if command[:1] == ["--"]:
        command = command[1:]
    if not command or args.summary_path.exists():
        raise ValueError("fresh summary and actual test executable required")
    # The existing outer owner arms its timer before this transport starts.
    # The original test inherits that exact process group, cwd, environment,
    # stdin and stdout. Only its stderr passes through the bounded recogniser.
    child = subprocess.Popen(command, stderr=subprocess.PIPE)
    assert child.stderr is not None

    def emit(value):
        sys.stderr.buffer.write(value)
        sys.stderr.buffer.flush()

    repeated = DiagnosticRepeatLines(emit)
    filtered = TableFilter(repeated.feed)
    while True:
        chunk = os.read(child.stderr.fileno(), 65536)
        if not chunk:
            break
        filtered.feed(chunk)
    table_summary = filtered.finish()
    repeat_summary = repeated.finish()
    summary = dict(table_summary)
    summary["table_stage"] = table_summary
    summary["repeat_stage"] = repeat_summary
    summary["ordinary_stderr_verbatim"] = False
    summary["stderr_losslessly_reconstructible_after_table_filter"] = True
    summary["retained_stderr_bytes"] = repeat_summary["output_bytes"]
    child.stderr.close()
    code = child.wait()
    summary["child_exit_code"] = code
    args.summary_path.write_text(json.dumps(summary, indent=2, sort_keys=True) + "\n")
    if code < 0:
        number = -code
        if number not in (signal.SIGKILL, signal.SIGSTOP):
            signal.signal(number, signal.SIG_DFL)
        os.kill(os.getpid(), number)
        raise RuntimeError("signal did not terminate transport")
    return code


if __name__ == "__main__":
    sys.exit(main())
