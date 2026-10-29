// Embeds the art toolkit (`web/art`, docs/ART.md 15-16) into the binary for the browser client:
// `$OUT_DIR/art.bundle.txt` (the sprite data in one text) and `$OUT_DIR/art_assets.rs` (the
// modules, each by `include_str!`), which `src/transport/http.rs` includes. The file lists come
// from `src/transport/art_files.rs`, which the server also uses for `--assets DIR`.
// Also embeds the isometric renderer's sprites (`assets/app/iso/*.png`, docs/ISO.md, written by
// `tools/m7/m7.py export`) and the HD-2D mode's own art (`assets/app/r3d/hd2d/art/*.png`, its
// ground and prop cards, docs/HD2D.md): `$OUT_DIR/iso_assets.rs`, every PNG there by `include_bytes!`.
// And sets `FFH_BUILD_COMMIT` (`crate::BUILD_COMMIT`): the commit being built, for bug reports.

use std::path::{Path, PathBuf};

include!("src/transport/art_files.rs");

fn main() {
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let art = manifest.join("web").join("art");
    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/transport/art_files.rs");
    // A directory: cargo rescans every file under it.
    println!("cargo:rerun-if-changed=web/art");

    let bundle = art_bundle(&art).unwrap_or_else(|e| panic!("web/art: the sprite bundle: {e}"));
    write_if_changed(&out.join("art.bundle.txt"), &bundle);

    let modules = art_modules(&art).unwrap_or_else(|e| panic!("web/art: the modules: {e}"));
    let mut rs = String::from("/// The art modules (`web/art`), served at `/art/<path>`: `(path, content)`.\n");
    rs.push_str("pub const ART_MODULES: &[(&str, &str)] = &[\n");
    for m in &modules {
        let file = art.join(m);
        rs.push_str(&format!("    ({:?}, include_str!({:?})),\n", format!("art/{m}"), file.to_string_lossy()));
    }
    rs.push_str("];\n");
    write_if_changed(&out.join("art_assets.rs"), &rs);

    let mut rs = String::from("/// The isometric renderer's sprites (`assets/app/iso/*.png`, docs/ISO.md) and the HD-2D mode's art\n/// (`assets/app/r3d/hd2d/art/*.png`, docs/HD2D.md): `(path, content type, bytes)`.\n");
    rs.push_str("pub const ASSET_BYTES: &[(&str, &str, &[u8])] = &[\n");
    for sub in ["iso", "r3d/hd2d/art"] {
        let dir = sub.split('/').fold(manifest.join("assets").join("app"), |d, s| d.join(s));
        println!("cargo:rerun-if-changed=assets/app/{sub}");
        let mut pngs: Vec<String> = std::fs::read_dir(&dir)
            .unwrap_or_else(|e| panic!("{}: {e}", dir.display()))
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .filter(|n| n.ends_with(".png"))
            .collect();
        pngs.sort();
        for n in &pngs {
            rs.push_str(&format!("    ({:?}, PNG, include_bytes!({:?})),\n", format!("app/{sub}/{n}"), dir.join(n).to_string_lossy()));
        }
    }
    rs.push_str("];\n");
    write_if_changed(&out.join("iso_assets.rs"), &rs);

    build_commit(&manifest);
}

/// `cargo:rustc-env=FFH_BUILD_COMMIT`: `$FFH_BUILD_COMMIT` when set, else the short hash of git's
/// HEAD, else "unknown"; the script reruns when HEAD moves (its file and the branch it names).
fn build_commit(manifest: &Path) {
    println!("cargo:rerun-if-env-changed=FFH_BUILD_COMMIT");
    let git = |args: &[&str]| -> Option<String> {
        let out = std::process::Command::new("git").args(args).current_dir(manifest).output().ok()?;
        let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
        (out.status.success() && !s.is_empty()).then_some(s)
    };
    let commit = std::env::var("FFH_BUILD_COMMIT").ok().filter(|s| !s.is_empty()).or_else(|| git(&["rev-parse", "--short=12", "HEAD"]));
    println!("cargo:rustc-env=FFH_BUILD_COMMIT={}", commit.as_deref().unwrap_or("unknown"));
    let abs = |p: String| {
        let p = PathBuf::from(p);
        if p.is_absolute() {
            p
        } else {
            manifest.join(p)
        }
    };
    let mut watch = Vec::new();
    if let Some(dir) = git(&["rev-parse", "--git-dir"]) {
        let dir = abs(dir);
        // HEAD's reflog moves with every commit, checkout and pull, also when the branch's ref
        // is packed (no loose ref file to watch).
        watch.push(dir.join("logs").join("HEAD"));
        watch.push(dir.join("HEAD"));
    }
    if let (Some(common), Some(branch)) = (git(&["rev-parse", "--git-common-dir"]), git(&["symbolic-ref", "-q", "HEAD"])) {
        let common = abs(common);
        watch.push(common.join(branch));
        watch.push(common.join("packed-refs"));
    }
    // A path that does not exist would rerun the script on every build.
    for p in watch.into_iter().filter(|p| p.exists()) {
        println!("cargo:rerun-if-changed={}", p.display());
    }
}

fn write_if_changed(path: &Path, text: &str) {
    if std::fs::read_to_string(path).ok().as_deref() != Some(text) {
        std::fs::write(path, text).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    }
}
