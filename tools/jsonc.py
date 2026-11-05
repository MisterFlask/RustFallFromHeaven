"""The game data's format for Python tools: JSON with comments (data/json/*.jsonc).

    sys.path.insert(0, os.path.join(REPO, "tools")); import jsonc
    units = jsonc.table("units")          # the entries of data/json/units.jsonc
    defines = jsonc.load(path)            # any .jsonc file

JSON plus // and /* */ comments and trailing commas (crates/ffh-data/src/jsonc.rs).
"""
import json
import os

DATA = os.path.normpath(os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "data", "json"))


def loads(text):
    """Parses JSON with comments: drops the comments and trailing commas outside strings."""
    out, i, n, in_str = [], 0, len(text), False
    while i < n:
        c = text[i]
        if in_str:
            out.append(c)
            if c == "\\":
                out.append(text[i + 1])
                i += 1
            elif c == '"':
                in_str = False
        elif c == '"':
            in_str = True
            out.append(c)
        elif text.startswith("//", i):
            while i < n and text[i] != "\n":
                i += 1
            continue
        elif text.startswith("/*", i):
            i = text.index("*/", i) + 2
            continue
        elif c == ",":
            k = i + 1
            while k < n and text[k] in " \t\r\n":
                k += 1
            if k < n and text[k] in "]}":
                i += 1
                continue
            out.append(c)
        else:
            out.append(c)
        i += 1
    return json.loads("".join(out))


def load(path):
    with open(path, encoding="utf-8") as f:
        return loads(f.read())


def table(name, data_dir=DATA):
    """The entries of a table's file, `data/json/<name>.jsonc`."""
    return load(os.path.join(data_dir, name + ".jsonc"))
