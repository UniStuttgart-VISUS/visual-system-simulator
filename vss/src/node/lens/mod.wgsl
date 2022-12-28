// plane in front of the eye
let TEXTURE_SCALE: f32 = 2.0;

let CORNEA_MAP_FACTOR: f32 = 0.2;

struct Uniforms {
    lens_position: vec2<f32>,

    is_active: i32,
    samplecount: i32,
    depth_min: f32,
    depth_max: f32,

    // smallest distance on which the eye can focus, in mm
    near_point: f32,

    // largest  distance on which the eye can focus, in mm
    far_point: f32,

    // determines the bluriness of objects that are too close to focus
    // should be between 0 and 2
    near_vision_factor: f32,

    // determines the bluriness of objects that are too far to focus
    // should be between 0 and 2
    far_vision_factor: f32,

    dir_calc_scale: f32,
    astigmatism_ecc_mm: f32,
    astigmatism_angle_deg: f32,
    eye_distance_center: f32,
    track_error: i32,

    _padding: i32,
};

struct Simulation {
  color: vec4<f32>,
  color_change: vec4<f32>,
  color_uncertainty: vec4<f32>,
  color_covariance: vec4<f32>,
  dir_unc: vec2<f32>,
  dir_covar: f32,
  target_location: vec2<f32>,
};

struct RefractInfo {
    position: vec3<f32>,
    normal: vec3<f32>,
};

struct FragmentOutput {
    @location(0) color: vec4<f32>,
    @location(1) deflection: vec4<f32>,
    @location(2) color_change: vec4<f32>,
    @location(3) color_uncertainty: vec4<f32>,
    @location(4) covariances: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

@group(1) @binding(0)
var in_color_s: sampler;
@group(1) @binding(1)
var in_color_t: texture_2d<f32>;
@group(1) @binding(2)
var in_depth_s: sampler;
@group(1) @binding(3)
var in_depth_t: texture_2d<f32>;
@group(1) @binding(4)
var in_deflection_s: sampler;
@group(1) @binding(5)
var in_deflection_t: texture_2d<f32>;
@group(1) @binding(6)
var in_color_change_s: sampler;
@group(1) @binding(7)
var in_color_change_t: texture_2d<f32>;
@group(1) @binding(8)
var in_color_uncertainty_s: sampler;
@group(1) @binding(9)
var in_color_uncertainty_t: texture_2d<f32>;
@group(1) @binding(10)
var in_covariances_s: sampler;
@group(1) @binding(11)
var in_covariances_t: texture_2d<f32>;

@group(2) @binding(0)
var in_normal_t: texture_2d<f32>;
@group(2) @binding(1)
var in_normal_s: sampler;

@group(3) @binding(0)
var in_cornea_t: texture_2d<f32>;
@group(3) @binding(1)
var in_cornea_s: sampler;

// alters the direction of a ray according to the imperfection in the cornea specified by the cornea map
fn applyCorneaImperfection(pos: vec3<f32>, dir: vec3<f32>) -> vec3<f32> {
    // scale 3D ray to 2D texture coordinates for lookup
    // The size of the cornea (~5.5mm radius) is mapped to [0;1],[0;1] texture coordinates
    let corneaMapSample = textureSample(in_color_t, in_color_s, pos.xy * vec2<f32>(0.17, -0.17) + 0.5).rg;
    let deviation = vec3<f32>((corneaMapSample - 0.5) * vec2<f32>(CORNEA_MAP_FACTOR), 0.0);
    return dir + deviation;
}

fn getDepth(pos: vec2<f32>) -> f32 {
    return uniforms.depth_max - textureSample(in_depth_t, in_depth_s, pos).r * (uniforms.depth_max - uniforms.depth_min);
}

fn getTargetLocation(start: vec3<f32>, dir: vec3<f32>, v_tex: vec2<f32>) -> vec3<f32> {
    //TODO why is the resolution hardcoded ?
    let t = (getDepth(v_tex+(uniforms.lens_position/(vec2<f32>(1920.0, 1080.0) / 2.0))) + VITREOUS_HUMOUR_RADIUS - start.z) / dir.z;
    return start + dir * t;
}

// intesects the ray with the image and returns the color of the texture at this position
fn getColorWithRay(target_location: vec3<f32>) -> vec4<f32> {
    return textureSample(in_color_t, in_color_s, target_location.xy / ((target_location.z - VITREOUS_HUMOUR_RADIUS) * TEXTURE_SCALE) + 0.5);
}

fn rayIntersectEllipsoid(rPos: vec3<f32>, rDir: vec3<f32>, cPos: vec3<f32>, cRad: f32) -> RefractInfo {

    let mul = radians(uniforms.astigmatism_angle_deg);
    // float mul = 0.25 * 0;

    // We start off by defining the three main axis of the ellispoid.
    var a = vec3<f32>(cRad - 0.094, 0.0, 0.0); //experimental value, remove when cubemap works
    var b = vec3<f32>(0.0, cRad - 0.078 - uniforms.astigmatism_ecc_mm, 0.0); //experimental value, remove when cubemap works
    var c = vec3<f32>(0.0, 0.0, cRad);

    // Now we allow for rotation along the Z axis (viewing direction)
    // This enables us to simulate Atigmatism obliquus, with the main axis not in line with world-coordinates

    // The matrix for rotating the cornea elipsoid
    let rot = mat3x3<f32>(
        vec3<f32>(cos(mul), sin(mul), 0.0),
        vec3<f32>(-sin(mul), cos(mul), 0.0),
        vec3<f32>(0.0, 0.0, 1.0)
    );

    // rotate the ellispoid (main axis)
    a = rot * a;
    b = rot * b;
    c = rot * c;

    // Now that the ellipsoid is defined, we prepare to transform the whole space, so that the ellipsoid
    // becomes a up-oriented-unit-sphere (|a|=|b|=|c| ; a = a.x, r=1), and center in the origin.
    // inspired by http://www.illusioncatalyst.com/notes_files/mathematics/line_nu_sphere_intersection.php



    let T = mat4x4<f32>(
        vec4<f32>(1.0, 0.0, 0.0, 0.0),
        vec4<f32>(0.0, 1.0, 0.0, 0.0),
        vec4<f32>(0.0, 0.0, 1.0, 0.0),
        vec4<f32>(cPos.x, cPos.y, cPos.z, 1.0)
    );
    let T_inv = mat4x4<f32>(
        vec4<f32>(1.0, 0.0, 0.0, 0.0),
        vec4<f32>(0.0, 1.0, 0.0, 0.0),
        vec4<f32>(0.0, 0.0, 1.0, 0.0),
        vec4<f32>(-cPos.x, -cPos.y, -cPos.z, 1.0)
    );

    let an = normalize(a);
    let bn = normalize(b);
    let cn = normalize(c);

    let R = mat4x4<f32>(
        vec4<f32>(an.x, an.y, an.z, 0.0),
        vec4<f32>(bn.x, bn.y, bn.z, 0.0),
        vec4<f32>(cn.x, cn.y, cn.z, 0.0),
        vec4<f32>( 0.0,  0.0,  0.0, 1.0)
    );

    let R_d = an.y * bn.z * cn.x - an.x * bn.z * cn.y + an.z * bn.x * cn.y + cn.z * (bn.y * an.x - an.y * bn.x) - bn.y * an.z * cn.x;
    let R_inv = mat4x4<f32>(
        vec4<f32>(  -(  bn.z * cn.y - bn.y * cn.z) / R_d,
                    (- an.y * cn.z + an.z * cn.y) / R_d,
                    -(- an.y * bn.z + bn.y * an.z) / R_d,
                    0.0),
        vec4<f32>(   (bn.z * cn.x - bn.x * cn.z) / R_d,
                    (an.x * cn.z - an.z * cn.x) / R_d,
                    -(an.x * bn.z - an.z * bn.x) / R_d,
                    0.0),
        vec4<f32>( (- bn.y * cn.x + bn.x * cn.y) / R_d,
                    -(an.x * cn.y - an.y * cn.x) / R_d,
                    (bn.y * an.x - an.y * bn.x) / R_d,
                    0.0),
        vec4<f32>(0.0, 0.0, 0.0, 1.0)
    );

    let S = mat4x4<f32>(
        vec4<f32>(length(a), 0.0, 0.0, 0.0),
        vec4<f32>(0.0, length(b), 0.0, 0.0),
        vec4<f32>(0.0, 0.0, length(c), 0.0),
        vec4<f32>(0.0, 0.0, 0.0, 1.0)
    );
    let S_inv = mat4x4<f32>(
        vec4<f32>(1.0 / length(a), 0.0, 0.0, 0.0),
        vec4<f32>(0.0, 1.0 / length(b), 0.0, 0.0),
        vec4<f32>(0.0, 0.0, 1.0 / length(c), 0.0),
        vec4<f32>(0.0, 0.0, 0.0, 1.0)
    );

    // Transform the Ray according to the inverse transformation, that once created the ellipsoid from the sphere
    // We do **not** translate the direction component of the ray, since it would change the direction of the ray and hence the resulting normal
    let srPos =  S_inv * T_inv * R_inv * vec4<f32>(rPos, 1.0);
    let srDir =  S_inv * R_inv * vec4<f32>(rDir, 1.0);
        
    // Since we transformed everything, sphere position is now the origin and its radius is 1
    let sTarget = rayIntersectSphere(srPos.xyz, srDir.xyz, vec3<f32>(0.0), 1.0);
    
    // We can gradually transform everything back to its original state.
    // The first step is scaling the space to once again get the original ellipsoid shape
    // This is used for calculating the normal. However, the normal caluclation assumes a,b,c, to be aligned with
    //   the coordinate axis, so apply the rotation to the normal instead
    let rel_target = S * vec4<f32>(sTarget, 1.0);

    // to calculate the normal, we use n = x_0/a^2 , y_0/b^2, z_0/c^2 with the ellipsoid centered at the origin
    var normal = vec4<f32>(rel_target.x / pow(length(a), 2.0), rel_target.y / pow(length(b), 2.0), rel_target.z / pow(length(c), 2.0), 1.0);
    // restore the original ellipsoid rotation
    normal = R * normal;

    // the intersection point of the ray with the ellipsoid. All transformations applied initially to the ray-position
    // are now applied as inverted matrices (so the original ones, that created the ellipsoid) and in inverse order.
    let world_target = (R*(T*(S * vec4<f32>(sTarget, 1.0)))).rgb;

    return RefractInfo(
        world_target.xyz,
        normal.xyz
    );
}

// sends the ray through all four surfaces in the optical system and returns the color, position, error and uncertainty
// of the texture at the intersection point.
fn getColorSample(original_start: vec3<f32>, aim: vec2<f32>, focalLength: f32, nAnteriorChamberFactor: f32, v_tex: vec2<f32>) -> Simulation {
    var start = original_start;
    var dir = normalize(vec3<f32>(aim, VITREOUS_HUMOUR_RADIUS) - start);

    start = rayIntersectSphere(start, dir, getInnerLensCenter(focalLength), getInnerLensRadius(focalLength));
    dir = refract(dir, normalize(start - getInnerLensCenter(focalLength)), N_VITREOUS_HUMOUR / N_LENS);

    start = rayIntersectSphere(start, dir, getOuterLensCenter(focalLength), getOuterLensRadius(focalLength));
    dir = refract(dir, -normalize(start - getOuterLensCenter(focalLength)), N_LENS / getNAnteriorChamber(nAnteriorChamberFactor));

    start = rayIntersectSphere(start, dir, INNER_CORNEA_CENTER, INNER_CORNEA_RADIUS);
    dir = refract(dir, -normalize(start - INNER_CORNEA_CENTER), getNAnteriorChamber(nAnteriorChamberFactor) / N_CORNEA);

    let ri = rayIntersectEllipsoid(start, dir, OUTER_CORNEA_CENTER, OUTER_CORNEA_RADIUS);
    start = ri.position;

    // we modeled the cornea as an ellipsoid instead of a sphere.
    // although the lens can also contribute to a asigmatism, the result is the same, as if the cornea would introduce all the error 
    // start = rayIntersectSphere(start, dir, OUTER_CORNEA_CENTER, OUTER_CORNEA_RADIUS);


    dir = refract(dir, -normalize(ri.normal), N_CORNEA / N_AIR);
    // dir = refract(dir, -normalize(start - OUTER_CORNEA_CENTER), N_CORNEA / N_AIR);
    dir = applyCorneaImperfection(start, dir);

    start += vec3<f32>(uniforms.lens_position, 0.0) + vec3<f32>(uniforms.eye_distance_center, 0.0, 0.0);

    // position where the ray hits the image, including depth
    let target_location = getTargetLocation(start, dir, v_tex);

    // The color and the error values are sampled from the original textures
    let color = getColorWithRay(target_location);

    if(uniforms.track_error == 1){
        let sample_pos = target_location.xy / ((target_location.z - VITREOUS_HUMOUR_RADIUS) * TEXTURE_SCALE) + 0.5;
        let color_change = textureSample(in_color_change_t, in_color_change_s, sample_pos);
        let color_uncertainty = textureSample(in_color_uncertainty_t, in_color_uncertainty_s, sample_pos);
        let color_covariance = textureSample(in_covariances_t, in_covariances_s, sample_pos);
        let deflection = textureSample(in_deflection_t, in_deflection_s, sample_pos);
        let dir_change = deflection.rg;
        let dir_unc = deflection.ba;
        let dir_covar = color_covariance.a;
        return Simulation(
            color,
            color_change,
            color_uncertainty,
            color_covariance,
            dir_unc,
            dir_covar,
            target_location.xy / target_location.z
            // target.xy / ((target.z - VITREOUS_HUMOUR_RADIUS) * TEXTURE_SCALE) + 0.5
            );
    }
    else{
        return Simulation(
            color,
            vec4<f32>(0.0),
            vec4<f32>(0.0),
            vec4<f32>(0.0),
            vec2<f32>(0.0),
            0.0,
            target_location.xy / target_location.z
            // target.xy / ((target.z - VITREOUS_HUMOUR_RADIUS) * TEXTURE_SCALE) + 0.5
            );
    }
}

@fragment
fn fs_main(in: VertexOutput) -> FragmentOutput {
    var out: FragmentOutput;

    if (1 == uniforms.is_active) {
        out.color = vec4<f32>(0.5);
        let start = (textureSample(in_normal_t, in_normal_s, in.tex_coords)).xyz * VITREOUS_HUMOUR_RADIUS;
        let original_color = textureSample(in_color_t, in_color_s, in.tex_coords).rgb;

        var sampleCount = 0.0;
        var color = vec4<f32>(0.0);
        var avg_target = vec2<f32>(0.0);

        // prepare accomodation
        // This implementation focuses on all possible depths: for each pixel, the lens can focus on the individual depth
        var focalLength = length(vec3<f32>(
            (in.tex_coords.xy - 0.5) * TEXTURE_SCALE,
            uniforms.depth_max - textureSample(in_depth_t, in_depth_s, in.tex_coords).r * (uniforms.depth_max - uniforms.depth_min)));     
       
        // This implementation focuses soley on the depth that is in the middle of the screen
        // float focalLength = length(
        //     vec3<f32>(
        //         (in.tex_coords.xy - 0.5) * TEXTURE_SCALE,
        //         uniforms.depth_max - texture(s_depth, vec2<f32>(0.5,0.5)).r * (uniforms.depth_max - uniforms.depth_min)
        //     )
        // );

        var nAnteriorChamberFactor = 1.0;
        if (focalLength > uniforms.far_point) {
            // TODO factor 0.08
            nAnteriorChamberFactor = 1.0 / pow((focalLength - uniforms.far_point) / focalLength + 1.0, 0.08 * uniforms.far_vision_factor);
            focalLength = uniforms.far_point;
        } else if (focalLength < uniforms.near_point) {
            // TODO factor 0.12
            nAnteriorChamberFactor = pow(focalLength / uniforms.near_point, 0.12 * uniforms.near_vision_factor);
            focalLength = uniforms.near_point;
        }

        var rays: array<Simulation,17>;

        // The higher the sampleCount, the more rays are cast. Rays are cast in this fashion,
        // where the number indicates the minimum sampleCount required for this ray to be cast:
        // 3   2   3
        //   4 4 4
        // 2 4 1 4 2
        //   4 4 4
        // 3   2   3
        // Since the ray direction is only altered a small amount compared to the radius, 
        //     it is safe to assume that all of them will still hit the lens.
        switch (uniforms.samplecount) {
            case 4:{
                sampleCount += 8.0;
                rays[0] = getColorSample(start, vec2<f32>(-0.5, -0.5), focalLength, nAnteriorChamberFactor, in.tex_coords);
                rays[1] = getColorSample(start, vec2<f32>(-0.5, 0.5), focalLength, nAnteriorChamberFactor, in.tex_coords);
                rays[2] = getColorSample(start, vec2<f32>(0.5, -0.5), focalLength, nAnteriorChamberFactor, in.tex_coords);
                rays[3] = getColorSample(start, vec2<f32>(0.5, 0.5), focalLength, nAnteriorChamberFactor, in.tex_coords);
                rays[4] = getColorSample(start, vec2<f32>(-0.5, 0.0), focalLength, nAnteriorChamberFactor, in.tex_coords);
                rays[5] = getColorSample(start, vec2<f32>(0.5, 0.0), focalLength, nAnteriorChamberFactor, in.tex_coords);
                rays[6] = getColorSample(start, vec2<f32>(0.0, -0.5), focalLength, nAnteriorChamberFactor, in.tex_coords);
                rays[7] = getColorSample(start, vec2<f32>(0.0, 0.5), focalLength, nAnteriorChamberFactor, in.tex_coords);
            }
            case 3:{
                sampleCount += 4.0;
                rays[8] = getColorSample(start, vec2<f32>(-1.0, -1.0), focalLength, nAnteriorChamberFactor, in.tex_coords);
                rays[9] = getColorSample(start, vec2<f32>(-1.0, 1.0), focalLength, nAnteriorChamberFactor, in.tex_coords);
                rays[10] = getColorSample(start, vec2<f32>(1.0, -1.0), focalLength, nAnteriorChamberFactor, in.tex_coords);
                rays[11] = getColorSample(start, vec2<f32>(1.0, 1.0), focalLength, nAnteriorChamberFactor, in.tex_coords);
            }
            case 2:{
                sampleCount += 4.0;
                rays[12] = getColorSample(start, vec2<f32>(-1.0, 0.0), focalLength, nAnteriorChamberFactor, in.tex_coords);
                rays[13] = getColorSample(start, vec2<f32>(1.0, 0.0), focalLength, nAnteriorChamberFactor, in.tex_coords);
                rays[14] = getColorSample(start, vec2<f32>(0.0, -1.0), focalLength, nAnteriorChamberFactor, in.tex_coords);
                rays[15] = getColorSample(start, vec2<f32>(0.0, 1.0), focalLength, nAnteriorChamberFactor, in.tex_coords);
            }
            case 1:{
                sampleCount += 1.0;
                rays[16] = getColorSample(start, vec2<f32>(0.0, 0.0), focalLength, nAnteriorChamberFactor, in.tex_coords);
            }
            default:{
                // leave default value
            }
        }

        // calculate the mean color and target vector
        // Thie mean target vector is the "center" of the scatter
        for (var i: i32 = 0; i < 17; i++) {
            avg_target += rays[i].target_location;
            color+=rays[i].color;
        }
        avg_target /= sampleCount;
        color /= sampleCount;

        if(uniforms.track_error==1){
            var dir_diff = vec2<f32>(0.0);
            var col_diff = vec3<f32>(0.0);

            var col_unc_new = vec3<f32>(0.0);
            var col_covar_new = vec3<f32>(0.0);
            var dir_unc_new = vec2<f32>(0.0);
            var dir_covar_new = 0.0;

            var col_unc_prop = vec3<f32>(0.0);
            var col_covar_prop = vec3<f32>(0.0);
            var dir_unc_prop = vec2<f32>(0.0);
            var dir_covar_prop = 0.0;

            for (var i: i32 = 0; i < 17; i++) {
                dir_diff = rays[i].target_location - avg_target;
                col_diff = rays[i].color.rgb - color.rgb;

                col_unc_new += pow(col_diff, vec3<f32>(2.0));
                col_covar_new += vec3<f32>(col_diff.r*col_diff.g, col_diff.r*col_diff.b, col_diff.g*col_diff.b);
                dir_unc_new += pow(dir_diff , vec2<f32>(2.0));
                dir_covar_new += dir_diff.x * dir_diff.y;

                col_unc_prop += rays[i].color_uncertainty.rgb;
                col_covar_prop += rays[i].color_covariance.rgb;
                dir_unc_prop += rays[i].dir_unc;
                dir_covar_prop += rays[i].color_covariance.a;

            }

            col_unc_new /= sampleCount;
            col_covar_new /= sampleCount;
            dir_unc_new /= sampleCount;
            dir_covar_new /= sampleCount;

            col_unc_prop /= sampleCount;        
            col_covar_prop /= sampleCount;        
            dir_unc_prop /= sampleCount;
            dir_covar_prop /= sampleCount;


            let col_var = col_unc_prop + col_unc_new;
            let col_covar = col_covar_prop + col_covar_new;
            let dir_var = dir_unc_prop + dir_unc_new;
            let dir_covar = dir_covar_prop + dir_covar_new;

            // the final color is the average of all samples (calculated above)
            out.color = color;
            
            let color_diff = out.color.rgb - original_color;
            let deflection = (in.tex_coords.xy - 0.5) * TEXTURE_SCALE - avg_target;

            out.color_change =      vec4<f32>(textureSample(in_color_change_t, in_color_change_s, in.tex_coords).rgb + color_diff,0.0);
            out.color_uncertainty = vec4<f32>(col_var, 0.0);
            out.deflection =        vec4<f32>(textureSample(in_deflection_t, in_deflection_s, in.tex_coords).rg + deflection, dir_var);
            out.covariances =       vec4<f32>(col_covar, dir_covar);
        } else {
            // quite easy, huh?
            out.color = color;
        }
    } else {
        out.color = textureSample(in_color_t, in_color_s, in.tex_coords);

        if(uniforms.track_error==1){
            out.color_change =      textureSample(in_color_change_t, in_color_change_s, in.tex_coords);
            out.color_uncertainty = textureSample(in_color_uncertainty_t, in_color_uncertainty_s, in.tex_coords);
            out.deflection =        textureSample(in_deflection_t, in_deflection_s, in.tex_coords);
            out.covariances =       textureSample(in_covariances_t, in_covariances_s, in.tex_coords);
        }
    }

    // out.color = abs(textureSample(in_normal_t, in_normal_s, in.tex_coords));
    return out;
}
