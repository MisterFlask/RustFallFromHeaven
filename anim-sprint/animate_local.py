"""Animates many units on the local model (FLUX 3 Image Klein + the pose LoRA, lib/klein.py / lib/perframe.py), one
after another: `lib/recipe.py --unit units/<key>.json --out final_<key>` (the frames, the automatic variant pick and
both output folders) and then `lib/preview.py` (the strips and sequences the pick by eye reads). Resumable: a unit
whose `final_<key>/sheet_death.png` exists is skipped unless --redo. One line per unit goes to work/animate_local.jsonl.

  C:/ComfyUI/python_embeded/python.exe animate_local.py --units warrior,lizardman      # some units
  C:/ComfyUI/python_embeded/python.exe animate_local.py --all                          # every unit file not done yet
      [--redo] [--no-preview] [--order biped-first]
ComfyUI must be running (local-image-gen skill: `generate.py status`).
"""
import argparse, json, subprocess, sys, time
from pathlib import Path

ROOT = Path(__file__).resolve().parent
PY = sys.executable


def run(cmd):
    p = subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True)
    return p.returncode, (p.stdout + p.stderr)[-2000:]


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--units", default="")
    ap.add_argument("--all", action="store_true")
    ap.add_argument("--redo", action="store_true")
    ap.add_argument("--no-preview", action="store_true")
    a = ap.parse_args()
    if a.all:
        keys = []
        for f in sorted((ROOT / "units").glob("*.json")):
            u = json.loads(f.read_text(encoding="utf-8"))
            if not u.get("legacy"):
                keys.append(f.stem)
    else:
        keys = [k for k in a.units.split(",") if k]
    log = ROOT / "work" / "animate_local.jsonl"
    log.parent.mkdir(exist_ok=True)
    todo = [k for k in keys if a.redo or not (ROOT / f"final_{k}" / "sheet_death.png").exists()]
    print(f"{len(todo)} of {len(keys)} units to animate", flush=True)
    for n, k in enumerate(todo, 1):
        t0 = time.time()
        code, out = run([PY, "lib/recipe.py", "--unit", f"units/{k}.json", "--out", f"final_{k}"])
        row = {"unit": k, "ok": code == 0, "seconds": round(time.time() - t0, 1)}
        if code:
            row["error"] = out[-600:]
        elif not a.no_preview:
            pc, pout = run([PY, "lib/preview.py", "--arm", f"final_{k}", "--root", f"final_{k}"])
            if pc:
                row["preview_error"] = pout[-300:]
        with log.open("a", encoding="utf-8") as fh:
            fh.write(json.dumps(row) + "\n")
        print(f"[{n}/{len(todo)}] {k}: {'ok' if row['ok'] else 'FAILED'} in {row['seconds']} s", flush=True)


if __name__ == "__main__":
    main()
