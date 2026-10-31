#!/usr/bin/env python3
"""Float reference for the engagement estimator's moment integrator (crates/ffh-core/src/engagement/
expect.rs), on the review's ten battles of one class (frontline only, 120 HP, each side dealing a
fixed mean hit). It is the phase 6 prototype's method (moments: alive weight, mean and variance of
the damage taken; q = a_h / (1 + sum of the other targets' weights); alive = 1 - Phi((mu - hp) /
sqrt(v))) with the integrator's rules: the stop when a side's alive weight is below 0.05, checked
after each round; r's discrete variance (80..=120: 140 %^2); the losing side's residual weight
counted as lost. With --presence it adds the covariance of a hitter's hits across rounds (the
integrator's `Tuning::presence` variant). Prints the rows of `expect::tests::CASES`:
(n_att, n_def, attacker's mean hit, defender's, [losses x1000 per side, rounds]).

    python3 tools/engagement_moments.py [--presence]
"""
import math
import sys

STOP = 0.05
VR = 0.014


def phi_cdf(z):
    return 0.5 * (1 + math.erf(z / math.sqrt(2)))


def moment(att, dfn, presence=False, rounds=40):
    sides = [[{'hp': hp, 'm': m, 'mu': 0.0, 'v': 0.0, 'al': 1.0} for hp, m in att],
             [{'hp': hp, 'm': m, 'mu': 0.0, 'v': 0.0, 'al': 1.0} for hp, m in dfn]]
    acc = {}
    played = 0

    def side_alive(s):
        return sum(f['al'] for f in sides[s])

    for r in range(rounds):
        played = r + 1
        inc = [[(0.0, 0.0) for _ in s] for s in sides]
        new_acc = {}
        for s in (0, 1):
            hitters, targets = sides[s], sides[1 - s]
            total = sum(t['al'] for t in targets)
            for ti, t in enumerate(targets):
                if t['al'] <= 0:
                    continue
                n = 1 + total - t['al']
                dm = dv = 0.0
                for hi, h in enumerate(hitters):
                    q, m = h['al'] / n, h['m']
                    dm += q * m
                    dv += q * m * m * (1 + VR) - q * q * m * m
                    if presence:
                        key = (s, hi, ti)
                        a = acc.get(key, 0.0)
                        dv += 2 * (q * m) * a
                        new_acc[key] = a + (q * m / h['al']) * (1 - h['al']) if h['al'] > 0 else a
                inc[1 - s][ti] = (dm, dv)
        acc = new_acc
        for s in (0, 1):
            for ti, t in enumerate(sides[s]):
                if t['al'] <= 0:
                    continue
                dm, dv = inc[s][ti]
                t['mu'] += dm
                t['v'] += dv
                alive = 1 - phi_cdf((t['mu'] - t['hp']) / math.sqrt(t['v'])) if t['v'] > 0 else (1.0 if t['mu'] < t['hp'] else 0.0)
                t['al'] = min(t['al'], alive)
        if side_alive(0) < STOP or side_alive(1) < STOP:
            break
    loss = [len(att) - side_alive(0), len(dfn) - side_alive(1)]
    for s in (0, 1):
        if side_alive(s) < STOP:
            loss[s] = len(sides[s])
    return loss[0], loss[1], played


CASES = [(4, 4, 1000, 1000), (8, 8, 1000, 1000), (8, 6, 1000, 1000), (8, 8, 1100, 900), (8, 4, 1000, 1000),
         (8, 8, 800, 1200), (1, 1, 1000, 1000), (2, 1, 1000, 1000), (8, 8, 1200, 800), (6, 3, 700, 1300)]

if __name__ == '__main__':
    presence = '--presence' in sys.argv[1:]
    for na, nd, ma, md in CASES:
        la, ld, played = moment([(12000, ma)] * na, [(12000, md)] * nd, presence)
        print(f"({na}, {nd}, {ma}, {md}, [{la * 1000:.0f}, {ld * 1000:.0f}, {played}]),")
