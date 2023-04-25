#version 300 es
precision mediump float;

out vec4 FRAG_COLOR;

in vec3 vertex_color;
in vec3 tbn_normal;
in vec3 tbn_tangent;
in vec3 tbn_bitangent;
in vec2 tex_coords;

uniform sampler2D base_color_tex;
uniform sampler2D metallic_roughness_tex;
uniform sampler2D normal_tex;
uniform sampler2D occlusion_tex;
uniform sampler2D emissive_tex;

void main() {
  vec4 base_color = texture(base_color_tex, tex_coords);
  vec2 metallic_roughness = texture(metallic_roughness_tex, tex_coords).rg;
  vec3 ts_normal = texture(normal_tex, tex_coords).rgb;
  float occlusion = texture(occlusion_tex, tex_coords).r;
  vec3 emissive = texture(emissive_tex, tex_coords).rgb;

  vec3 normal =
      mat3(tbn_tangent, tbn_bitangent, tbn_normal) * ts_normal * 0.0001 +
      tbn_normal;

  float brightness =
      0.2 * occlusion +
      0.8 * max(0.0, dot(normalize(vec3(1.0, 1.0, -1.0)), normal)) +
      metallic_roughness.x * 0.01;
  vec3 output_linear_color =
      vertex_color * base_color.rgb * vec3(brightness) + emissive;

  // The framebuffer is not SRGB, so we transform the linear color to
  // close-enough-to-srgb.
  FRAG_COLOR = vec4(pow(output_linear_color, vec3(1.0 / 2.2)), 1.0);
}
