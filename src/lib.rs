use std::cmp::{min, PartialEq};
use std::collections::HashSet;
use std::env::current_dir;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::fs::File;
use std::io;
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom, Write};
use std::num::ParseIntError;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use bytemuck::Pod;
use bzip2::read::BzDecoder;
use bzip2::write::BzEncoder;
use regex::Regex;
use sprintf::sprintf;
use num_traits::{Euclid, NumCast, ToPrimitive};
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;

#[cfg(test)]
mod tests {
    use std::fs::File;
    use std::io::{BufRead, BufReader, Read};
    use super::*;

    #[test]
    fn magic() {
        let s = "NRRD000000003\n";
        let magic = Magic::from_str(s).unwrap();
        assert_eq!(magic.version, 3);
    }

    #[test]
    fn header_read_list() {
        let mut f = File::open("test_nrrds/detached_list.nhdr").unwrap();
        let mut s = String::new();
        f.read_to_string(&mut s).unwrap();
        let hdr = Header::from_str(&s).unwrap();
        hdr.resolve_data_files();
        let (paths,subdim) = hdr.resolve_data_files().unwrap();
        println!("{:?}", paths);
        println!("{:?}", subdim);
    }

    #[test]
    fn header_read_multi() {
        let mut f = File::open("test_nrrds/detached_multi.nhdr").unwrap();
        let mut s = String::new();
        f.read_to_string(&mut s).unwrap();
        let hdr = Header::from_str(&s).unwrap();
        let (paths,subdim) = hdr.resolve_data_files().unwrap();
        println!("{:?}", paths);
        println!("{:?}", subdim);
    }

    #[test]
    fn header_read_single() {
        let mut f = File::open("test_nrrds/detached_single.nhdr").unwrap();
        let mut s = String::new();
        f.read_to_string(&mut s).unwrap();
        let hdr = Header::from_str(&s).unwrap();
        let (paths,subdim) = hdr.resolve_data_files().unwrap();
        println!("{:?}", paths);
        println!("{:?}", subdim);
    }


    #[test]
    fn read_all() {
        let nrrd = "test_nrrds/detached_single.nhdr";
        let hdr = read_nhdr(nrrd);
        println!("{:?}", hdr);
        println!("{:?}", hdr.resolve_data_files());
    }

    #[test]
    fn read_attached() {
        let nrrd = "B:/ProjectSpace/wa41/test_nrrds/Reg_1500_avg.nhdr";
        let (payload,hdr) = read_nrrd_payload(nrrd);
        println!("{:?}", hdr);
        println!("{:?}", payload.len());

        write_nrrd_payload_detached("B:/ProjectSpace/wa41/test_nrrds/Reg_1500_avg_copy",&hdr,&payload);

    }

}


pub fn read_nhdr(file_path:impl AsRef<Path>) -> Header {
    let mut f = File::open(file_path).unwrap();
    // read the file until the first blank line is encountered
    let (bytes,..) = read_until_blank(&mut f).unwrap();
    // convert bytes to string and parse header
    let hdr_str = String::from_utf8(bytes).unwrap();
    Header::from_str(&hdr_str).unwrap()
}


pub fn read_nrrd_payload(file_path:impl AsRef<Path>) -> (Vec<u8>, Header) {

    let mut f = File::open(&file_path).unwrap();

    // read the file until the first blank line is encountered
    let (bytes,..) = read_until_blank(&mut f).unwrap();
    // convert bytes to string and parse header
    let hdr_str = String::from_utf8(bytes).unwrap();
    let hdr = Header::from_str(&hdr_str).unwrap();

    // determine the total number of bytes we need to extract for the array
    let total_bytes = hdr.sizes.iter().product::<usize>() * hdr.type_.size();

    let mut bytes = vec![0u8; total_bytes];

    let encoding = hdr.encoding;
    let line_skip = hdr.line_skip;
    let read_from_end = hdr.byte_skip < 0;
    let byte_skip = if read_from_end { 0 } else {hdr.byte_skip.abs()} as usize;

    if let Some((detached_files,..)) = hdr.resolve_data_files() {
        let (bytes_per_file,r) = total_bytes.div_rem_euclid(&detached_files.len());
        assert_eq!(r,0,"total bytes not divisible my number of files");
        bytes.chunks_exact_mut(bytes_per_file)
            .zip(detached_files)
            .for_each(|(byte_chunk,file)|{
                let abs_path = if file.is_relative() {
                    file_path.as_ref().with_file_name(file)
                }else {
                    file.to_owned()
                };
                let mut f = File::open(abs_path).unwrap();
                read_bytes(&mut f,encoding,read_from_end,line_skip,byte_skip,byte_chunk);
            });
    }else {
        // attached header condition
        read_bytes(&mut f,encoding,read_from_end,line_skip,byte_skip,&mut bytes);
    }

    (bytes,hdr)
}


pub fn write_nrrd_payload_detached(file_path:impl AsRef<Path>, hdr:&Header, payload:&[u8]) {

    let mut hdr =  hdr.clone();

    let idx = hdr.lines.iter().position(|line|{
        match line {
            LineType::Field { id,.. } => {
                id == "data file" || id == "datafile"
            }
            _ => return false,
        }
    });

    let fname = file_path.as_ref().file_name().unwrap().to_str().unwrap();

    let detached_filename = match hdr.encoding {
        Encoding::raw => Path::new(fname).with_extension("raw"),
        Encoding::rawgz => Path::new(fname).with_extension("raw.gz"),
        Encoding::rawbz2 => Path::new(fname).with_extension("raw.bz2"),
        _=> panic!("not yet implemented"),
    };

    if let Some(idx) = idx {
        hdr.lines.remove(idx);
    }
    hdr.lines.push(LineType::Field {id: "datafile".to_string(),desc:detached_filename.display().to_string() });

    match hdr.encoding {
        Encoding::raw => {
            let mut f = File::create(file_path.as_ref().with_extension("raw")).unwrap();
            write_raw(&mut f,payload)
        }
        Encoding::rawgz => {
            let mut f = File::create(file_path.as_ref().with_extension("raw.gz")).unwrap();
            write_gzip(&mut f,payload)
        }
        Encoding::rawbz2 => {
            let mut f = File::create(file_path.as_ref().with_extension("raw.bz2")).unwrap();
            write_bzip2(&mut f,payload);
        }
        _=> panic!("not yet implemented"),
    }

}

/// helper function to read bytes from a file while accounting for different encodings and compression
pub fn read_bytes(f:&mut File,encoding:Encoding,read_from_end:bool,line_skip:usize,byte_skip:usize,bytes: &mut [u8]) {
    skip_lines(f,line_skip);
    match encoding {
        Encoding::raw => {
            if read_from_end {
                read_tail(f,bytes)
            }else {
                read_raw(f, None, bytes, byte_skip)
            }
        }
        Encoding::rawgz => read_gzip(f, None, bytes, byte_skip),
        Encoding::rawbz2 => read_bzip2(f, None, bytes, byte_skip),
        _=> panic!("text and hex not supported for now"),
    };

}



#[derive(Debug)]
pub enum NrrdError {
    NrrdMagic,
    Dimension,
    DimensionParse,
    DimensionAfterSizes,
    ParseSizes,
    ZeroSize,
    UnknownDType,
    UnknownEncoding,
    BlockSizeParse,
    UnknownBlockSize,
    InvalidType,
    NoBlankLine(String),
    IOError(io::Error),
    ParseLineSkip(ParseIntError),
    ParseByteSkip(ParseIntError),
}

impl Display for NrrdError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Error for NrrdError {}

#[derive(Debug,Clone,Copy)]
struct Magic {
    pub version: u8,
}

impl FromStr for Magic {
    type Err = NrrdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let idx = s.find("NRRD").ok_or(NrrdError::NrrdMagic)?;
        let version = s[idx+4..].trim().parse::<u8>().map_err(|_| NrrdError::NrrdMagic)?;
        Ok(Magic{version})
    }
}

impl Display for Magic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", format!("NRRD000{}", self.version))
    }
}

#[derive(Debug,Clone)]
enum LineType {
    Magic(Magic),
    Field{id:String,desc:String},
    Key{key:String,val:String},
    Comment(String),
}

impl Display for LineType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            LineType::Magic(magic) => write!(f,"{magic}"),
            LineType::Field { id,desc } => write!(f,"{id}: {desc}"),
            LineType::Key { key,val } => write!(f,"{key}:={val}"),
            LineType::Comment(comment) => write!(f,"# {comment}"),
        }
    }
}

#[derive(Debug,Clone,Copy,PartialEq,Eq)]
#[allow(non_camel_case_types)]
enum DType {
    int8,
    uint8,
    int16,
    uint16,
    int32,
    uint32,
    int64,
    uint64,
    f32,
    f64,
    Block,
}

impl DType {
    pub fn size(&self) -> usize {
        match self {
            DType::int8 => size_of::<i8>(),
            DType::uint8 => size_of::<u8>(),
            DType::int16 => size_of::<i16>(),
            DType::uint16 => size_of::<u16>(),
            DType::int32 => size_of::<i32>(),
            DType::uint32 => size_of::<u32>(),
            DType::int64 => size_of::<i64>(),
            DType::uint64 => size_of::<u64>(),
            DType::f32 => size_of::<f32>(),
            DType::f64 => size_of::<f64>(),
            DType::Block => 1, // this needs to be multiplied by block size to be valid
        }
    }
}

impl FromStr for DType {
    type Err = NrrdError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use DType::*;
        let t = match s {
            "signed char" | "int8" | "int8_t" => int8,
            "uchar" | "unsigned char" | "uint8" | "uint8_t" => uint8,
            "short" | "short int" | "signed short" | "signed short int" | "int16" | "int16_t" => int16,
            "ushort" | "unsigned short" | "unsigned short int" | "uint16" | "uint16_t"  => uint16,
            "int" | "signed int" | "int32" | "int32_t" => int32,
            "uint" | "unsigned int" | "uint32" | "uint32_t" => uint32,
            "longlong" | "long long" | "long long int" | "signed long long" | "signed long long int" | "int64" | "int64_t" => int64,
            "ulonglong" | "unsigned long long" | "unsigned long long int" | "uint64" | "uint64_t" => uint64,
            "float" => f32,
            "double" => f64,
            "block" => Block,
            _=> Err(NrrdError::UnknownDType)?
        };
        Ok(t)
    }
}

#[derive(Debug,PartialEq,Eq,Clone,Copy)]
#[allow(non_camel_case_types)]
pub enum Encoding {
    raw,
    txt,
    hex,
    rawgz,
    rawbz2,
}

impl FromStr for Encoding {
    type Err = NrrdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use Encoding::*;
        let e = match s.trim() {
            "raw" => raw,
            "txt" | "text" | "ascii" => txt,
            "gz" | "gzip" => rawgz,
            "bz2" | "bzip2" =>  rawbz2,
            "hex" => hex,
            _=> Err(NrrdError::UnknownEncoding)?
        };
        Ok(e)
    }
}

#[derive(Debug,Clone)]
struct Header {
    pub magic: Magic,
    pub lines: Vec<LineType>,
    pub dimension: usize,
    pub sizes: Vec<usize>,
    pub type_: DType,
    pub encoding: Encoding,
    pub line_skip:usize,
    pub byte_skip:isize,
    pub block_size: Option<usize>,
    pub data_file_pattern: Option<String>,
    pub data_file_list: Vec<PathBuf>,
}

impl Header {


    pub fn resolve_data_files(&self) -> Option<(Vec<PathBuf>,Option<usize>)> {

        let mut paths = vec![];
        let mut subdim = None;

        if let Some(data_file) = &self.data_file_pattern {

            // check if data_file describes a list of files
            if data_file.contains("LIST") {

                let re = Regex::new(r"LIST (\d)").expect("Regex error");
                if let Some(cap) = re.captures(data_file) {
                    subdim = cap.get(1).map(|s| s.as_str().parse::<usize>().unwrap());
                }
                paths = self.data_file_list.clone();
                return Some((paths,subdim))

            }

            // check if data_file describes multiple files with sprintf pattern
            let re = Regex::new(r#"(?:(\S+))\s+(-?\d+)\s+(-?\d+)\s+(-?\d+)(?:\s+(-?\d+))?"#).expect("invalid regex");
            if let Some(capture) = re.captures(data_file) {

                let sprintf_pattern = capture.get(1).unwrap().as_str();
                let min = capture.get(2).unwrap().as_str().parse::<i32>().unwrap();
                let max = capture.get(3).unwrap().as_str().parse::<i32>().unwrap();
                let step = capture.get(4).unwrap().as_str().parse::<i32>().unwrap();
                subdim = capture.get(5).map(|s| s.as_str().parse::<usize>().unwrap());

                if step > 0 {
                    for i in (min.abs()..=max.abs()).step_by(step as usize) {
                        paths.push(
                            PathBuf::from(sprintf!(sprintf_pattern, i).unwrap())
                        )
                    }
                }

                if step < 0 {
                    for i in (max.abs()..=min.abs()).rev().step_by(step.abs() as usize) {
                        paths.push(
                            PathBuf::from(sprintf!(sprintf_pattern, i).unwrap())
                        )
                    }
                }

                return Some((paths,subdim));
            }

            // single file case
            paths.push(PathBuf::from(data_file));
            return Some((paths,subdim));

        }

        // attached header case
        None

    }

}

impl Display for Header {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for line in &self.lines {
            writeln!(f, "{}", line)?;
        }
        writeln!(f,"")?;
        Ok(())
    }
}

impl FromStr for Header {
    type Err = NrrdError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {

        let mut magic = None;

        let mut dimension = None;
        let mut type_ = None;
        let mut encoding = None;
        let mut sizes = vec![];
        let mut data_file_list = vec![];
        let mut block_size = None;
        let mut data_file = None;
        let mut line_skip = None;
        let mut byte_skip = None;

        let mut lines:Vec<LineType> = vec![];

        for raw_line in s.lines() {

            // magic should be the first line in the string
            if magic.is_none() {
                magic = Some(Magic::from_str(raw_line)?);
                lines.push(LineType::Magic(magic.unwrap()));
            }

            // stop reading if an empty line is encountered
            if raw_line.is_empty() {
                break
            }

            if raw_line.starts_with("#"){ // comment
                lines.push(LineType::Comment(raw_line.to_string()));
                continue
            }

            if let Some(idx) = raw_line.find(":="){
                let key = raw_line[0..].to_string();
                let val = raw_line[idx+2..].to_string();
                lines.push(LineType::Key {key,val});
                continue
            }

            if let Some(idx) = raw_line.find(": "){
                let id = raw_line[0..idx].to_string();
                let desc = raw_line[idx+2..].to_string();

                if id.trim() == "dimension" {
                    dimension = Some(desc.parse::<usize>().map_err(|_| NrrdError::DimensionParse)?);
                }

                if id.trim() == "sizes" {
                    let d = dimension.ok_or(NrrdError::DimensionAfterSizes)?;
                    sizes.resize(d,0);
                    for (s,a) in sizes.iter_mut().zip(desc.split_whitespace()) {
                        *s = a.parse::<usize>().map_err(|_|NrrdError::ParseSizes)?
                    }
                }

                if id.trim() == "type" {
                    type_ = Some(DType::from_str(desc.trim())?);
                }

                if id.trim() == "encoding" {
                    encoding = Some(Encoding::from_str(desc.trim())?);
                }

                if id.trim() == "blocksize" || id.trim() == "block size" {
                    block_size = Some(desc.trim().parse::<usize>().map_err(|_| NrrdError::BlockSizeParse)?);
                }

                if id.trim() == "data file" {
                    data_file = Some(desc.trim().to_string());
                }

                if id.trim() == "lineskip" || id.trim() == "line skip" {
                    line_skip = Some(desc.trim().parse::<usize>().map_err(|e| NrrdError::ParseLineSkip(e))?);
                }

                if id.trim() == "byteskip" || id.trim() == "byte skip" {
                    byte_skip = Some(desc.trim().parse::<isize>().map_err(|e| NrrdError::ParseByteSkip(e))?);
                }

                lines.push(LineType::Field{id,desc});
                continue;
            }

            if let Some(data_file) = &data_file {
                if data_file.contains("LIST") {
                    data_file_list.push(
                        PathBuf::from(raw_line.to_string())
                    )
                }
            }

        }

        let magic = magic.ok_or(NrrdError::NrrdMagic)?;
        let dimension = dimension.ok_or(NrrdError::Dimension)?;
        let type_ = type_.ok_or(NrrdError::UnknownDType)?;
        let encoding = encoding.ok_or(NrrdError::UnknownEncoding)?;

        let byte_skip = byte_skip.unwrap_or(0);
        let line_skip = line_skip.unwrap_or(0);

        if type_ == DType::Block && block_size.is_none() {
            Err(NrrdError::UnknownBlockSize)?
        }

        if block_size.is_some() && type_ != DType::Block {
            Err(NrrdError::InvalidType)?
        }

        if sizes.iter().product::<usize>() == 0 || sizes.is_empty() {
            Err(NrrdError::ZeroSize)?
        }

        Ok(Header {magic, lines, dimension, sizes, type_, encoding, line_skip, byte_skip, block_size, data_file_pattern: data_file, data_file_list })

    }
}

// /// read the nrrd or nhdr until a blank line is reached (the end of the header section). This returns
// /// the bytes read and the byte offset to the next byte in the file. If a blank line is not encountered,
// /// None is returned for the byte offset
// fn read_until_blank(file:&mut File) -> io::Result<(Vec<u8>, Option<u64>)> {
//     let mut rdr  = BufReader::new(file);
//     let mut line = Vec::new();   // reused buffer for each line
//     let mut acc  = Vec::new();   // accumulator for all bytes before blank line
//     let mut pos: u64 = 0;        // bytes consumed so far
//     let mut offset_after_blank = None;
//
//     while rdr.read_until(b'\n', &mut line)? != 0 {
//         let is_blank = line == b"\n" || line == b"\r\n";
//         if is_blank {
//             // first byte AFTER the blank line:
//             offset_after_blank = Some(pos + line.len() as u64);
//             break;
//         }
//         acc.extend_from_slice(&line);
//         pos += line.len() as u64;
//         line.clear();
//     }
//
//     Ok((acc, offset_after_blank))
// }

fn read_until_blank(file: &mut File) -> io::Result<(Vec<u8>, Option<u64>)> {
    let start_pos = file.stream_position()?;          // where we began
    let mut rdr  = BufReader::new(file);
    let mut line = Vec::new();
    let mut acc  = Vec::new();
    let mut pos: u64 = 0;
    let mut off_after_blank = None;

    while rdr.read_until(b'\n', &mut line)? != 0 {
        let is_blank = line == b"\n" || line == b"\r\n";
        pos += line.len() as u64;

        if is_blank {
            off_after_blank = Some(pos);              // relative to start_pos
            break;
        }

        acc.extend_from_slice(&line);
        line.clear();
    }

    // Put the underlying File cursor exactly where we want it
    let unread = rdr.buffer().len();
    let file = rdr.into_inner();                      // back to &mut File

    // First, undo the unread buffered bytes (BufReader over-read)
    if unread > 0 {
        file.seek(SeekFrom::Current(-(unread as i64)))?;
    }

    // Then, if we found a blank line, seek to its end; otherwise to EOF we consumed
    if let Some(rel_off) = off_after_blank {
        file.seek(SeekFrom::Start(start_pos + rel_off))?;
    } else {
        file.seek(SeekFrom::Start(start_pos + pos))?;
    }

    Ok((acc, off_after_blank))
}

/// advances the file cursor to the byte just after the nth line
pub fn skip_lines(f: &mut File, n_lines: usize) -> usize {
    let mut rdr = BufReader::new(f);
    let mut buf = Vec::new();
    let mut bytes = 0usize;

    for _ in 0..n_lines {
        buf.clear();
        let n = rdr.read_until(b'\n', &mut buf).expect("failed to read line");
        if n == 0 { break; } // EOF before hitting n_lines
        bytes += n;
    }

    // Rewind by what BufReader buffered but we didn't consume
    let unread = rdr.buffer().len();
    let inner = rdr.into_inner(); // gives us back &mut File
    if unread > 0 {
        inner.seek(SeekFrom::Current(-(unread as i64))).expect("failed to seek");
    }
    bytes
}

pub fn read_tail(f:&mut File, bytes: &mut [u8]) -> usize {

    // 1. how many bytes do we *need* and how many are *there*?
    let file_len = f.metadata().expect("failed to get file metadata").len();
    let want = bytes.len() as u64;
    if want == 0 || file_len == 0 {
        return 0
    }

    let to_read = want.min(file_len);              // never larger than the file
    let offset = -(to_read as i64);                // safe: to_read ≤ file_len ≤ i64::MAX

    // 2. jump to the start of the “tail” segment
    f.seek(SeekFrom::End(offset)).expect("failed to seek backward from EOF");

    // 3. read exactly `to_read` bytes
    f.read_exact(&mut bytes[..to_read as usize]).expect("failed to read file");

    to_read as usize
}


pub fn write_raw(
    f: &mut File,
    payload: &[u8],
) {
    f.write_all(payload).expect("failed to write raw");
}

pub fn read_raw(
    f: &mut File,
    seek_to_raw: Option<u64>,
    bytes: &mut [u8],
    bytes_to_skip: usize,
) -> usize {
    if let Some(seek_to) = seek_to_raw {
        f.seek(SeekFrom::Start(seek_to)).expect("seek to raw compressed data failed");
    }
    read_with_skip(f, bytes, bytes_to_skip)
}


pub fn write_gzip(
    f: &mut File,
    payload: &[u8],
) {
    let mut enc = GzEncoder::new(f,flate2::Compression::fast());
    enc.write_all(payload).expect("failed to write to GZ");
}

pub fn read_gzip(
    f: &mut File,
    seek_to_raw_compressed: Option<u64>,
    decompressed: &mut [u8],
    bytes_to_skip: usize,
) -> usize{
    if let Some(seek_to) = seek_to_raw_compressed {
        f.seek(SeekFrom::Start(seek_to)).expect("seek to raw compressed data failed");
    }
    let mut dec = GzDecoder::new(&mut *f);
    read_with_skip(&mut dec, decompressed, bytes_to_skip)
}

pub fn read_bzip2(
    f: &mut File,
    seek_to_raw_compressed: Option<u64>,
    decompressed: &mut [u8],
    bytes_to_skip: usize,
) -> usize{
    if let Some(seek_to) = seek_to_raw_compressed {
        f.seek(SeekFrom::Start(seek_to)).expect("seek to raw compressed data failed");
    }
    let mut dec = BzDecoder::new(&mut *f);
    read_with_skip(&mut dec, decompressed, bytes_to_skip)
}

pub fn write_bzip2(
    f: &mut File,
    payload: &[u8],
) {
    let mut enc = BzEncoder::new(f,bzip2::Compression::fast());
    enc.write_all(payload).expect("failed to write to BZ");
}

pub fn read_with_skip<R:Read>(reader:&mut R, decompressed: &mut [u8], bytes_to_skip: usize) -> usize {
    // Discard the first `bytes_to_skip` bytes of the stream
    if bytes_to_skip > 0 {
        let mut skipped = 0usize;
        let mut tmp = [0u8; 8 * 1024];
        while skipped < bytes_to_skip {
            let need = min(tmp.len(), bytes_to_skip - skipped);
            let n = reader.read(&mut tmp[..need]).expect("failed to read from reader");
            if n == 0 {
                panic!("reached EOF while skipping")
            }
            skipped += n;
        }
    }

    // Now read into the provided buffer.
    let mut written = 0usize;
    while written < decompressed.len() {
        let n = reader.read(&mut decompressed[written..]).expect("failed to read from reader");
        if n == 0 {
            break; // EOF of decompressed stream
        }
        written += n;
    }

    written

}