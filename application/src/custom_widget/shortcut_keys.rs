use std::collections::HashSet;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::process::exit;
use druid::{Code, Data, Lens, Selector};
use crate::{BASE_PATH_FAVORITE_SHORTCUT, BASE_PATH_SCREENSHOT};

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

pub fn write_to_file<T>(file_path: &str, data: &T) -> Result<(), Box<dyn std::error::Error>>
    where
        T: serde::Serialize,
{
    verify_exists_dir(BASE_PATH_FAVORITE_SHORTCUT);
    let serialized_data = serde_json::to_string(data)?;

    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(file_path)?;

    file.write_all(serialized_data.as_bytes())?;

    Ok(())
}

pub fn read_from_file<T>(file_path: &str) -> Option<T>
    where
        T: for<'de> serde::Deserialize<'de>,
{
    if let Ok(mut file) = File::open(file_path) {
        let mut buffer = String::new();
        if file.read_to_string(&mut buffer).is_ok() {
            Some(serde_json::from_str(&buffer).ok()?)
        } else {
            None
        }
    } else {
        None
    }
}

pub fn verify_exists_dir(path: &str) {
    if std::fs::metadata(path).is_ok() && std::fs::metadata(path).unwrap().is_dir() {
    } else {
        match std::fs::create_dir(path) {
            Ok(_) => {}
            Err(_) => {
                if path.eq(BASE_PATH_SCREENSHOT) {
                    eprintln!("Error during the creation of the screenshots directory, please create it manually with the name 'screenshots' in the src dir!");
                } else if path.eq(BASE_PATH_FAVORITE_SHORTCUT) {
                    eprintln!("Error during the creation of the favorite shortcut directory, please create it manually with the name 'shortcut' in the src dir!");
                }
                exit(1);
            }
        }
    }
}