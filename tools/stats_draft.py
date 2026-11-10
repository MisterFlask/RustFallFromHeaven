#!/usr/bin/env python3
"""The unit stats content pass (docs/UNIT_STATS.md section 7): rules that turn every land unit's
derived stat sheet into a differentiated one, and the stat changes of the race and line promotions.

  python tools/stats_draft.py            writes docs/STATS_CONTENT.md: the rules and every unit's
                                         sheet, against its current one (applied 2026-11-05: the
                                         committed doc is the record against the day-one sheets)
  python tools/stats_draft.py --apply    also writes the drafted sheets and promotion changes into
                                         data/json/units.jsonc and promotions.jsonc

The rules are the data below: change a number and run it again.
"""
import os
import re
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import jsonc  # noqa: E402

REPO = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
STATS = ["hp", "attack", "defense", "magic", "magic_defense", "accuracy", "evasion", "crit", "speed"]
ABBR = {"hp": "HP", "attack": "ATK", "defense": "DEF", "magic": "MAG", "magic_defense": "MDEF", "accuracy": "ACC",
        "evasion": "EVA", "crit": "CRIT", "speed": "SPD"}

# ---- 1. By role: magic defence as a percentage of the unit's defence, accuracy, evasion, crit;
#      a caster's magic as a percentage of its attack. HP, attack, defence and speed stay as derived.
ROLE = {
    "COMBATROLE_FRONTLINE": dict(mdef=75, acc=90, eva=0, crit=10),
    "COMBATROLE_FLANKER": dict(mdef=90, acc=90, eva=15, crit=10),
    # Every assassin has Marksman free (+10 ACC, +20 CRIT): 95 ACC and 30 CRIT with it.
    "COMBATROLE_ASSASSIN": dict(mdef=100, acc=85, eva=20, crit=10),
    "COMBATROLE_ARCHER": dict(mdef=100, acc=95, eva=5, crit=10),
    "COMBATROLE_SIEGE": dict(mdef=75, acc=85, eva=0, crit=10),
    "COMBATROLE_ARCANE": dict(mdef=250, acc=90, eva=5, crit=5, mag=110),
    "COMBATROLE_DIVINE": dict(mdef=250, acc=90, eva=5, crit=5),
}

# ---- 2. By kind, after the role: recon skirmishers (not assassins), animals and beasts.
KIND = {
    "UNITCOMBAT_RECON": dict(mdef=90, acc=95, eva=10),
    "UNITCOMBAT_ANIMAL": dict(mdef=60, eva_add=5),
    "UNITCOMBAT_BEAST": dict(mdef=60, eva_add=5),
}
LARGE_EVASION = 0  # a large unit is easy to hit

# ---- 3. Single units.
UNIT = {
    "UNIT_NULLSTONE_GOLEM": dict(mdef=400),  # near immune to magic (FfH's lore)
    # This game's early national archer, above the opening's curve (docs/PORTING.md, "Balance").
    "UNIT_SKIRMISHER": dict(acc=100, eva=10, crit=15),
}

# ---- 4. Promotions: (stat, "flat" | "percent") -> value, set over what the promotion has.
PROMOTION = {
    # Lines
    "PROMOTION_COMBAT1": {("magic", "percent"): 10},
    "PROMOTION_COMBAT2": {("magic", "percent"): 10},
    "PROMOTION_COMBAT3": {("magic", "percent"): 10},
    "PROMOTION_COMBAT4": {("magic", "percent"): 10},
    "PROMOTION_COMBAT5": {("magic", "percent"): 10},
    "PROMOTION_DRILL1": {("accuracy", "flat"): 5, ("crit", "flat"): 10},
    "PROMOTION_DRILL2": {("accuracy", "flat"): 5, ("crit", "flat"): 10},
    "PROMOTION_DRILL3": {("accuracy", "flat"): 5, ("crit", "flat"): 10},
    "PROMOTION_DRILL4": {("accuracy", "flat"): 5, ("crit", "flat"): 10},
    # Mobility stays a movement pick (no battle stats: its +1 move is its worth).
    "PROMOTION_FLANKING1": {("evasion", "flat"): 8},
    "PROMOTION_FLANKING2": {("evasion", "flat"): 8},
    "PROMOTION_FLANKING3": {("evasion", "flat"): 8},
    "PROMOTION_MARKSMAN": {("crit", "flat"): 20, ("accuracy", "flat"): 10},
    "PROMOTION_MAGIC_RESISTANCE": {("magic_defense", "percent"): 75},
    "PROMOTION_HERO": {("accuracy", "flat"): 5, ("crit", "flat"): 5, ("magic_defense", "percent"): 20},
    # Races (a unit's own race promotion, or its civilization's)
    "PROMOTION_DWARF": {("defense", "percent"): 10, ("magic_defense", "percent"): 10, ("evasion", "flat"): -5},
    "PROMOTION_ELF": {("accuracy", "flat"): 5, ("evasion", "flat"): 5},
    "PROMOTION_DARK_ELF": {("accuracy", "flat"): 5, ("magic", "percent"): 10},
    "PROMOTION_ORC": {("hp", "percent"): 10, ("accuracy", "flat"): -5},
    "PROMOTION_UNDEAD": {("hp", "percent"): 10, ("magic_defense", "percent"): -20},
    "PROMOTION_DEMON": {("magic_defense", "percent"): 20, ("crit", "flat"): 5},
    "PROMOTION_ANGEL": {("magic_defense", "percent"): 20, ("accuracy", "flat"): 5},
    "PROMOTION_GOLEM": {("magic_defense", "percent"): 40, ("evasion", "flat"): -10},
    "PROMOTION_LIZARDMAN": {("evasion", "flat"): 5},
    "PROMOTION_CENTAUR": {("accuracy", "flat"): 5},
    "PROMOTION_DRAGON": {("magic_defense", "percent"): 30},
    "PROMOTION_FROSTLING": {("evasion", "flat"): 5},
    "PROMOTION_ELEMENTAL": {("magic_defense", "percent"): 20},
    "PROMOTION_ILLUSION": {("evasion", "flat"): 10},
}

# ---- 5. What each level-up promotion is worth (docs/STATS_CONTENT.md, "Level-up promotions"):
#      fighting power (damage dealt per action x hits survived) for the unit that takes it, in the
#      fights it is made for, against an even enemy. A row: (label, promotion, unit, where it
#      applies, situational bonus as (ATK %, DEF %) in its place, the side it is on today ("atk",
#      "def", "both"), first strikes (expected) and the first strike they start from, withdrawal
#      %, share of the damage taken that is spells, other worth not in the formulas).
ENEMY = dict(attack=40, defense=40, magic_defense=30, magic=40, accuracy=90, evasion=5, crit=10)
SPELL_SHARE = 0.15
FIRST_STRIKE = [0.06, 0.03, 0.02, 0.01, 0.01, 0.01]  # worth of the n-th first strike (speed +2)
WITHDRAW = 0.5  # withdrawal w is worth w x this
LEVEL_UP = [
    ("Combat (each level)", "PROMOTION_COMBAT1", "UNIT_AXEMAN", "always", (0, 0), "both", 0, 0, 0, None, 0),
    ("Combat (each level), a caster", "PROMOTION_COMBAT1", "UNIT_ADEPT", "always", (0, 0), "both", 0, 0, 0, None, 0),
    ("Drill I", "PROMOTION_DRILL1", "UNIT_AXEMAN", "always", (0, 0), "both", 0.5, 0, 0, None, 0.01),
    ("Drill II", "PROMOTION_DRILL2", "UNIT_AXEMAN", "always", (0, 0), "both", 1, 0, 0, None, 0.01),
    ("Drill III", "PROMOTION_DRILL3", "UNIT_AXEMAN", "always", (0, 0), "both", 1, 1, 0, None, 0.02),
    ("Drill IV", "PROMOTION_DRILL4", "UNIT_AXEMAN", "always", (0, 0), "both", 2, 2, 0, None, 0.04),
    ("Drill (each level), an archer", "PROMOTION_DRILL2", "UNIT_ARCHER", "always", (0, 0), "both", 1, 0, 0, None, 0.01),
    ("Flanking I, II", "PROMOTION_FLANKING1", "UNIT_HORSEMAN", "always", (0, 0), "both", 0, 0, 20, None, 0),
    ("Flanking III", "PROMOTION_FLANKING3", "UNIT_HORSEMAN", "always", (0, 0), "both", 0, 0, 10, None, 0.03),
    ("Marksman", "PROMOTION_MARKSMAN", "UNIT_ARCHER", "always", (0, 0), "both", 0, 0, 0, None, 0),
    ("Magic Resistance", "PROMOTION_MAGIC_RESISTANCE", "UNIT_AXEMAN", "against a mage-heavy army (spells half its damage)", (0, 0), "both", 0, 0, 0, 0.5, 0.10),
    ("Shock, Cover, Formation II, Scourge, Undead/Demon Slaying", None, "UNIT_AXEMAN", "against the unit type or race it names", (40, 40), "both", 0, 0, 0, None, 0),
    ("Formation I", "PROMOTION_FORMATION", "UNIT_AXEMAN", "against mounted units (and a Screen: anti-flank)", (40, 40), "both", 0, 0, 0, None, 0),
    ("Subdue Animals, Subdue Beasts", None, "UNIT_HUNTER", "against animals or beasts (and it captures them)", (25, 25), "both", 0, 0, 0, None, 0),
    ("City Garrison (each level)", None, "UNIT_ARCHER", "defending its city", (30, 30), "def", 0, 0, 0, None, 0),
    ("City Raider (each level)", None, "UNIT_AXEMAN", "attacking a city", (30, 30), "atk", 0, 0, 0, None, 0),
    ("Guerilla I", None, "UNIT_AXEMAN", "defending hills", (40, 40), "def", 0, 0, 0, None, 0),
    ("Woodsman (each level)", None, "UNIT_HUNTER", "in a forest or jungle", (30, 30), "def", 0, 0, 0, None, 0),
    ("Defensive", None, "UNIT_AXEMAN", "fortified (its fortification 25% -> 50%)", (25, 25), "def", 0, 0, 0, None, 0),
]
UTILITY = [
    "Mobility I-II (moves)", "Sentry I-II (sight)", "Amphibious", "Commando (enemy roads)", "Blitz",
    "Navigation I-II", "Bounty Hunter (gold)", "March (heals on the move)", "Command I-IV (captures)",
    "Fear", "Guardsman", "Keen Sight and Precision (archers' defensive strikes)", "Barrage I-III and Accuracy (siege)",
    "the elemental resistances (against that element)", "Cannibalize", "Stigmata", "Heroic Strength and Defense (heroes)",
    "Extension I-II and the spheres (casters)",
]


def hit_chance(acc, eva, extra):
    return max(5, min(100, acc - eva - extra)) / 100


def fighting_power(u, base, atk_pct=0, def_pct=0, share=SPELL_SHARE, shot=False, spell=False):
    e = ENEMY
    a = u["attack"] * (100 + atk_pct) / 100
    d = u["defense"] * (100 + def_pct) / 100
    if spell:
        dealt = 40 * u["magic"] / (u["magic"] + e["magic_defense"])
    else:
        dealt = hit_chance(u["accuracy"], e["evasion"], 10 if shot else 0) * (1 + u["crit"] / 200) * 40 * a / (a + e["defense"])
    phys = lambda x, dd: hit_chance(e["accuracy"], x["evasion"], 0) * (1 + e["crit"] / 200) * 40 * e["attack"] / (e["attack"] + dd)
    mag = lambda x: 40 * e["magic"] / (e["magic"] + max(1, x["magic_defense"]))
    taken = (1 - share) * phys(u, d) / phys(base, base["defense"]) + share * mag(u) / mag(base)
    return dealt * u["hp"] / taken


def with_changes(sheet, sc):
    v = dict(sheet)
    for stat, c in sc.items():
        v[stat] = v[stat] + c.get("flat", 0)
        v[stat] = v[stat] * (100 + c.get("percent", 0)) / 100
    v["evasion"] = max(0, v["evasion"])
    return v


def promotion_worth(row, drafted, pby, two_sided):
    label, promo, unit, where, (ap, dp), side, fs, fs_from, withdraw, share, extra = row
    base = drafted[unit]
    sc = draft_changes(pby[promo]) if promo else {}
    if not two_sided:
        ap, dp = (ap, 0) if side == "atk" else (0, dp) if side == "def" else (ap, dp)
    kw = dict(share=SPELL_SHARE if share is None else share, shot=unit == "UNIT_ARCHER", spell=unit == "UNIT_ADEPT")
    v = fighting_power(with_changes(base, sc), base, ap, dp, **kw) / fighting_power(base, base, **kw)
    whole = int(fs)
    v *= 1 + sum(FIRST_STRIKE[fs_from:fs_from + whole]) + (fs - whole) * FIRST_STRIKE[min(fs_from + whole, len(FIRST_STRIKE) - 1)]
    v *= 1 + withdraw * WITHDRAW / 100 + extra
    return v - 1


GROUPS = [
    ("UNITCOMBAT_MELEE", "Melee"), ("UNITCOMBAT_MOUNTED", "Mounted"), ("UNITCOMBAT_ARCHER", "Archery"),
    ("UNITCOMBAT_RECON", "Recon"), ("UNITCOMBAT_ADEPT", "Arcane"), ("UNITCOMBAT_DISCIPLE", "Disciples"),
    ("UNITCOMBAT_SIEGE", "Siege"), ("UNITCOMBAT_ANIMAL", "Animals"), ("UNITCOMBAT_BEAST", "Beasts"),
    ("UNITCOMBAT_NAVAL", "Naval"), ("NONE", "No unit combat (summons, workers, settlers, great people, equipment)"),
]


def text():
    t = {}
    for line in open(os.path.join(REPO, "data", "text", "en.tsv"), encoding="utf-8"):
        k, _, v = line.rstrip("\n").partition("\t")
        t[k] = v
    return t


# The rules' groups by the action of a unit's main row (its row with the most actions, the first
# such: the rows `engage_rows` in units.jsonc; there are no combat roles since 2026-11-06), named
# as the roles these rules were written for.
GROUP_OF_ACTION = {
    "STRIKE": "COMBATROLE_FRONTLINE",
    "CHARGE": "COMBATROLE_FLANKER",
    "STAB": "COMBATROLE_ASSASSIN",
    "SHOOT": "COMBATROLE_ARCHER",
    "BOMBARD": "COMBATROLE_SIEGE",
    "SPELL": "COMBATROLE_ARCANE",
    "HEAL": "COMBATROLE_DIVINE",
}


def role_of(u):
    """The rules' group of unit entry `u`: by the action of its main row."""
    rows = u.get("engage_rows") or []
    best = max(range(len(rows)), key=lambda i: (rows[i].get("count", 0), -i)) if rows else 0
    action = rows[best]["action"] if rows else "STRIKE"
    return GROUP_OF_ACTION.get(action, "COMBATROLE_FRONTLINE")


def draft_sheet(u, role):
    """The drafted sheet of unit entry `u` with type role `role`."""
    s = dict(u["stats"])
    rr = ROLE.get(role, {})
    kind = KIND.get(u.get("unit_combat")) if role != "COMBATROLE_ASSASSIN" else None
    rules = dict(rr)
    if kind:
        rules.update({k: v for k, v in kind.items() if k != "eva_add"})
    rules.update(UNIT.get(u["type"], {}))
    if "mdef" in rules:
        s["magic_defense"] = s["defense"] * rules["mdef"] // 100
    if "mag" in rules and s["magic"] > 0:
        s["magic"] = s["attack"] * rules["mag"] // 100
    for k, stat in (("acc", "accuracy"), ("eva", "evasion"), ("crit", "crit")):
        if k in rules:
            s[stat] = rules[k]
    if kind and "eva_add" in kind:
        s["evasion"] += kind["eva_add"]
    if u.get("large"):
        s["evasion"] = LARGE_EVASION
    if u["type"] == "UNIT_SENTINEL":
        return dict(u["stats"])  # the walls keep theirs (docs/SIEGES.md)
    return s


def draft_changes(p):
    sc = {k: dict(v) for k, v in p.get("stat_changes", {}).items()}
    for (stat, kind), v in PROMOTION.get(p["type"], {}).items():
        sc.setdefault(stat, {})[kind] = v
        if sc[stat].get(kind) == 0:
            del sc[stat][kind]
    return {k: v for k, v in sc.items() if v}


def cell(new, old):
    return str(new) if new == old else f"**{new}** ({old})"


def changes_text(sc):
    parts = []
    for stat in STATS:
        c = sc.get(stat, {})
        if c.get("flat"):
            parts.append(f"{c['flat']:+} {ABBR[stat]}")
        if c.get("percent"):
            parts.append(f"{c['percent']:+}% {ABBR[stat]}")
    return ", ".join(parts) or "none"


def expected(a, d, power=100):
    return 40 * a / (a + d) * power / 100


def main():
    apply = "--apply" in sys.argv
    tx = text()
    units = jsonc.table("units")
    promos = jsonc.table("promotions")
    pby = {p["type"]: p for p in promos}
    races = {p["type"] for p in promos if p.get("race")}
    land = [u for u in units if "stats" in u]
    rows, drafted = {}, {}
    for u in land:
        role = role_of(u)
        new = draft_sheet(u, role)
        drafted[u["type"]] = new
        uc = u.get("unit_combat", "NONE")
        name = tx.get(u.get("description", ""), u["type"])
        tags = sorted(p.replace("PROMOTION_", "").title().replace("_", " ") for p, on in u.get("free_promotions", {}).items() if on and p in races)
        if u.get("free_promotions", {}).get("PROMOTION_HERO", False):
            tags.append("hero")
        rows.setdefault(uc, []).append((u.get("tier", 0), name, role, new, u["stats"], ", ".join(tags), u.get("large", False)))

    out = []
    out.append("# Unit stats: the content pass\n")
    out.append("docs/UNIT_STATS.md section 7, applied 2026-11-05. Generated by `tools/stats_draft.py` from the")
    out.append("rules in its first lines, against the data as it is; `python tools/stats_draft.py --apply` writes")
    out.append("the rules into the data. A cell shows the rules' value, **bold** with the data's value in")
    out.append("brackets where they differ (the Before columns: the data's values).\n")
    out.append("## What the pass does\n")
    out.append("- **Hit points, attack, defence and speed stay as they are** (the balance the game has today).")
    out.append("  The pass only moves the new stats: magic defence, accuracy, evasion and crit, and casters'")
    out.append("  magic.")
    out.append("- **Armour against magic.** Heavy infantry keeps three quarters of its defence against spells;")
    out.append("  casters and priests get two and a half times theirs. Mages crack shield walls; archers and")
    out.append("  cavalry, whose hits magic defence does not stop, kill mages.")
    out.append("- **Cavalry and skirmishers dodge, assassins crit, archers aim.**")
    out.append("- **Races and lines are promotions**, so they follow every unit of the race, a civilization's")
    out.append("  default race included (a Khazad Axeman is a Dwarf). Races may differ in worth; the level-up")
    out.append("  promotions are balanced against each other: an always-on pick is worth about a Combat level,")
    out.append("  one that works in some fights about one and a half where it works (the table below).\n")
    out.append("### By role\n")
    out.append("| Role | MDEF (of its DEF) | ACC | EVA | CRIT | MAG (of its ATK) |")
    out.append("|---|---|---|---|---|---|")
    for r, v in ROLE.items():
        out.append(f"| {r.replace('COMBATROLE_', '').title()} | {v['mdef']}% | {v['acc']} | {v['eva']} | {v['crit']} | {str(v['mag']) + '%' if 'mag' in v else ''} |")
    out.append("\nThe current sheets: MDEF equal to DEF, ACC 90, EVA 0 (flankers 10, assassins 15), CRIT 10")
    out.append("(assassins 25), MAG equal to ATK.\n")
    out.append("### By kind and unit\n")
    out.append("- Every assassin has Marksman free (+10 ACC, +20 CRIT), so its sheet has 85 ACC and 10 CRIT:")
    out.append("  95 and 30 with it.")
    out.append("- Casters' magic is 110% of their attack, against the Combat line's smaller magic bonus.")
    out.append("- Recon skirmishers (recon units that are not assassins): MDEF 90% of DEF, ACC 95, EVA 10.")
    out.append("- Animals and beasts: MDEF 60% of DEF, EVA +5.")
    out.append(f"- Large units: EVA {LARGE_EVASION}.")
    out.append("- The Nullstone Golem: MDEF 400% of DEF. The Sentinels keep theirs.\n")
    out.append("### Promotions\n")
    out.append("| Promotion | Pass | Before |")
    out.append("|---|---|---|")
    for t in PROMOTION:
        p = pby.get(t)
        if p is None:
            out.append(f"| {t} | (no such promotion) | |")
            continue
        label = "Dark elven" if t == "PROMOTION_DARK_ELF" else tx.get(p.get("description", ""), t)
        out.append(f"| {label} | {changes_text(draft_changes(p))} | {changes_text(p.get('stat_changes', {}))} |")
    out.append("")
    out.append("### Level-up promotions: what each is worth\n")
    out.append("Fighting power (damage dealt per action times hits survived) a promotion adds to the unit that")
    out.append("takes it, in the fights it is made for, against an even enemy (spells 15% of the damage taken).")
    out.append("The aim: an always-on pick worth about a Combat level (+20%), a pick that works only in some fights")
    out.append("about one and a half (+30%) where it works. First strikes, withdrawal and the like are estimated")
    out.append("(a first strike +6%, the next ones less; withdrawal at half its chance).\n")
    out.append("A city, hills, forest or fortification bonus counts on both sides of a fight in its place (the")
    out.append("owner's rule, 2026-11-05): on the unit's ATK and DEF alike. The last column is its worth when")
    out.append("it counted on one side only (DEF when the unit defended, ATK when it attacked).\n")
    out.append("| Promotion | For | Applies | Worth | One side (before) |")
    out.append("|---|---|---|---|---|")
    for row in LEVEL_UP:
        label, promo, unit, where = row[:4]
        one = promotion_worth(row, drafted, pby, False)
        two = promotion_worth(row, drafted, pby, True)
        uname = tx.get(next(u for u in land if u["type"] == unit).get("description", ""), unit)
        out.append(f"| {label} | {uname} | {where} | {two * 100:+.0f}% | {'' if abs(two - one) < 0.005 else f'{one * 100:+.0f}%'} |")
    out.append("\nNot on this scale (their worth is outside battles, or narrow): " + "; ".join(UTILITY) + ".\n")
    out.append("### What it does in a battle\n")
    out.append("Expected damage of one hit (40 x A / (A + D); a spell times its power, the bolt 60%), and the")
    out.append("chance to hit (accuracy less evasion less the action's: a shot 10), for units without")
    out.append("promotions or a race:\n")
    sheet = lambda t: drafted[t]
    cur = {u["type"]: u["stats"] for u in land}
    out.append("| Matchup | Before | Pass |")
    out.append("|---|---|---|")
    pairs = [
        ("Adept's magic bolt on an Axeman", "UNIT_ADEPT", "UNIT_AXEMAN", "magic", "magic_defense", 60),
        ("Mage's magic bolt on a Champion", "UNIT_MAGE", "UNIT_CHAMPION", "magic", "magic_defense", 60),
        ("Mage's magic bolt on a Priest of the Order", "UNIT_MAGE", "UNIT_PRIEST_OF_THE_ORDER", "magic", "magic_defense", 60),
        ("Archer's shot on an Adept", "UNIT_ARCHER", "UNIT_ADEPT", "attack", "defense", 100),
        ("Horseman's charge on an Archer", "UNIT_HORSEMAN", "UNIT_ARCHER", "attack", "defense", 100),
    ]
    for label, h, t, a, d, power in pairs:
        if h in cur and t in cur:
            now = expected(cur[h][a], cur[t][d], power)
            new = expected(sheet(h)[a], sheet(t)[d], power)
            out.append(f"| {label} | {now:.1f} HP | {new:.1f} HP |")
    for label, h, t, extra in [("Archer shooting a Horseman", "UNIT_ARCHER", "UNIT_HORSEMAN", 10), ("Axeman striking an Assassin", "UNIT_AXEMAN", "UNIT_ASSASSIN", 0)]:
        if h in cur and t in cur:
            hit = lambda s1, s2: max(5, min(100, s1["accuracy"] - s2["evasion"] - extra))
            out.append(f"| {label}: chance to hit | {hit(cur[h], cur[t])}% | {hit(sheet(h), sheet(t))}% |")
    out.append("")
    out.append("## Open questions\n")
    out.append("- Combat is worth +11% to a caster (+20% to a fighter); casters' main picks are their spheres.")
    out.append("- Hit points, attack and defence are untouched: should tiers or races differ there too (for")
    out.append("  example more HP for heavy infantry, less for skirmishers)?")
    out.append("- Combat I-V give casters +10% magic (+5% on day one; +20% to a spell's A before unit stats).")
    out.append("- Heals stay `40 x 50%`: scale them with magic?")
    out.append("- Heroes get +5 ACC, +5 CRIT, +20% MDEF from the Hero promotion; give some their own sheets?\n")
    out.append("## The units\n")
    for uc, title in GROUPS:
        if uc not in rows:
            continue
        out.append(f"### {title}\n")
        out.append("| Unit | Tier | Role | HP | ATK | DEF | MAG | MDEF | ACC | EVA | CRIT | SPD | Race |")
        out.append("|---|---|---|---|---|---|---|---|---|---|---|---|---|")
        for tier, name, role, new, old, race, large in sorted(rows[uc], key=lambda r: (r[0], r[1])):
            cells = " | ".join(cell(new[s], old[s]) for s in STATS)
            out.append(f"| {name}{' (large)' if large else ''} | {tier} | {role.replace('COMBATROLE_', '').title()} | {cells} | {race} |")
        out.append("")
    with open(os.path.join(REPO, "docs", "STATS_CONTENT.md"), "w", encoding="utf-8", newline="\n") as f:
        f.write("\n".join(out))
    changed = sum(1 for u in land if drafted[u["type"]] != u["stats"])
    print(f"docs/STATS_CONTENT.md: {len(land)} units, {changed} with changes; {len(PROMOTION)} promotions")

    if apply:
        path = os.path.join(REPO, "data", "json", "units.jsonc")
        s = open(path, encoding="utf-8").read()
        for t, new in drafted.items():
            line = '"stats": {' + ", ".join(f'"{k}": {new[k]}' for k in STATS) + "}"
            s, n = re.subn(r'("type": "' + re.escape(t) + r'",\n\s*)"stats": \{[^}]*\}', lambda m: m.group(1) + line, s)
            assert n == 1, t
        open(path, "w", encoding="utf-8", newline="\n").write(s)
        path = os.path.join(REPO, "data", "json", "promotions.jsonc")
        s = open(path, encoding="utf-8").read()
        for t in PROMOTION:
            p = pby[t]
            sc = draft_changes(p)
            body = ", ".join(f'"{k}": {{' + ", ".join(f'"{kk}": {vv}' for kk, vv in sorted(sc[k].items(), key=lambda kv: kv[0] != "flat")) + "}" for k in STATS if k in sc)
            line = '"stat_changes": {' + body + "}"
            if "stat_changes" in p:
                s, n = re.subn(r'("type": "' + re.escape(t) + r'",\n\s*)"stat_changes": \{(?:[^{}]|\{[^{}]*\})*\}', lambda m: m.group(1) + line, s)
            else:
                s, n = re.subn(r'("type": "' + re.escape(t) + r'",\n(\s*))', lambda m: m.group(1) + line + ",\n" + m.group(2), s)
            assert n == 1, t
        open(path, "w", encoding="utf-8", newline="\n").write(s)
        print("applied to data/json/units.jsonc and promotions.jsonc")


if __name__ == "__main__":
    main()
