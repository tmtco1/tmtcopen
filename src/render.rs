use crate::state::{Stroke, Tool};
use cairo::Context;

pub fn draw_stroke(cr: &Context, stroke: &Stroke) {
    if stroke.points.len() < 2 {
        return;
    }

    if stroke.is_eraser {
        cr.set_operator(cairo::Operator::Clear);
    } else if stroke.tool == Tool::Highlighter {
        cr.set_operator(cairo::Operator::Over);
    } else {
        cr.set_operator(cairo::Operator::Over);
    }

    if stroke.tool == Tool::Highlighter {
        cr.set_source_rgba(stroke.color.0, stroke.color.1, stroke.color.2, 0.5);
    } else {
        cr.set_source_rgba(stroke.color.0, stroke.color.1, stroke.color.2, 1.0);
    }

    cr.set_line_width(stroke.width);

    match stroke.tool {
        Tool::StraightLine | Tool::DashedLine => {
            cr.set_line_cap(cairo::LineCap::Round);
            cr.set_line_join(cairo::LineJoin::Round);
            if stroke.tool == Tool::DashedLine {
                let dash_len = stroke.width * 2.0;
                let gap_len = stroke.width * 2.5;
                cr.set_dash(&[dash_len, gap_len], 0.0);
            }
        }
        Tool::Highlighter => {
            cr.set_line_cap(cairo::LineCap::Round);
            cr.set_line_join(cairo::LineJoin::Round);
        }
        _ => {
            cr.set_line_cap(cairo::LineCap::Round);
            cr.set_line_join(cairo::LineJoin::Round);
        }
    }

    let first = &stroke.points[0];
    cr.move_to(first.x, first.y);
    for p in &stroke.points[1..] {
        cr.line_to(p.x, p.y);
    }
    cr.stroke().unwrap();

    cr.set_dash(&[], 0.0);
}

pub fn draw_line_preview(
    cr: &Context,
    start_x: f64,
    start_y: f64,
    end_x: f64,
    end_y: f64,
    color: (f64, f64, f64),
    width: f64,
    is_dashed: bool,
) {
    cr.set_operator(cairo::Operator::Over);
    cr.set_source_rgba(color.0, color.1, color.2, 1.0);
    cr.set_line_width(width);
    cr.set_line_cap(cairo::LineCap::Round);
    cr.set_line_join(cairo::LineJoin::Round);

    if is_dashed {
        let dash_len = width * 2.0;
        let gap_len = width * 2.5;
        cr.set_dash(&[dash_len, gap_len], 0.0);
    }

    cr.move_to(start_x, start_y);
    cr.line_to(end_x, end_y);
    cr.stroke().unwrap();

    cr.set_dash(&[], 0.0);
}

pub fn draw_eraser_cursor(cr: &Context, x: f64, y: f64, radius: f64) {
    cr.set_operator(cairo::Operator::Over);
    cr.set_source_rgba(1.0, 1.0, 1.0, 0.8);
    cr.set_line_width(2.0);
    cr.arc(x, y, radius, 0.0, std::f64::consts::PI * 2.0);
    cr.stroke().unwrap();
}
