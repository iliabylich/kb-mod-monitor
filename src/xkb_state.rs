use anyhow::{Context as _, Result, ensure};
use std::{cell::RefCell, rc::Rc};
use xkbcommon::xkb::{
    COMPILE_NO_FLAGS, Context, KEYMAP_COMPILE_NO_FLAGS, KeyDirection, Keycode, Keymap, MOD_INVALID,
    MOD_NAME_CAPS, STATE_MODS_LOCKED, State,
};

pub struct XkbState {
    state: Rc<RefCell<State>>,
}

const XKB_EVDEV_OFFSET: u16 = 8;
const XKB_CALS_LOCK_KEYCODE: Keycode =
    Keycode::new((evdev::KeyCode::KEY_CAPSLOCK.code() + XKB_EVDEV_OFFSET) as u32);

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

    pub(crate) fn set_initial_caps_lock_state(&self, caps_lock_is_active: bool) -> Result<()> {
        log::trace!("Initial caps lock state: {caps_lock_is_active}");
        let mut state = self.state.borrow_mut();
        let caps_index = state.get_keymap().mod_get_index(MOD_NAME_CAPS);
        ensure!(
            caps_index != MOD_INVALID,
            "generated layout doesn't have MOD_NAME_CAPS"
        );
        if caps_lock_is_active {
            state.update_mask(0, 0, 1 << caps_index, 0, 0, 0);
        }
        Ok(())
    }

    pub(crate) fn on_caps_lock_pressed(&self) -> Rc<dyn Fn() -> bool> {
        let state = Rc::clone(&self.state);

        Rc::new(move || {
            let mut state = state.borrow_mut();
            state.update_key(XKB_CALS_LOCK_KEYCODE, KeyDirection::Down);
            state.mod_name_is_active(MOD_NAME_CAPS, STATE_MODS_LOCKED)
        })
    }

    pub(crate) fn on_caps_lock_released(&self) -> Rc<dyn Fn() -> bool> {
        let state = Rc::clone(&self.state);

        Rc::new(move || {
            let mut state = state.borrow_mut();
            state.update_key(XKB_CALS_LOCK_KEYCODE, KeyDirection::Up);
            state.mod_name_is_active(MOD_NAME_CAPS, STATE_MODS_LOCKED)
        })
    }
}
