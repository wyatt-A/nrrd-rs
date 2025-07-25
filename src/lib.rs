use std::any::TypeId;
use std::cmp::PartialEq;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::fs::File;
use std::io;
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom};
use std::num::ParseIntError;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use bytemuck::Pod;
use bzip2::bufread::BzDecoder;
use flate2::bufread::GzDecoder;
use regex::Regex;
use sprintf::sprintf;
use num_traits::{Euclid, NumCast, ToPrimitive};

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
    fn read() {

        let nrrd = "test_nrrds/detached_single.nhdr";

        let mut f = File::open(nrrd).unwrap();

        let (bytes,offset) = read_until_blank(&mut f).unwrap();
        let s = String::from_utf8(bytes).unwrap();
        let hdr = Header::from_str(&s);


        println!("{:?}",hdr);
        println!("offset: {:?}",offset);
    }

}

pub struct NrrdReader {
    /// path to header
    file:PathBuf,
    /// header info
    header:Header,
    /// bytes in header
    header_offset:u64,
}

impl NrrdReader {

    pub fn new(file:impl AsRef<Path>) -> Result<NrrdReader,NrrdError> {
        let mut f = File::open(&file).map_err(|e|NrrdError::IOError(e))?;
        let (h_bytes,header_offset) = read_until_blank(&mut f).map_err(|e|NrrdError::IOError(e))?;
        let h_str = String::from_utf8(h_bytes).expect("invalid utf-8");
        if header_offset.is_none() {
            // did not encounter the blank line in nhdr
            Err(NrrdError::NoBlankLine(h_str.clone()))?
        }
        // attempt to parse the header
        let h = Header::from_str(&h_str)?;
        Ok(
            NrrdReader {
                file:file.as_ref().to_path_buf(),
                header:h,
                header_offset: header_offset.unwrap_or(0)
            }
        )
    }

    fn read_bytes(&self) -> Vec<u8> {

        let n_elements: usize = self.header.sizes.iter().product();
        let total_bytes = n_elements * self.header.type_.size();

        let mut bytes = vec![0; total_bytes];

        match self.header.resolve_data_files() {

            None => {
                // this is the attached header case
            }

            Some((files,sub_dim)) => {
                // this is the detached case
                let (bytes_per_file,remainder) = total_bytes.div_rem_euclid(&files.len());
                assert_eq!(remainder,0,"total bytes not divisible by number of files");

                let line_skip = self.header.line_skip;
                let byte_skip = self.header.byte_skip;

                match self.header.encoding {
                    Encoding::raw => {

                        for p in files {
                            let mut f = File::open(p).unwrap();

                        }


                    }
                    Encoding::txt => {}
                    Encoding::hex => {}
                    Encoding::rawgz => {}
                    Encoding::rawbz2 => {}
                }

            }

        }


        let (files,header_offset) = if let Some((files,..)) = self.header.resolve_data_files() {
            (files,0) // assume the header offset is 0 for detached data files
        }else {
            (vec![self.file.clone()],self.header_offset)
        };

        // determine the number of bytes to read
        let n: usize = self.header.sizes.iter().product();
        let total_bytes = self.header.type_.size() * n;

        let (bytes_per_file,remainder) = total_bytes.div_rem_euclid(&files.len());
        assert_eq!(remainder,0);

        let mut bytes = vec![0u8;total_bytes];

        let byte_skip = self.header.byte_skip;

        if byte_skip == -1 {
            // we have to read backward from EOF
            if self.header.encoding != Encoding::raw {
                panic!("byte skip of -1 is only valid for raw encodings")
            }
            // read data from EOF
            bytes.chunks_exact_mut(bytes_per_file).zip(&files).for_each(|(chunk,file)| {
                let bytes_read = read_tail(&file,chunk).expect("failed to read");
                if bytes_read != chunk.len() {
                    panic!("failed to fill buffer from {}",file.display());
                }
            });
            return Ok(bytes)
        }

        // assume byte skip is always positive or 0 if not -1
        let byte_skip = self.header.byte_skip.abs() as usize;
        let line_skip = self.header.line_skip;

        match self.header.encoding {

            Encoding::raw => {
                bytes.chunks_exact_mut(bytes_per_file).zip(&files).for_each(|(chunk,file)| {
                    let bytes_read = read_with_skips(&file,header_offset,line_skip,byte_skip,chunk).expect("failed to read");
                    if bytes_read != chunk.len() {
                        panic!("failed to fill buffer from {}",file.display());
                    }
                });
                Ok(bytes)
            }

            Encoding::rawgz => {
                bytes.chunks_exact_mut(bytes_per_file).zip(&files).for_each(|(chunk,file)| {
                    let bytes_read = read_with_skips_gz(&file,header_offset,line_skip,byte_skip,chunk).expect("failed to read");
                    if bytes_read != chunk.len() {
                        panic!("failed to fill buffer from {}",file.display());
                    }
                });
                Ok(bytes)
            }

            Encoding::rawbz2 => {
                bytes.chunks_exact_mut(bytes_per_file).zip(&files).for_each(|(chunk,file)| {
                    let bytes_read = read_with_skips_gz(&file,header_offset,line_skip,byte_skip,chunk).expect("failed to read");
                    if bytes_read != chunk.len() {
                        panic!("failed to fill buffer from {}",file.display());
                    }
                });
                Ok(bytes)
            }

            _ => panic!("unsupported encoding for now: {:?}",self.header.encoding)
        }

    }


    pub fn read_all<T:ToPrimitive>(&self) -> Result<Vec<T>,NrrdError> {
        todo!()
    }



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

#[derive(Debug)]
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

#[derive(Debug)]
enum LineType {
    Field{id:String,desc:String},
    Key{key:String,val:String},
    Comment(String),
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

#[derive(Debug)]
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

/// read the nrrd or nhdr until a blank line is reached (the end of the header section). This returns
/// the bytes read and the byte offset to the next byte in the file. If a blank line is not encountered,
/// None is returned for the byte offset
fn read_until_blank(file:&mut File) -> io::Result<(Vec<u8>, Option<u64>)> {
    let mut rdr  = BufReader::new(file);
    let mut line = Vec::new();   // reused buffer for each line
    let mut acc  = Vec::new();   // accumulator for all bytes before blank line
    let mut pos: u64 = 0;        // bytes consumed so far
    let mut offset_after_blank = None;

    while rdr.read_until(b'\n', &mut line)? != 0 {
        let is_blank = line == b"\n" || line == b"\r\n";
        if is_blank {
            // first byte AFTER the blank line:
            offset_after_blank = Some(pos + line.len() as u64);
            break;
        }
        acc.extend_from_slice(&line);
        pos += line.len() as u64;
        line.clear();
    }

    Ok((acc, offset_after_blank))
}

pub fn read_tail<P: AsRef<Path>>(path: P, buf: &mut [u8]) -> io::Result<usize> {
    // 1. open the file
    let mut file = File::open(path)?;

    // 2. how many bytes do we *need* and how many are *there*?
    let file_len = file.metadata()?.len(); // u64
    let want = buf.len() as u64;
    if want == 0 || file_len == 0 {
        return Ok(0);
    }

    let to_read = want.min(file_len);              // never larger than the file
    let offset = -(to_read as i64);                // safe: to_read ≤ file_len ≤ i64::MAX

    // 3. jump to the start of the “tail” segment
    file.seek(SeekFrom::End(offset))?;

    // 4. read exactly `to_read` bytes
    file.read_exact(&mut buf[..to_read as usize])?;

    Ok(to_read as usize)
}
