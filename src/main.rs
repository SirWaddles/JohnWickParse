#![allow(dead_code)]

use std::path::Path;
use std::fs;
use std::io::{Read, Write};
use std::env;

mod dispatch;
mod decompress;
mod mapping;
mod assets;
mod archives;
mod texture;
// mod sound;

use dispatch::{ChunkData, LoaderGlobalData};

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

    let mut dispatch = dispatch::Extractor::new("paks/global", None)?;
    let global_data = dispatch.read_global()?;

    let package = assets::Package::from_file(path, &global_data)?;
    let serial_package = serde_json::to_string(&package).unwrap();
    let mut file = fs::File::create(path.to_owned() + ".json").unwrap();
    file.write_all(serial_package.as_bytes()).unwrap();

    Ok(())
}

fn debug(params: &[String]) -> CommandResult {
    let path = match params.get(0) {
        Some(data) => data,
        None => return cerr("No path specified"),
    };

    let name_map = LoaderGlobalData::empty();
    let package = assets::Package::from_file(path, &name_map)?;
    println!("{:#?}", package);

    Ok(())
}

fn texture(params: &[String]) -> CommandResult {
    let path = match params.get(0) {
        Some(data) => data,
        None => return cerr("No path specified"),
    };

    let mut dispatch = dispatch::Extractor::new("paks/global", None)?;
    let global_data = dispatch.read_global()?;

    let package = assets::Package::from_file(path, &global_data)?;
    let package_export = package.get_export_move(0)?.into_any();
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

/*fn sound(params: &[String]) -> CommandResult {
    let path = match params.get(0) {
        Some(data) => data,
        None => return cerr("No path specified"),
    };

    let name_map = LoaderGlobalData::empty();
    let package = assets::Package::from_file(path, &name_map)?;
    let package_export = package.get_export_move(0)?;
    let sound = match package_export.downcast::<assets::USoundWave>() {
        Ok(data) => data,
        Err(_) => return cerr("Package not exporting sound"),
    };

    let sound_data = sound::decode_sound(*sound)?;

    let save_path = path.clone() + ".ogg";
    let mut file = fs::File::create(save_path).unwrap();
    file.write_all(&sound_data).unwrap();

    Ok(())
}*/

fn dispatch(params: &[String]) -> CommandResult {
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

    let mut dispatch = dispatch::Extractor::new(&path, Some(&key))?;
    let data = dispatch.get_file(pattern)?;

    let filename = pattern.rsplit("/").next().unwrap();

    let mut file = fs::File::create(filename).unwrap();
    file.write_all(&data).unwrap();

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
                            let extension_str = extension.to_str().unwrap();
                            extension_str == "pak" || extension_str == "utoc"
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
        let file_list: Vec<String> = match &path[(path.len() - 4)..] {
            ".pak" => {
                let archive = match archives::PakExtractor::new(&path, &key) {
                    Ok(archive) => archive,
                    Err(_) => continue,
                };
                let entries = archive.get_entries();
                entries.iter().map(|v| v.get_filename().to_owned()).collect()
            },
            "utoc" => {
                let dispatch = dispatch::Extractor::new(&path[..(path.len() - 5)], Some(&key))?;
                dispatch.get_file_list().iter().map(|v| v.to_owned()).collect()
            }
            _ => return cerr("Unrecognised Extension"),
        };
        let file_str = file_list.iter().fold(String::new(), |acc, v| acc + v + "\n");
        let mut file = fs::File::create(path.to_owned() + ".txt").unwrap();
        file.write_all(file_str.as_bytes()).unwrap();
    }

    Ok(())
}

fn idlist(params: &[String]) -> CommandResult {
    let path = match params.get(0) {
        Some(data) => data,
        None => return cerr("No path specified"),
    };
    let key = match std::fs::read_to_string("key.txt") {
        Ok(data) => data,
        Err(_) => return cerr("Could not read key"),
    };
    let dispatch = dispatch::Extractor::new(&path[..(path.len() - 5)], Some(&key))?;

    let file_list: Vec<String> = dispatch.get_chunk_ids().iter().map(|v| v.get_id().to_string()).collect();
    let file_str = file_list.iter().fold(String::new(), |acc, v| acc + v + "\n");
    let mut file = fs::File::create(path.to_owned() + ".txt").unwrap();
    file.write_all(file_str.as_bytes()).unwrap();

    Ok(())
}

fn read_header(params: &[String]) -> CommandResult {
    let path = match params.get(0) {
        Some(data) => data,
        None => return cerr("No path specified"),
    };
    let key = match std::fs::read_to_string("key.txt") {
        Ok(data) => data,
        Err(_) => return cerr("Could not read key"),
    };

    let mut dispatch = dispatch::Extractor::new(&path, Some(&key))?;

    let loader_data = match dispatch.read_chunk(0)? {
        ChunkData::ContainerHeader(data) => data,
        _ => return cerr("Could not find map"),
    };

    println!("{:#?}", loader_data);

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

fn locale(params: &[String]) -> CommandResult {
    let path = match params.get(0) {
        Some(data) => data,
        None => return cerr("No path specified"),
    };

    let mut locres = match fs::File::open(path) {
        Ok(data) => data,
        Err(_) => return cerr("Could not read file"),
    };
    let mut locres_buf = Vec::new();
    locres.read_to_end(&mut locres_buf).unwrap();

    let package = assets::locale::FTextLocalizationResource::from_buffer(&locres_buf)?;
    let serial_package = serde_json::to_string(&package).unwrap();
    let mut file = fs::File::create(path.to_owned() + ".json").unwrap();
    file.write_all(serial_package.as_bytes()).unwrap();

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
        "idlist" => idlist(params),
        "extract" => extract(params),
        "texture" => texture(params),
        "locale" => locale(params),
        "debug" => debug(params),
        //"sound" => sound(params),
        "dispatch" => dispatch(params),
        "read_header" => read_header(params),
        _ => {
            println!("Invalid command");
            Ok(())
        },
    };
    if let Err(error) = err {
        println!("Error: {}", error);
    }
}
