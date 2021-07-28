uniform int u_track_error;

uniform sampler2D s_color;
uniform sampler2D s_original;
uniform sampler2D s_deflection;
uniform sampler2D s_color_change;
uniform sampler2D s_color_uncertainty;
uniform sampler2D s_covariances;

uniform vec2 u_resolution;
uniform int u_show_variance;
uniform int u_color_space;
uniform int u_variance_measure;

in vec2 v_tex;
out vec4 rt_color;
out vec4 rt_measure;
out vec4 rt_deflection;
out vec4 rt_color_change;
out vec4 rt_color_uncertainty;
out vec4 rt_covariances;

#define VAR_SIZE 10
#define VAR_MEASURE 2

#define COLOR_SPACE_RGB 0
#define COLOR_SPACE_LAB 1
#define COLOR_SPACE_ITP 2

#define SHOW_VARIANCE_NONE 0
#define SHOW_VARIANCE_PRE 1
#define SHOW_VARIANCE_POST 2
#define SHOW_VARIANCE_DIFF 3

#define MEASURE_VARIANCE_FIRST 0
#define MEASURE_VARIANCE_AVG 1
#define MEASURE_VARIANCE_VAR_AVG 2
#define MEASURE_VARIANCE_FIRST_CONTRAST 3
#define MEASURE_VARIANCE_DELTA_E 4
#define MEASURE_VARIANCE_MICHELSON_CONTRAST 5

vec3 rgb2xyz(vec3 c){
	vec3 tmp=vec3(
		(c.r>.04045)?pow((c.r+.055)/1.055,2.4):c.r/12.92,
		(c.g>.04045)?pow((c.g+.055)/1.055,2.4):c.g/12.92,
		(c.b>.04045)?pow((c.b+.055)/1.055,2.4):c.b/12.92
	);
	mat3 mat=mat3(
		.4124,.3576,.1805,
		.2126,.7152,.0722,
		.0193,.1192,.9505
	);
	return 100.*(tmp*mat);
}
vec3 xyz2lab(vec3 c){
	vec3 n=c/vec3(95.047,100.,108.883),
	     v=vec3(
		(n.x>.008856)?pow(n.x,1./3.):(7.787*n.x)+(16./116.),
		(n.y>.008856)?pow(n.y,1./3.):(7.787*n.y)+(16./116.),
		(n.z>.008856)?pow(n.z,1./3.):(7.787*n.z)+(16./116.)
	);
	return vec3((116.*v.y)-16.,500.*(v.x-v.y),200.*(v.y-v.z));
}
vec3 rgb2lab(vec3 c){
	vec3 lab=xyz2lab(rgb2xyz(c));
	return vec3(lab.x/100.,.5+.5*(lab.y/127.),.5+.5*(lab.z/127.));
}

//E is the signal for each colour component {R, G, B} proportional to scene linear light and scaled by camera exposure, normalized to the range [0:12]
//https://glenwing.github.io/docs/ITU-R-BT.2100-0.pdf site 5(7 on pdf)
vec3 oetf(vec3 e){
    float a = 0.17883277;
    float b = 0.28466892;
    float c = 0.55991073;
    vec3 result = vec3(0.0);
    if (e.r <= 1.0){result.r = sqrt(e.r)/2.0;} else {result.r = a*log(e.r-b)+c;}
    if (e.g <= 1.0){result.g = sqrt(e.g)/2.0;} else {result.g = a*log(e.g-b)+c;}
    if (e.b <= 1.0){result.b = sqrt(e.b)/2.0;} else {result.b = a*log(e.b-b)+c;}
    //vec3 secondWeight = floor(min(e, vec3(1.0)));
    //vec3 firstWeight = vec3(1.0) - secondWeight;
    return result;//firstWeight*(sqrt(e)/2.0) + secondWeight*(a*log(e-b)+c);
}

//https://en.wikipedia.org/wiki/ICtCp
vec3 rgb2itp(vec3 c){
    mat3 lmsMat = mat3(1688., 683., 99., 2146., 2951., 309., 262., 462., 3688.);
    mat3 ictcpMat = mat3(2048., 3625., 9500., 2048., -7465., -9212., 0., 3840., -288.);
    vec3 lms = lmsMat * c / 4096.;
    return ictcpMat * oetf(lms) / 4096.;
}

vec3 sampleColor(in sampler2D texSampler, in vec2 fragCoord){
    if(u_color_space == COLOR_SPACE_RGB){
        return texture(texSampler, fragCoord).rgb;
    }else if(u_color_space == COLOR_SPACE_LAB){
        return rgb2lab(texture(texSampler, fragCoord).rgb);
    }else if(u_color_space == COLOR_SPACE_ITP){
        return rgb2itp(texture(texSampler, fragCoord).rgb);
    }else{
        return texture(texSampler, fragCoord).rgb;
    }
}

//first method of measuring variance (only taking into account the difference to main color)
float varianceMeasure_first(in sampler2D texSampler, in vec2 fragCoord){
    float result = 0.0;
    vec3 main_color = sampleColor(texSampler, fragCoord);

    for (int i = -VAR_SIZE; i <= VAR_SIZE; ++i) {
        for (int j = -VAR_SIZE; j <= VAR_SIZE; ++j) {
            vec3 diff = abs(main_color - sampleColor(texSampler, (fragCoord + vec2(float(i), float(j)) / u_resolution)));
            result += diff.x*diff.x + diff.y*diff.y + diff.z*diff.z;
        }
    }
    return result/pow(VAR_SIZE*2+1, 2);
}

//average difference to main color
float varianceMeasure_avg(in sampler2D texSampler, in vec2 fragCoord){
    vec3 avg = vec3(0.0);
    vec3 main_color = sampleColor(texSampler, fragCoord);

    for (int i = -VAR_SIZE; i <= VAR_SIZE; ++i) {
        for (int j = -VAR_SIZE; j <= VAR_SIZE; ++j) {
            avg += abs(main_color - sampleColor(texSampler, (fragCoord + vec2(float(i), float(j)) / u_resolution)));
        }
    }
    return (avg.r + avg.g + avg.b) / (pow(VAR_SIZE*2+1, 2) * 3.0);
}

//second method of measuring variance (taking into account the difference to main color relative to average difference)
float varianceMeasure_var_avg(in sampler2D texSampler, in vec2 fragCoord){
    float result = 0.0;
    vec3 avg = vec3(0.0);
    vec3 main_color = sampleColor(texSampler, fragCoord);

    for (int i = -VAR_SIZE; i <= VAR_SIZE; ++i) {
        for (int j = -VAR_SIZE; j <= VAR_SIZE; ++j) {
            avg += abs(main_color - sampleColor(texSampler, (fragCoord + vec2(float(i), float(j)) / u_resolution)));
        }
    }
    avg /= pow(VAR_SIZE*2+1, 2);

    //durchschnittliche varianz
    for (int i = -VAR_SIZE; i <= VAR_SIZE; ++i) {
        for (int j = -VAR_SIZE; j <= VAR_SIZE; ++j) {
            vec3 diff = abs(main_color - sampleColor(texSampler, (fragCoord + vec2(float(i), float(j)) / u_resolution))) - avg;
            result += diff.x*diff.x + diff.y*diff.y + diff.z*diff.z;
        }
    }
    return result/pow(VAR_SIZE*2+1, 2);
}

float varianceMeasure_first_contrast(in sampler2D texSampler, in vec2 fragCoord){
    vec3 avg = vec3(0.0);
    vec3 std = vec3(0.0);
    vec3 main_color = sampleColor(texSampler, fragCoord);

    for (float i = -VAR_SIZE; i <= VAR_SIZE; ++i) {
        for (float j = -VAR_SIZE; j <= VAR_SIZE; ++j) {
            avg += abs(main_color - sampleColor(texSampler, (fragCoord + vec2(float(i), float(j)) / u_resolution)));
        }
    }
    avg /= pow(VAR_SIZE*2.0+1.0, 2.0);
    
    for (float i = -VAR_SIZE; i <= VAR_SIZE; ++i) {
        for (float j = -VAR_SIZE; j <= VAR_SIZE; ++j) {
            vec3 diff = main_color - sampleColor(texSampler, (fragCoord + vec2(float(i), float(j)) / u_resolution));
            std += diff*diff;
        }
    }
    return (std/(pow(VAR_SIZE*2.0+1.0, 2.0)-1.0)).x;
}

//delta E distance measure
float varianceMeasure_delta_e(in sampler2D texSampler, in vec2 fragCoord){
    float result = 0.0;
    vec3 itpWeights = vec3(1.0, 0.5, 1.0);
    vec3 main_color = sampleColor(texSampler, fragCoord)*itpWeights;
    
    for (float i = -VAR_SIZE; i <= VAR_SIZE; ++i) {
        for (float j = -VAR_SIZE; j <= VAR_SIZE; ++j) {
            vec3 diff = main_color - sampleColor(texSampler, (fragCoord + vec2(float(i), float(j)) / u_resolution))*itpWeights;
            //float value = 720.0*length(diff);
            float value = length(diff);
            //quadratiches mittel: quadrieren, summieren, teilen, wurzeln
            result += value*value;
        }
    }
    return sqrt(result/(pow(VAR_SIZE*2.0+1.0, 2.0)-1.0));
}

//Michelson-Kontrast
float varianceMeasure_michelson_contrast(in sampler2D texSampler, in vec2 fragCoord){
    vec3 minValues = vec3(1.0, 1.0, 1.0);
    vec3 maxValues = vec3(0.0, 0.0, 0.0);
    
    for (float i = -VAR_SIZE; i <= VAR_SIZE; ++i) {
        for (float j = -VAR_SIZE; j <= VAR_SIZE; ++j) {
            vec3 color = sampleColor(texSampler, (fragCoord + vec2(float(i), float(j)) / u_resolution));
            minValues = min(minValues, color);
            maxValues = max(maxValues, color);
        }
    }
    vec3 contrast = (maxValues-minValues)/(maxValues+minValues);
    return (contrast.r + contrast.g + contrast.b) / 3.0;
}

float sampleVariance(in sampler2D texSampler, in vec2 fragCoord){
    if(u_variance_measure == MEASURE_VARIANCE_FIRST){
        return varianceMeasure_first(texSampler, fragCoord);
    }else if(u_variance_measure == MEASURE_VARIANCE_AVG){
        return varianceMeasure_avg(texSampler, fragCoord);
    }else if(u_variance_measure == MEASURE_VARIANCE_VAR_AVG){
        return varianceMeasure_var_avg(texSampler, fragCoord);
    }else if(u_variance_measure == MEASURE_VARIANCE_FIRST_CONTRAST){
        return varianceMeasure_first_contrast(texSampler, fragCoord);
    }else if(u_variance_measure == MEASURE_VARIANCE_DELTA_E){
        return varianceMeasure_delta_e(texSampler, fragCoord);
    }else if(u_variance_measure == MEASURE_VARIANCE_MICHELSON_CONTRAST){
        return varianceMeasure_michelson_contrast(texSampler, fragCoord);
    }else{
        return 0.0;
    }
}

float global_variance(in vec2 fragCoord, in sampler2D tex, vec2 resolution) { // iterate over entire texture
    float result = 0.0;

    //read out the texels
    vec3 main_color = texture(tex, fragCoord).rgb;
    for (float x = 0.5; x < u_resolution.x; x+=10.0) {
        for (float y = 0.5; y < u_resolution.y; y+=10.0) {
            vec3 diff = abs(main_color - texture(tex, vec2(x, y) / resolution).rgb);
            result += diff.x + diff.y + diff.z;
        }
    }

    // the average of the weighted samples and hence the result
    return result / (u_resolution.x * u_resolution.y * 3.0*10.0*10.0);
}

vec3 TurboColormap(in float x) {
  const vec4 kRedVec4 = vec4(0.13572138, 4.61539260, -42.66032258, 132.13108234);
  const vec4 kGreenVec4 = vec4(0.09140261, 2.19418839, 4.84296658, -14.18503333);
  const vec4 kBlueVec4 = vec4(0.10667330, 12.64194608, -60.58204836, 110.36276771);
  const vec2 kRedVec2 = vec2(-152.94239396, 59.28637943);
  const vec2 kGreenVec2 = vec2(4.27729857, 2.82956604);
  const vec2 kBlueVec2 = vec2(-89.90310912, 27.34824973);
  
  x = clamp(x, 0.0, 1.0);

  vec4 v4 = vec4( 1.0, x, x * x, x * x * x);
  vec2 v2 = v4.zw * v4.z;
  return vec3(
    dot(v4, kRedVec4)   + dot(v2, kRedVec2),
    dot(v4, kGreenVec4) + dot(v2, kGreenVec2),
    dot(v4, kBlueVec4)  + dot(v2, kBlueVec2)
  );
}

void main() {
    //loss /= variance_original+0.01
    //loss = loss/2.0 + 0.5;
    //loss = clamp(loss, 0.0, 1.0);    
    if(u_show_variance == SHOW_VARIANCE_PRE){
        float variance_original = sampleVariance(s_original, v_tex);
        rt_color = vec4(TurboColormap(variance_original), 1.0);
        rt_measure = vec4(variance_original);
    }else if(u_show_variance == SHOW_VARIANCE_POST){
        float variance_sim = sampleVariance(s_color, v_tex);
        rt_color = vec4(TurboColormap(variance_sim), 1.0);
        rt_measure = vec4(variance_sim);
    }else if(u_show_variance == SHOW_VARIANCE_DIFF){
        float variance_original = sampleVariance(s_original, v_tex);
        float variance_sim = sampleVariance(s_color, v_tex);
        float loss = variance_original-variance_sim;
        rt_color = vec4(TurboColormap(loss), 1.0);
        rt_measure = vec4(loss);
    }else{//SHOW_VARIANCE_NONE
        rt_color = vec4(sampleColor(s_color, v_tex), 1.0);
        rt_measure = vec4(0.0);
    }

    if(u_track_error==1){
        rt_deflection = texture(s_deflection, v_tex);
        rt_color_change = texture(s_color_change, v_tex);
        rt_color_uncertainty = texture(s_color_uncertainty, v_tex);
        rt_covariances = texture(s_covariances, v_tex);
    }
}
