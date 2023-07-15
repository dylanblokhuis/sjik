#version 400
#extension GL_ARB_separate_shader_objects : enable
#extension GL_ARB_shading_language_420pack : enable

layout (set = 0, binding = 0) uniform sampler2D u_textureLinearYUV420P;

layout (location = 0) in vec2 o_uv;
layout (location = 0) out vec4 a_frag_color;

void main() {
    a_frag_color = texture(u_textureLinearYUV420P, o_uv);
}