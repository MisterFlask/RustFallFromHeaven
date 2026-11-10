#!/usr/bin/env python3
"""ai-branch-rolls.py: the `--rolls N` half of tools/ai-branch.sh (std only).

  ai-branch-rolls.py --save FILE --turns N --rolls R --out DIR [--trace P]
                     --side NAME FFH ENV [--side NAME FFH ENV] -- WATCH..

Resumes the save R times per side (`ffh resume --reseed K --summary`, K = 1..R), each run under
tools/agents/slot.sh (one machine-wide CPU slot), as many at once as the slots allow. FFH is an
`ffh` binary (a build output's wrapper), ENV one `VAR=VALUE` set for that side or `-`. Each
side's summary lines go to DIR/NAME.rolls.jsonl (one per K, in K order). Then prints, for every
watched value, a table over the rolls: numbers as mean [min..max] per side, the difference of
the means and a paired t statistic (rolls paired by K: both sides share each K's reseed);
categorical values (strings, booleans) as the count of each value per side.
"""
import concurrent.futures
import json
import math
import os
import subprocess
import sys


def parse(argv):
    a = {'sides': [], 'watches': [], 'trace': None}
    i = 0
    while i < len(argv):
        o = argv[i]
        if o == '--':
            a['watches'] = argv[i + 1:]
            break
        if o == '--side':
            a['sides'].append(tuple(argv[i + 1:i + 4]))
            i += 4
            continue
        a[o.lstrip('-')] = argv[i + 1]
        i += 2
    for k in ('save', 'turns', 'rolls', 'out'):
        if k not in a:
            sys.exit(f'ai-branch-rolls.py: --{k} is required')
    if not a['sides']:
        sys.exit('ai-branch-rolls.py: at least one --side')
    return a


def run_one(a, side, k):
    name, ffh, envspec = side
    env = dict(os.environ)
    if envspec != '-':
        var, _, val = envspec.partition('=')
        env[var] = val
    slot = os.path.join(os.path.dirname(os.path.abspath(__file__)), 'agents', 'slot.sh')
    cmd = [slot, ffh, 'resume', a['save'], '--turns', a['turns'], '--reseed', str(k), '--summary']
    for w in a['watches']:
        cmd += ['--watch', w]
    if a['trace']:
        env['FFH_AI_TRACE'] = a['trace']
        with open(os.path.join(a['out'], f'{name}.r{k}.trace'), 'w') as err:
            p = subprocess.run(cmd, env=env, stdout=subprocess.PIPE, stderr=err, text=True, stdin=subprocess.DEVNULL)
        tail = ''
    else:
        p = subprocess.run(cmd, env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, stdin=subprocess.DEVNULL)
        tail = p.stderr.strip().splitlines()[-1:] if p.stderr.strip() else []
        tail = tail[0] if tail else ''
    line = p.stdout.strip().splitlines()[-1] if p.stdout.strip() else ''
    if p.returncode != 0 or not line.startswith('{'):
        return name, k, None, f'exit {p.returncode}: {tail}'
    return name, k, line, None


def is_cat(v):
    return isinstance(v, (str, bool)) or v is None


def fmt(x):
    if x is None:
        return '-'
    if abs(x - round(x)) < 1e-9:
        return str(int(round(x)))
    return f'{x:.2f}' if abs(x) < 100 else f'{x:.1f}'


def paired_t(xs, ys):
    d = [x - y for x, y in zip(xs, ys)]
    n = len(d)
    if n < 2:
        return None
    m = sum(d) / n
    var = sum((v - m) ** 2 for v in d) / (n - 1)
    if var == 0:
        return 0.0 if m == 0 else math.copysign(math.inf, m)
    return m / math.sqrt(var / n)


def main():
    a = parse(sys.argv[1:])
    rolls = int(a['rolls'])
    os.makedirs(a['out'], exist_ok=True)
    jobs = [(s, k) for k in range(1, rolls + 1) for s in a['sides']]
    results = {s[0]: {} for s in a['sides']}
    failures = []
    with concurrent.futures.ThreadPoolExecutor(max_workers=max(1, min(len(jobs), os.cpu_count() or 4))) as ex:
        for name, k, line, err in ex.map(lambda j: run_one(a, *j), jobs):
            if err:
                failures.append(f'{name} roll {k}: {err}')
            else:
                results[name][k] = json.loads(line)
    names = [s[0] for s in a['sides']]
    for name in names:
        with open(os.path.join(a['out'], f'{name}.rolls.jsonl'), 'w') as f:
            for k in sorted(results[name]):
                f.write(json.dumps(results[name][k]) + '\n')
    if failures:
        print('failed rolls:', file=sys.stderr)
        for f in failures:
            print('  ' + f, file=sys.stderr)
        if not all(results[n] for n in names):
            sys.exit(1)

    # Keys in first-appearance order; a key missing from a roll (an area player gone) is 0 / '-'.
    keys = []
    for name in names:
        for k in sorted(results[name]):
            for key in results[name][k]['values']:
                if key not in keys:
                    keys.append(key)
    ks = sorted(set.intersection(*(set(results[n]) for n in names)))
    first = next(results[n][k] for n in names for k in sorted(results[n]))
    turns = sorted({r['turn'] for n in names for r in results[n].values()})
    print(f"{len(ks)} rolls (--reseed {ks[0] if ks else '-'}..{ks[-1] if ks else '-'}) per side, "
          f"fork turn {first['fork_turn']} to turn {'/'.join(map(str, turns))}; sides: {', '.join(names)}")
    for n in names:
        cs = {r['checksum'] for r in results[n].values()}
        over = sum(1 for r in results[n].values() if not r['game_on'])
        print(f"  {n}: {len(cs)} distinct end states of {len(results[n])}" + (f", game over in {over}" if over else ''))

    def col(n, key, k):
        return results[n][k]['values'].get(key)

    width = max([len(k) for k in keys] + [10])
    two = len(names) == 2
    head = f"{'value':<{width}}  " + '  '.join(f'{n + " mean [min..max]":<24}' for n in names)
    if two:
        head += f"  {'diff':>7}  {'t':>6}"
    print()
    print(head)
    for key in keys:
        vals = {n: [col(n, key, k) for k in ks] for n in names}
        allv = [v for n in names for v in vals[n]]
        if any(is_cat(v) for v in allv if v is not None) or all(v is None for v in allv):
            parts = []
            for n in names:
                c = {}
                for v in vals[n]:
                    s = '-' if v is None else (str(v).lower() if isinstance(v, bool) else str(v))
                    c[s] = c.get(s, 0) + 1
                parts.append(', '.join(f'{s} x{c[s]}' for s in sorted(c, key=lambda s: (-c[s], s))))
            mark = '*' if two and sorted(map(str, vals[names[0]])) != sorted(map(str, vals[names[1]])) else ' '
            print(f'{key:<{width}}  ' + '  '.join(f'{p:<24}' for p in parts) + (f'  {mark}' if two else ''))
            continue
        nums = {n: [0 if v is None else v for v in vals[n]] for n in names}
        cells = []
        for n in names:
            xs = nums[n]
            m = sum(xs) / len(xs) if xs else None
            cells.append(f'{fmt(m)} [{fmt(min(xs)) if xs else "-"}..{fmt(max(xs)) if xs else "-"}]')
        line = f'{key:<{width}}  ' + '  '.join(f'{c:<24}' for c in cells)
        if two and ks:
            xa, xb = nums[names[0]], nums[names[1]]
            diff = sum(xa) / len(xa) - sum(xb) / len(xb)
            t = paired_t(xa, xb)
            ts = '-' if t is None else ('inf' if t == math.inf else '-inf' if t == -math.inf else f'{t:.2f}')
            line += f'  {fmt(diff):>7}  {ts:>6}'
        print(line)
    print()
    print(f"per-roll lines: {', '.join(os.path.join(a['out'], n + '.rolls.jsonl') for n in names)}")
    if failures:
        sys.exit(1)


if __name__ == '__main__':
    main()
