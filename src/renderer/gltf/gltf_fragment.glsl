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
uniform Material {
  vec4 base_color_factor;
  float metallic_factor;
  float roughness_factor;
  float normal_scale;
  float occlusion_strength;
  vec4 emissive_factor;
};

vec3 aces_filmic(vec3 x) {
  float a = 2.51;
  float b = 0.03;
  float c = 2.43;
  float d = 0.59;
  float e = 0.14;
  return clamp(x * (a * x + b) / (x * (c * x + d) + e), vec3(0), vec3(1));
}

void main() {
  vec4 texel_base_color = texture(base_color_tex, tex_coords);
  vec2 texel_metallic_roughness =
      texture(metallic_roughness_tex, tex_coords).rg;
  vec3 texel_normal = texture(normal_tex, tex_coords).rgb * 2.0 - 1.0;
  float texel_occlusion = texture(occlusion_tex, tex_coords).r;
  vec3 texel_emissive = texture(emissive_tex, tex_coords).rgb;

  vec3 pixel_base_color =
      texel_base_color.rgb * vertex_color * base_color_factor.rgb;
  float pixel_metallic = texel_metallic_roughness.x * metallic_factor;
  float pixel_roughness = texel_metallic_roughness.y * roughness_factor;

  vec3 tangent_space_normal =
      normalize(vec3(texel_normal.xy * normal_scale, texel_normal.z));
  vec3 pixel_normal =
      mat3(tbn_tangent, tbn_bitangent, tbn_normal) * tangent_space_normal;

  float pixel_occlusion = 1.0 + occlusion_strength * (texel_occlusion - 1.0);
  vec3 pixel_emissive = texel_emissive.rgb * emissive_factor.rgb;

  float light_indirect = 0.1;
  float brightness = light_indirect * pixel_occlusion;
  vec3 output_linear_color =
      aces_filmic(pixel_base_color * brightness + pixel_emissive);

  // The framebuffer is not SRGB, so we transform the linear color to
  // close-enough-to-srgb.
  FRAG_COLOR = vec4(pow(output_linear_color, vec3(1.0 / 2.2)), 1.0);
}
