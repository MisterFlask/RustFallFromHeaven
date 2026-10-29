// The HD-2D mode's passes (docs/HD2D.md), plugged into the 3D renderer's pipeline as its `ext`
// (pipeline.js): the sprite cards in the sun's shadow map and the scene, the light shafts and a wide
// bloom added before the filmic curve, the region moods graded into the display colour, contact
// shadows, x-ray ghosts and light beams with the overlays, and the tilt-shift as the frame goes to
// the canvas. Each is timed like the pipeline's own passes (the panel, tools/r3d-perf.mjs).

import { Program, texture, framebuffer, FULLSCREEN_VS } from "../gl.js";
import { COMMON } from "../shaders.js";
import * as H from "./shaders.js";

export class Hd2dPasses {
  /** `pipeline`: the 3D pipeline (its targets, programs and timers); `sprites`: the SpriteLayer. */
  constructor(gl, pipeline, sprites) {
    this.gl = gl;
    this.pl = pipeline;
    this.sprites = sprites;
    const P = (name, vs, fs) => new Program(gl, name, vs, fs);
    const C = COMMON;
    this.p = {
      sprite: P("hd2d-sprite", C + H.SPRITE_VS, C + H.SPRITE_FS),
      spriteShadow: P("hd2d-sprite-shadow", C + H.SPRITE_VS, C + H.SPRITE_SHADOW_FS),
      contact: P("hd2d-contact", C + H.SPRITE_VS, C + H.CONTACT_FS),
      ghost: P("hd2d-ghost", C + H.SPRITE_VS, C + H.SPRITE_GHOST_FS),
      beam: P("hd2d-beam", C + H.SPRITE_VS, C + H.BEAM_FS),
      rays: P("hd2d-rays", FULLSCREEN_VS, C + H.RAYS_FS),
      bright: P("hd2d-bright", FULLSCREEN_VS, H.BRIGHT_FS),
      compose: P("hd2d-compose", FULLSCREEN_VS, H.COMPOSE_FS),
      mood: P("hd2d-mood", FULLSCREEN_VS, C + H.MOOD_FS),
      tilt: P("hd2d-tilt", FULLSCREEN_VS, H.TILT_FS),
    };
    /** Set by the map before each frame (hd2d/index.js): the config in force, the tilt-shift's focus
     *  (screen uv y) and strength, the far depth, the mood texture and the map's uniforms. */
    this.frame = null;
    this.w = this.h = 0;
  }

  resize(w, h) {
    this.disposeTargets();
    const gl = this.gl;
    this.w = w;
    this.h = h;
    const F = this.pl.hdr ? { internal: gl.RGBA16F, format: gl.RGBA, type: gl.HALF_FLOAT } : {};
    const hw = Math.max(1, w >> 1), hh = Math.max(1, h >> 1);
    const qw = Math.max(1, w >> 2), qh = Math.max(1, h >> 2);
    this.t = {
      rays: texture(gl, hw, hh, F),
      rays2: texture(gl, hw, hh, F),
      bright: texture(gl, hw, hh, F),
      bloom: texture(gl, qw, qh, F),
      bloom2: texture(gl, qw, qh, F),
      mood: texture(gl, w, h),
      tiltA: texture(gl, hw, hh),
      tiltA2: texture(gl, hw, hh),
      tiltB: texture(gl, qw, qh),
      tiltB2: texture(gl, qw, qh),
    };
    this.f = Object.fromEntries(Object.entries(this.t).map(([k, t]) => [k, framebuffer(gl, [t])]));
    this.size = { hw, hh, qw, qh };
  }

  disposeTargets() {
    const gl = this.gl;
    if (this.t) for (const x of Object.values(this.t)) gl.deleteTexture(x);
    if (this.f) for (const x of Object.values(this.f)) gl.deleteFramebuffer(x);
    this.t = this.f = null;
  }

  dispose() {
    this.disposeTargets();
    for (const p of Object.values(this.p)) p.dispose();
  }

  cardUniforms(U, scene, mode) {
    return { ...U, ...scene.mapUniforms(), uSprites: this.sprites.atlas.tex, uMode: mode, uLean: this.frame?.cfg.lean ?? 1 };
  }

  /** The figures' outlines in the sun's shadow map (the shadow pass's state: depth, offset). */
  shadow(SU, scene) {
    if (!this.frame) return;
    this.sprites.atlas.flush();
    const prog = this.p.spriteShadow.use().set(this.cardUniforms(SU, scene, 1));
    this.sprites.draw(prog, "casters", scene.copies);
  }

  /** The cards in the scene, after the props: depth-tested and written, cut out by their alpha. */
  scene(SU, scene) {
    if (!this.frame) return;
    const gl = this.gl;
    this.sprites.atlas.flush();
    gl.depthMask(true);
    gl.depthFunc(gl.LEQUAL);
    const prog = this.p.sprite.use().set(this.cardUniforms(SU, scene, 0));
    this.sprites.draw(prog, "static", scene.copies);
    this.sprites.draw(prog, "units", scene.copies);
    gl.depthMask(false);
  }

  /** The HDR passes and the compose (into the pipeline's ldrA): light shafts at half resolution,
   *  a wide bloom at a quarter, then occlusion, glow, bloom and shafts under the filmic curve. */
  post(fr, cfg, ao, glow) {
    const pl = this.pl, gl = this.gl;
    const U = fr.uniforms;
    const { hw, hh, qw, qh } = this.size;
    const fx = this.frame ?? { cfg };
    let rays = null;
    if ((cfg.rays ?? 0) > 0) {
      pl.timed("rays", () => {
        pl.pass(this.p.rays, this.f.rays, hw, hh, {
          ...U,
          uShadowMap: pl.shadowMap,
          uShadowOn: cfg.shadows ? 1 : 0,
          uCopyX: 0,
          uMapSize: fx.mapSize,
          uWrapX: fx.wrapX,
          uND: pl.t.nd,
          uTan: fr.tan,
          uInvView: fr.invView,
          uFarDepth: fx.farDepth ?? 40,
          uAspect: pl.w / Math.max(1, pl.h),
          uBeamZoom: fx.beamZoom ?? 1,
        });
        pl.pass(pl.p.blur, this.f.rays2, hw, hh, { uSrc: this.t.rays, uDir: [1 / hw, 0], uSigma: 1.5 });
        pl.pass(pl.p.blur, this.f.rays, hw, hh, { uSrc: this.t.rays2, uDir: [0, 1 / hh], uSigma: 1.5 });
      });
      rays = this.t.rays;
    }
    let bloom = null;
    if ((cfg.bloom ?? 0) > 0) {
      pl.timed("bloom", () => {
        pl.pass(this.p.bright, this.f.bright, hw, hh, { uColor: pl.t.color, uEmit: pl.t.emit, uThreshold: 0.95 });
        pl.pass(pl.p.blur, this.f.bloom2, qw, qh, { uSrc: this.t.bright, uDir: [1 / qw, 0], uSigma: 4 });
        pl.pass(pl.p.blur, this.f.bloom, qw, qh, { uSrc: this.t.bloom2, uDir: [0, 1 / qh], uSigma: 4 });
      });
      bloom = this.t.bloom;
    }
    pl.timed("compose", () => {
      pl.pass(this.p.compose, pl.f.ldrA, pl.w, pl.h, {
        uColor: pl.t.color,
        uAO: ao ?? pl.t.color,
        uGlow: glow ?? pl.t.emit,
        uBloom: bloom ?? pl.t.emit,
        uRays: rays ?? pl.t.emit,
        uAOAmount: ao ? cfg.ssao : 0,
        uGlowAmount: glow ? cfg.glow : 0,
        uBloomAmount: bloom ? cfg.bloom : 0,
        uRaysAmount: rays ? cfg.rays : 0,
        uExposure: fr.exposure,
      });
    });
    gl.bindFramebuffer(gl.FRAMEBUFFER, null);
  }

  /** The region moods and the land's palette, graded into the display colour: returns the graded
 *  frame (or null). */
  grade(fr, cfg) {
    const fx = this.frame;
    if (!fx?.mood || !((cfg.mood ?? 0) > 0 || (cfg.landGrade ?? 0) > 0)) return null;
    const pl = this.pl;
    pl.timed("mood", () => {
      pl.pass(this.p.mood, this.f.mood, pl.w, pl.h, {
        uSrc: pl.t.ldrA,
        uND: pl.t.nd,
        uEmit: pl.t.emit,
        uMood: fx.mood,
        uTan: fr.tan,
        uInvView: fr.invView,
        uMoodAmount: cfg.mood ?? 0,
        uLand: cfg.landGrade ?? 0,
        uTime: fr.uniforms.uTime,
        uMapSize: fx.mapSize,
        uWrapX: fx.wrapX,
        uFarDepth: fx.farDepth ?? 40,
      });
    });
    return this.t.mood;
  }

  /** Contact shadows under the figures, before the ground marks (blend on, depth LEQUAL). */
  underlay(U, scene) {
    const cfg = this.frame?.cfg;
    if (!cfg || !((cfg.contact ?? 0) > 0)) return;
    const gl = this.gl;
    gl.depthFunc(gl.LEQUAL);
    const prog = this.p.contact.use().set({ ...this.cardUniforms(U, scene, 2), uContact: cfg.contact });
    this.sprites.draw(prog, "casters", scene.copies);
  }

  /** Figures behind scenery, as hatched silhouettes (depth GREATER, set by the pipeline). */
  ghosts(U, scene) {
    if (!this.frame) return;
    const prog = this.p.ghost.use().set(this.cardUniforms(U, scene, 0));
    this.sprites.draw(prog, "units", scene.copies);
  }

  /** The light beams, added over the frame (premultiplied, as the particles). */
  additive(U, scene) {
    const cfg = this.frame?.cfg;
    if (!cfg || !((cfg.beams ?? 0) > 0)) return;
    const gl = this.gl;
    gl.depthFunc(gl.LEQUAL);
    gl.blendFunc(gl.ONE, gl.ONE_MINUS_SRC_ALPHA);
    const prog = this.p.beam.use().set({ ...this.cardUniforms(U, scene, 0), uBeams: cfg.beams });
    this.sprites.draw(prog, "beams", scene.copies);
  }

  /** The frame to the canvas through the tilt-shift; false when it is off (the pipeline blits). */
  present(fr, cfg, cw, ch) {
    const fx = this.frame;
    const tilt = (cfg.tiltShift ?? 0) * (fx?.tilt ?? 0);
    if (!(tilt > 0.01)) return false;
    const pl = this.pl, gl = this.gl;
    const { hw, hh, qw, qh } = this.size;
    pl.timed("tilt", () => {
      pl.pass(pl.p.blur, this.f.tiltA2, hw, hh, { uSrc: pl.t.final, uDir: [1 / hw, 0], uSigma: 1.2 });
      pl.pass(pl.p.blur, this.f.tiltA, hw, hh, { uSrc: this.t.tiltA2, uDir: [0, 1 / hh], uSigma: 1.2 });
      pl.pass(pl.p.blur, this.f.tiltB2, qw, qh, { uSrc: this.t.tiltA, uDir: [1 / qw, 0], uSigma: 1.6 });
      pl.pass(pl.p.blur, this.f.tiltB, qw, qh, { uSrc: this.t.tiltB2, uDir: [0, 1 / qh], uSigma: 1.6 });
      pl.pass(this.p.tilt, null, cw, ch, { uSrc: pl.t.final, uBlurA: this.t.tiltA, uBlurB: this.t.tiltB, uFocus: fx.focus ?? 0.5, uBand: 0.24, uTilt: Math.min(1, tilt) });
    });
    return true;
  }
}
