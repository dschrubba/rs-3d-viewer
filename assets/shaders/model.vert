#version 330

// ---------------------------------------------------------------------------
// rs-3d-viewer  |  model.vert
// Vertex shader for 3D model rendering.
//
// Key feature: UVs are declared "noperspective" which forces the GPU to
// interpolate them linearly across the triangle (affine texture mapping).
// This produces the classic PS1-style UV warping on oblique surfaces.
// ---------------------------------------------------------------------------

// ---- Vertex attributes (names match raylib defaults for auto-binding) ----
layout(location = 0) in vec3 vertexPosition;
layout(location = 1) in vec2 vertexTexCoord;
layout(location = 2) in vec3 vertexNormal;
layout(location = 3) in vec4 vertexColor;

// ---- Standard raylib uniforms (set automatically by rlgl) ----
uniform mat4 mvp;       // Model-View-Projection matrix
uniform mat4 matModel;  // Model matrix (world transform)
uniform mat4 matNormal; // Normal matrix (inverse transpose of model)

// ---- Outputs to fragment shader ----
noperspective out vec2 fragTexCoord;    // Affine UV -- no perspective correction!
out            vec4 fragColor;          // Vertex color (passed through)
out            vec3 fragNormalWorld;    // Normal in world space

void main()
{
    // Pass UV through without any perspective weighting.
    // The "noperspective" qualifier on the 'in' side of the fragment shader
    // ensures the GPU uses simple linear interpolation, giving affine mapping.
    fragTexCoord     = vertexTexCoord;
    fragColor        = vertexColor;

    // Transform normal to world space using the normal matrix.
    // mat3(matNormal) strips the translation component.
    fragNormalWorld  = normalize(mat3(matNormal) * vertexNormal);

    gl_Position = mvp * vec4(vertexPosition, 1.0);
}
