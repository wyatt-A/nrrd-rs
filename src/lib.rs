use std::cmp::PartialEq;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::str::FromStr;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn magic() {
        let s = "NRRD000000003\n";
        let magic = Magic::from_str(s).unwrap();
        assert_eq!(magic.version, 3);
    }

    #[test]
    fn header_read() {
        let h = "NRRD0003\nthing1: thing1\nthing2: thing2\n\n thing3: thing3\n";
        let hdr = Header::from_str(h).unwrap();
        println!("{:?}", hdr);
    }

}

#[derive(Debug)]
enum NrrdError {
    NrrdMagic,
    Dimension,
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
}

impl FromStr for Header {
    type Err = NrrdError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {

        let mut magic = None;

        let mut dimension = None;
        let mut type_ = None;
        let mut encoding = None;
        let mut sizes = vec![];
        let mut block_size = None;

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
                    dimension = Some(id.parse::<usize>().map_err(|_| NrrdError::Dimension)?);
                }

                if id.trim() == "sizes" {
                    let d = dimension.ok_or(NrrdError::DimensionAfterSizes)?;
                    sizes.resize(d,0);
                    for (s,a) in sizes.iter_mut().zip(desc.split_whitespace()) {
                        *s = a.parse::<usize>().map_err(|_|NrrdError::ParseSizes)?
                    }
                    if sizes.iter().product() == 0 {
                        Err(NrrdError::ZeroSize)?
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

                lines.push(LineType::Field{id,desc});
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

        Ok(Header {magic, lines, dimension, sizes, type_, encoding, block_size })

    }
}