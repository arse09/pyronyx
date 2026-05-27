use std::collections::HashSet;

use super::{Writer, file_header};
use crate::{
    codegen::{VENDORS, rust_member, rust_name},
    parse::{
        self,
        registry::{Depends, Registry},
    },
};
use heck::ToSnakeCase;
use indexmap::{IndexMap, IndexSet};
use parse::registry::VkCommand;

pub fn generate(registry: &Registry, out_path: &str, table_out_path: &str) {
    let mut w = Writer::new();
    file_header(&mut w, "vk/commands.rs");
    w.ln("#![allow(non_camel_case_types)]");
    w.ln("#![allow(unused)]");
    w.ln("use super::types::*;");
    w.ln("use super::platform_types::*;");
    w.ln("use super::enums::*;");
    w.ln("use super::bitflags::*;");
    w.ln("use core::ffi::{c_void, c_char, c_int};");
    w.ln("use crate::utils::to_option;");
    w.blank();

    w.ln("// ── fn-pointer Typen ──────────────────────────────────────────────");
    for (_, cmd) in &registry.commands {
        write_pfn_type(&mut w, cmd);
        w.blank();
    }
    w.save(out_path);

    let mut w = Writer::new();
    file_header(&mut w, "vk/commands.rs");
    w.ln("use super::vk::*;");
    w.ln("use core::ffi::{CStr, c_void, c_char};");
    w.ln("use crate::utils::to_option;");
    w.ln("use crate::utils::to_panic;");

    let mut instance_table = IndexSet::new();
    let mut device_table = IndexSet::new();

    write_dispatch_groups(
        &mut w,
        "InstanceFn",
        registry,
        &mut instance_table,
        is_instance_command,
    );

    write_dispatch_groups(
        &mut w,
        "PhysicalDeviceFn",
        registry,
        &mut instance_table,
        is_physical_device_command,
    );

    write_dispatch_groups(&mut w, "DeviceFn", registry, &mut device_table, |cmd| {
        is_device_command(cmd)
    });

    write_dispatch_groups(
        &mut w,
        "QueueFn",
        registry,
        &mut device_table,
        is_queue_command,
    );

    write_dispatch_groups(
        &mut w,
        "CommandBufferFn",
        registry,
        &mut device_table,
        is_cmd_command,
    );

    write_dispatch_table(&mut w, "InstanceVTable", instance_table);
    write_dispatch_table(&mut w, "DeviceVTable", device_table);

    w.save(table_out_path);
}

fn write_pfn_type(w: &mut Writer, cmd: &VkCommand) {
    let mut ps = HashSet::new();

    let params = cmd
        .params
        .iter()
        .map(|p| format!("{}: {}", rust_member(&p.name, &mut ps), &p.ty))
        .collect::<Vec<_>>()
        .join(", ");

    let ret = if cmd.return_type == "c_void" {
        String::new()
    } else {
        format!(" -> {}", rust_name(&cmd.return_type))
    };
    w.ln(&format!(
        "pub type {name} = unsafe extern \"system\" fn({params}){ret};",
        name = cmd.name,
    ));
}

fn version_const_name(depends_str: &str) -> String {
    let version_part = depends_str.strip_prefix("v").unwrap_or(depends_str);
    format!("API_VERSION_{}", version_part)
}

fn write_dispatch_groups(
    w: &mut Writer,
    struct_name: &str,
    registry: &Registry,
    groups: &mut IndexSet<String>,
    filter: impl Fn(&VkCommand) -> bool,
) {
    let cmds: Vec<&VkCommand> = registry.commands.values().filter(|c| filter(c)).collect();
    if cmds.is_empty() {
        return;
    }

    let mut versions: IndexMap<Depends, Vec<&VkCommand>> = IndexMap::new();
    for cmd in cmds {
        if cmd.name.ends_with("ProcAddr") {
            continue;
        }
        versions.entry(cmd.table_name()).or_default().push(cmd);
    }
    versions.sort_keys();

    groups.insert(struct_name.to_string());

    w.ln("#[derive(Clone)]");
    w.ln(&format!("pub struct {struct_name} {{"));
    for (depends, _) in &versions {
        let optional = matches!(depends, Depends::Ext(_));
        let member_name = if optional {
            format!("Option<{struct_name}{}>", depends.to_string())
        } else {
            format!("{struct_name}{}", depends.to_string())
        };
        w.ln(&format!(
            "    pub {}: {},",
            depends.to_string().to_snake_case(),
            member_name
        ));
    }
    w.ln("}");
    w.blank();

    // ── load-Methode mit api_version ─────────────────────────────────────
    w.ln(&format!("impl {struct_name} {{"));
    w.ln("    pub fn load<F: FnMut(&CStr) -> *const c_void>(");
    w.ln("        mut loader: F,");
    w.ln("        api_version: u32,");
    w.ln("        extensions: &[*const c_char],");
    w.ln("    ) -> Self {");
    w.ln("        let mut out = Self {");

    for (depends, _) in &versions {
        let field = depends.to_string().to_snake_case();
        let subgroup = format!("{struct_name}{}", depends.to_string());

        match depends {
            Depends::Ext(_) => w.ln(&format!(r#"{field}: None,"#)),
            Depends::Core(v) => {
                if v.as_str() == "v1_0" {
                    w.ln(&format!(r#"{field}: {subgroup}::load(&mut loader),"#))
                } else {
                    let ver_const = version_const_name(depends.to_string().as_str());
                    w.ln(&format!(
                        r#"            {field}: if api_version >= {ver_const} {{
                {subgroup}::load(&mut loader)
            }} else {{
                {subgroup}::default()
            }},"#
                    ));
                }
            }
            _ => {
                let ver_const = version_const_name(depends.to_string().as_str());
                w.ln(&format!(
                    r#"            {field}: if api_version >= {ver_const} {{
                {subgroup}::load(&mut loader)
            }} else {{
                {subgroup}::default()
            }},"#
                ));
            }
        }
    }
    w.ln("        };");

    w.ln("        for &ext in extensions {");
    w.ln("            let ext = unsafe { CStr::from_ptr(ext).to_bytes() };");
    w.ln("            match ext {");
    for (depends, cmd) in &versions {
        if let Some(ext_name) = cmd.first().and_then(|c| c.extension.as_ref()) {
            let field = depends.to_string().to_snake_case();
            let subgroup = format!("{struct_name}{}", depends.to_string());
            w.ln(&format!(
                r#"                b"{ext_name}" => out.{field} = Some({subgroup}::load(&mut loader)),"#
            ));
        }
    }
    w.ln("                _ => (),");
    w.ln("            }");
    w.ln("        }");
    w.ln("        out");
    w.ln("    }");
    w.ln("}");
    w.blank();

    for (depends, cmds) in &versions {
        let struct_name_sub = format!("{struct_name}{}", depends.to_string());
        let optional_struct = matches!(depends, Depends::Ext(_));

        if optional_struct {
            w.ln("#[derive(Clone)]");
        } else {
            w.ln("#[derive(Clone, Default)]");
        }
        w.ln(&format!("pub struct {struct_name_sub} {{"));
        for cmd in cmds {
            let typ = if cmd.option_member() {
                format!("Option<{}>", cmd.name)
            } else {
                cmd.name.clone()
            };
            w.ln(&format!("    pub {}: {},", fn_field(&cmd.name), typ));
        }
        w.ln("}");
        w.blank();

        w.ln(&format!("impl {struct_name_sub} {{"));
        w.ln("    pub fn load<F: FnMut(&CStr) -> *const c_void>(mut loader: F) -> Self {");
        w.ln("        Self {");
        for cmd in cmds {
            let field = fn_field(&cmd.name);
            let name = &cmd.name;
            if optional_struct {
                w.ln(&format!(
                    r#"            {field}: to_panic(loader(c"{name}")),"#
                ));
            } else {
                w.ln(&format!(
                    r#"            {field}: to_option(loader(c"{name}")),"#
                ));
            }
        }
        w.ln("        }");
        w.ln("    }");
        w.ln("}");
        w.blank();
    }
}

fn write_dispatch_table(w: &mut Writer, struct_name: &str, groups: IndexSet<String>) {
    w.ln("#[derive(Clone)]");
    w.ln(&format!("pub struct {struct_name} {{"));
    for group in &groups {
        w.ln(&format!("    pub {}: {},", fn_field(group), group));
    }
    w.ln("}");
    w.blank();

    w.ln(&format!("impl {struct_name} {{"));
    w.ln("    pub fn load<F: FnMut(&CStr) -> *const c_void>(");
    w.ln("        mut loader: F,");
    w.ln("        api_version: u32,");
    w.ln("        extensions: &[*const c_char],");
    w.ln("    ) -> Self {");
    w.ln("debug_assert!(api_version >= API_VERSION_1_0);");
    w.ln("Self {");
    for group in &groups {
        w.ln(&format!(
            r#"{field}: {name}::load(&mut loader, api_version, extensions),"#,
            field = fn_field(group),
            name = group,
        ));
    }
    w.ln("        }");
    w.ln("    }");
    w.ln("}");
    w.blank();
}

/// "vkCreateInstance" → "create_instance"
pub fn fn_field(vk_name: &str) -> String {
    let name = vk_name.strip_prefix("vk").unwrap_or(vk_name);
    let name = name.strip_prefix("Cmd").unwrap_or(name);
    let name = name.replace("Fn", "");
    name.to_snake_case()
}

/// "vkCreateInstance" → "create_instance"
pub fn fn_sig(vk_name: &str, self_name: &str) -> String {
    let name =
        if (vk_name.contains("Buffer") || vk_name.contains("Image") || vk_name.contains("Memory"))
            && self_name == "Device"
        {
            vk_name
        } else {
            &vk_name.replace(self_name, "")
        };
    let name = name.strip_prefix("vk").unwrap_or(name);
    let name = name.strip_prefix("Cmd").unwrap_or(name);
    let name = name.replace("Fn", "");
    let mut rname = name.as_str();
    for vendor in VENDORS {
        if let Some(new) = name.strip_suffix(&vendor.to_uppercase()) {
            rname = new;
            break;
        }
    }
    rname.to_snake_case()
}

fn is_instance_command(cmd: &VkCommand) -> bool {
    cmd.params
        .first()
        .map(|p| p.ty == "vkInstance")
        .unwrap_or(false)
}

fn is_physical_device_command(cmd: &VkCommand) -> bool {
    cmd.params
        .first()
        .map(|p| &p.ty == "vkPhysicalDevice")
        .unwrap_or(false)
}

fn is_device_command(cmd: &VkCommand) -> bool {
    cmd.params
        .first()
        .map(|p| &p.ty == "vkDevice")
        .unwrap_or(false)
}

fn is_queue_command(cmd: &VkCommand) -> bool {
    cmd.params
        .first()
        .map(|p| &p.ty == "vkQueue")
        .unwrap_or(false)
}

fn is_cmd_command(cmd: &VkCommand) -> bool {
    cmd.params
        .first()
        .map(|p| &p.ty == "vkCommandBuffer")
        .unwrap_or(false)
}
