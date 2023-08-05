#version 450
#extension GL_ARB_separate_shader_objects : enable
#extension GL_ARB_shading_language_420pack : enable

layout (set = 0, binding = 0) uniform sampler2D ui_texture;
layout (set = 0, binding = 1) uniform sampler2D media_texture;

layout (location = 0) in vec2 o_uv;
layout (location = 0) out vec4 a_frag_color;

void main() {
    vec4 ui_data = texture(ui_texture, o_uv);
    vec4 media_data = texture(media_texture, o_uv);
    
    a_frag_color = mix(media_data, ui_data, ui_data.a);
}