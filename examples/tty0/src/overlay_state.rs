use std::{cell::Cell, rc::Rc};

#[derive(Clone, Default)]
pub struct OverlayRenderState {
    editor_open: Rc<Cell<bool>>,
}

impl OverlayRenderState {
    pub fn set_editor_open(&self, is_open: bool) {
        self.editor_open.set(is_open);
    }

    pub fn editor_open(&self) -> bool {
        self.editor_open.get()
    }
}
