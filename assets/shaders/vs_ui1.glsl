#version 460

layout(location = 0) in vec2 pos;
layout(location = 0) out vec2 fragPos;

void main() {
    fragPos = pos;
    gl_Position = vec4(pos, -50.0, 1.0);
}