#!/usr/bin/env python3
"""The battle tactics' round robin (docs/TACTICS.md, section 7): both sides the same army, so only
the tactics differ; every tactic against every tactic, attacking and defending.

Armies, each at an early, a middle and a late tech level, of 8 and of 12 units:
  melee    a melee core with 2 archers and 2 mages (Fire II casters: mages without a battle
           spell are a net loss, docs/ENGAGEMENTS.md)
  mounted  a mounted army with 2 archers
  ranged   an archer and mage army with 3 guards
  hero     a core with 2 archers, a hero (the core unit with the Hero promotion) and 2 level-5
           units (Combat I-IV)
Grounds: open, forest, a walled city (the defender's; fortified 5 turns).

    battles_tactics.py specs OUT [--chunks N]     spec files OUT/part*.txt for `ffh battles`
    battles_tactics.py report OUT/*.jsonl         the matrices and the section 7 criteria

    python3 crates/ffh-cli/scripts/battles_tactics.py specs OUT --chunks 4
    for f in OUT/part*.txt; do tools/agents/slot.sh $W/ffh battles $f --runs 400 > ${f%.txt}.jsonl & done; wait
    python3 crates/ffh-cli/scripts/battles_tactics.py report OUT/*.jsonl

A matchup's name is `LEVEL:SIZE:ARMY:GROUND:ATT>DEF`. Its result is the attacker's net share of
HP (`net_hp`: the defender's share of HP lost less the attacker's, percent, per run). The
head-to-head score of X against Y is the mean of X attacking Y and of X defending against Y,
`(net(X>Y) - net(Y>X)) / 2`, with its standard error.
"""
import collections, itertools, json, math, os, sys

TACTICS = ['LINE', 'CHARGE', 'VOLLEY', 'VANGUARD']
LEVELS = ['early', 'mid', 'late']
SIZES = [8, 12]
GROUNDS = {'open': 'open', 'forest': 'forest', 'walls': 'walls fort:5'}

# Per level: the melee core, the mounted unit, the archer, the mage, the guard of the ranged army.
UNITS = {
    'early': dict(core='AXEMAN', mount='HORSEMAN', arch='ARCHER', mage='ADEPT+FIRE1', guard='AXEMAN'),
    'mid': dict(core='CHAMPION', mount='HORSE_ARCHER', arch='LONGBOWMAN', mage='MAGE+FIRE1+FIRE2', guard='CHAMPION'),
    'late': dict(core='PHALANX', mount='KNIGHT', arch='MARKSMAN', mage='ARCHMAGE+FIRE1+FIRE2', guard='PHALANX'),
}
VET = '+COMBAT1+COMBAT2+COMBAT3+COMBAT4:5'


def army(kind, level, n):
    u = UNITS[level]
    if kind == 'melee':
        return f"{n - 4}*{u['core']}, 2*{u['arch']}, 2*{u['mage']}"
    if kind == 'mounted':
        return f"{n - 2}*{u['mount']}, 2*{u['arch']}"
    if kind == 'ranged':
        mages = (n - 3) // 2
        return f"3*{u['guard']}, {n - 3 - mages}*{u['arch']}, {mages}*{u['mage']}"
    if kind == 'hero':
        return f"{n - 5}*{u['core']}, 2*{u['arch']}, 1*{u['core']}+HERO, 2*{u['core']}{VET}"
    raise ValueError(kind)


ARMIES = ['melee', 'mounted', 'ranged', 'hero']


def specs():
    lines = []
    for lvl, n, kind, g in itertools.product(LEVELS, SIZES, ARMIES, GROUNDS):
        spec = army(kind, lvl, n)
        for a, d in itertools.product(TACTICS, repeat=2):
            lines.append(f"{lvl}:{n}:{kind}:{g}:{a}>{d} | {spec} | {spec} | {GROUNDS[g]} tactic:{a} dtactic:{d}")
    return lines


def load(files):
    out = {}
    for f in files:
        for line in open(f):
            line = line.strip()
            if line.startswith('{'):
                j = json.loads(line)
                out[j['name']] = j
    return out


def net(j):
    """The attacker's mean net share of HP and its standard error."""
    r = j['runs']
    s, ss = j['net_hp']
    m = s / r
    var = max(ss / r - m * m, 0.0)
    return m, math.sqrt(var / r) if r > 1 else 0.0


def report(files):
    R = load(files)
    conds = [c for c in itertools.product(LEVELS, SIZES, ARMIES, GROUNDS) if all(f"{c[0]}:{c[1]}:{c[2]}:{c[3]}:{a}>{d}" in R for a in TACTICS for d in TACTICS)]
    cell = lambda c, a, d: R[f"{c[0]}:{c[1]}:{c[2]}:{c[3]}:{a}>{d}"]
    print(f"# {len(R)} matchups, {len(conds)} complete conditions, runs per matchup: {sorted({j['runs'] for j in R.values()})}")
    for j in R.values():
        want = j['name'].split(':')[-1].split('>')
        got = [x.replace('TACTIC_', '') for x in j.get('tactics', ['', ''])]
        if got != want:
            print(f"!! {j['name']}: the battle logs name {got}")
    short = {a: a[:3] for a in TACTICS}

    # Head-to-head scores per condition: (X vs Y) = (net(X>Y) - net(Y>X)) / 2.
    def h2h(c, x, y):
        m1, s1 = net(cell(c, x, y))
        m2, s2 = net(cell(c, y, x))
        return (m1 - m2) / 2, math.sqrt(s1 * s1 + s2 * s2) / 2

    print("\n## Head-to-head score of the row tactic against the column tactic (net share of HP, percent; * beyond 2 SE)")
    for lvl, kind, g in itertools.product(LEVELS, ARMIES, GROUNDS):
        for n in SIZES:
            c = (lvl, n, kind, g)
            if c not in conds:
                continue
            rows = []
            for x in TACTICS:
                vals = []
                for y in TACTICS:
                    if x == y:
                        vals.append('    .')
                        continue
                    m, s = h2h(c, x, y)
                    vals.append(f"{m:+5.1f}{'*' if abs(m) > 2 * s else ' '}")
                rows.append(f"  {short[x]} " + ' '.join(vals))
            print(f"{lvl} {kind} {n} {g}: " + ' '.join(f'{short[y]:>6}' for y in TACTICS))
            print('\n'.join(rows))

    # Compact summaries: the mean head-to-head score over the conditions of each ground, and each
    # tactic against Line per army and ground (the mean over levels and sizes; in brackets the
    # conditions beyond 2 SE either way).
    print("\n## Mean head-to-head score per ground (row against column)")
    for g in GROUNDS:
        cs = [c for c in conds if c[3] == g]
        print(f"{g:8} " + ' '.join(f'{short[y]:>6}' for y in TACTICS))
        for x in TACTICS:
            vals = ['     .' if x == y else f"{sum(h2h(c, x, y)[0] for c in cs) / len(cs):+6.1f}" for y in TACTICS]
            print(f"  {short[x]:6} " + ' '.join(vals))
    print("\n## Against Line per army and ground: mean score (conditions won / lost beyond 2 SE, of 6)")
    print("         " + ' '.join(f'{a + " " + g:>16}' for a in ARMIES for g in GROUNDS))
    for x in TACTICS[1:]:
        vals = []
        for a, g in itertools.product(ARMIES, GROUNDS):
            cs = [c for c in conds if c[2] == a and c[3] == g]
            hs = [h2h(c, x, 'LINE') for c in cs]
            w = sum(1 for m, s in hs if m > 2 * s)
            l = sum(1 for m, s in hs if m < -2 * s)
            vals.append(f"{sum(m for m, _ in hs) / len(hs):+6.1f} ({w}/{l})".rjust(16))
        print(f"  {short[x]:6} " + ' '.join(vals))

    # (b) beats Line clearly somewhere.
    print("\n## (b) Against Line: conditions where the tactic beats Line beyond 2 SE (best three)")
    verdict = {}
    for x in TACTICS[1:]:
        wins = []
        for c in conds:
            m, s = h2h(c, x, 'LINE')
            if m > 2 * s:
                wins.append((m / s if s else 99, m, s, c))
        wins.sort(reverse=True)
        losses = sum(1 for c in conds if h2h(c, x, 'LINE')[0] < -2 * h2h(c, x, 'LINE')[1])
        best = '; '.join(f"{c[0]} {c[2]} {c[1]} {c[3]} {m:+.1f} (SE {s:.1f})" for _, m, s, c in wins[:3])
        print(f"{x}: beats Line in {len(wins)} of {len(conds)}, loses in {losses}. {best}")
        verdict[x] = {'b': len(wins) > 0}

    # (a) dominance: Y at least as good as X in every cell: every condition, against every other
    # tactic O (Y attacking O against X attacking O, Y defending against O against X defending),
    # and Y against X head to head.
    print("\n## (a) Dominance: is the column tactic at least as good as the row tactic in every cell? (cells where it is / cells)")
    print("        " + ' '.join(f'{short[y]:>9}' for y in TACTICS))
    for x in TACTICS:
        dom = []
        row = []
        for y in TACTICS:
            if y == x:
                row.append(f"{'.':>9}")
                continue
            ok = total = 0
            for c in conds:
                for o in TACTICS:
                    if o in (x, y):
                        continue
                    ok += net(cell(c, y, o))[0] >= net(cell(c, x, o))[0]
                    ok += net(cell(c, o, y))[0] <= net(cell(c, o, x))[0]
                    total += 2
                ok += h2h(c, y, x)[0] >= 0
                total += 1
            if ok == total:
                dom.append(y)
            row.append(f"{ok:>4}/{total:<4}")
        print(f"  {short[x]}   " + ' '.join(row))
        verdict.setdefault(x, {})['a'] = not dom
        if dom:
            print(f"  {x} is dominated by {dom}")

    # (c) Vanguard: veterans' deaths in the battles its side wins, against Line's.
    print("\n## (c) Veterans fallen per veteran committed, in the battles the side wins (hero army)")
    agg = collections.defaultdict(lambda: [0, 0, 0])  # tactic -> [won, vets, fell]
    agg_c = collections.defaultdict(lambda: [0, 0, 0])
    for c in conds:
        if c[2] != 'hero':
            continue
        for a, d in itertools.product(TACTICS, repeat=2):
            j = cell(c, a, d)
            for side, arr in ((0, a), (1, d)):
                for k, key in enumerate(('won', 'vet_won', 'vet_fell_won')):
                    agg[arr][k] += j[key][side]
                    agg_c[(c, arr)][k] += j[key][side]
    for x in TACTICS:
        w, v, f = agg[x]
        print(f"{x}: won {w}, veterans committed {v}, fell {f} ({100 * f / v if v else 0:.1f}%)")
    vl, vv = agg['LINE'], agg['VANGUARD']
    rl = vl[2] / vl[1] if vl[1] else 0
    rv = vv[2] / vv[1] if vv[1] else 0
    print(f"Vanguard / Line veteran deaths per veteran: {rv / rl if rl else float('nan'):.2f}")
    # Where Vanguard gives Line's formation the two tactics' battles are the same (the score is
    # exactly 0 and every cell equal): (c) is asked only where they differ.
    def same(c):
        return all(cell(c, 'VANGUARD', o)['net_hp'] == cell(c, 'LINE', o)['net_hp'] and cell(c, o, 'VANGUARD')['net_hp'] == cell(c, o, 'LINE')['net_hp'] for o in TACTICS)
    print("per condition (Line %, Vanguard %, Vanguard / Line; the score against Line):")
    for c in conds:
        if c[2] != 'hero':
            continue
        l, v = agg_c[(c, 'LINE')], agg_c[(c, 'VANGUARD')]
        pl = 100 * l[2] / l[1] if l[1] else 0
        pv = 100 * v[2] / v[1] if v[1] else 0
        m, se = h2h(c, 'VANGUARD', 'LINE')
        tag = 'as Line' if same(c) else f"{pv / pl if pl else float('nan'):.2f}"
        print(f"  {c[0]} {c[1]} {c[3]}: {pl:.1f} {pv:.1f} {tag}; {m:+.1f} (SE {se:.1f})")
    print("per army size and ground, over the levels where Vanguard differs from Line (Line %, Vanguard %, ratio, (c)):")
    c_every = True
    for n, g in itertools.product(SIZES, GROUNDS):
        cs = [c for c in conds if c[2] == 'hero' and c[1] == n and c[3] == g and not same(c)]
        if not cs:
            print(f"  {n} {g}: as Line at every level")
            continue
        l = [sum(agg_c[(c, 'LINE')][k] for c in cs) for k in range(3)]
        v = [sum(agg_c[(c, 'VANGUARD')][k] for c in cs) for k in range(3)]
        pl = l[2] / l[1] if l[1] else 0
        pv = v[2] / v[1] if v[1] else 0
        ok = pv <= pl / 2  # at most half of Line's (0 against 0 holds)
        c_every = c_every and ok
        sc = sum(h2h(c, 'VANGUARD', 'LINE')[0] for c in cs) / len(cs)
        print(f"  {n} {g} ({len(cs)} levels): {100 * pl:.1f} {100 * pv:.1f} {pv / pl if pl else float('nan'):.2f} {'holds' if ok else 'FAILS'}; score against Line {sc:+.1f}")
    print(f"(c) in every army size and ground where Vanguard differs from Line: {c_every}")
    verdict['VANGUARD']['c'] = rl > 0 and rv <= rl / 2

    print("\n## Verdict (section 7)")
    for x in TACTICS[1:]:
        v = verdict[x]
        need = ['a', 'c'] if x == 'VANGUARD' else ['a', 'b']
        ok = all(v.get(k) for k in need)
        print(f"{x}: {'ships' if ok else 'FAILS'} ({', '.join(f'{k}={v.get(k)}' for k in sorted(v))})")


def main():
    args = sys.argv[1:]
    if args and args[0] == 'specs':
        out = args[1]
        chunks = int(args[args.index('--chunks') + 1]) if '--chunks' in args else 1
        os.makedirs(out, exist_ok=True)
        lines = specs()
        for k in range(chunks):
            with open(os.path.join(out, f'part{k}.txt'), 'w') as f:
                f.write('\n'.join(lines[k::chunks]) + '\n')
        print(f"{len(lines)} matchups in {chunks} files in {out}")
    elif args and args[0] == 'report':
        report(args[1:])
    else:
        print(__doc__)
        sys.exit(2)


if __name__ == '__main__':
    main()
