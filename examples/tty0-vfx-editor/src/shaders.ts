export const VERTEX_SHADER = `#version 300 es
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
}`;

export const FRAGMENT_SHADER = `#version 300 es
precision highp float;

in vec2 v_uv;

uniform vec2 u_resolution;
uniform float u_time;
uniform float u_motion_rate;
uniform float u_lattice_density;
uniform float u_circle_radius;
uniform float u_circle_falloff_start;
uniform float u_circle_falloff_end;
uniform float u_bulge_amount;
uniform float u_rim_guard;
uniform float u_rim_exponent;
uniform float u_rim_warp;
uniform float u_spacing_max_px;
uniform float u_spacing_min_px;
uniform float u_dot_size;
uniform float u_outer_dot_scale;
uniform float u_edge_softness;
uniform float u_chromatic_aberration;
uniform float u_scroll_base;
uniform float u_scroll_motion_scale;
uniform float u_scroll_motion_floor;
uniform float u_scroll_motion_ceiling;
uniform vec3 u_cold_color;
uniform vec3 u_hot_color;
uniform float u_color_cycle_rate;
uniform float u_inner_alpha;

out vec4 out_color;

float dot_mask(vec2 sample_px, float spacing_px, float radius_px, float edge_px) {
  vec2 local = mod(sample_px + 0.5 * spacing_px, spacing_px) - 0.5 * spacing_px;
  float distance_to_center = length(local);
  return 1.0 - smoothstep(radius_px, radius_px + edge_px, distance_to_center);
}

void main() {
  vec2 frag_px = v_uv * u_resolution;
  vec2 center = 0.5 * u_resolution;
  vec2 lens_delta = frag_px - center;
  vec2 radial_axis = length(lens_delta) > 0.0001 ? normalize(lens_delta) : vec2(1.0, 0.0);
  float lens_radius = u_circle_radius * min(u_resolution.x, u_resolution.y);
  float lens_distance = length(lens_delta);
  float normalized_radius = lens_distance / max(lens_radius, 1.0);
  float falloff = 1.0 - smoothstep(u_circle_falloff_start, u_circle_falloff_end, normalized_radius);
  float hemisphere = sqrt(max(0.0, 1.0 - normalized_radius * normalized_radius));
  vec2 sphere_xy = lens_radius > 0.0 ? lens_delta / lens_radius : vec2(0.0);
  vec3 sphere_normal = normalize(vec3(sphere_xy, max(hemisphere, 0.001)));
  float density = clamp((u_lattice_density - 2.0) / 10.0, 0.0, 1.0);
  float spacing_px = mix(u_spacing_max_px, u_spacing_min_px, density);
  float base_radius_px = spacing_px * u_dot_size;
  float scroll_px = u_time * (u_scroll_base + u_scroll_motion_scale * clamp(u_motion_rate - u_scroll_motion_floor, 0.0, u_scroll_motion_ceiling));
  vec2 base_sample_px = frag_px;
  base_sample_px.x += scroll_px;
  vec2 sphere_offset = frag_px - center;
  float center_profile = falloff * hemisphere;
  float magnify = 1.0 - center_profile * u_bulge_amount;
  vec2 warped_screen_px = center + sphere_offset * magnify;
  vec2 rim_direction = sphere_normal.xy / max(sphere_normal.z, u_rim_guard);
  float rim_profile = falloff * pow(clamp(1.0 - sphere_normal.z, 0.0, 1.0), u_rim_exponent);
  float rimWarp = rim_profile * u_rim_warp;
  vec2 sphere_sample_px = warped_screen_px + rim_direction * rimWarp;
  sphere_sample_px.x += scroll_px;
  float sphere_mix = clamp(falloff * hemisphere, 0.0, 1.0);
  float dot_radius_px = mix(base_radius_px * u_outer_dot_scale, base_radius_px, sphere_mix);
  vec2 final_sample_px = mix(base_sample_px, sphere_sample_px, sphere_mix);
  float chroma_drive = sphere_mix * u_chromatic_aberration;
  vec2 chroma_offset = radial_axis * chroma_drive;
  float mask_g = dot_mask(final_sample_px, spacing_px, dot_radius_px, u_edge_softness);
  float mask_r = dot_mask(final_sample_px + chroma_offset, spacing_px, dot_radius_px, u_edge_softness);
  float mask_b = dot_mask(final_sample_px - chroma_offset, spacing_px, dot_radius_px, u_edge_softness);
  float color_phase = 0.5 + 0.5 * sin(u_time * u_color_cycle_rate);
  vec3 base_color = mix(u_cold_color, u_hot_color, color_phase);
  vec3 dot_color = mix(vec3(1.0), base_color, sphere_mix);
  float outer_alpha = mask_g;
  float inner_alpha = mask_g * u_inner_alpha;
  float alpha = mix(outer_alpha, inner_alpha, sphere_mix);
  vec3 color = vec3(mask_r, mask_g, mask_b) * dot_color;
  out_color = vec4(clamp(color, 0.0, 1.0), clamp(alpha, 0.0, 1.0));
}`;
