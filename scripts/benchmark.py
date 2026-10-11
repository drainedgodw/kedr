#!/usr/bin/env python3
"""Capture raw benchmark runs plus the exact source/environment fingerprint."""
import argparse, hashlib, json, os, platform, subprocess
from datetime import datetime, timezone
from pathlib import Path

parser = argparse.ArgumentParser()
parser.add_argument('--runs', type=int, default=3)
parser.add_argument('--out', type=Path, default=Path('results'))
args = parser.parse_args()
if not 1 <= args.runs <= 20:
    parser.error('--runs must be between 1 and 20')
root = Path(__file__).resolve().parents[1]
out = args.out.resolve()
out.mkdir(parents=True, exist_ok=True)
sources = [root/'Cargo.toml', root/'Cargo.lock'] + sorted((root/'src').glob('*.rs')) + sorted((root/'benches').glob('*.rs'))
fingerprints = {p.relative_to(root).as_posix(): hashlib.sha256(p.read_bytes()).hexdigest() for p in sources}
metadata = {'utc': datetime.now(timezone.utc).isoformat(), 'platform': platform.platform(),
            'cpu_count': os.cpu_count(), 'rustc': subprocess.check_output(['rustc','--version','--verbose'],text=True),
            'profile': 'bench; thin LTO; codegen-units=1', 'runs': args.runs,
            'warmup_batches': 10, 'samples_per_case_per_run': 40, 'source_sha256': fingerprints}
if platform.system() == 'Linux':
    metadata['cpu_info'] = subprocess.check_output(['lscpu'],text=True)
(out/'environment.json').write_text(json.dumps(metadata,indent=2)+'\n')
for run in range(1,args.runs+1):
    target=out/f'run-{run}.csv'
    with target.open('w') as stream, (out/f'run-{run}.log').open('w') as log:
        subprocess.run(['cargo','bench','--locked','--bench','core'],cwd=root,stdout=stream,stderr=log,check=True)
    print(f'Completed run {run}/{args.runs}')
