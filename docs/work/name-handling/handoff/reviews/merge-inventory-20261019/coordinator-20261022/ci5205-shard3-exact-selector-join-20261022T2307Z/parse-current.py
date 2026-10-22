from __future__ import annotations
import hashlib,json,re,sys
from collections import defaultdict,Counter
from pathlib import Path
raw_path=Path('/tmp/ci5205-shard3-120061100614.log')
text=raw_path.read_text(encoding='utf-8-sig')
raw_hash=hashlib.sha256(raw_path.read_bytes()).hexdigest()
ansi=re.compile(r'\x1b\[[0-?]*[ -/]*[@-~]')
plain=ansi.sub('',text)
pattern=re.compile(r'^(\S+Z)\s+(PASS|FAIL|SKIP|ABORT|TIMEOUT)\s+\[([^\]]+)\]\s+\(\s*(\d+)\s*/\s*(\d+)\)\s+(.+?)\s*$',re.M)
by_ordinal=defaultdict(list)
for m in pattern.finditer(plain):
    timestamp,status,duration,ordinal,total,rendered=m.groups()
    fields=rendered.split()
    if len(fields)<2:
        raise SystemExit(f'unparseable status identity: {m.group(0)!r}')
    label=fields[0]
    selector=' '.join(fields[1:])
    if '::' in label:
        package,binary_label=label.split('::',1)
        binary_id=f'{package}::{binary_label}'
    else:
        package=label
        binary_label=None
        binary_id=package
    by_ordinal[int(ordinal)].append({
        'status':status,'timestamp':timestamp,'duration':duration,
        'reporter_ordinal':int(ordinal),'reporter_total':int(total),
        'package':package,'binary_label_as_rendered':binary_label,
        'binary_id':binary_id,'selector':selector,
    })
if not by_ordinal: raise SystemExit('no named nextest status rows parsed')
if set(by_ordinal)!=set(range(1,3898)):
    missing=sorted(set(range(1,3898))-set(by_ordinal)); extra=sorted(set(by_ordinal)-set(range(1,3898)))
    raise SystemExit(f'ordinal coverage mismatch missing={missing[:20]} extra={extra[:20]}')
records=[]; duplicate_ordinals=[]
for ordinal,rows in sorted(by_ordinal.items()):
    identities={(r['status'],r['package'],r['binary_label_as_rendered'],r['binary_id'],r['selector'],r['reporter_total']) for r in rows}
    if len(identities)!=1:
        raise SystemExit(f'conflicting repeated ordinal {ordinal}: {rows}')
    first=rows[0]
    first['duplicate_summary_lines']=len(rows)-1
    if len(rows)>1:
        if first['status']!='FAIL': raise SystemExit(f'only repeated failures expected; ordinal={ordinal} rows={rows}')
        duplicate_ordinals.append(ordinal)
    records.append(first)
counts=Counter(r['status'] for r in records)
if counts != Counter({'PASS':3053,'FAIL':844}): raise SystemExit(f'dedup outcome count does not match raw summary: {counts}')
if len(duplicate_ordinals)!=844: raise SystemExit(f'expected 844 repeated FAIL summary ordinals, got {len(duplicate_ordinals)}')
summary=re.search(r'Summary\s+\[[^]]+\]\s+(\d+)\s+tests run:.*?(\d+)\s+passed.*?(\d+)\s+failed.*?(\d+)\s+skipped',plain)
if not summary: raise SystemExit('final summary line not found')
tests,passed,failed,skipped=map(int,summary.groups())
if (tests,passed,failed,skipped)!=(3897,3053,844,1): raise SystemExit(f'unexpected final summary: {(tests,passed,failed,skipped)}')
receipt={
 'schema':'nextest-human-reporter-observed-identities-v1',
 'recorded_at_utc':'2026-10-22T23:04Z',
 'run':{'number':5205,'head_sha':'2b93470548313b99d29af108d8b38d52b2db7ceb','workflow_event':'verify'},
 'job':{'name':'rust-tests-shard (3/5)','id':120061100614,'nextest_run_id':'43cde985-03be-460a-a4b5-cf88e50ec0f2','profile':'default'},
 'source_log':{'path':str(raw_path),'sha256':raw_hash,'bytes':raw_path.stat().st_size},
 'summary':{'tests_run':tests,'passed':passed,'failed':failed,'skipped':skipped,'named_status_records':len(records),'named_status_counts':dict(counts),'reported_repeated_failure_ordinals':len(duplicate_ordinals),'ordinal_coverage':'1..3897 complete'},
 'summary_notes':['Every failure is printed twice in raw nextest log: once at its execution ordinal and once in final failure list. Identical entries were deduplicated by ordinal only; conflicting duplicates fail closed.','The summary separately reports 1 skipped test, while named status ordinals cover all 3897 runs and contain only 3053 PASS + 844 FAIL. No named SKIP selector is available; do not assign the summary-level skip to an identity.','The nextest run says 1 test and 43 binaries skipped during startup; those have no named result identities in this log.'],
 'records':records,
}
out=Path('/tmp/ci5205-shard3-exact-join/current-outcomes.json')
out.write_text(json.dumps(receipt,indent=2,ensure_ascii=False)+'\n')
print(json.dumps({'raw_log_sha256':raw_hash,'raw_bytes':raw_path.stat().st_size,'records':len(records),'status_counts':dict(counts),'repeated_failure_ordinals':len(duplicate_ordinals),'summary':{'tests':tests,'pass':passed,'fail':failed,'skip':skipped},'current_json_sha256':hashlib.sha256(out.read_bytes()).hexdigest(),'unique_binary_ids':len({x['binary_id'] for x in records})},indent=2))
