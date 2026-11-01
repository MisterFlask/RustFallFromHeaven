#!/usr/bin/env python3
"""Where a game's CPU time goes, from `perf` call stacks (std only; see docs/PERF.md).

Record and convert (the release profile keeps line tables, `debug = 1`, so DWARF unwinding works;
only the cpu-clock event exists in this VM):

  tools/agents/slot.sh -n 1 env FFH_DATA_DIR=$B/data perf record -F 250 -e cpu-clock \\
      --call-graph dwarf,32768 -o x.data -- $B/bin/ffh bench --seed 1 --civs 8 --size WORLDSIZE_STANDARD --turns 400
  perf script -i x.data -F comm,period,ip,sym --no-demangle > x.txt

Then:

  tools/perf-stacks.py x.txt incl [N]            inclusive share by function (each sample counted
                                                 once per function on its stack)
  tools/perf-stacks.py x.txt self [N]            self share (the leaf frame; inlined frames count)
  tools/perf-stacks.py x.txt callers FN [CRATE]  samples through FN, by the nearest caller frame of
                                                 CRATE (ai or core; default ai) above it
  tools/perf-stacks.py x.txt parent FN           ... by the immediate caller
  tools/perf-stacks.py x.txt callees FN          inclusive share of the functions below FN

Names are demangled with c++filt (binutils reads Rust v0 symbols) and shortened: crate hashes
dropped, `<ffh_core::game::Game>::` dropped (Game methods appear bare), ffh_core:: -> core:,
ffh_ai:: -> ai:. FN is a name as printed by incl/self. Inlined frames appear as frames of their
own, so the inclusive share of small inlined helpers is approximate.
"""
import collections
import re
import shutil
import subprocess
import sys


def demangle(syms):
    """Demangled, shortened names for a list of raw symbols (one c++filt call)."""
    out = syms
    if shutil.which('c++filt'):
        r = subprocess.run(['c++filt'], input='\n'.join(syms), capture_output=True, text=True)
        if r.returncode == 0:
            out = r.stdout.split('\n')[:len(syms)]
    short = []
    for s in out:
        s = re.sub(r'\[[0-9a-f]{16}\]', '', s).replace(' (inlined)', '')
        s = s.replace('<ffh_core::game::Game>::', '')
        s = re.sub(r'::<[^>]*>$', '', s)
        s = re.sub(r'^<(.*) as [^>]*>::', r'\1::', s)
        short.append(s.replace('ffh_core::', 'core:').replace('ffh_ai::', 'ai:').replace('ffh_data::', 'data:'))
    return short


def samples(path):
    """Stacks (leaf first) of the samples in a `perf script` text dump."""
    stacks, cur = [], []
    with (sys.stdin if path == '-' else open(path, errors='replace')) as f:
        for line in f:
            if not line.strip():
                if cur:
                    stacks.append(cur)
                cur = []
            elif line[0] in ' \t':
                p = line.split(None, 1)
                cur.append(p[1].strip().replace(' (inlined)', '') if len(p) > 1 else '?')
    if cur:
        stacks.append(cur)
    raw = sorted({s for st in stacks for s in st})
    names = dict(zip(raw, demangle(raw)))
    return [[names[s] for s in st] for st in stacks]


def matches(name, fn):
    return name == fn or name.endswith(':' + fn) or name.endswith('::' + fn)


def main():
    if len(sys.argv) < 3:
        print(__doc__)
        return 2
    stacks = samples(sys.argv[1])
    mode, rest = sys.argv[2], sys.argv[3:]
    total = max(len(stacks), 1)
    c = collections.Counter()
    if mode in ('incl', 'self'):
        n = int(rest[0]) if rest else 60
        for st in stacks:
            if mode == 'self':
                c[st[0]] += 1
            else:
                c.update(set(st))
        for name, v in c.most_common(n):
            print('%6.2f%% %s' % (100 * v / total, name))
        return 0
    if mode not in ('callers', 'parent', 'callees') or not rest:
        print(__doc__)
        return 2
    fn = rest[0]
    crate = rest[1] if mode == 'callers' and len(rest) > 1 else 'ai'
    hit = 0
    for st in stacks:
        idx = [i for i, name in enumerate(st) if matches(name, fn)]
        if not idx:
            continue
        hit += 1
        i = max(idx)  # the outermost occurrence
        if mode == 'callees':
            c.update(set(st[:i]))
        elif mode == 'parent':
            c[next((x for x in st[i + 1:] if not matches(x, fn)), '<top>')] += 1
        else:
            c[next((x for x in st[i + 1:] if (x.startswith(crate + ':') or x.startswith('<' + crate + ':')) and not matches(x, fn)), '<none>')] += 1
    print('%s: %.2f%% of samples' % (fn, 100 * hit / total))
    for name, v in c.most_common(40):
        print('%6.2f%%  (%5.1f%% of %s)  %s' % (100 * v / total, 100 * v / max(hit, 1), fn, name))
    return 0


if __name__ == '__main__':
    sys.exit(main())
