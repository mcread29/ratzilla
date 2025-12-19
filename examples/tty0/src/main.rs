//! # [Ratatui] Original Demo example
//!
//! The latest version of this example is available in the [examples] folder in the upstream.
//!
//! [Ratatui]: https://github.com/ratatui/ratatui
//! [examples]: https://github.com/ratatui/ratatui/blob/main/examples
//! [examples readme]: https://github.com/ratatui/ratatui/blob/main/examples/README.md

use std::{cell::RefCell, io::Result, rc::Rc};

use app::App;
use examples_shared::backend::WebGl2BackendBuilder;
use ratzilla::backend::cursor::CursorShape;
use ratzilla::backend::webgl2::WebGl2BackendOptions;
use ratzilla::WebRenderer;

mod app;
mod introstate;
mod state;

fn main() -> Result<()> {
    let app_state = Rc::new(RefCell::new(App::new()));

    let webgl2_options = WebGl2BackendOptions::new()
        .cursor_shape(CursorShape::SteadyUnderScore)
        .enable_post_processing();

    let terminal = WebGl2BackendBuilder::with_options(webgl2_options).build_terminal()?;

    terminal.on_key_event({
        let app_state_cloned = app_state.clone();
        move |event| {
            let mut app_state = app_state_cloned.borrow_mut();
            app_state
                .key_press(event.code)
                .map_err(|e| eprintln!("Error: {}", e))
                .unwrap();
        }
    });

    terminal.draw_web(move |f| {
        let mut app_state = app_state.borrow_mut();
        app_state
            .update(f)
            .map_err(|e| eprintln!("Error: {}", e))
            .unwrap();
    });

    Ok(())
}
