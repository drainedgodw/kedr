#!/usr/bin/env python3
"""Capture deterministic long sessions and summarize individual-operation tails."""
import argparse
import csv
import hashlib
import json
import os
import platform
import subprocess
from datetime import datetime, timezone
from pathlib import Path

parser = argparse.ArgumentParser()
parser.add_argument('--operations', type=int, default=10000)
parser.add_argument('--out', type=Path, default=Path('results/local-endurance'))
args = parser.parse_args()
if not 1 <= args.operations <= 2_000_000:
    parser.error('--operations must be between 1 and 2000000')
root = Path(__file__).resolve().parents[1]
out = args.out.resolve()
out.mkdir(parents=True, exist_ok=True)
sources = [root / 'Cargo.toml', root / 'Cargo.lock', root / 'examples/endurance.rs']
sources += sorted((root / 'src').rglob('*.rs'))
metadata = {
    'utc': datetime.now(timezone.utc).isoformat(),
    'platform': platform.platform(),
    'cpu_count': os.cpu_count(),
    'rustc': subprocess.check_output(['rustc', '--version', '--verbose'], text=True),
    'runs_per_case': 1,
    'operations': args.operations,
    'seed': '0x5eeda11',
    'profile': 'release; opt-level=3; default codegen-units; no explicit LTO',
    'quantiles': 'linear interpolation; individual wall-clock samples; no warmup discarded',
    'timing_scope': 'real replacement including threshold compaction; excludes reference/validation, snapshot submission, memory sampling and output',
    'source_sha256': {p.relative_to(root).as_posix(): hashlib.sha256(p.read_bytes()).hexdigest() for p in sources},
}
if platform.system() == 'Linux':
    metadata['cpu_info'] = subprocess.check_output(['lscpu'], text=True)
(out / 'environment.json').write_text(json.dumps(metadata, indent=2) + '\n')
subprocess.run(['cargo', 'build', '--locked', '--release', '--example', 'endurance'], cwd=root, check=True)
# Respect custom Cargo target directories instead of guessing the binary path.
cargo_metadata = json.loads(subprocess.check_output(['cargo', 'metadata', '--locked', '--no-deps', '--format-version', '1'], cwd=root, text=True))
binary = Path(cargo_metadata['target_directory']) / 'release/examples' / ('endurance.exe' if os.name == 'nt' else 'endurance')
summary = []
for case in ['ascii', 'unicode']:
    subprocess.run([str(binary), case, str(args.operations), str(out)], cwd=root, check=True)
    path = out / f'{case}.csv'
    with path.open(newline='') as f:
        values = []
        for i, row in enumerate(csv.DictReader(f)):
            if int(row['operation']) != i:
                raise ValueError('nonconsecutive operation index')
            value = int(row['elapsed_ns'])
            if value < 0:
                raise ValueError('negative latency')
            values.append(value)
    if len(values) != args.operations:
        raise ValueError('incorrect sample count')
    values.sort()
    def quantile(fraction):
        position = (len(values) - 1) * fraction
        lo = int(position)
        hi = min(lo + 1, len(values) - 1)
        return (values[lo] + (values[hi] - values[lo]) * (position - lo)) / 1000
    fixture = json.loads(path.with_suffix('.json').read_text())
    row = {**fixture, 'p50_us': quantile(.5), 'p95_us': quantile(.95), 'p99_us': quantile(.99), 'max_us': quantile(1), 'over_1ms_operations': sum(v > 1_000_000 for v in values), 'raw_sha256': hashlib.sha256(path.read_bytes()).hexdigest()}
    summary.append(row)
    print(f"{case}: p50={row['p50_us']:.3f}us p99={row['p99_us']:.3f}us max={row['max_us']/1000:.3f}ms")
(out / 'summary.json').write_text(json.dumps(summary, indent=2) + '\n')
print('Single-run fixture evidence, not a universal latency or memory guarantee.')
