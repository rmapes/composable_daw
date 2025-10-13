use std::{collections::HashMap, path::Path};

fn main() {
    let manifest_dir = std::env::var_os("CARGO_MANIFEST_DIR").unwrap();
    let material_path = Path::new(&manifest_dir).join("material-1.0/material.slint");

    let config = slint_build::CompilerConfiguration::new().with_library_paths(HashMap::from([(
        "material".to_string(),
        material_path,
    )]));
    slint_build::compile_with_config("ui/main.slint", config).expect("Slint build failed");
}