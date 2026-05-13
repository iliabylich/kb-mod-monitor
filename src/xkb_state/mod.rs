use crate::{key::Key, keyboard::state::KeyboardState};
use anyhow::{Context as _, Result};
use std::{cell::RefCell, rc::Rc};
use xkbcommon::xkb::{
    COMPILE_NO_FLAGS, Context, KEYMAP_COMPILE_NO_FLAGS, KeyDirection, Keymap, STATE_MODS_LOCKED,
    State,
};

pub mod callbacks;

pub struct XkbState {
    state: Rc<RefCell<State>>,
}

impl XkbState {
    pub(crate) fn new() -> Result<Self> {
        let context = Context::new(COMPILE_NO_FLAGS);
        let keymap =
            Keymap::new_from_names(&context, "", "", "", "", None, KEYMAP_COMPILE_NO_FLAGS)
                .context("failed to create xkb keymap")?;
        let state = State::new(&keymap);

        Ok(Self {
            state: Rc::new(RefCell::new(state)),
        })
    }

    pub(crate) fn set_initial_state(&self, kb_state: KeyboardState) {
        log::trace!("Initial keyboard state: {kb_state:?}");
        let mut state = self.state.borrow_mut();
        let keymap = state.get_keymap();

        let mut locked_mods = 0;

        if let Some(idx) = Key::CapsLock.index_in_xkb_keymap(&keymap)
            && kb_state.get(Key::CapsLock)
        {
            locked_mods |= 1 << idx;
        }

        if let Some(idx) = Key::NumLock.index_in_xkb_keymap(&keymap)
            && kb_state.get(Key::NumLock)
        {
            locked_mods |= 1 << idx;
        }

        state.update_mask(0, 0, locked_mods, 0, 0, 0);
    }

    pub(crate) fn on_mod_toggle(&self, key: Key, dir: Dir) -> Rc<dyn Fn() -> bool> {
        let state = Rc::clone(&self.state);

        Rc::new(move || {
            let mut state = state.borrow_mut();
            state.update_key(key.xkb_key_code(), KeyDirection::from(dir));
            state.mod_name_is_active(key.xkb_mod_name(), STATE_MODS_LOCKED)
        })
    }
}

// KeyDirection is not Copy for some reason
#[derive(Clone, Copy)]
pub enum Dir {
    Up,
    Down,
}
impl From<Dir> for KeyDirection {
    fn from(dir: Dir) -> Self {
        match dir {
            Dir::Up => Self::Up,
            Dir::Down => Self::Down,
        }
    }
}
