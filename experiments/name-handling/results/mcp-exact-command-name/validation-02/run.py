from pathlib import Path
import subprocess,json,time,datetime,sys
p=Path('/tmp/ai-tool-command-info-validation-02');name=sys.argv[1]
cmd=['cargo','test','--offline'] if name=='offline' else ['cargo','test']
cmd+=['-j2','-p','tcl-mcp','--bin','tcl-mcp','--message-format=json','tools::dialect_param_tests::command_info_preserves_exact_literal_names','--','--exact','--nocapture']
start=datetime.datetime.now(datetime.timezone.utc).isoformat(); t=time.monotonic()
with (p/(name+'.stdout')).open('wb') as out,(p/(name+'.stderr')).open('wb') as err:
    proc=subprocess.run(cmd,stdout=out,stderr=err,cwd='/workspace/tcl-lsp')
info={'argv':cmd,'start_utc':start,'end_utc':datetime.datetime.now(datetime.timezone.utc).isoformat(),'exit_code':proc.returncode,'elapsed_seconds':time.monotonic()-t}
(p/(name+'.invocation.json')).write_text(json.dumps(info,indent=2)+'\n')
print(json.dumps(info)); print((p/(name+'.stderr')).read_text(errors='replace')[-1800:]); print((p/(name+'.stdout')).read_text(errors='replace')[-1000:]); sys.exit(proc.returncode)
