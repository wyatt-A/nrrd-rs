use std::fmt::{write, Display, Formatter};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use regex::{Regex, RegexSet, SubCaptureMatches};
use sprintf::sprintf;

/// Header Definition
pub trait HeaderDef {
    fn patterns<'a>() -> &'a[&'a str];
    fn matches(s:&str) -> bool {
        // add an anchor to the front of each pattern to avoid duplicate matching
        let pats = Self::patterns().iter().map(|p| format!(r"^{p}"));
        RegexSet::new(pats).unwrap()
            .is_match(s)
    }



    /// return the byte index in 's' of the first character after the pattern match
    fn idx(s:&str) -> Option<usize> {
        for pat in Self::patterns() {
            if let Some(idx) = s.find(pat) {
                return Some(idx + pat.len());
            }
        }
        None
    }

}

/******************************
 ********** MAGIC ************
 ****************************/

#[derive(Debug,Clone,Copy)]
pub struct Magic {
    pub version: u8,
}

impl HeaderDef for Magic {
    fn patterns<'a>() -> &'a [&'a str] {
        &["NRRD"]
    }
}

impl FromStr for Magic {
    type Err = ();

    fn from_str(s: &str) -> Result<Self,()> {
        let idx = Magic::idx(s).unwrap();
        let version = s[idx..].trim().parse::<u8>().unwrap();
        Ok(Magic{version})
    }
}

impl Display for Magic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", format!("NRRD000{}", self.version))
    }
}

/******************************
 ********** Comment *********
 ****************************/
#[derive(Debug,Clone)]
pub struct Comment {
    pub val: String,
}

impl HeaderDef for Comment {
    fn patterns<'a>() -> &'a [&'a str] {
        &["#"]
    }
}

impl FromStr for Comment {
    type Err = ();
    fn from_str(s: &str) -> Result<Self,()> {
        let idx = Comment::idx(s).unwrap();
        // comment starts one character after '#'
        if idx+1 >= s.len() {
            // comment is empty
            Err(())
        }else {
            let val = s[idx+1..].to_string();
            Ok(Comment{val})
        }
    }
}

impl Display for Comment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}",Self::patterns()[0], self.val)
    }
}

/******************************
 ********** KEY-VALUE *********
 ****************************/

#[derive(Debug,Clone)]
pub struct Value {
    pub val: String,
}

impl HeaderDef for Value {
    fn patterns<'a>() -> &'a [&'a str] {
        &[":="]
    }
}

impl Value {
    /// returns true if the header line matches the key-value pattern
    pub fn matches_key_value(s:&str) -> bool {
        let pats = Self::patterns();
        RegexSet::new(pats).unwrap()
            .is_match(s)
    }

    /// extracts the key from the key-value header line
    pub fn key(s:&str) -> String {
        let idx = Value::idx(s).unwrap();
        s[0..idx - ":=".len()].to_string()
    }
}

impl FromStr for Value {
    type Err = ();
    fn from_str(s: &str) -> Result<Self,()> {
        let idx = Value::idx(s).unwrap();
        let val = s[idx..].to_string();
        Ok(Value{val})
    }
}

impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}", Self::patterns()[0], self.val)
    }
}


/******************************
 ************ SPACE ***********
 ****************************/

#[derive(Debug,Clone,Copy,Eq,PartialEq)]
#[allow(non_camel_case_types)]
pub enum Space {
    RAS,
    LAS,
    LPS,
    RAST,
    LAST,
    LPST,
    scanner_xyz,
    scanner_xyz_time,
    _3D_right_handed,
    _3D_left_handed,
    _3D_right_handed_time,
    _3D_left_handed_time,
}

impl HeaderDef for Space {
    fn patterns<'a>() -> &'a [&'a str] {
        &["space: "]
    }
}

impl FromStr for Space {
    type Err = ();
    fn from_str(s: &str) -> Result<Self,()> {
        use Space::*;
        let idx = Space::idx(s).unwrap();
        let s = s[idx..].trim().to_lowercase();
        match s.as_str() {
            "right-anterior-superior" | "ras" => Ok(RAS),
            "left-anterior-superior" | "las" => Ok(LAS),
            "left-posterior-superior" | "lps" => Ok(LPS),
            "right-anterior-superior-time" | "rast" => Ok(RAST),
            "left-anterior-superior-time" | "last" => Ok(LAST),
            "left-posterior-superior-time" | "lpst" => Ok(LPST),
            "scanner-xyz" => Ok(scanner_xyz),
            "scanner-xyz-time" => Ok(scanner_xyz_time),
            "3d-right-handed" => Ok(_3D_right_handed),
            "3d-left-handed" => Ok(_3D_left_handed),
            "3d-right-handed-time" => Ok(_3D_right_handed_time),
            "3d-left-handed-time" => Ok(_3D_left_handed_time),
            _ => panic!("invalid space: {}", s)
        }
    }
}

impl Display for Space {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use Space::*;
        match self {
            RAS => write!(f, "right-anterior-superior"),
            LAS => write!(f, "left-anterior-superior"),
            LPS => write!(f, "left-posterior-superior"),
            RAST => write!(f, "right-anterior-superior-time"),
            LAST => write!(f, "left-anterior-superior-time"),
            LPST => write!(f, "left-posterior-superior-time"),
            scanner_xyz => write!(f, "scanner-xyz" ),
            scanner_xyz_time => write!(f, "scanner-xyz-time" ),
            _3D_right_handed => write!(f, "3D-right-handed" ),
            _3D_left_handed => write!(f, "3D-left-handed" ),
            _3D_right_handed_time => write!(f, "3D-right-handed-time" ),
            _3D_left_handed_time => write!(f, "3D-left-handed-time" ),
        }
    }
}

/******************************
 ***** SPACE DIMENSION ********
 ****************************/

#[derive(Debug,Clone)]
pub struct SpaceDimension {
    dim:usize
}

impl HeaderDef for SpaceDimension {
    fn patterns<'a>() -> &'a [&'a str] {
        &["space dimension: "]
    }
}

impl FromStr for SpaceDimension {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let idx = SpaceDimension::idx(s).unwrap();
        let dim = s[idx..].trim().parse::<usize>().unwrap();
        Ok(SpaceDimension{dim})
    }
}

impl Display for SpaceDimension {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}",Self::patterns()[0],self.dim)
    }
}

/******************************
 *********** SPACE UNITS ***********
 ****************************/

#[derive(Debug,Clone)]
pub struct SpaceUnits {
    units: Vec<String>
}

impl HeaderDef for SpaceUnits {
    fn patterns<'a>() -> &'a [&'a str] {
        &["space units: "]
    }
}

impl FromStr for SpaceUnits {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let idx = SpaceUnits::idx(s).unwrap();
        let s = s[idx..].trim();
        let re = Regex::new(r#""([^"]+)""#).unwrap();
        let units = re.find_iter(s)
            .map(|m| m.as_str()[1..m.as_str().len() - 1].to_string()) // Strip quotes
            .collect();
        Ok(SpaceUnits{units})
    }
}

impl Display for SpaceUnits {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f,"{}{}",
               Self::patterns()[0],
               self.units.iter().map(|x|format!("\"{}\"", x)).collect::<Vec<_>>().join(" ")
        )
    }
}

/******************************
 ********* NRRD VEC **********
 ****************************/

#[derive(Debug,Clone)]
pub struct NrrdVec {
    v: Vec<f64>
}

impl FromStr for NrrdVec {
    type Err = ();
    fn from_str(s: &str) -> Result<Self,Self::Err> {

        let trimmed = s.trim();

        if !(trimmed.starts_with('(') && trimmed.ends_with(')')) {
            panic!("invalid NRD vector: {}", s);
        }

        // Strip outer parens
        let inner = &trimmed[1..trimmed.len() - 1];
        if inner.is_empty() {
            panic!("empty vector entry")
        }

        let v = inner
            .split(',')
            .map(|piece| {
                if piece.is_empty() {
                    panic!("empty vector entry")
                }
                piece
                    .parse::<f64>().expect("failed to parse vector entry to f64")
            })
            .collect();

        Ok(NrrdVec{v})

    }
}

impl Display for NrrdVec {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let s:Vec<_> = self.v.iter().map(|x| format!("{:.17}", x)).collect();
        write!(f,"({})",s.join(","))
    }
}

/******************************
 ********* SPACE ORIGIN *******
 ****************************/

#[derive(Debug,Clone)]
pub struct SpaceOrigin {
    origin: NrrdVec,
}

impl HeaderDef for SpaceOrigin {
    fn patterns<'a>() -> &'a [&'a str] {
        &["space origin: "]
    }
}

impl FromStr for SpaceOrigin {
    type Err = ();
    fn from_str(s: &str) -> Result<Self,Self::Err> {
        let idx = SpaceOrigin::idx(s).unwrap();
        let origin = s[idx..].trim().parse::<NrrdVec>().unwrap();
        Ok(SpaceOrigin{origin})
    }
}

impl Display for SpaceOrigin {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f,"{}{}",Self::patterns()[0],self.origin)
    }
}

/******************************
 ****** SPACE DIRECTIONS ******
 ****************************/

#[derive(Debug,Clone)]
pub struct SpaceDirections {
    directions:Vec<Option<NrrdVec>>,
}

impl HeaderDef for SpaceDirections {
    fn patterns<'a>() -> &'a [&'a str] {
        &["space directions: "]
    }
}

impl FromStr for SpaceDirections {
    type Err = ();
    fn from_str(s: &str) -> Result<Self,Self::Err> {
        let idx = SpaceDirections::idx(s).unwrap();
        let directions = s[idx..].trim().split_ascii_whitespace().map(|x|{
            if x.trim() == "none" {
                None
            }else {
                Some(x.trim().parse::<NrrdVec>().unwrap())
            }
        }).collect();
        Ok(SpaceDirections{directions})
    }
}

impl Display for SpaceDirections {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f,"{}{}",Self::patterns()[0],
            self.directions.iter()
                .map(|x| x.as_ref().map(|x|x.to_string()).unwrap_or("none".to_string()))
                .collect::<Vec<_>>().join(" ")
        )
    }
}

/******************************
 **** MEASUREMENT FRAME ******
 ****************************/
pub struct MeasurementFrame {
    frame_vecs:Vec<NrrdVec>,
}

impl HeaderDef for MeasurementFrame {
    fn patterns<'a>() -> &'a [&'a str] {
        &["measurement frame: "]
    }
}

impl FromStr for MeasurementFrame {
    type Err = ();
    fn from_str(s: &str) -> Result<Self,Self::Err> {
        let idx = MeasurementFrame::idx(s).unwrap();
        let frame_vecs = s[idx..].trim()
            .split_ascii_whitespace()
            .map(|x|x.parse::<NrrdVec>().unwrap())
            .collect();
        Ok(MeasurementFrame{frame_vecs})
    }
}

impl Display for MeasurementFrame {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f,"{}{}",Self::patterns()[0],
               self.frame_vecs.iter()
                   .map(|x| x.to_string())
                   .collect::<Vec<_>>().join(" ")
        )
    }
}

/******************************
 ******** DIMENSION ***********
 ****************************/

#[derive(Debug,Clone)]
pub struct Dimension {
    dim:usize,
}

impl HeaderDef for Dimension {
    fn patterns<'a>() -> &'a [&'a str] {
        &["dimension: "]
    }
}

impl FromStr for Dimension {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let idx = Dimension::idx(s).unwrap();
        let dim = s[idx..].trim().parse::<usize>().unwrap();
        Ok(Dimension {dim})
    }
}

impl Display for Dimension {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}",Self::patterns()[0],self.dim)
    }
}

/******************************
 ************* TYPE **********
 ****************************/

#[derive(Debug,Clone,Copy,PartialEq,Eq)]
#[allow(non_camel_case_types)]
pub enum DType {
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

impl HeaderDef for DType {
    fn patterns<'a>() -> &'a [&'a str] {
        &["type: "]
    }
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
            DType::Block => 1, // placeholder for blocksize
        }
    }
}

impl FromStr for DType {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let idx = DType::idx(s).unwrap();
        use DType::*;
        let t = match s[idx..].trim() {
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
            _=> panic!("unknown data type {}",s)
        };
        Ok(t)
    }
}

impl Display for DType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            DType::int8 => "int8",
            DType::uint8 => "uint8",
            DType::int16 => "int16",
            DType::uint16 => "uint16",
            DType::int32 => "int32",
            DType::uint32 => "uint32",
            DType::int64 => "int64",
            DType::uint64 => "uint64",
            DType::f32 => "float",
            DType::f64 => "double",
            DType::Block => "block",
        };
        write!(f, "{}{s}",Self::patterns()[0])
    }
}

/******************************
 ******* BLOCKSIZE ***********
 ****************************/

#[derive(Debug,Clone)]
pub struct BlockSize {
    bs: usize,
}

impl BlockSize {
    pub fn size(&self) -> usize {
        self.bs
    }
}

impl HeaderDef for BlockSize {
    fn patterns<'a>() -> &'a [&'a str] {
        &["block size: ","blocksize: "]
    }
}

impl FromStr for BlockSize {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let idx = BlockSize::idx(s).unwrap();
        let bs = s[idx..].trim().parse::<usize>().unwrap();
        Ok(BlockSize{bs})
    }
}

impl Display for BlockSize {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}",Self::patterns()[0],self.bs)
    }
}

/******************************
 ******** ENCODING ***********
 ****************************/

#[derive(Debug,PartialEq,Eq,Clone,Copy)]
#[allow(non_camel_case_types)]
pub enum Encoding {
    raw,
    txt,
    hex,
    rawgz,
    rawbz2,
}

impl HeaderDef for Encoding {
    fn patterns<'a>() -> &'a [&'a str] {
        &["encoding: "]
    }
}


impl FromStr for Encoding {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let idx = Encoding::idx(s).unwrap();
        let s = s[idx..].trim().to_ascii_lowercase();
        use Encoding::*;
        let e = match s.as_str() {
            "raw" => raw,
            "txt" | "text" | "ascii" => txt,
            "gz" | "gzip" => rawgz,
            "bz2" | "bzip2" =>  rawbz2,
            "hex" => hex,
            _=> panic!("unknown encoding {}",s)
        };
        Ok(e)
    }
}

impl Display for Encoding {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Encoding::raw => write!(f,"{}raw",Self::patterns()[0]),
            Encoding::txt => write!(f,"{}txt",Self::patterns()[0]),
            Encoding::hex => write!(f,"{}hex",Self::patterns()[0]),
            Encoding::rawgz => write!(f,"{}gzip",Self::patterns()[0]),
            Encoding::rawbz2 => write!(f,"{}bzip2",Self::patterns()[0]),
        }
    }
}

/******************************
 ********** ENDIAN ***********
 ****************************/

#[derive(Debug,PartialEq,Eq,Clone,Copy)]
pub enum Endian {
    Big,
    Little,
}

impl HeaderDef for Endian {
    fn patterns<'a>() -> &'a [&'a str] {
        &["endian: "]
    }
}

impl FromStr for Endian {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let idx = Endian::idx(s).unwrap();
        let s = s[idx..].trim().to_lowercase();
        match s.as_str() {
            "big" => Ok(Endian::Big),
            "little" => Ok(Endian::Little),
            _=> panic!("endianness: {} not recognized", s),
        }
    }
}

impl Display for Endian {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Endian::Big => write!(f,"{}big",Self::patterns()[0]),
            Endian::Little => write!(f,"{}little",Self::patterns()[0]),
        }
    }
}

/******************************
 ********** CONTENT **********
 ****************************/

#[derive(Debug,PartialEq,Eq,Clone)]
pub struct Content {
    content: String,
}

impl HeaderDef for Content {
    fn patterns<'a>() -> &'a [&'a str] {
        &["content: "]
    }
}

impl FromStr for Content {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let idx = Content::idx(s).unwrap();
        let content = s[idx..].to_string();
        Ok(Content { content })
    }
}

impl Display for Content {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}",Self::patterns()[0], self.content)
    }
}

/******************************
 ********** MIN/MAX **********
 ****************************/

#[derive(Debug,PartialEq,Clone,Copy)]
pub struct Min {
    min: f64,
}

impl HeaderDef for Min {
    fn patterns<'a>() -> &'a [&'a str] {
        &["min: "]
    }
}

impl FromStr for Min {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let idx = Min::idx(s).unwrap();
        let min = s[idx..].trim().parse::<f64>().unwrap();
        Ok(Min{min})
    }
}

impl Display for Min {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}",Self::patterns()[0], self.min)
    }
}



#[derive(Debug,PartialEq,Clone,Copy)]
pub struct OldMin {
    min: f64,
}

impl HeaderDef for OldMin {
    fn patterns<'a>() -> &'a [&'a str] {
        &["min: "]
    }
}

impl FromStr for OldMin {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let idx = OldMin::idx(s).unwrap();
        let min = s[idx..].trim().parse::<f64>().unwrap();
        Ok(OldMin{min})
    }
}

impl Display for OldMin {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}",Self::patterns()[0], self.min)
    }
}

#[derive(Debug,PartialEq,Clone,Copy)]
pub struct Max {
    max: f64,
}

impl HeaderDef for Max {
    fn patterns<'a>() -> &'a [&'a str] {
        &["max: "]
    }
}

impl FromStr for Max {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let idx = Max::idx(s).unwrap();
        let max = s[idx..].trim().parse::<f64>().unwrap();
        Ok(Max{max})
    }
}

impl Display for Max {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}", Self::patterns()[0], self.max)
    }
}

#[derive(Debug,PartialEq,Clone,Copy)]
pub struct OldMax {
    max: f64,
}

impl HeaderDef for OldMax {
    fn patterns<'a>() -> &'a [&'a str] {
        &["old max: ","oldmax: "]
    }
}

impl FromStr for OldMax {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let idx = OldMax::idx(s).unwrap();
        let max = s[idx..].trim().parse::<f64>().unwrap();
        Ok(OldMax{max})
    }
}

impl Display for OldMax {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}", Self::patterns()[0],self.max)
    }
}

/******************************
 ********** DATAFILE *********
 ****************************/

#[derive(Debug,Clone)]
pub enum DataFile {
    SingleFile{filename: PathBuf},
    FileFormat{fmt_string: String, min:i32, max:i32, step:i32, sub_dim: Option<usize>},
    List{sub_dim: Option<usize>, file_paths: Vec<PathBuf>},
}

impl DataFile {

    pub fn paths(&self) -> Vec<PathBuf> {

        match &self {
            DataFile::SingleFile { filename } => vec![filename.clone()],
            DataFile::FileFormat { fmt_string, min, max, step, sub_dim } => {
                let mut paths = vec![];
                if *step > 0 {
                    for i in (min.abs()..=max.abs()).step_by(*step as usize) {
                        paths.push(
                            PathBuf::from(sprintf!(fmt_string, i).unwrap())
                        )
                    }
                }
                if *step < 0 {
                    for i in (max.abs()..=min.abs()).rev().step_by(step.abs() as usize) {
                        paths.push(
                            PathBuf::from(sprintf!(fmt_string, i).unwrap())
                        )
                    }
                }
                paths
            }
            DataFile::List { file_paths,.. } => file_paths.clone(),
        }

    }

}

impl HeaderDef for DataFile {
    fn patterns<'a>() -> &'a [&'a str] {
        &["data file: ", "datafile: "]
    }
}

impl FromStr for DataFile {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {

        let idx = DataFile::idx(s).unwrap();
        let s = s[idx..].trim();

        let re = Regex::new(r"(?:(\S+))\s+(-?\d+)\s+(-?\d+)\s+(-?\d+)(?:\s+(-?\d+))?")
            .expect("invalid regex");

        if let Some(capture) = re.captures(s) {
            let fmt_string = capture.get(1).unwrap().as_str().to_string();
            let min = capture.get(2).unwrap().as_str().parse::<i32>().unwrap();
            let max = capture.get(3).unwrap().as_str().parse::<i32>().unwrap();
            let step = capture.get(4).unwrap().as_str().parse::<i32>().unwrap();
            let sub_dim = capture.get(5).map(|s| s.as_str().parse::<usize>().unwrap());
            return Ok(DataFile::FileFormat { fmt_string, min, max, step, sub_dim })
        }

        let re = Regex::new(r"LIST ?(\d)?").expect("invalid regex");
        if let Some(cap) = re.captures(s) {
            let sub_dim = cap.get(1).map(|s| s.as_str().parse::<usize>().unwrap());
            return Ok(DataFile::List{sub_dim, file_paths: vec![]}) // we don't know the files yet
        }
        Ok(DataFile::SingleFile{filename: PathBuf::from(s)})

    }
}

impl Display for DataFile {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self {
            DataFile::SingleFile{filename} => write!(f,"data file: {}", filename.display()),
            DataFile::FileFormat {fmt_string, min, max, step, sub_dim} => {
                if let Some(sub_dim) = sub_dim {
                    write!(f,"{}{fmt_string} {min} {max} {step} {sub_dim}",Self::patterns()[0])
                }else {
                    write!(f,"{}{fmt_string} {min} {max} {step}",Self::patterns()[0])
                }
            }
            DataFile::List{sub_dim, file_paths: filepaths } => {
                let files = filepaths.iter().map(|p| p.display().to_string()).collect::<Vec<String>>();
                if let Some(sub_dim) = sub_dim {
                    if !files.is_empty() {
                        writeln!(f,"{}LIST {sub_dim}",Self::patterns()[0])?;
                        write!(f,"{}",files.join("\n"))
                    }else {
                        write!(f,"{}LIST {sub_dim}",Self::patterns()[0])
                    }
                }else {
                    if !files.is_empty() {
                        writeln!(f,"{}LIST",Self::patterns()[0])?;
                        write!(f,"{}",files.join("\n"))
                    }else {
                        write!(f,"{}LIST",Self::patterns()[0])
                    }
                }
            }
        }
    }
}

/******************************
 ********** LINE SKIP ********
 ****************************/

#[derive(Debug,Clone)]
pub struct LineSkip {
    skip: usize,
}

impl LineSkip {
    pub fn to_skip(&self) -> usize {
        self.skip
    }
}

impl HeaderDef for LineSkip {
    fn patterns<'a>() -> &'a [&'a str] {
        &["line skip: ", "lineskip: "]
    }
}

impl FromStr for LineSkip {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let idx = LineSkip::idx(s).unwrap();
        let skip = s[idx..].trim().parse::<usize>().unwrap();
        Ok(LineSkip{skip})
    }
}

impl Display for LineSkip {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f,"{}{}",Self::patterns()[0], self.skip)
    }
}

/******************************
 ********** BYTE SKIP ********
 ****************************/

#[derive(Debug,PartialEq,Eq,Clone,Copy)]
#[allow(non_camel_case_types)]
pub enum ByteSkip {
    skip(usize),
    rev,
}

impl ByteSkip {
    pub fn to_skip(&self) -> usize {
        match self {
            ByteSkip::skip(skip) => *skip,
            ByteSkip::rev => 0,
        }
    }
    pub fn read_tail(&self) -> bool {
        match self {
            Self::skip(_) => false,
            Self::rev => true,
        }
    }
}

impl HeaderDef for ByteSkip {
    fn patterns<'a>() -> &'a [&'a str] {
        &["byte skip: ", "byteskip: "]
    }
}

impl FromStr for ByteSkip {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let idx = ByteSkip::idx(s).unwrap();
        let skip = s[idx..].trim().parse::<isize>().unwrap();
        if skip < 0 {
            Ok(ByteSkip::rev)
        }else {
            Ok(ByteSkip::skip(skip as usize))
        }
    }
}

impl Display for ByteSkip {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ByteSkip::skip(skip) => write!(f,"{}{}",Self::patterns()[0], skip),
            ByteSkip::rev => write!(f,"{}-1",Self::patterns()[0]),
        }
    }
}

/******************************
 ******** SAMPLE UNITS ********
 ****************************/

#[derive(Debug,Clone)]
pub struct SampleUnits {
    units: String,
}

impl HeaderDef for SampleUnits {
    fn patterns<'a>() -> &'a [&'a str] {
        &["sample units: ", "sampleunits: "]
    }
}

impl FromStr for SampleUnits {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let idx = SampleUnits::idx(s).unwrap();
        Ok(SampleUnits{units: s[idx..].trim().to_string()})
    }
}

impl Display for SampleUnits {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f,"{}{}",Self::patterns()[0], self.units)
    }
}

/******************************
 *********** SIZES ***********
 ****************************/

#[derive(Debug,PartialEq,Eq,Clone)]
pub struct Sizes {
    sizes: Vec<usize>
}

impl Sizes {
    pub fn n_elements(&self) -> usize {
        self.sizes.iter().product()
    }
}

impl HeaderDef for Sizes {
    fn patterns<'a>() -> &'a [&'a str] {
        &["sizes: "]
    }
}

impl FromStr for Sizes {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let idx = Sizes::idx(s).unwrap();
        let s = s[idx..].trim();
        let mut sizes = vec![];
        for size_str in s.split_ascii_whitespace() {
            let size = size_str.parse::<usize>().unwrap();
            assert!(size > 0,"size must be larger than 0");
            sizes.push(size);
        }
        Ok(Sizes{sizes})
    }
}

impl Display for Sizes {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f,"{}{}",
               Self::patterns()[0],
               self.sizes.iter().map(|size| size.to_string()).collect::<Vec<_>>().join(" ")
        )
    }
}

/******************************
 *********** SPACINGS ********
 ****************************/

#[derive(Debug,PartialEq,Clone)]
pub struct Spacings {
    spacings: Vec<f64>
}

impl HeaderDef for Spacings {
    fn patterns<'a>() -> &'a [&'a str] {
        &["spacings: "]
    }
}

impl FromStr for Spacings {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let idx = Spacings::idx(s).unwrap();
        let s = s[idx..].trim();
        let mut spacings = vec![];
        for spacing_str in s.split_ascii_whitespace() {
            let spacing = spacing_str.parse::<f64>().unwrap();
            // nans are allowed, but no Inf, -Inf or 0.
            assert!(!spacing.is_infinite() && spacing != 0.,"infinite or 0 spacings are not valid");
            spacings.push(spacing);
        }
        Ok(Spacings{spacings})
    }
}

impl Display for Spacings {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f,"{}{}",
               Self::patterns()[0],
               self.spacings.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(" ")
        )
    }
}

/******************************
 *********** THICKNESS ********
 ****************************/

#[derive(Debug,PartialEq,Clone)]
pub struct Thicknesses {
    thicknesses: Vec<f64>
}

impl HeaderDef for Thicknesses {
    fn patterns<'a>() -> &'a [&'a str] {
        &["thicknesses: "]
    }
}

impl FromStr for Thicknesses {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let idx = Thicknesses::idx(s).unwrap();
        let s = s[idx..].trim();
        let mut thicknesses = vec![];
        for thickness_str in s.split_ascii_whitespace() {
            let thickness = thickness_str.parse::<f64>().unwrap();
            thicknesses.push(thickness);
        }
        Ok(Thicknesses {thicknesses})
    }
}

impl Display for Thicknesses {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f,"{}{}",
               Self::patterns()[0],
               self.thicknesses.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(" ")
        )
    }
}

/******************************
 *********** AXIS MINS ********
 ****************************/

#[derive(Debug,PartialEq,Clone)]
pub struct AxisMins {
    mins: Vec<f64>
}

impl HeaderDef for AxisMins {
    fn patterns<'a>() -> &'a [&'a str] {
        &["axis mins: ","axismins: "]
    }
}

impl FromStr for AxisMins {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let idx = AxisMins::idx(s).unwrap();
        let s = s[idx..].trim();
        let mut mins = vec![];
        for mins_str in s.split_ascii_whitespace() {
            let min = mins_str.parse::<f64>().unwrap();
            // nans are allowed, but no Inf or -Inf
            assert!(!min.is_infinite(),"infinite min values are not valid");
            mins.push(min);
        }
        Ok(AxisMins{mins})
    }
}

impl Display for AxisMins {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f,"{}{}",
               Self::patterns()[0],
               self.mins.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(" ")
        )
    }
}

/******************************
 *********** AXIS MAX ********
 ****************************/

#[derive(Debug,PartialEq,Clone)]
pub struct AxisMaxs {
    maxs: Vec<f64>
}

impl HeaderDef for AxisMaxs {
    fn patterns<'a>() -> &'a [&'a str] {
        &["axis maxs: ","axismaxs: "]
    }
}

impl FromStr for AxisMaxs {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let idx = AxisMaxs::idx(s).unwrap();
        let s = s[idx..].trim();
        let mut maxs = vec![];
        for maxs_str in s.split_ascii_whitespace() {
            let max = maxs_str.parse::<f64>().unwrap();
            // nans are allowed, but no Inf, -Inf
            assert!(!max.is_infinite(),"infinite max values are not valid");
            maxs.push(max);
        }
        Ok(AxisMaxs{maxs})
    }
}

impl Display for AxisMaxs {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f,"{}{}",
               Self::patterns()[0],
               self.maxs.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(" ")
        )
    }
}


/******************************
 *********** CENTERING *******
 ****************************/

#[derive(Debug,Clone)]
pub enum Centering {
    Cell,
    Node,
    None,
}

#[derive(Debug,Clone)]
pub struct Centerings {
    centerings: Vec<Centering>
}

impl HeaderDef for Centerings {
    fn patterns<'a>() -> &'a [&'a str] {
        &["centerings: ","centers: "]
    }
}

impl Display for Centering {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Centering::Cell => write!(f,"cell"),
            Centering::Node => write!(f,"node"),
            Centering::None => write!(f,"none")
        }
    }
}

impl FromStr for Centerings {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let idx = Centerings::idx(s).unwrap();
        let s = s[idx..].trim();
        let mut centerings = vec![];

        for center in s.split_ascii_whitespace() {
            match center {
                "cell" => centerings.push(Centering::Cell),
                "node" => centerings.push(Centering::Node),
                _=> centerings.push(Centering::None),
            }
        }
        Ok(Centerings{centerings})
    }
}

impl Display for Centerings {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f,"{}{}",
               Self::patterns()[0],
               self.centerings.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(" ")
        )
    }
}

/******************************
 ********** LABELS ***********
 ****************************/

#[derive(Debug,Clone)]
pub struct Labels {
    labels: Vec<String>
}

impl HeaderDef for Labels {
    fn patterns<'a>() -> &'a [&'a str] {
        &["labels: "]
    }
}

impl FromStr for Labels {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let idx = Labels::idx(s).unwrap();
        let s = s[idx..].trim();
        let re = Regex::new(r#""([^"]+)""#).unwrap();
        let labels = re.find_iter(s)
            .map(|m| m.as_str()[1..m.as_str().len() - 1].to_string()) // Strip quotes
            .collect();
        Ok(Labels{labels})
    }
}

impl Display for Labels {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f,"{}{}",
            Self::patterns()[0],
            self.labels.iter().map(|x|format!("\"{}\"", x)).collect::<Vec<_>>().join(" ")
        )
    }
}

/******************************
 *********** UNITS ***********
 ****************************/

#[derive(Debug,Clone)]
pub struct Units {
    units: Vec<String>
}

impl HeaderDef for Units {
    fn patterns<'a>() -> &'a [&'a str] {
        &["units: "]
    }
}

impl FromStr for Units {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let idx = Units::idx(s).unwrap();
        let s = s[idx..].trim();
        let re = Regex::new(r#""([^"]+)""#).unwrap();
        let units = re.find_iter(s)
            .map(|m| m.as_str()[1..m.as_str().len() - 1].to_string()) // Strip quotes
            .collect();
        Ok(Units{units})
    }
}

impl Display for Units {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f,"{}{}",
               Self::patterns()[0],
               self.units.iter().map(|x|format!("\"{}\"", x)).collect::<Vec<_>>().join(" ")
        )
    }
}

/******************************
 *********** KINDS ***********
 ****************************/

#[derive(Debug,Clone)]
pub struct Kinds {
    kinds: Vec<Kind>
}

impl HeaderDef for Kinds {
    fn patterns<'a>() -> &'a [&'a str] {
        &["kinds: "]
    }
}

impl FromStr for Kinds {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let idx = Kinds::idx(s).unwrap();
        let s = s[idx..].trim();
        let mut kinds = vec![];
        for kind_str in s.split_ascii_whitespace() {
            let kind:Kind = kind_str.parse::<Kind>().expect("kind parsing failed");
            kinds.push(kind);
        }
        Ok(Kinds{kinds})
    }
}

impl Display for Kinds {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f,"{}{}",
            Self::patterns()[0],
            self.kinds.iter().map(|x|x.to_string()).collect::<Vec<_>>().join(" ")
        )
    }
}

#[derive(Debug,PartialEq,Clone,Copy,Eq)]
#[allow(non_camel_case_types)]
pub enum Kind {
    domain,
    space,
    time,
    list,
    point,
    vector,
    covariant_vector,
    normal,
    stub,
    scalar,
    complex,
    _2_vector,
    _3_color,
    RGB_color,
    HSV_color,
    XYZ_color,
    _4_color,
    RGBA_color,
    _3_vector,
    _3_gradient,
    _3_normal,
    _4_vector,
    quaternion,
    _2D_symmetric_matrix,
    _2D_masked_symmetric_matrix,
    _2D_matrix,
    _2D_masked_matrix,
    _3D_symmetric_matrix,
    _3D_masked_symmetric_matrix,
    _3D_matrix,
    _3D_masked_matrix,
    none,
}

impl Display for Kind {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {

        use Kind::*;

        match self {
            domain => write!(f,"domain"),
            space => write!(f,"space"),
            time => write!(f,"time"),
            list => write!(f,"list"),
            point => write!(f,"point"),
            vector => write!(f,"vector"),
            covariant_vector => write!(f,"covariant-vector"),
            normal => write!(f,"normal"),
            stub => write!(f,"stub"),
            scalar => write!(f,"scalar"),
            complex => write!(f,"complex"),
            _2_vector => write!(f,"2-vector"),
            _3_color => write!(f,"3-color"),
            RGB_color => write!(f,"RGB-color"),
            HSV_color => write!(f,"HSV-color"),
            XYZ_color => write!(f,"XYZ-color"),
            _4_color => write!(f,"4-color"),
            RGBA_color => write!(f,"RGBA-color"),
            _3_vector => write!(f,"3-vector"),
            _3_gradient => write!(f,"3-gradient"),
            _3_normal => write!(f,"3-normal"),
            _4_vector => write!(f,"4-vector"),
            quaternion => write!(f,"quaternion"),
            _2D_symmetric_matrix => write!(f,"2D-symmetric-matrix"),
            _2D_masked_symmetric_matrix => write!(f,"2D-masked-symmetric-matrix"),
            _2D_matrix => write!(f,"2D-matrix"),
            _2D_masked_matrix => write!(f,"2D-masked-matrix"),
            _3D_symmetric_matrix => write!(f,"3D-symmetric-matrix"),
            _3D_masked_symmetric_matrix => write!(f,"3D-masked-symmetric-matrix"),
            _3D_matrix => write!(f,"3D-matrix"),
            _3D_masked_matrix => write!(f,"3D-masked-matrix"),
            none => write!(f,"none"),
        }
    }
}

impl FromStr for Kind {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use Kind::*;
        match s.trim() {
            "domain" => Ok(domain),
            "space" => Ok(space),
            "time" => Ok(time),
            "list" => Ok(list),
            "point" => Ok(point),
            "vector" => Ok(vector),
            "covariant-vector" => Ok(covariant_vector),
            "normal" => Ok(normal),
            "stub" => Ok(stub),
            "scalar" => Ok(scalar),
            "complex" => Ok(complex),
            "2-vector" => Ok(_2_vector),
            "3-color" => Ok(_3_color),
            "RGB-color" => Ok(RGB_color),
            "HSV-color" => Ok(HSV_color),
            "XYZ-color" => Ok(XYZ_color),
            "4-color" => Ok(_4_color),
            "RGBA-color" => Ok(RGBA_color),
            "3-vector" => Ok(_3_vector),
            "3-gradient" => Ok(_3_gradient),
            "3-normal" => Ok(_3_normal),
            "4-vector" => Ok(_4_vector),
            "quaternion" => Ok(quaternion),
            "2D-symmetric-matrix" => Ok(_2D_symmetric_matrix),
            "2D-masked-symmetric-matrix" => Ok(_2D_masked_symmetric_matrix),
            "2D-matrix" => Ok(_2D_matrix),
            "2D-masked-matrix" => Ok(_2D_masked_matrix),
            "3D-symmetric-matrix" => Ok(_3D_symmetric_matrix),
            "3D-masked-symmetric-matrix" => Ok(_3D_masked_symmetric_matrix),
            "3D-matrix" => Ok(_3D_matrix),
            "3D-masked-matrix" => Ok(_3D_masked_matrix),
            "none" => Ok(none),
            _ => panic!("invalid kind type {}",s),
        }
    }
}