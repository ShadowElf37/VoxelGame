#version 460

layout(location = 0) in vec3 position;
layout(location = 1) in vec2 uv;

layout(location = 0) out vec2 tex_coords;

//layout(location = 0) out vec3 view_normal;

layout(set = 0, binding = 0) uniform Data {
    //vec4 facing;
    //vec4 cam_pos;
    mat4 view;
    mat4 proj;
} data;

void main() {
    tex_coords = uv;
    //view_normal = transpose(inverse(mat3(data.view))) * normal;
    gl_Position = data.proj * data.view * vec4(position, 1.0);
}