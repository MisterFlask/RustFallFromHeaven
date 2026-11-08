// Leader portraits (tools/m7/leaders.py, docs/ISO.md "The interface"): a half-length HD key-art
// painting of each playable leader, `iso/leader_<key>.png` (384 px, drawn at 192 CSS px), and the
// square around its face, `iso/leader_<key>_face.png` (160 px), for the medium and small slots,
// where the whole painting would leave the face a few pixels high. Exported with the isometric
// sprites. Shown in every renderer: the foreign affairs screen, the trade screen, a deal offered to
// you, the take-control question, a vote's candidates and the new-game screen. A leader without a
// portrait shows none.

import { S, dname, tname, me } from "./state.js";
import { h } from "./dom.js";
import { SPRITES } from "./iso/sprites.js";

/** The sizes a portrait is drawn at, CSS px: the whole painting at large, its face below that. */
export const PORTRAIT = { large: 192, medium: 80, small: 40 };

/** The leaders of the civilizations born from the great-person cards (docs/CARDS.md), shown with
 *  another leader's portrait until they have their own. */
const PORTRAIT_ALIASES = {
  LEADER_AURELIAN: "valledia", LEADER_HALRIC: "sabathiel", LEADER_AINE: "amelanchier", LEADER_VESK: "hannah",
  LEADER_BURGOMASTER: "cardith",
};

/** The sprite key of leader type `type` ("LEADER_OS-GABELLA" -> "leader_os_gabella"; every random
 *  choice shares "leader_random"), or null when there is no such portrait. */
export function portraitKey(type) {
  if (!type) return null;
  const k = /^LEADER_RANDOM/.test(type)
    ? "random"
    : PORTRAIT_ALIASES[type] ?? type.replace(/^LEADER_/, "").toLowerCase().replace(/-/g, "_");
  return Object.hasOwn(SPRITES, `leader_${k}`) ? `leader_${k}` : null;
}

/** Leader `leader`'s portrait (a LeaderHeadType index or type name) `size` CSS px a side, or null. */
export function leaderPortrait(leader, { size = PORTRAIT.medium, title } = {}) {
  const type = typeof leader === "string" ? leader : leader !== null && leader !== undefined ? tname("LeaderHeadType", leader) : null;
  const key = portraitKey(type);
  if (!key) return null;
  const name = typeof leader === "string" ? null : dname("LeaderHeadType", leader);
  const file = size < PORTRAIT.large && Object.hasOwn(SPRITES, `${key}_face`) ? `${key}_face` : key;
  return h("img.leader-portrait", {
    src: new URL(`./iso/${file}.png`, import.meta.url).href,
    width: size,
    height: size,
    alt: title ?? name ?? "",
    title: title ?? name ?? null,
    draggable: false,
    "data-size": size <= PORTRAIT.small ? "small" : size <= PORTRAIT.medium ? "medium" : "large",
  });
}

/** Team `t`'s leader's portrait (its first player you know of, you included), or null. */
export function teamPortrait(t, opts) {
  const pl = [...S.players.values()].find((p) => p.team === t) ?? (me()?.team === t ? me() : null);
  return pl ? leaderPortrait(pl.leader, opts) : null;
}

/** Player `p`'s leader's portrait (a player you know of), or null. */
export function playerPortrait(p, opts) {
  const pl = S.players.get(p) ?? (me()?.player === p ? me() : null);
  return pl ? leaderPortrait(pl.leader, opts) : null;
}
