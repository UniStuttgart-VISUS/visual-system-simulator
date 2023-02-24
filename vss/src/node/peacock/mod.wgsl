//modified version of https://raw.githubusercontent.com/jkulesza/peacock/master/python/peacock.py

struct Uniforms{
    cb_cpu: f32,
    cb_cpv: f32,
    cb_am: f32,
    cb_ayi: f32,

    track_error: i32,
    cb_monochrome: i32,
    cb_strength: f32,
};
// possible types:
// 0 = Protanopia
// 1 = Deuteranopia
// 2 = Tritanopia
// 3 = Monochromacy

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

struct FragmentOutput {
    @location(0) color: vec4<f32>,
    @location(1) deflection: vec4<f32>,
    @location(2) color_change: vec4<f32>,
    @location(3) color_uncertainty: vec4<f32>,
    @location(4) covariances: vec4<f32>,
};

const rgb2xyz = mat3x3<f32>(vec3<f32>(0.430574, 0.341550, 0.178325), vec3<f32>(0.222015, 0.706655, 0.071330), vec3<f32>(0.020183, 0.129553, 0.939180));
const xyz2rgb = mat3x3<f32>(vec3<f32>(3.063218,-1.393325,-0.475802), vec3<f32>(-0.969243, 1.875966, 0.041555), vec3<f32>(0.067871,-0.228834, 1.069251));

const gamma: f32 = 2.2;
const wx: f32 = 0.312713;
const wy: f32 = 0.329016;
const wz: f32 = 0.358271;

fn invPow(x: f32) -> f32{
    return pow(clamp(x, 0.0, 1.0), 1.0/gamma);
}

fn convert_colorblind(color: vec3<f32>) -> vec3<f32>{
    let cpu = uniforms.cb_cpu;
    let cpv = uniforms.cb_cpv;
    let am  = uniforms.cb_am;
    let ayi = uniforms.cb_ayi;

    var c = vec3<f32>(pow(color.r, gamma), pow(color.g, gamma), pow(color.b, gamma));
    c *= rgb2xyz;
    let sum = c.x + c.y + c.z;
    var cu = 0.0;
    var cv = 0.0;
    if(sum != 0.0){
        cu = c.x/sum;
        cv = c.y/sum;
    }
    let nx = wx * c.y / wy;
    let nz = wz * c.y / wy;
    var clm = 0.0;
    var d = vec3<f32>(0.0, 0.0, 0.0);
    
    if(cu < cpu){
        clm = (cpv - cv) / (cpu - cu);
    }else{
        clm = (cv - cpv) / (cu - cpu);
    }
    
    let clyi = cv - cu * clm;
    let du = (ayi - clyi) / (clm - am);
    let dv = (clm * du) + clyi;

    var s = vec3<f32>(du * c.y / dv, c.y, (1.0 - (du + dv)) * c.y / dv);

    d.x = nx - s.x;
    d.z = nz - s.z;

    s *= xyz2rgb;
    d *= xyz2rgb;

    let adjr = mix(0.0, ((mix(1.0, 0.0, f32(s.r < 0.0)) - s.r) / d.r), f32(d.r > 0.0));
    let adjg = mix(0.0, ((mix(1.0, 0.0, f32(s.g < 0.0)) - s.g) / d.g), f32(d.g > 0.0));
    let adjb = mix(0.0, ((mix(1.0, 0.0, f32(s.b < 0.0)) - s.b) / d.b), f32(d.b > 0.0));

    let adjust = max(max(
        mix(adjr, 0.0, f32((adjr > 1.0) || (adjr < 0.0))),
        mix(adjg, 0.0, f32((adjg > 1.0) || (adjg < 0.0)))),
        mix(adjb, 0.0, f32((adjb > 1.0) || (adjb < 0.0))));

    s += adjust*d;
    
    return vec3<f32>(invPow(s.r), invPow(s.g), invPow(s.b));
}

//unused right now, only here for reference
fn convert_anomylize(color: vec3<f32>, origin: vec3<f32>) -> vec3<f32>{
    let v = 1.75;
    let d = v + 1.0;
    
    return (v * color + origin) / d;
}

fn convert_monochrome(color: vec3<f32>) -> vec3<f32>{
    let g_new = (color.r * 0.299) + (color.g * 0.587) + (color.b * 0.114);
    return vec3<f32>(g_new, g_new, g_new);
}

// Fragment shader

@group(1) @binding(0)
var in_color_s: sampler;
@group(1) @binding(1)
var in_color_t: texture_2d<f32>;
@group(1) @binding(2)
var in_deflection_s: sampler;
@group(1) @binding(3)
var in_deflection_t: texture_2d<f32>;
@group(1) @binding(4)
var in_color_change_s: sampler;
@group(1) @binding(5)
var in_color_change_t: texture_2d<f32>;
@group(1) @binding(6)
var in_color_uncertainty_s: sampler;
@group(1) @binding(7)
var in_color_uncertainty_t: texture_2d<f32>;
@group(1) @binding(8)
var in_covariances_s: sampler;
@group(1) @binding(9)
var in_covariances_t: texture_2d<f32>;

@fragment
fn fs_main(in: VertexOutput) -> FragmentOutput {
    var out: FragmentOutput;
    let oldColor = textureSample(in_color_t, in_color_s, in.tex_coords);
    var newColor = oldColor;

    if(uniforms.cb_strength > 0.0){
        if(uniforms.cb_monochrome == 1){
            newColor = vec4<f32>(convert_monochrome(oldColor.rgb), newColor.a);
        }else{
            newColor = vec4<f32>(convert_colorblind(oldColor.rgb), newColor.a);
        }
        //newColor.rgb = convert_anomylize(newColor.rgb, oldColor.rgb);
        newColor = vec4<f32>(mix(oldColor.rgb, newColor.rgb, uniforms.cb_strength), newColor.a);
    }

    out.color = newColor;

    if(uniforms.track_error == 1){
        out.deflection = textureSample(in_deflection_t, in_deflection_s, in.tex_coords);
        out.color_change = textureSample(in_color_change_t, in_color_change_s, in.tex_coords);
        out.color_uncertainty = textureSample(in_color_uncertainty_t, in_color_uncertainty_s, in.tex_coords);
        out.covariances = textureSample(in_covariances_t, in_covariances_s, in.tex_coords);
    }
    return out;
}
