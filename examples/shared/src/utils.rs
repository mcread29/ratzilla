use wasm_bindgen::JsValue;

/// Inject a fixed FPS widget above the app in the top-right corner.
pub(crate) fn inject_fps_widget() -> Result<(), JsValue> {
    let window = web_sys::window().ok_or("No window")?;
    let document = window.document().ok_or("No document")?;

    // Remove an existing widget before recreating it.
    if let Some(existing) = document.get_element_by_id("ratzilla-fps-widget") {
        existing.remove();
    }

    let widget = document.create_element("div")?;
    widget.set_id("ratzilla-fps-widget");

    widget.set_attribute(
        "style",
        "position: fixed; top: 12px; right: 12px; \
         display: flex; align-items: baseline; gap: 8px; \
         padding: 8px 12px; border-radius: 10px; \
         background: rgba(15, 23, 42, 0.82); color: #e2e8f0; \
         border: 1px solid rgba(148, 163, 184, 0.28); \
         box-shadow: 0 10px 30px rgba(15, 23, 42, 0.35); \
         backdrop-filter: blur(8px); -webkit-backdrop-filter: blur(8px); \
         font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, \
         'Liberation Mono', 'Courier New', monospace; \
         font-size: 12px; line-height: 1; z-index: 2147483647; \
         pointer-events: none;",
    )?;

    widget.set_inner_html(
        "<span style=\"color: #94a3b8; text-transform: uppercase; letter-spacing: 0.08em;\">FPS</span> \
         <span id=\"ratzilla-fps\" style=\"color: #4ade80; font-weight: 700; font-size: 14px;\">--</span>",
    );

    let body = document.body().ok_or("No body")?;
    body.append_child(&widget)?;

    Ok(())
}
