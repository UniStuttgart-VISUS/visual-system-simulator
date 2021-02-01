uniform sampler2D s_color_r;
uniform sampler2D s_color_l;

uniform int u_flow_idx;

in vec2 v_tex;
out vec4 rt_color;

void main() {

  if (v_tex.x < 0.0 || v_tex.y < 0.0 ||
      v_tex.x > 1.0 || v_tex.y > 1.0) {
      discard;
  }
  if ( u_flow_idx == 0) {
    rt_color =  texture(s_color_r, v_tex);
  }
  else{
    rt_color =  texture(s_color_l, v_tex);
  }
}
