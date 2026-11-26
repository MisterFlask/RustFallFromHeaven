// memfs.js: in-memory file system semantics and host error codes shared with
// `crates/ffh-play/src/platform.rs` (zero WASI).
import test from "node:test";
import assert from "node:assert/strict";
import { MemFS, FsError, ERR } from "../memfs.js";

const enc = new TextEncoder();
const dec = new TextDecoder();

function expectErr(fn, code) {
  assert.throws(fn, (e) => e instanceof FsError && e.code === code);
}

test("MemFS stat, mkdir, mkdirp, listDir, writeHost, readHost, rename, remove", () => {
  const fs = new MemFS();
  assert.equal(fs.stat("/"), 2);
  assert.equal(fs.stat("/missing"), 0);
  assert.equal(fs.mtimeSecs("/missing"), -1);

  // mkdir vs mkdirp and AlreadyExists / NotFound semantics (unused_dir relies on EXIST).
  expectErr(() => fs.mkdir(fs.root, "/a/b"), ERR.NOENT);
  fs.mkdir(fs.root, "/saves");
  assert.equal(fs.stat("/saves"), 2);
  expectErr(() => fs.mkdir(fs.root, "/saves"), ERR.EXIST);
  fs.mkdirp("/saves/game-1/auto");
  assert.equal(fs.stat("/saves/game-1/auto"), 2);
  assert.deepEqual(fs.listDir("/saves"), ["game-1"]);
  assert.deepEqual(fs.listDir("/saves/game-1"), ["auto"]);
  assert.deepEqual(fs.listDir("/saves/game-1/auto"), []);

  // writeHost requires parent dir, supports overwrite and append.
  expectErr(() => fs.writeHost("/no-parent/x", enc.encode("a"), false), ERR.NOENT);
  fs.writeHost("/saves/game-1/journal.jsonl", enc.encode("line1\n"), false);
  assert.equal(fs.stat("/saves/game-1/journal.jsonl"), 1);
  assert.ok(fs.mtimeSecs("/saves/game-1/journal.jsonl") > 1_000_000_000);
  fs.writeHost("/saves/game-1/journal.jsonl", enc.encode("line2\n"), true);
  assert.equal(dec.decode(fs.readHost("/saves/game-1/journal.jsonl")), "line1\nline2\n");
  fs.writeHost("/saves/game-1/journal.jsonl", enc.encode("reset\n"), false);
  assert.equal(dec.decode(fs.readHost("/saves/game-1/journal.jsonl")), "reset\n");

  // Type errors: reading a dir, listing a file, writing over a dir, mkdirp over a file.
  expectErr(() => fs.readHost("/saves/game-1"), ERR.INVAL);
  expectErr(() => fs.readHost("/saves/game-1/missing"), ERR.NOENT);
  expectErr(() => fs.listDir("/saves/game-1/journal.jsonl"), ERR.INVAL);
  expectErr(() => fs.listDir("/saves/missing"), ERR.NOENT);
  expectErr(() => fs.writeHost("/saves/game-1", enc.encode("x"), false), ERR.INVAL);
  expectErr(() => fs.mkdirp("/saves/game-1/journal.jsonl/sub"), ERR.INVAL);

  // Atomic rename over existing file (save.rs write_atomic).
  fs.writeHost("/saves/game-1/a.ffhsave", enc.encode("old"), false);
  fs.writeHost("/saves/game-1/a.ffhsave.tmp", enc.encode("new-bytes"), false);
  const beforeGen = fs.generation;
  fs.rename(fs.root, "/saves/game-1/a.ffhsave.tmp", fs.root, "/saves/game-1/a.ffhsave");
  assert.ok(fs.generation > beforeGen);
  assert.equal(fs.stat("/saves/game-1/a.ffhsave.tmp"), 0);
  assert.equal(dec.decode(fs.readHost("/saves/game-1/a.ffhsave")), "new-bytes");

  // unlinkFile refuses directories; rmdir refuses non-empty; remove deletes whole tree.
  expectErr(() => fs.unlinkFile(fs.root, "/saves/game-1"), ERR.INVAL);
  expectErr(() => fs.rmdir(fs.root, "/saves/game-1"), ERR.NOTEMPTY);
  fs.unlinkFile(fs.root, "/saves/game-1/a.ffhsave");
  assert.equal(fs.stat("/saves/game-1/a.ffhsave"), 0);
  assert.equal(fs.remove("/saves/game-1"), true);
  assert.equal(fs.stat("/saves/game-1"), 0);
  assert.equal(fs.remove("/saves/game-1"), false);
});
