pub const VERTEX_SHADER_SOURCE: &str = "#version 300 es
precision highp float;

out vec2 v_uv;

const vec2 pos[3] = vec2[](
  vec2(-1.0, -1.0),
  vec2( 3.0, -1.0),
  vec2(-1.0,  3.0)
);

void main() {
  vec2 p = pos[gl_VertexID];
  v_uv = 0.5 * (p + 1.0);
  gl_Position = vec4(p, 0.0, 1.0);
}
";

pub const FRAGMENT_SHADER_SOURCE: &str = "#version 300 es
precision highp float;

in vec2 v_uv;

uniform sampler2D u_scene;
uniform vec2 u_resolution;
uniform float u_time;

uniform float u_curvature;
uniform float u_scanline_strength;
uniform float u_mask_strength;
uniform float u_vignette_strength;
uniform float u_aberration;
uniform float u_bloom;

out vec4 out_color;

vec2 curve_uv(vec2 uv, float k) {
  vec2 p = uv * 2.0 - 1.0;
  vec2 a = abs(p);
  p += p * (a.yx * a.yx) * k;
  return p * 0.5 + 0.5;
}

vec3 triad_mask(vec2 frag_coord) {
  float x = mod(frag_coord.x, 3.0);
  if (x < 1.0) return vec3(1.0, 0.65, 0.65);
  if (x < 2.0) return vec3(0.65, 1.0, 0.65);
  return vec3(0.65, 0.65, 1.0);
}

vec3 sample_scene(vec2 uv) {
  return texture(u_scene, uv).rgb;
}

void main() {
  vec2 frag = v_uv * u_resolution;

  vec2 uv = curve_uv(v_uv, u_curvature);
  if (uv.x < 0.0 || uv.x > 1.0 || uv.y < 0.0 || uv.y > 1.0) {
    out_color = vec4(0.0, 0.0, 0.0, 1.0);
    return;
  }

  vec2 px = 1.0 / u_resolution;

  float ab = u_aberration;
  vec3 c;
  c.r = sample_scene(uv + vec2( ab, 0.0) * px).r;
  c.g = sample_scene(uv).g;
  c.b = sample_scene(uv + vec2(-ab, 0.0) * px).b;

  float scan = 0.5 + 0.5 * sin((frag.y + mod(u_time * 60.0, 1000.0)) * 3.14159265 / 2.0);
  float scan_mul = mix(1.0, 0.5 + 0.5 * scan, u_scanline_strength);
  c *= scan_mul;

  vec3 mask = triad_mask(frag);
  c *= mix(vec3(1.0), mask, u_mask_strength);

  float dx = px.x;
  float dy = px.y;
  vec3 bloom =
    sample_scene(uv + vec2( dx, 0.0)) +
    sample_scene(uv + vec2(-dx, 0.0)) +
    sample_scene(uv + vec2(0.0,  dy)) +
    sample_scene(uv + vec2(0.0, -dy));
  bloom *= 0.25;
  c = mix(c, max(c, bloom), u_bloom);

  vec2 d = v_uv * (1.0 - v_uv);
  float vig = d.x * d.y * 16.0;
  vig = pow(clamp(vig, 0.0, 1.0), 0.25);
  c *= mix(1.0, vig, u_vignette_strength);

  c = pow(c, vec3(1.0 / 2.2));
  out_color = vec4(c, 1.0);
}
";
