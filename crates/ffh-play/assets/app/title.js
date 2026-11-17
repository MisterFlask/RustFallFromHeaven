// The title screen: the game's front page. It shows at boot while the client connects, stays when
// there is no game (Continue, New game, Load game, How to play, Options), fades out when a game is
// running, and comes back from the game menu's "Main menu" (then with Resume). Behind it, the HD-2D
// map (docs/HD2D.md) as a picture (`title/hd2d.webp`: a scene of a played game rendered by the
// HD-2D renderer, borders off; docs/HD2D.md §7), drifting slowly; prefers-reduced-motion keeps it
// still. A picture, not the live renderer: the front page needs no game and compiles no shaders.

import * as api from "./api.js";
import { S } from "./state.js";
import { h } from "./dom.js";
import { look, UI } from "./look.js";
import { saveTurn, whenText } from "./screens.js";

/** The backdrop's picture. */
const BACKDROP = new URL("./title/hd2d.webp", import.meta.url).href;

/** The backdrop: the picture, its drift (style.css `title-drift`) paused while the screen is hidden. */
class Backdrop {
  constructor(el) {
    this.el = el;
  }

  start() {
    this.el.classList.add("drifting");
  }

  stop() {
    this.el.classList.remove("drifting");
  }
}

// ---------------------------------------------------------------------------------------------
// The screen

let el = null;
let picture = null;
let actions = null;
/** "loading" (the client connects), "menu" (no game), "game" (opened over a running game). */
let mode = "loading";
let visible = false;

/** Callbacks of the menu (main.js): `newGame()`, `load()`, `keys()`, `options()`, `loadSave(name)`. */
export function initTitle(handlers) {
  actions = handlers;
  el = document.getElementById("title-screen");
  if (!el) return;
  const backdrop = h("div.title-backdrop", { "aria-hidden": "true" }, h("img.title-picture", { src: BACKDROP, alt: "", decoding: "async", draggable: "false" }));
  el.replaceChildren(
    backdrop,
    h("div.title-shade", { "aria-hidden": "true" }),
    h(
      "div.title-column",
      h("div.title-logo.pending", { role: "heading", "aria-level": "1" }, h("span.title-logo-text", "Fall from Heaven")),
      h("p.title-tagline", "Turn-based fantasy strategy: build a civilization, master the spheres of magic, and decide whether the world ends."),
      h("nav.title-menu", { "aria-label": "Main menu" }),
      h("p.title-status", { "aria-live": "polite" }),
    ),
    h(
      "footer.title-foot",
      h("span", "Based on Fall from Heaven 2 (0.41), the Civilization IV mod by Kael and the Fall from Heaven team, ported to Rust."),
      h("span.title-build"),
    ),
  );
  picture = new Backdrop(backdrop);
  // The logo waits a moment for the pixel art (titleArt), so the serif stand-in does not flash;
  // without the art it shows anyway.
  setTimeout(() => el.querySelector(".title-logo")?.classList.remove("pending"), 2500);
  el.addEventListener("keydown", (ev) => {
    if (ev.key === "Escape" && mode === "game" && !document.querySelector("#modals .backdrop")) {
      ev.stopPropagation();
      hideTitle();
    }
  });
  showTitle({ loading: true });
}

/** The logo in the Vigil title face once the art has loaded (a serif title before). */
export function titleArt() {
  const logo = el?.querySelector(".title-logo");
  if (!logo || !look.art || logo.querySelector(".ui-ptext")) return;
  const u = window.innerWidth < 700 ? 4 : window.innerWidth < 1400 ? 6 : 8;
  const span = UI.textSpan(look.art, "Fall from Heaven", { font: "vigil", u, dpr: look.dpr, cls: "title-logo-px" });
  logo.replaceChildren(span);
  logo.classList.remove("pending");
  renderMenu();
}

/** Whether the title screen is up. */
export function titleShown() {
  return visible;
}

/** Shows the title screen. `loading`: while the client connects (no menu yet); `game`: over a
 *  running game (Resume, Escape closes it). */
export function showTitle({ loading = false, game = false } = {}) {
  if (!el) return;
  mode = loading ? "loading" : game ? "game" : "menu";
  el.hidden = false;
  el.classList.remove("leaving");
  document.body.classList.add("title-up");
  visible = true;
  picture?.start();
  const build = el.querySelector(".title-build");
  if (build) build.textContent = globalThis.ffhWeb?.build ? `build ${globalThis.ffhWeb.build}` : "";
  renderMenu();
}

/** Fades the title screen out (a game is running, or Resume). */
export function hideTitle() {
  if (!el || !visible) return;
  visible = false;
  document.body.classList.remove("title-up");
  const reduce = window.matchMedia?.("(prefers-reduced-motion: reduce)").matches ?? false;
  if (reduce) {
    el.hidden = true;
    picture?.stop();
    return;
  }
  el.classList.add("leaving");
  setTimeout(() => {
    if (visible) return;
    el.hidden = true;
    el.classList.remove("leaving");
    picture?.stop();
  }, 450);
}

/** The phase changed (main.js): no game shows the menu, a game hides the screen unless the
 *  player opened it over the game. */
export function titlePhase(phase) {
  if (!el) return;
  if (phase === "no_game" || phase === "crashed") {
    if (!visible || mode !== "menu") showTitle();
    return;
  }
  if (visible && mode !== "game") hideTitle();
}

let savesAsked = false;
/** The newest save is known (or the lookup gave up): the menu can show. */
let savesKnown = false;
let latest = null;

/** The newest save (by time, else the newest autosave's turn) for Continue. */
async function findLatest() {
  try {
    const r = (await api.session("saves")).r;
    let best = null;
    for (const g of r.games ?? []) {
      for (const s of g.saves ?? []) {
        const turn = saveTurn(s.name);
        const rank = s.modified ?? turn ?? -1;
        if (!best || rank > best.rank) best = { name: `${g.game}/${s.name}`, game: g.game, save: s.name, rank, turn, modified: s.modified ?? null };
      }
    }
    return best;
  } catch {
    return null;
  }
}

function menuButton(label, opts, onClick) {
  return h(`button.title-item${opts.primary ? ".primary" : ""}`, { id: opts.id, title: opts.title ?? "", disabled: !!opts.disabled, onclick: onClick }, label, opts.sub ? h("small", opts.sub) : null);
}

function renderMenu() {
  if (!el) return;
  const nav = el.querySelector(".title-menu");
  const status = el.querySelector(".title-status");
  if (!nav) return;
  if (mode === "loading") {
    nav.replaceChildren();
    if (status) status.textContent = "Preparing the world"; // style.css animates the dots
    return;
  }
  if (status) status.textContent = "";
  // With no game the menu waits for the newest save (a moment), so Continue does not appear
  // above the other items after they are up and move them under the pointer.
  if (mode === "menu" && !savesAsked) {
    savesAsked = true;
    savesKnown = false;
    const done = (b) => {
      if (savesKnown) return;
      savesKnown = true;
      latest = b;
      renderMenu();
    };
    findLatest().then(done);
    setTimeout(() => done(null), 3000);
  }
  if (mode === "menu" && !savesKnown) {
    nav.replaceChildren();
    if (status) status.textContent = "Preparing the world";
    return;
  }
  const items = [];
  if (mode === "game") items.push(menuButton("Resume", { id: "title-resume", primary: true, title: "Back to the game [Esc]" }, () => hideTitle()));
  if (mode === "menu" && latest) {
    items.push(
      menuButton("Continue", { id: "title-continue", primary: true, sub: [latest.game, latest.turn !== null ? `turn ${latest.turn}` : latest.save, whenText(latest.modified)].filter(Boolean).join(" · "), title: `Load ${latest.name}` }, async (ev) => {
        const b = ev.currentTarget;
        b.disabled = true;
        try {
          await actions?.loadSave(latest.name);
        } finally {
          b.disabled = false;
        }
      }),
    );
  }
  items.push(menuButton("New game", { id: "title-new", primary: mode === "menu" && !latest, title: "Choose a civilization and a world" }, () => actions?.newGame()));
  items.push(menuButton("Load game", { id: "title-load", title: "Your saved games and autosaves" }, () => actions?.load()));
  items.push(menuButton("Options", { id: "title-options", title: "The map renderer and the player options" }, () => actions?.options()));
  items.push(menuButton("How to play", { id: "title-keys", title: "How to play, the keys, the rules" }, () => actions?.keys()));
  const had = nav.contains(document.activeElement);
  nav.replaceChildren(...items);
  // The first item takes the keys (Enter), unless a dialog over the screen has them.
  if ((had || document.activeElement === document.body) && !document.querySelector("#modals .backdrop")) items[0]?.focus({ preventScroll: true });
}

/** Forgets the newest save (a game was saved or started): Continue asks again next time. */
export function titleSavesChanged() {
  savesAsked = false;
  savesKnown = false;
  latest = null;
}

/** Whether a game exists (S.view): used by Resume. */
export function hasGame() {
  return !!S.view;
}
