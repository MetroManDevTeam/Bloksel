#version 330 core
in vec3 FragPos;
in vec3 Normal;
in vec2 TexCoord;
flat in uint BlockId;
flat in uint VariantData;

out vec4 FragColor;

uniform sampler2D textureAtlas;
uniform vec3 lightPos;
uniform vec3 viewPos;

void main() {
    // Basic lighting
    vec3 lightDir = normalize(lightPos - FragPos);
    vec3 viewDir = normalize(viewPos - FragPos);
    vec3 normal = normalize(Normal);
    
    // Ambient
    float ambientStrength = 0.1;
    vec3 ambient = ambientStrength * vec3(1.0);
    
    // Diffuse
    float diff = max(dot(normal, lightDir), 0.0);
    vec3 diffuse = diff * vec3(1.0);
    
    // Specular
    float specularStrength = 0.5;
    vec3 reflectDir = reflect(-lightDir, normal);
    float spec = pow(max(dot(viewDir, reflectDir), 0.0), 32);
    vec3 specular = specularStrength * spec * vec3(1.0);
    
    // Combine lighting with texture
    vec4 texColor = texture(textureAtlas, TexCoord);
    vec3 result = (ambient + diffuse + specular) * texColor.rgb;
    FragColor = vec4(result, texColor.a);
} 