//modified version of https://raw.githubusercontent.com/jkulesza/peacock/master/python/peacock.py

struct Uniforms{
    cb_cpu: f32,
    cb_cpv: f32,
    cb_am: f32,
    cb_ayi: f32,

    track_error: i32,
    cb_monochrome: i32,
    cb_strength: f32,
    
    _padding: f32
};
// possible types:
// 0 = Protanopia
// 1 = Deuteranopia
// 2 = Tritanopia
// 3 = Monochromacy

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

const rgb2xyz = mat3x3<f32>(vec3<f32>(0.430574, 0.341550, 0.178325), vec3<f32>(0.222015, 0.706655, 0.071330), vec3<f32>(0.020183, 0.129553, 0.939180));
const xyz2rgb = mat3x3<f32>(vec3<f32>(3.063218,-1.393325,-0.475802), vec3<f32>(-0.969243, 1.875966, 0.041555), vec3<f32>(0.067871,-0.228834, 1.069251));

const gamma: f32 = 2.2;
const wx: f32 = 0.312713;
const wy: f32 = 0.329016;
const wz: f32 = 0.358271;

struct ColorOut {
    @location(0) color: vec4<f32>,
};

struct MetricsAbOut {
    @location(0) metrics_a: vec4<f32>,
    @location(1) metrics_b: vec4<f32>,
};

struct MetricsCdOut {
    @location(0) metrics_c: vec4<f32>,
    @location(1) metrics_d: vec4<f32>,
};

@group(1) @binding(0)
var in_color_s: sampler;
@group(1) @binding(1)
var in_color_t: texture_2d<f32>;
@group(1) @binding(2)
var in_metrics_a_s: sampler;
@group(1) @binding(3)
var in_metrics_a_t: texture_2d<f32>;
@group(1) @binding(4)
var in_metrics_b_s: sampler;
@group(1) @binding(5)
var in_metrics_b_t: texture_2d<f32>;
@group(1) @binding(6)
var in_metrics_c_s: sampler;
@group(1) @binding(7)
var in_metrics_c_t: texture_2d<f32>;
@group(1) @binding(8)
var in_metrics_d_s: sampler;
@group(1) @binding(9)
var in_metrics_d_t: texture_2d<f32>;

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

fn convert_monochrome(color: vec3<f32>) -> vec3<f32>{
    let g_new = (color.r * 0.299) + (color.g * 0.587) + (color.b * 0.114);
    return vec3<f32>(g_new, g_new, g_new);
}

fn transformColor(tex_coords: vec2<f32>) -> vec4<f32> {
    let oldColor = textureSample(in_color_t, in_color_s, tex_coords);
    var newColor = oldColor;

    if(uniforms.cb_strength > 0.0){
        if(uniforms.cb_monochrome == 1){
            newColor = vec4<f32>(convert_monochrome(oldColor.rgb), newColor.a);
        }else{
            newColor = vec4<f32>(convert_colorblind(oldColor.rgb), newColor.a);
        }
        newColor = vec4<f32>(mix(oldColor.rgb, newColor.rgb, uniforms.cb_strength), newColor.a);
    }

    return newColor;
}

fn loadMetrics(tex_coords: vec2<f32>) -> PackedMetricsState {
    return unpackMetrics(
        textureSample(in_metrics_a_t, in_metrics_a_s, tex_coords),
        textureSample(in_metrics_b_t, in_metrics_b_s, tex_coords),
        textureSample(in_metrics_c_t, in_metrics_c_s, tex_coords),
        textureSample(in_metrics_d_t, in_metrics_d_s, tex_coords)
    );
}

@fragment
fn fs_color(in: VertexOutput) -> ColorOut {
    var out: ColorOut;
    out.color = transformColor(in.tex_coords);
    return out;
}

@fragment
fn fs_metrics_ab(in: VertexOutput) -> MetricsAbOut {
    var out: MetricsAbOut;
    let metrics = loadMetrics(in.tex_coords);
    out.metrics_a = packMetricsA(metrics);
    out.metrics_b = packMetricsB(metrics);
    return out;
}

@fragment
fn fs_metrics_cd(in: VertexOutput) -> MetricsCdOut {
    var out: MetricsCdOut;
    let metrics = loadMetrics(in.tex_coords);
    out.metrics_c = packMetricsC(metrics);
    out.metrics_d = packMetricsD(metrics);
    return out;
}
