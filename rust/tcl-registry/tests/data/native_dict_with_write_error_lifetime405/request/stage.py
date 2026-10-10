from pathlib import Path
import hashlib,json,difflib
root=Path('/workspace/tcl-lsp');r=Path('/tmp/native-dictionary-with-lifetime405')
sha=lambda b:hashlib.sha256(b).hexdigest()
original_probe=(root/'rust/tcl-vm/tests/data/native_dictionary_body/probe.c').read_bytes()
(r/'original-probe.c').write_bytes(original_probe)
rows=dict(row.split('\t') for row in (root/'rust/tcl-vm/tests/data/native_dictionary_body/cases.tsv').read_text().splitlines())
source=bytes.fromhex(rows['compiled-with-dict-write-error'])
(r/'original-source.tcl').write_bytes(source)
observed=original_probe.decode()
observed=observed.replace('#include <stdio.h>', '#include <stdio.h>\nextern int tclLspWithEntered, tclLspWithBeforeRefcount, tclLspWithAllocated;\nextern int tclLspWithFailed, tclLspWithSameCell, tclLspWithDefinedScalar, tclLspWithFinalFree;')
needle='int code=SCRIPT(active,o);report("native-result",code);'
assert observed.count(needle)==1
observed=observed.replace(needle,'int code=SCRIPT(active,o);fprintf(stderr,"WITH_BODY_RETURN|entered=%d|before_refcount=%d|allocated=%d|failed=%d|same_cell=%d|defined_scalar=%d|final_free=%d\\n",tclLspWithEntered,tclLspWithBeforeRefcount,tclLspWithAllocated,tclLspWithFailed,tclLspWithSameCell,tclLspWithDefinedScalar,tclLspWithFinalFree);report("native-result",code);')
(r/'observed-probe.c').write_text(observed)
required={str(root/'rust/tcl-vm/tests/data/native_dictionary_body/probe.c'):sha(original_probe),str(root/'rust/tcl-vm/tests/data/native_dictionary_body/cases.tsv'):sha((root/'rust/tcl-vm/tests/data/native_dictionary_body/cases.tsv').read_bytes())}
versions=[]
for v in ['8.6.18','9.0.4','9.1.0']:
 sdk=root/'tmp'/('tcl'+v); d=r/v;d.mkdir(exist_ok=True)
 archive=sdk/'unix'/('libtcl'+'.'.join(v.split('.')[:2])+'.a')
 versions.append({'version':v,'sdk':str(sdk),'archive':str(archive)})
 for basename in ['tclDictObj.c','tclObj.c']:
  p=sdk/'generic'/basename;old=p.read_text();new=old
  if basename=='tclDictObj.c':
   decl='\nTcl_Obj *tclLspWithWatch = NULL;\nint tclLspWithEntered=0, tclLspWithBeforeRefcount=-1, tclLspWithAllocated=-1;\nint tclLspWithFailed=0, tclLspWithSameCell=-1, tclLspWithDefinedScalar=-1, tclLspWithFinalFree=0;\n'
   new=new.replace('#include "tclInt.h"','#include "tclInt.h"'+decl,1)
   start=new.index('\nTclDictWithFinish(\n');end=new.index('\n}\n',start)+3
   function=new[start:end]
   needle='    if (TclPtrSetVarIdx(interp, varPtr, arrayPtr, part1Ptr, part2Ptr,\n\t    dictPtr, TCL_LEAVE_ERR_MSG, index) == NULL) {'
   assert function.count(needle)==1,(v,'setter scope')
   prefix='    tclLspWithEntered++;\n    tclLspWithBeforeRefcount = dictPtr->refCount;\n    tclLspWithAllocated = allocdict;\n    tclLspWithWatch = dictPtr;\n    fprintf(stderr,"WITH_BEFORE_SET|refcount=%d|allocated=%d\\n",tclLspWithBeforeRefcount,tclLspWithAllocated);\n'
   failure='\n        tclLspWithFailed++;\n        tclLspWithSameCell = varPtr->value.objPtr == dictPtr;\n        tclLspWithDefinedScalar = TclIsVarScalar(varPtr) && !TclIsVarUndefined(varPtr);\n        fprintf(stderr,"WITH_SET_ERROR|same_cell=%d|defined_scalar=%d|final_free=%d\\n",tclLspWithSameCell,tclLspWithDefinedScalar,tclLspWithFinalFree);'
   function=function.replace(needle,prefix+needle+failure)
   new=new[:start]+function+new[end:]
  else:
   new=new.replace('#include "tclInt.h"','#include "tclInt.h"\nextern Tcl_Obj *tclLspWithWatch;\nextern int tclLspWithFinalFree;',1)
   import re
   marker=re.compile(r'(TclFreeObj\(\n    Tcl_Obj \*objPtr\)[^\n]*\n\{\n)')
   new,n=marker.subn(r'\1    if (objPtr == tclLspWithWatch) {\n        tclLspWithFinalFree++;\n        tclLspWithWatch = NULL;\n        fprintf(stderr,"WITH_FINAL_FREE|count=%d\\n",tclLspWithFinalFree);\n    }\n',new)
   assert n==2,(v,n)
  (d/('original-'+basename)).write_text(old);(d/('observed-'+basename)).write_text(new)
  (d/(basename+'.patch')).write_text(''.join(difflib.unified_diff(old.splitlines(True),new.splitlines(True),fromfile='original/'+basename,tofile='observed/'+basename)))
  required[str(p)]=sha(p.read_bytes())
 for p in [archive,sdk/'generic/tcl.h',sdk/'unix/Makefile']:
  required[str(p)]=sha(p.read_bytes())
for p in r.rglob('*'):
 if p.is_file() and p.name!='stage.py':required[str(p)]=sha(p.read_bytes())
request={'id':'native-dictionary-with-lifetime405','question':'For the exact original compiled-with-dict-write-error source from native329, does TclDictWithFinish select a newly allocated dictionary and reach final free after failed writeback while the Var still selects that header? Are later public bytes observed after that release?','purpose':'Independent WithFinish selection/release purpose, not the update-end branch in native398/399. Unchanged original guest source/probe; separate observer source/archive. Observer samples actual pre-set refcount/allocdict and post-set pointer/flags without dereferencing a released header, holding it or changing Tcl ownership. Final-free events and later process exits/signals are retained. A post-free result/status is only a process observation, never defined storage or execution/effect authority.','cases':[19,10,9],'versions':versions,'required_sha256':required,'output_directory':'/tmp/native-dictionary-with-lifetime405-capture'}
(r/'request.json').write_text(json.dumps(request,indent=2)+'\n')
print('source_sha256',sha(source),'request_sha256',sha((r/'request.json').read_bytes()))
