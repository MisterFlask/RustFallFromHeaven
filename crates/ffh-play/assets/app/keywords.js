// Keywords and reminder text (docs/PLAY_DESIGN.md, "Keywords and reminder text"; docs/HUMAN.md):
// the one component every tooltip and help surface uses for them.
//
// - Rules text is what is unique to an entity (the body of its tooltip, as markup); a keyword in
//   it (`[KW=KEYWORD_X]word[\KW]`, markup.js) is drawn in the keyword style (`.kw`).
// - Reminder text is a keyword's rule, the same for every entity with it: never in the body, each
//   in a box of its own (`.reminder`: the keyword's name as its title, the rule below, smaller, on
//   its own background), beside the tooltip on a wide window and below it on a narrow one
//   (tips.js lays them out: `remindersNode` marks them as the tooltip's side boxes).
// - A list entry without a tooltip of its own shows its keywords as chips (`keywordChips`): a
//   chip's hover, focus or tap shows its reminder boxes; a tap on a chip does not do the entry's
//   action.
//
// The server sends the reminders (`reminders: [{key, name, text, with}]`) with the payload of each
// entity: this module decides none. The pure helpers are in keyword_view.js.

import { h } from "./dom.js";
import { showTip } from "./tips.js";
import { chipsOf, remindersFor } from "./keyword_view.js";

/** The reminder boxes of `reminders` (null for none): the tooltip's side boxes (tips.js). */
export function remindersNode(reminders) {
  const list = reminders ?? [];
  if (!list.length) return null;
  return h(
    "div.tip-reminders",
    { role: "note", "aria-label": "Reminder text" },
    list.map((r) => h("div.reminder", { "data-keyword": r.key }, h("div.reminder-name", r.name), h("div.reminder-text", r.text))),
  );
}

/** The keyword chips of an entry (null for none): one per reminder; each shows its reminder
 *  boxes on hover, focus or tap, and swallows the tap (the entry's action does not run). */
export function keywordChips(reminders) {
  const chips = chipsOf(reminders);
  if (!chips.length) return null;
  return h(
    "span.kw-chips",
    chips.map((c) => {
      const el = h("span.kw-chip", {
        role: "button",
        tabIndex: 0,
        "data-keyword": c.key,
        "aria-label": `${c.name}: ${remindersFor(reminders, c.key).map((r) => `${r.name}. ${r.text}`).join(" ")}`,
        tip: () => remindersNode(remindersFor(reminders, c.key)),
      });
      el.textContent = c.name;
      const open = (ev) => {
        ev.stopPropagation();
        ev.preventDefault();
        showTip(el);
      };
      el.addEventListener("click", open);
      // Inside a disabled button no click arrives: the pointer's release shows the boxes.
      el.addEventListener("pointerup", (ev) => ev.pointerType !== "mouse" && open(ev));
      el.addEventListener("keydown", (ev) => (ev.key === "Enter" || ev.key === " ") && open(ev));
      return el;
    }),
  );
}
