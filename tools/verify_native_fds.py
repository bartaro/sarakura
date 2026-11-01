"""Verify the exact public SARAKURA commit and exercise both FDS rules natively."""
from pathlib import Path
import os,subprocess,json,hashlib,platform,re,datetime,tempfile

ROOT=Path(__file__).resolve().parents[1];SOURCE=ROOT;OUT=ROOT/'ci-output/sarakura';OUT.mkdir(parents=True,exist_ok=True)
ENV=dict(os.environ,CARGO_INCREMENTAL='0');commands=[]
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def run(name,args,env=ENV):
 with (OUT/(name+'.log')).open('wb') as f:p=subprocess.run(args,cwd=SOURCE,env=env,stdout=f,stderr=subprocess.STDOUT,timeout=1500)
 text=(OUT/(name+'.log')).read_text(errors='replace');commands.append({'name':name,'arguments':args,'exit_code':p.returncode,'test_count':sum(map(int,re.findall(r'test result: ok\. (\d+) passed;',text)))})
 if p.returncode:print(text[-6000:]);raise SystemExit(p.returncode)
 return text
commit=subprocess.check_output(['git','rev-parse','HEAD'],cwd=SOURCE,text=True).strip();assert commit==os.environ['SARAKURA_SOURCE_COMMIT']
tracked=subprocess.check_output(['git','ls-files','-z'],cwd=SOURCE).decode().split('\0')
sources=[]
for rel in filter(None,tracked):
 if Path(rel).suffix not in ['.rs','.toml','.lock','.json']:continue
 if not (rel.startswith('crates/') or '/' not in rel and rel.startswith('Cargo.')):continue
 data=(SOURCE/rel).read_bytes();assert subprocess.check_output(['git','show','HEAD:'+rel],cwd=SOURCE)==data
 sources.append({'path':rel,'sha256':sha(SOURCE/rel)})
toolchain=run('toolchain',['rustc','--version','--verbose'])
run('tests',['cargo','test','--locked','--workspace','--all-targets'])
# The pre-existing public core has style warnings under Rust 1.93. Keep them in
# the Clippy log while tests, compilation and native FDS assertions remain gates.
run('clippy',['cargo','clippy','--locked','--workspace','--all-targets'])
release=dict(ENV)
if platform.system()=='Windows':release['RUSTFLAGS']='-C target-feature=+crt-static'
run('release',['cargo','build','--locked','--release','-p','sarakura-cli'],release)
exe=SOURCE/'target/release'/('sarakura.exe' if platform.system()=='Windows' else 'sarakura')
run('cli-help',[str(exe),'--help']);native={'checks':[{'case':'cli-help','passed':True}]}
for hardware in ['gb','fc']:
 events=SOURCE/'examples'/hardware/('kokura_diagnostic_events.jsonl' if hardware=='gb' else 'kurosaki_diagnostic_events.jsonl')
 run(hardware+'-analyze',[str(exe),hardware,'analyze','--metadata',str(SOURCE/f'examples/{hardware}/kitaq{hardware}_build_metadata.json'),'--events',str(events),'--out',str(OUT/hardware),'--fail-on','never'])
 run(hardware+'-validate',[str(exe),'validate',str(OUT/hardware/'ai_diagnostics.json'),'--strict'])
 native['checks'].append({'case':hardware+'-analyze-validate','passed':True})
with tempfile.TemporaryDirectory() as folder:
 work=Path(folder);(work/'meta.json').write_text('{}')
 events=[{'event_type':kind,'message':'Original synthetic FDS verification','frame':1} for kind in ['FDS_MEDIA_CRC_ERROR','FDS_MEDIA_WRITE_PROTECTED']]
 (work/'events.jsonl').write_text(''.join(json.dumps(e)+'\n' for e in events))
 run('fds-analyze',[str(exe),'fc','analyze','--metadata',str(work/'meta.json'),'--events',str(work/'events.jsonl'),'--out',str(work/'report'),'--no-repro-bundle'])
 diagnostics=json.loads((work/'report/ai_diagnostics.json').read_text())
 ids={d['catalog_id'] for d in diagnostics['diagnostics']};assert {'FC_FDS_MEDIA_CRC_ERROR','FC_FDS_MEDIA_WRITE_PROTECTED'}<=ids
dependencies=subprocess.check_output(['otool','-L',str(exe)] if platform.system()=='Darwin' else ['ldd',str(exe)],text=True) if platform.system()!='Windows' else None
report={'passed':True,'utc':datetime.datetime.now(datetime.timezone.utc).isoformat(),'source_commit':commit,'validation_commit':os.environ.get('GITHUB_SHA'),'system':platform.system(),'machine':platform.machine(),'toolchain':toolchain,'commands':commands,'source_files':sources,'native_smoke':native,'fds_rules_verified':sorted(ids),'binary_sha256':sha(exe),'executable_dependencies':dependencies}
(OUT/'verification.json').write_text(json.dumps(report,indent=2)+'\n');(OUT/exe.name).write_bytes(exe.read_bytes())
print(json.dumps({'passed':True,'tests':sum(x['test_count'] for x in commands),'system':platform.system()}))
