use crate::parse::registry::{Registry, VkConstant, VkEnum, VkEnumValue};
use indexmap::IndexMap;
use roxmltree::Node;

pub fn parse_into(enums_node: &Node, reg: &mut Registry) {
    let name = match enums_node.attribute("name") {
        Some(n) => n,
        None => return,
    };

    if name == "API Constants" {
        parse_api_constants(enums_node, &mut reg.constants);
        return;
    }

    let is_bitmask = enums_node.attribute("type").unwrap_or("") == "bitmask";
    let bitwidth_64 = enums_node.attribute("bitwidth").unwrap_or("") == "64";

    let values = enums_node
        .children()
        .filter(|n| n.has_tag_name("enum"))
        .filter_map(|n| parse_enum_value(&n, is_bitmask))
        .collect();

    if let Some(en) = reg.enums.get_mut(name) {
        en.values = values;
        en.bitwidth_64 = bitwidth_64;
    } else {
        reg.enums.insert(
            name.to_string(),
            VkEnum {
                bitwidth_64,
                name: name.to_string(),
                is_bitmask,
                values,
            },
        );
    }
}

/// Injects an enum value added by a feature or extension into an existing enum.
/// `extension` is `Some(name)` when called from an extension, `None` for core features.
pub fn parse_extension_enum(
    node: &Node,
    enums: &mut IndexMap<String, VkEnum>,
    extension: Option<&str>,
) {
    let name = node.attribute("name").unwrap_or("");
    let extends = match node.attribute("extends") {
        Some(e) => e,
        None => return,
    };

    let en = match enums.get_mut(extends) {
        Some(e) => e,
        None => return,
    };

    let value = resolve_extension_value(node);

    en.values.push(VkEnumValue {
        name: name.to_string(),
        value,
        comment: node.attribute("comment").map(str::to_string),
        extension: extension.map(str::to_string),
    });
}

fn parse_enum_value(node: &Node, is_bitmask: bool) -> Option<VkEnumValue> {
    let name = node.attribute("name")?;

    if node.attribute("alias").is_some() {
        return None;
    }

    let value = if is_bitmask {
        if let Some(bitpos) = node.attribute("bitpos") {
            1i64 << bitpos.parse::<u32>().ok()?
        } else if let Some(val) = node.attribute("value") {
            parse_value_str(val)?
        } else {
            return None;
        }
    } else {
        parse_value_str(node.attribute("value")?)?
    };

    Some(VkEnumValue {
        name: name.to_string(),
        value,
        comment: node.attribute("comment").map(str::to_string),
        extension: None,
    })
}

fn resolve_extension_value(node: &Node) -> i64 {
    if let Some(val) = node.attribute("value") {
        return parse_value_str(val).unwrap_or(0);
    }
    if let Some(bitpos) = node.attribute("bitpos") {
        return 1i64 << bitpos.parse::<u32>().unwrap_or(0);
    }
    if let Some(offset) = node.attribute("offset") {
        let offset: i64 = offset.parse().unwrap_or(0);
        let extnumber: i64 = node
            .attribute("extnumber")
            .and_then(|n| n.parse().ok())
            .unwrap_or(1);
        let base = 1_000_000_000 + (extnumber - 1) * 1000 + offset;
        return if node.attribute("dir") == Some("-") {
            -base
        } else {
            base
        };
    }
    0
}

fn parse_api_constants(node: &Node, constants: &mut Vec<VkConstant>) {
    for child in node.children().filter(|n| n.has_tag_name("enum")) {
        let name = match child.attribute("name") {
            Some(n) => n,
            None => continue,
        };
        let value = match child.attribute("value") {
            Some(v) => v,
            None => continue,
        };

        constants.push(VkConstant {
            name: name.to_string(),
            ty: infer_constant_type(value).to_string(),
            value: rust_constant_value(value),
            comment: child.attribute("comment").map(str::to_string),
        });
    }
}

fn infer_constant_type(value: &str) -> &'static str {
    if value.ends_with("ULL")
        || value.ends_with("ull")
        || (value.len() > 10 && value.starts_with("0x"))
    {
        "u64"
    } else if value.ends_with('f') || value.contains('.') {
        "f32"
    } else if value.starts_with("(~0") {
        if value.contains("ULL") { "u64" } else { "u32" }
    } else {
        "u32"
    }
}

fn rust_constant_value(value: &str) -> String {
    match value {
        "(~0U)" => "!0u32".to_string(),
        "(~0ULL)" => "!0u64".to_string(),
        "(~0U-1)" => "!0u32 - 1".to_string(),
        "(~0U-2)" => "!0u32 - 2".to_string(),
        v if v.ends_with('f') => format!("{}f32", &v[..v.len() - 1]),
        v if v.ends_with("ULL") => format!("{}u64", &v[..v.len() - 3]),
        v => v.to_string(),
    }
}

fn parse_value_str(s: &str) -> Option<i64> {
    let s = s.trim();
    if s.starts_with("0x") || s.starts_with("0X") {
        i64::from_str_radix(&s[2..], 16).ok()
    } else {
        s.parse().ok()
    }
}
