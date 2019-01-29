extern crate byteorder;
extern crate hex;

mod rijndael;
mod assets;
mod archives;

fn main() {
    /*let key = "265e1a5e2741895843d75728b73aeb6a814d3b0302fc69be39bb3f408b9b54e6";
    let archive = archives::PakExtractor::new("pakchunk0-WindowsClient.pak", key);
    let entries = archive.get_entries();
    let file_list = entries.into_iter().map(|v| v.get_filename()).fold(String::new(), |acc, v| acc + v + "\n");
    let mut file = File::create("assets.txt").unwrap();
    file.write_all(file_list.as_bytes()).unwrap();*/

    let package = assets::Package::new("t-featured-glider-teddybear.uasset", "t-featured-glider-teddybear.uexp");
    println!("{:#?}", package);
}
