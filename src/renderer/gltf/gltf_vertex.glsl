#version 300 es
layout(location = 0) in vec3 POSITION;
layout(location = 1) in vec3 NORMAL;
layout(location = 2) in vec4 TANGENT;
layout(location = 3) in vec2 TEXCOORD_0;
layout(location = 4) in vec2 TEXCOORD_1;
layout(location = 5) in vec3 COLOR_0;
layout(location = 6) in mat4 MODEL_TRANSFORM;

out vec3 vertex_color;
out vec3 tbn_normal;
out vec3 tbn_tangent;
out vec3 tbn_bitangent;
out vec2 tex_coords;

uniform mat4 proj_view_matrix;

void main() {
  vertex_color = COLOR_0;
  tbn_normal = normalize(NORMAL);
  tbn_tangent = normalize(TANGENT.xyz);
  tbn_bitangent = normalize(cross(tbn_normal, tbn_tangent) * TANGENT.w);
  tex_coords = TEXCOORD_0;
  gl_Position = proj_view_matrix * MODEL_TRANSFORM * vec4(POSITION, 1.0);
}
