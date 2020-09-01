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

uniform sampler2D s_color;
uniform sampler2D s_normal;
uniform sampler2D s_depth;
uniform sampler2D s_cornea;

in vec2 v_tex;
out vec4 rt_color;

// alters the direction of a ray according to the imperfection in the cornea specified by the cornea map
vec3 applyCorneaImperfection(vec3 pos, vec3 dir) {
    vec2 corneaMapSample = texture(s_cornea, pos.xy * vec2(0.17, -0.17) - vec2(-0.5)).rg;
    vec3 deviation = vec3((corneaMapSample - vec2(0.5)) * vec2(CORNEA_MAP_FACTOR), 0.0);
    return dir + deviation;
}

// calculates the radius of curvature of the outer lens surface
// while taking accomodation into account.
// accDistance: the distance between the frontmost point of the
//              vitreous humour and the focused point
float getOuterLensRadius(float accDistance) {
    return OUTER_LENS_RADIUS_A * accDistance * accDistance
        + OUTER_LENS_RADIUS_B * accDistance
        + OUTER_LENS_RADIUS_C;
}

// calculates the center of the sphere representing the outer
// lens surface while taking accomodation into account.
// accDistance: the distance between the frontmost point of the
//              vitreous humour and the focused point
vec3 getOuterLensCenter(float accDistance) {
    float outerLensTip = OUTER_LENS_TIP_A * accDistance * accDistance
        + OUTER_LENS_TIP_B * accDistance
        + OUTER_LENS_TIP_C;
    return vec3(0.0, 0.0, outerLensTip - getOuterLensRadius(accDistance));
}

// calculates the radius of curvature of the inner lens surface
// while taking accomodation into account.
// accDistance: the distance between the frontmost point of the
//              vitreous humour and the focused point
float getInnerLensRadius(float accDistance) {
    return INNER_LENS_RADIUS_A * accDistance * accDistance
        + INNER_LENS_RADIUS_B * accDistance
        + INNER_LENS_RADIUS_C;
}

// calculates the center of the sphere representing the inner
// lens surface while taking accomodation into account.
// accDistance: the distance between the frontmost point of the
//              vitreous humour and the focused point
vec3 getInnerLensCenter(float accDistance) {
    float innerLensTip = INNER_LENS_TIP_A * accDistance * accDistance
        + INNER_LENS_TIP_B * accDistance
        + INNER_LENS_TIP_C;
    return vec3(0.0, 0.0, innerLensTip + getInnerLensRadius(accDistance));
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
vec4 getColorSample(vec3 start, vec2 aim, float accDistance, float nAnteriorChamberFactor) {
    vec3 dir = normalize(vec3(aim, VITREOUS_HUMOUR_RADIUS) - start);

    start = rayIntersectSphere(start, dir, getInnerLensCenter(accDistance), getInnerLensRadius(accDistance));
    dir = refract(dir, normalize(start - getInnerLensCenter(accDistance)), N_VITREOUS_HUMOUR / N_LENS);

    start = rayIntersectSphere(start, dir, getOuterLensCenter(accDistance), getOuterLensRadius(accDistance));
    dir = refract(dir, -normalize(start - getOuterLensCenter(accDistance)), N_LENS / getNAnteriorChamber(nAnteriorChamberFactor));

    start = rayIntersectSphere(start, dir, INNER_CORNEA_CENTER, INNER_CORNEA_RADIUS);
    dir = refract(dir, -normalize(start - INNER_CORNEA_CENTER), getNAnteriorChamber(nAnteriorChamberFactor) / N_CORNEA);

    start = rayIntersectSphere(start, dir, OUTER_CORNEA_CENTER, OUTER_CORNEA_RADIUS);
    dir = refract(dir, -normalize(start - OUTER_CORNEA_CENTER), N_CORNEA / N_AIR);
    dir = applyCorneaImperfection(start, dir);

    return getColorWithRay(start, dir);
}

void main() {
    if (1 == u_active) {
        rt_color = vec4(0.5);
        vec3 start = (texture(s_normal, v_tex) * 2.0 - 1.0).xyz * VITREOUS_HUMOUR_RADIUS;
        float sampleCount = 0.0;
        vec4 color = vec4(0.0);

        // prepare accomodation
        float accomodationDistance = length(vec3(
            (v_tex.xy - 0.5) * TEXTURE_SCALE,
            u_depth_max - texture(s_depth, v_tex).r * (u_depth_max - u_depth_min)));

        float nAnteriorChamberFactor = 1.0;
        if (accomodationDistance > u_far_point) {
            nAnteriorChamberFactor = 1.0 / pow((accomodationDistance - u_far_point) / accomodationDistance + 1.0, 0.08 * u_far_vision_factor);
            accomodationDistance = u_far_point;
        } else if (accomodationDistance < u_near_point) {
            nAnteriorChamberFactor = pow(accomodationDistance / u_near_point, 0.12 * u_near_vision_factor);
            accomodationDistance = u_near_point;
        }

        // The higher the sampleCount, the more rays are cast. Rays are cast in this fashion,
        // where the number indicates the minimum sampleCount required for this ray to be cast:
        // 3   2   3
        //   4 4 4
        // 2 4 1 4 2
        //   4 4 4
        // 3   2   3
        switch (u_samplecount) {
        case 4:
            sampleCount += 8.0;
            color += getColorSample(start, vec2(-0.5, -0.5), accomodationDistance, nAnteriorChamberFactor);
            color += getColorSample(start, vec2(-0.5, 0.5), accomodationDistance, nAnteriorChamberFactor);
            color += getColorSample(start, vec2(0.5, -0.5), accomodationDistance, nAnteriorChamberFactor);
            color += getColorSample(start, vec2(0.5, 0.5), accomodationDistance, nAnteriorChamberFactor);
            color += getColorSample(start, vec2(-0.5, 0.0), accomodationDistance, nAnteriorChamberFactor);
            color += getColorSample(start, vec2(0.5, 0.0), accomodationDistance, nAnteriorChamberFactor);
            color += getColorSample(start, vec2(0.0, -0.5), accomodationDistance, nAnteriorChamberFactor);
            color += getColorSample(start, vec2(0.0, 0.5), accomodationDistance, nAnteriorChamberFactor);
        case 3:
            sampleCount += 4.0;
            color += getColorSample(start, vec2(-1.0, -1.0), accomodationDistance, nAnteriorChamberFactor);
            color += getColorSample(start, vec2(-1.0, 1.0), accomodationDistance, nAnteriorChamberFactor);
            color += getColorSample(start, vec2(1.0, -1.0), accomodationDistance, nAnteriorChamberFactor);
            color += getColorSample(start, vec2(1.0, 1.0), accomodationDistance, nAnteriorChamberFactor);
        case 2:
            sampleCount += 4.0;
            color += getColorSample(start, vec2(-1.0, 0.0), accomodationDistance, nAnteriorChamberFactor);
            color += getColorSample(start, vec2(1.0, 0.0), accomodationDistance, nAnteriorChamberFactor);
            color += getColorSample(start, vec2(0.0, -1.0), accomodationDistance, nAnteriorChamberFactor);
            color += getColorSample(start, vec2(0.0, 1.0), accomodationDistance, nAnteriorChamberFactor);
        case 1:
            sampleCount += 1.0;
            color += getColorSample(start, vec2(0.0, 0.0), accomodationDistance, nAnteriorChamberFactor);
        }

        // the final color is the average of all samples
        rt_color = color / sampleCount;
    } else {
        rt_color = texture(s_color, v_tex);
    }
}
