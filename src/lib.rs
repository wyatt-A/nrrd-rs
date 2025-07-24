use std::cmp::PartialEq;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::path::PathBuf;
use std::str::FromStr;
use regex::Regex;
use sprintf::sprintf;

#[cfg(test)]
mod tests {
    use std::fs::File;
    use std::io::Read;
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

#[derive(Debug)]
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
    pub block_size: Option<usize>,
    pub data_file: Option<String>,
    pub data_file_list: Vec<PathBuf>,
}

impl Header {

    pub fn resolve_data_files(&self) -> Option<(Vec<PathBuf>,Option<usize>)> {

        let mut paths = vec![];
        let mut subdim = None;

        if let Some(data_file) = &self.data_file {

            // check if data_file describes multiple files
            if data_file.contains("LIST") {

                let re = Regex::new(r"LIST (\d)").expect("Regex error");
                if let Some(cap) = re.captures(data_file) {
                    subdim = cap.get(1).map(|s| s.as_str().parse::<usize>().unwrap());
                }
                paths = self.data_file_list.clone();
                return Some((paths,subdim))

            }

            // check if data_file describes multiple files
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

            paths.push(PathBuf::from(data_file));
            return Some((paths,subdim));

        }

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

        if type_ == DType::Block && block_size.is_none() {
            Err(NrrdError::UnknownBlockSize)?
        }

        if block_size.is_some() && type_ != DType::Block {
            Err(NrrdError::InvalidType)?
        }

        if sizes.iter().product::<usize>() == 0 || sizes.is_empty() {
            Err(NrrdError::ZeroSize)?
        }

        Ok(Header {magic, lines, dimension, sizes, type_, encoding, block_size, data_file, data_file_list })

    }
}