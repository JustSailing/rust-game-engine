#version 450
#extension GL_ARB_separate_shader_objects : enable

layout(location = 0) out vec4 out_colour;

layout(set = 1, binding = 0) uniform local_uniform_object {
  vec4 diffuse_colour;
}
object_ubo;

void main() { out_colour = object_ubo.diffuse_colour; }