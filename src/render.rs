use cairo::Context;
use crate::state::Stroke;

pub fn draw_stroke(cr: &Context, stroke: &Stroke) {
    if stroke.points.is_empty() { return; }
    
    if stroke.is_eraser {
        cr.set_operator(cairo::Operator::Clear);
    } else {
        cr.set_operator(cairo::Operator::Over);
    }
    
    cr.set_source_rgba(stroke.color.0, stroke.color.1, stroke.color.2, 1.0);
    cr.set_line_width(stroke.width);
    cr.set_line_cap(cairo::LineCap::Round);
    cr.set_line_join(cairo::LineJoin::Round);
    
    let first = &stroke.points[0];
    cr.move_to(first.x, first.y);
    for p in &stroke.points[1..] {
        cr.line_to(p.x, p.y);
    }
    cr.stroke().unwrap();
}