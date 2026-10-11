#!/usr/bin/env python3
"""Whole-process Linux peak RSS from independent measured executables."""
import argparse,csv,hashlib,json,subprocess,sys
from pathlib import Path
p=argparse.ArgumentParser()
p.add_argument('--before',type=Path,required=True)
p.add_argument('--after',type=Path,required=True)
p.add_argument('--out',type=Path,required=True)
a=p.parse_args()
if sys.platform!='linux': p.error('Linux /proc VmHWM is required')
rows=[];source={}
for version,root in [('before',a.before.resolve()),('after',a.after.resolve())]:
    subprocess.run(['cargo','build','--locked','--release','--example','footprint'],cwd=root,check=True,stdout=subprocess.DEVNULL)
    source[version]=hashlib.sha256((root/'examples/footprint.rs').read_bytes()).hexdigest()
    for case in ['lines','fragmented','history','clones']:
        for run in range(1,4):
            value=subprocess.check_output([str(root/'target/release/examples/footprint'),case],text=True).strip()
            if not value.isdigit():raise ValueError(f'Missing VmHWM for {version}/{case}: {value!r}')
            rows.append(dict(version=version,workload=case,run=run,peak_rss_kib=int(value)))
a.out.parent.mkdir(parents=True,exist_ok=True)
with a.out.open('w') as stream:
    w=csv.DictWriter(stream,fieldnames=list(rows[0]));w.writeheader();w.writerows(rows)
a.out.with_suffix('.environment.json').write_text(json.dumps({'method':'Linux child executable VmHWM; whole-process peak RSS KiB','runs_per_case':3,'source_sha256':source,'fixture':'4 MiB line-rich text; 4096 ASCII inserts, 1024 history-enabled inserts, or 32 TextBuffer clones'},indent=2)+'\n')
print(f'Captured {len(rows)} independent RSS samples')
