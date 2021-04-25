#version 100
varying highp vec2 v_uv;
varying highp vec4 v_color;

uniform sampler2D u_texture;
uniform highp float u_alpha;

void main()
{
    highp vec4 color = texture2D(u_texture, v_uv);
    gl_FragColor =  vec4(color.rgb * u_alpha, color.a * u_alpha) * v_color;
}