#version 330

// ---------------------------------------------------------------------------
// rs-3d-viewer  |  model.frag
// Fragment shader for 3D model rendering.
// Samples the diffuse texture and applies simple directional + ambient light.
// ---------------------------------------------------------------------------

// ---- Inputs (must match vertex shader declarations) ----
noperspective in vec2 fragTexCoord;
in            vec4 fragColor;
in            vec3 fragNormalWorld;

// ---- Standard raylib uniforms ----
uniform sampler2D texture0;   // Diffuse / albedo texture (set by raylib)
uniform vec4      colDiffuse; // Material base color tint (set by raylib)

// ---- Output ----
out vec4 finalColor;

void main()
{
    // Sample diffuse texture
    vec4 texSample = texture(texture0, fragTexCoord);

    // Simple single directional light from upper-right (world space)
    vec3  lightDir  = normalize(vec3(0.6, 1.0, 0.4));
    float diffuse   = max(dot(fragNormalWorld, lightDir), 0.0);
    float ambient   = 0.30;

    // Combine ambient + diffuse, keep full alpha
    float lighting  = ambient + (1.0 - ambient) * diffuse;

    // Final color: texture * material tint * vertex color * lighting
    finalColor = texSample
               * colDiffuse
               * fragColor
               * vec4(vec3(lighting), 1.0);
}
