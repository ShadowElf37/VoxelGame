#version 460

//layout(location = 0) in vec3 view_normal;
layout(location = 0) in vec2 tex_coords;
layout(location = 1) flat in uint tex_id;

layout(set = 0, binding = 1) uniform sampler2D tex[3];

layout(location = 0) out vec4 f_color;

void main() {
    f_color = texture(tex[tex_id], tex_coords);
}