from pathlib import Path
import hashlib,json,subprocess,time
p=Path('/workspace/.proofs'); image=Path('/workspace/.pins/naming-owned352/pinned-vm352.elf')
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
expected='10b2ae8bda8a021532b163c36b93ab3ecb18e1374f5e5c3f008145dda6abb345'
assert sha(image)==expected
selector='cmd_dict::native_rmw_fixture_tests::dictionary_command_and_local_cell_publication_match_345_complete_native_processes'
command=[str(image),'--exact',selector,'--test-threads=1','--nocapture']
log=p/'vm-dict-long352-child.log'; started=time.monotonic(); timed_out=False
with log.open('wb') as output:
 try:
  result=subprocess.run(command,stdout=output,stderr=subprocess.STDOUT,timeout=1800)
  code=result.returncode
 except subprocess.TimeoutExpired:
  timed_out=True;code=None
record={'command':command,'image_sha256_before':expected,'image_sha256_after':sha(image),'timeout_seconds':1800,'timed_out':timed_out,'exit':code,'seconds':time.monotonic()-started,'log_sha256':sha(log)}
(p/'vm-dict-long352-child.json').write_text(json.dumps(record,indent=2)+'\n')
print(json.dumps(record))
raise SystemExit(0 if code==0 and not timed_out and record['image_sha256_after']==expected else 1)
