#version 100
uniform highp mat3 u_transform;

attribute highp vec2 a_pos;
attribute highp vec2 a_uv;
attribute highp vec4 a_color;

varying vec2 v_uv;
varying vec4 v_color;

void main()
{
    v_uv = a_uv;
    v_color = a_color;
    gl_Position = vec4((u_transform * vec3(a_pos, 1.0)).xy, 0.0, 1.0);
}