use super::{Writer, file_header};
use crate::{codegen::const_name, parse::registry::Registry};

pub fn generate(registry: &Registry, out_path: &str) {
    let mut w = Writer::new();
    file_header(&mut w, "vk/constants.rs");

    w.ln(
        r#"/// <https://docs.vulkan.org/refpages/latest/refpages/source/VK_MAKE_API_VERSION.html>
pub const fn make_api_version(variant: u32, major: u32, minor: u32, patch: u32) -> u32 {
    ((variant) << 29) | ((major) << 22) | ((minor) << 12) | (patch)
}
/// <https://docs.vulkan.org/refpages/latest/refpages/source/VK_API_VERSION_VARIANT.html>
pub const fn api_version_variant(version: u32) -> u32 {
    (version) >> 29
}
/// <https://docs.vulkan.org/refpages/latest/refpages/source/VK_API_VERSION_MAJOR.html>
pub const fn api_version_major(version: u32) -> u32 {
    ((version) >> 22) & 0x7fu32
}
/// <https://docs.vulkan.org/refpages/latest/refpages/source/VK_API_VERSION_MINOR.html>
pub const fn api_version_minor(version: u32) -> u32 {
    ((version) >> 12) & 0x3ffu32
}
/// <https://docs.vulkan.org/refpages/latest/refpages/source/VK_API_VERSION_PATCH.html>
pub const fn api_version_patch(version: u32) -> u32 {
    (version) & 0xfffu32
}
/// <https://docs.vulkan.org/refpages/latest/refpages/source/VK_API_VERSION_1_0.html>
pub const API_VERSION_1_0: u32 = make_api_version(0, 1, 0, 0);
/// <https://docs.vulkan.org/refpages/latest/refpages/source/VK_API_VERSION_1_1.html>
pub const API_VERSION_1_1: u32 = make_api_version(0, 1, 1, 0);
/// <https://docs.vulkan.org/refpages/latest/refpages/source/VK_API_VERSION_1_2.html>
pub const API_VERSION_1_2: u32 = make_api_version(0, 1, 2, 0);
/// <https://docs.vulkan.org/refpages/latest/refpages/source/VK_API_VERSION_1_3.html>
pub const API_VERSION_1_3: u32 = make_api_version(0, 1, 3, 0);
/// <https://docs.vulkan.org/refpages/latest/refpages/source/VK_API_VERSION_1_4.html>
pub const API_VERSION_1_4: u32 = make_api_version(0, 1, 4, 0);
"#,
    );

    for c in &registry.constants {
        let name = const_name(&c.name);
        let value = to_rust_value(&c.value);
        let ty = infer_type(&c.value);

        if let Some(comment) = &c.comment {
            w.ln(&format!("/// {comment}"));
        }
        w.ln(&format!("pub const {name}: {ty} = {value};"));
    }

    w.save(out_path);
}

fn infer_type(value: &str) -> &'static str {
    let v = value.trim();

    // Float
    if v.ends_with('f') || v.ends_with('F') || v.contains('.') {
        return "f32";
    }

    // "(~0ULL)" → u64,  "(~0U)" → u32,  "(~0U-1)" → u32
    if v.starts_with("(~") && v.ends_with(')') {
        let inner = &v[2..v.len() - 1];
        // Subtraktion ignorieren: "0U-1" → "0U"
        let base = inner.split('-').next().unwrap_or(inner);
        if base.ends_with("ULL")
            || base.ends_with("ull")
            || base.ends_with("UL")
            || base.ends_with("ul")
        {
            return "u64";
        }
        return "u32";
    }

    if v.ends_with("ULL") || v.ends_with("ull") || v.ends_with("UL") || v.ends_with("ul") {
        return "u64";
    }

    if (v.starts_with("0x") || v.starts_with("0X")) && v.len() - 2 > 8 {
        return "u64";
    }

    if v.ends_with("u64") {
        return "u64";
    }

    "u32"
}

fn to_rust_value(value: &str) -> String {
    let v = value.trim();

    if v.starts_with("(~") && v.ends_with(')') {
        let inner = &v[2..v.len() - 1];
        return parse_bitnot_expr(inner);
    }

    if v.ends_with('f') || v.ends_with('F') {
        return v[..v.len() - 1].to_string();
    }

    // TODO fix roots instead of stripping u32
    if v.ends_with("ULL") || v.ends_with("ull") || v.ends_with("u32") || v.ends_with("u64") {
        return v[..v.len() - 3].to_string();
    }
    if v.ends_with("UL") || v.ends_with("ul") {
        return v[..v.len() - 2].to_string();
    }
    if v.ends_with('U') || v.ends_with('u') {
        return v[..v.len() - 1].to_string();
    }

    if v.starts_with("0x") || v.starts_with("0X") {
        return v.to_ascii_lowercase();
    }

    v.to_string()
}

/// "0U"    → "!0u32"
/// "0ULL"  → "!0u64"
/// "2U"    → "!2u32"
/// "2ULL"  → "!2u64"
/// "0U-1"  → "!0u32 - 1"
/// "0U-2"  → "!0u32 - 2"
fn parse_bitnot_expr(inner: &str) -> String {
    let (base, sub) = if let Some(pos) = inner.find('-') {
        let sub: u32 = inner[pos + 1..].parse().unwrap_or(0);
        (&inner[..pos], Some(sub))
    } else {
        (inner, None)
    };

    let digits = base
        .trim_end_matches("ULL")
        .trim_end_matches("ull")
        .trim_end_matches("UL")
        .trim_end_matches("ul")
        .trim_end_matches('U')
        .trim_end_matches('u');

    match sub {
        None => format!("!{digits}"),
        Some(s) => format!("!{digits} - {s}"),
    }
}
