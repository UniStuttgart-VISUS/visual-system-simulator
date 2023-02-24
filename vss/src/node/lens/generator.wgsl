// plane in front of the eye
const TEXTURE_DISTANCE: f32 = 1000.0;
const TEXTURE_SCALE: f32 = 2000.0;

struct FragmentOutput {
    @location(0) color: vec4<f32>,
};

fn getRay(start: ptr<function, vec3<f32>>, dir: ptr<function, vec3<f32>>, v_tex: vec2<f32>){
	// TODO subtract (0.5, 0.5) ?
	*start = vec3<f32>((v_tex - 0.5) * TEXTURE_SCALE, TEXTURE_DISTANCE + VITREOUS_HUMOUR_RADIUS);
	*dir = normalize(vec3<f32>(0.0, 0.0, VITREOUS_HUMOUR_RADIUS) - *start);
}

@fragment
fn fs_main(in: VertexOutput) -> FragmentOutput {
    var out: FragmentOutput;

	var start = vec3<f32>(0.0, 0.0, 0.0);
	var dir = vec3<f32>(0.0, 0.0, 0.0);
	getRay(&start, &dir, in.tex_coords);
	
	let distance = length(vec3<f32>((in.tex_coords - 0.5) * TEXTURE_SCALE, TEXTURE_DISTANCE));

	start = rayIntersectSphere(start, dir, OUTER_CORNEA_CENTER, OUTER_CORNEA_RADIUS);
	dir = refract(dir, normalize(start - OUTER_CORNEA_CENTER), N_AIR/N_CORNEA);

	start = rayIntersectSphere(start, dir, INNER_CORNEA_CENTER, INNER_CORNEA_RADIUS);
	dir = refract(dir, normalize(start - INNER_CORNEA_CENTER), N_CORNEA/N_ANTERIOR_CHAMBER);

	start = rayIntersectSphere(start, dir, getOuterLensCenter(distance), getOuterLensRadius(distance));
	dir = refract(dir, normalize(start - getOuterLensCenter(distance)), N_ANTERIOR_CHAMBER/N_LENS);

	start = rayIntersectSphere(start, dir, getInnerLensCenter(distance), getInnerLensRadius(distance));
	dir = refract(dir, -normalize(start - getInnerLensCenter(distance)), N_LENS/N_VITREOUS_HUMOUR);

	start = rayIntersectSphere(start, dir, VITREOUS_HUMOUR_CENTER, VITREOUS_HUMOUR_RADIUS);

	out.color = vec4<f32>(normalize(start), 1.0);

    return out;
}
