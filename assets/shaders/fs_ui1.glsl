#version 460

layout(location = 0) in vec2 fragPos;
layout(location = 0) out vec4 fragColor;

layout(set = 0, binding = 0) uniform sampler2D ui_texture;

void main() {
    fragColor = texture(ui_texture, fragPos);
}