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
pub enum Tool { Pen, Eraser }

#[derive(Clone)]
pub struct Point { pub x: f64, pub y: f64 }

#[derive(Clone)]
pub struct Stroke {
    pub points: Vec<Point>,
    pub color: (f64, f64, f64),
    pub width: f64,
    pub is_eraser: bool,
}

pub struct AppState {
    pub strokes: Vec<Stroke>,
    pub undo_stack: Vec<Stroke>, // Silinen hamleleri burada tutarız
    pub current_stroke: Option<Stroke>,
    pub tool: Tool,
    pub color: (f64, f64, f64),
    pub brush_size: f64,
    pub passthrough: bool,
    pub drawing: bool,
}

impl AppState {
    pub fn new() -> Self {
        let cfg: AppConfig = confy::load("tmtcopen", "config").unwrap_or_default();
        AppState {
            strokes: Vec::new(),
            undo_stack: Vec::new(),
            current_stroke: None,
            tool: Tool::Pen,
            color: cfg.color,
            brush_size: cfg.brush_size,
            passthrough: false,
            drawing: false,
        }
    }

    pub fn save_config(&self) {
        let cfg = AppConfig { color: self.color, brush_size: self.brush_size };
        let _ = confy::store("tmtcopen", "config", cfg);
    }
}