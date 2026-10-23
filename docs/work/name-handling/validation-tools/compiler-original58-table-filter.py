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

"""Preserve test stderr except complete original command-table observer lines."""

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
HEADER = re.compile(rb"ORIGINAL_COMMAND_INSTALLED offset=([0-9]{1,10}) bindings=SourceCommandTable\(")


def possible_header(value):
    if PREFIX.startswith(value):
        return True
    if not value.startswith(PREFIX):
        return False
    suffix = value[len(PREFIX):]
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
        self.table_hash.update(str(self.line_bytes).encode() + b":" + digest.encode() + b"\n")
        self.output(
            b"ORIGINAL_COMMAND_INSTALLED offset=" + self.line_offset
            + b" bindings=SourceCommandTable(SUMMARY count=1 bytes="
            + str(self.line_bytes).encode() + b" sha256=" + digest.encode()
            + b" newline=" + str(int(newline)).encode() + b")\n"
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
                take = chunk if at < 0 else chunk[:at + 1]
                if self.state == "table":
                    self.line_hash.update(take)
                    self.line_bytes += len(take)
                    if at >= 0:
                        self.finish_table(True)
                else:
                    self.output(take)
                chunk = chunk[len(take):]
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
    filtered = TableFilter(emit)
    while True:
        chunk = os.read(child.stderr.fileno(), 65536)
        if not chunk:
            break
        filtered.feed(chunk)
    summary = filtered.finish()
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
