use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("missing manifest dir"));
    println!("cargo:rerun-if-changed={}", manifest_dir.display());

    let mut tools = Vec::new();

    for entry in fs::read_dir(&manifest_dir).expect("read_dir failed") {
        let entry = entry.expect("dir entry failed");
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let cargo_toml = path.join("Cargo.toml");
        if !cargo_toml.exists() {
            continue;
        }

        println!("cargo:rerun-if-changed={}", cargo_toml.display());

        let content = fs::read_to_string(&cargo_toml).expect("read Cargo.toml failed");
        let value: toml::Value = toml::from_str(&content).expect("parse Cargo.toml failed");

        let package = value
            .get("package")
            .and_then(toml::Value::as_table)
            .expect("missing package section");

        let package_name = package
            .get("name")
            .and_then(toml::Value::as_str)
            .expect("missing package.name");

        let display_name = value
            .get("package")
            .and_then(|v| v.get("metadata"))
            .and_then(|v| v.get("play-gui"))
            .and_then(|v| v.get("display-name"))
            .and_then(toml::Value::as_str)
            .unwrap_or(package_name);

        let description = value
            .get("package")
            .and_then(|v| v.get("metadata"))
            .and_then(|v| v.get("play-gui"))
            .and_then(|v| v.get("description"))
            .and_then(toml::Value::as_str)
            .or_else(|| package.get("description").and_then(toml::Value::as_str))
            .unwrap_or("Local desktop tool");

        let relative_dir = path
            .file_name()
            .and_then(|v| v.to_str())
            .expect("invalid directory name");

        tools.push((
            package_name.to_string(),
            display_name.to_string(),
            relative_dir.to_string(),
            description.to_string(),
        ));
    }

    tools.sort_by(|a, b| a.0.cmp(&b.0));

    let mut generated = String::from("pub const DISCOVERED_TOOLS: &[(&str, &str, &str, &str)] = &[\n");
    for (package_name, display_name, relative_dir, description) in tools {
        generated.push_str(&format!(
            "    ({:?}, {:?}, {:?}, {:?}),\n",
            package_name, display_name, relative_dir, description
        ));
    }
    generated.push_str("];\n");

    let output_dir = PathBuf::from(env::var("OUT_DIR").expect("missing OUT_DIR"));
    fs::write(output_dir.join("tool_registry.rs"), generated).expect("write tool registry failed");
}
