"""Preserve REPORT_ substrings even when libtest prefixes the same line."""
from pathlib import Path
import datetime
import hashlib
import json
import sys
root = Path(sys.argv[1])
rows = []
for log in sorted(root.glob('*.log')):
    observations = []
    for line_number, raw in enumerate(log.read_text().splitlines(), 1):
        start = raw.find('REPORT_')
        if start >= 0:
            observations.append({'raw_line_number': line_number, 'report': raw[start:], 'raw_line': raw})
    process = json.loads(Path(str(log)+'.process.json').read_text())
    result = json.loads(Path(str(log)+'.result.json').read_text())
    rows.append({'label': log.stem, 'raw_log': str(log), 'raw_sha256': hashlib.sha256(log.read_bytes()).hexdigest(),
                 'observations': observations, 'process': process, 'result': result})
out = {'prepared_utc': datetime.datetime.now(datetime.timezone.utc).isoformat(),
       'scope': 'Pure supplementary postprocessing of immutable raw log; original runner/plan/assertions unchanged',
       'suite_final': json.loads((root/'final.json').read_text()),
       'runtime_watchdog': json.loads((root/'runtime-watchdog.json').read_text()),
       'results': rows}
(root/'report-observations-complete.json').write_text(json.dumps(out, indent=2)+'\n')
