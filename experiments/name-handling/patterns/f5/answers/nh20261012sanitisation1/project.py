#!/usr/bin/env python3
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Project transport receipts without changing captured Tcl operations."""
import ipaddress
import io
import subprocess
import tarfile
import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[6]
BASE = Path(__file__).resolve().parents[1]
RUNS = ['repair2', 'harness1', 'harness2', 'collision1', 'nativecap1', 'aplrepair1']
ADDRESS = re.compile(r'(?<![\w.])(?:\d{1,3}\.){3}\d{1,3}(?![\w.])')
OMISSION = '[omitted unrelated appliance inventory]\n'
NOTE = ('Retained sanitised transport/deployment projection: documentation address, NHO-only logs, '
        'and omitted hardware/unrelated inventory. Tcl operation and result bytes are preserved. '
        'Historical executed-input hashes describe original execution, not these projected deployment fixtures.')


def text_projection(text):
    def address(match):
        try:
            value = ipaddress.ip_address(match.group())
        except ValueError:
            return match.group()
        return '192.0.2.10' if value.is_private and not value.is_loopback and str(value) not in ['0.0.0.0', '255.255.255.255', '255.255.255.0'] else match.group()
    return ADDRESS.sub(address, text)


def project_json(value, key=''):
    if isinstance(value, dict):
        result = {}
        for k, v in value.items():
            if isinstance(v, dict) and str(v.get('path', '')).endswith('/browser.png'):
                continue
            result[k] = project_json(v, k)
        if 'line_hex' in result and 'path' in result:
            row = bytes.fromhex(result['line_hex']).decode('utf-8', errors='strict')
            if 'NHO|' in row:
                row = row[row.index('NHO|'):]
                path = ROOT / result['path']
                lines = path.read_text().splitlines() if path.exists() else []
                indices = [i + 1 for i, line in enumerate(lines) if line == row]
                if indices:
                    result['line'] = indices[0]
                    result['line_hex'] = row.encode().hex()
        return result
    if isinstance(value, list):
        return [project_json(v, key) for v in value if not (isinstance(v, dict) and str(v.get('path', '')).endswith('/browser.png'))]
    if isinstance(value, str):
        if key in ['operation', 'setup', 'value_hex', 'result_hex', 'chunk_hex', 'line_hex', 'errorinfo', 'errorcode']:
            return value
        return text_projection(value)
    return value


def main():
    raw = subprocess.check_output(['git', 'archive', 'HEAD', *[str((BASE / ('nh20261012' + n)).relative_to(ROOT)) for n in RUNS]], cwd=ROOT)
    archive = tarfile.open(fileobj=io.BytesIO(raw))
    baseline = {m.name: archive.extractfile(m).read() for m in archive.getmembers() if m.isfile()}
    counts = {'runs': len(RUNS), 'files_changed': 0, 'screenshots_removed': 0}
    for run in RUNS:
        directory = BASE / ('nh20261012' + run)
        for path in sorted(directory.rglob('*')):
            if not path.is_file() or path.name == 'MANIFEST.sha256' or path.suffix == '.json':
                continue
            if path.suffix == '.png':
                path.unlink(); counts['screenshots_removed'] += 1; continue
            current = path.read_text()
            old = baseline[str(path.relative_to(ROOT))].decode()
            if path.name in ['platform.stdout', 'destination-inventory.stdout', 'virtual-inventory.stdout']:
                new = OMISSION
            elif path.name == 'runtime-logs.stdout':
                new = ''.join(line[line.index('NHO|'):] + '\n' for line in old.splitlines() if 'NHO|' in line)
            else:
                new = text_projection(old)
            if new != current:
                path.write_text(new); counts['files_changed'] += 1
        for path in sorted(directory.rglob('*.json')):
            current = json.loads(path.read_text())
            value = json.loads(baseline[str(path.relative_to(ROOT))])
            new = project_json(value)
            if path.name == 'platform.json' and isinstance(new, dict):
                for key in ['stdout', 'stderr']:
                    if key in new:
                        new[key] = OMISSION if key == 'stdout' else ''
            if path.name == 'provenance.json':
                new['retained_projection'] = NOTE
            if path.name == 'records.json':
                for record in new:
                    record['retained_projection'] = NOTE
            if new != current:
                path.write_text(json.dumps(new, indent=2) + '\n'); counts['files_changed'] += 1
        readme = directory / 'README.md'
        text = readme.read_text().replace('and cropped form screenshot are retained; the screenshot contains only the template form and callback fields.', 'are retained; the screenshot was removed under the evidence privacy policy.')
        if NOTE not in text:
            text += '\n' + NOTE + '\n'
        readme.write_text(text)
    (Path(__file__).parent / 'projection-counts.json').write_text(json.dumps(counts, indent=2) + '\n')
    print(json.dumps(counts))


if __name__ == '__main__':
    main()
