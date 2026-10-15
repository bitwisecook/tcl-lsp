from pathlib import Path
import argparse, hashlib, json
p=argparse.ArgumentParser();p.add_argument('--current-provider',action='store_true');a=p.parse_args();here=Path(__file__).resolve().parent
sha=lambda q:hashlib.sha256(q.read_bytes()).hexdigest()
for line in (here/'SHA256SUMS').read_text().splitlines():
 h,n=line.split(None,1);assert sha(here/n.strip())==h,n
for capture in [here,here/'trace-add']:
 m=json.loads((capture/'manifest.json').read_text());assert sha(capture/'probe.tcl')==m['probe_sha256']
 for row in m['rows']:
  assert row['original_files_before']==row['original_files_after']
  for channel in ['stdout','stderr']:assert sha(capture/row[channel+'_file'])==row[channel+'_sha256']
  if a.current_provider:
   for file,h in row['original_files_before'].items():assert sha(Path(file))==h,file
for row in json.loads((here/'source-provenance.json').read_text())['anchors']:
 assert sha(here/row['path'])==row['sha256']
 if a.current_provider:assert sha(Path(row['original']))==row['original_sha256']
print('PASS retained archive, two original captures and five source anchors; no provider executed')
