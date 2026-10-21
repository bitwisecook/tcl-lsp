"""Write a 30-minute progress report from the audit snapshot plus a JSON delta file."""
from pathlib import Path
import datetime,json,sys

from _paths import R
D=R/'docs/work/name-handling'
delta=json.loads(Path(sys.argv[1]).read_text())
prior=json.loads((D/'progress-report-20261017T1739Z.json').read_text())
now=datetime.datetime.now(datetime.timezone.utc)
report={k:prior[k] for k in ['original_LSP','baseline','annex','effective','scope','stage_order']}
for section,values in delta.pop('counts',{}).items():report[section].update(values)
report.update(utc=now.isoformat(),scheduled_report_utc=delta.pop('scheduled'),report_cadence_minutes=30,delivery='Chat and repository only; external guide unchanged.')
report.update(delta)
report.setdefault('green_PR',False);report.setdefault('merge_or_issue_closure',False)
path=D/('progress-report-'+now.strftime('%Y%m%dT%H%MZ')+'.json')
assert not path.exists()
path.write_text(json.dumps(report,indent=2)+'\n');print(path)
