#!/usr/bin/env python3
"""Paired A/B of leader-trait play (crates/ffh-ai/src/traits.rs), per trait, over every player.

Both sides are `ai_metrics.py --out` files of the same seeds and settings (random civilizations:
the same seed gives the same leaders on both sides), typically the same binary with
`FFH_AI_TRAIT_OFF=all` on the base side. Every player present at the end of a game on both sides
is a pair; a trait's rows are the pairs whose leader has it at the start (Adaptive and Insane
leaders change traits later). The Infernals and Mercurians, who join mid-game (the Mercurians on
the team of the civ that built their gate, whose wins they share), are left out unless
`--joiners`.

Per trait (and pooled over players with none of the played traits, the spill-over on the rest):
- win: the player's team decided the game (victory, last standing, `--until-supremacy`'s clear
  supremacy) or held the largest share at the end of an undecided one; the seeds won by one side
  only, with an exact two-sided sign test;
- share (score v2, basis points of the world; dead = 0) at the checkpoints and at the end, legacy
  score at the end, with the paired t over pairs (|t| < 2 is noise);
- survival, and the end-of-game counters that show the trait's play (great people, wonders,
  casters, summons, cities taken, ...).

usage:
  trait_ab.py --base off.json --new on.json [--at 100,200,300] [--trait TRAIT_X ..]
              [--counters gp_created,wonders,..] [--per-player]
"""
import argparse
import json
import math
import os
import re
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.normpath(os.path.join(HERE, '..', '..', '..'))
LEADERS_XML = os.path.join(ROOT, 'data', 'xml', 'Civilizations', 'CIV4LeaderHeadInfos.xml')

# The traits traits.rs plays, in its order.
PLAYED = ['RAIDERS', 'SPIRITUAL', 'ARCANE', 'SUMMONER', 'PHILOSOPHICAL', 'FINANCIAL', 'INDUSTRIOUS',
          'AGGRESSIVE', 'INGENUITY', 'TOLERANT', 'EXPANSIVE']

# Civilizations that join mid-game (not in a starting slot).
JOINERS = ('MERCURIANS', 'INFERNAL')

COUNTERS = ['cities', 'pop', 'techs', 'research', 'gold', 'gp_created', 'wonders', 'nat_wonders',
            'casters', 'summons', 'mil_str', 'taken_civ', 'lost_civ', 'razed', 'wars', 'anarchy_turns',
            'impr']


def leader_traits():
    with open(LEADERS_XML) as f:
        s = f.read()
    out = {}
    for m in re.finditer(r'<LeaderHeadInfo>(.*?)</LeaderHeadInfo>', s, re.S):
        b = m.group(1)
        t = re.search(r'<Type>LEADER_(.*?)</Type>', b)
        if not t:
            continue
        tr = re.search(r'<Traits>(.*?)</Traits>', b, re.S)
        names = []
        if tr:
            for x in re.finditer(r'<TraitType>TRAIT_(.*?)</TraitType>\s*<bTrait>1</bTrait>', tr.group(1)):
                names.append(x.group(1))
        out[t.group(1)] = names
    return out


def load(path):
    with open(path) as f:
        games = json.load(f)
    return {g['seed']: g for g in games if 'error' not in g}


def final_turn(game):
    return max(int(t) for t in game['snapshots'])


def rows_at(game, turn):
    return game['snapshots'].get(str(turn)) or []


def winner_team(game):
    o = game.get('outcome') or {}
    if o.get('kind') in ('victory', 'supremacy', 'last_standing') and o.get('team', -1) >= 0:
        return o['team']
    by_team = {}
    for r in rows_at(game, final_turn(game)):
        if r.get('barb') or not r.get('alive'):
            continue
        by_team[r['team']] = by_team.get(r['team'], 0) + r.get('score', 0)
    if not by_team:
        return -1
    return max(sorted(by_team), key=lambda t: by_team[t])


def row(game, turn, p):
    """The player's row at `turn` ('end', or a checkpoint: a game decided before keeps its end)."""
    end = final_turn(game)
    t = end if turn == 'end' else min(int(turn), end)
    for r in rows_at(game, t):
        if r['p'] == p:
            return r
    return None


def value(r, field):
    if r is None or not r.get('alive', True):
        return 0
    v = r.get(field, 0)
    if isinstance(v, bool):
        return 1 if v else 0
    return v if isinstance(v, (int, float)) else 0


def facts(game, p, checkpoints, counters):
    me = row(game, 'end', p)
    if me is None or me.get('barb'):
        return None
    f = {'win': 1 if winner_team(game) == me['team'] else 0, 'alive': 1 if me.get('alive') else 0,
         'leader': me.get('leader', '?'), 'civ': me.get('civ', '?')}
    for t in checkpoints + ['end']:
        f['share@%s' % t] = value(row(game, t, p), 'score')
    f['legacy@end'] = value(me, 'legacy_score')
    for c in counters:
        # Cumulative counters keep their value after death; the rest are 0 when dead.
        v = me.get(c, 0)
        f[c] = v if isinstance(v, (int, float)) and not isinstance(v, bool) else 0
    return f


def mean(v):
    return sum(v) / len(v) if v else float('nan')


def paired_t(a, b):
    d = [y - x for x, y in zip(a, b)]
    n = len(d)
    if n < 2:
        return mean(d), float('nan')
    m = mean(d)
    var = sum((x - m) ** 2 for x in d) / (n - 1)
    if var == 0:
        return m, (float('inf') if m else 0.0)
    return m, m / math.sqrt(var / n)


def sign_test(k, n):
    if n == 0:
        return 1.0
    k = min(k, n - k)
    return min(1.0, 2 * sum(math.comb(n, i) for i in range(0, k + 1)) / 2 ** n)


def report(name, pairs, checkpoints, counters):
    n = len(pairs)
    if n == 0:
        print('%-14s no players' % name)
        return
    fb = [b for b, _ in pairs]
    fn = [x for _, x in pairs]
    wb, wn = sum(f['win'] for f in fb), sum(f['win'] for f in fn)
    only_new = sum(1 for b, x in pairs if x['win'] and not b['win'])
    only_base = sum(1 for b, x in pairs if b['win'] and not x['win'])
    print('%-14s n=%-3d win %5.1f%% -> %5.1f%%  one side only %d / %d (p %.2f)  alive %.2f -> %.2f' % (
        name, n, 100.0 * wb / n, 100.0 * wn / n, only_new, only_base, sign_test(only_new, only_new + only_base),
        mean([f['alive'] for f in fb]), mean([f['alive'] for f in fn])))
    parts = []
    for t in checkpoints + ['end']:
        k = 'share@%s' % t
        d, tt = paired_t([f[k] for f in fb], [f[k] for f in fn])
        parts.append('%s %+.0f (t %.2f)' % (k, d, tt))
    d, tt = paired_t([f['legacy@end'] for f in fb], [f['legacy@end'] for f in fn])
    parts.append('legacy@end %+.0f (t %.2f)' % (d, tt))
    print('    ' + '  '.join(parts))
    parts = []
    for c in counters:
        a, b = [f[c] for f in fb], [f[c] for f in fn]
        d, tt = paired_t(a, b)
        if d == 0:
            continue
        parts.append('%s %.1f->%.1f (t %.1f)' % (c, mean(a), mean(b), tt))
    for i in range(0, len(parts), 5):
        print('    ' + '  '.join(parts[i:i + 5]))


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--base', required=True)
    ap.add_argument('--new', required=True)
    ap.add_argument('--at', default='100,200,300')
    ap.add_argument('--trait', action='append', default=[], help='TRAIT_X (default: every played trait)')
    ap.add_argument('--counters', default=','.join(COUNTERS))
    ap.add_argument('--per-player', action='store_true', help='list every pair whose win or share@end differs')
    ap.add_argument('--joiners', action='store_true', help='count the Infernals and Mercurians too')
    a = ap.parse_args()
    checkpoints = [t for t in a.at.split(',') if t]
    counters = [c for c in a.counters.split(',') if c]
    traits_of = leader_traits()
    base, new = load(a.base), load(a.new)
    seeds = sorted(set(base) & set(new))
    pairs = []  # (seed, p, traits, base facts, new facts)
    for s in seeds:
        ps = sorted({r['p'] for r in rows_at(base[s], final_turn(base[s]))} & {r['p'] for r in rows_at(new[s], final_turn(new[s]))})
        for p in ps:
            fb, fn = facts(base[s], p, checkpoints, counters), facts(new[s], p, checkpoints, counters)
            if fb is None or fn is None or fb['leader'] != fn['leader']:
                continue
            if not a.joiners and fb['civ'] in JOINERS:
                continue
            pairs.append((s, p, traits_of.get(fb['leader'], []), fb, fn))
    print('%d seeds, %d player pairs (%s vs %s)' % (len(seeds), len(pairs), a.base, a.new))
    wanted = [t.upper().replace('TRAIT_', '') for t in a.trait] or PLAYED
    for t in wanted:
        report(t, [(b, x) for _, _, trs, b, x in pairs if t in trs], checkpoints, counters)
    report('(none played)', [(b, x) for _, _, trs, b, x in pairs if not set(trs) & set(PLAYED)], checkpoints, counters)
    report('(all)', [(b, x) for _, _, _, b, x in pairs], checkpoints, counters)
    if a.per_player:
        for s, p, trs, b, x in pairs:
            if b['win'] != x['win'] or abs(b['share@end'] - x['share@end']) > 500:
                print('seed %d p%d %s %s %s: win %d -> %d, share@end %d -> %d' % (
                    s, p, b['civ'], b['leader'], '/'.join(trs), b['win'], x['win'], b['share@end'], x['share@end']))


if __name__ == '__main__':
    sys.exit(main())
