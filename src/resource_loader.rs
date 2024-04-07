use std::{env, fs::{read_dir, File}, io::{BufWriter, Write}, path::{Path, PathBuf}};

pub fn generate_resources(res_dir: &Path) {
    let res_dir = &workspace_dir().as_path().join(res_dir);
    let path = Path::new(&env::var("OUT_DIR").unwrap()).join("resources.rs");
    let mut file = BufWriter::new(File::create(&path).unwrap());
    let mut resources = phf_codegen::Map::new();
    let mut folders = Vec::new();
    folders.push(res_dir.as_os_str().to_str().unwrap().to_string());
    while let Some(folder) = folders.pop() {
        for path_result in read_dir(folder).unwrap() {
            if let Ok(path) = path_result.map(|entry| entry.path()) {
                if path.is_dir() {
                    folders.push(path.as_path().as_os_str().to_str().unwrap().to_string());
                }
                if path.is_file() {
                    let path_string = path.as_os_str().to_str().unwrap().to_string();
                    let src_relative_path = pathdiff::diff_paths(path_string.clone(), &workspace_dir().as_path().join("src")).unwrap().as_os_str().to_str().unwrap().to_string();
                    let out_relative_path = pathdiff::diff_paths(path_string, env::var("OUT_DIR").unwrap()).unwrap().as_os_str().to_str().unwrap().to_string();
                    resources.entry(src_relative_path.clone(), &format!("include_bytes!(\"{out_relative_path}\")"));
                }
            }
        }
    }
    write!(
        &mut file,
        "static RESOURCES: phf::Map<&'static str, &[u8]> = {}",
            resources
            .build()
    )
    .unwrap();
    write!(&mut file, r#";
pub fn load_resource(path: &str) -> Option<&&[u8]> {{
    println!("loading resource: {{path}}");
    RESOURCES.get(path)
}}

pub fn load_resource_vec(path: &str) -> Option<Vec<u8>> {{
    println!("loading resource: {{path}}");
    RESOURCES.get(path).map(|res| {{ res.to_vec() }})
}}

pub fn load_resource_string(path: &str) -> Option<String> {{
    load_resource(path).map(|res| String::from_utf8(res.to_vec()).ok()).flatten()
}}"#).unwrap();
}

fn workspace_dir() -> PathBuf {
    let output = std::process::Command::new(env!("CARGO"))
        .arg("locate-project")
        .arg("--workspace")
        .arg("--message-format=plain")
        .output()
        .unwrap()
        .stdout;
    let cargo_path = Path::new(std::str::from_utf8(&output).unwrap().trim());
    cargo_path.parent().unwrap().to_path_buf()
}