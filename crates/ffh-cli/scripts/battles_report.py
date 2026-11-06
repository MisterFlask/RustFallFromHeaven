#!/usr/bin/env python3
"""Tables from `ffh battles` JSON lines (docs/ENGAGEMENTS.md, "Balance lab").

    battles_report.py rows FILE      one line per matchup: results, units and HP lost per side,
                                     cost-weighted damage each way
    battles_report.py matrix FILE    a round robin (names `prefix:A>B`): the attacker's net
                                     cost-weighted damage (hammers of enemy HP destroyed less its
                                     own), a ranking averaged over attack and defence, take rates
    battles_report.py hpmatrix FILE  the same with the net share of HP (enemy HP% lost less own)
    battles_report.py types FILE     per unit type: fall and withdrawal rates, damage dealt, taken
                                     and healed per battle, actions, misses, crits, kills, damage
                                     (and heals) per hammer, the rows it stood in
"""
import json, sys, collections

def load(f):
    return [json.loads(l) for l in open(f) if l.strip()]

def side_stats(j, s):
    S = j['sides'][s]; o = j['sides'][1-s]; r = j['runs']
    # cost-weighted damage taken: per type, (taken/100)/max_hp * cost
    def cw(side):
        tot = 0.0
        for t, v in side['types'].items():
            if v['max_hp'] == 0: continue
            per_hp_cost = v['cost'] / (v['max_hp'] / v['n'])  # cost per HP of one unit
            tot += v['taken'] / 100 * per_hp_cost
        return tot / r
    return dict(
        lost=S['fallen'] / r, esc=S['escaped'] / r, committed=S['committed'] / r,
        hp_lost_pct=100 * S['taken'] / 100 / max(1, S['max_hp']),
        cost=S['cost'] / r, cost_lost=S['cost_lost'] / r,
        cw_taken=cw(S), cw_dealt=cw(o),
    )

def row(j):
    r = j['runs']; a = side_stats(j, 0); d = side_stats(j, 1)
    res = [100 * x / r for x in j['results']]
    return res, a, d

def fmt_row(j):
    res, a, d = row(j)
    return (f"{j['name']:<44} taken {res[0]:5.1f} clr {res[1]:4.1f} rep {res[2]:5.1f} held {res[3]:5.1f} | "
            f"att lost {a['lost']:.2f}/{a['committed']:.1f} hp {a['hp_lost_pct']:5.1f}% | "
            f"def lost {d['lost']:.2f}/{d['committed']:.1f} hp {d['hp_lost_pct']:5.1f}% | "
            f"cw att->def {a['cw_dealt']:6.1f} def->att {d['cw_dealt']:6.1f}")

def matrix(js, prefix):
    # name "prefix:A>D"; value: attacker's net cost-weighted damage (dealt - taken) as % of its cost
    names = []
    cell = {}
    for j in js:
        k = j['name'].split(':', 1)[1]
        A, D = k.split('>')
        for n in (A, D):
            if n not in names: names.append(n)
        res, a, d = row(j)
        cell[(A, D)] = (a['cw_dealt'] - a['cw_taken'], a['cost'], d['cost'], res, a, d, d['hp_lost_pct'] - a['hp_lost_pct'])
    return names, cell

def print_matrix(js, title, idx=0):
    names, cell = matrix(js, '')
    for A in names:
        for D in names:
            cell.setdefault((A, D), (0, 0, 0, [0,0,0,0], None, None, 0))
    if idx:
        title += ' [HP-share net: enemy HP% lost - own HP% lost]'
    
    print(f"\n== {title}: attacker's net cost-weighted damage (dealt - taken), hammers per battle; row = attacker, col = defender")
    w = max(len(n) for n in names) + 1
    print(" " * w + "".join(f"{n[:9]:>10}" for n in names) + "   | att avg  def avg  overall")
    tot = {}
    for A in names:
        line = f"{A:<{w}}"
        for D in names:
            v = cell[(A, D)][idx]
            line += f"{v:10.0f}"
        att_avg = sum(cell[(A, D)][idx] for D in names) / len(names)
        # as defender: -(attacker's net) when A defends
        def_avg = sum(-cell[(X, A)][idx] for X in names) / len(names)
        tot[A] = (att_avg + def_avg) / 2
        line += f"   | {att_avg:7.0f} {def_avg:8.0f} {tot[A]:8.0f}"
        print(line)
    print("ranking (net per battle, avg over attack and defence, vs field):")
    costs = {A: cell[(A, A)][1] for A in names}
    for A in sorted(names, key=lambda n: -tot[n]):
        print(f"   {A:<{w}} {tot[A]:7.0f}   (army cost {costs[A]:.0f})")
    # take rate
    print("plot taken % (row attacks col):")
    print(" " * w + "".join(f"{n[:9]:>10}" for n in names))
    for A in names:
        print(f"{A:<{w}}" + "".join(f"{cell[(A, D)][3][0]:10.0f}" for D in names))

def types_table(js, title, side=None):
    agg = collections.defaultdict(lambda: collections.Counter())
    for j in js:
        for s in (0, 1):
            if side is not None and s != side: continue
            for t, v in j['sides'][s]['types'].items():
                c = agg[t]
                for k in ('n', 'fell', 'withdrew', 'max_hp', 'dealt', 'taken', 'healed', 'actions', 'swings', 'misses', 'crits', 'kills'):
                    c[k] += v[k]
                c['cost'] = v['cost']
                for i in range(3): c[f'r{i}'] += v['rows'][i]
    print(f"\n== {title}: per unit type, per battle fought (sums over all matchups)")
    print(f"{'type':<30}{'cost':>5}{'n':>7}{'fell%':>7}{'wd%':>6}{'dealt':>7}{'taken':>7}{'heal':>6}{'act':>5}{'miss%':>6}{'crit%':>6}{'kills':>6}{'dlt/c':>7}  rows F/M/R")
    for t, c in sorted(agg.items(), key=lambda kv: -kv[1]['dealt'] / max(1, kv[1]['n']) / max(1, kv[1]['cost'])):
        n = c['n']
        print(f"{t[5:]:<30}{c['cost']:>5}{n:>7}{100*c['fell']/n:7.1f}{100*c['withdrew']/n:6.1f}{c['dealt']/100/n:7.1f}{c['taken']/100/n:7.1f}{c['healed']/100/n:6.1f}{c['actions']/n:5.1f}{100*c['misses']/max(1,c['swings']):6.1f}{100*c['crits']/max(1,c['swings']):6.1f}{c['kills']/n:6.2f}{100*(c['dealt']+c['healed'])/100/n/max(1,c['cost']):7.1f}  {100*c['r0']/n:.0f}/{100*c['r1']/n:.0f}/{100*c['r2']/n:.0f}")

if __name__ == '__main__':
    mode = sys.argv[1]
    js = load(sys.argv[2])
    if mode == 'rows':
        for j in js: print(fmt_row(j))
    elif mode == 'matrix':
        print_matrix(js, sys.argv[2])
    elif mode == 'hpmatrix':
        print_matrix(js, sys.argv[2], 6)
    elif mode == 'types':
        types_table(js, sys.argv[2])
