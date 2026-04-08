#version 330

in vec2 fragTexCoord;           // UV [0..1] from the fullscreen quad
in vec4 fragColor;              // Raylib tint color (WHITE in normal use)

// Uniforms
uniform sampler2D texture0;     // The internal render texture
uniform vec4      colDiffuse;   // Raylib tint (always WHITE here)
uniform vec2      u_resolution; // Internal render resolution (e.g. 320x240)
uniform float     u_time;       // Elapsed time in seconds
uniform float     u_filterMode; // 0=point  1=linear  2=bilinear  3=cubic

// PS1 dither table from PSY-Q Docs:
// https://psx.arthus.net/sdk/Psy-Q/DOCS/LIBREF46.PDF, PDF page 242
const int psx_dither_table[16] = int[16](
     0,  8,  2, 10,
    12,  4, 14,  6,
     3, 11,  1,  9,
    15,  7, 13,  5
);

// Helper to get dither value from 1D array using 2D coordinates
int get_dither(int x, int y) {
    int xi = x & 3; // faster + safe modulo 4
    int yi = y & 3;
    return psx_dither_table[yi * 4 + xi];
}

// Adds PSX style dithering only
vec3 PSXDitherOnly(vec3 col, vec2 uv, float ditherStr)
{
    ivec2 p = ivec2(uv * u_resolution);
    int dither_i = get_dither(p.x, p.y);
    col += (float(dither_i) / 2.0 - 4.0) * ditherStr;
    return col;
}

// col -> your high-precision color input
// p   -> screen position in pixel space
vec3 PSXDither(vec3 col, vec2 uv){
  //extrapolate 16bit color float to 16bit integer space
  col*=255.0;

  // Apply dithering according to PSY-Q Docs
  // Convert to integer pixels first, then use modulo
  ivec2 p = ivec2(uv * u_resolution);
  int dither_i = get_dither(p.x, p.y);
  col += (float(dither_i) / 2.0 - 4.0);

  //truncate to 5bpc precision via bitwise AND operator, and limit value max to prevent wrapping.
  //PS1 colors in default color mode have a maximum integer value of 248 (0xf8)
  uvec3 icol = uvec3(clamp(col, 0.0, 255.0));
  icol = icol & 0xF8u;
  col = mix(vec3(icol), vec3(248.0), step(248.0, col));

  //bring color back to floating point number space
  col /= 255;
  return col;
}

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
// Main Shader
// ---------------------------------------------------------------------------

// Output
out vec4 finalColor;
void main()
{
    vec4 color;

    // Upscaling
    if (u_filterMode > 2.5)
    {
        // mode 3      -> bicubic
        color = sampleBicubic(texture0, fragTexCoord);
    }
    else if (u_filterMode > 1.5)
    {
        // mode 2      -> bilinear (software)
        color = sampleBilinear(texture0, fragTexCoord);
    }
    else
    {
        // mode 0 or 1 -> handled by GPU sampler
        color = texture(texture0, fragTexCoord);
    }

    // Post-processing effects
    color.rgb = PSXDither(color.rgb, fragTexCoord);

    // Vignette
    // color.rgb *= vignette(fragTexCoord);

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
