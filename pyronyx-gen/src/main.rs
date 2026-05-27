mod codegen;
mod parse;

use parse::registry::Registry;
use std::{collections::HashSet, fs, process::Command};

pub const VK_OUT: &str = "pyronyx/src/vk";
pub const OUT: &str = "pyronyx/src";

pub const VULKAN_SDK: &str = env!("VULKAN_SDK");
const VK_XML: &str = "/share/vulkan/registry/vk.xml";
const VIDEO_XML: &str = "/share/vulkan/registry/video.xml";

/// This crate might be the ugliest thing you've ever seen, but it gets the job done. It parses the vk.xml and video.xml files from the Vulkan SDK and generates Rust code for the Pyronyx crate.
/// The crate is the place where the code should be perfect. Still good to make this also cleaner.

fn main() {
    let vk_xml = format!("{VULKAN_SDK}{VK_XML}");
    let video_xml = format!("{VULKAN_SDK}{VIDEO_XML}");

    let xml = fs::read_to_string(&vk_xml).expect("vk.xml nicht gefunden");
    let video_src = fs::read_to_string(&video_xml).expect("video.xml nicht gefunden");

    let mut lifetimes = HashSet::new();

    // vk.xml is dependent on video.xml so we need to parse it first to propagate the lifetimes
    let mut video_registry = Registry::parse_video(&video_src, &mut lifetimes);
    let mut registry = Registry::parse(&xml, &mut lifetimes);

    // ── video ──────────────────────────────────────────────────────────────
    codegen::enums::generate(&mut video_registry, &format!("{OUT}/video/enums.rs"));
    codegen::video::generate(
        &video_registry,
        &format!("{OUT}/video/types.rs"),
        &lifetimes,
    );

    // ── vk ─────────────────────────────────────────────
    codegen::enums::generate(&mut registry, &format!("{VK_OUT}/enums.rs"));
    codegen::types::generate(&registry, &format!("{VK_OUT}/types.rs"), &lifetimes);
    codegen::bitflags::generate(&registry, &format!("{VK_OUT}/bitflags.rs"));
    codegen::bitflags_debug::generate(&registry, &format!("{VK_OUT}/display_bitflags.rs"));
    codegen::commands::generate(
        &registry,
        &format!("{VK_OUT}/commands.rs"),
        &format!("{OUT}/vtables.rs"),
    );
    codegen::constants::generate(&registry, &format!("{VK_OUT}/constants.rs"));
    codegen::impls::generate(&registry, &format!("{OUT}/impl"), &lifetimes);
    codegen::extensions::generate(&registry, &format!("{OUT}/extensions"), &lifetimes);

    // ── rustfmt ───────────────────────────────────────────────────────────
    Command::new("cargo")
        .args(["fmt"])
        .current_dir("../pyronyx")
        .status()
        .expect("cargo fmt failed");

    println!("✓ vk.xml     ({} Types)", registry.types.len());
    println!("✓ vk.xml     ({} Enums)", registry.enums.len());
    println!("✓ vk.xml     ({} Commands)", registry.commands.len());
    println!("✓ video.xml  ({} Types)", video_registry.types.len());
    println!("✓ video.xml  ({} Enums)", video_registry.enums.len());
    println!("→ {VK_OUT}/");
}
