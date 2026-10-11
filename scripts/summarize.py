"""Reconcile raw timed batches; independently verify all medians."""
import argparse, csv, json, math, statistics
from pathlib import Path
import pandas as pd

parser=argparse.ArgumentParser()
parser.add_argument('--raw',type=Path,default=Path('results'))
parser.add_argument('--out',type=Path,default=Path('results'))
args=parser.parse_args()
raw=args.raw
out=args.out
out.mkdir(parents=True,exist_ok=True)
frames=[]
independent={}
for run in range(1,4):
    path=raw/f'run-{run}.csv'
    frame=pd.read_csv(path)
    assert len(frame)==960 and not frame.isna().any().any()
    assert not frame.duplicated(['workload','engine','text_bytes','sample']).any()
    assert (frame.batch_operations>0).all() and (frame.elapsed_ns>0).all()
    frame['run']=run; frames.append(frame)
    with path.open() as stream:
        for row in csv.DictReader(stream):
            key=(row['workload'],row['engine'],int(row['text_bytes']))
            independent.setdefault(key,[]).append(int(row['elapsed_ns'])/int(row['batch_operations'])/1000)
all_rows=pd.concat(frames,ignore_index=True)
all_rows['us_per_operation']=all_rows.elapsed_ns/all_rows.batch_operations/1000
keys=['workload','engine','text_bytes']
records=[]
for key, rows in all_rows.groupby(keys,sort=True):
    values=rows.us_per_operation
    median=float(values.median())
    assert len(values)==120 and math.isclose(median,statistics.median(independent[key]),rel_tol=1e-12)
    records.append(dict(zip(keys,key))|{'samples':len(values),'median_us':median,'sample_p95_us':float(values.quantile(.95)),
                                      'min_us':float(values.min()),'max_us':float(values.max())})
summary=pd.DataFrame(records)
summary.to_csv(out/'kedr-summary.csv',index=False)
all_rows.to_csv(out/'kedr-normalized.csv',index=False)
lookup={(r['workload'],r['engine'],r['text_bytes']):r for r in records}
comparisons=[]
for workload in ['local_insert_delete','scattered_insert_delete','fragmented_insert']:
    sizes=sorted(summary.loc[summary.workload==workload,'text_bytes'].unique())
    for size in sizes:
        core=lookup[(workload,'piece_table',int(size))]; baseline=lookup[(workload,'std_string',int(size))]
        comparisons.append({'workload':workload,'text_bytes':int(size),'core_median_us':core['median_us'],
                            'string_median_us':baseline['median_us'],'string_over_core_ratio':baseline['median_us']/core['median_us']})
evidence={'source':'3 release process runs; no excluded samples','raw_rows':len(all_rows),'cases':len(records),
          'grain':'one timed batch; reported latency is a median of batch-normalized means',
          'validation':'all case medians independently reproduced using csv + statistics.median',
          'results':records,'comparisons':comparisons,'comparison_plan':[]}
(out/'kedr-evidence.json').write_text(json.dumps(evidence,indent=2)+'\n')
print(f'{len(all_rows)} rows; {len(records)} cases; {len(comparisons)} paired comparisons; all medians independently verified')
for c in comparisons:
    print(f"{c['workload']:26} {c['text_bytes']:8}B  core {c['core_median_us']:9.3f}us  String {c['string_median_us']:9.3f}us  String/core {c['string_over_core_ratio']:.2f}x")
for r in records:
    if r['workload'] not in ['local_insert_delete','scattered_insert_delete','fragmented_insert']:
        print(f"{r['workload']:26} {r['median_us']:10.3f}us  sample-p95 {r['sample_p95_us']:10.3f}us")

local=next(c for c in comparisons if c['workload']=='local_insert_delete' and c['text_bytes']==1048576)
fragmented=next(c for c in comparisons if c['workload']=='fragmented_insert')
assert local['core_median_us'] < local['string_median_us']
assert fragmented['core_median_us'] > fragmented['string_median_us']
evidence['comparison_plan']=[
    {'grain':'1 MiB initial buffer; matched ASCII changes; history off','population':'120 batch samples per scenario and engine',
     'unit':'milliseconds per primitive text change',
     'finding':'Local insert/delete favors Kedr; sustained fragmented inserts favor String',
     'derivation':'local pair time divided by 2; fragmented time per single insert; median of normalized batch means',
     'disposition':'chart','reason':'Preserves the opposing fragmentation result instead of advertising only a favorable case'},
    {'grain':'all four buffer sizes, both editing patterns','population':'matched cases','unit':'microseconds per pair',
     'finding':'String wins the 1 KiB cases; Kedr wins larger restored-pair cases',
     'derivation':'median(elapsed_ns / batch_operations) / 1000','disposition':'table','reason':'Full size breakdown remains in the benchmark report'},
    {'grain':'cursor/document/project scenarios','population':'120 batch samples each','unit':'microseconds per named workload',
     'finding':'Near-end cursor mapping is a measured optimization target','derivation':'median of normalized batch means',
     'disposition':'table','reason':'Different workloads have no common baseline; avoid implying an IDE-level latency comparison'}]
(out/'kedr-evidence.json').write_text(json.dumps(evidence,indent=2)+'\n')
derivation='median(elapsed_ns / batch_operations / edits_per_operation) / 1000000; local pair edits_per_operation=2, fragmented inserts=1'
spec={'version':1,'form':'categorical-bars','title':'Фрагментация — слабое место первого ядра',
      'aside':'Буфер 1 MiB, undo отключён; медиана времени на правку',
      'mode':'grouped','labels':['Локальная вставка/удаление','1024 разнесённые вставки'],
      'axis':{'zero':True},'derived':True,'derivationReason':derivation,
      'colorMap':{'Kedr':'blue','Rust String':'orange'},
      'series':[{'label':'Kedr','valueKind':'durationMs','role':'subject','derived':True,'derivationReason':derivation,
                 'data':[local['core_median_us']/2/1000,fragmented['core_median_us']/1000]},
                {'label':'Rust String','valueKind':'durationMs','role':'base','derived':True,'derivationReason':derivation,
                 'data':[local['string_median_us']/2/1000,fragmented['string_median_us']/1000]}]}
(out/'kedr-chart.json').write_text(json.dumps(spec,ensure_ascii=False,indent=2)+'\n')
lines=['# Initial benchmark results','',
       'Three release-process runs on a shared Linux x86_64 VM; 2,880 timed batches,',
       '24 cases, 120 measured batches per case. 10 warm-ups per case/run. Rust 1.99.0.',
       'Intel Xeon @ 2.50 GHz, 2 visible logical CPUs. This is not an Arch desktop test.', '',
       'Times below are median microseconds per **named workload operation**, computed',
       'from normalized batch means. Sample-p95 is not individual-event p95. No samples',
       'were excluded. All medians independently reconciled via Python csv/statistics.', '',
       '## Editing against the standard library','',
       'Both engines have history disabled. Insert/delete cases count one pair; the',
       'fragmented case counts one sustained insert. Do not compare those rows as identical operations.', '',
       '| Workload | Starting bytes | Kedr, µs | String, µs | String / Kedr |',
       '| --- | ---: | ---: | ---: | ---: |']
for c in comparisons:
    lines.append(f"| {c['workload']} | {c['text_bytes']:,} | {c['core_median_us']:.3f} | {c['string_median_us']:.3f} | {c['string_over_core_ratio']:.2f}× |")
lines += ['', 'Kedr loses the small restored-pair cases and the fragmented-insert case.',
          'Its local-pair advantage at large sizes is a microbenchmark result, not a',
          'comparison against PyCharm or a production editor. Piece-vector fragmentation',
          'must be addressed before treating this engine as ready for interactive workloads.', '',
          '## Other core workloads', '', '| Workload | Median, µs | Sample-p95, µs |', '| --- | ---: | ---: |']
for r in records:
    if r['workload'] not in ['local_insert_delete','scattered_insert_delete','fragmented_insert']:
        lines.append(f"| {r['workload']} | {r['median_us']:.3f} | {r['sample_p95_us']:.3f} |")
lines += ['', '## Next measured priorities','',
          '1. Balanced piece index: improve sustained scattered inserts, not just easy local edits.',
          '2. Line/cursor index: avoid rescanning nearly the whole document on cursor queries.',
          '3. Streaming search and scheduled added-storage compaction.', '',
          'Filesystem cache/order bias and shared VM scheduling limit generalization. No',
          'peak-memory, UI, language-analysis, IDE-comparison or power-loss benchmark was run.', '',
          'Raw CSV files, source fingerprints and environment metadata are in `results/`.',
          'See `docs/BENCHMARKS.md` for exact workloads and setup exclusions.']
(out/'BENCHMARK_RESULTS.md').write_text('\n'.join(lines)+'\n')
