use evdev::KeyCode;
use xkbcommon::xkb::{Keycode, Keymap, MOD_INVALID, MOD_NAME_CAPS, MOD_NAME_NUM};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    CapsLock,
    NumLock,
}

const XKB_EVDEV_OFFSET: u16 = 8;
const XKB_CAPS_LOCK_KEYCODE: Keycode =
    Keycode::new((KeyCode::KEY_CAPSLOCK.code() + XKB_EVDEV_OFFSET) as u32);
const XKB_NUM_LOCK_KEYCODE: Keycode =
    Keycode::new((KeyCode::KEY_NUMLOCK.code() + XKB_EVDEV_OFFSET) as u32);

impl Key {
    pub(crate) const fn xkb_mod_name(self) -> &'static str {
        match self {
            Self::CapsLock => MOD_NAME_CAPS,
            Self::NumLock => MOD_NAME_NUM,
        }
    }

    pub(crate) const fn xkb_key_code(self) -> Keycode {
        match self {
            Self::CapsLock => XKB_CAPS_LOCK_KEYCODE,
            Self::NumLock => XKB_NUM_LOCK_KEYCODE,
        }
    }

    pub(crate) fn index_in_xkb_keymap(self, keymap: &Keymap) -> Option<u32> {
        let idx = keymap.mod_get_index(self.xkb_mod_name());
        if idx == MOD_INVALID { None } else { Some(idx) }
    }

    pub(crate) const fn from_evdev_keycode(keycode: u16) -> Option<Self> {
        match keycode {
            x if x == KeyCode::KEY_CAPSLOCK.code() => Some(Self::CapsLock),
            x if x == KeyCode::KEY_NUMLOCK.code() => Some(Self::NumLock),
            _ => None,
        }
    }
}
