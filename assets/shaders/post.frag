#version 330

// ---------------------------------------------------------------------------
// rs-3d-viewer  |  post.frag
// Post-processing fragment shader applied when blitting the internal render
// texture to the display window.
//
// Responsibilities
// ----------------
//  1. Custom upscaling: bilinear (mode 2) or bicubic / Catmull-Rom (mode 3).
//     For Point (0) and Linear (1) the GPU's own sampler handles it --
//     we just pass through.
//  2. Vignette: subtle edge darkening for a "monitor" feel.
// ---------------------------------------------------------------------------

in vec2 fragTexCoord;           // UV [0..1] from the fullscreen quad
in vec4 fragColor;              // Raylib tint color (WHITE in normal use)

// Uniforms
uniform sampler2D texture0;     // The internal render texture
uniform vec4      colDiffuse;   // Raylib tint (always WHITE here)
uniform vec2      u_resolution; // Internal render resolution (e.g. 320x240)
uniform float     u_time;       // Elapsed time in seconds
uniform float     u_filterMode; // 0=point  1=linear  2=bilinear  3=cubic

// Output
out vec4 finalColor;

// ---------------------------------------------------------------------------
// Manual bilinear sample.
// Used when the GPU filter is POINT and we want software bilinear.
// Performs a 2x2 gather and lerps.
// ---------------------------------------------------------------------------
vec4 sampleBilinear(sampler2D tex, vec2 uv)
{
    vec2 texel = 1.0 / u_resolution;

    // Place sample at the center of the texel grid
    vec2 st   = uv * u_resolution - 0.5;
    vec2 frac = fract(st);
    vec2 base = (floor(st) + 0.5) / u_resolution;

    vec4 c00 = texture(tex, base);
    vec4 c10 = texture(tex, base + vec2(texel.x, 0.0));
    vec4 c01 = texture(tex, base + vec2(0.0,     texel.y));
    vec4 c11 = texture(tex, base + texel);

    return mix(mix(c00, c10, frac.x),
               mix(c01, c11, frac.x), frac.y);
}

// ---------------------------------------------------------------------------
// Catmull-Rom cubic weights for a single axis.
// t : fractional position in [0, 1]
// Returns weights for the 4 surrounding samples (p-1, p0, p+1, p+2).
// ---------------------------------------------------------------------------
vec4 catmullRomWeights(float t)
{
    float t2 = t * t;
    float t3 = t2 * t;
    return vec4(
        -0.5*t3 + 1.0*t2 - 0.5*t,           // w[-1]
         1.5*t3 - 2.5*t2          + 1.0,     // w[ 0]
        -1.5*t3 + 2.0*t2 + 0.5*t,            // w[+1]
         0.5*t3 - 0.5*t2                     // w[+2]
    );
}

// ---------------------------------------------------------------------------
// Bicubic (Catmull-Rom) sample >> 4x4 tap.
// ---------------------------------------------------------------------------
vec4 sampleBicubic(sampler2D tex, vec2 uv)
{
    vec2 texel = 1.0 / u_resolution;
    vec2 px    = uv * u_resolution - 0.5;
    vec2 frac  = fract(px);

    // Top-left corner of the 4x4 neighbourhood
    vec2 p0    = (floor(px) - 0.5) / u_resolution;

    vec4 xw = catmullRomWeights(frac.x);
    vec4 yw = catmullRomWeights(frac.y);

    vec4 result = vec4(0.0);
    for (int j = 0; j < 4; j++)
    {
        vec4 row = vec4(0.0);
        for (int i = 0; i < 4; i++)
        {
            row += xw[i] * texture(tex, p0 + vec2(float(i) * texel.x,
                                                   float(j) * texel.y));
        }
        result += yw[j] * row;
    }
    return result;
}

// ---------------------------------------------------------------------------
// Vignette: radial falloff towards screen edges.
// Returns a [0..1] multiplier; 1.0 at center, <1.0 at edges.
// ---------------------------------------------------------------------------
float vignette(vec2 uv)
{
    vec2  v       = uv - 0.5;
    float dist    = dot(v, v);       // squared distance from center
    float strength = 1.4;            // tweak to taste
    return clamp(1.0 - dist * strength, 0.0, 1.0);
}

// ---------------------------------------------------------------------------
void main()
{
    // ---- 1. Upscaling sample ----
    vec4 color;

    if (u_filterMode > 2.5)          // mode 3 -- bicubic
    {
        color = sampleBicubic(texture0, fragTexCoord);
    }
    else if (u_filterMode > 1.5)     // mode 2 -- bilinear (software)
    {
        color = sampleBilinear(texture0, fragTexCoord);
    }
    else                             // mode 0 or 1 -- handled by GPU sampler
    {
        color = texture(texture0, fragTexCoord);
    }

    // Post-processing effects

    // Vignette
    color.rgb *= vignette(fragTexCoord);

    // (Placeholder for future effects: bloom, color grading, CRT scanlines...)

    // Apply raylib tint and emit
    finalColor = color * colDiffuse * fragColor;
}

// ---------------------------------------------------------------------------
// HLSL equivalent (for reference / DirectX port):
// ---------------------------------------------------------------------------
// Texture2D    texture0    : register(t0);
// SamplerState sampler0    : register(s0);
// float2       u_resolution : register(b0);  // in a cbuffer
// float        u_filterMode : register(b0);
//
// float4 PSMain(float2 uv : TEXCOORD) : SV_Target
// {
//     float4 color = texture0.Sample(sampler0, uv);
//     float2 v     = uv - 0.5;
//     color.rgb   *= saturate(1.0 - dot(v, v) * 1.4);
//     return color;
// }
// ---------------------------------------------------------------------------
