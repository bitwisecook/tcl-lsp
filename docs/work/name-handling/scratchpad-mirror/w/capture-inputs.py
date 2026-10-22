from pathlib import Path
import subprocess,hashlib,json,sys
r=Path('/home/user/tcl-lsp')
paths=subprocess.check_output(['git','ls-files','--cached','--others','--exclude-standard','-z'],cwd=r).decode().split('\0')
facts={}
for name in sorted(set(paths)):
 if not name or name.startswith('docs/work/name-handling/'):continue
 p=r/name
 if p.is_file():facts[name]=hashlib.sha256(p.read_bytes()).hexdigest()
out=Path(sys.argv[1]);out.write_text(json.dumps(facts,sort_keys=True,indent=2)+'\n')
print(json.dumps({'files':len(facts),'snapshot':str(out),'sha256':hashlib.sha256(out.read_bytes()).hexdigest()}))
