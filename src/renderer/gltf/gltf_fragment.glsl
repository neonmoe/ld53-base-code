#version 300 es
precision highp float;
precision highp int;

#define PI 3.14159265
#define MAX_LIGHTS 32

out vec4 FRAG_COLOR;

in vec3 world_pos;
in vec3 vertex_color;
in vec3 vertex_normal;
in vec4 vertex_tangent;
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
uniform Lights {
  int light_kind_and_color[MAX_LIGHTS];
  float light_intensity[MAX_LIGHTS];
  float light_angle_scale[MAX_LIGHTS];
  float light_angle_offset[MAX_LIGHTS];
  vec3 light_position[MAX_LIGHTS];
  vec3 light_direction[MAX_LIGHTS];
};

vec3 aces_filmic(vec3 x) {
  float a = 2.51;
  float b = 0.03;
  float c = 2.43;
  float d = 0.59;
  float e = 0.14;
  return clamp(x * (a * x + b) / (x * (c * x + d) + e), vec3(0), vec3(1));
}

vec3 diffuse_brdf(vec3 color) { return color / PI; }

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
  vec3 vertex_bitangent =
      normalize(cross(vertex_normal, vertex_tangent.xyz) * vertex_tangent.w);
  vec3 pixel_normal =
      normalize(mat3(vertex_tangent.xyz, vertex_bitangent, vertex_normal) *
                tangent_space_normal);

  float pixel_occlusion = 1.0 + occlusion_strength * (texel_occlusion - 1.0);
  vec3 light_emitted = texel_emissive.rgb * emissive_factor.rgb;

  float ambient_brightness = 0.3 * pixel_occlusion;
  vec3 light_incoming = vec3(ambient_brightness);
  for (int i = 0; i < MAX_LIGHTS; i++) {
    int kind_and_color = light_kind_and_color[i];
    int kind = kind_and_color >> 24;
    if (kind == 0) {
      break;
    }
    vec3 color = vec3(float((kind_and_color >> 16) & 0xFF) / 255.0,
                      float((kind_and_color >> 8) & 0xFF) / 255.0,
                      float(kind_and_color & 0xFF) / 255.0);
    vec3 to_light = light_position[i] - world_pos;
    // TODO: Handle spot and directional lights
    float distance_squared = dot(to_light, to_light);
    // The unit for point light intensity in KHR_lights_punctual is the candela,
    // which is luminous intensity.
    float luminous_intensity = light_intensity[i];
    // For PBR rendering, we want the radiant intensity (watts) instead. (I
    // think?) (TODO: The luminosity function is missing here, do we need it?)
    // Formula from: https://en.wikipedia.org/wiki/Luminous_intensity#Usage
    float radiant_intensity = luminous_intensity / 683.0;
    // Point light radiant intensity to radiant flux hack from:
    // https://pbr-book.org/3ed-2018/Light_Sources/Point_Lights
    float radiant_flux = radiant_intensity / distance_squared;
    float cos_factor = max(0.0, dot(pixel_normal, normalize(to_light)));
    // Without this, the scene is definitely too bright. I'm unsure if this is
    // the right factor though - this converts back from flux (W) to intensity
    // (W/sr), but afaik, we'd actually want the radiance (W/sr/m²)? Otoh, I
    // don't know what the area to divide by would be, so I guess radiant
    // intensity it is, for now.
    float fudge_factor = 1.0 / (4.0 * PI);
    light_incoming += color * radiant_intensity * cos_factor * fudge_factor;
  }

  // TODO: Add the rest of the BRDFs
  // (Very "draw the rest of the fucking owl" energy, I know.)
  vec3 brdf = diffuse_brdf(pixel_base_color);
  vec3 light_outgoing_to_camera = light_emitted + brdf * light_incoming;
  vec3 output_linear_color = aces_filmic(light_outgoing_to_camera);

  // The framebuffer is not SRGB, so we transform the linear color to
  // close-enough-to-srgb.
  FRAG_COLOR = vec4(pow(output_linear_color, vec3(1.0 / 2.2)), 1.0);
}
