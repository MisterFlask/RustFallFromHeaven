//! Writes everything the game data loader produces, one `name<TAB>Debug text` line per table entry
//! (`GameData::dump`), to the file given (default: stdout). Two dumps compare line by line: a
//! change of the data's format or of the loader must leave them equal.
//!
//! `cargo run --release -p ffh-data --example dump_data -- OUT.tsv` (data from `FFH_DATA_DIR`,
//! else the repository's `data/`).

use std::io::Write;

fn main() {
    let data = ffh_data::GameData::load(&ffh_data::default_data_dir()).expect("failed to load game data");
    let mut text = String::new();
    for (name, value) in data.dump() {
        text.push_str(&name);
        text.push('\t');
        text.push_str(&value);
        text.push('\n');
    }
    match std::env::args().nth(1) {
        Some(path) => std::fs::write(&path, text).expect("cannot write the dump"),
        None => std::io::stdout().write_all(text.as_bytes()).expect("cannot write the dump"),
    }
}
