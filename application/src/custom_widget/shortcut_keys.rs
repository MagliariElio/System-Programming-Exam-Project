use std::collections::HashSet;
use std::error::Error;
use std::fs::{File, OpenOptions};
use std::io::{Cursor, Read, Write};
use std::process::exit;
use druid::{Code, Data, Lens, Selector};
use flate2::Compression;
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use image::EncodableLayout;
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

fn compress(data: &[u8]) -> std::io::Result<Vec<u8>> {
    let mut encoder = ZlibEncoder::new(Vec::<u8>::new(), Compression::best());
    encoder.write_all(data).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    let compressed_data = encoder.finish()?;
    Ok(compressed_data)
}

fn decompress(data: Vec<u8>) -> Result<Vec<u8>, Box<dyn Error>> {
    let cursor = Cursor::new(data);
    let mut decoder = ZlibDecoder::new(cursor);
    let mut decompressed_data = Vec::new();
    decoder.read_to_end(&mut decompressed_data)?;
    Ok(decompressed_data)
}

pub fn write_to_file<T>(file_path: &str, data: &T) -> Result<(), Box<dyn std::error::Error>>
    where
        T: serde::Serialize,
{
    verify_exists_dir(BASE_PATH_FAVORITE_SHORTCUT);
    let serialized_data = serde_json::to_string(data)?;
    let binding = compress(serialized_data.as_bytes())?;
    let compress_serialized_data = binding.as_bytes();

    //let serialized_compress_data = serde_json::to_string(&compress_serialized_data)?;
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(file_path)?;

    file.write_all(compress_serialized_data)?;

    Ok(())
}

pub fn read_from_file<T>(file_path: &str) -> Option<T>
    where
        T: for<'de> serde::Deserialize<'de>,
{
    if let Ok(mut file) = File::open(file_path) {
        let mut buffer = Vec::new();
        if file.read_to_end(&mut buffer).is_ok() {
            let decompress_serialized_data = decompress(buffer).ok()?;
            let decompressed = std::str::from_utf8(&decompress_serialized_data).ok()?;
            Some(serde_json::from_str(decompressed).ok()?)
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