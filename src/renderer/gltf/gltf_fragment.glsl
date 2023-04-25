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

  float brightness =
      0.2 * pixel_occlusion +
      0.8 * max(0.0, dot(normalize(vec3(1.0, 1.0, -1.0)), pixel_normal));
  // "Use" these as well, so that the uniforms don't get optimized out
  brightness += (pixel_metallic + pixel_roughness) * 0.001;
  vec3 output_linear_color = pixel_base_color * brightness + pixel_emissive;

  // The framebuffer is not SRGB, so we transform the linear color to
  // close-enough-to-srgb.
  FRAG_COLOR = vec4(pow(output_linear_color, vec3(1.0 / 2.2)), 1.0);
}
