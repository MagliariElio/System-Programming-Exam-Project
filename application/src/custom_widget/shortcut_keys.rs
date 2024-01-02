use std::collections::HashSet;
use druid::{Data, Lens, Selector};
use druid::keyboard_types::Code;

pub const SHORTCUT_KEYS: Selector = Selector::new("ShortcutKeys-Command");

#[derive(Clone, PartialEq, Debug)]
pub enum StateShortcutKeys {
    StartScreenGrabber,
    NotBusy,
    SetFavoriteShortcut,
    ShortcutNotAvailable
}

#[derive(Clone, Lens)]
pub struct ShortcutKeys {
    pub(crate) favorite_hot_keys: HashSet<Code>,              // favorite key codes
    pub(crate) pressed_hot_keys: HashSet<Code>,            // keys pressed by the user
    pub(crate) state: StateShortcutKeys
}

impl Data for ShortcutKeys {
    fn same(&self, other: &Self) -> bool {
        self.favorite_hot_keys == other.favorite_hot_keys
            && self.pressed_hot_keys == other.pressed_hot_keys
            && self.state == other.state
    }
}