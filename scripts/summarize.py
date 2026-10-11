"""Compare matched release batches and RSS; independently check all medians."""
import argparse,csv,json,math,statistics
from pathlib import Path
import pandas as pd

p=argparse.ArgumentParser()
p.add_argument('--before',type=Path,default=Path('results/before'))
p.add_argument('--after',type=Path,default=Path('results/after'))
p.add_argument('--memory',type=Path,default=Path('results/memory.csv'))
p.add_argument('--out',type=Path,default=Path('results/comparison'))
a=p.parse_args();a.out.mkdir(parents=True,exist_ok=True)
records=[];counts={}
keys=['workload','engine','text_bytes']
for version,root in [('before',a.before),('after',a.after)]:
    frames=[];check={}
    for run in range(1,4):
        path=root/f'run-{run}.csv';d=pd.read_csv(path)
        assert not d.isna().any().any() and not d.duplicated(keys+['sample']).any()
        assert (d.elapsed_ns>0).all() and (d.batch_operations>0).all()
        d['run']=run;frames.append(d)
        with path.open() as stream:
            for row in csv.DictReader(stream):
                key=(row['workload'],row['engine'],int(row['text_bytes']))
                check.setdefault(key,[]).append(int(row['elapsed_ns'])/int(row['batch_operations'])/1000)
    d=pd.concat(frames,ignore_index=True);d['us']=d.elapsed_ns/d.batch_operations/1000;counts[version]=len(d)
    for key,group in d.groupby(keys):
        assert len(group)==120
        median=float(group.us.median())
        assert math.isclose(median,statistics.median(check[key]),rel_tol=1e-12)
        run_medians=group.groupby('run').us.median()
        records.append(dict(zip(keys,key))|dict(version=version,samples=len(group),median_us=median,
            sample_p95_us=float(group.us.quantile(.95)),run_median_min_us=float(run_medians.min()),run_median_max_us=float(run_medians.max())))
s=pd.DataFrame(records);before=s[s.version=='before'].drop(columns='version');after=s[s.version=='after'].drop(columns='version')
merged=before.merge(after,on=keys,suffixes=('_before','_after'),validate='one_to_one')
assert len(merged)==29 and counts=={'before':3480,'after':3480}
merged['before_over_after']=merged.median_us_before/merged.median_us_after
merged['after_over_before']=merged.median_us_after/merged.median_us_before
merged.to_csv(a.out/'timing.csv',index=False)
m=pd.read_csv(a.memory);assert len(m)==24 and not m.isna().any().any() and not m.duplicated(['version','workload','run']).any()
assert (m.peak_rss_kib>0).all()
mrecords=[]
for (version,case),g in m.groupby(['version','workload']):
    assert len(g)==3
    value=float(g.peak_rss_kib.median());assert value==statistics.median(g.peak_rss_kib.tolist())
    mrecords.append(dict(version=version,workload=case,median_peak_rss_kib=value,min_peak_rss_kib=int(g.peak_rss_kib.min()),max_peak_rss_kib=int(g.peak_rss_kib.max())))
ms=pd.DataFrame(mrecords);ms.to_csv(a.out/'memory.csv',index=False)
lookup={(r['workload'],r['engine'],int(r['text_bytes'])):r for r in merged.to_dict('records')}
selected=[lookup[(name,'piece_table',size)] for name,size in [('local_insert_delete',1048576),('fragmented_insert',1048576),('cursor_lookup_near_end',1048575),('undo_redo',1048576),('construct_unicode',1048572)]]
evidence={'timing_rows':counts,'cases_per_version':29,'rss_rows':24,
    'grain':'median of batch-normalized mean microseconds per named operation, 120 batches/case/version',
    'validation':'all timing medians independently reproduced via csv + statistics; RSS medians independently checked',
    'timings':merged.to_dict('records'),'memory':mrecords,'selected':selected,'comparison_plan':[]}
for r in selected: print(f"{r['workload']:25} {r['median_us_before']:11.4f} -> {r['median_us_after']:11.4f} us; before/after {r['before_over_after']:.2f}x")
print(ms.to_string(index=False))
(a.out/'evidence.json').write_text(json.dumps(evidence,indent=2)+'\n')
# All numbers in prose/figure/report derive from the same reconciled evidence.
labels=['Локальная пара','Разнесённые вставки','Координаты курсора','Undo/redo','Создание Unicode']
assert selected[1]['before_over_after']>1 and selected[2]['before_over_after']>1 and selected[3]['before_over_after']>1
assert selected[0]['after_over_before']>1 and selected[4]['after_over_before']>1
for row in selected:
    evidence['comparison_plan'].append({'grain':row['workload'],'population':'120 timed batches per version, matched workload','unit':'after/before median latency ratio',
      'finding':'improvement' if row['after_over_before']<1 else 'regression','derivation':'median_after_us / median_before_us',
      'disposition':'chart','reason':'Includes large wins and meaningful losses, not just the favorable benchmarks'})
evidence['comparison_plan'].append({'grain':'4 independent Linux RSS fixtures','population':'3 child processes per version/case','unit':'whole-process peak RSS KiB',
  'finding':'shared clones reduce memory; index/history metadata increases other footprints','derivation':'median of child VmHWM observations',
  'disposition':'table','reason':'Different units from timing; keep precise memory costs without conflating RSS with editor event latency'})
(a.out/'evidence.json').write_text(json.dumps(evidence,indent=2)+'\n')
formula='median_after(elapsed_ns / batch_operations) / median_before(elapsed_ns / batch_operations), matched workload and buffer size'
spec={'version':1,'form':'categorical-bars','mode':'grouped','labels':labels,
      'title':'Новое ядро ускоряет большие задачи, но не каждую операцию',
      'aside':'Время относительно прежней версии; меньше — быстрее',
      'axis':{'zero':True},'colorMap':{'До':'orange','После':'blue'},'derived':True,'derivationReason':formula,
      'series':[{'label':'До','valueKind':'ratio','role':'base','data':[1]*len(selected)},
                {'label':'После','valueKind':'ratio','role':'subject','derived':True,'derivationReason':formula,
                 'data':[r['after_over_before'] for r in selected]}]}
(a.out/'chart.json').write_text(json.dumps(spec,ensure_ascii=False,indent=2)+'\n')
lines=['# Indexed core performance comparison','',
       'Version 0.1 vector engine versus version 0.2 persistent AVL engine, with the',
       'same expanded benchmark harness on the same shared Linux x86_64 VM.',
       'Rust 1.99.0, Intel Xeon @ 2.50 GHz, two visible logical CPUs. Three processes',
       'per engine: 29 cases, 120 measured batches/case, 6,960 timing rows total.',
       'Ten warm-ups per case/run. No samples were discarded. All timing medians were',
       'independently checked via csv/statistics. Values are normalized batch means,',
       'not individual-event p95 or real IDE latency. See source fingerprints in results.', '',
       '## Important matched workloads (approximately 1 MiB)', '',
       '| Workload | Before, µs | After, µs | Before / after |', '| --- | ---: | ---: | ---: |']
for r in selected:
    lines.append(f"| {r['workload']} | {r['median_us_before']:.4f} | {r['median_us_after']:.4f} | {r['before_over_after']:.2f}× |")
lines+=['',
        'The cursor/fragmentation/undo wins do not erase the local-edit and Unicode',
        'construction regressions. Local pairs count insert plus delete; fragmented',
        'operations count one insert. Unicode construction includes index building.',
        'Future work should optimize tiny-edit paths and Unicode index construction',
        'without removing snapshot/version safety or hiding retention costs.', '',
        '## Whole-process memory', '',
        'Independent Linux VmHWM measurements: 4 MiB line-rich text. Median of three',
        'fresh processes/case/version, including setup and transient allocations.',
        'This is peak process RSS, not current/exclusive heap, not MemoryStats.', '',
        '| Fixture | Before peak RSS, MiB | After peak RSS, MiB |', '| --- | ---: | ---: |']
for case in ['lines','fragmented','history','clones']:
    b=next(r for r in mrecords if r['version']=='before' and r['workload']==case)
    c=next(r for r in mrecords if r['version']=='after' and r['workload']==case)
    lines.append(f"| {case} | {b['median_peak_rss_kib']/1024:.3f} | {c['median_peak_rss_kib']/1024:.3f} |")
lines+=['', 'Index/tree/history metadata increases the non-clone fixtures. Structural',
        'sharing substantially reduces the 32-clone fixture; this does not promise',
        'bounded memory when arbitrary long-lived snapshots or history are retained.', '',
        '## Complete timing evidence', '',
        '| Workload | Engine | Starting bytes | Before, µs | After, µs | Before / after |',
        '| --- | --- | ---: | ---: | ---: | ---: |']
for r in merged.to_dict('records'):
    lines.append(f"| {r['workload']} | {r['engine']} | {r['text_bytes']:,} | {r['median_us_before']:.4f} | {r['median_us_after']:.4f} | {r['before_over_after']:.2f}× |")
lines+=['', 'std_string rows are control workloads, not another IDE. Fixed engine/case',
        'order, warm caches and shared-VM load can bias small differences; run-median',
        'ranges and sample-p95 values are retained in comparison/timing.csv. These',
        'are descriptive measurements, not confidence intervals. No causal claim is',
        'made for unchanged project/OS code or small control fluctuations.', '',
        '## Integration gains', '',
        'Immutable snapshots, process-local document/revision tokens, stale/wrong-document',
        'batch rejection, latest change envelopes, lazy search, allocation diagnostics',
        'and an integration example. No UI, parsers, language servers or debugger were',
        'added; these are future clients of the storage API.', '',
        'Raw timing files: results/before and results/after. Raw RSS: results/memory.csv.',
        'Source/environment fingerprints and reconciled evidence are checked in.',
        'The original historical baseline remains in BENCHMARK_RESULTS.md.']
(a.out/'PERFORMANCE.md').write_text('\n'.join(lines)+'\n')
