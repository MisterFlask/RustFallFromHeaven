// The odds of exploring a lair or entering a goody hut (the plot query's and the Explore Lair
// action's `explore`, crates/ffh-play/src/view/explore.rs), as Civ4 markup: a short form for the
// map tooltip and a full table for the plot panel and the action's help. Pure (no DOM), so the
// node tests import it.

export const TIER_NAMES = { big_bad: "Big bad", bad: "Bad", neutral: "Neutral", good: "Good", big_good: "Big good" };
const TIER_COLORS = { big_bad: "COLOR_NEGATIVE_TEXT", bad: "COLOR_NEGATIVE_TEXT", neutral: "COLOR_WHITE", good: "COLOR_POSITIVE_TEXT", big_good: "COLOR_POSITIVE_TEXT" };

/** A chance in parts per million as a percentage: `12.5%`, `0.3%`, `<0.1%`, `0%`. */
export function pct(ppm) {
  if (!ppm) return "0%";
  const p = ppm / 10000;
  if (p < 0.1) return "<0.1%";
  if (p >= 99.95 && ppm < 1000000) return ">99.9%";
  return `${p >= 10 ? p.toFixed(0) : p.toFixed(1)}%`;
}

/** "Warrior, level 2" for the unit the odds are for (`dname`: a type's display name). */
function unitName(e, dname) {
  if (e.unit === null || e.unit === undefined) return null;
  const name = dname("UnitType", e.unit_type);
  return e.level !== undefined ? `${name}, level ${e.level}` : name;
}

/** The tier line: `Big bad 13% · Bad 30% · ...`. */
export function tierLine(tiers) {
  return tiers.map((t) => `[${TIER_COLORS[t.tier]}]${TIER_NAMES[t.tier]} ${pct(t.ppm)}[COLOR_REVERT]`).join(" · ");
}

/** The tooltip form: who, the tiers and what becomes of the lair; for a hut, each outcome. */
export function exploreShort(e, dname) {
  const who = unitName(e, dname);
  if (e.kind === "hut") {
    const head = `[BOLD]Goody hut[\\BOLD]${who ? ` (${who} enters)` : " (select a unit for its odds)"}`;
    if (!who) return head;
    const rows = e.outcomes.filter((o) => o.allowed).sort((a, b) => b.ppm - a.ppm).map((o) => `[ICON_BULLET]${pct(o.ppm)} ${o.label}`);
    if (e.nothing_ppm > 0) rows.push(`[ICON_BULLET]${pct(e.nothing_ppm)} Nothing`);
    return [head, ...rows].join("[NEWLINE]");
  }
  const head = `[BOLD]Explore Lair${e.epic ? " (epic)" : ""}[\\BOLD]${who ? ` (${who})` : " (level 1; select a unit)"}`;
  const lines = [head, tierLine(e.tiers)];
  if (e.lair_survives_ppm !== undefined) lines.push(`Lair remains: ${pct(e.lair_survives_ppm)}`);
  for (const o of e.others ?? []) lines.push(`[ICON_BULLET]then ${dname("UnitType", o.unit_type)}, level ${o.level}: ${tierLine(o.tiers)}`);
  return lines.join("[NEWLINE]");
}

/** The full table: each tier with its outcomes, chance and what happens to the lair. */
export function exploreFull(e, dname) {
  if (e.kind === "hut") {
    const who = unitName(e, dname);
    const lines = [`[BOLD]Goody hut[\\BOLD]${who ? ` (${who} enters)` : ""}`];
    lines.push(`${e.attempts} draws from ${e.slots_total} slots; a draw the unit cannot receive is drawn again.`);
    for (const o of e.outcomes) {
      const share = `${o.slots}/${e.slots_total}`;
      if (!who) lines.push(`[ICON_BULLET]${share} ${o.label}`);
      else if (o.allowed) lines.push(`[ICON_BULLET]${pct(o.ppm)} ${o.label} [COLOR_LIGHT_GREY](${share})[COLOR_REVERT]`);
      else lines.push(`[ICON_BULLET][COLOR_LIGHT_GREY]0% ${o.label} (${share}, not possible now)[COLOR_REVERT]`);
    }
    if (who && e.nothing_ppm > 0) lines.push(`[ICON_BULLET]${pct(e.nothing_ppm)} Nothing`);
    return lines.join("[NEWLINE]");
  }
  const lines = [exploreShort(e, dname)];
  if (!e.outcomes) {
    if (e.unit !== null && e.unit !== undefined && e.in_view === false) lines.push("[COLOR_LIGHT_GREY]The outcomes show while the plot is in view.[COLOR_REVERT]");
    return lines.join("[NEWLINE]");
  }
  lines.push(e.epic ? "Roll: 0-99 + unit level; big bad below 54, big good from 54." : "Roll: 0-99 + unit level; big bad below 14, bad below 44, neutral below 74, good below 94, big good from 94.");
  for (const t of e.tiers) {
    lines.push(`[${TIER_COLORS[t.tier]}][BOLD]${TIER_NAMES[t.tier]} ${pct(t.ppm)}[\\BOLD][COLOR_REVERT]`);
    for (const o of e.outcomes.filter((x) => x.tier === t.tier)) {
      const fate = o.replaces_lair ? "lair replaced" : o.destroy_pct === 0 ? "lair stays" : o.destroy_pct === 100 ? "lair destroyed" : `lair destroyed ${o.destroy_pct}%`;
      lines.push(`[ICON_BULLET]${pct(o.ppm)} ${o.label} [COLOR_LIGHT_GREY](${fate})[COLOR_REVERT]`);
    }
    if (t.tier === "big_bad" && e.big_bad) {
      lines.push(`[COLOR_LIGHT_GREY]Monster: one of ${e.big_bad.monsters.join(", ")}.[COLOR_REVERT]`);
      lines.push(`[COLOR_LIGHT_GREY]Henchmen: one of ${e.big_bad.henchmen.join(", ")}.[COLOR_REVERT]`);
      lines.push(`[COLOR_LIGHT_GREY]Promotion: one of ${e.big_bad.promotions.join(", ")}.[COLOR_REVERT]`);
    }
  }
  if (e.grace_ppm !== undefined && e.grace_ppm > 0) lines.push(`[COLOR_LIGHT_GREY]Grace period: ${pct(e.grace_ppm)} per check that the big results leave out their strongest entries.[COLOR_REVERT]`);
  return lines.join("[NEWLINE]");
}
