// plane in front of the eye
#define TEXTURE_SCALE 2.0

// This model was obtained via machine learning, using a
// particle swarm optimisation algorithm. The cost gives a
// rough estimate for the quality of the model (the lower
// the cost, the better).
// Cost:  0.11083103563252572
#define N_AIR 1.0
#define N_CORNEA 1.64099928591927
#define N_ANTERIOR_CHAMBER 1.4532300393624265
#define N_LENS 1.4393602250536033
#define N_VITREOUS_HUMOUR 1.362681339093861

#define OUTER_CORNEA_RADIUS 7.873471062713062
#define INNER_CORNEA_RADIUS 4.7658074271207695
#define VITREOUS_HUMOUR_RADIUS 11.704040541034434

#define OUTER_CORNEA_CENTER vec3(0., 0., 6.673155500500702)
#define INNER_CORNEA_CENTER vec3(0., 0., 7.877094062114152)
#define VITREOUS_HUMOUR_CENTER vec3(0., 0., 0.0)

#define OUTER_LENS_RADIUS_A 5.0719602142230795E-11
#define OUTER_LENS_RADIUS_B -4.728025830060936E-6
#define OUTER_LENS_RADIUS_C 8.768369917723753
#define INNER_LENS_RADIUS_A 1.8872104415414247E-11
#define INNER_LENS_RADIUS_B 1.3008349405807643E-5
#define INNER_LENS_RADIUS_C 7.237406273489772

#define OUTER_LENS_TIP_A -3.867155451878399E-11
#define OUTER_LENS_TIP_B 1.5465450056679198E-7
#define OUTER_LENS_TIP_C 8.839055805878198
#define INNER_LENS_TIP_A -8.525281533727847E-12
#define INNER_LENS_TIP_B 3.777299048943033E-6
#define INNER_LENS_TIP_C 7.03207918882336
// end of model

#define CORNEA_MAP_FACTOR 0.2

uniform int u_active;
uniform int u_samplecount;
uniform float u_depth_min;
uniform float u_depth_max;
uniform float u_near_point;
uniform float u_far_point;
uniform float u_near_vision_factor;
uniform float u_far_vision_factor;
uniform float u_dir_calc_scale;

uniform sampler2D s_color;
uniform sampler2D s_normal;
uniform sampler2D s_depth;
uniform sampler2D s_cornea;
uniform sampler2D s_deflection;
uniform sampler2D s_color_change;
uniform sampler2D s_color_uncertainty;

in vec2 v_tex;
out vec4 rt_color;
out vec4 rt_deflection;
out vec4 rt_color_change;
out vec4 rt_color_uncertainty;

struct Simulation
{
  vec4 color;
  vec4 color_change;
  vec4 color_uncertainty;
  vec2 target;
};


// alters the direction of a ray according to the imperfection in the cornea specified by the cornea map
vec3 applyCorneaImperfection(vec3 pos, vec3 dir) {
    // scale 3D ray to 2D texture coordinates for lookup
    // The size of the cornea (~5.5mm radius) is mapped to [0;1],[0;1] texture coordinates
    vec2 corneaMapSample = texture(s_cornea, pos.xy * vec2(0.17, -0.17) + 0.5).rg;
    vec3 deviation = vec3((corneaMapSample - 0.5) * vec2(CORNEA_MAP_FACTOR), 0.0);
    return dir + deviation;
}

// calculates the radius of curvature of the outer lens surface
// while taking accomodation into account.
// focalLength: the distance between the frontmost point of the
//              vitreous humour and the focused point
float getOuterLensRadius(float focalLength) {
    return OUTER_LENS_RADIUS_A * focalLength * focalLength
        + OUTER_LENS_RADIUS_B * focalLength
        + OUTER_LENS_RADIUS_C;
}

// calculates the center of the sphere representing the outer
// lens surface while taking accomodation into account.
// focalLength: the distance between the frontmost point of the
//              vitreous humour and the focused point
vec3 getOuterLensCenter(float focalLength) {
    float outerLensTip = OUTER_LENS_TIP_A * focalLength * focalLength
        + OUTER_LENS_TIP_B * focalLength
        + OUTER_LENS_TIP_C;
    return vec3(0.0, 0.0, outerLensTip - getOuterLensRadius(focalLength));
}

// calculates the radius of curvature of the inner lens surface
// while taking accomodation into account.
// focalLength: the distance between the frontmost point of the
//              vitreous humour and the focused point
float getInnerLensRadius(float focalLength) {
    return INNER_LENS_RADIUS_A * focalLength * focalLength
        + INNER_LENS_RADIUS_B * focalLength
        + INNER_LENS_RADIUS_C;
}

// calculates the center of the sphere representing the inner
// lens surface while taking accomodation into account.
// focalLength: the distance between the frontmost point of the
//              vitreous humour and the focused point
vec3 getInnerLensCenter(float focalLength) {
    float innerLensTip = INNER_LENS_TIP_A * focalLength * focalLength
        + INNER_LENS_TIP_B * focalLength
        + INNER_LENS_TIP_C;
    return vec3(0.0, 0.0, innerLensTip + getInnerLensRadius(focalLength));
}

float getNAnteriorChamber(float nAnteriorChamberFactor) {
    return N_ANTERIOR_CHAMBER * nAnteriorChamberFactor;
}

float getDepth(vec2 pos) {
    return u_depth_max - texture(s_depth, pos).r * (u_depth_max - u_depth_min);
}

vec3 getTargetLocation(in vec3 start, in vec3 dir){
    float t = (getDepth(v_tex) + VITREOUS_HUMOUR_RADIUS - start.z) / dir.z;
    start += dir * t;
    return start;
}

// intesects the ray with the image and returns the color of the texture at this position
vec4 getColorWithRay(in vec3 target) {
    return texture(s_color, target.xy / ((target.z - VITREOUS_HUMOUR_RADIUS) * TEXTURE_SCALE) + 0.5);
}

//rPos = ray position
//rDir = ray direction
//cPos = sphere position
//cRad = sphere radius
// returns a point where the ray intersects the sphere
vec3 rayIntersectSphere(vec3 rPos, vec3 rDir, vec3 cPos, float cRad) {
    float a = dot(rDir, rDir);
    float b = 2.0 * dot(rDir, (rPos - cPos));
    float c = dot(rPos - cPos, rPos - cPos) - cRad * cRad;
    float disk = sqrt(b * b - 4.0 * a * c);
    float t1 = (-b + disk) / (2.0 * a);
    float t2 = (-b - disk) / (2.0 * a);

    if (t1 < t2) {
        return (t1 >= 0.0 ? t1 : t2) * rDir + rPos;
    } else {
        return (t2 >= 0.0 ? t2 : t1) * rDir + rPos;
    }
}

vec2 to_dir_angle(in vec3 dir){		
	vec2 pt;
    pt.x = atan(dir.x/dir.z)*u_dir_calc_scale;
    pt.y = atan(dir.y/dir.z)*u_dir_calc_scale;
    return pt;
}

// sends the ray through all four surfaces in the optical system and returns the color
// of the texture at the intersection point.
Simulation getColorSample(vec3 start, vec2 aim, float focalLength, float nAnteriorChamberFactor) {
    vec3 dir = normalize(vec3(aim, VITREOUS_HUMOUR_RADIUS) - start);

    start = rayIntersectSphere(start, dir, getInnerLensCenter(focalLength), getInnerLensRadius(focalLength));
    dir = refract(dir, normalize(start - getInnerLensCenter(focalLength)), N_VITREOUS_HUMOUR / N_LENS);

    start = rayIntersectSphere(start, dir, getOuterLensCenter(focalLength), getOuterLensRadius(focalLength));
    dir = refract(dir, -normalize(start - getOuterLensCenter(focalLength)), N_LENS / getNAnteriorChamber(nAnteriorChamberFactor));

    start = rayIntersectSphere(start, dir, INNER_CORNEA_CENTER, INNER_CORNEA_RADIUS);
    dir = refract(dir, -normalize(start - INNER_CORNEA_CENTER), getNAnteriorChamber(nAnteriorChamberFactor) / N_CORNEA);

    start = rayIntersectSphere(start, dir, OUTER_CORNEA_CENTER, OUTER_CORNEA_RADIUS);
    dir = refract(dir, -normalize(start - OUTER_CORNEA_CENTER), N_CORNEA / N_AIR);
    dir = applyCorneaImperfection(start, dir);

    // position where the ray hits the image, including depth
    vec3 target = getTargetLocation(start, dir);

    // The color and the error values are sampled from the original textures
    vec4 color = getColorWithRay(target);
    vec4 color_change = texture(s_color_change, target.xy / ((target.z - VITREOUS_HUMOUR_RADIUS) * TEXTURE_SCALE) + 0.5);
    vec4 color_uncertainty = texture(s_color_uncertainty, target.xy / ((target.z - VITREOUS_HUMOUR_RADIUS) * TEXTURE_SCALE) + 0.5);
    Simulation sim = Simulation(
        color,
        color_change,
        color_uncertainty,
        (target.xy/target.z)*u_dir_calc_scale);
    return sim;
}


void main() {

    if (1 == u_active) {
        rt_color = vec4(0.5);
        vec3 start = (texture(s_normal, v_tex) * 2.0 - 1.0).xyz * VITREOUS_HUMOUR_RADIUS;
        vec3 original_color = texture(s_color, v_tex).rgb;

        float sampleCount = 0.0;
        vec4 color = vec4(0.0);
   
        // // dir sd
        vec2 avg_target = vec2(0.0);
        vec2 var_target = vec2(0.0);
        // since we have only two dimensions, the covariance can be calculated as a single float
        float covar_target = 0.0;


        // prepare accomodation
        // This implementation focuses on all possible depths: for each pixel, the lens can focus on the individual depth
        // float focalLength = length(vec3(
        //     (v_tex.xy - 0.5) * TEXTURE_SCALE,
        //     u_depth_max - texture(s_depth, v_tex).r * (u_depth_max - u_depth_min)));     
       
        // This implementation focuses soley on the depth that is in the middle of the screen
        float focalLength = length(
            vec3(
                (v_tex.xy - 0.5) * TEXTURE_SCALE,
                u_depth_max - texture(s_depth, vec2(0.5,0.5)).r * (u_depth_max - u_depth_min)
            )
        );  


        float nAnteriorChamberFactor = 1.0;
        if (focalLength > u_far_point) {
            // TODO factor 0.08
            nAnteriorChamberFactor = 1.0 / pow((focalLength - u_far_point) / focalLength + 1.0, 0.08 * u_far_vision_factor);
            focalLength = u_far_point;
        } else if (focalLength < u_near_point) {
            // TODO factor 0.12
            nAnteriorChamberFactor = pow(focalLength / u_near_point, 0.12 * u_near_vision_factor);
            focalLength = u_near_point;
        }

        Simulation rays[17];

        // The higher the sampleCount, the more rays are cast. Rays are cast in this fashion,
        // where the number indicates the minimum sampleCount required for this ray to be cast:
        // 3   2   3
        //   4 4 4
        // 2 4 1 4 2
        //   4 4 4
        // 3   2   3
        // Since the ray direction is only altered a small amount compared to the radius, 
        //     it is safe to assume that all of them will still hit the lens.
        switch (u_samplecount) {
        case 4:
            sampleCount += 8.0;
            rays[0] = getColorSample(start, vec2(-0.5, -0.5), focalLength, nAnteriorChamberFactor);
            rays[1] = getColorSample(start, vec2(-0.5, 0.5), focalLength, nAnteriorChamberFactor);
            rays[2] = getColorSample(start, vec2(0.5, -0.5), focalLength, nAnteriorChamberFactor);
            rays[3] = getColorSample(start, vec2(0.5, 0.5), focalLength, nAnteriorChamberFactor);
            rays[4] = getColorSample(start, vec2(-0.5, 0.0), focalLength, nAnteriorChamberFactor);
            rays[5] = getColorSample(start, vec2(0.5, 0.0), focalLength, nAnteriorChamberFactor);
            rays[6] = getColorSample(start, vec2(0.0, -0.5), focalLength, nAnteriorChamberFactor);
            rays[7] = getColorSample(start, vec2(0.0, 0.5), focalLength, nAnteriorChamberFactor);
        case 3:
            sampleCount += 4.0;
            rays[8] = getColorSample(start, vec2(-1.0, -1.0), focalLength, nAnteriorChamberFactor);
            rays[9] = getColorSample(start, vec2(-1.0, 1.0), focalLength, nAnteriorChamberFactor);
            rays[10] = getColorSample(start, vec2(1.0, -1.0), focalLength, nAnteriorChamberFactor);
            rays[11] = getColorSample(start, vec2(1.0, 1.0), focalLength, nAnteriorChamberFactor);
        case 2:
            sampleCount += 4.0;
            rays[12] = getColorSample(start, vec2(-1.0, 0.0), focalLength, nAnteriorChamberFactor);
            rays[13] = getColorSample(start, vec2(1.0, 0.0), focalLength, nAnteriorChamberFactor);
            rays[14] = getColorSample(start, vec2(0.0, -1.0), focalLength, nAnteriorChamberFactor);
            rays[15] = getColorSample(start, vec2(0.0, 1.0), focalLength, nAnteriorChamberFactor);
        case 1:
            sampleCount += 1.0;
            rays[16] = getColorSample(start, vec2(0.0, 0.0), focalLength, nAnteriorChamberFactor);
        }

        // calculate the mean color and target vector
        // Thie mean target vector is the "center" of the scatter
        for (int i = 0; i < 17; i++) {
            avg_target += rays[i].target;
            color+=rays[i].color;
        }
        avg_target /= sampleCount;
        color /= sampleCount;

// TODO: deflection. unfortunately it cannot be conveyed in an independent color target
// TODO: Possibly reenable covariance
        vec2 diff_vec = vec2(0.0);
        vec3 col_unc_prop = vec3(0.0);
        vec3 col_unc_new = vec3(0.0);
        vec2 dir_unc_prop = vec2(0.0);
        
        for (int i = 0; i < 17; i++) {
            // express the difference of the current target vector and the average
            diff_vec = rays[i].target-avg_target;

            // calculate the variance of the target scatters for each dimension    
            var_target += pow(diff_vec , vec2(2.0));

            col_unc_prop += rays[i].color_uncertainty.rgb;
            col_unc_new += pow(rays[i].color.rgb - color.rgb, vec3(2.0));
            dir_unc_prop += vec2(rays[i].color_change.a, rays[i].color_uncertainty.a);
        }

        var_target /= sampleCount;
        col_unc_new /= sampleCount;
        col_unc_prop /= sampleCount * sampleCount;
        dir_unc_prop /= sampleCount * sampleCount;

        vec3 col_var = col_unc_prop + col_unc_new;
        vec2 dir_unc = dir_unc_prop + var_target;

        // the final color is the average of all samples (calculated above)
        rt_color = color;

// TODO propagate color error ? (wenn man nur den absoluten fehler betrachtet, dann nicht)
        vec3 color_diff = rt_color.rgb - original_color;
        // vec3 color_diff = rt_color.rgb - rays[16].color.rgb;


        /*
            For writing back the values:
            - The uncertainty was calculated from the input uncertainties and the uncertainty introdced here.
              Therefore the old uncertainty is replaced
            - The change introduced here has to be added to the change already present and therefore needs to be added.
        */
        rt_color_change =    
            vec4(
                texture(s_color_change, v_tex).rgb + color_diff,
                dir_unc.x
            );
        rt_color_uncertainty = 
            vec4(
                col_var,
                dir_unc.y
            );

    } else {
        rt_color = texture(s_color, v_tex);
        rt_color_change =       texture(s_color_change, v_tex);
        rt_color_uncertainty =  texture(s_color_uncertainty, v_tex);
    }

}
