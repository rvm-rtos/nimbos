use std::fs::{read_dir, File};
use std::io::{Result, Write};
use std::path::PathBuf;
use toml::Value;

fn main() {
    println!("cargo:rerun-if-changed=../user/c/src");
    println!("cargo:rerun-if-changed=../user/rust/src");
    println!("cargo:rerun-if-changed=.makeargs");

    let arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap();
    let platform = if cfg!(feature = "platform-pc") {
        "pc"
    } else if cfg!(feature = "platform-pc-rvm") {
        "pc-rvm"
    } else if cfg!(feature = "platform-qemu-virt-arm") {
        "qemu-virt-arm"
    } else if cfg!(feature = "platform-qemu-virt-riscv") {
        "qemu-virt-riscv"
    } else {
        panic!("Unsupported platform!");
    };

    parse_platform_config(&arch, platform).unwrap();
    link_app_data(&arch).unwrap();
}

fn parse_platform_config(arch: &str, platform: &str) -> Result<()> {
    // Load config file
    let config_path = PathBuf::from("platforms").join(format!("{}.toml", platform));
    println!("Reading config file: {}", config_path.display());
    let config_content = std::fs::read_to_string(config_path)?;
    let config: Value = toml::from_str(&config_content)?;

    // Generate config.rs
    let mut out_file = File::create("src/platform/config.rs")?;
    writeln!(out_file, "// {}-{}", arch, platform)?;
    writeln!(out_file, "// Generated by build.rs, DO NOT edit!")?;
    writeln!(out_file, "#![allow(dead_code)]\n")?;

    for (key, value) in config.as_table().unwrap() {
        let var_name = key.to_uppercase().replace('-', "_");
        if let Value::String(s) = value {
            writeln!(out_file, "pub const {}: usize = {};", var_name, s)?;
        }
    }

    writeln!(out_file, "#[rustfmt::skip]")?;
    writeln!(out_file, "pub const MMIO_REGIONS: &[(usize, usize)] = &[")?;
    if let Some(regions) = config["mmio-regions"].as_array() {
        for r in regions {
            let r = r.as_array().unwrap();
            writeln!(
                out_file,
                "    ({}, {}),",
                r[0].as_str().unwrap(),
                r[1].as_str().unwrap()
            )?;
        }
    }
    writeln!(out_file, "];")?;

    // Update linker.ld
    let kernel_base_vaddr = config["kernel-base-vaddr"]
        .as_str()
        .unwrap()
        .replace('_', "");
    let ld_content = std::fs::read_to_string("linker.lds")?;
    let ld_content = ld_content
        .replace("%ARCH%", arch)
        .replace("%KERNEL_BASE%", &kernel_base_vaddr);
    std::fs::write("linker.ld", ld_content)?;

    Ok(())
}

fn link_app_data(arch: &str) -> Result<()> {
    let app_path = PathBuf::from("../user/build/").join(arch);
    let link_app_path = PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("link_app.S");

    if let Ok(dir) = read_dir(&app_path) {
        let mut apps = dir
            .into_iter()
            .map(|dir_entry| dir_entry.unwrap().file_name().into_string().unwrap())
            .collect::<Vec<_>>();
        apps.sort();

        let mut f = File::create(link_app_path)?;
        writeln!(
            f,
            "
.section .data
.balign 8
.global _app_count
_app_count:
    .quad {}",
            apps.len()
        )?;
        for i in 0..apps.len() {
            writeln!(f, "    .quad app_{}_name", i)?;
            writeln!(f, "    .quad app_{}_start", i)?;
        }
        writeln!(f, "    .quad app_{}_end", apps.len() - 1)?;

        for (idx, app) in apps.iter().enumerate() {
            println!("app_{}: {}", idx, app_path.join(app).display());
            writeln!(
                f,
                "
app_{0}_name:
    .string \"{1}\"
.balign 8
app_{0}_start:
    .incbin \"{2}\"
app_{0}_end:",
                idx,
                app,
                app_path.join(app).display()
            )?;
        }
    } else {
        let mut f = File::create(link_app_path)?;
        writeln!(
            f,
            "
.section .data
.balign 8
.global _app_count
_app_count:
    .quad 0"
        )?;
    }
    Ok(())
}
