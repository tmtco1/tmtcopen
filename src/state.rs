
use cairo::ImageSurface;
use gdk_pixbuf::Pixbuf;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
    pub active_strokes: HashMap<u32, Stroke>,
    pub tool: Tool,
    pub color: (f64, f64, f64),
    pub brush_size: f64,
    pub passthrough: bool,
    pub view_mode: ViewMode,
    pub zoom_image: Option<Pixbuf>,
    pub zoom_offset_x: f64,
    pub zoom_offset_y: f64,
    pub committed_surface: Option<ImageSurface>,
    pub surface_width: i32,
    pub surface_height: i32,
    pub device_scale: f64,
}

fn dirty_rect(x0: f64, y0: f64, x1: f64, y1: f64, width: f64) -> (i32, i32, i32, i32) {
    let pad = width + 2.0;
    let rx = (x0.min(x1) - pad).floor() as i32;
    let ry = (y0.min(y1) - pad).floor() as i32;
    let rw = ((x0.max(x1) - x0.min(x1)) + pad * 2.0).ceil() as i32;
    let rh = ((y0.max(y1) - y0.min(y1)) + pad * 2.0).ceil() as i32;
    (rx, ry, rw.max(1), rh.max(1))
}

impl AppState {
    pub fn new() -> Self {
        let cfg: AppConfig =
            confy::load("tmtcopen", "config")
                .unwrap_or_default();

        Self {
            strokes: Vec::new(),
            undo_stack: Vec::new(),
            active_strokes: HashMap::new(),
            tool: Tool::Pen,
            color: cfg.color,
            brush_size: cfg.brush_size,
            passthrough: false,
            view_mode: ViewMode::Desktop,
            zoom_image: None,
            zoom_offset_x: 0.0,
            zoom_offset_y: 0.0,
            committed_surface: None,
            surface_width: 0,
            surface_height: 0,
            device_scale: 1.0,
        }
    }

    pub fn ensure_surface(&mut self, width: i32, height: i32, device_scale: f64) {
        let scale = if device_scale > 0.0 { device_scale } else { 1.0 };

        let needs_new = self.committed_surface.is_none()
            || self.surface_width != width
            || self.surface_height != height
            || (self.device_scale - scale).abs() > f64::EPSILON;

        if needs_new {
            let phys_w = (width as f64 * scale).round() as i32;
            let phys_h = (height as f64 * scale).round() as i32;

            let surface =
                ImageSurface::create(cairo::Format::ARgb32, phys_w, phys_h)
                    .expect("Cairo surface oluşturulamadı");

            surface.set_device_scale(scale, scale);

            self.committed_surface = Some(surface);
            self.surface_width = width;
            self.surface_height = height;
            self.device_scale = scale;
            self.redraw_committed();
        }
    }

    pub fn redraw_committed(&mut self) {
        use crate::render::draw_stroke;
        use cairo::Context;

        let surface = match &self.committed_surface {
            Some(s) => s,
            None => return,
        };

        let cr = Context::new(surface).unwrap();

        cr.set_operator(cairo::Operator::Clear);
        cr.paint().unwrap();
        cr.set_operator(cairo::Operator::Over);

        for stroke in &self.strokes {
            draw_stroke(&cr, stroke);
        }
    }

    pub fn commit_stroke(&mut self, stroke: &Stroke) {
        use crate::render::draw_stroke;
        use cairo::Context;

        let surface = match &self.committed_surface {
            Some(s) => s,
            None => return,
        };

        let cr = Context::new(surface).unwrap();
        draw_stroke(&cr, stroke);
    }

    pub fn begin_stroke(&mut self, seq: u32, x: f64, y: f64) {
        let stroke_width = if self.tool == Tool::Eraser {
            self.brush_size * 5.0
        } else {
            self.brush_size
        };

        let mut points = Vec::with_capacity(256);
        points.push(Point { x, y });
        points.push(Point { x: x + 0.1, y: y + 0.1 });

        let stroke = Stroke {
            points,
            color: self.color,
            width: stroke_width,
            is_eraser: self.tool == Tool::Eraser,
        };

        self.active_strokes.insert(seq, stroke);
    }

    pub fn extend_stroke(&mut self, seq: u32, x: f64, y: f64) -> Option<(i32, i32, i32, i32)> {
        let stroke = self.active_strokes.get_mut(&seq)?;
        let last = stroke.points.last()?;
        let lx = last.x;
        let ly = last.y;
        let w = stroke.width;
        stroke.points.push(Point { x, y });
        Some(dirty_rect(lx, ly, x, y, w))
    }

    pub fn end_stroke(&mut self, seq: u32) {
        if let Some(stroke) = self.active_strokes.remove(&seq) {
            self.undo_stack.clear();
            self.commit_stroke(&stroke);
            self.strokes.push(stroke);
        }
    }

    pub fn save_config(&self) {
        let cfg = AppConfig {
            color: self.color,
            brush_size: self.brush_size,
        };

        let _ = confy::store("tmtcopen", "config", cfg);
    }
}
