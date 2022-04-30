use clap::{Arg, ArgMatches, Command};
use crunch::{Item, Rotation};
use image::{GenericImage, ImageFormat, RgbaImage};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::ffi::OsStr;
use std::fs;
use std::path::PathBuf;

/// TODO
/// - padding!
/// - rotation!
/// - extruding
/// - trimming transparency
/// - different formats
/// - output name/folder/etc.
/// - what happens if images can't fit? multiple packs?
/// - replace unwraps() with proper errors (crates: thiserror, eyre/anyhow)
/// -
fn main() {
    // Describe our command line program
    let matches: ArgMatches = Command::new("Scrunch")
        .arg_required_else_help(true)
        .subcommand(
            Command::new("new")
                .about("Creates a new sprite atlas.")
                .arg(Arg::new("name").required(true)),
        )
        .subcommand(
            Command::new("add")
                .about("Adds an image or folder to the sprite atlas.")
                .arg(Arg::new("file").required(true)),
        )
        .subcommand(Command::new("pack").about("Packs the sprite atlas."))
        .get_matches();

    // Parse the user input and see which command they called
    match matches.subcommand() {
        Some(("new", args)) => {
            let name = args.value_of("name").unwrap();
            match fs::create_dir(name) {
                Ok(()) => {
                    let new_atlas = Atlas::new();
                    new_atlas.save(Some(name));
                    println!("successfully created atlas: {}", name);
                }
                Err(err) => {
                    println!("failed to create atlas {} ({})", name, err);
                }
            }
        }

        Some(("add", args)) => {
            let mut atlas = Atlas::load();

            let file = args.value_of("file").unwrap();
            let file = PathBuf::from(file);
            if !file.exists() {
                println!("file does not exist: {:?}", file);
                return;
            }

            if file.is_file() {
                atlas.images.insert(file);
            } else if file.is_dir() {
                atlas.dirs.insert(file);
            }

            atlas.save(None);
        }
        Some(("pack", _args)) => {
            let atlas = Atlas::load();

            let mut used_paths = HashSet::new();
            let mut images = Vec::new();
            let mut to_pack = Vec::new();

            println!("loading images...");

            for img_path in atlas
                .images
                .into_iter()
                .chain(
                    atlas
                        .dirs
                        .iter()
                        .filter_map(|dir| fs::read_dir(dir).ok())
                        .flat_map(|dir| dir.filter_map(|e| e.ok().and_then(|e| Some(e.path())))),
                )
                .filter(|p| used_paths.insert(p.clone()))
                .filter(|p| matches!(p.extension().and_then(OsStr::to_str), Some("png" | "jpg")))
            {
                println!("\t{:?}", img_path);

                let img = image::open(&img_path).unwrap().to_rgba8();

                to_pack.push(Item::new(
                    images.len(),
                    img.width() as usize,
                    img.height() as usize,
                    Rotation::None,
                ));

                images.push((img_path.clone(), img));
            }

            println!("packing rectangles...");

            // Pack the rectangles
            let (atlas_w, atlas_h, packed) = crunch::pack_into_po2(4096, to_pack).unwrap();

            println!("rendering final atlas...");

            // Create an atlas image to draw to
            let mut atlas_img: RgbaImage = RgbaImage::new(atlas_w as u32, atlas_h as u32);

            let mut desc = Descriptor::default();

            // Draw all the images to the atlas image where they were packed
            for (rect, img_index) in packed {
                let (img_path, img) = &images[img_index];

                desc.entries.push(Entry {
                    name: img_path.clone(),
                    x: rect.x,
                    y: rect.y,
                    w: rect.w,
                    h: rect.h,
                });

                atlas_img
                    .copy_from(img, rect.x as u32, rect.y as u32)
                    .unwrap();
            }

            println!("saving atlas...");

            // Save the atlas image file
            atlas_img
                .save_with_format("atlas.png", ImageFormat::Png)
                .unwrap();

            // Save the descriptor file
            let desc_str = serde_json::to_string_pretty(&desc).unwrap();
            fs::write("atlas.json", desc_str).unwrap();

            println!("atlas packed and exported!");
        }
        _ => {}
    }
}

#[derive(Serialize, Deserialize)]
pub struct Atlas {
    #[serde(default)]
    images: HashSet<PathBuf>,

    #[serde(default)]
    dirs: HashSet<PathBuf>,
}

impl Atlas {
    pub fn new() -> Self {
        Self {
            images: HashSet::new(),
            dirs: HashSet::new(),
        }
    }

    pub fn save(&self, dir: Option<&str>) {
        let toml_str = toml::to_string(&self).unwrap();
        let path = dir
            .and_then(|dir| Some(PathBuf::from(dir)))
            .unwrap_or_else(PathBuf::new)
            .join("atlas.toml");
        fs::write(path, toml_str).unwrap();
    }

    pub fn load() -> Self {
        let path = PathBuf::from("atlas.toml");
        let toml_str = fs::read_to_string(path).unwrap();
        toml::from_str(&toml_str).unwrap()
    }
}

#[derive(Default, Serialize, Deserialize)]
pub struct Descriptor {
    entries: Vec<Entry>,
}

#[derive(Default, Serialize, Deserialize)]
pub struct Entry {
    name: PathBuf,
    x: usize,
    y: usize,
    w: usize,
    h: usize,
}
