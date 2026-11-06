#!/usr/bin/env python3
"""The squad meta of a round robin from `ffh battles` (names `LEVEL:GROUND:A>B`; the battery of
battles_squads.py; docs/ENGAGEMENTS.md, "Balance lab"). The result of squad i against j on a
ground is its net share of HP (the enemy's HP% lost less its own; `--metric cw`: hammers of HP
destroyed less lost) attacking j and defending against j.

    battles_meta.py meta LEVEL FILES.. [--metric hp|cw] [--ctx open,forest,..] [--only F.json]
        the payoff matrix (the mean over grounds and both roles), each squad against the field
        and against the equilibrium (the symmetric zero-sum game's, by fictitious play: a squad
        at 100% beats or ties every other one), its worst matchup; and dominance: i dominates k
        when against every opponent, on every ground, in both roles, i does at least as well as k
        less 2 standard errors of the difference, and better on average
    battles_meta.py swaps FILES..
        unit against unit: squads that differ only by one unit type (X in one, Y in the other,
        the rest equal), compared cell by cell (opponent, ground, role): the mean change and the
        share of cells where Y is better, and where it is significantly worse
    battles_meta.py h2h LEVEL A,B,C FILES..
        head-to-head of the named squads, per ground
    battles_meta.py targets LEVEL FILES..
        the owner's two targets per ground: squads nothing beats, the equilibrium; units with and
        without a place
"""
import collections
import glob
import json
import math
import re
import sys

import numpy as np


def load(files):
    for f in files:
        for l in open(f):
            if l.strip():
                yield json.loads(l)


def meta(args):
    level = args[0]
    metric = 'hp'
    ctxs = None
    only = None
    files = []
    i = 1
    while i < len(args):
        if args[i] == '--metric':
            metric = args[i + 1]; i += 2
        elif args[i] == '--only':
            only = set(json.load(open(args[i + 1]))[level]); i += 2
        elif args[i] == '--ctx':
            ctxs = args[i + 1].split(','); i += 2
        else:
            files.append(args[i]); i += 1

    cells = {}
    names = []
    costs = {}
    for j in load(files):
        if True:
            lv, ctx, pair = j['name'].split(':', 2)
            if lv != level or (ctxs and ctx not in ctxs):
                continue
            a, d = pair.split('>')
            if only and (a not in only or d not in only):
                continue
            r = j['runs']
            if r == 0:
                continue
            s, ss = j['net_' + metric]
            m = s / r
            var = max(0.0, ss / r - m * m)
            cells[(ctx, a, d)] = (m, math.sqrt(var / r), j)
            for n in (a, d):
                if n not in names:
                    names.append(n)
            if ctx == 'open' or a not in costs:
                costs[a] = j['sides'][0]['cost'] / r  # the committed units' cost
    ctx_list = sorted({c for c, _, _ in cells})
    missing = {n for n in names for m in names for c in ctx_list if (c, n, m) not in cells}
    if missing:
        print('dropped (cells missing):', ', '.join(sorted(missing)))
    names = [n for n in names if n not in missing]
    N = len(names)
    idx = {n: k for k, n in enumerate(names)}
    # result vectors: R[i] = list over (ctx, j, role) of (mean, se)
    def res(i, j, c, role):
        if role == 'att':
            m, se, _ = cells[(c, names[i], names[j])]
            return m, se
        m, se, _ = cells[(c, names[j], names[i])]
        return -m, se

    P = np.zeros((N, N))
    for i in range(N):
        for j in range(N):
            v = [res(i, j, c, r)[0] for c in ctx_list for r in ('att', 'def')]
            P[i, j] = sum(v) / len(v)

    # fictitious play
    def equilibrium(P, iters=200000):
        n = len(P)
        counts = np.zeros(n); counts[0] = 1
        payoff = P[:, 0].copy()  # cumulative payoff of each pure strategy vs opponent's history
        for t in range(iters):
            b = int(np.argmax(payoff))
            counts[b] += 1
            payoff += P[:, b]
        x = counts / counts.sum()
        return x, float((P @ x).max())

    x, exploit = equilibrium(P, 100000)
    avg = P.mean(axis=1)
    vs_eq = P @ x
    order = sorted(range(N), key=lambda i: -avg[i])
    print(f"== {level} [{metric}] grounds {','.join(ctx_list)}: {N} squads; equilibrium exploitability {exploit:.2f}")
    print(f"{'squad':<34}{'cost':>6}{'vs field':>9}{'vs eq':>8}{'eq%':>6}  worst matchup")
    for i in order:
        w = int(np.argmin(P[i]))
        print(f"{names[i]:<34}{costs.get(names[i],0):6.0f}{avg[i]:9.1f}{vs_eq[i]:8.1f}{100*x[i]:6.1f}  {P[i,w]:6.1f} vs {names[w]}")

    # dominance
    dom = {k: [] for k in range(N)}
    for i in range(N):
        for k in range(N):
            if i == k:
                continue
            ok = True
            diffs = []
            for j in range(N):
                for c in ctx_list:
                    for r in ('att', 'def'):
                        mi, si = res(i, j, c, r)
                        mk, sk = res(k, j, c, r)
                        margin = 2 * math.sqrt(si * si + sk * sk)
                        if mi < mk - margin:
                            ok = False
                            break
                        diffs.append(mi - mk)
                    if not ok: break
                if not ok: break
            if ok and sum(diffs) / len(diffs) > 0:
                dom[k].append(i)
    undominated = [k for k in range(N) if not dom[k]]
    print(f"\nundominated ({len(undominated)}): " + ', '.join(names[k] for k in sorted(undominated, key=lambda k: -avg[k])))
    print("dominated squads, by the best squad that dominates them:")
    for k in order:
        if dom[k]:
            best = max(dom[k], key=lambda i: avg[i])
            print(f"   {names[k]:<34} by {len(dom[k]):2d}, e.g. {names[best]}")
    # who dominates the most
    top = sorted(range(N), key=lambda i: -sum(i in dom[k] for k in range(N)))[:8]
    print("squads dominating the most others: " + ', '.join(f"{names[i]} ({sum(i in dom[k] for k in range(N))})" for i in top))


def swaps(files):
    cells = {}
    for j in load(files):
        if True:
            lv, c, pair = j['name'].split(':', 2); a, d = pair.split('>'); r = j['runs']
            if r == 0: continue
            s, ss = j['net_hp']; m = s / r
            cells[(lv, c, a, d)] = (m, math.sqrt(max(0, ss / r - m * m) / r))
    def parse(n):
        return {re.sub(r'^\d+', '', p): int(re.match(r'\d+', p).group()) for p in n.split('+')}
    by_lvl = collections.defaultdict(set)
    for (lv, c, a, d) in cells: by_lvl[lv].add(a)
    out = collections.defaultdict(list)
    for lv, names in by_lvl.items():
        P = {n: parse(n) for n in names}
        for A in names:
            for B in names:
                if A == B: continue
                ka, kb = set(P[A]), set(P[B])
                xa, xb = ka - kb, kb - ka
                if len(xa) != 1 or len(xb) != 1: continue
                if any(P[A][k] != P[B][k] for k in ka & kb): continue
                X, Y = xa.pop(), xb.pop()
                for (l2, c, a, d), (m, se) in cells.items():
                    if l2 != lv: continue
                    # attack cells: A attacks opp vs B attacks opp
                    if a == A and (lv, c, B, d) in cells:
                        m2, se2 = cells[(lv, c, B, d)]
                        out[(lv, X, Y)].append((m2 - m, math.sqrt(se * se + se2 * se2), c, A, B))
                    if d == A and (lv, c, a, B) in cells:
                        m2, se2 = cells[(lv, c, a, B)]
                        out[(lv, X, Y)].append((m - m2, math.sqrt(se * se + se2 * se2), c, A, B))
    rows = []
    for (lv, X, Y), v in out.items():
        n = len(v); mean = sum(x[0] for x in v) / n
        better = sum(x[0] > 0 for x in v) / n
        sig_worse = sum(x[0] < -2 * x[1] for x in v) / n
        ctxs = sorted({x[2] for x in v})
        pairs = len({(x[3], x[4]) for x in v})
        rows.append((lv, X, Y, mean, better, sig_worse, n, pairs, ctxs))
    order = {'early': 0, 'mid': 1, 'late': 2}
    print("swap X -> Y in otherwise identical squads: mean change in net HP%, share of cells where Y is better, share where Y is significantly worse (2 SE)")
    for lv, X, Y, mean, better, sw, n, pairs, ctxs in sorted(rows, key=lambda r: (order[r[0]], -r[3])):
        if mean <= 0: continue
        print(f"{lv:<5} {X:>8} -> {Y:<8} {mean:+6.1f}  better in {100*better:5.1f}%  sig. worse in {100*sw:4.1f}%  ({n} cells, {pairs} squad pairs, {','.join(ctxs)})")


def h2h(level, squads, files):
    cells = {}
    for j in load(files):
        if True:
            lv, c, pair = j['name'].split(':', 2); a, d = pair.split('>'); r = j['runs']
            if r == 0: continue
            s, ss = j['net_hp']; cells[(lv, c, a, d)] = (s / r, 100 * j['results'][0] / r)
    lv = level; names = squads
    G = ['open', 'forest', 'hills', 'walls']
    print(f"{lv}: row's net HP% vs column, mean of attacking and defending; per ground {'/'.join(G)}")
    for a in names:
        line = f"{a:<28}"
        for d in names:
            if a == d: line += f"{'—':>26}"; continue
            v = []
            for g in G:
                if (lv, g, a, d) in cells and (lv, g, d, a) in cells:
                    v.append((cells[(lv, g, a, d)][0] - cells[(lv, g, d, a)][0]) / 2)
                else: v.append(float('nan'))
            line += "  " + "/".join(f"{x:+4.0f}" for x in v)
        print(line)


def targets(level, files):
    """The owner's two targets, per ground: the squads nothing beats (by 2 standard errors) and the
    equilibrium's support; per unit type, the grounds where a squad with it is in the support (5%
    or more) or among the 5 best; and the unit types another one replaces for the better in 90% of
    comparisons and for the worse in none (2 standard errors)."""
    cells = {}
    for j in load(files):
        lv, c, pair = j['name'].split(':', 2)
        if lv != level or j['runs'] == 0:
            continue
        a, d = pair.split('>')
        r = j['runs']
        s, ss = j['net_hp']
        m = s / r
        cells[(c, a, d)] = (m, math.sqrt(max(0.0, ss / r - m * m) / r))
    unit = lambda n: [re.sub(r'^\d+', '', p) for p in n.split('+')]
    places = collections.defaultdict(set)
    total_unc = 0
    for g in sorted({c for c, _, _ in cells}):
        names = sorted({a for (c, a, _) in cells if c == g})
        names = [n for n in names if all((g, n, m) in cells and (g, m, n) in cells for m in names)]
        N = len(names)
        P = np.zeros((N, N))
        SE = np.zeros((N, N))
        for i, a in enumerate(names):
            for k, b in enumerate(names):
                m1, s1 = cells[(g, a, b)]
                m2, s2 = cells[(g, b, a)]
                P[i, k] = (m1 - m2) / 2
                SE[i, k] = math.sqrt(s1 * s1 + s2 * s2) / 2
        counts = np.zeros(N)
        counts[0] = 1
        pay = P[:, 0].copy()
        for _ in range(60000):
            b = int(np.argmax(pay))
            counts[b] += 1
            pay += P[:, b]
        x = counts / counts.sum()
        rank = {names[i]: r + 1 for r, i in enumerate(np.argsort(-P.mean(axis=1)))}
        unc = [names[i] for i in range(N) if all(P[i, k] >= -2 * SE[i, k] for k in range(N))]
        total_unc += len(unc)
        sup = [(names[i], round(100 * x[i])) for i in np.argsort(-x) if x[i] >= 0.02]
        print(f"{level} {g:<7} {N:2d} squads; equilibrium {sup}; nothing beats: {unc}")
        for i, n in enumerate(names):
            if x[i] >= 0.05 or rank[n] <= 5:
                for u in set(unit(n)):
                    places[u].add(g)
    allu = sorted({u for (_, a, _) in cells for u in unit(a)})
    print(f"{level}: {total_unc} squads nothing beats; units with a place: " + ', '.join(f"{u} ({'/'.join(sorted(places[u]))})" for u in allu if places[u]))
    print(f"{level}: units without a place: {[u for u in allu if not places[u]]}")


if __name__ == '__main__':
    cmd = sys.argv[1] if len(sys.argv) > 1 else ''
    if cmd == 'meta':
        meta(sys.argv[2:])
    elif cmd == 'swaps':
        swaps(sys.argv[2:])
    elif cmd == 'targets':
        targets(sys.argv[2], sys.argv[3:])
    elif cmd == 'h2h':
        h2h(sys.argv[2], sys.argv[3].split(','), sys.argv[4:])
    else:
        print(__doc__)
        sys.exit(2)
