// Vertex shader

struct Uniforms{
    hive_rotation: mat4x4<f32>,

    resolution_in: vec2<f32>,
    hive_position: vec2<f32>,

    hive_visible: i32,
    flow_idx: i32,

    heat_scale: f32,
    dir_calc_scale: f32,

    base_image: i32,
    combination_function: i32,
    mix_type: i32,
    colormap_type: i32,
};

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

struct FragmentOutput {
    @location(0) color: vec4<f32>,
};

// Viridis colormap, based on https://github.com/glslify/glsl-colormap
const e0 = 0.0;
const v0 = vec3<f32>(0.26666666666666666,0.00392156862745098,0.32941176470588235);
const e1 = 0.13;
const v1 = vec3<f32>(0.2784313725490196,0.17254901960784313,0.47843137254901963);
const e2 = 0.25;
const v2 = vec3<f32>(0.23137254901960785,0.3176470588235294,0.5450980392156862);
const e3 = 0.38;
const v3 = vec3<f32>(0.17254901960784313,0.44313725490196076,0.5568627450980392);
const e4 = 0.5;
const v4 = vec3<f32>(0.12941176470588237,0.5647058823529412,0.5529411764705883);
const e5 = 0.63;
const v5 = vec3<f32>(0.15294117647058825,0.6784313725490196,0.5058823529411764);
const e6 = 0.75;
const v6 = vec3<f32>(0.3607843137254902,0.7843137254901961,0.38823529411764707);
const e7 = 0.88;
const v7 = vec3<f32>(0.6666666666666666,0.8627450980392157,0.19607843137254902);
const e8 = 1.0;
const v8 = vec3<f32>(0.9921568627450981,0.9058823529411765,0.1450980392156863);

fn ViridisColormap(x_0: f32) -> vec3<f32>{
  let a0 = mix(v0,v1,smoothstep(e0,e1,x_0))*step(e0,x_0)*step(x_0,e1);
  let a1 = mix(v1,v2,smoothstep(e1,e2,x_0))*step(e1,x_0)*step(x_0,e2);
  let a2 = mix(v2,v3,smoothstep(e2,e3,x_0))*step(e2,x_0)*step(x_0,e3);
  let a3 = mix(v3,v4,smoothstep(e3,e4,x_0))*step(e3,x_0)*step(x_0,e4);
  let a4 = mix(v4,v5,smoothstep(e4,e5,x_0))*step(e4,x_0)*step(x_0,e5);
  let a5 = mix(v5,v6,smoothstep(e5,e6,x_0))*step(e5,x_0)*step(x_0,e6);
  let a6 = mix(v6,v7,smoothstep(e6,e7,x_0))*step(e6,x_0)*step(x_0,e7);
  let a7 = mix(v7,v8,smoothstep(e7,e8,x_0))*step(e7,x_0)*step(x_0,e8);
  return
    max(max(max(a0, a1), max(a2, a3)), max(max(a4, a5), max(a6, a7)));
}

// probably use this instead:
// https://www.shadertoy.com/view/XtGGzG

// from https://gist.github.com/mikhailov-work/0d177465a8151eb6ede1768d51d476c7
fn TurboColormap(value: f32) -> vec3<f32>{
  let kRedVec4 = vec4<f32>(0.13572138, 4.61539260, -42.66032258, 132.13108234);
  let kGreenVec4 = vec4<f32>(0.09140261, 2.19418839, 4.84296658, -14.18503333);
  let kBlueVec4 = vec4<f32>(0.10667330, 12.64194608, -60.58204836, 110.36276771);
  let kRedVec2 = vec2<f32>(-152.94239396, 59.28637943);
  let kGreenVec2 = vec2<f32>(4.27729857, 2.82956604);
  let kBlueVec2 = vec2<f32>(-89.90310912, 27.34824973);
  
  let x = clamp(value, 0.0, 1.0);

  let v4 = vec4<f32>( 1.0, x, x * x, x * x * x);
  let v2 = v4.zw * v4.z;
  return vec3<f32>(
    dot(v4, kRedVec4)   + dot(v2, kRedVec2),
    dot(v4, kGreenVec4) + dot(v2, kGreenVec2),
    dot(v4, kBlueVec4)  + dot(v2, kBlueVec2)
  );
}

// Watson et al approximate the distance on the retina to degrees conversion as a linear equation
// Since we do not have the distance in mm from the center but in [0; 0.5], 
// and the lens maps the image space to +/- 90°, we can map 90 deg to 1.0 linear
fn dist_to_deg(dist: f32) -> f32{
    return 2.*dist*90.;
}
fn deg_to_dist(deg: f32) -> f32{
    return deg/(90.*2.);
}

// depending on which principal meridians enclose the point, we need to calculate with different values for RGCf density
// finding the meridians is a bit tricky for the individual eyes, so i did some "art"
/*
                                 ┌─────┐
               2         eye:l   │     │   eye:r          2
            superior             │     │               superior
               │                 │     │                  │
               │                 │     │                  │
    temporal   │                 │     │                  │     temporal
1  ────────────┼───────────── 3  │     │   3  ────────────┼───────────── 1
               │      nasal      │     │       nasal      │
               │                 │     │                  │
               │                 │     │                  │
           inferior              │     │              inferior
                                 └─────┘
               4                  nose                    4
*/

fn watsons_params_x(x: f32) -> vec3<f32>{
    var x = x;
    // flip the x axis if we are in the left eye
    if( uniforms.flow_idx > 0 ){
        x *= -1.0;
    }
    if(x <= 0.){
        // temporal 1
        return vec3<f32>(0.9851, 1.058,  22.14);
    }else{
        // nasal 3
        return vec3<f32>(0.9729, 1.084,  7.633);
    }
}

fn watsons_params_y(y: f32) -> vec3<f32>{
    if(y >= 0.){
        // superior 2
        return vec3<f32>(0.9935, 1.035,  16.35);
    }else{
        // inferior 4
        return vec3<f32>(0.996,  0.9932, 12.13);
    }
}

const max_d_rgcf = 33163.2;

fn weighted_density_meridian(x: f32, r_xy: f32, params: vec3<f32>) -> f32{
    let rel_density =  params.x * pow(1.0+(r_xy/params.y),-2.0) + (1.0-params.x) * exp(-1.0 * r_xy/params.z);
    let weighted_density = pow(x, 2.0) * pow(rel_density, 2.0);
    return weighted_density;
}

// calculate the spacing between RGCf
fn spacing(pos: vec2<f32>) -> f32{
    // fix the aspect ratio
    var pos = vec2<f32>(pos.x * (uniforms.resolution_in.x/uniforms.resolution_in.y), pos.y);

    pos *= 2.0; //map to +/- 1.0 on the meridians
    // note that this does not create problems in the corners, 
    // since we use the iso-spacing ellipses from Watson to calculate the actual spacing

    let r_xy = dist_to_deg(sqrt(pos.x * pos.x + pos.y * pos.y));
    
    // lookup the parameters for the enclosing meridians
    let m_x_params = watsons_params_x(pos.x);
    let m_y_params = watsons_params_y(pos.y);
    let x = dist_to_deg(pos.x);
    let y = dist_to_deg(pos.y);

    // get the weighted densities at the meridians
    let wdx = weighted_density_meridian(abs(x), r_xy, m_x_params);
    let wdy = weighted_density_meridian(abs(y), r_xy, m_y_params);

    // calculate the density from the two components
    let density = max_d_rgcf * (1.0/r_xy) * sqrt( wdx + wdy );

    var spacing = sqrt(1.0/(density));
    spacing = deg_to_dist(spacing);

    /*
    // uncomment this if you do not want to simulate ganglion cells smaller than a pixel 
    // This has some severe implications in the periphery: a RGCf that would encompass a 8x8 px field 
    // and hence could be correctly represented, is scaled unnecessarily
    float desired_res = sqrt(max_d_rgcf)*90;
    float available_res = min(u_resolution_in.x, u_resolution_in.y);
    float scale_factor = desired_res/available_res;
    spacing *= scale_factor;
    // spacing /= u_heat_scale;
    */
    return spacing;
}

fn HiveFive(color: vec3<f32>, tex: vec2<f32>) -> vec3<f32>{
    var self_pos = tex;
    var out_color = color;
    // return abs(u_hive_rotation[1].rgb);
    let aspect = uniforms.resolution_in.x/uniforms.resolution_in.y;    
    var hl_pos = vec3<f32>(uniforms.hive_position, 0.0);
    hl_pos = vec3<f32>(hl_pos.x * aspect, 1.0-hl_pos.y, 0.0);

    self_pos.x *= aspect;
    var bee = vec3<f32>(0.0, 0.0, 0.0);
    let hive_radius = 0.05;
    let bee_radius = 0.01;

    for (var i=0.; i<8.; i+=1.0){
        let offset = vec4<f32>( (i % 2.) - .5, (floor(i/2.) % 2.) - .5, floor(i/4.) - .5, 1.0);
        bee = hl_pos + (uniforms.hive_rotation*(offset*hive_radius)).rgb;
        if( length(bee.xy - self_pos) < bee_radius ){
            out_color += smoothstep(bee_radius, 0.0, length(bee.xy - self_pos)) * vec3<f32>(1.0,1.0,0.0);
        }   
    }   
    return out_color;
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

@group(2) @binding(0)
var in_original_t: texture_2d<f32>;
@group(2) @binding(1)
var in_original_s: sampler;

fn RetinalGanglion(tex: vec2<f32>) -> vec3<f32>{
    // want to see the density? use this!
    //return vec3<f32>(TurboColormap(16*spacing(tex-0.5)));

    var len = 0.5;
    var center = vec2<f32>(0.5);
    
    // subdivide until we have the required granularity
    for (var i:i32 = 0; i < 16; i++) {  
        // 2*len, since len is radius instead of diameter (=spacing)
        if(2.*len <= spacing(center - 0.5) ){  
            break;
        } 
        len/=2.0;

        // select the center of the new square, such that it contains the actual position
        center = center + step(center,tex)*len - step(tex,center)*len;
    }
    
    // make sure we sample at least one pixel around the own one
    let sample_distance = max(len, 1.0/min(uniforms.resolution_in.x, uniforms.resolution_in.y) );

    let gray_own = getPerceivedBrightness(textureSample(in_color_t, in_color_s, center).rgb);
    var gray_others = 0.0;

    // sample 8 pixels with the distance calculated above
    for (var i:i32 = -1; i <= 1; i++) {
        for (var j:i32 = -1; j <= 1; j++) {
            if(i != 0 || j != 0){ // exclude i=j=0
                gray_others += getPerceivedBrightness(textureSampleLevel(in_color_t, in_color_s, center + vec2<f32>(f32(i), f32(j)) * sample_distance, 0.0).rgb);
            }
        }
    }
    gray_others /= 8.0;

    let lum_diff = gray_own - gray_others;

    let threshold = 0.01;
    if( lum_diff > threshold){  // ON center
        return vec3<f32>(1.0,0.0,0.0) * 8.0 * ( lum_diff );
    }
    else if( lum_diff < (-1.0 * threshold) ){ // OFF center
        return vec3<f32>(0.0,0.0,1.0) * 8.0 * abs( lum_diff );
    }
    else{
        return vec3<f32>(0.0, 0.0, 0.0);
    }
}

@fragment
fn fs_main(in: VertexOutput) -> FragmentOutput {
    var out: FragmentOutput;

    var base_color = vec3<f32>(0.0, 0.0, 0.0);
    var cm_color = vec3<f32>(0.0, 0.0, 0.0);
    var color = vec3<f32>(0.0, 0.0, 0.0);

    var value = 0.0;

    // xy error and sd are tiny compared to their rgb counterparts. We should normalize them to make the featues at least visible
    let max_e_xy = 200.0/1080.0; // expermimental value, looks good, dont ask
    let max_u_2_xy = 20.0/1080.0; // expermimental value, looks good, dont ask


    switch (uniforms.base_image) {
        case 0:{ // simulated image
            color = textureSampleLevel(in_color_t, in_color_s, in.tex_coords, 0.0).rgb;
        }
        case 1:{ // original image
            color = textureSampleLevel(in_original_t, in_original_s, in.tex_coords, 0.0).rgb;
        }
        case 2:{
            color = RetinalGanglion(in.tex_coords);
        }
        case 3:{
            color = textureSampleLevel(in_color_t, in_color_s, in.tex_coords, 0.0).rgb;
        }
        default:{
            // leave default value
        }
    }

    if (uniforms.mix_type != 0){ // -- only calculate colormaps if they may be used

        switch (uniforms.combination_function){
            case 0:{ // Length of RGB error
                let e_rgb_2 = pow(textureSampleLevel(in_color_change_t, in_color_change_s, in.tex_coords, 0.0).rgb, vec3<f32>(2.0));
                value = sqrt( e_rgb_2.r + e_rgb_2.g + e_rgb_2.b);
            }
            case 1:{ // Length of XY error
                let e_xy_2 = pow(textureSampleLevel(in_deflection_t, in_deflection_s, in.tex_coords, 0.0).xy / max_e_xy, vec2<f32>(2.0)) ; 
                value = sqrt( e_xy_2.x + e_xy_2.y );
            }
            case 2:{ // Length of RGBXY error
                let e_5_rgb_2 = pow(textureSampleLevel(in_color_change_t, in_color_change_s, in.tex_coords, 0.0).rgb, vec3<f32>(2.0));
                let e_5_xy_2 = pow(textureSampleLevel(in_deflection_t, in_deflection_s, in.tex_coords, 0.0).xy / max_e_xy, vec2<f32>(2.0));
                value = sqrt( e_5_rgb_2.r + e_5_rgb_2.g + e_5_rgb_2.b + e_5_xy_2.x + e_5_xy_2.y );
            }
            case 3:{ // Length of RGB uncertainty
                let u_rgb = textureSampleLevel(in_color_uncertainty_t, in_color_uncertainty_s, in.tex_coords, 0.0).rgb;
                value = sqrt( u_rgb.r + u_rgb.g + u_rgb.b);
            }
            case 4:{ // Length of XY uncertainty
                let u_xy = textureSampleLevel(in_deflection_t, in_deflection_s, in.tex_coords, 0.0).ba / max_u_2_xy;
                value = sqrt( u_xy.x + u_xy.y );
            }
            case 5:{ // Length of RGBXY uncertainty
                let u_5_rgb = textureSampleLevel(in_color_uncertainty_t, in_color_uncertainty_s, in.tex_coords, 0.0).rgb;
                let u_5_xy = textureSampleLevel(in_deflection_t, in_deflection_s, in.tex_coords, 0.0).ba / max_u_2_xy;
                value = sqrt(u_5_rgb.r + u_5_rgb.g + u_5_rgb.b + u_5_xy.x + u_5_xy.y );
            }
            case 6:{ // GenvarRGB
                let cvm_var = textureSampleLevel(in_color_uncertainty_t, in_color_uncertainty_s, in.tex_coords, 0.0).rgb;
                let cvm_covar = textureSampleLevel(in_covariances_t, in_covariances_s, in.tex_coords, 0.0).rgb;
                let cvm = mat3x3<f32>(
                    vec3<f32>(cvm_var.r, cvm_covar.r, cvm_covar.g), 
                    vec3<f32>(cvm_covar.r, cvm_var.g, cvm_covar.b),
                    vec3<f32>(cvm_covar.g, cvm_covar.b, cvm_var.b)
                );
                value = pow(abs(determinant(cvm)), 1.0/6.0) * 3.0;
            }
            default:{}
        }

        value *= uniforms.heat_scale;

        switch (uniforms.colormap_type) {
            case 0:{ // Viridis
                cm_color = ViridisColormap(value);
            }
            case 1:{ // Turbo
                cm_color = TurboColormap(value);
            }
            case 2:{ // Grayscale, e.g. for external normalization
                cm_color = vec3(value);
            }
            default:{}
        }
    }

    switch (uniforms.mix_type) {
        case 0:{ // keep base image            
        }
        case 1:{ // vis_only
            color = cm_color;
        }
        case 2:{
            if (value > 0.25) {
                color = cm_color;
            }
        }
        default:{}
    }

    if (uniforms.hive_visible == 1) {
        color = HiveFive(color, in.tex_coords);
    }
    
    out.color = vec4<f32>(color, 1.0);
    return out;
}