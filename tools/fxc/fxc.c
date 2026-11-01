// fxc.exe FILE.hlsl ENTRY PROFILE [FLAGS [ASM_OUT]]: compiles FILE with D3DCompile from the
// d3dcompiler_47.dll beside it (the one Chrome ships: ANGLE's Direct3D 11 backend compiles every
// WebGL shader with it) and prints "OK <seconds> s <n> instructions", or FAIL and the log.
// <n> is the disassembly's "Approximately <n> instruction slots used". Built and run by
// tools/fxc/measure.sh (mingw-w64, run under wine); docs/R3D.md §11.1, "Measuring".
#include <windows.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

typedef struct BlobVtbl {
  void *qi, *addref, *release;
  void *(__stdcall *GetBufferPointer)(void *);
  SIZE_T(__stdcall *GetBufferSize)(void *);
} BlobVtbl;
typedef struct { BlobVtbl *v; } Blob;
typedef HRESULT(WINAPI *PCompile)(const void *, SIZE_T, const char *, const void *, void *, const char *, const char *, UINT, UINT, Blob **, Blob **);
typedef HRESULT(WINAPI *PDisasm)(const void *, SIZE_T, UINT, const char *, Blob **);

int main(int argc, char **argv) {
  if (argc < 4) {
    fprintf(stderr, "usage: fxc.exe FILE.hlsl ENTRY PROFILE [FLAGS [ASM_OUT]]\n");
    return 2;
  }
  HMODULE h = LoadLibraryA("d3dcompiler_47.dll");
  if (!h) {
    fprintf(stderr, "d3dcompiler_47.dll not loaded (%lu)\n", GetLastError());
    return 3;
  }
  PCompile compile = (PCompile)GetProcAddress(h, "D3DCompile");
  PDisasm disasm = (PDisasm)GetProcAddress(h, "D3DDisassemble");
  FILE *f = fopen(argv[1], "rb");
  if (!f) return 4;
  fseek(f, 0, SEEK_END);
  long n = ftell(f);
  fseek(f, 0, SEEK_SET);
  char *src = malloc(n + 1);
  fread(src, 1, n, f);
  src[n] = 0;
  fclose(f);
  UINT flags = argc > 4 ? (UINT)strtoul(argv[4], 0, 0) : 0;
  Blob *code = 0, *err = 0;
  LARGE_INTEGER fr, t0, t1;
  QueryPerformanceFrequency(&fr);
  QueryPerformanceCounter(&t0);
  HRESULT hr = compile(src, n, argv[1], 0, 0, argv[2], argv[3], flags, 0, &code, &err);
  QueryPerformanceCounter(&t1);
  double s = (double)(t1.QuadPart - t0.QuadPart) / fr.QuadPart;
  if (FAILED(hr)) {
    printf("FAIL %.3f s hr=%08lx\n", s, hr);
    if (err) fwrite(err->v->GetBufferPointer(err), 1, err->v->GetBufferSize(err), stdout);
    return 1;
  }
  long ins = -1;
  Blob *d = 0;
  if (disasm && SUCCEEDED(disasm(code->v->GetBufferPointer(code), code->v->GetBufferSize(code), 0, 0, &d))) {
    const char *t = (const char *)d->v->GetBufferPointer(d);
    const char *p = strstr(t, "// Approximately ");
    if (p) ins = strtol(p + 17, 0, 10);
    if (argc > 5) {
      FILE *o = fopen(argv[5], "wb");
      if (o) {
        fwrite(t, 1, d->v->GetBufferSize(d), o);
        fclose(o);
      }
    }
  }
  printf("OK %.3f s %ld instructions\n", s, ins);
  return 0;
}
