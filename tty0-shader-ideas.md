# tty0 Shader Ideas

This document captures follow-up shader directions for the `tty0` visualizer after the initial `containment_lattice` effect.

The goal is not "generic EDM visualizer." The goal is panel-local WebGL2 visuals that feel like:

- classified instrumentation
- hostile cyberpunk control surfaces
- machine-lit architecture
- quarantine, judgment, and redaction

All snippets below are GLSL-style pseudocode meant to guide implementation, not compile as-is.

## Shared Inputs

Assumed uniforms:

```glsl
uniform vec2 u_resolution;
uniform float u_time;

uniform float u_energy;
uniform float u_bass;
uniform float u_mid;
uniform float u_treble;
uniform float u_peak;
uniform float u_progress;

uniform float u_motion_rate;
uniform float u_ring_count;
uniform float u_lattice_density;

uniform sampler2D u_prev_frame;
```

Useful conventions:

- `u_energy`: overall brightness and feedback strength
- `u_bass`: large-scale motion, pulses, shock rings
- `u_mid`: rotation, structure deformation, line thickness
- `u_treble`: spark density, edge split, high-frequency detail
- `u_peak`: transient alert flash
- `u_progress`: slow color drift across the track

## Shared Helpers

```glsl
vec2 center_uv(vec2 frag, vec2 resolution) {
  vec2 uv = frag / resolution;
  vec2 p = uv * 2.0 - 1.0;
  p.x *= resolution.x / resolution.y;
  return p;
}

float hash21(vec2 p) {
  p = fract(p * vec2(123.34, 456.21));
  p += dot(p, p + 45.32);
  return fract(p.x * p.y);
}

mat2 rot(float a) {
  float c = cos(a);
  float s = sin(a);
  return mat2(c, -s, s, c);
}

float ring(vec2 p, float radius, float width) {
  float d = abs(length(p) - radius);
  return 1.0 - smoothstep(width, width + 0.004, d);
}

float line_mask(float d, float width) {
  return 1.0 - smoothstep(width, width + 0.003, abs(d));
}

vec3 tty0_palette(float t) {
  vec3 cyan = vec3(0.10, 0.95, 0.90);
  vec3 amber = vec3(1.00, 0.62, 0.18);
  vec3 red = vec3(0.96, 0.14, 0.16);
  vec3 base = mix(cyan, amber, clamp(t, 0.0, 1.0));
  return mix(base, red, smoothstep(0.75, 1.0, t));
}
```

## 1. Containment Lattice

Core feel:
- quarantine field
- hex lattice
- concentric containment rings
- scan wedge
- restrained panel-local feedback

Audio mapping:
- bass: ring pulse and radial zoom
- mids: lattice rotation and thickness
- treble: sparks and RGB edge split
- peak: warning flash
- progress: cyan to amber to red drift

Example scene composition:

```glsl
vec3 containment_lattice(vec2 frag) {
  vec2 p = center_uv(frag, u_resolution);
  p *= rot(u_time * 0.18 * u_motion_rate + u_mid * 0.35);

  float r = length(p);
  float angle = atan(p.y, p.x);

  float zoom = 1.0 + u_bass * 0.08;
  vec2 q = p * zoom;

  float density = mix(5.0, 16.0, clamp(u_lattice_density / 12.0, 0.0, 1.0));
  vec2 g = q * density;
  vec2 cell = abs(fract(g) - 0.5);
  float lattice = 1.0 - smoothstep(0.10, 0.16 + u_mid * 0.03, min(cell.x, cell.y));

  float rings = 0.0;
  for (int i = 0; i < 8; i++) {
    float fi = float(i);
    float radius = 0.16 + fi * 0.11 + u_bass * 0.045 * sin(u_time * 3.0 + fi);
    rings += ring(p, radius, 0.008 + u_mid * 0.006);
  }

  float wedge = smoothstep(0.16, 0.0, abs(angle - sin(u_time * u_motion_rate) * 2.4));
  wedge *= 0.25 + u_energy * 0.7;

  float spark_grid = floor((p.x + 1.0) * 22.0) + floor((p.y + 1.0) * 18.0) * 31.0;
  float spark = step(0.992 - u_treble * 0.05, hash21(vec2(spark_grid, floor(u_time * 18.0))));

  vec2 prev_uv = frag / u_resolution;
  prev_uv = (prev_uv - 0.5) * (1.0 - u_bass * 0.006) + 0.5;
  vec3 prev = texture(u_prev_frame, prev_uv).rgb * (0.965 + u_energy * 0.02);

  vec3 cyan = vec3(0.08, 0.90, 0.88) * lattice;
  vec3 amber = vec3(1.00, 0.58, 0.18) * rings * 0.75;
  vec3 red = vec3(0.92, 0.16, 0.18) * wedge * (0.35 + u_peak * 0.65);

  vec3 current = cyan + amber + red + spark * vec3(0.8, 1.0, 1.0) * u_treble;
  current *= 0.35 + u_energy * 0.9;

  float flash = smoothstep(0.7, 1.0, u_peak);
  current += flash * vec3(0.55, 0.10, 0.10);

  return max(prev * 0.88, current);
}
```

## 2. Quarantine Prism

Core feel:
- faceted triangular prisms
- classified optics
- cold, expensive, dangerous

Audio mapping:
- bass: shell expansion
- mids: facet rotation
- treble: chromatic edge split
- peak: fracture flash

Pseudo-structure:

```glsl
vec3 quarantine_prism(vec2 frag) {
  vec2 p = center_uv(frag, u_resolution);
  p *= rot(u_time * 0.22 + u_mid * 0.5);

  float a = atan(p.y, p.x);
  float sector = 6.28318 / 3.0;
  float folded = abs(fract(a / sector - 0.5) - 0.5) * sector;
  vec2 prism_uv = vec2(cos(folded), sin(folded)) * length(p);

  float shell = ring(prism_uv, 0.28 + u_bass * 0.10, 0.012);
  float inner = ring(prism_uv, 0.52 + u_bass * 0.08, 0.008);
  float facets = line_mask(prism_uv.x * 1.4 + prism_uv.y * 0.2, 0.018 + u_mid * 0.01);

  float crack = smoothstep(0.85, 1.0, u_peak) * step(0.94, hash21(floor(prism_uv * 18.0)));

  vec3 col = vec3(0.0);
  col += vec3(0.05, 0.85, 0.95) * shell;
  col += vec3(1.0, 0.65, 0.18) * inner;
  col += vec3(0.55, 0.95, 1.0) * facets * (0.4 + u_energy * 0.6);
  col += vec3(1.0, 0.18, 0.18) * crack;
  return col;
}
```

## 3. Signal Cathedral

Core feel:
- machine chapel
- vertical beam arrays
- vaulted arcs
- sacred but synthetic

Audio mapping:
- bass: arch pulses
- mids: beam sway
- treble: filament shimmer
- peak: halo overexposure

Pseudo-structure:

```glsl
vec3 signal_cathedral(vec2 frag) {
  vec2 p = center_uv(frag, u_resolution);

  float beam_x = abs(sin((p.x + u_time * 0.05 * u_motion_rate) * 18.0));
  float beams = 1.0 - smoothstep(0.10, 0.22 - u_mid * 0.04, beam_x);

  float arch0 = ring(vec2(p.x, p.y + 0.35), 0.62 + u_bass * 0.05, 0.010);
  float arch1 = ring(vec2(p.x, p.y + 0.12), 0.40 + u_bass * 0.04, 0.008);

  float filament = step(0.986 - u_treble * 0.04, hash21(floor(p * 40.0 + u_time * 14.0)));
  float halo = smoothstep(0.75, 1.0, u_peak) * ring(p, 0.18, 0.030);

  vec3 col = vec3(0.0);
  col += vec3(0.06, 0.72, 0.88) * beams * (0.2 + u_energy * 0.6);
  col += vec3(1.0, 0.58, 0.22) * (arch0 + arch1);
  col += vec3(0.8, 1.0, 1.0) * filament * u_treble;
  col += vec3(1.0, 0.85, 0.7) * halo;
  return col;
}
```

## 4. Relay Spindle

Core feel:
- transmission relay under load
- rotating spokes
- concentric machine rings
- stable, readable, mechanical

Audio mapping:
- bass: ring impacts
- mids: spoke wobble and rotation
- treble: spark chatter
- energy: glow and line persistence

Pseudo-structure:

```glsl
vec3 relay_spindle(vec2 frag) {
  vec2 p = center_uv(frag, u_resolution);
  float t = u_time * (0.35 + u_mid * 0.4) * u_motion_rate;
  p *= rot(t);

  float r = length(p);
  float a = atan(p.y, p.x);

  float outer = ring(p, 0.70 + u_bass * 0.04, 0.008);
  float mid = ring(p, 0.42 + u_bass * 0.03, 0.012);
  float core = ring(p, 0.18, 0.020);

  float spokes = 1.0 - smoothstep(0.02, 0.04 + u_mid * 0.02, abs(sin(a * 8.0 + r * 12.0)));
  spokes *= smoothstep(0.85, 0.08, r);

  float chatter = step(0.990 - u_treble * 0.04, hash21(floor(p * 36.0 + t * 10.0)));

  vec3 col = vec3(0.0);
  col += vec3(0.08, 0.90, 0.85) * (outer + core);
  col += vec3(1.0, 0.60, 0.20) * mid;
  col += vec3(0.65, 0.95, 1.0) * spokes * (0.35 + u_energy * 0.55);
  col += vec3(1.0, 0.25, 0.18) * chatter * u_treble;
  return col;
}
```

## 5. Redaction Grid

Core feel:
- censorship machinery
- shutters, masking bars, hidden cells
- archive hostility

Audio mapping:
- bass: large shutter slams
- mids: grid offset and alignment drift
- treble: checksum chatter
- peak: red denial bars

Pseudo-structure:

```glsl
vec3 redaction_grid(vec2 frag) {
  vec2 p = frag / u_resolution;
  vec2 grid = floor((p + vec2(u_mid * 0.03, 0.0)) * vec2(18.0, 14.0));
  vec2 local = fract((p + vec2(u_mid * 0.03, 0.0)) * vec2(18.0, 14.0)) - 0.5;

  float border = 1.0 - smoothstep(0.10, 0.16, min(0.5 - abs(local.x), 0.5 - abs(local.y)));
  float redact = step(0.72 - u_bass * 0.20, hash21(grid * 1.3 + floor(u_time * 2.0)));
  float chatter = step(0.985 - u_treble * 0.05, hash21(grid * 8.0 + floor(u_time * 24.0)));
  float deny = smoothstep(0.72, 1.0, u_peak) * step(0.92, sin(p.y * 140.0 + u_time * 30.0));

  vec3 base = vec3(0.03, 0.08, 0.09);
  vec3 grid_col = vec3(0.08, 0.88, 0.84) * border;
  vec3 shut = vec3(0.0) * redact;
  vec3 warn = vec3(0.95, 0.15, 0.16) * deny;
  vec3 noise = vec3(1.0, 0.65, 0.2) * chatter * 0.8;

  return base + max(grid_col * (1.0 - redact), shut) + warn + noise;
}
```

## 6. Event Horizon Lattice

Core feel:
- everything bends toward a central sink
- analytical dread
- high contrast, minimal decoration

Audio mapping:
- bass: inward pull
- mids: lattice twist
- treble: edge shimmer
- peak: expanding warning ring

Pseudo-structure:

```glsl
vec3 event_horizon_lattice(vec2 frag) {
  vec2 p = center_uv(frag, u_resolution);
  float r = length(p);
  float pull = 0.12 + u_bass * 0.18;
  vec2 q = p * (1.0 + pull / max(r, 0.16));
  q *= rot(u_mid * 0.4 + r * 0.8);

  vec2 g = q * (7.0 + u_lattice_density);
  vec2 c = abs(fract(g) - 0.5);
  float lattice = 1.0 - smoothstep(0.12, 0.18, min(c.x, c.y));

  float warning = ring(p, 0.34 + u_peak * 0.12, 0.015);
  float edge = smoothstep(0.94, 1.0, sin((q.x + q.y) * 42.0 + u_time * 8.0)) * u_treble;

  vec3 col = vec3(0.0);
  col += vec3(0.07, 0.82, 0.90) * lattice * (0.3 + u_energy * 0.5);
  col += vec3(0.95, 0.18, 0.16) * warning;
  col += vec3(0.8, 1.0, 1.0) * edge * 0.5;
  col *= smoothstep(1.05, 0.06, r);
  return col;
}
```

## 7. Neon Bastion

Core feel:
- shield walls
- defensive architecture
- overt cyberpunk

Audio mapping:
- bass: shield impact ripples
- mids: wall shifts
- treble: corner sparks
- peak: breach flash

Pseudo-structure:

```glsl
vec3 neon_bastion(vec2 frag) {
  vec2 p = center_uv(frag, u_resolution);
  p.x = abs(p.x);

  float wall0 = line_mask(p.x - (0.22 + u_bass * 0.04), 0.015);
  float wall1 = line_mask(p.x - (0.54 + u_mid * 0.03), 0.010);
  float brace = line_mask(p.y + p.x * 0.9 - 0.3, 0.018);

  float corner = step(0.992 - u_treble * 0.05, hash21(floor(p * 34.0 + u_time * 16.0)));
  float breach = smoothstep(0.76, 1.0, u_peak) * ring(p, 0.62, 0.025);

  vec3 col = vec3(0.0);
  col += vec3(0.05, 0.88, 0.95) * wall0;
  col += vec3(1.0, 0.58, 0.18) * wall1;
  col += vec3(0.60, 0.95, 1.0) * brace * (0.4 + u_energy * 0.5);
  col += vec3(1.0, 0.2, 0.16) * breach;
  col += vec3(1.0, 0.8, 0.35) * corner * u_treble;
  return col;
}
```

## 8. Judgment Engine

Core feel:
- classifier logic
- threshold bars
- decision geometry
- inhuman authority

Audio mapping:
- bass: threshold pulses
- mids: gate shifts
- treble: diagnostic chatter
- peak: denial flash

Pseudo-structure:

```glsl
vec3 judgment_engine(vec2 frag) {
  vec2 p = center_uv(frag, u_resolution);

  float threshold = smoothstep(-0.03, 0.03, p.x + sin(u_time * 0.8) * 0.1 + u_bass * 0.08);
  float gate_a = line_mask(p.y - 0.28 - u_mid * 0.06, 0.014);
  float gate_b = line_mask(p.y + 0.28 + u_mid * 0.06, 0.014);
  float ticks = 1.0 - smoothstep(0.06, 0.12, abs(fract(p.x * 10.0) - 0.5));
  float chatter = step(0.987 - u_treble * 0.05, hash21(floor(p * 48.0 + u_time * 18.0)));
  float deny = smoothstep(0.72, 1.0, u_peak) * line_mask(p.x, 0.060);

  vec3 accepted = vec3(0.08, 0.84, 0.80) * (1.0 - threshold);
  vec3 denied = vec3(0.95, 0.15, 0.16) * threshold;
  vec3 gates = vec3(1.0, 0.60, 0.20) * (gate_a + gate_b);
  vec3 marks = vec3(0.45, 0.95, 1.0) * ticks * 0.35;

  return accepted + denied * 0.35 + gates + marks + chatter * 0.8 + deny;
}
```

## 9. Devotional Interdict

Core feel:
- sacred symmetry
- machine lock bars
- worship contaminated by enforcement

Audio mapping:
- bass: halo expansion
- mids: liturgical rotation
- treble: choir-like shimmer
- peak: interdict flash

Pseudo-structure:

```glsl
vec3 devotional_interdict(vec2 frag) {
  vec2 p = center_uv(frag, u_resolution);
  float r = length(p);
  float a = atan(p.y, p.x);

  float halo = ring(p, 0.48 + u_bass * 0.08, 0.016);
  float spoke = 1.0 - smoothstep(0.02, 0.05, abs(sin(a * 6.0 + u_mid * 2.0)));
  spoke *= smoothstep(0.72, 0.14, r);

  float lockbar = line_mask(p.y, 0.035) * smoothstep(0.55, 0.08, abs(p.x));
  float shimmer = step(0.990 - u_treble * 0.05, hash21(floor(p * 42.0 + u_time * 20.0)));
  float interdict = smoothstep(0.8, 1.0, u_peak) * ring(p, 0.22, 0.030);

  vec3 col = vec3(0.0);
  col += vec3(0.08, 0.90, 0.84) * halo;
  col += vec3(1.0, 0.68, 0.24) * spoke * (0.3 + u_energy * 0.5);
  col += vec3(0.92, 0.14, 0.16) * lockbar;
  col += vec3(0.85, 1.0, 1.0) * shimmer * u_treble;
  col += vec3(1.0, 0.35, 0.2) * interdict;
  return col;
}
```

## Recommended Implementation Order

If these are turned into actual `tty0` visualizer modes, the strongest order is:

1. `containment_lattice`
2. `redaction_grid`
3. `judgment_engine`
4. `signal_cathedral`
5. `relay_spindle`

Why:

- they read clearly in a rectangular panel
- they fit the fiction
- they avoid generic liquid/noise sludge
- they give a good spread of moods without changing the rendering architecture

## Notes for Real GLSL

When these are implemented for WebGL2:

- keep loops with bounded constant upper limits
- avoid heavy branching in hot paths
- prefer SDF-like masks over large noise stacks
- keep feedback subtle unless the record specifically calls for drift or corruption
- treat red as a warning accent, not the base color
- keep the center readable; this app is an archive workstation, not a club flyer
