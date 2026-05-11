// ui.rs

use crate::render::draw_stroke;
use crate::state::{AppState, Point, Stroke, Tool, ViewMode};
use crate::window_utils::apply_input_shape;

use gdk::prelude::*;
use gdk::{EventMask, RGBA, WindowTypeHint};

use gdk_pixbuf::{InterpType, Pixbuf};

use glib::clone;

use gtk::prelude::*;
use gtk::{
    Application,
    ApplicationWindow,
    Box as GtkBox,
    Button,
    ColorButton,
    Orientation,
    Scale,
    Window,
    WindowType,
};

use std::cell::RefCell;
use std::path::Path;
use std::process::Command;
use std::rc::Rc;

fn capture_area() -> Option<Pixbuf> {
    let path = "/tmp/tmtcopen_zoom.png";

    let status = Command::new("sh")
        .arg("-c")
        .arg(format!("maim -s {}", path))
        .status()
        .ok()?;

    if !status.success() {
        return None;
    }

    Pixbuf::from_file(Path::new(path)).ok()
}

pub fn build_ui(app: &Application) {
    let screen = gdk::Screen::default().unwrap();
    let monitor = screen.display().monitor(0).unwrap();
    let geometry = monitor.geometry();

    let overlay_win =
        Window::new(WindowType::Toplevel);

    overlay_win.set_decorated(false);
    overlay_win.set_app_paintable(true);
    overlay_win.set_keep_above(true);

    overlay_win.set_type_hint(
        WindowTypeHint::Splashscreen,
    );

    overlay_win.set_default_size(
        geometry.width(),
        geometry.height(),
    );

    overlay_win.move_(0, 0);

    if let Some(visual) =
        screen.rgba_visual()
    {
        overlay_win.set_visual(Some(&visual));
    }

    overlay_win.set_events(
        EventMask::BUTTON_PRESS_MASK
            | EventMask::BUTTON_RELEASE_MASK
            | EventMask::POINTER_MOTION_MASK,
    );

    let menu_win =
        ApplicationWindow::builder()
            .application(app)
            .title("")
            .resizable(false)
            .build();

    menu_win.set_keep_above(true);

    menu_win.set_type_hint(
        WindowTypeHint::Dialog,
    );

    menu_win.move_(
        20,
        (geometry.height() / 2) - 150,
    );

    let state =
        Rc::new(RefCell::new(AppState::new()));

    let (r, g, b) =
        state.borrow().color;

    let panel =
        GtkBox::new(Orientation::Vertical, 6);

    panel.set_margin_start(10);
    panel.set_margin_end(10);
    panel.set_margin_top(10);
    panel.set_margin_bottom(10);

    panel.set_size_request(170, -1);

    menu_win.add(&panel);

    let history_box =
        GtkBox::new(Orientation::Horizontal, 4);

    let undo_btn =
        Button::with_label("↩️");

    let redo_btn =
        Button::with_label("↪️");

    history_box.pack_start(
        &undo_btn,
        true,
        true,
        0,
    );

    history_box.pack_start(
        &redo_btn,
        true,
        true,
        0,
    );

    panel.pack_start(
        &history_box,
        false,
        false,
        0,
    );

    let pen_btn =
        Button::with_label("✏️ Kalem");

    let eraser_btn =
        Button::with_label("⬜ Silgi");

    panel.pack_start(
        &pen_btn,
        false,
        false,
        0,
    );

    panel.pack_start(
        &eraser_btn,
        false,
        false,
        0,
    );

    let color_btn =
        ColorButton::new();

    color_btn.set_rgba(
        &RGBA::new(r, g, b, 1.0),
    );

    panel.pack_start(
        &color_btn,
        false,
        false,
        0,
    );

    let size_scale =
        Scale::with_range(
            Orientation::Horizontal,
            1.0,
            40.0,
            1.0,
        );

    size_scale.set_value(
        state.borrow().brush_size,
    );

    panel.pack_start(
        &size_scale,
        false,
        false,
        0,
    );

    let passthrough_btn =
        Button::with_label("Mod: Çizim");

    panel.pack_start(
        &passthrough_btn,
        false,
        false,
        0,
    );

    let zoom_btn =
        Button::with_label("🔍 Alan Seç");

    panel.pack_start(
        &zoom_btn,
        false,
        false,
        0,
    );

    let clear_btn =
        Button::with_label("Temizle");

    panel.pack_start(
        &clear_btn,
        false,
        false,
        0,
    );

    overlay_win.connect_draw(
        clone!(@strong state =>
        move |_, cr| {

            let st = state.borrow();

            if st.view_mode == ViewMode::Zoomed {

                cr.set_source_rgb(
                    1.0,
                    1.0,
                    1.0,
                );

                cr.paint().unwrap();

                if let Some(ref pixbuf) =
                    st.zoom_image
                {
                    cr.set_source_pixbuf(
                        pixbuf,
                        st.zoom_offset_x,
                        st.zoom_offset_y,
                    );

                    cr.paint().unwrap();
                }

            } else {

                cr.set_operator(
                    cairo::Operator::Clear,
                );

                cr.paint().unwrap();

                cr.set_operator(
                    cairo::Operator::Over,
                );
            }

            for stroke in &st.strokes {
                draw_stroke(cr, stroke);
            }

            if let Some(ref cs) =
                st.current_stroke
            {
                draw_stroke(cr, cs);
            }

            glib::Propagation::Proceed.into()
        }),
    );

    overlay_win.connect_button_press_event(
        clone!(@strong state =>
        move |win, ev| {

            let mut st =
                state.borrow_mut();

            if st.passthrough {
                return glib::Propagation::Proceed.into();
            }

            st.drawing = true;

            st.undo_stack.clear();

            let (x, y) =
                ev.position();

            let stroke = Stroke {
                points: vec![
                    Point { x, y },
                    Point {
                        x: x + 0.1,
                        y: y + 0.1,
                    },
                ],

                color: st.color,

                width:
                    if st.tool == Tool::Eraser
                {
                    st.brush_size * 5.0
                } else {
                    st.brush_size
                },

                is_eraser:
                    st.tool == Tool::Eraser,
            };

            st.current_stroke =
                Some(stroke);

            win.queue_draw();

            glib::Propagation::Stop.into()
        }),
    );

    overlay_win.connect_motion_notify_event(
        clone!(@strong state =>
        move |win, ev| {

            let mut st =
                state.borrow_mut();

            if !st.drawing
                || st.passthrough
            {
                return glib::Propagation::Proceed.into();
            }

            let (x, y) =
                ev.position();

            if let Some(ref mut cs) =
                st.current_stroke
            {
                cs.points.push(
                    Point { x, y },
                );
            }

            win.queue_draw();

            glib::Propagation::Stop.into()
        }),
    );

    overlay_win.connect_button_release_event(
        clone!(@strong state =>
        move |win, _| {

            let mut st =
                state.borrow_mut();

            st.drawing = false;

            if let Some(stroke) =
                st.current_stroke.take()
            {
                st.strokes.push(stroke);
            }

            win.queue_draw();

            glib::Propagation::Stop.into()
        }),
    );

    undo_btn.connect_clicked(
        clone!(@strong state,
               @strong overlay_win =>
        move |_| {

            let mut st =
                state.borrow_mut();

            if let Some(s) =
                st.strokes.pop()
            {
                st.undo_stack.push(s);

                overlay_win.queue_draw();
            }
        }),
    );

    redo_btn.connect_clicked(
        clone!(@strong state,
               @strong overlay_win =>
        move |_| {

            let mut st =
                state.borrow_mut();

            if let Some(s) =
                st.undo_stack.pop()
            {
                st.strokes.push(s);

                overlay_win.queue_draw();
            }
        }),
    );

    color_btn.connect_color_set(
        clone!(@strong state =>
        move |btn| {

            let mut st =
                state.borrow_mut();

            let rgba =
                btn.rgba();

            st.color = (
                rgba.red(),
                rgba.green(),
                rgba.blue(),
            );

            st.save_config();
        }),
    );

    size_scale.connect_value_changed(
        clone!(@strong state =>
        move |sc| {

            let mut st =
                state.borrow_mut();

            st.brush_size =
                sc.value();

            st.save_config();
        }),
    );

    passthrough_btn.connect_clicked(
        clone!(@strong state,
               @strong overlay_win,
               @strong menu_win =>
        move |btn| {

            let mut st =
                state.borrow_mut();

            st.passthrough =
                !st.passthrough;

            let is_p =
                st.passthrough;

            btn.set_label(
                if is_p {
                    "Mod: Tıklama"
                } else {
                    "Mod: Çizim"
                },
            );

            apply_input_shape(
                &overlay_win,
                &menu_win,
                is_p,
            );
        }),
    );

    clear_btn.connect_clicked(
        clone!(@strong state,
               @strong overlay_win =>
        move |_| {

            let mut st =
                state.borrow_mut();

            st.strokes.clear();
            st.undo_stack.clear();

            overlay_win.queue_draw();
        }),
    );

    zoom_btn.connect_clicked(
        clone!(@strong state,
               @strong overlay_win,
               @strong zoom_btn =>
        move |_| {

            let current_mode = {
                state.borrow()
                    .view_mode
                    .clone()
            };

            if current_mode
                == ViewMode::Zoomed
            {
                let mut st =
                    state.borrow_mut();

                st.zoom_image = None;

                st.view_mode =
                    ViewMode::Desktop;

                zoom_btn.set_label(
                    "🔍 Alan Seç",
                );

                overlay_win.queue_draw();

                return;
            }

            if let Some(original) =
                capture_area()
            {
                let screen =
                    gdk::Screen::default()
                        .unwrap();

                let monitor =
                    screen.display()
                        .monitor(0)
                        .unwrap();

                let geom =
                    monitor.geometry();

                let screen_w =
                    geom.width() as f64;

                let screen_h =
                    geom.height() as f64;

                let img_w =
                    original.width() as f64;

                let img_h =
                    original.height() as f64;

                let scale =
                    (screen_w / img_w)
                        .min(
                            screen_h / img_h,
                        );

                let final_w =
                    (img_w * scale)
                        as i32;

                let final_h =
                    (img_h * scale)
                        as i32;

                let offset_x =
                    (screen_w
                        - final_w as f64)
                        / 2.0;

                let offset_y =
                    (screen_h
                        - final_h as f64)
                        / 2.0;

                let scaled =
                    original.scale_simple(
                        final_w,
                        final_h,
                        InterpType::Bilinear,
                    );

                let mut st =
                    state.borrow_mut();

                st.zoom_image = scaled;

                st.zoom_offset_x =
                    offset_x;

                st.zoom_offset_y =
                    offset_y;

                st.view_mode =
                    ViewMode::Zoomed;

                zoom_btn.set_label(
                    "🖥 Geri Dön",
                );

                overlay_win.queue_draw();
            }
        }),
    );

    pen_btn.connect_clicked(
        clone!(@strong state =>
        move |_| {

            state.borrow_mut().tool =
                Tool::Pen;
        }),
    );

    eraser_btn.connect_clicked(
        clone!(@strong state =>
        move |_| {

            state.borrow_mut().tool =
                Tool::Eraser;
        }),
    );

    overlay_win.show_all();
    menu_win.show_all();
}