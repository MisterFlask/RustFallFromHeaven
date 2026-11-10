#!/usr/bin/env python3
"""Calibration of the AI's estimators (docs/CALIBRATION.md): the prediction ledger's resolved
predictions, per kind, as a forecaster would be scored.

  calibration.py --bin DIR --seeds 1-24 --turns 300 --out DIR [--civs 6] [--size WORLDSIZE_X]
                 [--jobs N] [--option GAMEOPTION_X]       play the games, then report
  calibration.py --load DIR|FILE.jsonl [..]                report on games already played
  options: [--kind K[,K..]] [--by civ] [--slice KEY] [--threshold 50] [--json]

Playing: each seed runs `ffh stats` with FFH_TELEMETRY_FILE=<out>/seed<N>.jsonl (the stats JSON
goes to <out>/seed<N>.json); every game holds one machine-wide CPU slot (tools/agents,
docs/AGENTS.md). Telemetry files are not served by the run cache: each game is played.

Report, per kind (and per group with --by / --slice): N resolved, void, mean predicted pct,
observed rate, Brier score (and the Brier score of always predicting the observed rate, with the
skill 1 - Brier/reference), the reliability table (predicted pct in tenths against the observed
rate), and the 2x2 table at --threshold (predicted yes = pct >= threshold) with precision and
recall. --slice KEY splits by a field of the prediction (`acted`, `civ`) or of its ctx
(`src`, `vs_civ`, `barbarian`, ...); --by civ is --slice civ.
"""
import argparse
import glob
import json
import os
import subprocess
import sys
from collections import defaultdict
from concurrent.futures import ThreadPoolExecutor

try:
    sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', '..', '..', 'tools', 'agents'))
    import ffh_agents
except ImportError:
    ffh_agents = None
finally:
    del sys.path[0]


def parse_seeds(s):
    out = []
    for part in s.split(','):
        if '-' in part:
            a, b = part.split('-')
            out.extend(range(int(a), int(b) + 1))
        else:
            out.append(int(part))
    return out


def ffh_bin(path):
    return os.path.join(path, 'ffh') if os.path.isdir(path) else path


def play(args):
    os.makedirs(args.out, exist_ok=True)
    binary = ffh_bin(args.bin)

    def one(seed):
        jsonl = os.path.join(args.out, 'seed%d.jsonl' % seed)
        cmd = [binary, 'stats', '--seed', str(seed), '--civs', str(args.civs), '--size', args.size, '--turns', str(args.turns)]
        for opt in args.option:
            cmd += ['--option', opt]
        env = dict(os.environ, FFH_TELEMETRY_FILE=jsonl)
        slots = ffh_agents.cpu_slots(1, ' '.join(cmd)) if ffh_agents else None
        if slots:
            with slots:
                r = subprocess.run(cmd, env=env, capture_output=True, text=True)
        else:
            r = subprocess.run(cmd, env=env, capture_output=True, text=True)
        if r.returncode != 0:
            print('seed %d failed: %s' % (seed, (r.stderr or r.stdout)[-800:]), file=sys.stderr)
            return seed, False
        with open(os.path.join(args.out, 'seed%d.json' % seed), 'w') as f:
            f.write(r.stdout)
        print('seed %d done' % seed, file=sys.stderr)
        return seed, True

    with ThreadPoolExecutor(max_workers=args.jobs) as ex:
        res = list(ex.map(one, parse_seeds(args.seeds)))
    return [os.path.join(args.out, 'seed%d.jsonl' % s) for s, ok in res if ok]


def load(paths):
    files = []
    for p in paths:
        files.extend(sorted(glob.glob(os.path.join(p, '*.jsonl'))) if os.path.isdir(p) else [p])
    preds = []
    for i, path in enumerate(files):
        with open(path) as f:
            for line in f:
                if '"e":"prediction"' not in line:
                    continue
                e = json.loads(line)
                e['game'] = i
                preds.append(e)
    return files, preds


def field(e, key):
    if key in e and key != 'ctx':
        v = e[key]
    else:
        v = (e.get('ctx') or {}).get(key, '-')
    if isinstance(v, bool):
        return 'true' if v else 'false'
    return str(v)


def summarize(preds, threshold):
    res = [e for e in preds if e.get('outcome') is not None]
    void = len(preds) - len(res)
    n = len(res)
    out = {'n': n, 'void': void}
    if n == 0:
        return out
    hits = sum(1 for e in res if e['outcome'])
    rate = hits / n
    mean = sum(e['pct'] for e in res) / n / 100
    brier = sum((e['pct'] / 100 - (1 if e['outcome'] else 0)) ** 2 for e in res) / n
    ref = rate * (1 - rate)
    bins = []
    for b in range(10):
        lo, hi = b * 10, (b + 1) * 10 if b < 9 else 101
        inb = [e for e in res if lo <= e['pct'] < hi]
        if inb:
            bins.append({'bin': '%d-%d' % (lo, min(hi, 100) if b == 9 else hi - 1), 'n': len(inb),
                         'mean_pct': sum(e['pct'] for e in inb) / len(inb),
                         'observed': 100 * sum(1 for e in inb if e['outcome']) / len(inb)})
    tp = sum(1 for e in res if e['pct'] >= threshold and e['outcome'])
    fp = sum(1 for e in res if e['pct'] >= threshold and not e['outcome'])
    fn = sum(1 for e in res if e['pct'] < threshold and e['outcome'])
    tn = n - tp - fp - fn
    out.update({
        'hits': hits, 'observed': 100 * rate, 'mean_pct': 100 * mean, 'brier': brier, 'brier_ref': ref,
        'skill': (1 - brier / ref) if ref > 0 else None, 'bins': bins,
        'table': {'tp': tp, 'fp': fp, 'fn': fn, 'tn': tn},
        'precision': tp / (tp + fp) if tp + fp else None, 'recall': tp / (tp + fn) if tp + fn else None,
    })
    return out


def pct(x):
    return '-' if x is None else '%.0f%%' % (100 * x)


def print_summary(title, s, threshold):
    if s['n'] == 0:
        print('%s: N 0, void %d' % (title, s['void']))
        return
    skill = '-' if s['skill'] is None else '%+.2f' % s['skill']
    print('%s: N %d, void %d, predicted %.1f%%, observed %.1f%%, Brier %.3f (reference %.3f, skill %s)'
          % (title, s['n'], s['void'], s['mean_pct'], s['observed'], s['brier'], s['brier_ref'], skill))
    print('    %-8s %6s %10s %9s' % ('pct', 'n', 'predicted', 'observed'))
    for b in s['bins']:
        print('    %-8s %6d %9.1f%% %8.1f%%' % (b['bin'], b['n'], b['mean_pct'], b['observed']))
    t = s['table']
    print('    at %d%%: yes&happened %d, yes&not %d, no&happened %d, no&not %d; precision %s, recall %s'
          % (threshold, t['tp'], t['fp'], t['fn'], t['tn'], pct(s['precision']), pct(s['recall'])))


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument('--bin', help='directory holding ffh (or the binary)')
    ap.add_argument('--seeds', default='1-24')
    ap.add_argument('--turns', type=int, default=300)
    ap.add_argument('--civs', type=int, default=6)
    ap.add_argument('--size', default='WORLDSIZE_STANDARD')
    ap.add_argument('--option', action='append', default=[])
    ap.add_argument('--jobs', type=int, default=os.cpu_count() or 4)
    ap.add_argument('--out', help='directory for the games played')
    ap.add_argument('--load', nargs='+', help='directories or .jsonl files already played')
    ap.add_argument('--kind', help='only these kinds (comma-separated)')
    ap.add_argument('--by', choices=['civ'], help='split by civilization')
    ap.add_argument('--slice', help='split by a prediction or ctx field')
    ap.add_argument('--threshold', type=int, default=50)
    ap.add_argument('--json', action='store_true')
    args = ap.parse_args()

    if args.load:
        paths = args.load
    elif args.bin and args.out:
        paths = play(args)
    else:
        ap.error('give --bin and --out to play games, or --load')
    files, preds = load(paths)
    if args.kind:
        kinds = set(args.kind.split(','))
        preds = [e for e in preds if e['kind'] in kinds]
    split = 'civ' if args.by == 'civ' else args.slice
    by_kind = defaultdict(list)
    for e in preds:
        by_kind[e['kind']].append(e)
    report = {'games': len(files), 'kinds': {}}
    for kind in sorted(by_kind):
        es = by_kind[kind]
        entry = {'all': summarize(es, args.threshold)}
        if split:
            groups = defaultdict(list)
            for e in es:
                groups[field(e, split)].append(e)
            entry['slices'] = {k: summarize(v, args.threshold) for k, v in sorted(groups.items())}
        report['kinds'][kind] = entry
    if args.json:
        json.dump(report, sys.stdout, indent=1)
        print()
        return
    print('%d games, %d predictions' % (len(files), len(preds)))
    for kind, entry in report['kinds'].items():
        print()
        print_summary(kind, entry['all'], args.threshold)
        for k, s in entry.get('slices', {}).items():
            print_summary('  %s %s=%s' % (kind, split, k), s, args.threshold)


if __name__ == '__main__':
    main()
