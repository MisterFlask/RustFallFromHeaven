#!/usr/bin/env python3
"""The battle battery of the engagement balance pass (docs/ENGAGEMENTS.md, "Balance lab"):
writes one `ffh battles` spec file per section into OUTDIR (default: the current directory).

    python3 crates/ffh-cli/scripts/battles_battery.py OUT
    for f in OUT/*.txt; do tools/agents/slot.sh $B/ffh battles $f --runs 200 > ${f%.txt}.jsonl; done
    python3 crates/ffh-cli/scripts/battles_report.py matrix OUT/army_mid.jsonl

Sections: marg_* (a core line plus 2 of X, round robin per era), army_* (plausible armies of
about equal cost, round robin per era), ground (an era's army against typical defences by
ground), tiers and cost (numbers against quality), promos (promotion sets), probes (row
overrides, mounted, one-turn city captures in waves, large units).
"""
import itertools
import os
import sys

OUT = sys.argv[1] if len(sys.argv) > 1 else "."
os.makedirs(OUT, exist_ok=True)


def write(name, lines):
    with open(os.path.join(OUT, name), "w") as f:
        f.write("\n".join(lines) + "\n")


def armies_rr(prefix, armies, ground="open"):
    out = []
    for (an, a), (dn, d) in itertools.product(armies.items(), repeat=2):
        out.append(f"{prefix}:{an}>{dn} | {a} | {d} | {ground}")
    return out

# --- Marginal value: a core line plus 2 of X, round robin.
early_core = "5*AXEMAN"
early_x = {
    "axe": "2*AXEMAN", "warrior": "2*WARRIOR", "archer": "2*ARCHER", "horse": "2*HORSEMAN",
    "hunter": "2*HUNTER", "chariot": "2*CHARIOT", "adept": "2*ADEPT", "disciple": "2*DISCIPLE_THE_ORDER",
    "catapult": "2*CATAPULT", "skirm": "2*SKIRMISHER", "scout": "2*SCOUT",
}
mid_core = "5*CHAMPION"
mid_x = {
    "champ": "2*CHAMPION", "longbow": "2*LONGBOWMAN", "horsearch": "2*HORSE_ARCHER", "mage": "2*MAGE",
    "firemage": "2*MAGE+FIRE1+FIRE2", "airmage": "2*MAGE+AIR1+AIR2", "priest": "2*PRIEST_OF_THE_ORDER",
    "assassin": "2*ASSASSIN", "trebuchet": "2*TREBUCHET", "ranger": "2*RANGER", "elephant": "1*WAR_ELEPHANT",
    "axe": "2*AXEMAN",
}
late_core = "5*PHALANX"
late_x = {
    "phalanx": "2*PHALANX", "crossbow": "2*CROSSBOWMAN", "marksman": "2*MARKSMAN", "knight": "2*KNIGHT",
    "archmage": "2*ARCHMAGE", "firearch": "2*ARCHMAGE+FIRE1+FIRE2", "hipriest": "2*HIGH_PRIEST_OF_THE_ORDER",
    "shadow": "2*SHADOW", "cannon": "2*CANNON", "berserker": "2*BERSERKER", "paladin": "2*PALADIN",
    "druid": "2*DRUID", "champ": "2*CHAMPION",
}
def marg(prefix, core, xs):
    return armies_rr(prefix, {k: f"{core}, {v}" for k, v in xs.items()})

write("marg_early.txt", marg("me", early_core, early_x))
write("marg_mid.txt", marg("mm", mid_core, mid_x))
write("marg_late.txt", marg("ml", late_core, late_x))

# --- Plausible armies per era, round robin on open ground (about equal cost).
early = {
    "axe8": "8*AXEMAN",
    "axe5arch3": "5*AXEMAN, 3*ARCHER",
    "arch6axe2": "2*AXEMAN, 6*ARCHER",
    "horse8": "8*HORSEMAN",
    "combined": "4*AXEMAN, 2*ARCHER, 2*HORSEMAN",
    "adept": "4*AXEMAN, 2*ARCHER, 1*ADEPT, 1*WARRIOR",
    "siege": "4*AXEMAN, 1*ARCHER, 2*CATAPULT",
    "heal": "5*AXEMAN, 1*ARCHER, 2*DISCIPLE_THE_ORDER",
    "warrior10": "10*WARRIOR",
    "hunter8": "8*HUNTER",
    "chariot4": "4*CHARIOT",
}
mid = {
    "champ8": "8*CHAMPION",
    "champ5lb3": "5*CHAMPION, 3*LONGBOWMAN",
    "lb6champ2": "2*CHAMPION, 6*LONGBOWMAN",
    "ha8": "8*HORSE_ARCHER",
    "combined": "4*CHAMPION, 2*LONGBOWMAN, 2*HORSE_ARCHER",
    "mage": "4*CHAMPION, 2*LONGBOWMAN, 2*MAGE",
    "firemage": "4*CHAMPION, 2*LONGBOWMAN, 2*MAGE+FIRE1+FIRE2",
    "priest": "5*CHAMPION, 1*LONGBOWMAN, 2*PRIEST_OF_THE_ORDER",
    "assassin": "4*CHAMPION, 4*ASSASSIN",
    "siege": "4*CHAMPION, 2*LONGBOWMAN, 2*TREBUCHET",
    "ranger6": "6*RANGER",
    "elephant4": "4*WAR_ELEPHANT",
}
late = {
    "phalanx8": "8*PHALANX",
    "imm5xb3": "5*IMMORTAL, 3*CROSSBOWMAN",
    "knight8": "8*KNIGHT",
    "combined": "4*PHALANX, 2*CROSSBOWMAN, 2*KNIGHT",
    "archmage": "4*PHALANX, 2*CROSSBOWMAN, 2*ARCHMAGE+FIRE1+FIRE2",
    "hipriest": "5*PHALANX, 1*CROSSBOWMAN, 2*HIGH_PRIEST_OF_THE_ORDER",
    "shadow": "4*PHALANX, 4*SHADOW",
    "cannon": "4*PHALANX, 2*CROSSBOWMAN, 2*CANNON",
    "paladin8": "8*PALADIN",
    "berserk8": "8*BERSERKER",
    "marksman": "4*PHALANX, 4*MARKSMAN",
}
write("army_early.txt", armies_rr("ae", early))
write("army_mid.txt", armies_rr("am", mid))
write("army_late.txt", armies_rr("al", late))

# --- Ground: an era's combined army against a typical defence, by ground.
grounds = ["open", "forest", "hills", "fort:5", "city", "city fort:5", "walls", "walls fort:5", "hills city fort:5"]
g = []
for era, att, defs in [
    ("early", "4*AXEMAN, 2*ARCHER, 2*HORSEMAN", ["2*AXEMAN, 3*ARCHER", "4*AXEMAN, 4*ARCHER", "3*ARCHER", "2*WARRIOR, 2*ARCHER"]),
    ("early8axe", "8*AXEMAN", ["2*AXEMAN, 3*ARCHER", "4*AXEMAN, 4*ARCHER"]),
    ("early_siege", "4*AXEMAN, 2*ARCHER, 2*CATAPULT", ["2*AXEMAN, 3*ARCHER", "4*AXEMAN, 4*ARCHER"]),
    ("mid", "4*CHAMPION, 2*LONGBOWMAN, 2*HORSE_ARCHER", ["2*CHAMPION, 3*LONGBOWMAN", "4*CHAMPION, 4*LONGBOWMAN"]),
    ("mid_fire", "4*CHAMPION, 2*LONGBOWMAN, 2*MAGE+FIRE1+FIRE2", ["2*CHAMPION, 3*LONGBOWMAN", "4*CHAMPION, 4*LONGBOWMAN"]),
    ("late", "4*PHALANX, 2*CROSSBOWMAN, 2*KNIGHT", ["2*PHALANX, 3*CROSSBOWMAN", "4*PHALANX, 4*CROSSBOWMAN"]),
]:
    for di, d in enumerate(defs):
        for gr in grounds:
            g.append(f"gr:{era}>def{di}@{gr.replace(' ', '+')} | {att} | {d} | {gr}")
write("ground.txt", g)

# --- Tier gaps and numbers.
t = []
gaps = [
    ("10war>4champ", "10*WARRIOR", "4*CHAMPION"),
    ("10axe>5champ", "10*AXEMAN", "5*CHAMPION"),
    ("8axe>4champ", "8*AXEMAN", "4*CHAMPION"),
    ("8champ>4phalanx", "8*CHAMPION", "4*PHALANX"),
    ("10axe>4phalanx", "10*AXEMAN", "4*PHALANX"),
    ("10axe>2phalanx", "10*AXEMAN", "2*PHALANX"),
    ("8champ>2irongolem", "8*CHAMPION", "2*IRON_GOLEM"),
    ("4phalanx>8champ", "4*PHALANX", "8*CHAMPION"),
    ("4champ>8axe", "4*CHAMPION", "8*AXEMAN"),
    ("4champ>10war", "4*CHAMPION", "10*WARRIOR"),
    ("1champ>1axe", "1*CHAMPION", "1*AXEMAN"),
    ("1axe>1champ", "1*AXEMAN", "1*CHAMPION"),
    ("1phalanx>3axe", "1*PHALANX", "3*AXEMAN"),
    ("3axe>1phalanx", "3*AXEMAN", "1*PHALANX"),
    ("1knight>4horse", "1*KNIGHT", "4*HORSEMAN"),
    ("6archer>6longbow", "6*ARCHER", "6*LONGBOWMAN"),
    ("6longbow>6archer", "6*LONGBOWMAN", "6*ARCHER"),
    ("4axe+2adept>4axe+2mage", "4*AXEMAN, 2*ADEPT", "4*AXEMAN, 2*MAGE"),
    ("4axe+2mage>4axe+2archmage", "4*AXEMAN, 2*MAGE", "4*AXEMAN, 2*ARCHMAGE"),
    ("4ph+2disc>4ph+2hipriest", "4*PHALANX, 2*DISCIPLE_THE_ORDER", "4*PHALANX, 2*HIGH_PRIEST_OF_THE_ORDER"),
    ("4ph+2hipriest>4ph+2disc", "4*PHALANX, 2*HIGH_PRIEST_OF_THE_ORDER", "4*PHALANX, 2*DISCIPLE_THE_ORDER"),
]
for n, a, d in gaps:
    t.append(f"tier:{n} | {a} | {d} | open")
write("tiers.txt", t)

# --- Promotions: 6 axemen with a promotion set vs 6 plain axemen (both ways), and specialists.
p = []
promos = {
    "c1": "+COMBAT1", "c2": "+COMBAT1+COMBAT2", "c3": "+COMBAT1+COMBAT2+COMBAT3",
    "c5": "+COMBAT1+COMBAT2+COMBAT3+COMBAT4+COMBAT5",
    "d1": "+DRILL1", "d2": "+DRILL1+DRILL2", "d4": "+DRILL1+DRILL2+DRILL3+DRILL4",
    "bronze": "+BRONZE_WEAPONS", "iron": "+IRON_WEAPONS", "mithril": "+MITHRIL_WEAPONS",
    "shock": "+SHOCK", "formation": "+FORMATION", "mres": "+MAGIC_RESISTANCE",
}
for k, v in promos.items():
    p.append(f"pr:axe_{k}>axe | 6*AXEMAN{v} | 6*AXEMAN | open")
    p.append(f"pr:axe>axe_{k} | 6*AXEMAN | 6*AXEMAN{v} | open")
p.append("pr:horse>axe_formation | 6*HORSEMAN | 6*AXEMAN+FORMATION | open")
p.append("pr:horse>axe | 6*HORSEMAN | 6*AXEMAN | open")
p.append("pr:horse>axe_formation+arch | 4*HORSEMAN, 2*AXEMAN | 4*AXEMAN+FORMATION, 2*ARCHER | open")
p.append("pr:horse>axe+arch | 4*HORSEMAN, 2*AXEMAN | 4*AXEMAN, 2*ARCHER | open")
p.append("pr:axe+2mage>axe_mres | 4*AXEMAN, 2*MAGE | 6*AXEMAN+MAGIC_RESISTANCE | open")
p.append("pr:axe+2mage>axe | 4*AXEMAN, 2*MAGE | 6*AXEMAN | open")
p.append("pr:axe+2arch>axe+2arch_mk | 4*AXEMAN, 2*ARCHER | 4*AXEMAN, 2*ARCHER+MARKSMAN | open")
p.append("pr:axe+2arch_mk>axe+2arch | 4*AXEMAN, 2*ARCHER+MARKSMAN | 4*AXEMAN, 2*ARCHER | open")
p.append("pr:axe_cr2>axe+arch@city | 6*AXEMAN+CITY_RAIDER1+CITY_RAIDER2 | 3*AXEMAN, 3*ARCHER | city fort:5")
p.append("pr:axe>axe+arch@city | 6*AXEMAN | 3*AXEMAN, 3*ARCHER | city fort:5")
p.append("pr:axe>axe+arch_cg2@city | 6*AXEMAN | 3*AXEMAN, 3*ARCHER+CITY_GARRISON1+CITY_GARRISON2 | city fort:5")
p.append("pr:axe>axe_guer@hills | 6*AXEMAN | 6*AXEMAN+GUERILLA1 | hills")
p.append("pr:axe>axe@hills | 6*AXEMAN | 6*AXEMAN | hills")
write("promos.txt", p)

# --- Probes: row overrides, mounted, one-turn city captures in waves, large units; and equal
# cost against a fixed line, numbers against one strong unit.
PROBES = '''
# --- Row overrides: the same army, a unit kind moved to another row (vs a standard defence).
rows:early heal@auto      | 5*AXEMAN, 1*ARCHER, 2*DISCIPLE_THE_ORDER | 5*AXEMAN, 3*ARCHER | open
rows:early heal@rear      | 5*AXEMAN, 1*ARCHER, 2*DISCIPLE_THE_ORDER@rear | 5*AXEMAN, 3*ARCHER | open
rows:early axe-instead    | 7*AXEMAN, 1*ARCHER | 5*AXEMAN, 3*ARCHER | open
rows:mid heal@auto        | 5*CHAMPION, 1*LONGBOWMAN, 2*PRIEST_OF_THE_ORDER | 5*CHAMPION, 3*LONGBOWMAN | open
rows:mid heal@rear        | 5*CHAMPION, 1*LONGBOWMAN, 2*PRIEST_OF_THE_ORDER@rear | 5*CHAMPION, 3*LONGBOWMAN | open
rows:mid champ-instead    | 7*CHAMPION, 1*LONGBOWMAN | 5*CHAMPION, 3*LONGBOWMAN | open
rows:late heal@auto       | 5*PHALANX, 1*CROSSBOWMAN, 2*HIGH_PRIEST_OF_THE_ORDER | 5*PHALANX, 3*CROSSBOWMAN | open
rows:late heal@rear       | 5*PHALANX, 1*CROSSBOWMAN, 2*HIGH_PRIEST_OF_THE_ORDER@rear | 5*PHALANX, 3*CROSSBOWMAN | open
rows:late phalanx-instead | 7*PHALANX, 1*CROSSBOWMAN | 5*PHALANX, 3*CROSSBOWMAN | open
rows:mid fire@auto(rear)  | 4*CHAMPION, 2*LONGBOWMAN, 2*MAGE+FIRE1+FIRE2 | 5*CHAMPION, 3*LONGBOWMAN | open
rows:mid fire@middle      | 4*CHAMPION, 2*LONGBOWMAN, 2*MAGE+FIRE1+FIRE2@middle | 5*CHAMPION, 3*LONGBOWMAN | open
rows:mid fire+lb@rear     | 4*CHAMPION, 2*LONGBOWMAN@rear, 2*MAGE+FIRE1+FIRE2@middle | 5*CHAMPION, 3*LONGBOWMAN | open
rows:mid lb@auto(middle)  | 5*CHAMPION, 3*LONGBOWMAN | 5*CHAMPION, 3*LONGBOWMAN | open
rows:mid lb@rear          | 5*CHAMPION, 3*LONGBOWMAN@rear | 5*CHAMPION, 3*LONGBOWMAN | open
rows:mid ha@auto          | 5*CHAMPION, 3*HORSE_ARCHER | 5*CHAMPION, 3*LONGBOWMAN | open
rows:mid ha@front         | 4*CHAMPION, 1*CHAMPION@middle, 3*HORSE_ARCHER@front | 5*CHAMPION, 3*LONGBOWMAN | open
rows:late knight@auto     | 4*PHALANX, 2*CROSSBOWMAN, 2*KNIGHT | 5*PHALANX, 3*CROSSBOWMAN | open
rows:late knight@front    | 4*PHALANX, 2*CROSSBOWMAN, 2*KNIGHT@front | 5*PHALANX, 3*CROSSBOWMAN | open
rows:late berserk@auto    | 5*PHALANX, 3*BERSERKER | 5*PHALANX, 3*CROSSBOWMAN | open
rows:late berserk@front   | 5*PHALANX, 3*BERSERKER@front | 5*PHALANX, 3*CROSSBOWMAN | open
rows:early 4arch@middle   | 4*AXEMAN, 4*ARCHER | 4*AXEMAN, 4*ARCHER | open
rows:early 4arch@rear     | 4*AXEMAN, 4*ARCHER@rear | 4*AXEMAN, 4*ARCHER | open
rows:early 2arch@rear     | 4*AXEMAN, 2*ARCHER, 2*ARCHER@rear | 4*AXEMAN, 4*ARCHER | open
# --- Mounted: where its edge comes from.
mnt:horse8>axe8 open        | 8*HORSEMAN | 8*AXEMAN | open
mnt:horse8>axe8 forest      | 8*HORSEMAN | 8*AXEMAN | forest
mnt:axe8>horse8 open        | 8*AXEMAN | 8*HORSEMAN | open
mnt:horse8>axe8form open    | 8*HORSEMAN | 8*AXEMAN+FORMATION | open
mnt:horse8>axe5arch3 open   | 8*HORSEMAN | 5*AXEMAN, 3*ARCHER | open
mnt:horse8>axe5arch3 city   | 8*HORSEMAN | 5*AXEMAN, 3*ARCHER | city
mnt:ha8>champ8 open         | 8*HORSE_ARCHER | 8*CHAMPION | open
mnt:ha8>champ8 forest       | 8*HORSE_ARCHER | 8*CHAMPION | forest
mnt:champ8>ha8 open         | 8*CHAMPION | 8*HORSE_ARCHER | open
mnt:knight8>phalanx8 open   | 8*KNIGHT | 8*PHALANX | open
mnt:knight8>phalanx8 forest | 8*KNIGHT | 8*PHALANX | forest
mnt:knight8>phalanx8 city   | 8*KNIGHT | 8*PHALANX | city
mnt:phalanx8>knight8 open   | 8*PHALANX | 8*KNIGHT | open
mnt:knight8>phalanx8form    | 8*KNIGHT | 8*PHALANX+FORMATION | open
# --- Taking a city in one turn: waves of attackers.
siege:early 16axe>walls def0       | 16*AXEMAN | 2*AXEMAN, 3*ARCHER | walls fort:5 waves:3
siege:early 16mix>walls def0       | 8*AXEMAN, 4*ARCHER, 4*HORSEMAN | 2*AXEMAN, 3*ARCHER | walls fort:5 waves:3
siege:early 16cat>walls def0       | 8*AXEMAN, 4*ARCHER, 4*CATAPULT | 2*AXEMAN, 3*ARCHER | walls fort:5 waves:3
siege:early 16axe>city def0        | 16*AXEMAN | 2*AXEMAN, 3*ARCHER | city fort:5 waves:3
siege:early 16axe>hillcity def0    | 16*AXEMAN | 2*AXEMAN, 3*ARCHER | hills city fort:5 waves:3
siege:early 16axe>hillwalls def0   | 16*AXEMAN | 2*AXEMAN, 3*ARCHER | hills walls fort:5 waves:3
siege:early 24axe>hillwalls def0   | 24*AXEMAN | 2*AXEMAN, 3*ARCHER | hills walls fort:5 waves:4
siege:early 16axe>walls def1       | 16*AXEMAN | 4*AXEMAN, 4*ARCHER | walls fort:5 waves:3
siege:early 24axe>walls def1       | 24*AXEMAN | 4*AXEMAN, 4*ARCHER | walls fort:5 waves:4
siege:mid 16champ>walls def0       | 16*CHAMPION | 2*CHAMPION, 3*LONGBOWMAN | walls fort:5 waves:3
siege:mid 16mix>walls def0         | 8*CHAMPION, 4*LONGBOWMAN, 4*HORSE_ARCHER | 2*CHAMPION, 3*LONGBOWMAN | walls fort:5 waves:3
siege:mid 16champ>hillwalls def0   | 16*CHAMPION | 2*CHAMPION, 3*LONGBOWMAN | hills walls fort:5 waves:3
siege:mid 16champ>walls def1       | 16*CHAMPION | 4*CHAMPION, 4*LONGBOWMAN | walls fort:5 waves:3
siege:late 16phalanx>walls def0    | 16*PHALANX | 2*PHALANX, 3*CROSSBOWMAN | walls fort:5 waves:3
siege:late 16knight>walls def0     | 16*KNIGHT | 2*PHALANX, 3*CROSSBOWMAN | walls fort:5 waves:3
siege:late 16phalanx>hillwalls def0| 16*PHALANX | 2*PHALANX, 3*CROSSBOWMAN | hills walls fort:5 waves:3
siege:cross mid16champ>early walls | 16*CHAMPION | 2*AXEMAN, 3*ARCHER | walls fort:5 waves:3
siege:cross late8ph>mid walls      | 8*PHALANX | 2*CHAMPION, 3*LONGBOWMAN | walls fort:5 waves:3
# --- Equal cost against 6 fortified Champions (720) on open ground: numbers vs quality.
cost:28war>6champ   | 28*WARRIOR | 6*CHAMPION | fort:5 waves:3
cost:12axe>6champ   | 12*AXEMAN | 6*CHAMPION | fort:5 waves:2
cost:6champ>6champ  | 6*CHAMPION | 6*CHAMPION | fort:5
cost:3phalanx>6champ| 3*PHALANX | 6*CHAMPION | fort:5
cost:6champ>28war   | 6*CHAMPION | 28*WARRIOR | fort:5
cost:6champ>12axe   | 6*CHAMPION | 12*AXEMAN | fort:5
cost:6champ>3phalanx| 6*CHAMPION | 3*PHALANX | fort:5
# --- Large units: the auto formation's centre lanes.
large:4eleph>champ8 | 4*WAR_ELEPHANT | 8*CHAMPION | open
large:2eleph>champ4 | 2*WAR_ELEPHANT | 4*CHAMPION | open
large:2irongolem+4champ>champ8 | 2*IRON_GOLEM, 4*CHAMPION | 8*CHAMPION | open
'''
write("probes.txt", PROBES.strip().splitlines())

COST = '''
eqA:19war   | 19*WARRIOR      | 6*AXEMAN | fort:5 waves:2
eqA:8axe    | 8*AXEMAN        | 6*AXEMAN | fort:5
eqA:8horse  | 8*HORSEMAN      | 6*AXEMAN | fort:5
eqA:8arch   | 8*ARCHER        | 6*AXEMAN | fort:5
eqA:4champ  | 4*CHAMPION      | 6*AXEMAN | fort:5
eqA:4chariot| 4*CHARIOT       | 6*AXEMAN | fort:5
eqA:4ha     | 4*HORSE_ARCHER  | 6*AXEMAN | fort:5
eqA:2phalanx| 2*PHALANX       | 6*AXEMAN | fort:5
eqA:2knight | 2*KNIGHT        | 6*AXEMAN | fort:5
eqD:19war   | 12*AXEMAN | 19*WARRIOR     | fort:5 waves:2
eqD:8axe    | 12*AXEMAN | 8*AXEMAN       | fort:5 waves:2
eqD:8horse  | 12*AXEMAN | 8*HORSEMAN     | fort:5 waves:2
eqD:8arch   | 12*AXEMAN | 8*ARCHER       | fort:5 waves:2
eqD:4champ  | 12*AXEMAN | 4*CHAMPION     | fort:5 waves:2
eqD:2phalanx| 12*AXEMAN | 2*PHALANX      | fort:5 waves:2
eqD:4ha     | 12*AXEMAN | 4*HORSE_ARCHER | fort:5 waves:2
v:1champ>2war | 1*CHAMPION | 2*WARRIOR | open
v:2war>1champ | 2*WARRIOR | 1*CHAMPION | open
v:1champ>3war | 1*CHAMPION | 3*WARRIOR | open
v:3war>1champ | 3*WARRIOR | 1*CHAMPION | open
v:1phalanx>4war | 1*PHALANX | 4*WARRIOR | open
v:4war>1phalanx | 4*WARRIOR | 1*PHALANX | open
v:1phalanx>8war | 1*PHALANX | 8*WARRIOR | open
v:8war>1phalanx | 8*WARRIOR | 1*PHALANX | open
'''
write("cost.txt", COST.strip().splitlines())
