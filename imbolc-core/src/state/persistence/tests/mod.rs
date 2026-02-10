use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::state::persistence::{save_project, load_project};

mod arrangement;
mod basic;
mod decoders;
mod instruments;
mod mixer;

fn temp_db_path() -> PathBuf {
    let mut path = std::env::temp_dir();
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    path.push(format!("imbolc_persistence_test_{}.sqlite", nanos));
    path
}
