use crate::{
    key::Key,
    xkb_state::{Dir, XkbState},
};
use std::rc::Rc;

#[derive(Clone)]
pub struct XkbCallbacks {
    caps_lock_press: Rc<dyn Fn() -> bool>,
    caps_lock_release: Rc<dyn Fn() -> bool>,
    num_lock_press: Rc<dyn Fn() -> bool>,
    num_lock_release: Rc<dyn Fn() -> bool>,
}

impl XkbCallbacks {
    pub(crate) fn new(xkb_state: &XkbState) -> Self {
        Self {
            caps_lock_press: xkb_state.on_mod_toggle(Key::CapsLock, Dir::Down),
            caps_lock_release: xkb_state.on_mod_toggle(Key::CapsLock, Dir::Up),
            num_lock_press: xkb_state.on_mod_toggle(Key::NumLock, Dir::Down),
            num_lock_release: xkb_state.on_mod_toggle(Key::NumLock, Dir::Up),
        }
    }

    pub(crate) fn dispatch(&self, key: Key, dir: Dir) -> bool {
        match (key, dir) {
            (Key::CapsLock, Dir::Up) => (self.caps_lock_release)(),
            (Key::CapsLock, Dir::Down) => (self.caps_lock_press)(),
            (Key::NumLock, Dir::Up) => (self.num_lock_release)(),
            (Key::NumLock, Dir::Down) => (self.num_lock_press)(),
        }
    }
}
