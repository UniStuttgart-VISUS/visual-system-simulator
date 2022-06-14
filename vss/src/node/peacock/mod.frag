//modified version of https://raw.githubusercontent.com/jkulesza/peacock/master/python/peacock.py

uniform int u_track_error;
uniform float u_cb_strength;
uniform int u_cb_type;
// 0 = Protanopia
// 1 = Deuteranopia
// 2 = Tritanopia
// 3 = Monochromacy

uniform sampler2D s_color;
uniform sampler2D s_deflection;
uniform sampler2D s_color_change;
uniform sampler2D s_color_uncertainty;
uniform sampler2D s_covariances;

in vec2 v_tex;

out vec4 rt_color;
out vec4 rt_deflection;
out vec4 rt_color_change;
out vec4 rt_color_uncertainty;
out vec4 rt_covariances;

mat3 rgb2xyz = mat3(0.430574, 0.341550, 0.178325, 0.222015, 0.706655, 0.071330, 0.020183, 0.129553, 0.939180);
mat3 xyz2rgb = mat3(3.063218,-1.393325,-0.475802,-0.969243, 1.875966, 0.041555, 0.067871,-0.228834, 1.069251);

float gamma = 2.2;
float wx = 0.312713;
float wy = 0.329016;
float wz = 0.358271;

float v_cpu[3] = float[3](0.753, 1.140, 0.171);
float v_cpv[3] = float[3](0.265,-0.140,-0.003);
float v_am[3] = float[3](1.273463, 0.968437, 0.062921);
float v_ayi[3] = float[3](-0.073894, 0.003331, 0.292119);

float invPow(float x){
    return pow(clamp(x, 0.0, 1.0), 1.0/gamma);
}

vec3 convert_colorblind(in vec3 color){
    float cpu = v_cpu[u_cb_type];
    float cpv = v_cpv[u_cb_type];
    float am  = v_am[u_cb_type];
    float ayi = v_ayi[u_cb_type];

    vec3 c = vec3(pow(color.r, gamma), pow(color.g, gamma), pow(color.b, gamma));
    c *= rgb2xyz;
    float sum = c.x + c.y + c.z;
    float cu = 0.0;
    float cv = 0.0;
    if(sum != 0.0){
        cu = c.x/sum;
        cv = c.y/sum;
    }
    float nx = wx * c.y / wy;
    float nz = wz * c.y / wy;
    float clm = 0.0;
    vec3 d = vec3(0.0, 0.0, 0.0);
    
    if(cu < cpu){
        clm = (cpv - cv) / (cpu - cu);
    }else{
        clm = (cv - cpv) / (cu - cpu);
    }
    
    float clyi = cv - cu * clm;
    float du = (ayi - clyi) / (clm - am);
    float dv = (clm * du) + clyi;

    vec3 s = vec3(du * c.y / dv, c.y, (1.0 - (du + dv)) * c.y / dv);

    d.x = nx - s.x;
    d.z = nz - s.z;

    s *= xyz2rgb;
    d *= xyz2rgb;

    float adjr = (d.r > 0.0) ? (((s.r < 0.0 ? 0.0 : 1.0) - s.r) / d.r) : 0.0;
    float adjg = (d.g > 0.0) ? (((s.g < 0.0 ? 0.0 : 1.0) - s.g) / d.g) : 0.0;
    float adjb = (d.b > 0.0) ? (((s.b < 0.0 ? 0.0 : 1.0) - s.b) / d.b) : 0.0;

    float adjust = max(max(((adjr > 1.0) || (adjr < 0.0)) ? 0.0 : adjr, ((adjg > 1.0) || (adjg < 0.0)) ? 0.0 : adjg), ((adjb > 1.0) || (adjb < 0.0)) ? 0.0 : adjb);

    s += adjust*d;
    
    return vec3(invPow(s.r), invPow(s.g), invPow(s.b));
}

//unused right now, only here for reference
vec3 convert_anomylize(in vec3 color, in vec3 origin){
    float v = 1.75;
    float d = v + 1.0;
    
    return (v * color + origin) / d;
}

vec3 convert_monochrome(in vec3 color){
    float g_new = (color.r * 0.299) + (color.g * 0.587) + (color.b * 0.114);
    return vec3(g_new, g_new, g_new);
}

void main() {
    vec4 oldColor = texture(s_color, v_tex);
    vec4 newColor = oldColor;
    if(u_cb_strength > 0.0){
        if(u_cb_type < 3){
            newColor.rgb = convert_colorblind(oldColor.rgb);
        }else{
            newColor.rgb = convert_monochrome(oldColor.rgb);
        }
        //newColor.rgb = convert_anomylize(newColor.rgb, oldColor.rgb);
        newColor.rgb = mix(oldColor.rgb, newColor.rgb, u_cb_strength);
    }
    rt_color = newColor;

    if(u_track_error==1){
        rt_deflection = texture(s_deflection, v_tex);
        rt_color_change = texture(s_color_change, v_tex);
        rt_color_uncertainty = texture(s_color_uncertainty, v_tex);
        rt_covariances = texture(s_covariances, v_tex);
    }
}
