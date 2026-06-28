use crate::{key::Key, keyboard::KeyboardStateDiff};

#[derive(Debug, Clone, Copy)]
pub struct KeyboardState {
    caps_lock: bool,
    num_lock: bool,
}

impl KeyboardState {
    pub(crate) const fn empty() -> Self {
        Self {
            caps_lock: false,
            num_lock: false,
        }
    }

    pub(crate) const fn as_diff(self) -> KeyboardStateDiff {
        let mut diff = KeyboardStateDiff::empty();
        diff.set(Key::CapsLock, Some(self.caps_lock));
        diff.set(Key::NumLock, Some(self.num_lock));
        diff
    }

    pub(crate) const fn get(self, key: Key) -> bool {
        match key {
            Key::CapsLock => self.caps_lock,
            Key::NumLock => self.num_lock,
        }
    }

    pub(crate) const fn set(&mut self, key: Key, value: bool) {
        match key {
            Key::CapsLock => self.caps_lock = value,
            Key::NumLock => self.num_lock = value,
        }
    }

    pub(crate) const fn apply(&mut self, diff: KeyboardStateDiff) -> Option<KeyboardStateDiff> {
        let mut out = KeyboardStateDiff::empty();

        if let Some(caps_lock) = diff.get(Key::CapsLock)
            && self.caps_lock != caps_lock
        {
            self.caps_lock = caps_lock;
            out.set(Key::CapsLock, Some(caps_lock));
        }
        if let Some(num_lock) = diff.get(Key::NumLock)
            && self.num_lock != num_lock
        {
            self.num_lock = num_lock;
            out.set(Key::NumLock, Some(num_lock));
        }

        if out.is_empty() { None } else { Some(out) }
    }

    pub(crate) const fn or(self, other: Self) -> Self {
        Self {
            caps_lock: self.caps_lock | other.caps_lock,
            num_lock: self.num_lock | other.num_lock,
        }
    }
}
