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


in vec2 v_tex;
out vec4 rt_color;
out vec4 rt_deflection;

struct Simulation
{
  vec4 color;
  vec3 dir;
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

// intesects the ray with the image and returns the color of the texture at this position
vec4 getColorWithRay(in vec3 start, in vec3 dir) {
    float t = (getDepth(v_tex) + VITREOUS_HUMOUR_RADIUS - start.z) / dir.z;
    start += dir * t;
    return texture(s_color, start.xy / ((start.z - VITREOUS_HUMOUR_RADIUS) * TEXTURE_SCALE) + 0.5);
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

// sends the ray through all four surfaces in the optical system and returns the color
// of the texture at the intersection point.
vec4 getColorSample(vec3 start, vec2 aim, float focalLength, float nAnteriorChamberFactor) {
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

    vec4 color = getColorWithRay(start, dir);
    Simulation sim = Simulation(color,dir);
    return sim;
}


vec2 to_dir_angle(in vec3 dir){		
	vec2 pt;
    pt.x = atan(dir.x/dir.z)*u_dir_calc_scale;
    pt.y = atan(dir.y/dir.z)*u_dir_calc_scale;
    return pt;
}

float[17] sort_observations(float[17] obs)
{
	float t;
	for (int j = 0; j < 17-1; ++j)
	{
		int swap = j;
		for (int i = j+1; i < 17; ++i)
		{
			if (obs[swap] > obs[i])
				swap = i;
		}
		t = obs[swap];
		obs[swap] = obs[j];
		obs[j] = t;
	}
    return obs;
}

void main() {

    // // color range
    // vec3 color_min = vec3(1.0);
    // vec3 color_max = vec3(0.0);

    // // mist
    // float dev_min = 1.0;
    // float dev_max = 0.0;

    // // rg range
    // float color_min_r = 1.0;
    // float color_max_r = 0.0;
    // float color_min_g = 1.0;
    // float color_max_g = 0.0;

    // // color sd
    // vec3 avg_rgb = vec3(0.0);
    // vec3 var_rgb = vec3(0.0);
    // vec3 sd_rgb = vec3(0.0);
    float sd_avg = 0.0;

    // // dir sd
    vec2 avg_dir = vec2(0.0);
    vec2 var_dir = vec2(0.0);
    vec2 sd_dir = vec2(0.0);


    // values stolen from the interwebs: https://www.real-statistics.com/statistics-tables/shapiro-wilk-table/
    float sw_weights[8] = float[](
        0.4968, 0.3273, 0.2540, 0.1988,
        0.1524, 0.1109, 0.0725, 0.0359
        ); 
    float sw_x = 0;
    float sw_y = 0;




    if (1 == u_active) {
        rt_color = vec4(0.5);
        vec3 start = (texture(s_normal, v_tex) * 2.0 - 1.0).xyz * VITREOUS_HUMOUR_RADIUS;
        float sampleCount = 0.0;
        vec4 color = vec4(0.0);

        // prepare accomodation
        // We focus on all possible depths: for each pixel, the lens can focus on the individual depth
        float focalLength = length(vec3(
            (v_tex.xy - 0.5) * TEXTURE_SCALE,
            u_depth_max - texture(s_depth, v_tex).r * (u_depth_max - u_depth_min)));

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

        Simulation colors[17];
        float sw_values_x[17];
        float sw_values_y[17];


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
            colors[0] = getColorSample(start, vec2(-0.5, -0.5), accomodationDistance, nAnteriorChamberFactor);
            colors[1] = getColorSample(start, vec2(-0.5, 0.5), accomodationDistance, nAnteriorChamberFactor);
            colors[2] = getColorSample(start, vec2(0.5, -0.5), accomodationDistance, nAnteriorChamberFactor);
            colors[3] = getColorSample(start, vec2(0.5, 0.5), accomodationDistance, nAnteriorChamberFactor);
            colors[4] = getColorSample(start, vec2(-0.5, 0.0), accomodationDistance, nAnteriorChamberFactor);
            colors[5] = getColorSample(start, vec2(0.5, 0.0), accomodationDistance, nAnteriorChamberFactor);
            colors[6] = getColorSample(start, vec2(0.0, -0.5), accomodationDistance, nAnteriorChamberFactor);
            colors[7] = getColorSample(start, vec2(0.0, 0.5), accomodationDistance, nAnteriorChamberFactor);
        case 3:
            sampleCount += 4.0;
            colors[8] = getColorSample(start, vec2(-1.0, -1.0), accomodationDistance, nAnteriorChamberFactor);
            colors[9] = getColorSample(start, vec2(-1.0, 1.0), accomodationDistance, nAnteriorChamberFactor);
            colors[10] = getColorSample(start, vec2(1.0, -1.0), accomodationDistance, nAnteriorChamberFactor);
            colors[11] = getColorSample(start, vec2(1.0, 1.0), accomodationDistance, nAnteriorChamberFactor);
        case 2:
            sampleCount += 4.0;
            colors[12] = getColorSample(start, vec2(-1.0, 0.0), accomodationDistance, nAnteriorChamberFactor);
            colors[13] = getColorSample(start, vec2(1.0, 0.0), accomodationDistance, nAnteriorChamberFactor);
            colors[14] = getColorSample(start, vec2(0.0, -1.0), accomodationDistance, nAnteriorChamberFactor);
            colors[15] = getColorSample(start, vec2(0.0, 1.0), accomodationDistance, nAnteriorChamberFactor);
        case 1:
            sampleCount += 1.0;
            colors[16] = getColorSample(start, vec2(0.0, 0.0), accomodationDistance, nAnteriorChamberFactor);
        }


        // color sd ----------------------------------------------------
        // // calculate the mean color
        // for (int i = 0; i < 17; i++) {
        //     avg_rgb += colors[i].color.rgb;
        // }
        // avg_rgb = avg_rgb/sampleCount;

        // // calculate the color variances for each channel
        // // pow and sqrt are operating componentwise
        // for (int i = 0; i < 17; i++) {
        //     var_rgb += pow(colors[i].color.rgb-avg_rgb , vec3(2.0));
        // }
        // var_rgb = var_rgb/sampleCount;  // alt sampleCount-1
        // sd_rgb = sqrt(var_rgb);

        // // lets calculate the average variance across all channels
        // sd_avg+=sd_rgb.r;
        // sd_avg+=sd_rgb.g;
        // sd_avg+=sd_rgb.b;
        // sd_avg /= 3;

        // dir sd ----------------------------------------------------
        // calculate the mean direction
        for (int i = 0; i < 17; i++) {
            avg_dir += to_dir_angle(colors[i].dir);
            sw_values_x[i] = to_dir_angle(colors[i].dir).x;
            sw_values_y[i] = to_dir_angle(colors[i].dir).y;
        }
        avg_dir /= sampleCount;

        for (int i = 0; i < 17; i++) {
            var_dir += pow(to_dir_angle(colors[i].dir)-avg_dir , vec2(2.0));
        }
        var_dir /= sampleCount;
        sd_dir = sqrt(var_dir);
        
        // // lets calculate the average variance across all channels
        sd_avg+=sd_dir.x;
        sd_avg+=sd_dir.y;
        sd_avg /= 2;


        // sort sw_values

        sw_values_x = sort_observations(sw_values_x);
        sw_values_y = sort_observations(sw_values_y);


        // calculate sw value based on weights
        float b_x = 0.0;
        float b_y = 0.0;
        for (int i = 0; i < 8; i++) {
            b_x += sw_weights[i]*(sw_values_x[16-i]-sw_values_x[i]);
            b_y += sw_weights[i]*(sw_values_y[16-i]-sw_values_y[i]);
        }
        sw_x = b_x*b_x / ( 16*var_dir.x );
        sw_y = b_y*b_y / ( 16*var_dir.y );

        // for sign. niv. 5%, with 17 samples, w-crit is 0.892, according to https://scistatcalc.blogspot.com/2013/09/critical-value-of-w-statistic.html?m=1
        float w_crit = 0.892;
        if (sw_x > w_crit){ sw_x=1.0; } else{ sw_x=0.0; }
        if (sw_y > w_crit){ sw_y=1.0; } else{ sw_y=0.0; }





        for (int i = 0; i < 17; i++) {
            
            // dot div ----------------------------------------------------
            // float dev = abs(dot(normalize(colors[i].dir),vec3(0.0,0.0,1.0)));
            // if (dev < dev_min) {
            //     dev_min = dev;
            // }
            // if (dev > dev_max) {
            //     dev_max = dev;
            // }

            //r/g difference ----------------------------------------------------
            // if(abs(normalize(colors[i].dir).r) < color_min_r){
            //     color_min_r = abs(normalize(colors[i].dir).r);
            // }
            // if(abs(normalize(colors[i].dir).b) < color_min_g){
            //     color_min_g = abs(normalize(colors[i].dir).b);
            // }         
            // if(abs(normalize(colors[i].dir).r) > color_max_r){
            //     color_max_r = abs(normalize(colors[i].dir).r);
            // }
            // if(abs(normalize(colors[i].dir).b) > color_max_g){
            //     color_max_g = abs(normalize(colors[i].dir).b);
            // }   

            //color diff range ----------------------------------------------------
            // if(colors[i].color.r < color_min.r){
            //     color_min.r = colors[i].color.r;
            // }
            // if(colors[i].color.g < color_min.g){
            //     color_min.g = colors[i].color.g;
            // }
            // if(colors[i].color.b < color_min.b){
            //     color_min.b = colors[i].color.b;
            // }
            // if(colors[i].color.r > color_max.r){
            //     color_max.r = colors[i].color.r;
            // }
            // if(colors[i].color.g > color_max.g){
            //     color_max.g = colors[i].color.g;
            // }
            // if(colors[i].color.b > color_max.b){
            //     color_max.b = colors[i].color.b;
            // }

            color+=colors[i].color;
            //color+=vec4(1.0);
        }

        // the final color is the average of all samples
        rt_color = color / sampleCount;
    } else {
        rt_color = texture(s_color, v_tex);
    }


    //rt_deflection = vec4(abs(color_max_r-color_min_r), abs(color_max_g-color_min_g), 0.0, 0.0);


    // color_diff
    // rt_deflection = vec4(
    //     abs(color_max.r-color_min.r), 
    //     abs(color_max.g-color_min.g), 
    //     abs(color_max.b-color_min.b), 
    //     0.0);


    // color variance
    // rt_deflection = vec4(
    //    //sd_rgb,
    //    //vec3(sd_avg),
    //     0.0);


    // direction variance
    rt_deflection = vec4(
        vec3(sd_dir.x, sd_dir.y, 0.0),
        //vec3(sd_avg),
        //vec3(sw_x,sw_y,0.0),
        0.0);

    // rt_deflection = vec4(vec3(dev_max-dev_min), 0.0);
}
