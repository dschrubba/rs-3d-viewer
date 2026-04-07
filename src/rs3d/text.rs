use raylib::prelude::*;
use crate::{DEFAULT_GUI_FONT_SIZE, DEFAULT_GUI_FONT_SPACING};

pub fn draw_default_gui_text(d: &mut RaylibDrawHandle, font: &Font, text: &str, pos: Vector2, color: Color) {
    d.draw_text_ex(font, text, pos, DEFAULT_GUI_FONT_SIZE.as_f32(), DEFAULT_GUI_FONT_SPACING.as_f32(), color);
}

pub fn draw_gui_text(d: &mut RaylibDrawHandle, font: &Font, text: &str, pos: Vector2, font_size: &i32, font_spacing: &i32, color: Color) {
    d.draw_text_ex(font, text, pos, font_size.as_f32(), font_spacing.as_f32(), color);
}
