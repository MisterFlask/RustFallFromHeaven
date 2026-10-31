#!/usr/bin/env bash
# Direct3D's shader compiler on Linux: how long Chrome on Windows (ANGLE on Direct3D 11, its default)
# takes over each of the 3D renderer's shaders, and how long they are (docs/R3D.md §11.1,
# "Measuring"). The GLSL goes to SPIR-V (glslang), then to HLSL (SPIRV-Cross), then through
# D3DCompile from the d3dcompiler_47.dll Chrome ships (taken from Electron's Windows build),
# run under wine. ANGLE translates GLSL to HLSL itself; this path's instruction counts come within
# about 2% of the HLSL ANGLE makes (the HD-2D terrain of 2026-10-30: 6707 against 6621), and its
# times scale the same way. Compare old against new on the same machine, idle, alternating: absolute
# times vary with the CPU, and one run by 20% or so.
#
#   tools/fxc/measure.sh [--mode 3d|hd2d] [--all] [NAME...]   this tree's programs (dump.mjs)
#   tools/fxc/measure.sh --tree ROOT [...]                     another checkout's (an A/B's old side)
#   tools/fxc/measure.sh --dir DIR NAME...                     DIR/NAME.frag, as dump.mjs writes it
#
# The first run installs glslang-tools, spirv-cross, mingw-w64 and wine64 (apt) and downloads
# Electron's Windows zip for its d3dcompiler_47.dll, into $FXC_HOME (default ~/.cache/ffh-fxc).
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
home="${FXC_HOME:-$HOME/.cache/ffh-fxc}"
electron="${FXC_ELECTRON:-v33.2.0}"

setup() {
  if ! command -v glslangValidator >/dev/null || ! command -v spirv-cross >/dev/null || ! command -v x86_64-w64-mingw32-gcc >/dev/null || ! command -v wine >/dev/null; then
    DEBIAN_FRONTEND=noninteractive apt-get install -y -qq glslang-tools spirv-cross mingw-w64 wine64 >/dev/null
  fi
  mkdir -p "$home"
  if [ ! -f "$home/d3dcompiler_47.dll" ]; then
    curl -sSL -o "$home/electron.zip" "https://github.com/electron/electron/releases/download/$electron/electron-$electron-win32-x64.zip"
    (cd "$home" && unzip -o -q electron.zip d3dcompiler_47.dll && rm electron.zip)
  fi
  if [ ! -f "$home/fxc.exe" ] || [ "$root/tools/fxc/fxc.c" -nt "$home/fxc.exe" ]; then
    x86_64-w64-mingw32-gcc -O2 -o "$home/fxc.exe" "$root/tools/fxc/fxc.c"
  fi
}

dir=""
dump=()
names=()
while [ $# -gt 0 ]; do
  case "$1" in
    --dir) dir="$2"; shift 2 ;;
    --mode) dump+=(--mode "$2"); shift 2 ;;
    --all) dump+=(--all); shift ;;
    --tree) dump+=(--tree "$2"); shift 2 ;;
    *) names+=("$1"); shift ;;
  esac
done
setup
if [ -z "$dir" ]; then
  dir="$(mktemp -d)"
  listed=$(node "$root/tools/fxc/dump.mjs" "$dir" "${dump[@]}")
  [ ${#names[@]} -eq 0 ] && read -r -a names <<< "$listed"
fi
export WINEDEBUG=-all WINEPREFIX="$home/wine"
printf "%-20s %8s %13s\n" program seconds instructions
for p in "${names[@]}"; do
  f="$dir/$p"
  sed '1s/.*/#version 450/' "$f.frag" > "$f.450.frag"
  if ! glslangValidator -G --auto-map-locations --auto-map-bindings -S frag -o "$f.spv" "$f.450.frag" > "$f.glslang.log"; then
    echo "$p: glslang failed ($f.glslang.log)"
    continue
  fi
  spirv-cross --hlsl --shader-model 50 "$f.spv" --output "$f.hlsl"
  # FXC assigns the registers (ANGLE binds its own; explicit ones past s15 fail on ps_5_0).
  sed -i -E "s/ : register\([a-z][0-9]+\)//g" "$f.hlsl"
  r=$( (cd "$home" && wine ./fxc.exe "$f.hlsl" main ps_5_0 0 "$f.asm") 2>&1 | tr -d '\0' | grep -E "^(OK|FAIL)" | head -1 || true)
  read -r ok s _ n _ <<< "$r"
  if [ "${ok:-}" = OK ]; then printf "%-20s %8s %13s\n" "$p" "$s" "$n"; else echo "$p: $r"; fi
done
