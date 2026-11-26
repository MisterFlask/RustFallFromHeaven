// In-memory file system for the static site's engine worker (docs/WEB.md). Used by engine.js
// and the node tests; no DOM, no WASI, no dependencies.
//
// Files and directories live in memory (`/data` from `data.pak.gz`, `/saves` synced with
// IndexedDB by engine.js's `SavesPersister`). `MemFS.generation` is bumped by every change and
// `MemFS.takeDirty()` returns the paths touched since the last call.

/** Host error codes shared with `crates/ffh-play/src/platform.rs` (`wasm::err`). */
export const ERR = Object.freeze({
  NOENT: -1,
  EXIST: -2,
  INVAL: -3,
  NOTEMPTY: -4,
  IO: -5,
});

const EMPTY = new Uint8Array(0);

/** A file system error carrying a host error code from `ERR`. */
export class FsError extends Error {
  constructor(code, msg) {
    super(msg ?? `MemFS error ${code}`);
    this.name = "FsError";
    this.code = code;
  }
}

function nowNs() {
  return BigInt(Date.now()) * 1000000n;
}

function fail(code) {
  throw new FsError(code);
}

/** Directories and files in memory. A file's bytes are `data[0, size)`; the bytes of `data` past
 *  `size` are always zero. Nodes know their parent and name (no hard or symbolic links), so a
 *  node's path can be reported for change tracking. */
export class MemFS {
  constructor() {
    this.nextIno = 1;
    /** Bumped by every change. */
    this.generation = 0;
    this.dirty = new Set();
    this.root = this.#node("dir");
    this.root.parent = this.root;
  }

  #node(kind) {
    const t = nowNs();
    const n = { kind, ino: this.nextIno++, parent: null, name: "", atim: t, mtim: t, ctim: t, gen: 0 };
    if (kind === "dir") {
      n.entries = new Map();
    } else {
      n.data = EMPTY;
      n.size = 0;
    }
    return n;
  }

  /** The absolute path of a node, or null when it is no longer in the tree. */
  pathOf(node) {
    const parts = [];
    let n = node;
    while (n !== this.root) {
      const p = n.parent;
      if (!p || p.entries.get(n.name) !== n) return null;
      parts.push(n.name);
      n = p;
    }
    return "/" + parts.reverse().join("/");
  }

  #changed(node, path = undefined) {
    this.generation++;
    const p = path === undefined ? this.pathOf(node) : path;
    if (p !== null) this.dirty.add(p);
  }

  /** The paths changed (written, created, removed, renamed from or to) since the last call. */
  takeDirty() {
    const d = this.dirty;
    this.dirty = new Set();
    return d;
  }

  /** Resolves `path` against directory `base` (an absolute path starts at the root): the parent
   *  directory, the last name and the node there (null when absent). Throws `FsError` for a
   *  missing or non-directory intermediate component. */
  resolve(base, path) {
    if (path.length === 0) fail(ERR.NOENT);
    if (path.includes("\0")) fail(ERR.INVAL);
    let dir = path.startsWith("/") ? this.root : base;
    if (dir.kind !== "dir") fail(ERR.INVAL);
    const comps = path.split("/").filter((c) => c !== "");
    const mustDir = path.endsWith("/");
    if (comps.length === 0) return { parent: null, name: null, node: dir, mustDir: true };
    for (let i = 0; i < comps.length - 1; i++) {
      const c = comps[i];
      if (c === ".") continue;
      if (c === "..") {
        dir = dir.parent;
        continue;
      }
      const child = dir.entries.get(c);
      if (!child) fail(ERR.NOENT);
      if (child.kind !== "dir") fail(ERR.INVAL);
      dir = child;
    }
    const last = comps[comps.length - 1];
    if (last === ".") return { parent: null, name: null, node: dir, mustDir: true };
    if (last === "..") return { parent: null, name: null, node: dir.parent, mustDir: true };
    const child = dir.entries.get(last);
    return { parent: dir, name: last, node: child ?? null, mustDir };
  }

  #link(dir, name, node) {
    dir.entries.delete(name);
    dir.entries.set(name, node);
    node.parent = dir;
    node.name = name;
    dir.mtim = dir.ctim = nowNs();
  }

  #unlink(dir, name) {
    dir.entries.delete(name);
    dir.mtim = dir.ctim = nowNs();
  }

  /** A new empty file `name` in `dir`. */
  createFile(dir, name) {
    const f = this.#node("file");
    this.#link(dir, name, f);
    this.#changed(f);
    return f;
  }

  /** A new directory `name` in `dir`. */
  createDir(dir, name) {
    const d = this.#node("dir");
    this.#link(dir, name, d);
    this.#changed(d);
    return d;
  }

  /** Creates a single directory (fails with `EXIST` if present, `NOENT` if parent missing). */
  mkdir(base, path) {
    const r = this.resolve(base, path);
    if (r.node || !r.parent) fail(ERR.EXIST);
    return this.createDir(r.parent, r.name);
  }

  /** Removes an empty directory. */
  rmdir(base, path) {
    const r = this.resolve(base, path);
    if (!r.node) fail(ERR.NOENT);
    if (r.node.kind !== "dir") fail(ERR.INVAL);
    if (!r.parent) fail(ERR.INVAL);
    if (r.node.entries.size) fail(ERR.NOTEMPTY);
    const p = this.pathOf(r.node);
    this.#unlink(r.parent, r.name);
    this.#changed(null, p);
  }

  /** Removes a regular file. */
  unlinkFile(base, path) {
    const r = this.resolve(base, path);
    if (!r.node) fail(ERR.NOENT);
    if (r.node.kind === "dir" || r.mustDir) fail(ERR.INVAL);
    const p = this.pathOf(r.node);
    this.#unlink(r.parent, r.name);
    this.#changed(null, p);
  }

  /** Renames `path1` to `path2` with POSIX `rename(2)` semantics (atomic replacement). */
  rename(base1, path1, base2, path2) {
    const src = this.resolve(base1, path1);
    if (!src.node) fail(ERR.NOENT);
    if (!src.parent) fail(ERR.INVAL);
    if (src.mustDir && src.node.kind !== "dir") fail(ERR.INVAL);
    const dst = this.resolve(base2, path2);
    if (!dst.parent) fail(ERR.INVAL);
    if (dst.node === src.node) return;
    if (src.node.kind === "dir") {
      for (let d = dst.parent; ; d = d.parent) {
        if (d === src.node) fail(ERR.INVAL);
        if (d === this.root) break;
      }
      if (dst.node) {
        if (dst.node.kind !== "dir") fail(ERR.INVAL);
        if (dst.node.entries.size) fail(ERR.NOTEMPTY);
      }
    } else {
      if ((dst.node && dst.node.kind === "dir") || dst.mustDir) fail(ERR.INVAL);
    }
    const from = this.pathOf(src.node);
    this.#unlink(src.parent, src.name);
    this.#link(dst.parent, dst.name, src.node);
    src.node.ctim = nowNs();
    this.#changed(null, from);
    this.#changed(src.node);
  }

  /** Sets a file's size (cut or zero-extended). */
  truncate(f, size) {
    if (f.kind !== "file") fail(ERR.INVAL);
    if (size < f.size) {
      f.data.fill(0, size, Math.min(f.size, f.data.length));
    } else if (size > f.data.length) {
      this.#grow(f, size);
    }
    f.size = size;
    this.#touch(f);
  }

  #grow(f, min) {
    const d = new Uint8Array(Math.max(min, f.data.length * 2, 64));
    d.set(f.data.subarray(0, f.size));
    f.data = d;
  }

  #touch(f) {
    f.gen++;
    f.mtim = f.ctim = nowNs();
    this.#changed(f);
  }

  /** Writes `bytes` at `pos` (extending the file); returns the count. */
  write(f, pos, bytes) {
    if (f.kind !== "file") fail(ERR.INVAL);
    const end = pos + bytes.length;
    if (end > f.data.length) this.#grow(f, end);
    f.data.set(bytes, pos);
    if (end > f.size) f.size = end;
    this.#touch(f);
    return bytes.length;
  }

  /** At most `len` bytes from `pos` (a view, not a copy). */
  read(f, pos, len) {
    if (f.kind !== "file") fail(ERR.INVAL);
    if (pos >= f.size) return EMPTY;
    return f.data.subarray(pos, Math.min(f.size, pos + len));
  }

  // --- Path helpers (absolute paths) ---

  /** The node at `path`, or null when absent / invalid. */
  lookup(path) {
    try {
      return this.resolve(this.root, path).node;
    } catch (e) {
      if (e instanceof FsError) return null;
      throw e;
    }
  }

  /** 0 = absent, 1 = file, 2 = directory. */
  stat(path) {
    const n = this.lookup(path);
    if (!n) return 0;
    return n.kind === "dir" ? 2 : 1;
  }

  /** Modification time in seconds since the Unix epoch, or -1 when absent. */
  mtimeSecs(path) {
    const n = this.lookup(path);
    return n ? Number(n.mtim / 1000000000n) : -1;
  }

  /** Direct entry names of directory `path`. */
  listDir(path) {
    const r = this.resolve(this.root, path);
    if (!r.node) fail(ERR.NOENT);
    if (r.node.kind !== "dir") fail(ERR.INVAL);
    return [...r.node.entries.keys()];
  }

  /** Creates `path` and any missing parent directories (`mkdir -p`); returns the directory. */
  mkdirp(path) {
    if (path.includes("\0")) fail(ERR.INVAL);
    let dir = this.root;
    for (const c of path.split("/").filter((x) => x !== "" && x !== ".")) {
      if (c === "..") {
        dir = dir.parent;
        continue;
      }
      const child = dir.entries.get(c);
      if (!child) dir = this.createDir(dir, c);
      else if (child.kind === "dir") dir = child;
      else fail(ERR.INVAL);
    }
    return dir;
  }

  /** Writes a whole file at `path`, creating parent directories (for pack mounting and store
   *  restoration). With `copy: false` the file takes `bytes` directly as its backing buffer. */
  writeFile(path, bytes, { copy = true, mtimeMs } = {}) {
    const i = path.lastIndexOf("/");
    const dir = this.mkdirp(path.slice(0, i));
    const name = path.slice(i + 1);
    if (!name) fail(ERR.INVAL);
    let f = dir.entries.get(name);
    if (f && f.kind !== "file") fail(ERR.INVAL);
    if (!f) f = this.createFile(dir, name);
    f.data = copy ? new Uint8Array(bytes) : bytes;
    f.size = bytes.length;
    this.#touch(f);
    if (mtimeMs !== undefined) f.mtim = BigInt(Math.round(mtimeMs)) * 1000000n;
    return f;
  }

  /** Writes or appends `bytes` to `path` with `std::fs::write` / `OpenOptions::append` semantics
   *  (parent directory must already exist). */
  writeHost(path, bytes, append = false) {
    const r = this.resolve(this.root, path);
    if (!r.parent || r.mustDir) fail(ERR.INVAL);
    if (r.node && r.node.kind !== "file") fail(ERR.INVAL);
    const f = r.node ?? this.createFile(r.parent, r.name);
    if (append) {
      if (bytes.length > 0) this.write(f, f.size, bytes);
    } else {
      f.data = new Uint8Array(bytes);
      f.size = bytes.length;
      this.#touch(f);
    }
    return f;
  }

  /** Reads a file's bytes (view) at `path`, throwing `FsError` if missing or a directory. */
  readHost(path) {
    const r = this.resolve(this.root, path);
    if (!r.node) fail(ERR.NOENT);
    if (r.node.kind !== "file" || r.mustDir) fail(ERR.INVAL);
    return r.node.data.subarray(0, r.node.size);
  }

  /** A copy of a file's bytes at `path`, or null when absent or a directory. */
  readFile(path) {
    const f = this.lookup(path);
    return f && f.kind === "file" ? f.data.slice(0, f.size) : null;
  }

  /** Removes a file or a directory tree at `path`; false when absent. */
  remove(path) {
    const r = this.resolve(this.root, path);
    if (!r.node || !r.parent) return false;
    const p = this.pathOf(r.node);
    this.#unlink(r.parent, r.name);
    this.#changed(null, p);
    return true;
  }

  /** Calls `fn(relPath, node)` for every node under the directory at `path` (parents first). */
  walk(path, fn) {
    const top = this.lookup(path);
    if (!top || top.kind !== "dir") return;
    const go = (dir, prefix) => {
      for (const [name, node] of dir.entries) {
        const rel = prefix ? `${prefix}/${name}` : name;
        fn(rel, node);
        if (node.kind === "dir") go(node, rel);
      }
    };
    go(top, "");
  }
}

/** Sleeps `ms` milliseconds using `Atomics.wait` on a scratch SharedArrayBuffer when available
 *  (no-op on non-isolated pages without SharedArrayBuffer). */
let sleepWord;
export function sleepMs(ms) {
  if (!(ms > 0)) return;
  if (sleepWord === undefined) {
    try {
      sleepWord = typeof SharedArrayBuffer === "function" ? new Int32Array(new SharedArrayBuffer(4)) : null;
    } catch {
      sleepWord = null;
    }
  }
  if (!sleepWord) return;
  try {
    Atomics.wait(sleepWord, 0, 0, ms);
  } catch {
    /* main thread or disallowed */
  }
}
