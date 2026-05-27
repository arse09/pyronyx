use std::collections::HashSet;

use super::{Writer, file_header};
use crate::{
    codegen::{bitflags::value_to_const_name, rust_name},
    parse::registry::{Registry, VkEnum},
    parse::types::TypeKind,
};

pub fn generate(registry: &Registry, out_path: &str) {
    let mut w = Writer::new();
    file_header(&mut w, "generated Display impls for bitflag types");
    w.ln("use crate::vk::*;");
    w.ln("use core::fmt::{Formatter, Debug, Display, Result};");
    w.blank();

    for (_, ty) in &registry.types {
        let (bits_enum, _repr) = match &ty.kind {
            TypeKind::Bitmask { bits_enum, repr } => (bits_enum, repr),
            _ => continue,
        };

        let en = if let Some(name) = bits_enum
            && let Some(en) = registry.enums.get(name)
        {
            en
        } else {
            continue;
        };

        write_display_impl(&mut w, en);
    }

    w.save(out_path);
}

fn write_display_impl(w: &mut Writer, en: &VkEnum) {
    let struct_name = rust_name(&en.name);

    w.ln(&format!("impl Display for {struct_name} {{"));
    w.ln("    fn fmt(&self, f: &mut Formatter<'_>) -> Result {");

    if en.values.is_empty() {
        // No named flags defined — always print "0".
        w.ln("        f.write_str(\"0\")");
    } else {
        // Deduplicate by const name — same as in write_bitflag_struct.
        let mut seen: HashSet<String> = HashSet::new();
        let variants: Vec<String> = en
            .values
            .iter()
            .filter_map(|v| {
                let const_name = value_to_const_name(&en.name, &v.name);
                seen.insert(const_name.clone()).then_some(const_name)
            })
            .collect();

        w.ln(&format!(
            "        const BITS: &[({struct_name}, &str)] = &["
        ));
        for const_name in &variants {
            w.ln(&format!(
                "            ({struct_name}::{const_name}, \"{const_name}\"),"
            ));
        }
        w.ln("        ];");
        w.blank();
        w.ln("        let mut first = true;");
        w.ln("        for &(flag, name) in BITS {");
        w.ln("            if self.contains(flag) {");
        w.ln("                if !first { f.write_str(\" | \")?; }");
        w.ln("                f.write_str(name)?;");
        w.ln("                first = false;");
        w.ln("            }");
        w.ln("        }");
        w.ln("        if first { f.write_str(\"0\")?; }");
        w.ln("        Ok(())");
    }

    w.ln("    }");
    w.ln("}");
    w.blank();

    // Debug delegates to Display — no duplicated logic.
    w.ln(&format!("impl Debug for {struct_name} {{"));
    w.ln("    fn fmt(&self, f: &mut Formatter<'_>) -> Result {");
    w.ln("        Display::fmt(self, f)");
    w.ln("    }");
    w.ln("}");
    w.blank();
}
