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

"""Bound the existing client's wire/capture storage without changing server answers."""
import json,threading,os
CAP=int(os.environ["TCL_LSP_WIP_WIRE_CAP"])
if not 0<CAP<=20*1024**2:raise ValueError("invalid remaining aggregate wire cap")

def unique(pairs):
    value={}
    for key,item in pairs:
        if key in value:raise ValueError("duplicate JSON key: "+key)
        value[key]=item
    return value

def client_class(base):
    class BoundedList(list):
        def __init__(self,owner):super().__init__();self.owner=owner
        def append(self,item):
            count=len(json.dumps(item).encode())
            with self.owner.budget_lock:
                self.owner.capture_bytes+=count
                if self.owner.capture_bytes>CAP:self.owner.capture_error='decoded capture exceeded20MiB';raise ValueError(self.owner.capture_error)
            super().append(item)
    class Client(base):
        def __init__(self,*args,**kwargs):
            capture_directory=kwargs.pop('capture_directory');self.raw_stdout=(capture_directory/'server.stdout.frames').open('wb',buffering=0);self.raw_stderr=(capture_directory/'server.stderr').open('wb',buffering=0)
            super().__init__(*args,**kwargs);self.budget_lock=threading.Lock();self.wire_bytes=0;self.capture_bytes=0;self.capture_error=None;self._notifications=BoundedList(self);self._stderr_lines=BoundedList(self)
        def account(self,count):
            with self.budget_lock:
                self.wire_bytes+=count
                if self.wire_bytes>CAP:self.capture_error='server combined wire output exceeded20MiB';raise ValueError(self.capture_error)
        def _read_message(self):
            try:return self._bounded_read_message()
            except (ValueError,OSError) as error:self.capture_error=str(error);raise
        def _bounded_read_message(self):
            stream=self.process.stdout;length=None
            while True:
                line=stream.readline(8193)
                if not line:return None
                self.account(len(line));self.raw_stdout.write(line)
                if len(line)>8192:self.capture_error='oversized LSP header';raise ValueError(self.capture_error)
                if line in [b'\n',b'\r\n']:break
                key,separator,value=line.partition(b':')
                if separator and key.lower()==b'content-length':
                    if length is not None:raise ValueError('duplicate content-length')
                    length=int(value.strip())
            if length is None or length<=0 or length>CAP:raise ValueError('invalid LSP content-length')
            self.account(length);body=bytearray()
            while len(body)<length:
                chunk=stream.read(min(65536,length-len(body)))
                if not chunk:raise ValueError('incomplete LSP body')
                self.raw_stdout.write(chunk);body.extend(chunk)
            value=json.loads(body,object_pairs_hook=unique)
            if not isinstance(value,dict):raise ValueError('LSP message must be an object')
            return value
        def _stderr_loop(self):
            try:
                while True:
                    line=self.process.stderr.readline(65536)
                    if not line:break
                    self.account(len(line));self.raw_stderr.write(line)
                    with self._lock:self._stderr_lines.append(line.decode('utf-8',errors='replace').rstrip('\r\n'))
            except (OSError,ValueError) as error:self.capture_error=str(error)
    return Client
