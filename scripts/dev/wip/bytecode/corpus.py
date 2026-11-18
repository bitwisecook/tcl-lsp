import subprocess, sys, json
sys.path.insert(0, '.claude/skills/bytecode-compare')
import bytecode_compare as bc
res = {}
for name in bc.snippet_names():
    src = bc.SNIPPETS_DIR / f'{name}.tcl'
    row = {}
    for v in ('8.5','8.6','9.0','9.1'):
        ours = subprocess.run(['target/debug/tcl','dis','--dialect','tcl'+v,str(src)],capture_output=True,text=True).stdout
        ref = bc.load_disasm(bc.ref_dir(v)/f'{name}.disasm')
        o=[bc.normalise_instruction(i) for i in bc.extract_instructions(ours)]
        r=[bc.normalise_instruction(i) for i in bc.extract_instructions(ref)]
        row[v]={'match':o==r,'ours':o,'ref':r}
    res[name]=row
json.dump(res, open(sys.argv[1],'w'))
for v in ('8.5','8.6','9.0','9.1'):
    print(v, sum(res[n][v]['match'] for n in res), 'of', len(res))
