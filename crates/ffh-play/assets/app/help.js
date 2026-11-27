// The help display: the rules explanations that screens leave out, so that a screen holds only the
// decision at hand. Its topics are listed at its left, the chosen one's article at its right. The
// top bar's ? (and the ? key) opens it at the keys; a dialog's (i) (modals.js `help`) opens it at
// that dialog's topic, over the dialog, which is there again when the help closes. Modules add
// their topics with `addHelp` (crusade.js, screens.js). A topic belongs to one game mode: the
// classic game's (no `mode`) or the Cataclysm mode's (`mode: "cataclysm"`, cataclysm_help.js), and
// the display lists those of the game that is open (the classic ones when no game is).

import { h } from "./dom.js";
import { openModal } from "./modals.js";
import { S } from "./state.js";

/** The topics in the order they are listed: `{id, group, title, body, order, mode}`, `body()`
 *  making the article's nodes; `order` (default 50) sorts them, ties in the order they were added. */
const topics = [];

/** The topics of the open game's mode. */
function shown() {
  const cataclysm = S.view?.mode === "cataclysm";
  return topics.filter((t) => (t.mode === "cataclysm") === cataclysm);
}

/** Adds topic `{id, group, title, body, order?}` (a second one with the same id replaces the
 *  first). */
export function addHelp(topic) {
  const i = topics.findIndex((t) => t.id === topic.id);
  if (i >= 0) topics[i] = topic;
  else topics.push(topic);
  topics.sort((a, b) => (a.order ?? 50) - (b.order ?? 50));
}

/** The title of topic `id` (null if there is none). */
export function helpTitle(id) {
  return topics.find((t) => t.id === id)?.title ?? null;
}

/** Opens the help at topic `id` (the first topic when there is no such topic). */
export function openHelp(id = null) {
  const list = shown();
  let current = list.find((t) => t.id === id) ?? list[0] ?? null;
  const nav = h("nav.help-topics", { "aria-label": "Topics" });
  const article = h("article.help-article");
  const draw = () => {
    const groups = [];
    for (const t of list) {
      let g = groups.find((x) => x.name === t.group);
      if (!g) groups.push((g = { name: t.group, items: [] }));
      g.items.push(t);
    }
    nav.replaceChildren(
      ...groups.map((g) =>
        h(
          "div.help-group",
          g.name ? h("div.help-group-name", g.name) : null,
          g.items.map((t) =>
            h(
              `button.help-topic${t === current ? ".on" : ""}`,
              {
                "data-topic": t.id,
                "aria-current": t === current ? "true" : null,
                onclick: () => {
                  current = t;
                  draw();
                  article.scrollTop = 0;
                },
              },
              t.title,
            ),
          ),
        ),
      ),
    );
    article.dataset.topic = current?.id ?? "";
    article.replaceChildren(...(current ? [h("h3", current.title), ...[current.body()].flat(Infinity).filter(Boolean)] : [h("p.dim", "No help yet.")]));
  };
  draw();
  return openModal({ title: "Help", tag: "help", wide: true, className: "help", body: [h("div.help-layout", nav, article)], buttons: [{ label: "Close", value: null }] });
}
