// Modal dialogs (design §5.1 "Popups and decisions"): a stack in which only the top dialog
// takes input, confirmations as promises, and the blocking decision modal.

import { h, $, errorText } from "./dom.js";
import { openHelp, helpTitle } from "./help.js";
import { render as markup } from "./markup.js";
import { emit } from "./state.js";
import { playerPortrait, PORTRAIT } from "./portraits.js";

const stack = [];

/** How a page's text is set (look.js: an illuminated initial once the art has loaded). */
let illuminate = (frag) => frag;

export function setIlluminate(f) {
  illuminate = f;
}

/** Whether any dialog is open (the map's hotkeys are off then). */
export function modalOpen() {
  return stack.length > 0;
}

/** The open dialogs' tags (`tag` option), top last. */
export function openTags() {
  return stack.map((m) => m.tag);
}

/** Opens a dialog. `body`: nodes; `buttons`: `[{label, primary?, danger?, disabled?, title?,
 *  value?, onClick?}]` (a button without `onClick` closes the dialog with its `value`; `danger`:
 *  an irreversible action, see below); `dismissible`:
 *  Escape and the close button close it with `null`; `tag`: a name (one dialog per tag: a
 *  second opening replaces the first); `wide`: a large panel; `help`: the help topic explaining
 *  the dialog's rules (help.js), kept out of its body: an (i) in the title bar opens the help
 *  display there, over the dialog. The first enabled primary button has the focus, so Enter
 *  presses it; but when a button is `danger` (declare war, delete, a once-a-game spell) the
 *  focus goes to the button that cancels (a button without `onClick` whose value is null or
 *  undefined; the last such one), else to the dialog itself, so Enter (also the unit-cycle key)
 *  cannot confirm it. Returns `{el, close(value), setBody(nodes), done}`; `done`
 *  resolves with the value it closed with. */
export function openModal({ title, body = [], buttons = [], dismissible = true, tag = null, wide = false, className = "", help = null }) {
  if (tag) closeTag(tag);
  let resolve;
  const done = new Promise((r) => (resolve = r));
  const bodyEl = h("div.modal-body");
  const btnRow = h("div.modal-buttons");
  const helpLabel = help ? `Help: ${helpTitle(help) ?? "how this works"}` : null;
  const helpBtn = help ? h("button.info", { type: "button", title: helpLabel, "aria-label": helpLabel, "data-help": help, onclick: () => openHelp(help) }, "i") : null;
  const head = h("div.modal-head", h("h2", title ?? ""), helpBtn, dismissible ? h("button.close", { title: "Close (Esc)", "aria-label": "Close", onclick: () => m.close(null) }, "×") : null);
  const box = h(`div.modal${wide ? ".wide" : ""}`, { role: "dialog", "aria-modal": "true", "aria-label": title ?? "", class: className, "data-tag": tag }, head, bodyEl, btnRow);
  const backdrop = h("div.backdrop", box);
  const m = {
    el: box,
    tag,
    dismissible,
    closed: false,
    done,
    close(value) {
      if (m.closed) return;
      m.closed = true;
      backdrop.remove();
      const i = stack.indexOf(m);
      if (i >= 0) stack.splice(i, 1);
      updateLayer();
      resolve(value);
      // The deferred popups wait while another dialog is open.
      if (stack.length === 0) setTimeout(() => emit("modals-closed"), 0);
    },
    setBody(...nodes) {
      bodyEl.replaceChildren(...nodes.flat().filter(Boolean));
    },
    setButtons(list) {
      btnRow.replaceChildren(
        ...list.map((b) =>
          h(
            `button${b.primary ? ".primary" : ""}`,
            { disabled: b.disabled, title: b.title, "data-value": b.value === undefined ? undefined : String(b.value), "data-danger": b.danger ? true : undefined, onclick: () => (b.onClick ? m.run(b.onClick) : m.close(b.value)) },
            b.label,
          ),
        ),
      );
    },
    /** Runs a button's `onClick(m)`; while the promise it returns is pending (a session
     *  request: start, load, save) the dialog's buttons are disabled and further clicks are
     *  ignored, so a double click sends one request (P5 review). */
    run(onClick) {
      if (m.busy) return;
      const r = onClick(m);
      if (!r || typeof r.then !== "function") return;
      m.busy = true;
      const btns = [...btnRow.querySelectorAll("button")];
      const was = btns.map((x) => x.disabled);
      btns.forEach((x) => (x.disabled = true));
      Promise.resolve(r)
        .catch(() => {})
        .then(() => {
          m.busy = false;
          if (!m.closed) btns.forEach((x, i) => (x.disabled = was[i]));
        });
    },
  };
  m.setBody(body);
  m.setButtons(buttons);
  stack.push(m);
  $("#modals").appendChild(backdrop);
  updateLayer();
  emit("modal-opened", tag);
  const danger = buttons.some((b) => b?.danger);
  let first;
  if (danger) {
    const cancels = [...btnRow.children].filter((el, i) => !buttons[i].onClick && !buttons[i].danger && (buttons[i].value === null || buttons[i].value === undefined));
    first = cancels[cancels.length - 1];
    if (!first) {
      box.tabIndex = -1;
      first = box;
    }
  } else first = box.querySelector("button.primary:not([disabled])") ?? box.querySelector("input, select, button:not(.close):not(.info)");
  first?.focus({ preventScroll: true });
  return m;
}

function updateLayer() {
  stack.forEach((m, i) => m.el.parentElement?.classList.toggle("under", i < stack.length - 1));
  document.body.classList.toggle("has-modal", stack.length > 0);
}

/** Closes the dialogs with tag `tag`. */
export function closeTag(tag) {
  for (const m of [...stack]) if (m.tag === tag) m.close(null);
}

/** Closes every dialog except those whose tag is listed. */
export function closeAll(keep = []) {
  for (const m of [...stack].reverse()) if (!keep.includes(m.tag)) m.close(null);
}

/** Escape closes the top dialog when it may be dismissed. */
export function onEscape() {
  const top = stack[stack.length - 1];
  if (top?.dismissible) {
    top.close(null);
    return true;
  }
  return false;
}

/** A question with buttons; resolves with the chosen button's `value`, or null. */
export function ask(title, text, choices, { tag = "ask" } = {}) {
  const body = (Array.isArray(text) ? text : [text]).map((t) => (t instanceof Node ? t : h("p", t)));
  return openModal({ title, body, buttons: choices, tag }).done;
}

// ---------------------------------------------------------------------------------------------
// The blocking decision (design §2.3 (b)): free tech, keep, raze or give back a conquered city,
// keep or disband a city that joined, city or settlement, take control of a new civilization,
// offer, religion. It cannot be dismissed: the game waits inside a rules call for the answer.

let decisionModal = null;
let decisionId = null;

/** The tooltip of decision option `o`: its rules (`tip`) and, for a help of several lines (a
 *  free tech's), the whole help; null when the button says it all. */
function optionTip(o) {
  const long = o.help?.includes("[NEWLINE]");
  if (!o.tip && !long) return null;
  return [h("div.tip-title", o.label), long ? h("div.tip-effects", markup(o.help)) : null, o.tip ? h("div.tip-hint", markup(o.tip)) : null];
}

/** Shows decision `d` (`status.decision`); `answer(option | null)` sends the answer (null:
 *  the advisor's pick) and resolves when it was accepted. Showing the same decision again does
 *  nothing; `null` closes it. */
export function showDecision(d, answer) {
  if (!d) {
    decisionModal?.close(null);
    decisionModal = null;
    decisionId = null;
    return;
  }
  if (decisionId === d.id && decisionModal && !decisionModal.closed) return;
  decisionModal?.close(null);
  decisionId = d.id;
  let m = null;
  const send = async (option) => {
    try {
      await answer(option);
      m?.close(option);
    } catch (e) {
      errorLine.textContent = errorText(e);
    }
  };
  const errorLine = h("p.error");
  const options = h(
    "div.options",
    d.options.map((o, i) =>
      h(
        "button.option",
        // A long help (a free tech's) shows its first line and the whole text as the tooltip; the
        // rules behind a choice (`tip`: what a settlement is) are in the tooltip alone.
        { disabled: !o.enabled, tip: optionTip(o), "data-option": i, onclick: () => send(i) },
        h("span", o.label),
        d.advisor === i ? h("span.badge", "advisor") : null,
        o.help ? h("small", markup(o.help.split("[NEWLINE]")[0])) : null,
      ),
    ),
  );
  // A deal offered, a civilization to take over: that player's leader beside the text.
  const other = d.context?.other_player;
  const face = other !== null && other !== undefined ? playerPortrait(other, { size: PORTRAIT.medium }) : null;
  const text = h("p.page-text", illuminate(markup(d.text)));
  m = openModal({
    title: d.title,
    body: [face ? h("div.decision-head", face, text) : text, options, errorLine],
    buttons: [{ label: "AI default", title: "The advisor's pick", onClick: () => send(null) }],
    dismissible: false,
    tag: "decision",
    className: "decision",
  });
  decisionModal = m;
}
