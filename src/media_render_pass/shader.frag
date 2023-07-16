#version 450
#extension GL_ARB_separate_shader_objects : enable
#extension GL_ARB_shading_language_420pack : enable

layout (set = 0, binding = 0) uniform sampler2D textureLinearYUV420P;
layout (set = 0, binding = 1) uniform Uniform {
    int index;
} ubo;

layout (location = 0) in vec2 o_uv;
layout (location = 0) out vec4 a_frag_color;

void main() {
    // dynamically indexing into the array of textures doesnt work.
    // if (ubo.index == 0) {
        a_frag_color = texture(textureLinearYUV420P, o_uv);
        // return;
    // }

    // if (ubo.index == 1) {
    //     a_frag_color = texture(textureLinearYUV420P[1], o_uv);
    //     return;
    // }

    // if (ubo.index == 2) {
    //     a_frag_color = texture(textureLinearYUV420P[2], o_uv);
    //     return;
    // }

    // if (ubo.index == 3) {
    //     a_frag_color = texture(textureLinearYUV420P[3], o_uv);
    //     return;
    // }

    // a_frag_color = vec4(0.0, 0.0, 0.0, 1.0);
}