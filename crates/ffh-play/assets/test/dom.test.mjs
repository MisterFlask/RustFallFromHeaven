// node --test: the errors shown to the player (app/dom.js `errorText`): a plain sentence per
// error code, the rules' reason when it is plain, never JSON, type names or request details.
import test from "node:test";
import assert from "node:assert/strict";
import { errorText } from "../app/dom.js";

const quiet = (f) => {
  const warn = console.warn;
  console.warn = () => {};
  try {
    return f();
  } finally {
    console.warn = warn;
  }
};
const text = (code, detail = "") => quiet(() => errorText({ code, detail }));

test("errorText: one plain sentence per code, no repeated detail", () => {
  assert.equal(text("not_owner", "not yours"), "That is not yours.");
  assert.equal(text("gone", "no such object"), "That no longer exists.");
  assert.equal(text("not_your_turn", "the human does not have control"), "It is not your turn.");
  assert.equal(text("decision_pending", "decision 4 (keep_city) must be answered first"), "Answer the open decision first.");
  assert.equal(text("no_game", "no game is loaded: start a new game or load a save"), "No game is loaded.");
  assert.equal(text("busy", "later"), "The game is busy. Try again in a moment.");
  assert.equal(text("some_new_code", "x"), "The request failed.");
});

test("errorText: the rules' reason when it is plain", () => {
  assert.equal(text("cannot", "cannot declare war on Sabathiel"), "Cannot declare war on Sabathiel.");
  assert.equal(text("cannot", "the units are already there"), "The units are already there.");
  assert.equal(text("cannot", ""), "Not possible now.");
  // Engine identifiers, JSON, code and request details are not shown.
  assert.equal(text("cannot", "SPELL_RAISE_SKELETON is not possible"), "Not possible now.");
  assert.equal(text("invalid", "field `units`: empty selection"), "The game did not accept this order.");
  assert.equal(text("invalid", 'start one with {"k":"session"}'), "The game did not accept this order.");
});

test("errorText: a transport failure is a connection problem", () => {
  assert.equal(quiet(() => errorText(new Error("POST /api: HTTP 502"))), "Connection problem: the server does not answer.");
});
