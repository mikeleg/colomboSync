//! What the integration tests of both engines have in common.

use colombo_sync_lib::engine::preview_parser::Counts;
use colombo_sync_lib::run::runner::{self, ActiveRun};
use colombo_sync_lib::shared::domain::{Run, SavedSync, SyncMode};
use colombo_sync_lib::shared::storage::Storage;
use std::fs;
use std::path::Path;
use std::time::SystemTime;

pub fn write(path: &Path, text: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

pub fn path_str(path: &Path) -> &str {
    path.to_str().unwrap()
}

pub fn set_modified(path: &Path, time: SystemTime) {
    fs::File::options()
        .write(true)
        .open(path)
        .unwrap()
        .set_modified(time)
        .unwrap();
}

/// The app's data folder inside a test's temp folder.
pub fn storage_in(temp: &Path) -> Storage {
    Storage::at(temp.join("ColomboSync"))
}

pub fn saved(name: &str, a: &Path, b: &Path) -> SavedSync {
    SavedSync {
        id: 7,
        name: name.into(),
        source: path_str(a).into(),
        destination: path_str(b).into(),
        mode: SyncMode::Copy,
    }
}

pub fn run_mirror(storage: &Storage, a: &Path, b: &Path, counts: Counts) -> Result<Run, String> {
    runner::run(
        storage,
        &SavedSync {
            mode: SyncMode::Mirror,
            ..saved("Copia di A", a, b)
        },
        counts,
        &ActiveRun::default(),
        |_| {},
    )
}
