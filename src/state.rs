// state.rs

use gdk_pixbuf::Pixbuf;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AppConfig {
    pub color: (f64, f64, f64),
    pub brush_size: f64,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            color: (1.0, 0.0, 0.0),
            brush_size: 6.0,
        }
    }
}

#[derive(Clone, PartialEq)]
pub enum Tool {
    Pen,
    Eraser,
}

#[derive(Clone, PartialEq)]
pub enum ViewMode {
    Desktop,
    Zoomed,
}

#[derive(Clone)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

#[derive(Clone)]
pub struct Stroke {
    pub points: Vec<Point>,
    pub color: (f64, f64, f64),
    pub width: f64,
    pub is_eraser: bool,
}

pub struct AppState {
    pub strokes: Vec<Stroke>,
    pub undo_stack: Vec<Stroke>,
    pub current_stroke: Option<Stroke>,
    pub tool: Tool,
    pub color: (f64, f64, f64),
    pub brush_size: f64,
    pub passthrough: bool,
    pub drawing: bool,
    pub view_mode: ViewMode,
    pub zoom_image: Option<Pixbuf>,
    pub zoom_offset_x: f64,
    pub zoom_offset_y: f64,
}

impl AppState {
    pub fn new() -> Self {
        let cfg: AppConfig =
            confy::load("tmtcopen", "config")
                .unwrap_or_default();

        Self {
            strokes: Vec::new(),
            undo_stack: Vec::new(),
            current_stroke: None,
            tool: Tool::Pen,
            color: cfg.color,
            brush_size: cfg.brush_size,
            passthrough: false,
            drawing: false,
            view_mode: ViewMode::Desktop,
            zoom_image: None,
            zoom_offset_x: 0.0,
            zoom_offset_y: 0.0,
        }
    }

    pub fn save_config(&self) {
        let cfg = AppConfig {
            color: self.color,
            brush_size: self.brush_size,
        };

        let _ = confy::store(
            "tmtcopen",
            "config",
            cfg,
        );
    }
}