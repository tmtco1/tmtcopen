use gtk::prelude::*;
use gtk::{ApplicationWindow, Window};
use gdk::prelude::*;

pub fn apply_input_shape(overlay_win: &Window, menu_win: &ApplicationWindow, passthrough: bool) {
    let gdk_win = match overlay_win.window() {
        Some(w) => w,
        None => return,
    };

    if passthrough {
        gdk_win.input_shape_combine_region(&cairo::Region::create(), 0, 0);
    } else {
        let screen = gdk::Screen::default().unwrap();
        let geom = screen.display().monitor(0).unwrap().geometry();
        let full = cairo::Region::create_rectangle(&cairo::RectangleInt::new(0, 0, geom.width(), geom.height()));
        let (mx, my) = menu_win.position();
        let (mw, mh) = menu_win.size();
        let menu_region = cairo::Region::create_rectangle(&cairo::RectangleInt::new(mx, my, mw, mh));
        let mut active = full;
        active.subtract(&menu_region);
        gdk_win.input_shape_combine_region(&active, 0, 0);
    }
}
