#version 100
uniform highp mat3 u_transform;

attribute highp vec2 a_pos;
attribute highp vec2 a_uv;

varying vec2 v_uv;

void main()
{
    v_uv = a_uv;
    gl_Position = vec4((u_transform * vec3(a_pos, 1.0)).xy, 0.0, 1.0);
}