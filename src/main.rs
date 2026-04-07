// =============================================================================
// Simple drag-and-drop 3D model viewer using raylib-rs.
//
// Usage
//   rs-3d-viewer.exe <model_file>
//   or drag a 3D model file onto the executable in Explorer.
//
// Controls
//   Right-click + drag  : Orbit camera around the world origin
//   Mouse wheel         : Zoom in / out
//
// Pipeline
//   1. Render scene  --> internal RenderTexture2D  (INTERNAL_W x INTERNAL_H)
//   2. Blit RT       --> window                    (letterboxed or integer-scaled)
//      with the post.frag shader active
//
// NOTE: raylib uses OpenGL + GLSL.  A reference HLSL translation lives inside
//       post.frag as comments.
// =============================================================================
use raylib::prelude::*;
// RaylibShader trait exposes get_shader_location / set_shader_value directly
// on Shader / WeakShader objects.
use raylib::core::shaders::RaylibShader;

mod rs3d;

// -----------------------------------------------------------------------------
// Hard-coded settings
// (Change these constants to configure the viewer without command-line flags.)
// -----------------------------------------------------------------------------

/// Pixel dimensions of the internal render target.
/// The whole 3D scene is drawn here, then upscaled to the actual window.
const INTERNAL_W: u32 = 640;
const INTERNAL_H: u32 = 480;

/// Starting window size (window is resizable at runtime).
const WINDOW_W: i32 = 1280;
const WINDOW_H: i32 = 960;

/// Upscaling filter applied when blitting the RT to the window.
const UPSCALE: UpscaleFilter = UpscaleFilter::Point;

/// When true, scale is forced to the largest integer multiple that fits.
/// Also forces POINT texture filter regardless of UPSCALE.
const INTEGER_SCALE: bool = false;

/// Orbit sensitivity (radians per pixel of mouse drag).
const ORBIT_SPEED: f32 = 0.005;

/// Zoom sensitivity (world units per scroll notch).
const ZOOM_SPEED: f32 = 0.5;

/// Zoom limits.
const RADIUS_MIN: f32 = 0.5;
const RADIUS_MAX: f32 = 500.0;

// -----------------------------------------------------------------------------
// Shader sources embedded at compile-time.
// Paths are relative to this file (src/main.rs).
// -----------------------------------------------------------------------------
const MODEL_VERT_SRC: &str   = include_str!("../assets/shaders/model.vert");
const MODEL_FRAG_SRC: &str   = include_str!("../assets/shaders/model.frag");
const POST_FRAG_SRC:  &str   = include_str!("../assets/shaders/post.frag");

// -----------------------------------------------------------------------------
// Icon / Image Resources
// -----------------------------------------------------------------------------
const ICON_PNG_32:    &[u8]  = include_bytes!("../assets/icon/icon_32.png");
// const ICON_PNG_256:   &[u8]  = include_bytes!("../assets/icon/icon_256.png");

// -----------------------------------------------------------------------------
// Icon / Image Resources
// -----------------------------------------------------------------------------
const DEFAULT_FNT:    &[u8]  = include_bytes!("../assets/font/IBMPlexMono-Regular.ttf");

// -----------------------------------------------------------------------------
// GUI Defaults
// -----------------------------------------------------------------------------
pub const DEFAULT_GUI_FONT_SIZE:    i32 = 32;
pub const DEFAULT_GUI_FONT_SPACING: i32 = 2;

// -----------------------------------------------------------------------------
// Application Default Colours
// -----------------------------------------------------------------------------
const COLOR_LAMBDA_600:    Color = Color::new(0xFF, 0x66, 0x00, 0xFF);
const COLOR_BONESTORM_100: Color = Color::new(0xF2, 0xF5, 0xF4, 0xFF);

// -----------------------------------------------------------------------------
// Upscale filter
// -----------------------------------------------------------------------------
#[derive(Clone, Copy, PartialEq, Debug)]
enum UpscaleFilter {
    Point,      // Nearest-neighbour  -- GPU sampler
    Linear,     // Bilinear           -- GPU hardware filter
    Bilinear,   // Manual 2x2 sample  -- post shader
    Cubic,      // Catmull-Rom 4x4    -- post shader
}

impl UpscaleFilter {
    /// Float value sent to the post shader as u_filterMode.
    fn shader_mode(self) -> f32 {
        match self {
            UpscaleFilter::Point    => 0.0,
            UpscaleFilter::Linear   => 1.0,
            UpscaleFilter::Bilinear => 2.0,
            UpscaleFilter::Cubic    => 3.0,
        }
    }
}

// -----------------------------------------------------------------------------
// Orbit camera
// -----------------------------------------------------------------------------
struct OrbitCamera {
    yaw:    f32,   // Horizontal angle in radians
    pitch:  f32,   // Vertical angle   in radians (clamped away from poles)
    radius: f32,   // Distance from the world origin
}

impl OrbitCamera {
    fn new() -> Self {
        OrbitCamera {
            yaw:    std::f32::consts::FRAC_PI_4,
            pitch:  0.35,   // ~20 degrees upward
            radius: 5.0,
        }
    }

    /// Apply a mouse-drag rotation (right-button held).
    fn orbit(&mut self, dx: f32, dy: f32) {
        self.yaw   -= dx * ORBIT_SPEED;
        self.pitch += dy * ORBIT_SPEED;
        // Stay within +/- 89 degrees to avoid the pole singularity
        self.pitch  = self.pitch.clamp(-1.55, 1.55);
    }

    /// Zoom with scroll wheel (positive delta = move closer).
    fn zoom(&mut self, delta: f32) {
        self.radius = (self.radius - delta * ZOOM_SPEED)
            .clamp(RADIUS_MIN, RADIUS_MAX);
    }

    /// Build a Camera3D that looks at the world origin.
    fn to_camera3d(&self) -> Camera3D {
        let cos_p = self.pitch.cos();
        let pos   = Vector3 {
            x: self.radius * cos_p * self.yaw.sin(),
            y: self.radius * self.pitch.sin(),
            z: self.radius * cos_p * self.yaw.cos(),
        };
        Camera3D::perspective(pos, Vector3::zero(), Vector3::up(), 45.0)
    }
}

// =============================================================================
// main
// =============================================================================
fn main() {

    // Model path from first CLI argument (drag-and-drop exe target)
    let model_path: Option<String> = std::env::args()
        .nth(1)
        .filter(|p| !p.is_empty());

    if model_path.is_none() {
        eprintln!("[rs-3d-viewer] No model file specified.");
        eprintln!("  Usage: rs-3d-viewer.exe <model_file>");
        eprintln!("  Or drag a 3D model file onto the executable in Explorer.");
    }

    // Window Icon
    let icon_window = Image::load_image_from_mem(".png", ICON_PNG_32)
        .expect("Failed to load embedded icon");

    // Window
    // No MSAA flags >> no anti-aliasing
    let (mut rl, thread) = raylib::init()
        .size(WINDOW_W, WINDOW_H)
        .title("rs-3d-viewer")
        .resizable()
        .build();

    // Default font
    let default_font = &rl.load_font_from_memory(&thread, ".ttf", DEFAULT_FNT, 16, None)
        .expect("Failed to parse font data from memory");

    rl.gui_set_font(default_font);
    rl.set_window_icon(&icon_window);
    rl.set_target_fps(60);

    // Load model
    let model: Option<Model> = model_path.as_ref().and_then(|path| {
        match rl.load_model(&thread, path) {
            Ok(m) => {
                println!("[rs-3d-viewer] Loaded: {}", path);
                Some(m)
            }
            Err(e) => {
                eprintln!("[rs-3d-viewer] Failed to load '{}': {}", path, e);
                None
            }
        }
    });

    // Shaders
    // Model shader: vertex + fragment pair enabling affine UV (noperspective).
    // Applied per-frame via begin_shader_mode, no material assignment needed
    //
    // NOTE: If load_shader_from_memory returns Result<Shader, String> in your
    //       version of raylib-rs, append .expect("model shader") to the call.
    let mut model_shader = rl.load_shader_from_memory(
        &thread,
        Some(MODEL_VERT_SRC),
        Some(MODEL_FRAG_SRC),
    );

    // Post shader: fragment only, vertex defaults to raylib's 2D quad shader
    let mut post_shader = rl.load_shader_from_memory(
        &thread,
        None,
        Some(POST_FRAG_SRC),
    );

    // Uniform locations (-1 means "not found"; set_shader_value ignores those)
    let loc_resolution  = post_shader.get_shader_location("u_resolution");
    let loc_time        = post_shader.get_shader_location("u_time");
    let loc_filter_mode = post_shader.get_shader_location("u_filterMode");

    // Resolution is constant until we recreate the RT, so set it once
    post_shader.set_shader_value(
        loc_resolution,
        Vector2 { x: INTERNAL_W as f32, y: INTERNAL_H as f32 },
    );

    // Internal render texture
    let mut render_tex = rl
        .load_render_texture(&thread, INTERNAL_W, INTERNAL_H)
        .expect("[rs-3d-viewer] Failed to create render texture");

    // Set the GPU-level sampler filter on the RT's colour attachment.
    apply_rt_filter(&render_tex, UPSCALE, INTEGER_SCALE);

    // Camera
    let mut cam        = OrbitCamera::new();
    let mut prev_mouse = Vector2::zero();

    // ==========================================================================
    // Main loop
    // ==========================================================================
    while !rl.window_should_close() {

        // Input
        let cur_mouse = rl.get_mouse_position();
        let mdx = cur_mouse.x - prev_mouse.x;
        let mdy = cur_mouse.y - prev_mouse.y;
        prev_mouse = cur_mouse;

        if rl.is_mouse_button_down(MouseButton::MOUSE_BUTTON_RIGHT) {
            cam.orbit(mdx, mdy);
        }

        let wheel = rl.get_mouse_wheel_move();
        if wheel != 0.0 {
            cam.zoom(wheel);
        }

        let camera3d = cam.to_camera3d();

        // Update per-frame post-shader uniforms
        // (Must happen before begin_texture_mode / begin_drawing borrow rl)
        let elapsed = rl.get_time() as f32;
        post_shader.set_shader_value(loc_time, elapsed);

        let filter_val = if INTEGER_SCALE {
            UpscaleFilter::Point.shader_mode()
        } else {
            UPSCALE.shader_mode()
        };
        post_shader.set_shader_value(loc_filter_mode, filter_val);

        // Render 3D scene to the internal render texture
        {
            let mut rtm = rl.begin_texture_mode(&thread, &mut render_tex);
            rtm.clear_background(Color::BLACK);

            {
                let mut d3 = rtm.begin_mode3D(camera3d);

                // Draw model (or placeholder) with the affine-UV shader.
                // begin_shader_mode activates the shader for draw calls in
                // this scope; shader mode ends automatically on drop.
                {
                    let mut sd = d3.begin_shader_mode(&mut model_shader);

                    match model {
                        Some(ref m) => {
                            // Draw the loaded model at the world origin
                            sd.draw_model(m, Vector3::zero(), 1.0, Color::WHITE);
                        }
                        None => {
                            // Placeholder cube shown when no model is loaded
                            sd.draw_cube(
                                Vector3::zero(),
                                1.0, 1.0, 1.0,
                                Color::new(180, 80, 80, 255),
                            );
                            sd.draw_cube_wires(
                                Vector3::zero(),
                                1.0, 1.0, 1.0,
                                Color::DARKGRAY,
                            );
                        }
                    }
                } // end_shader_mode (shader deactivated before grid)

                // Reference grid, rendered without the custom model shader
                d3.draw_grid(20, 1.0);

            } // end_mode3d

        } // end_texture_mode

        // Blit render texture to the window with post shader
        {
            let win_w = rl.get_screen_width();
            let win_h = rl.get_screen_height();
            let dest  = compute_dest(win_w, win_h, INTEGER_SCALE);
            let mut text_draw_position = Vector2::new(0.0,0.0);

            let mut d = rl.begin_drawing(&thread);
            d.clear_background(Color::BLACK);
            d.gui_set_font(default_font);

            {
                let mut sd = d.begin_shader_mode(&mut post_shader);
                // Source rect spans the full render texture.
                // Height is neg to flip Y: OpenGL stores render texture
                // rows bottom-to-top, but raylib 2D draws top-to-bottom.
                sd.draw_texture_pro(
                    &render_tex,
                    Rectangle {
                        x:      0.0,
                        y:      0.0,
                        width:  INTERNAL_W as f32,
                        height: -(INTERNAL_H as f32),
                    },
                    dest,
                    Vector2::zero(),
                    0.0,
                    Color::WHITE,
                );
            } // end_shader_mode

            // HUD outside post shader so it renders at full window res
            // d.draw_fps(8, 8);

            // Help / filename overlay
            if model.is_none() {
                let cy = win_h / 2;
                text_draw_position = Vector2::new(20.0, cy.as_f32() - 18.0);
                rs3d::text::draw_default_gui_text(&mut d, default_font, "No model loaded.", text_draw_position, COLOR_BONESTORM_100);
            } else if let Some(ref path) = model_path {
                // Show the model filename at the bottom of the screen
                text_draw_position = Vector2::new((win_w - 20).as_f32(), (win_h - 22).as_f32());
                let name = std::path::Path::new(path)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or(path.as_str());
                rs3d::text::draw_default_gui_text(&mut d, default_font, name, text_draw_position, COLOR_BONESTORM_100);
            }

            // Controls reminder (top-right)
            let help = "RMB drag: orbit | Wheel: zoom";
            let tw = d.measure_text(help, DEFAULT_GUI_FONT_SIZE);
            text_draw_position.x = (win_w - tw - 8).as_f32();
            text_draw_position.y = 8.as_f32();
            rs3d::text::draw_default_gui_text(&mut d, default_font, help, text_draw_position, COLOR_BONESTORM_100);

        } // end_drawing

    } // main loop

    // All resources (Model, Shader, RenderTexture2D) are freed by Drop
}

// =============================================================================
// Helpers
// =============================================================================

/// Destination rectangle for drawing the render texture onto the window.
/// Preserves the internal aspect ratio and centres the image (letterbox /
/// pillarbox). In INTEGER_SCALE mode the largest whole-number multiple is used.
fn compute_dest(win_w: i32, win_h: i32, integer_scale: bool) -> Rectangle {
    let (dw, dh) = if integer_scale {
        let n = ((win_w as u32 / INTERNAL_W).min(win_h as u32 / INTERNAL_H)).max(1);
        ((INTERNAL_W * n) as f32, (INTERNAL_H * n) as f32)
    } else {
        let scale = (win_w as f32 / INTERNAL_W as f32)
            .min(win_h as f32 / INTERNAL_H as f32);
        (INTERNAL_W as f32 * scale, INTERNAL_H as f32 * scale)
    };
    Rectangle {
        x:      (win_w as f32 - dw) * 0.5,
        y:      (win_h as f32 - dh) * 0.5,
        width:  dw,
        height: dh,
    }
}

/// Set the GPU texture sampler filter on the render texture's colour attachment.
///
/// For Bilinear and Cubic modes the post shader handles filtering manually,
/// so the GPU filter is left at POINT to avoid double-blurring.
/// For Linear the GPU's built-in bilinear filter is used directly.
fn apply_rt_filter(
    rt:            &RenderTexture2D,
    filter:        UpscaleFilter,
    integer_scale: bool,
) {
    use raylib::ffi::{SetTextureFilter, TextureFilter};

    let mode = if integer_scale
               || filter == UpscaleFilter::Point
               || filter == UpscaleFilter::Bilinear
               || filter == UpscaleFilter::Cubic
    {
        TextureFilter::TEXTURE_FILTER_POINT
    } else {
        // UpscaleFilter::Linear -- let the GPU do a simple bilinear stretch
        TextureFilter::TEXTURE_FILTER_BILINEAR
    };

    // SAFETY:
    //   - rt is a live RenderTexture2D whose colour attachment is a valid
    //     OpenGL texture.
    //   - SetTextureFilter is a thin wrapper around glTexParameteri; it does
    //     not move or free any Rust-owned memory.
    //   - ffi::Texture is a plain-old struct of integers (Copy); reading it
    //     from the RenderTexture2D struct is safe.
    unsafe {
        // as_ref() gives &ffi::RenderTexture2D; .texture is ffi::Texture (Copy).
        SetTextureFilter(rt.texture, mode as i32);
    }
}
