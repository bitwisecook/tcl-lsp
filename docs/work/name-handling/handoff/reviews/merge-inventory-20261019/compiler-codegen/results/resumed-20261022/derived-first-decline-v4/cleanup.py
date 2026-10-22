from pathlib import Path
import argparse, hashlib, json
parser=argparse.ArgumentParser(description='Exact owned diagnostic cleanup; run only in coordinator source window, no Cargo/rustc.')
parser.add_argument('--root',required=True)
parser.add_argument('--apply',action='store_true')
args=parser.parse_args();root=Path(args.root)
target=root/'rust/tcl-spectcl/tests/pack_source_e2e.rs'
module=root/'rust/tcl-spectcl/tests/support/derived_first_decline.rs'
def sha(data):return hashlib.sha256(data).hexdigest()
original=target.read_bytes();support=module.read_bytes()
assert sha(original)=='e1ed4fb25d4aeba91df7e0185dec4b029d11bd7dda8d7093750bdc22f76307b7','STOP: target differs from exact owned observer version'
assert sha(support)=='6f430e9f4ef863ba559714749f5eddec5f09f07abe549dbb654c7a33d5f07bc1','STOP: support differs from exact owned observer bytes'
declaration=b'#[path = "support/derived_first_decline.rs"]\nmod derived_first_decline;\n\n'
call=b'    derived_first_decline::report(dialect, call, &registry, &unit);\n'
assert original.count(declaration)==1 and original.count(call)==1
restored=original.replace(declaration,b'',1).replace(call,b'',1)
assert sha(restored)=='a46e9deccdc44e9014b053069130bf3eb7e85a92e836705c0ef7385eaa0a8ff1','STOP: exact original recovery failed'
if args.apply:
 target.write_bytes(restored)
 module.unlink()
 assert target.read_bytes()==restored and not module.exists()
print(json.dumps({'status':'exact owned observer cleaned' if args.apply else 'check-only, no mutation','root':str(root),'target_restored_sha256':sha(restored),'module_removed':args.apply,'other_paths_mutated':False}))
