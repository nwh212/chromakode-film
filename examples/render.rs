//! Renders photos through the kit natively, the way the host would at the Display stage:
//! `cargo run --release --example render -- photo.jpg '{"hc": true}'` writes `photo.k64.png`.
//! The measurement for Normalize Exposure is taken from the photo first, as Measure would
//! (pass your own `_analysis` to override it).

use serde_json::Value;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (path, extra) = (&args[0], args.get(1).map(String::as_str).unwrap_or("{}"));
    let suffix = args.get(2).map(String::as_str).unwrap_or("k64");
    let img = image::open(path).expect("open").to_rgb32f();
    let (w, h) = (img.width() as usize, img.height() as usize);
    let mut buf: Vec<f32> = img.into_raw();
    let linear: Vec<f32> = buf
        .iter()
        .map(|&v| chromakode::color::decode_exact(v))
        .collect();
    let analysis = chromakode::analyze(&linear, w, h).unwrap();
    let mut p: Value = serde_json::from_str(extra).expect("params JSON");
    p["encoding"] = "srgb".into();
    if p.get("_analysis").is_none() {
        p["_analysis"] = analysis.clone();
    }
    p["_image"] = serde_json::json!({"x": 0, "y": 0, "canvasWidth": w, "canvasHeight": h});
    chromakode::filter(&mut buf, w, h, &p);
    let out = image::Rgb32FImage::from_raw(w as u32, h as u32, buf).unwrap();
    let out = image::DynamicImage::ImageRgb32F(out).to_rgb8();
    let dest = std::path::Path::new(path).with_extension(format!("{suffix}.png"));
    out.save(&dest).expect("save");
    eprintln!("{} ({analysis})", dest.display());
}
