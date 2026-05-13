use crate::key::Key;

#[derive(Debug, Clone, Copy)]
pub struct KeyboardStateDiff {
    caps_lock: Option<bool>,
    num_lock: Option<bool>,
}

impl std::ops::BitOrAssign for KeyboardStateDiff {
    fn bitor_assign(&mut self, rhs: Self) {
        self.caps_lock = rhs.caps_lock.or(self.caps_lock);
        self.num_lock = rhs.num_lock.or(self.num_lock);
    }
}

impl KeyboardStateDiff {
    pub(crate) const fn empty() -> Self {
        Self {
            caps_lock: None,
            num_lock: None,
        }
    }

    pub(crate) const fn is_empty(self) -> bool {
        self.caps_lock.is_none() && self.num_lock.is_none()
    }

    pub(crate) const fn get(self, key: Key) -> Option<bool> {
        match key {
            Key::CapsLock => self.caps_lock,
            Key::NumLock => self.num_lock,
        }
    }

    pub(crate) const fn set(&mut self, key: Key, value: Option<bool>) {
        match key {
            Key::CapsLock => self.caps_lock = value,
            Key::NumLock => self.num_lock = value,
        }
    }

    pub(crate) fn or(self, fallback: Self) -> Self {
        Self {
            caps_lock: self.caps_lock.or(fallback.caps_lock),
            num_lock: self.num_lock.or(fallback.num_lock),
        }
    }
}
