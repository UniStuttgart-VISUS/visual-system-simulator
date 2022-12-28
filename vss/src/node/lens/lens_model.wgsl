
// This model was obtained via machine learning, using a
// particle swarm optimisation algorithm. The cost gives a
// rough estimate for the quality of the model (the lower
// the cost, the better).
// Cost:  0.11083103563252572
let N_AIR: f32 = 1.0;
let N_CORNEA: f32 = 1.64099928591927;
let N_ANTERIOR_CHAMBER: f32 = 1.4532300393624265;
let N_LENS: f32 = 1.4393602250536033;
let N_VITREOUS_HUMOUR: f32 = 1.362681339093861;

let OUTER_CORNEA_RADIUS: f32 = 7.873471062713062;
let INNER_CORNEA_RADIUS: f32 = 4.7658074271207695;
let VITREOUS_HUMOUR_RADIUS: f32 = 11.704040541034434;

let OUTER_CORNEA_CENTER: vec3<f32> = vec3<f32>(0., 0., 6.673155500500702);
let INNER_CORNEA_CENTER: vec3<f32> = vec3<f32>(0., 0., 7.877094062114152);
let VITREOUS_HUMOUR_CENTER: vec3<f32> = vec3<f32>(0., 0., 0.0);

let OUTER_LENS_RADIUS_A: f32 = 5.0719602142230795E-11;
let OUTER_LENS_RADIUS_B: f32 = -4.728025830060936E-6;
let OUTER_LENS_RADIUS_C: f32 = 8.768369917723753;
let INNER_LENS_RADIUS_A: f32 = 1.8872104415414247E-11;
let INNER_LENS_RADIUS_B: f32 = 1.3008349405807643E-5;
let INNER_LENS_RADIUS_C: f32 = 7.237406273489772;

let OUTER_LENS_TIP_A: f32 = -3.867155451878399E-11;
let OUTER_LENS_TIP_B: f32 = 1.5465450056679198E-7;
let OUTER_LENS_TIP_C: f32 = 8.839055805878198;
let INNER_LENS_TIP_A: f32 = -8.525281533727847E-12;
let INNER_LENS_TIP_B: f32 = 3.777299048943033E-6;
let INNER_LENS_TIP_C: f32 = 7.03207918882336;

// calculates the radius of curvature of the outer lens surface
// while taking accomodation into account.
// distance: the distance between the frontmost point of the
//           vitreous humour and the focused point
fn getOuterLensRadius(distance: f32) -> f32 {
	return OUTER_LENS_RADIUS_A * distance * distance
	     + OUTER_LENS_RADIUS_B * distance
	     + OUTER_LENS_RADIUS_C;
}

// calculates the center of the sphere representing the outer
// lens surface while taking accomodation into account.
// distance: the distance between the frontmost point of the
//           vitreous humour and the focused point
fn getOuterLensCenter(distance: f32) -> vec3<f32> {
	let outerLensTip = OUTER_LENS_TIP_A * distance * distance
	                 + OUTER_LENS_TIP_B * distance
	                 + OUTER_LENS_TIP_C;
	return vec3<f32>(0.0, 0.0, outerLensTip - getOuterLensRadius(distance));
}

// calculates the radius of curvature of the inner lens surface
// while taking accomodation into account.
// distance: the distance between the frontmost point of the
//           vitreous humour and the focused point
fn getInnerLensRadius(distance: f32) -> f32 {
	return INNER_LENS_RADIUS_A * distance * distance
	     + INNER_LENS_RADIUS_B * distance
	     + INNER_LENS_RADIUS_C;
}

// calculates the center of the sphere representing the inner
// lens surface while taking accomodation into account.
// distance: the distance between the frontmost point of the
//           vitreous humour and the focused point
fn getInnerLensCenter(distance: f32) -> vec3<f32> {
	let innerLensTip = INNER_LENS_TIP_A * distance * distance
	                   + INNER_LENS_TIP_B * distance
	                   + INNER_LENS_TIP_C;
	return vec3<f32>(0.0, 0.0, innerLensTip + getInnerLensRadius(distance));
}

fn getNAnteriorChamber(nAnteriorChamberFactor: f32) -> f32 {
    return N_ANTERIOR_CHAMBER * nAnteriorChamberFactor;
}

//rPos = ray position
//rDir = ray direction
//cPos = sphere position
//cRad = sphere radius
// returns a point where the ray intersects the sphere
fn rayIntersectSphere(rPos: vec3<f32>, rDir: vec3<f32>, cPos: vec3<f32>, cRad: f32) -> vec3<f32> {
    let a = dot(rDir, rDir);
    let b = 2.0 * dot(rDir, (rPos - cPos));
    let c = dot(rPos - cPos, rPos - cPos) - cRad * cRad;
    let disk = sqrt(b * b - 4.0 * a * c);
    let t1 = (-b + disk) / (2.0 * a);
    let t2 = (-b - disk) / (2.0 * a);

    if (t1 < t2) {
        return select(t2, t1, t1 >= 0.0) * rDir + rPos;
    } else {
        return select(t1, t2, t2 >= 0.0) * rDir + rPos;
    }
}

// TODO find out why the built-in refract function isn't working (throws an "unknown identifier" error)
fn refract(x: vec3<f32>, y: vec3<f32>, z: f32) -> vec3<f32> {
	let k = 1.0 - z * z * (1.0 - dot(y, x) * dot(y, x));
	return select(vec3<f32>(0.0, 0.0, 0.0), z * x - (z * dot(y, x) + sqrt(k)) * y, k >= 0.0);
}
