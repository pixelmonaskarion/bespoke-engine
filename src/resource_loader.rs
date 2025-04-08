use core::str;
use std::{env, fs::{read_dir, File}, io::{BufWriter, Read, Write}, path::{Path, PathBuf}, sync::Mutex};

use phf_codegen::Map;

pub static GLOBAL_PROJECT_RESOURCES: Mutex<Option<&phf::Map<&'static str, ResourceType>>> = Mutex::new(None);

pub fn load_resource(path: &str) -> Option<Vec<u8>> {
    println!("loading resource: {}", path.replace("\\", "/"));
    let resources = GLOBAL_PROJECT_RESOURCES.lock().unwrap();
    if resources.is_none() { return None };
    let Some(resource) = resources.as_ref().unwrap().get(&path.replace("\\", "/")) else { return None; };
    return match resource {
        ResourceType::Static(bytes) => Some(bytes.to_vec()),
        ResourceType::Dynamic(path) => {
            let mut bytes = vec![];
            File::open(path).unwrap().read_to_end(&mut bytes).unwrap();
            Some(bytes)
        }
    }
}

pub fn load_resource_string(path: &str) -> String {
    String::from_utf8(load_resource(path).unwrap()).unwrap()
}

pub fn generate_resources(res_dir: &Path, dynamic: bool) {
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
                    // let out_relative_path = pathdiff::diff_paths(path_string, env::var("OUT_DIR").unwrap()).unwrap().as_os_str().to_str().unwrap().to_string();
                    if dynamic {
                        resources.entry(src_relative_path.clone().replace("\\", "/"), &format!("bespoke_engine::resource_loader::ResourceType::Dynamic(\"{path_string}\")"));
                    } else {
                        resources.entry(src_relative_path.clone().replace("\\", "/"), &format!("bespoke_engine::resource_loader::ResourceType::Static(include_bytes!(r#\"{path_string}\")\"#)"));
                    }
                }
            }
        }
    }
    buildin_resource(&mut resources, "buildins/culling.wgsl", include_bytes!("culling.wgsl"));
    buildin_resource(&mut resources, "buildins/global_shader_types.wgsl", include_bytes!("global_shader_types.wgsl"));
    buildin_resource(&mut resources, "buildins/screen_renderer.wgsl", include_bytes!("screen_renderer.wgsl"));
    write!(
        &mut file,
"static RESOURCES: phf::Map<&'static str, bespoke_engine::resource_loader::ResourceType> = {};",
            resources
            .build()
    )
    .unwrap();
//     write!(&mut file, r#";
// pub fn load_resource(path: &str) -> Option<&&[u8]> {{
//     println!("loading resource: {{}}", path.replace("\\", "/"));
//     RESOURCES.get(&path.replace("\\", "/"))
// }}

// pub fn load_resource_vec(path: &str) -> Option<Vec<u8>> {{
//     println!("loading resource: {{}}", path.replace("\\", "/"));
//     RESOURCES.get(&path.replace("\\", "/")).map(|res| {{ res.to_vec() }})
// }}

// pub fn load_resource_str(path: &str) -> std::borrow::Cow<str> {{
//     String::from_utf8_lossy(load_resource(path).unwrap())
// }}

// pub fn load_resource_string(path: &str) -> Option<String> {{
//     load_resource(path).map(|res| String::from_utf8(res.to_vec()).ok()).flatten()
// }}
// "#).unwrap();
}

fn buildin_resource(resources: &mut Map<String>, path: &str, bytes: &[u8]) {
    let path_buf = Path::new(&env::var("OUT_DIR").unwrap()).join(path);
    let path_string = path_buf.as_os_str().to_str().unwrap();
    println!("{}", path_string);
    let prefix = path_buf.parent().unwrap();
    std::fs::create_dir_all(prefix).unwrap();
    File::create(&path_buf).unwrap().write(bytes).unwrap();
    resources.entry(path.into(), &format!("bespoke_engine::resource_loader::ResourceType::Static(include_bytes!(r#\"{path_string}\"#))"));
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

pub enum ResourceType {
    Static(&'static [u8]),
    Dynamic(&'static str),
}