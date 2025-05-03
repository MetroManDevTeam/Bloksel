#version 450

layout(location = 0) in vec2 vTexCoord;
layout(location = 1) in vec3 vNormal;
layout(location = 2) in float vAO;
layout(location = 3) in float vLight;

layout(location = 0) out vec4 fColor;

layout(set = 1, binding = 0) uniform texture2D uTexture;
layout(set = 1, binding = 1) uniform sampler uSampler;

uniform float uTime;

void main() {
    // Animate texture coordinates
    vec2 uv = vTexCoord + vec2(sin(uTime * 0.5 + vTexCoord.y) * 0.02;
    
    vec4 texColor = texture(sampler2D(uTexture, uSampler), uv);
    
    // Water-specific effects
    float fresnel = pow(1.0 - abs(dot(normalize(vNormal), vec3(0.0, 1.0, 0.0))), 2.0);
    vec3 waterColor = mix(
        texColor.rgb * 0.8,
        vec3(0.2, 0.4, 0.8),
        fresnel * 0.7
    );
    
    // Transparency and refraction effect
    float alpha = 0.7 + fresnel * 0.2;
    
    fColor = vec4(waterColor * vLight * vAO, alpha);
}
