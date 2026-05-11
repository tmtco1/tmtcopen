mod render;
mod state;
mod ui;
mod window_utils;

use gtk::prelude::*;
use gtk::Application;

fn main() {
    let app = Application::builder()
        .application_id("com.tmtcopen.app")
        .build();
    app.connect_activate(ui::build_ui);
    app.run();
}