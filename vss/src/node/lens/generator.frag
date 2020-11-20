// plane in front of the eye
#define TEXTURE_DISTANCE 1000.0
#define TEXTURE_SCALE 2000.0

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

in vec2 v_tex;
out vec4 rt_color;

// calculates the radius of curvature of the outer lens surface
// while taking accomodation into account.
// distance: the distance between the frontmost point of the
//           vitreous humour and the focused point
float getOuterLensRadius(float distance) {
	return OUTER_LENS_RADIUS_A * distance * distance
	     + OUTER_LENS_RADIUS_B * distance
	     + OUTER_LENS_RADIUS_C;
}

// calculates the center of the sphere representing the outer
// lens surface while taking accomodation into account.
// distance: the distance between the frontmost point of the
//           vitreous humour and the focused point
vec3 getOuterLensCenter(float distance) {
	float outerLensTip = OUTER_LENS_TIP_A * distance * distance
	                   + OUTER_LENS_TIP_B * distance
	                   + OUTER_LENS_TIP_C;
	return vec3(0.0, 0.0, outerLensTip - getOuterLensRadius(distance));
}

// calculates the radius of curvature of the inner lens surface
// while taking accomodation into account.
// distance: the distance between the frontmost point of the
//           vitreous humour and the focused point
float getInnerLensRadius(float distance) {
	return INNER_LENS_RADIUS_A * distance * distance
	     + INNER_LENS_RADIUS_B * distance
	     + INNER_LENS_RADIUS_C;
}

// calculates the center of the sphere representing the inner
// lens surface while taking accomodation into account.
// distance: the distance between the frontmost point of the
//           vitreous humour and the focused point
vec3 getInnerLensCenter(float distance) {
	float innerLensTip = INNER_LENS_TIP_A * distance * distance
	                   + INNER_LENS_TIP_B * distance
	                   + INNER_LENS_TIP_C;
	return vec3(0.0, 0.0, innerLensTip + getInnerLensRadius(distance));
}



void getRay(inout vec3 start, inout vec3 dir){
	// TODO subtract (0.5, 0.5) ?
	start = vec3((v_tex.xy - 0.5) * TEXTURE_SCALE, TEXTURE_DISTANCE + VITREOUS_HUMOUR_RADIUS);
	dir = normalize(vec3(0.0, 0.0, VITREOUS_HUMOUR_RADIUS) - start);
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

void main() {
	vec3 start, dir;
	getRay(start, dir);
	
	float distance = length(vec3((v_tex.xy - 0.5) * TEXTURE_SCALE, TEXTURE_DISTANCE));

	start = rayIntersectSphere(start, dir, OUTER_CORNEA_CENTER, OUTER_CORNEA_RADIUS);
	dir = refract(dir, normalize(start - OUTER_CORNEA_CENTER), N_AIR/N_CORNEA);

	start = rayIntersectSphere(start, dir, INNER_CORNEA_CENTER, INNER_CORNEA_RADIUS);
	dir = refract(dir, normalize(start - INNER_CORNEA_CENTER), N_CORNEA/N_ANTERIOR_CHAMBER);

	start = rayIntersectSphere(start, dir, getOuterLensCenter(distance), getOuterLensRadius(distance));
	dir = refract(dir, normalize(start - getOuterLensCenter(distance)), N_ANTERIOR_CHAMBER/N_LENS);

	start = rayIntersectSphere(start, dir, getInnerLensCenter(distance), getInnerLensRadius(distance));
	dir = refract(dir, -normalize(start - getInnerLensCenter(distance)), N_LENS/N_VITREOUS_HUMOUR);


	start = rayIntersectSphere(start, dir, VITREOUS_HUMOUR_CENTER, VITREOUS_HUMOUR_RADIUS);

	rt_color = vec4(normalize(start)/2.+0.5, 1.0);
}
