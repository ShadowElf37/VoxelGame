#version 460

//layout(location = 0) in vec3 view_normal;
layout(location = 0) in vec2 tex_coords;

layout(set = 0, binding = 1) uniform sampler2D tex_side;
layout(set = 0, binding = 2) uniform sampler2D tex_top;
layout(set = 0, binding = 3) uniform sampler2D tex_bottom;

layout(location = 0) out vec4 f_color;

void main() {
    f_color = texture(tex, tex_coords);
}