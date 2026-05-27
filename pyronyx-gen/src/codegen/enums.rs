use std::collections::HashSet;

use super::{Writer, file_header};
use crate::{
    codegen::{
        VENDORS,
        bitflags::{enum_name_to_screaming_prefix, value_to_const_name},
        rust_name,
    },
    parse::{self, registry::Registry},
};
use heck::ToPascalCase;
use indexmap::IndexMap;
use parse::registry::VkEnum;

pub fn generate(registry: &mut Registry, out_path: &str) {
    let mut w = Writer::new();
    file_header(&mut w, "vk/enums.rs");
    w.ln("#![allow(non_camel_case_types)]");
    w.blank();

    for (_, en) in registry.enums.iter().filter(|(_, e)| !e.is_bitmask) {
        write_enum(&mut w, en, &mut registry.stypes);
    }

    w.save(out_path);
}

fn write_enum(w: &mut Writer, en: &VkEnum, types: &mut IndexMap<String, (String, bool)>) {
    w.ln(&format!(
        "/// <https://docs.vulkan.org/refpages/latest/refpages/source/{}.html>",
        en.name
    ));
    w.ln("#[repr(C)]");
    w.ln("#[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq)]");
    w.ln(&format!("pub enum {} {{", rust_name(&en.name)));

    let mut seen = HashSet::new();
    let mut first = true;

    for v in &en.values {
        if !seen.insert(v.value) {
            continue;
        }
        if let Some(c) = &v.comment {
            w.ln(&format!("/// {c}"));
        }
        if first {
            first = false;
            w.ln("#[default]");
        }
        w.ln(&format!(
            "{} = {},",
            variant_name(&en.name, &v.name, types),
            v.value
        ));
    }
    w.ln("}");
    w.blank();

    // Emit `impl` const aliases for duplicate discriminant values.
    for v in &en.values {
        if seen.contains(&v.value) {
            let vname = variant_name(&en.name, &v.name, types);
            let original = en
                .values
                .iter()
                .find(|x| x.value == v.value && x.name != v.name);
            if let Some(orig) = original {
                let name = rust_name(&en.name);
                w.ln(&format!(
                    "impl {name} {{\n    \
                     #[allow(non_upper_case_globals)]\n    \
                     pub const {vname}: {name} = {name}::{};\n}}",
                    variant_name(&en.name, &orig.name, types),
                ));
            }
        }
    }

    w.blank();
}

/// Converts a raw Vulkan enum value name to the Rust variant / const name that
/// appears inside the generated enum.
///
/// | enum              | strategy                                           |
/// |-------------------|----------------------------------------------------|
/// | `VkStructureType` | lookup in the known-struct map, pascal fallback     |
/// | `VkFormat`        | [`format_pascal_case`] to preserve channel specs   |
/// | everything else   | [`value_to_const_name`] (shared with bitflag codegen) |
pub fn variant_name(
    enum_name: &str,
    vk_name: &str,
    types: &mut IndexMap<String, (String, bool)>,
) -> String {
    if enum_name == "VkStructureType" {
        // Build a normalised lookup key: lowercase, no underscores, no "structuretype".
        let search = vk_name
            .to_lowercase()
            .replace('_', "")
            .replace("structuretype", "");

        if let Some(entry) = types.get_mut(&search) {
            entry.1 = true;
            return rust_name(&entry.0);
        }

        // Fallback: strip the "VK_STRUCTURE_TYPE_" prefix and convert to PascalCase,
        // then fix up any vendor suffix so it stays ALL_CAPS.
        let prefix = enum_name_to_screaming_prefix(enum_name);
        let without = vk_name.strip_prefix(&prefix).unwrap_or(vk_name);
        let mut camel = without.to_pascal_case();

        for vendor in VENDORS {
            if camel.ends_with(vendor) {
                let new_len = camel.len() - vendor.len();
                camel.truncate(new_len);
                camel.push_str(&vendor.to_uppercase());
                break;
            }
        }
        return camel;
    }

    // General case: shared logic with bitflag codegen.
    value_to_const_name(enum_name, vk_name)
}

/// Pascal-case converter for `VkFormat` value names.
///
/// Generic pascal-case (as provided by `heck`) destroys information in format
/// names: `R8G8B8A8` → `R8g8b8a8` because it treats the whole token as one
/// SCREAMING word and lowercases everything after the first character.
///
/// Format names split naturally on `_` into two kinds of segments:
///
/// - **Channel specs** – contain at least one digit, e.g. `R8G8B8A8`, `BC1`,
///   `PACK8`, `4x4`.  These encode bit-widths and component counts that must be
///   preserved verbatim; the letters are already upper-case in the XML.
/// - **Word segments** – purely alphabetic, e.g. `UNORM`, `BLOCK`, `SFLOAT`.
///   These are converted to first-upper / rest-lower (`Unorm`, `Block`, …).
///
/// Examples:
/// - `R8G8B8A8_UNORM`          → `R8G8B8A8Unorm`
/// - `BC1_RGB_UNORM_BLOCK`     → `BC1RgbUnormBlock`
/// - `ASTC_4x4_UNORM_BLOCK`    → `Astc4x4UnormBlock`
/// - `R16G16_SFLOAT`           → `R16G16Sfloat`
/// - `R8G8B8A8_UNORM_PACK32`   → `R8G8B8A8UnormPack32`
pub fn better_pascal_case(s: &str) -> String {
    s.split('_')
        .map(|seg| {
            if VENDORS.iter().any(|v| v.eq_ignore_ascii_case(seg)) {
                seg.to_string()
            } else {
                let chars = seg.chars();
                let mut is_first = true;

                let mut out = String::with_capacity(seg.len());

                for ch in chars {
                    if ch.is_ascii_digit() {
                        is_first = true;
                        out.push(ch);
                    } else if is_first {
                        out.push(ch);
                        is_first = false;
                    } else {
                        out.push(ch.to_ascii_lowercase());
                    }
                }
                out
            }
        })
        .collect()
}
