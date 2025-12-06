#version 450

layout(location = 0) out vec4 out_colour;


struct directional_light {
  vec3 direction;
  vec4 colour;
};

struct point_light {
  vec3 position;
  vec4 colour;
  float constant;
  float linear;
  float quadratic;
};

directional_light dir_light = {
  //vec3(0,0,0 ),
 // vec3(-4.7, -5.5, -5.5),
  vec3(-0.57735, -2.07735, -6.27735),
  vec4(0.8, 0.8, 0.8, 1.0)
};

point_light p_light_0 = {
  vec3( 1.0, 0, 0),
  vec4(0.0, 1.0, 0.0, 1.0),
  1.0,
  0.35,
  0.44,
};

point_light p_light_1 = {
  vec3(0.5, 0.5, 0.5),
  vec4(1.0, 0.0, 0.0, 1.0),
  1.0,
  0.35,
  0.44,
};

layout(set = 1, binding = 1) uniform sampler2D samplers[3];
const int SAMP_DIFFUSE = 0;
const int SAMP_SPECULAR = 1;
const int SAMP_NORMAL = 2;

layout(location = 0) flat in int in_mode;

layout(location = 1) in struct dto {
  vec4 ambient;
  vec2 tex_coord;
  vec3 normal;
  vec3 view_position;
  vec3 frag_position;
  vec4 colour;
  vec4 tangent;
  vec4 diffuse_colour;
  vec4 shininess;
} in_dto;

mat3 TBN;

vec4 calculate_directional_light(directional_light light, vec3 normal, vec3 view_direction);
vec4 calculate_point_light(point_light light, vec3 normal, vec3 frag_position, vec3 view_direction);

void main() { 
  vec3 normal = in_dto.normal;
  vec3 tangent = in_dto.tangent.xyz;
  tangent = (tangent - dot(tangent, normal) * normal);
  vec3 bitangent = cross(in_dto.normal, in_dto.tangent.xyz) * in_dto.tangent.w;
  TBN = mat3(tangent, bitangent, normal);

  vec3 localNormal = 2.0 * texture(samplers[SAMP_NORMAL], in_dto.tex_coord).rgb - 1.0;
  normal = normalize(TBN * localNormal);

  if (in_mode == 0 || in_mode == 1) {
     //vec3 view_direction = normalize(in_dto.view_position - in_dto.frag_position);
     //out_colour = calculate_directional_light(dir_light, normal, view_direction); 
     //out_colour += calculate_point_light(p_light_0, normal, in_dto.frag_position, view_direction);
     //out_colour += calculate_point_light(p_light_1, normal, in_dto.frag_position, view_direction);
    vec4 diff_samp = texture(samplers[SAMP_DIFFUSE], in_dto.tex_coord);
    out_colour = diff_samp;
  } else if(in_mode == 2) {
    out_colour = vec4(abs(normal), 1.0);
  }
}

vec4 calculate_directional_light(directional_light light, vec3 normal, vec3 view_direction) {
    vec3 N = normalize(in_dto.normal); 
    vec3 T = normalize(in_dto.tangent.xyz); 
    vec3 B = normalize(cross(N, T)) * in_dto.tangent.w;

    mat3 TBN = mat3(T, B, N);

    vec3 normal_sample = texture(samplers[SAMP_NORMAL], in_dto.tex_coord).rgb;
    
    vec3 tangent_space_normal = normalize(normal_sample * 2.0 - 1.0); 

    vec3 final_normal = normalize(TBN * tangent_space_normal); 

    if(in_mode == 2) {
        return vec4(final_normal * 0.5 + 0.5, 1.0);
    }
    

    vec4 diff_samp = texture(samplers[SAMP_DIFFUSE], in_dto.tex_coord);
    vec4 spec_samp = texture(samplers[SAMP_SPECULAR], in_dto.tex_coord); // sampler 1

    vec3 ambient = in_dto.ambient.rgb * diff_samp.rgb;

    vec3 light_direction = -normalize(light.direction);
    float diff_factor = max(dot(final_normal, light_direction), 0.0);
    vec3 diffuse = diff_factor * light.colour.rgb;

    vec3 view_dir = normalize(in_dto.view_position - in_dto.frag_position);
    vec3 half_vector = normalize(light_direction + view_dir);
    
    float spec_factor = pow(max(dot(final_normal, half_vector), 0.0), in_dto.shininess.x);
    
    vec3 specular = spec_factor * light.colour.rgb * spec_samp.rgb;

    if (in_mode == 0) {
        vec3 final_light = ambient + (diffuse * diff_samp.rgb) + specular;
        return vec4(final_light, diff_samp.a);
    } 

    return vec4(ambient + diffuse + specular, 1.0);
}

vec4 calculate_point_light(point_light light, vec3 normal, vec3 frag_position, vec3 view_direction) {
  vec3 light_direction =  normalize(light.position - frag_position);
    float diff = max(dot(normal, light_direction), 0.0);

    vec3 reflect_direction = reflect(-light_direction, normal);
    float spec = pow(max(dot(view_direction, reflect_direction), 0.0), in_dto.shininess.x);

    // Calculate attenuation, or light falloff over distance.
    float distance = length(light.position - frag_position);
    float attenuation = 1.0 / (light.constant + light.linear * distance + light.quadratic * (distance * distance));

    vec4 ambient = in_dto.ambient;
    vec4 diffuse = light.colour * diff;
    vec4 specular = light.colour * spec;
    
    if(in_mode == 0) {
        vec4 diff_samp = texture(samplers[SAMP_DIFFUSE], in_dto.tex_coord);
        diffuse *= diff_samp;
        ambient *= diff_samp;
        specular *= vec4(texture(samplers[SAMP_SPECULAR], in_dto.tex_coord).rgb, diffuse.a);
    }

    ambient *= attenuation;
    diffuse *= attenuation;
    specular *= attenuation;
    return (ambient + diffuse + specular);
}
