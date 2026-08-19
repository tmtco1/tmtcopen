use crate::render::{draw_eraser_cursor, draw_line_preview, draw_stroke};
use crate::state::{AppState, Tool, ViewMode};
use crate::window_utils::apply_input_shape;

use gdk::prelude::*;
use gdk::{EventMask, EventType, RGBA, WindowTypeHint};

use gdk_pixbuf::{InterpType, Pixbuf};

use glib::clone;

use gtk::prelude::*;
use gtk::{
    Application, ApplicationWindow, Box as GtkBox, Button, ColorButton, ColorChooserDialog,
    FileChooserAction, FileChooserDialog, Orientation, Scale, Window, WindowType,
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

fn seq_to_key(seq: Option<gdk::EventSequence>) -> u32 {
    match seq {
        Some(s) => unsafe {
            std::mem::transmute::<gdk::EventSequence, *mut std::ffi::c_void>(s) as u32
        },
        None => 0,
    }
}

pub fn build_ui(app: &Application) {
    let screen = gdk::Screen::default().unwrap();
    let monitor = screen.display().monitor(0).unwrap();
    let geometry = monitor.geometry();

    let overlay_win = Window::new(WindowType::Toplevel);

    overlay_win.set_decorated(false);
    overlay_win.set_app_paintable(true);
    overlay_win.set_keep_above(true);
    overlay_win.set_type_hint(WindowTypeHint::Splashscreen);
    overlay_win.set_default_size(geometry.width(), geometry.height());
    overlay_win.move_(0, 0);

    if let Some(visual) = screen.rgba_visual() {
        overlay_win.set_visual(Some(&visual));
    }

    overlay_win.set_events(
        EventMask::BUTTON_PRESS_MASK
            | EventMask::BUTTON_RELEASE_MASK
            | EventMask::POINTER_MOTION_MASK
            | EventMask::TOUCH_MASK,
    );

    let menu_win = ApplicationWindow::builder()
        .application(app)
        .title("")
        .resizable(false)
        .build();

    menu_win.set_keep_above(true);
    menu_win.set_type_hint(WindowTypeHint::Dialog);
    menu_win.set_size_request(140, -1);
    menu_win.move_(20, (geometry.height() / 2) - 150);

    let state = Rc::new(RefCell::new(AppState::new()));

    let panel = GtkBox::new(Orientation::Vertical, 4);
    panel.set_margin_start(4);
    panel.set_margin_end(4);
    panel.set_margin_top(4);
    panel.set_margin_bottom(4);
    panel.set_size_request(120, -1);
    menu_win.add(&panel);

    let history_box = GtkBox::new(Orientation::Horizontal, 2);
    let undo_btn = Button::with_label("↩️");
    let redo_btn = Button::with_label("↪️");
    history_box.pack_start(&undo_btn, true, true, 0);
    history_box.pack_start(&redo_btn, true, true, 0);
    panel.pack_start(&history_box, false, false, 0);

    let tool_select_btn = Button::with_label("Kalem");
    tool_select_btn.set_halign(gtk::Align::Fill);
    panel.pack_start(&tool_select_btn, false, false, 0);

    let active_popup: Rc<RefCell<Option<Window>>> = Rc::new(RefCell::new(None));
    let initial_w: Rc<RefCell<i32>> = Rc::new(RefCell::new(0));
    let initial_h: Rc<RefCell<i32>> = Rc::new(RefCell::new(0));
    let popup_source: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));

    let (cr, cg, cb) = state.borrow().color;
    let color_select_btn = ColorButton::new();
    color_select_btn.set_rgba(&RGBA::new(cr, cg, cb, 1.0));
    color_select_btn.set_halign(gtk::Align::Fill);
    panel.pack_start(&color_select_btn, false, false, 0);

    let size_scale = Scale::with_range(Orientation::Horizontal, 1.0, 40.0, 1.0);
    size_scale.set_value(state.borrow().brush_size);
    size_scale.set_size_request(-1, 20);
    panel.pack_start(&size_scale, false, false, 0);

    let passthrough_btn = Button::with_label("Mod: Çizim");
    passthrough_btn.set_halign(gtk::Align::Fill);
    panel.pack_start(&passthrough_btn, false, false, 0);

    let zoom_btn = Button::with_label("Arkaplan");
    panel.pack_start(&zoom_btn, false, false, 0);

    let clear_btn = Button::with_label("Temizle");
    panel.pack_start(&clear_btn, false, false, 0);

    // --- Tool selector popup ---
    tool_select_btn.connect_clicked(
        clone!(@strong state, @strong overlay_win, @strong menu_win, @strong active_popup, @strong tool_select_btn, @strong initial_w, @strong initial_h, @strong popup_source => move |_| {
            if *popup_source.borrow() == Some("tool".to_string()) {
                if let Some(existing) = active_popup.borrow_mut().take() {
                    unsafe { existing.destroy(); }
                }
                *popup_source.borrow_mut() = None;
                return;
            }
            if let Some(existing) = active_popup.borrow_mut().take() {
                unsafe { existing.destroy(); }
            }

            let screen = gdk::Screen::default().unwrap();

            let popup = Window::new(WindowType::Toplevel);
            popup.set_decorated(false);
            popup.set_keep_above(true);
            popup.set_type_hint(WindowTypeHint::Dialog);
            popup.set_default_size(180, -1);
            popup.set_resizable(false);

            if let Some(visual) = screen.rgba_visual() {
                popup.set_visual(Some(&visual));
            }

            let popup_box = GtkBox::new(Orientation::Vertical, 4);
            popup_box.set_margin_start(8);
            popup_box.set_margin_end(8);
            popup_box.set_margin_top(8);
            popup_box.set_margin_bottom(8);
            popup.add(&popup_box);

            let tools = [
                (Tool::Pen, "Kalem"),
                (Tool::StraightLine, "Düz Çizgi"),
                (Tool::DashedLine, "Kesikli Çizgi"),
                (Tool::Highlighter, "Highlighter"),
                (Tool::Eraser, "Silgi"),
            ];

            for (tool, label) in tools.iter() {
                let btn = Button::with_label(label);
                let t = tool.clone();
                let l = label.to_string();
                let popup_ref = popup.clone();
                let state_ref = state.clone();
                let overlay_ref = overlay_win.clone();
                let btn_ref = tool_select_btn.clone();
                let active_ref = active_popup.clone();
                let menu_win_ref = menu_win.clone();
                let initial_w_ref = initial_w.clone();
                let initial_h_ref = initial_h.clone();
                btn.connect_clicked(move |_| {
                    state_ref.borrow_mut().tool = t.clone();
                    btn_ref.set_label(&l);
                    active_ref.borrow_mut().take();
                    overlay_ref.queue_draw();
                    unsafe { popup_ref.destroy(); }
                    let w = *initial_w_ref.borrow();
                    let h = *initial_h_ref.borrow();
                    if w > 0 { menu_win_ref.resize(w, h); }
                });
                popup_box.pack_start(&btn, false, false, 0);
            }

            if let Some(ref gdk_win) = menu_win.window() {
                let (wx, wy) = gdk_win.position();
                let ww = gdk_win.width();
                popup.move_(wx + ww + 5, wy);
            }

            *active_popup.borrow_mut() = Some(popup.clone());
            *popup_source.borrow_mut() = Some("tool".to_string());
            popup.show_all();

            {
                let active_ref = active_popup.clone();
                let source_ref = popup_source.clone();
                popup.connect_focus_out_event(move |w, _| {
                    *active_ref.borrow_mut() = None;
                    *source_ref.borrow_mut() = None;
                    unsafe { w.destroy(); }
                    glib::Propagation::Proceed.into()
                });
            }
        }),
    );

    overlay_win.connect_draw(clone!(@strong state => move |widget, cr| {
        let mut st = state.borrow_mut();

        let (sx, _sy) = cr.target().device_scale();
        let device_scale = if sx > 0.0 { sx } else { 1.0 };

        let alloc = widget.allocation();
        st.ensure_surface(alloc.width(), alloc.height(), device_scale);

        if st.view_mode == ViewMode::Zoomed {
            let (br, bg, bb) = st.background_color;
            cr.set_operator(cairo::Operator::Source);
            cr.set_source_rgb(br, bg, bb);
            cr.paint().unwrap();
            cr.set_operator(cairo::Operator::Over);

            if let Some(ref pixbuf) = st.zoom_image {
                cr.set_source_pixbuf(pixbuf, st.zoom_offset_x, st.zoom_offset_y);
                cr.paint().unwrap();
            }
        } else {
            cr.set_operator(cairo::Operator::Clear);
            cr.paint().unwrap();
            cr.set_operator(cairo::Operator::Over);
        }

        if let Some(ref surface) = st.committed_surface {
            cr.set_source_surface(surface, 0.0, 0.0).unwrap();
            cr.set_operator(cairo::Operator::Over);
            cr.paint().unwrap();
        }

        for stroke in st.active_strokes.values() {
            draw_stroke(cr, stroke);
        }

        if let Some((ref start, ref end)) = st.line_preview {
            let is_dashed = st.tool == Tool::DashedLine;
            draw_line_preview(cr, start.x, start.y, end.x, end.y, st.color, st.brush_size, is_dashed);
        }

        if st.tool == Tool::Eraser {
            if let Some(ref pos) = st.cursor_pos {
                draw_eraser_cursor(cr, pos.x, pos.y, st.brush_size * 2.5);
            }
        }

        glib::Propagation::Proceed.into()
    }));

    overlay_win.connect_button_press_event(clone!(@strong state, @strong active_popup, @strong popup_source => move |win, ev| {
        if let Some(popup) = active_popup.borrow_mut().take() {
            *popup_source.borrow_mut() = None;
            unsafe { popup.destroy(); }
            return glib::Propagation::Stop.into();
        }

        let mut st = state.borrow_mut();
        if st.passthrough { return glib::Propagation::Proceed.into(); }

        let (x, y) = ev.position();
        if st.tool == Tool::Eraser {
            st.cursor_pos = Some(crate::state::Point { x, y });
        }
        st.begin_stroke(0, x, y);
        win.queue_draw();
        glib::Propagation::Stop.into()
    }));

    overlay_win.connect_motion_notify_event(clone!(@strong state => move |win, ev| {
        let mut st = state.borrow_mut();
        if st.passthrough { return glib::Propagation::Proceed.into(); }

        let (x, y) = ev.position();

        if st.tool == Tool::Eraser {
            st.cursor_pos = Some(crate::state::Point { x, y });
            win.queue_draw();
        }

        if st.tool == Tool::StraightLine {
            if st.line_start.is_some() {
                if let Some((rx, ry, rw, rh)) = st.extend_stroke(0, x, y) {
                    win.queue_draw_area(rx, ry, rw, rh);
                }
                return glib::Propagation::Stop.into();
            }
            return glib::Propagation::Proceed.into();
        }

        if !st.active_strokes.contains_key(&0) {
            return glib::Propagation::Proceed.into();
        }

        if let Some((rx, ry, rw, rh)) = st.extend_stroke(0, x, y) {
            win.queue_draw_area(rx, ry, rw, rh);
        }
        glib::Propagation::Stop.into()
    }));

    overlay_win.connect_button_release_event(clone!(@strong state => move |win, _| {
        let mut st = state.borrow_mut();
        st.end_stroke(0);
        win.queue_draw();
        glib::Propagation::Stop.into()
    }));

    overlay_win.connect_touch_event(clone!(@strong state, @strong active_popup, @strong popup_source => move |win, ev| {
        if let Some(popup) = active_popup.borrow_mut().take() {
            *popup_source.borrow_mut() = None;
            unsafe { popup.destroy(); }
            return glib::Propagation::Stop.into();
        }

        let mut st = state.borrow_mut();
        if st.passthrough { return glib::Propagation::Proceed.into(); }

        let touch = match ev.downcast_ref::<gdk::EventTouch>() {
            Some(t) => t,
            None => return glib::Propagation::Proceed.into(),
        };

        let key = seq_to_key(touch.event_sequence());
        let (x, y) = touch.position();

        match touch.event_type() {
            EventType::TouchBegin => {
                if st.tool == Tool::Eraser {
                    st.cursor_pos = Some(crate::state::Point { x, y });
                    st.touch_active = true;
                }
                st.begin_stroke(key, x, y);
                win.queue_draw();
            }
            EventType::TouchUpdate => {
                if st.tool == Tool::Eraser && st.touch_active {
                    st.cursor_pos = Some(crate::state::Point { x, y });
                    win.queue_draw();
                }
                if let Some((rx, ry, rw, rh)) = st.extend_stroke(key, x, y) {
                    win.queue_draw_area(rx, ry, rw, rh);
                }
            }
            EventType::TouchEnd => {
                if st.tool == Tool::Eraser {
                    st.touch_active = false;
                }
                st.end_stroke(key);
                win.queue_draw();
            }
            EventType::TouchCancel => {
                if st.tool == Tool::Eraser {
                    st.touch_active = false;
                }
                st.active_strokes.remove(&key);
                win.queue_draw();
            }
            _ => {}
        }

        glib::Propagation::Stop.into()
    }));

    undo_btn.connect_clicked(clone!(@strong state, @strong overlay_win => move |_| {
        let mut st = state.borrow_mut();
        if let Some(s) = st.strokes.pop() {
            st.undo_stack.push(s);
            st.redraw_committed();
            overlay_win.queue_draw();
        }
    }));

    redo_btn.connect_clicked(clone!(@strong state, @strong overlay_win => move |_| {
        let mut st = state.borrow_mut();
        if let Some(s) = st.undo_stack.pop() {
            st.commit_stroke(&s);
            st.strokes.push(s);
            overlay_win.queue_draw();
        }
    }));

    color_select_btn.connect_button_press_event(
        clone!(@strong state, @strong overlay_win, @strong active_popup, @strong color_select_btn, @strong menu_win, @strong popup_source => move |btn, _| {
            if *popup_source.borrow() == Some("color".to_string()) {
                if let Some(existing) = active_popup.borrow_mut().take() {
                    unsafe { existing.destroy(); }
                }
                *popup_source.borrow_mut() = None;
                return glib::Propagation::Stop.into();
            }
            if let Some(existing) = active_popup.borrow_mut().take() {
                unsafe { existing.destroy(); }
            }

            let screen = gdk::Screen::default().unwrap();

            let popup = Window::new(WindowType::Toplevel);
            popup.set_decorated(false);
            popup.set_keep_above(true);
            popup.set_type_hint(WindowTypeHint::Dialog);
            popup.set_default_size(180, -1);
            popup.set_resizable(false);

            if let Some(visual) = screen.rgba_visual() {
                popup.set_visual(Some(&visual));
            }

            let popup_box = GtkBox::new(Orientation::Vertical, 4);
            popup_box.set_margin_start(8);
            popup_box.set_margin_end(8);
            popup_box.set_margin_top(8);
            popup_box.set_margin_bottom(8);
            popup.add(&popup_box);

            let predefined_colors: Vec<(&str, f64, f64, f64)> = vec![
                ("Kırmızı", 1.0, 0.0, 0.0),
                ("Yeşil", 0.0, 0.8, 0.0),
                ("Mavi", 0.0, 0.0, 1.0),
                ("Sarı", 1.0, 1.0, 0.0),
                ("Turuncu", 1.0, 0.5, 0.0),
                ("Mor", 0.6, 0.0, 0.8),
                ("Beyaz", 1.0, 1.0, 1.0),
                ("Siyah", 0.0, 0.0, 0.0),
            ];

            for (_label, r, g, b) in predefined_colors {
                let color_box = GtkBox::new(Orientation::Horizontal, 0);
                color_box.set_size_request(-1, 28);
                let css = gtk::CssProvider::new();
                let color_str = format!(
                    "* {{ background-color: rgb({},{},{}); }}",
                    (r * 255.0) as u8,
                    (g * 255.0) as u8,
                    (b * 255.0) as u8
                );
                css.load_from_data(color_str.as_bytes()).unwrap();
                color_box.style_context().add_provider(&css, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION);
                let event_box = gtk::EventBox::new();
                event_box.add(&color_box);
                let state_ref = state.clone();
                let popup_ref = popup.clone();
                let active_ref = active_popup.clone();
                let overlay_ref = overlay_win.clone();
                let color_btn_ref = color_select_btn.clone();
                event_box.connect_button_press_event(move |_, _| {
                    state_ref.borrow_mut().color = (r, g, b);
                    state_ref.borrow().save_config();
                    color_btn_ref.set_rgba(&RGBA::new(r, g, b, 1.0));
                    overlay_ref.queue_draw();
                    active_ref.borrow_mut().take();
                    unsafe { popup_ref.destroy(); }
                    glib::Propagation::Stop.into()
                });
                popup_box.pack_start(&event_box, false, false, 0);
            }

            let custom_btn = Button::with_label("Renk seç...");
            let state_ref = state.clone();
            let popup_ref = popup.clone();
            let active_ref = active_popup.clone();
            let overlay_ref = overlay_win.clone();
            let color_btn_ref2 = color_select_btn.clone();
            custom_btn.connect_clicked(move |_| {
                let current = state_ref.borrow().color;
                let dialog = ColorChooserDialog::new(Some("Renk Seç"), Some(&popup_ref));
                let rgba = RGBA::new(current.0, current.1, current.2, 1.0);
                dialog.set_rgba(&rgba);
                if dialog.run() == gtk::ResponseType::Ok {
                    let rgba = dialog.rgba();
                    let nr = rgba.red();
                    let ng = rgba.green();
                    let nb = rgba.blue();
                    state_ref.borrow_mut().color = (nr, ng, nb);
                    state_ref.borrow().save_config();
                    color_btn_ref2.set_rgba(&RGBA::new(nr, ng, nb, 1.0));
                    overlay_ref.queue_draw();
                }
                unsafe { dialog.destroy(); }
                active_ref.borrow_mut().take();
                unsafe { popup_ref.destroy(); }
            });
            popup_box.pack_start(&custom_btn, false, false, 0);

            if let Some(ref gdk_win) = menu_win.window() {
                let (wx, wy) = gdk_win.position();
                let ww = gdk_win.width();
                popup.move_(wx + ww + 5, wy);
            }

            *active_popup.borrow_mut() = Some(popup.clone());
            *popup_source.borrow_mut() = Some("color".to_string());
            popup.show_all();

            {
                let active_ref = active_popup.clone();
                let source_ref = popup_source.clone();
                popup.connect_focus_out_event(move |w, _| {
                    *active_ref.borrow_mut() = None;
                    *source_ref.borrow_mut() = None;
                    unsafe { w.destroy(); }
                    glib::Propagation::Proceed.into()
                });
            }

            glib::Propagation::Stop.into()
        }),
    );

    size_scale.connect_value_changed(clone!(@strong state => move |sc| {
        let mut st = state.borrow_mut();
        st.brush_size = sc.value();
        st.save_config();
    }));

    passthrough_btn.connect_clicked(
        clone!(@strong state, @strong overlay_win, @strong menu_win, @strong initial_w, @strong initial_h => move |btn| {
            let mut st = state.borrow_mut();
            st.passthrough = !st.passthrough;
            let is_p = st.passthrough;
            btn.set_label(if is_p { "Mod: Tıklama" } else { "Mod: Çizim" });
            apply_input_shape(&overlay_win, &menu_win, is_p);
            drop(st);
            let w = *initial_w.borrow();
            let h = *initial_h.borrow();
            if w > 0 { menu_win.resize(w, h); }
        }),
    );

    clear_btn.connect_clicked(clone!(@strong state, @strong overlay_win => move |_| {
        let mut st = state.borrow_mut();
        st.strokes.clear();
        st.undo_stack.clear();
        st.active_strokes.clear();
        st.redraw_committed();
        overlay_win.queue_draw();
    }));

    zoom_btn.connect_clicked(
        clone!(@strong state, @strong overlay_win, @strong zoom_btn, @strong active_popup, @strong menu_win, @strong popup_source => move |_| {
            if *popup_source.borrow() == Some("arkaplan".to_string()) {
                if let Some(existing) = active_popup.borrow_mut().take() {
                    unsafe { existing.destroy(); }
                }
                *popup_source.borrow_mut() = None;
                return;
            }
            if let Some(existing) = active_popup.borrow_mut().take() {
                unsafe { existing.destroy(); }
            }

            let screen = gdk::Screen::default().unwrap();

            let popup = Window::new(WindowType::Toplevel);
            popup.set_decorated(false);
            popup.set_keep_above(true);
            popup.set_type_hint(WindowTypeHint::Dialog);
            popup.set_default_size(180, -1);
            popup.set_resizable(false);

            if let Some(visual) = screen.rgba_visual() {
                popup.set_visual(Some(&visual));
            }

            let popup_box = GtkBox::new(Orientation::Vertical, 4);
            popup_box.set_margin_start(8);
            popup_box.set_margin_end(8);
            popup_box.set_margin_top(8);
            popup_box.set_margin_bottom(8);
            popup.add(&popup_box);

            let current_mode = { state.borrow().view_mode.clone() };
            if current_mode == ViewMode::Zoomed {
                let back_btn = Button::with_label("Masaüstüne Dön");
                let state_ref = state.clone();
                let popup_ref = popup.clone();
                let active_ref = active_popup.clone();
                let overlay_ref = overlay_win.clone();
                let zoom_btn_ref = zoom_btn.clone();
                back_btn.connect_clicked(move |_| {
                    let mut st = state_ref.borrow_mut();
                    st.zoom_image = None;
                    st.view_mode = ViewMode::Desktop;
                    zoom_btn_ref.set_label("Arkaplan");
                    overlay_ref.queue_draw();
                    active_ref.borrow_mut().take();
                    unsafe { popup_ref.destroy(); }
                });
                popup_box.pack_start(&back_btn, false, false, 0);
            }

            let white_btn = Button::with_label("Beyaz");
            {
                let state_ref = state.clone();
                let popup_ref = popup.clone();
                let active_ref = active_popup.clone();
                let overlay_ref = overlay_win.clone();
                white_btn.connect_clicked(move |_| {
                    let mut st = state_ref.borrow_mut();
                    st.background_color = (1.0, 1.0, 1.0);
                    st.zoom_image = None;
                    st.view_mode = ViewMode::Zoomed;

                    overlay_ref.queue_draw();
                    active_ref.borrow_mut().take();
                    unsafe { popup_ref.destroy(); }
                });
            }
            popup_box.pack_start(&white_btn, false, false, 0);

            let black_btn = Button::with_label("Siyah");
            {
                let state_ref = state.clone();
                let popup_ref = popup.clone();
                let active_ref = active_popup.clone();
                let overlay_ref = overlay_win.clone();
                black_btn.connect_clicked(move |_| {
                    let mut st = state_ref.borrow_mut();
                    st.background_color = (0.0, 0.0, 0.0);
                    st.zoom_image = None;
                    st.view_mode = ViewMode::Zoomed;

                    overlay_ref.queue_draw();
                    active_ref.borrow_mut().take();
                    unsafe { popup_ref.destroy(); }
                });
            }
            popup_box.pack_start(&black_btn, false, false, 0);

            let file_btn = Button::with_label("Dosya Seç");
            {
                let state_ref = state.clone();
                let popup_ref = popup.clone();
                let active_ref = active_popup.clone();
                let overlay_ref = overlay_win.clone();
                file_btn.connect_clicked(move |_| {
                    let dialog = FileChooserDialog::new(
                        Some("Arka Plan Resmi Seç"),
                        Some(&popup_ref),
                        FileChooserAction::Open,
                    );
                    dialog.add_button("İptal", gtk::ResponseType::Cancel);
                    dialog.add_button("Aç", gtk::ResponseType::Accept);

                    let filter = gtk::FileFilter::new();
                    filter.set_name(Some("Resim Dosyaları"));
                    filter.add_mime_type("image/*");
                    dialog.add_filter(filter);

                    if dialog.run() == gtk::ResponseType::Accept {
                        if let Some(path) = dialog.filename() {
                            if let Ok(original) = Pixbuf::from_file(&path) {
                                let screen = gdk::Screen::default().unwrap();
                                let monitor = screen.display().monitor(0).unwrap();
                                let geom = monitor.geometry();

                                let screen_w = geom.width() as f64;
                                let screen_h = geom.height() as f64;
                                let img_w = original.width() as f64;
                                let img_h = original.height() as f64;

                                let scale = (screen_w / img_w).min(screen_h / img_h);
                                let final_w = (img_w * scale) as i32;
                                let final_h = (img_h * scale) as i32;

                                let offset_x = (screen_w - final_w as f64) / 2.0;
                                let offset_y = (screen_h - final_h as f64) / 2.0;

                                let scaled = original.scale_simple(final_w, final_h, InterpType::Bilinear);

                                let mut st = state_ref.borrow_mut();
                                st.background_color = (1.0, 1.0, 1.0);
                                st.zoom_image = scaled;
                                st.zoom_offset_x = offset_x;
                                st.zoom_offset_y = offset_y;
                                st.view_mode = ViewMode::Zoomed;
            
                                overlay_ref.queue_draw();
                            }
                        }
                    }
                    unsafe { dialog.destroy(); }
                    active_ref.borrow_mut().take();
                    unsafe { popup_ref.destroy(); }
                });
            }
            popup_box.pack_start(&file_btn, false, false, 0);

            let capture_btn = Button::with_label("Alan Seç");
            {
                let state_ref = state.clone();
                let popup_ref = popup.clone();
                let active_ref = active_popup.clone();
                let overlay_ref = overlay_win.clone();
                capture_btn.connect_clicked(move |_| {
                    active_ref.borrow_mut().take();
                    unsafe { popup_ref.destroy(); }

                    let state_inner = state_ref.clone();
                    let overlay_inner = overlay_ref.clone();
                    glib::timeout_add_local(std::time::Duration::from_millis(50), move || {
                        if let Some(original) = capture_area() {
                            let screen = gdk::Screen::default().unwrap();
                            let monitor = screen.display().monitor(0).unwrap();
                            let geom = monitor.geometry();

                            let screen_w = geom.width() as f64;
                            let screen_h = geom.height() as f64;
                            let img_w = original.width() as f64;
                            let img_h = original.height() as f64;

                            let scale = (screen_w / img_w).min(screen_h / img_h);
                            let final_w = (img_w * scale) as i32;
                            let final_h = (img_h * scale) as i32;

                            let offset_x = (screen_w - final_w as f64) / 2.0;
                            let offset_y = (screen_h - final_h as f64) / 2.0;

                            let scaled = original.scale_simple(final_w, final_h, InterpType::Bilinear);

                            let mut st = state_inner.borrow_mut();
                            st.background_color = (1.0, 1.0, 1.0);
                            st.zoom_image = scaled;
                            st.zoom_offset_x = offset_x;
                            st.zoom_offset_y = offset_y;
                            st.view_mode = ViewMode::Zoomed;

                            overlay_inner.queue_draw();
                        }
                        glib::ControlFlow::Break
                    });
                });
            }
            popup_box.pack_start(&capture_btn, false, false, 0);

            if let Some(ref gdk_win) = menu_win.window() {
                let (wx, wy) = gdk_win.position();
                let ww = gdk_win.width();
                popup.move_(wx + ww + 5, wy);
            }

            *active_popup.borrow_mut() = Some(popup.clone());
            *popup_source.borrow_mut() = Some("arkaplan".to_string());
            popup.show_all();

            {
                let active_ref = active_popup.clone();
                let source_ref = popup_source.clone();
                popup.connect_focus_out_event(move |w, _| {
                    *active_ref.borrow_mut() = None;
                    *source_ref.borrow_mut() = None;
                    unsafe { w.destroy(); }
                    glib::Propagation::Proceed.into()
                });
            }
        }),
    );

    overlay_win.show_all();
    menu_win.show_all();

    *initial_w.borrow_mut() = menu_win.allocated_width();
    *initial_h.borrow_mut() = menu_win.allocated_height();
    menu_win.set_size_request(*initial_w.borrow(), *initial_h.borrow());
}
