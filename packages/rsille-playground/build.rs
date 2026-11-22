use std::{
    collections::BTreeMap,
    env,
    fmt::Write,
    fs,
    io::BufReader,
    path::{Component, Path, PathBuf},
};

fn main() {
    let root = Path::new("deepseek-pet/assets/animations");
    println!("cargo:rerun-if-changed=deepseek-pet/assets/animations/clips.yaml");
    let manifest = fs::read_to_string(root.join("clips.yaml")).expect("read clips.yaml");
    let clips: BTreeMap<String, Vec<(String, u16)>> =
        yaml_serde::from_str(&manifest).expect("parse clips.yaml");

    let mut images = BTreeMap::new();
    let mut frames = String::new();
    let mut output = String::new();
    for (name, steps) in clips {
        assert!(
            name.starts_with(|c: char| c.is_ascii_lowercase())
                && name
                    .bytes()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'_'),
            "invalid clip name {name}"
        );
        assert!(!steps.is_empty(), "empty clip {name}");
        let mut duration = 0u16;
        let mut timeline = Vec::new();
        for (file, ticks) in steps {
            assert!(ticks > 0, "zero duration in {name}: {file}");
            duration = duration
                .checked_add(ticks)
                .expect("clip exceeds 65535 ticks");
            let next = images.len();
            let id = *images.entry(file.clone()).or_insert_with(|| {
                validate_frame(root, &file);
                writeln!(frames, "Frame::new(include_bytes!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/deepseek-pet/assets/animations/\", {file:?}))),").unwrap();
                next
            });
            timeline.push((duration, id));
        }
        let ident = name.to_ascii_uppercase();
        writeln!(output, "pub(super) static {ident}: Clip = Clip {{ duration: {duration}, timeline: &{timeline:?} }};").unwrap();
    }
    let count = images.len();
    writeln!(output, "static FRAMES: [Frame; {count}] = [{frames}];").unwrap();
    fs::write(
        PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("animations.rs"),
        output,
    )
    .expect("write embedded animation catalog");
}

fn validate_frame(root: &Path, file: &str) {
    assert!(
        Path::new(file)
            .components()
            .all(|c| matches!(c, Component::Normal(_))),
        "invalid frame path {file}"
    );
    let path = root.join(file);
    println!("cargo:rerun-if-changed={}", path.display());
    let input = fs::File::open(&path).unwrap_or_else(|e| panic!("{file}: {e}"));
    let reader = png::Decoder::new(BufReader::new(input))
        .read_info()
        .unwrap_or_else(|e| panic!("{file}: {e}"));
    let info = reader.info();
    assert!(
        (info.width, info.height) == (320, 320)
            && info.color_type == png::ColorType::Rgba
            && info.bit_depth == png::BitDepth::Eight,
        "{file}: expected a 320×320 RGBA8 PNG"
    );
}
