#version 450
layout(location = 0) in vec3 aPos;
layout(location = 1) in vec3 aNormal;
layout(location = 2) in vec2 aTexCoord;
layout(location = 3) in uint aBlockId;
layout(location = 4) in uint aVariantData;

layout(set = 0, binding = 0) uniform UniformBufferObject {
    mat4 model;
    mat4 view;
    mat4 projection;
} ubo;

layout(location = 0) out vec3 fragPos;
layout(location = 1) out vec3 normal;
layout(location = 2) out vec2 texCoord;
layout(location = 3) flat out uint blockId;
layout(location = 4) flat out uint variantData;

void main() {
    fragPos = vec3(ubo.model * vec4(aPos, 1.0));
    normal = mat3(transpose(inverse(ubo.model))) * aNormal;
    texCoord = aTexCoord;
    blockId = aBlockId;
    variantData = aVariantData;
    gl_Position = ubo.projection * ubo.view * vec4(fragPos, 1.0);
}
