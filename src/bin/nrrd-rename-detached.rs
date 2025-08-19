use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use clap::Parser;
use nrrd_rs::header_defs::DataFile;
use nrrd_rs::NRRD;
use nrrd_rs::io;

#[derive(Parser, Debug)]
struct Args {
    /// path to nhdr to rename
    src_nhdr:PathBuf,
    /// new file name
    dst_nhdr:PathBuf,
}

fn main() {

    let args = Args::parse();

    let src_hdr = args.src_nhdr.with_extension("nhdr");
    let dst_hdr = args.dst_nhdr.with_extension("nhdr");

    let mut f = match File::open(&src_hdr) {
        Ok(f) => f,
        Err(e) => panic!("unable to open nhdr: {} with error {e}", src_hdr.display()),
    };

    let (header_bytes,_offset) = io::read_until_blank(&mut f).expect("failed to read header");
    let header_str = String::from_utf8(header_bytes).expect("failed to convert bytes to string");
    let mut header_lines = header_str.lines().collect::<Vec<&str>>();
    let mut nrrd = NRRD::from_lines_full(&mut header_lines);

    let encoding = nrrd.encoding.to_owned();
    let dst_data_file = dst_hdr.with_extension(encoding.file_ext());

    match nrrd.data_file.as_mut() {
        Some(data_file) => {
            if let DataFile::SingleFile{filename} = data_file {
                *filename = PathBuf::from(dst_data_file.file_name().unwrap());
            }else {
                panic!("only single-file detached nhdrs are supported.")
            }
        }
        None => panic!("data file field not found!")
    }

    // make sure data file exists
    let src_data_file = src_hdr.with_extension(encoding.file_ext());
    if !src_data_file.exists() {
        panic!("detached data file doesn't exist: {}",src_data_file.display());
    }

    fs::rename(&src_data_file, dst_hdr.with_extension(encoding.file_ext()))
        .expect("failed to rename detached data file");

    let mut f = File::create(dst_hdr).expect("failed to create new header file");
    f.write_all(nrrd.to_string().as_bytes()).expect("failed to write to header file");
    fs::remove_file(src_hdr).expect("failed to remove old header file");

}