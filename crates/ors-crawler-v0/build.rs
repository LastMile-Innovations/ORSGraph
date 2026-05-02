use std::env;
use std::ffi::OsStr;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

fn main() -> io::Result<()> {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let package_cypher_dir = manifest_dir.join("cypher");
    let repo_cypher_dir = manifest_dir.join("../../cypher");
    let cypher_dir = if package_cypher_dir.is_dir() {
        println!("cargo:rerun-if-changed={}", package_cypher_dir.display());
        if repo_cypher_dir.is_dir() {
            println!("cargo:rerun-if-changed={}", repo_cypher_dir.display());
            assert_cypher_dirs_match(&package_cypher_dir, &repo_cypher_dir)?;
        }
        package_cypher_dir
    } else {
        repo_cypher_dir
    };

    println!("cargo:rerun-if-changed={}", cypher_dir.display());

    let mut assets = Vec::new();
    collect_cypher_assets(&cypher_dir, &cypher_dir, &mut assets)?;
    assets.sort_by(|left, right| left.0.cmp(&right.0));

    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let output = out_dir.join("cypher_assets.rs");
    let mut generated =
        String::from("pub fn get(path: &str) -> Option<&'static str> {\n    match path {\n");

    for (asset_path, source_path) in assets {
        let source = source_path
            .canonicalize()
            .unwrap_or(source_path)
            .to_string_lossy()
            .into_owned();
        generated.push_str(&format!(
            "        {:?} => Some(include_str!({:?})),\n",
            asset_path, source
        ));
    }

    generated.push_str("        _ => None,\n    }\n}\n");
    fs::write(output, generated)
}

fn collect_cypher_assets(
    root: &Path,
    dir: &Path,
    assets: &mut Vec<(String, PathBuf)>,
) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            collect_cypher_assets(root, &path, assets)?;
        } else if path.extension() == Some(OsStr::new("cypher")) {
            println!("cargo:rerun-if-changed={}", path.display());
            let relative = path.strip_prefix(root).expect("cypher asset under root");
            let asset_path = Path::new("cypher").join(relative);
            assets.push((asset_path.to_string_lossy().replace('\\', "/"), path));
        }
    }

    Ok(())
}

fn assert_cypher_dirs_match(package_dir: &Path, repo_dir: &Path) -> io::Result<()> {
    let mut package_files = Vec::new();
    let mut repo_files = Vec::new();

    collect_cypher_files(package_dir, package_dir, &mut package_files)?;
    collect_cypher_files(repo_dir, repo_dir, &mut repo_files)?;
    package_files.sort();
    repo_files.sort();

    let package_rel: Vec<&String> = package_files.iter().map(|(relative, _)| relative).collect();
    let repo_rel: Vec<&String> = repo_files.iter().map(|(relative, _)| relative).collect();
    if package_rel != repo_rel {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "crates/ors-crawler-v0/cypher file list does not match repo-root cypher",
        ));
    }

    for ((relative, package_path), (_, repo_path)) in package_files.iter().zip(repo_files.iter()) {
        println!("cargo:rerun-if-changed={}", repo_path.display());
        let package_content = fs::read(package_path)?;
        let repo_content = fs::read(repo_path)?;
        if package_content != repo_content {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "crates/ors-crawler-v0/cypher/{relative} differs from repo-root cypher/{relative}"
                ),
            ));
        }
    }

    Ok(())
}

fn collect_cypher_files(
    root: &Path,
    dir: &Path,
    files: &mut Vec<(String, PathBuf)>,
) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            collect_cypher_files(root, &path, files)?;
        } else if path.extension() == Some(OsStr::new("cypher")) {
            let relative = path.strip_prefix(root).expect("cypher asset under root");
            files.push((relative.to_string_lossy().replace('\\', "/"), path));
        }
    }

    Ok(())
}
