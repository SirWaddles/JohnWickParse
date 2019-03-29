extern crate byteorder;
extern crate hex;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
#[macro_use]
extern crate erased_serde;
extern crate image;

use std::path::Path;
use std::fs;
use std::io::Write;
use std::env;

mod rijndael;
mod assets;
mod archives;
mod texture;
mod meshes;
mod anims;

#[derive(Debug)]
struct CommandError {
    message: String,
}

impl std::error::Error for CommandError {

}

impl std::fmt::Display for CommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Error: {}", self.message)
    }
}

impl From<assets::ParserError> for CommandError {
    fn from(error: assets::ParserError) -> Self {
        let property_error = error.get_properties().into_iter().rev().fold(String::new(), |acc, v| acc + "\n" + v);
        CommandError {
            message: "Property error occurred: ".to_owned() + &property_error,
        }
    }
}

type CommandResult = Result<(), CommandError>;

fn cerr(message: &'static str) -> CommandResult {
    Err(CommandError {
        message: message.to_owned()
    })
}

fn serialize(params: &[String]) -> CommandResult {
    let path = match params.get(0) {
        Some(data) => data,
        None => return cerr("No path specified"),
    };

    let package = assets::Package::from_file(path)?;
    let serial_package = serde_json::to_string(&package).unwrap();
    let mut file = fs::File::create(path.to_owned() + ".json").unwrap();
    file.write_all(serial_package.as_bytes()).unwrap();

    Ok(())
}

fn texture(params: &[String]) -> CommandResult {
    let path = match params.get(0) {
        Some(data) => data,
        None => return cerr("No path specified"),
    };

    let package = assets::Package::from_file(path)?;
    let package_export = package.get_export_move(0)?;
    let texture = match package_export.downcast::<assets::Texture2D>() {
        Ok(data) => data,
        Err(_) => return cerr("Package not exporting texture"),
    };

    let texture_bytes = texture::decode_texture(*texture)?;

    let save_path = path.clone() + ".png";
    let mut file = fs::File::create(save_path).unwrap();
    file.write_all(&texture_bytes).unwrap();

    Ok(())
}

fn mesh(params: &[String]) -> CommandResult {
    let path = match params.get(0) {
        Some(data) => data,
        None => return cerr("No path specified"),
    };

    let package = assets::Package::from_file(path)?;
    let mesh = meshes::decode_mesh(package, path)?;
    let serial_mesh = serde_json::to_string(&mesh.data).unwrap();
    let mut gltf_file = fs::File::create(path.to_owned() + ".gltf").unwrap();
    gltf_file.write_all(serial_mesh.as_bytes()).unwrap();

    let mut bin_file = fs::File::create(path.to_owned() + ".bin").unwrap();
    bin_file.write_all(&mesh.buffer).unwrap();

    Ok(())
}

fn anim(params: &[String]) -> CommandResult {
    let path = match params.get(0) {
        Some(data) => data,
        None => return cerr("No path specified"),
    };

    let package = assets::Package::from_file(path)?;
    let anim = anims::decode_anim(package, path)?;
    let serial_anim = serde_json::to_string(&anim.data).unwrap();
    let mut gltf_file = fs::File::create(path.to_owned() + ".gltf").unwrap();
    gltf_file.write_all(serial_anim.as_bytes()).unwrap();

    let mut bin_file = fs::File::create(path.to_owned() + ".bin").unwrap();
    bin_file.write_all(&anim.buffer).unwrap();

    Ok(())
}

fn add_anim(params: &[String]) -> CommandResult {
    let path1 = match params.get(0) {
        Some(data) => data,
        None => return cerr("No path specified"),
    };

    let path2 = match params.get(1) {
        Some(data) => data,
        None => return cerr("No path specified"),
    };

    let package1 = assets::Package::from_file(path1)?.get_export_move(0)?;
    let package2 = assets::Package::from_file(path2)?.get_export_move(0)?;
    let mut anim1 = *package1.downcast::<assets::UAnimSequence>().unwrap();
    let anim2 = *package2.downcast::<assets::UAnimSequence>().unwrap();

    anim1.add_tracks(anim2);

    let serial_package = serde_json::to_string(&anim1).unwrap();
    let mut file = fs::File::create(path1.to_owned() + ".merge.json").unwrap();
    file.write_all(serial_package.as_bytes()).unwrap();

    /*let anim = anims::decode_anim_type(anim1, "merged_anim".to_owned())?;
    let serial_anim = serde_json::to_string(&anim.data).unwrap();
    let mut gltf_file = fs::File::create(path1.to_owned() + ".merge.gltf").unwrap();
    gltf_file.write_all(serial_anim.as_bytes()).unwrap();

    let mut bin_file = fs::File::create(path1.to_owned() + ".merge.bin").unwrap();
    bin_file.write_all(&anim.buffer).unwrap();*/

    Ok(())
}

fn filelist(params: &[String]) -> CommandResult {
    let path = match params.get(0) {
        Some(data) => data,
        None => return cerr("No path specified"),
    };
    let key = match std::fs::read_to_string("key.txt") {
        Ok(data) => data,
        Err(_) => return cerr("Could not read key"),
    };

    let path_dir = Path::new(path);
    let paths = match path_dir.is_dir() {
        true => {
            path_dir.read_dir().unwrap().filter(|v| {
                if let Ok(filepath) = v {
                    return match filepath.path().extension() {
                        Some(extension) => {
                            extension.to_str().unwrap() == "pak"
                        },
                        None => false,
                    };
                }
                false
            }).map(|v| path.clone() + v.unwrap().file_name().to_str().unwrap()).collect()
        },
        false => {
            vec![path.clone()]
        },
    };

    for path in paths {
        let archive = match archives::PakExtractor::new(&path, &key) {
            Ok(archive) => archive,
            Err(_) => continue,
        };
        let entries = archive.get_entries();
        let file_list = entries.into_iter().map(|v| v.get_filename()).fold(String::new(), |acc, v| acc + v + "\n");
        let mut file = fs::File::create(path.to_owned() + ".txt").unwrap();
        file.write_all(file_list.as_bytes()).unwrap();
    }

    Ok(())
}

fn extract(params: &[String]) -> CommandResult {
    let path = match params.get(0) {
        Some(data) => data,
        None => return cerr("No path specified"),
    };
    let key = match std::fs::read_to_string("key.txt") {
        Ok(data) => data,
        Err(_) => return cerr("Could not read key"),
    };
    let pattern = match params.get(1) {
        Some(data) => data,
        None => return cerr("No pattern specified"),
    };

    let mut archive = archives::PakExtractor::new(path, &key)?;
    let entries: Vec<archives::FPakEntry> = archive.get_entries().into_iter().filter(|v| v.get_filename().contains(pattern)).cloned().collect();

    for asset in entries {
        let file_contents = archive.get_file(&asset);
        let path = Path::new(asset.get_filename());
        if let Some(basename) = path.parent() {
            fs::create_dir_all(basename).expect("Could not create directory");
        }
        let mut file = fs::File::create(asset.get_filename()).unwrap();
        file.write_all(&file_contents).unwrap();
    }

    Ok(())
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let command = args.get(1);
    let command = match command {
        Some(data) => data,
        None => {
            println!("No command specified");
            return
        }
    };
    let params = &args[2..];

    let err = match (*command).as_ref() {
        "serialize" => serialize(params),
        "filelist" => filelist(params),
        "extract" => extract(params),
        "texture" => texture(params),
        "mesh" => mesh(params),
        "anim" => anim(params),
        "add_anim" => add_anim(params),
        _ => {
            println!("Invalid command");
            Ok(())
        },
    };
    if let Err(error) = err {
        println!("{}", error);
    }
}
