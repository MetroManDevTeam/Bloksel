#version 450
layout(location = 0) out vec4 fragColor;

layout(set = 0, binding = 1) uniform Material {
    vec3 albedo;
    float roughness;
    float metallic;
    int hasVariants;
    vec3 variantAlbedoMod;
    float roughnessMod;
    float metallicMod;
} material;

layout(set = 0, binding = 2) uniform sampler2DArray textureAtlas;

layout(set = 0, binding = 3) uniform LightData {
    vec3 viewPos;
    vec3 lightPos;
    float time;
    int connectedDirections;
} light;

layout(location = 0) in vec3 fragPos;
layout(location = 1) in vec3 normal;
layout(location = 2) in vec2 texCoord;
layout(location = 3) flat in uint blockId;
layout(location = 4) flat in uint variantData;

const float PI = 3.14159265359;

vec3 fresnelSchlick(float cosTheta, vec3 F0) {
    return F0 + (1.0 - F0) * pow(clamp(1.0 - cosTheta, 0.0, 1.0), 5.0);
}

float DistributionGGX(vec3 N, vec3 H, float roughness) {
    float a = roughness * roughness;
    float a2 = a * a;
    float NdotH = max(dot(N, H), 0.0);
    float NdotH2 = NdotH * NdotH;
    return a2 / (PI * pow(NdotH2 * (a2 - 1.0) + 1.0, 2.0));
}

vec2 get_connected_uv(uint connections, vec2 uv) {
    ivec2 texSize = textureSize(textureAtlas, 0).xy;
    vec2 pixelUV = uv * texSize;
    
    // Horizontal connections
    if ((connections & 0x3u) != 0u) {
        if (pixelUV.x < 2.0) pixelUV.x += 2.0;
        if (pixelUV.x > texSize.x - 2.0) pixelUV.x -= 2.0;
    }
    
    // Vertical connections
    if ((connections & 0xCu) != 0u) {
        if (pixelUV.y < 2.0) pixelUV.y += 2.0;
        if (pixelUV.y > texSize.y - 2.0) pixelUV.y -= 2.0;
    }
    
    return pixelUV / texSize;
}

void main() {
    // Extract variant data
    uint variantId = (variantData >> 16) & 0xFFFFu;
    uint facingBits = variantData & 0xFFFFu;
    
    // Calculate final material properties
    vec3 finalAlbedo = material.albedo;
    float finalRoughness = material.roughness;
    float finalMetallic = material.metallic;
    
    if (material.hasVariants == 1) {
        finalAlbedo *= material.variantAlbedoMod;
        finalRoughness = clamp(finalRoughness + material.roughnessMod, 0.0, 1.0);
        finalMetallic = clamp(finalMetallic + material.metallicMod, 0.0, 1.0);
    }

    // Calculate connected texture coordinates
    vec2 adjustedUV = get_connected_uv(uint(light.connectedDirections), texCoord);
    
    // Sample texture array using combined ID
    uint textureIndex = blockId * 16u + variantId;
    vec4 texColor = texture(textureAtlas, vec3(adjustedUV, float(textureIndex)));
    
    // PBR lighting calculations
    vec3 N = normalize(normal);
    vec3 V = normalize(light.viewPos - fragPos);
    vec3 F0 = mix(vec3(0.04), finalAlbedo, finalMetallic);

    // Direct lighting
    vec3 L = normalize(light.lightPos - fragPos);
    vec3 H = normalize(V + L);
    float NDF = DistributionGGX(N, H, finalRoughness);
    vec3 F = fresnelSchlick(max(dot(H, V), 0.0), F0);
    vec3 kS = F;
    vec3 kD = (vec3(1.0) - kS) * (1.0 - finalMetallic);

    float NdotL = max(dot(N, L), 0.0);
    vec3 radiance = vec3(1.0) * NdotL;

    vec3 Lo = (kD * finalAlbedo / PI + NDF * F) * radiance;
    vec3 ambient = vec3(0.03) * finalAlbedo;
    vec3 color = ambient + Lo;

    fragColor = vec4(color * texColor.rgb, texColor.a);
}
