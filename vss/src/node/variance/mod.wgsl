
struct Uniforms{
    resolution: vec2<f32>,
    track_error: i32,
    show_variance: u32,
    variance_metric: u32,
    color_space: u32,
}

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

const VAR_SIZE: i32 = 10;

const COLOR_SPACE_RGB: u32 = 0u;
const COLOR_SPACE_LAB: u32 = 1u;
const COLOR_SPACE_ITP: u32 = 2u;

const SHOW_VARIANCE_NONE: u32 = 0u;
const SHOW_VARIANCE_PRE:  u32 = 1u;
const SHOW_VARIANCE_POST: u32 = 2u;
const SHOW_VARIANCE_DIFF: u32 = 3u;

const VARIANCE_METRIC_FIRST: u32 = 0u;
const VARIANCE_METRIC_AVG: u32 = 1u;
const VARIANCE_METRIC_VAR_AVG: u32 = 2u;
const VARIANCE_METRIC_FIRST_CONTRAST: u32 = 3u;
const VARIANCE_METRIC_DELTA_E: u32 = 4u;
const VARIANCE_METRIC_MICHELSON_CONTRAST: u32 = 5u;
const VARIANCE_METRIC_HISTOGRAM: u32 = 6u;

fn rgb2xyz(c: vec3<f32>) -> vec3<f32>{
	let tmp=vec3<f32>(
        mix(c.r / 12.92, pow((c.r + .055) / 1.055, 2.4), f32(c.r > .04045)),
        mix(c.g / 12.92, pow((c.g + .055) / 1.055, 2.4), f32(c.g > .04045)),
        mix(c.b / 12.92, pow((c.b + .055) / 1.055, 2.4), f32(c.b > .04045))
	);
	let m = mat3x3<f32>(
		vec3<f32>(.4124, .3576, .1805),
		vec3<f32>(.2126, .7152, .0722),
		vec3<f32>(.0193, .1192, .9505)
	);
	return 100. * (tmp * m);
}
fn xyz2lab(c: vec3<f32>) -> vec3<f32>{
	let n = c / vec3<f32>(95.047, 100., 108.883);
	let v = vec3<f32>(
        mix((7.787 * n.x) + (16. / 116.), pow(n.x, 1. / 3.), f32(n.x > .008856)),
        mix((7.787 * n.y) + (16. / 116.), pow(n.y, 1. / 3.), f32(n.y > .008856)),
        mix((7.787 * n.z) + (16. / 116.), pow(n.z, 1. / 3.), f32(n.z > .008856))
	);
	return vec3<f32>((116. * v.y) - 16., 500. * (v.x - v.y), 200. * (v.y - v.z));
}
fn rgb2lab(c: vec3<f32>) -> vec3<f32>{
	let lab = xyz2lab(rgb2xyz(c));
	return vec3<f32>(lab.x / 100., .5 + .5 * (lab.y / 127.), .5 + .5 * (lab.z / 127.));
}

//E is the signal for each colour component {R, G, B} proportional to scene linear light and scaled by camera exposure, normalized to the range [0:12]
//https://glenwing.github.io/docs/ITU-R-BT.2100-0.pdf site 5(7 on pdf)
fn oetf(e: vec3<f32>) -> vec3<f32>{
    let a = 0.17883277;
    let b = 0.28466892;
    let c = 0.55991073;
    var result = vec3<f32>(0.0);
    result.r = mix(a * log(e.r - b) + c, sqrt(e.r) / 2.0, f32(e.r <= 1.0));
    result.g = mix(a * log(e.g - b) + c, sqrt(e.g) / 2.0, f32(e.g <= 1.0));
    result.b = mix(a * log(e.b - b) + c, sqrt(e.b) / 2.0, f32(e.b <= 1.0));
    return result;
}

//https://en.wikipedia.org/wiki/ICtCp
//we assume the rgb value is in https://de.wikipedia.org/wiki/ITU-R-Empfehlung_BT.2020
fn rgb2itp(c: vec3<f32>) -> vec3<f32>{
    let lmsMat = mat3x3<f32>(vec3<f32>(1688., 683., 99.), vec3<f32>(2146., 2951., 309.), vec3<f32>(262., 462., 3688.));
    let ictcpMat = mat3x3<f32>(vec3<f32>(2048., 3625., 9500.), vec3<f32>(2048., -7465., -9212.), vec3<f32>(0., 3840., -288.));
    let lms = lmsMat * c / 4096.;
    return ictcpMat * oetf(lms) / 4096.;
}

fn sampleColor(color_t: texture_2d<f32>, color_s: sampler, tex_coords: vec2<f32>) -> vec3<f32>{
    if(uniforms.color_space == COLOR_SPACE_RGB){
        return textureSample(color_t, color_s, tex_coords).rgb;
    }else if(uniforms.color_space == COLOR_SPACE_LAB){
        return rgb2lab(textureSample(color_t, color_s, tex_coords).rgb);
    }else if(uniforms.color_space == COLOR_SPACE_ITP){
        return rgb2itp(textureSample(color_t, color_s, tex_coords).rgb);
    }else{
        return textureSample(color_t, color_s, tex_coords).rgb;
    }
}

//first method of measuring variance (only taking into account the difference to main color)
fn varianceMeasure_first(color_t: texture_2d<f32>, color_s: sampler, tex_coords: vec2<f32>) -> f32{
    var result = 0.0;
    let main_color = sampleColor(color_t, color_s, tex_coords);

    for (var i: i32 = -VAR_SIZE; i <= VAR_SIZE; i++) {
        for (var j: i32 = -VAR_SIZE; j <= VAR_SIZE; j++) {
            let diff = abs(main_color - sampleColor(color_t, color_s, (tex_coords + vec2<f32>(f32(i), f32(j)) / uniforms.resolution)));
            result += diff.x * diff.x + diff.y * diff.y + diff.z * diff.z;
        }
    }
    return result/pow(f32(VAR_SIZE) * 2. + 1., 2.);
}

//average difference to main color
fn varianceMeasure_avg(color_t: texture_2d<f32>, color_s: sampler, tex_coords: vec2<f32>) -> f32{
    var avg = vec3<f32>(0.0);
    let main_color = sampleColor(color_t, color_s, tex_coords);

    for (var i: i32 = -VAR_SIZE; i <= VAR_SIZE; i++) {
        for (var j: i32 = -VAR_SIZE; j <= VAR_SIZE; j++) {
            avg += abs(main_color - sampleColor(color_t, color_s, (tex_coords + vec2<f32>(f32(i), f32(j)) / uniforms.resolution)));
        }
    }
    return (avg.r + avg.g + avg.b) / (pow(f32(VAR_SIZE) * 2. + 1., 2.) * 3.0);
}

//second method of measuring variance (taking into account the difference to main color relative to average difference)
fn varianceMeasure_var_avg(color_t: texture_2d<f32>, color_s: sampler, tex_coords: vec2<f32>) -> f32{
    var result = 0.0;
    var avg = vec3<f32>(0.0);
    let main_color = sampleColor(color_t, color_s, tex_coords);

    for (var i: i32 = -VAR_SIZE; i <= VAR_SIZE; i++) {
        for (var j: i32 = -VAR_SIZE; j <= VAR_SIZE; j++) {
            avg += abs(main_color - sampleColor(color_t, color_s, (tex_coords + vec2<f32>(f32(i), f32(j)) / uniforms.resolution)));
        }
    }
    avg /= pow(f32(VAR_SIZE) * 2. + 1., 2.);

    //durchschnittliche varianz
    for (var i: i32 = -VAR_SIZE; i <= VAR_SIZE; i++) {
        for (var j: i32 = -VAR_SIZE; j <= VAR_SIZE; j++) {
            let diff = abs(main_color - sampleColor(color_t, color_s, (tex_coords + vec2<f32>(f32(i), f32(j)) / uniforms.resolution))) - avg;
            result += diff.x * diff.x + diff.y * diff.y + diff.z * diff.z;
        }
    }
    return result / pow(f32(VAR_SIZE) * 2. + 1., 2.);
}

fn varianceMeasure_first_contrast(color_t: texture_2d<f32>, color_s: sampler, tex_coords: vec2<f32>) -> f32{
    var avg = vec3<f32>(0.0);
    var st_deviation = vec3<f32>(0.0);
    let main_color = sampleColor(color_t, color_s, tex_coords);

    for (var i: i32 = -VAR_SIZE; i <= VAR_SIZE; i++) {
        for (var j: i32 = -VAR_SIZE; j <= VAR_SIZE; j++) {
            avg += abs(main_color - sampleColor(color_t, color_s, (tex_coords + vec2<f32>(f32(i), f32(j)) / uniforms.resolution)));
        }
    }
    avg /= pow(f32(VAR_SIZE) * 2. + 1., 2.);
    
    for (var i: i32 = -VAR_SIZE; i <= VAR_SIZE; i++) {
        for (var j: i32 = -VAR_SIZE; j <= VAR_SIZE; j++) {
            let diff = main_color - sampleColor(color_t, color_s, (tex_coords + vec2<f32>(f32(i), f32(j)) / uniforms.resolution));
            st_deviation += diff * diff;
        }
    }
    return (st_deviation / (pow(f32(VAR_SIZE) * 2. + 1., 2.) - 1.0)).x;
}

//delta E distance measure
fn varianceMeasure_delta_e(color_t: texture_2d<f32>, color_s: sampler, tex_coords: vec2<f32>) -> f32{
    var result = 0.0;
    let itpWeights = vec3<f32>(1.0, 0.5, 1.0);
    let main_color = sampleColor(color_t, color_s, tex_coords)*itpWeights;
    
    for (var i: i32 = -VAR_SIZE; i <= VAR_SIZE; i++) {
        for (var j: i32 = -VAR_SIZE; j <= VAR_SIZE; j++) {
            let diff = main_color - sampleColor(color_t, color_s, (tex_coords + vec2<f32>(f32(i), f32(j)) / uniforms.resolution)) * itpWeights;
            //TODO: find proper scaling for this value
            result += 720.0 * length(diff);
        }
    }
    return result / (pow(f32(VAR_SIZE) * 2. + 1., 2.) - 1.0);
}

//Michelson-Kontrast
fn varianceMeasure_michelson_contrast(color_t: texture_2d<f32>, color_s: sampler, tex_coords: vec2<f32>) -> f32{
    var minValues = vec3<f32>(1.0, 1.0, 1.0);
    var maxValues = vec3<f32>(0.0, 0.0, 0.0);
    
    for (var i: i32 = -VAR_SIZE; i <= VAR_SIZE; i++) {
        for (var j: i32 = -VAR_SIZE; j <= VAR_SIZE; j++) {
            let color = sampleColor(color_t, color_s, (tex_coords + vec2<f32>(f32(i), f32(j)) / uniforms.resolution));
            minValues = min(minValues, color);
            maxValues = max(maxValues, color);
        }
    }
    let contrast = (maxValues-minValues)/(maxValues+minValues);
    //this way of adding up the components only applies to the LAB color format
    return abs(contrast.r) + length(contrast.gb);
}

fn sampleVariance(color_t: texture_2d<f32>, color_s: sampler, tex_coords: vec2<f32>) -> f32{
    if(uniforms.variance_metric == VARIANCE_METRIC_FIRST){
        return varianceMeasure_first(color_t, color_s, tex_coords);
    }else if(uniforms.variance_metric == VARIANCE_METRIC_AVG){
        return varianceMeasure_avg(color_t, color_s, tex_coords);
    }else if(uniforms.variance_metric == VARIANCE_METRIC_VAR_AVG){
        return varianceMeasure_var_avg(color_t, color_s, tex_coords);
    }else if(uniforms.variance_metric == VARIANCE_METRIC_FIRST_CONTRAST){
        return varianceMeasure_first_contrast(color_t, color_s, tex_coords);
    }else if(uniforms.variance_metric == VARIANCE_METRIC_DELTA_E){
        return varianceMeasure_delta_e(color_t, color_s, tex_coords);
    }else if(uniforms.variance_metric == VARIANCE_METRIC_MICHELSON_CONTRAST){
        return varianceMeasure_michelson_contrast(color_t, color_s, tex_coords);
    }else{
        return 0.0;
    }
}

const kRedVec4: vec4<f32> = vec4<f32>(0.13572138, 4.61539260, -42.66032258, 132.13108234);
const kGreenVec4: vec4<f32> = vec4<f32>(0.09140261, 2.19418839, 4.84296658, -14.18503333);
const kBlueVec4: vec4<f32> = vec4<f32>(0.10667330, 12.64194608, -60.58204836, 110.36276771);
const kRedVec2: vec2<f32> = vec2<f32>(-152.94239396, 59.28637943);
const kGreenVec2: vec2<f32> = vec2<f32>(4.27729857, 2.82956604);
const kBlueVec2: vec2<f32> = vec2<f32>(-89.90310912, 27.34824973);

fn TurboColormap(x_in: f32) -> vec3<f32>{
  let x = clamp(x_in, 0.0, 1.0);

  let v4 = vec4<f32>( 1.0, x, x * x, x * x * x);
  let v2 = v4.zw * v4.z;
  return vec3<f32>(
    dot(v4, kRedVec4)   + dot(v2, kRedVec2),
    dot(v4, kGreenVec4) + dot(v2, kGreenVec2),
    dot(v4, kBlueVec4)  + dot(v2, kBlueVec2)
  );
}

//taken from https://www.shadertoy.com/view/XtGGzG
fn viridis_quintic(x_in: f32) -> vec3<f32>{
	let x = clamp( x_in, 0.0, 1.0 );
	let x1 = vec4<f32>( 1.0, x, x * x, x * x * x ); // 1 x x2 x3
	let x2 = x1 * x1.w * x; // x4 x5 x6 x7
	return vec3<f32>(
		dot( x1.xyzw, vec4<f32>(  0.280268003, -0.143510503,  2.225793877 ,-14.815088879 ) ) + dot( x2.xy, vec2<f32>(  25.212752309, -11.772589584 ) ),
		dot( x1.xyzw, vec4<f32>( -0.002117546,  1.617109353, -1.909305070 , 2.701152864  ) ) + dot( x2.xy, vec2<f32>( -1.685288385 ,  0.178738871  ) ),
		dot( x1.xyzw, vec4<f32>(  0.300805501,  2.614650302, -12.019139090, 28.933559110 ) ) + dot( x2.xy, vec2<f32>( -33.491294770,  13.762053843 ) ) );
}

struct FragmentOutput {
    @location(0) color: vec4<f32>,
    @location(1) deflection: vec4<f32>,
    @location(2) color_change: vec4<f32>,
    @location(3) color_uncertainty: vec4<f32>,
    @location(4) covariances: vec4<f32>,
    @location(5) measurement: vec4<f32>,
};

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

@group(2) @binding(0)
var in_original_t: texture_2d<f32>;
@group(2) @binding(1)
var in_original_s: sampler;

@fragment
fn fs_main(in: VertexOutput) -> FragmentOutput {
    var out: FragmentOutput;
    if(uniforms.variance_metric == VARIANCE_METRIC_HISTOGRAM){
        if(uniforms.show_variance == SHOW_VARIANCE_PRE){
            out.measurement = vec4<f32>(sampleColor(in_original_t, in_original_s, in.tex_coords), 1.0);
        }else if(uniforms.show_variance == SHOW_VARIANCE_POST){
            out.measurement = vec4<f32>(sampleColor(in_color_t, in_color_s, in.tex_coords), 1.0);
        }else{// SHOW_VARIANCE_NONE & SHOW_VARIANCE_DIFF
            out.measurement = vec4<f32>(0.0);
        }
        out.color = vec4<f32>(textureSample(in_color_t, in_color_s, in.tex_coords).rgb, 1.0);
    }else{
        if(uniforms.show_variance == SHOW_VARIANCE_PRE){
            let variance_original = sampleVariance(in_original_t, in_original_s, in.tex_coords);
            out.color = vec4<f32>(viridis_quintic(variance_original), 1.0);
            out.measurement = vec4<f32>(variance_original);
        }else if(uniforms.show_variance == SHOW_VARIANCE_POST){
            let variance_sim = sampleVariance(in_color_t, in_color_s, in.tex_coords);
            out.color = vec4<f32>(viridis_quintic(variance_sim), 1.0);
            out.measurement = vec4<f32>(variance_sim);
        }else if(uniforms.show_variance == SHOW_VARIANCE_DIFF){
            let variance_original = sampleVariance(in_original_t, in_original_s, in.tex_coords);
            let variance_sim = sampleVariance(in_color_t, in_color_s, in.tex_coords);
            let loss = variance_original-variance_sim;
            out.color = vec4<f32>(viridis_quintic(loss), 1.0);
            out.measurement = vec4<f32>(loss);
        }else{// SHOW_VARIANCE_NONE
            out.color = vec4<f32>(sampleColor(in_color_t, in_color_s, in.tex_coords), 1.0);
            out.measurement = vec4<f32>(0.0);
        }
    }

    if ( uniforms.track_error == 1 ){
        out.color_change =      textureSample(in_color_change_t, in_color_change_s, in.tex_coords);
        out.color_uncertainty = textureSample(in_color_uncertainty_t, in_color_uncertainty_s, in.tex_coords);
        out.deflection =        textureSample(in_deflection_t, in_deflection_s, in.tex_coords);
        out.covariances =       textureSample(in_covariances_t, in_covariances_s, in.tex_coords);
    }
    return out;
}
