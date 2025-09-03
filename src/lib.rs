use std::collections::HashMap;
use std::fmt::{Debug, Display, Formatter};
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use byteorder::{BigEndian, ByteOrder, LittleEndian};
use num_traits::{Euclid, FromPrimitive};

pub mod header_defs;
pub mod io;

use header_defs::{AxisMaxs, AxisMins, BlockSize, ByteSkip, Centerings, Comment, Content, DType, DataFile, Dimension, Encoding, Endian, HeaderDef, Kinds, Labels, LineSkip, Magic, Max, Min, NRRDType, OldMax, OldMin, SampleUnits, Sizes, Space, SpaceDimension, SpaceDirections, SpaceOrigin, SpaceUnits, Spacings, Thicknesses, Units, Value};

#[cfg(test)]
mod tests {
    use std::fs;
    use std::fs::File;
    use crate::header_defs::Encoding;
    use super::*;

    #[test]
    pub fn read_header() {
        let test_header = "test_nrrds/detached_list.nhdr";
        let mut f = File::open(test_header).unwrap();
        let (header_bytes,..) = io::read_until_blank(&mut f).expect("failed to read header");
        let header_str = String::from_utf8(header_bytes).expect("failed to convert bytes to string");
        let mut header_lines = header_str.lines().collect::<Vec<&str>>();
        let _ = NRRD::from_lines_full(&mut header_lines);
        // this means we accounted for every line in the string
        assert!(header_lines.is_empty());
    }

    #[test]
    pub fn resolve_detached() {

        let test_header = "test_nrrds/detached_multi.nhdr";
        let mut f = File::open(test_header).unwrap();
        let (header_bytes,..) = io::read_until_blank(&mut f).expect("failed to read header");
        let header_str = String::from_utf8(header_bytes).expect("failed to convert bytes to string");
        let mut header_lines = header_str.lines().collect::<Vec<&str>>();
        let h = NRRD::from_lines_full(&mut header_lines);

        assert!(header_lines.is_empty());

        println!("{h}");
        let paths = h.data_file.as_ref().unwrap().paths();
        println!("{paths:?}");
    }

    #[test]
    fn literacy_attached_minimal() {

        let attached = true;
        let dims = [2,3,4];
        let n = dims.iter().product::<usize>();
        let data:Vec<_> = (0..n).map(|x| x as f64).collect();
        let nrrd = NRRD::new_from_dims::<f64>(&dims);

        let encodings = [Encoding::raw, Encoding::rawgz, Encoding::rawbz2];

        for encoding in encodings {
            write_nrrd("test_out", &nrrd, &data, attached, encoding);
            let (data_,..) = read_nrrd_to::<i8>("test_out.nrrd");
            let data_ = data_.into_iter().map(|x| x as f64).collect::<Vec<f64>>();
            assert_eq!(data_,data);
            fs::remove_file("test_out.nrrd").unwrap();
        }
    }

    #[test]
    fn literacy_detached_minimal() {

        let attached = false;
        let dims = [2,3,4];
        let n = dims.iter().product::<usize>();
        let data:Vec<_> = (0..n).map(|x| x as f64).collect();
        let nrrd = NRRD::new_from_dims::<f64>(&dims);

        let encodings = [Encoding::raw, Encoding::rawgz, Encoding::rawbz2];

        for encoding in encodings {
            write_nrrd("test_out", &nrrd, &data, attached, encoding);
            let (data_,nrrd) = read_nrrd_to::<i8>("test_out.nhdr");
            let data_ = data_.into_iter().map(|x| x as f64).collect::<Vec<f64>>();
            assert_eq!(data_,data);

            fs::remove_file("test_out.nhdr").unwrap();
            match encoding {
                Encoding::raw => fs::remove_file("test_out.raw").unwrap(),
                Encoding::rawgz => fs::remove_file("test_out.raw.gz").unwrap(),
                Encoding::rawbz2 => fs::remove_file("test_out.raw.bz2").unwrap(),
                _=> {}
            }
        }
    }
}

pub fn read_nrrd_to<T:NRRDType + FromPrimitive>(filepath:impl AsRef<Path>) -> (Vec<T>, NRRD) {

    // read bytes and header from nrrd
    let (bytes,h) = read_payload(filepath);

    let n = h.sizes.n_elements();

    // convert bytes to type T
    let x:Vec<T> = match h.dtype {
        DType::int8 => bytes.into_iter().map(|byte| T::from_i8(byte as i8).unwrap()).collect(),
        DType::uint8 => bytes.into_iter().map(|byte| T::from_u8(byte).unwrap()).collect(),
        DType::int16 => {
            let mut buf = vec![0i16;n];
            match h.endian {
                Endian::Big => BigEndian::read_i16_into(&bytes, &mut buf),
                Endian::Little => LittleEndian::read_i16_into(&bytes, &mut buf),
            }
            buf.into_iter().map(|x| T::from_i16(x).unwrap()).collect()
        }
        DType::uint16 => {
            let mut buf = vec![0u16;n];
            match h.endian {
                Endian::Big => BigEndian::read_u16_into(&bytes, &mut buf),
                Endian::Little => LittleEndian::read_u16_into(&bytes, &mut buf),
            }
            buf.into_iter().map(|x| T::from_u16(x).unwrap()).collect()
        }
        DType::int32 => {
            let mut buf = vec![0i32;n];
            match h.endian {
                Endian::Big => BigEndian::read_i32_into(&bytes, &mut buf),
                Endian::Little => LittleEndian::read_i32_into(&bytes, &mut buf),
            }
            buf.into_iter().map(|x| T::from_i32(x).unwrap()).collect()
        }
        DType::uint32 => {
            let mut buf = vec![0u32;n];
            match h.endian {
                Endian::Big => BigEndian::read_u32_into(&bytes, &mut buf),
                Endian::Little => LittleEndian::read_u32_into(&bytes, &mut buf),
            }
            buf.into_iter().map(|x| T::from_u32(x).unwrap()).collect()
        }
        DType::int64 => {
            let mut buf = vec![0i64;n];
            match h.endian {
                Endian::Big => BigEndian::read_i64_into(&bytes, &mut buf),
                Endian::Little => LittleEndian::read_i64_into(&bytes, &mut buf),
            }
            buf.into_iter().map(|x| T::from_i64(x).unwrap()).collect()
        }
        DType::uint64 => {
            let mut buf = vec![0u64;n];
            match h.endian {
                Endian::Big => BigEndian::read_u64_into(&bytes, &mut buf),
                Endian::Little => LittleEndian::read_u64_into(&bytes, &mut buf),
            }
            buf.into_iter().map(|x| T::from_u64(x).unwrap()).collect()
        }
        DType::f32 => {
            let mut buf = vec![0f32;n];
            match h.endian {
                Endian::Big => BigEndian::read_f32_into(&bytes, &mut buf),
                Endian::Little => LittleEndian::read_f32_into(&bytes, &mut buf),
            }
            buf.into_iter().map(|x| T::from_f32(x).unwrap()).collect()
        }
        DType::f64 => {
            let mut buf = vec![0f64;n];
            match h.endian {
                Endian::Big => BigEndian::read_f64_into(&bytes, &mut buf),
                Endian::Little => LittleEndian::read_f64_into(&bytes, &mut buf),
            }
            buf.into_iter().map(|x| T::from_f64(x).unwrap()).collect()
        }
        DType::block => {
            panic!("cannot read block data into primitive type")
        }
    };
    (x,h)
}

pub fn write_nrrd<T:NRRDType>(filepath:impl AsRef<Path>, ref_header:&NRRD, data:&[T], attached:bool, encoding:Encoding) {

    let mut h = ref_header.clone();

    // insert the data type of the array
    h.dtype = T::dtype();

    // we write in native endianness to avoid overhead of byte swapping
    h.endian = Endian::native();

    // this cast is valid only for native endianness
    let bytes:&[u8] = bytemuck::cast_slice(data);

    // assert that the number of bytes is as expected
    let expected_bytes = h.expected_bytes();
    assert_eq!(bytes.len(),expected_bytes);

    // set the encoding
    h.encoding = encoding;

    // ensure line skip and byte skip are null
    h.byte_skip = None;
    h.line_skip = None;

    if attached {

        h.data_file = None;
        let data_p = filepath.as_ref().with_extension("nrrd");
        let mut f = File::create(data_p).unwrap();
        f.write_all(h.to_string().as_bytes()).unwrap();

        encoding.write_payload(&mut f, bytes);

        // match encoding {
        //     Encoding::raw => io::write_raw(&mut f, bytes),
        //     Encoding::rawgz => io::write_gzip(&mut f, bytes),
        //     Encoding::rawbz2 => io::write_bzip2(&mut f, bytes),
        //     _=> panic!("encoding {} not yet supported",h.encoding)
        // };

    }else {

        let ext = encoding.file_ext();

        let df = Path::new(
            filepath.as_ref().file_name().unwrap().to_str().unwrap()
        ).with_extension(ext);
        h.data_file = Some(DataFile::SingleFile {
            filename: df,
        });
        let data_p = filepath.as_ref().with_extension(ext);
        let header_p = filepath.as_ref().with_extension("nhdr");

        let mut f = File::create(data_p).unwrap();

        encoding.write_payload(&mut f, bytes);

        // match encoding {
        //     Encoding::raw => io::write_raw(&mut f, bytes),
        //     Encoding::rawgz => io::write_gzip(&mut f, bytes),
        //     Encoding::rawbz2 => io::write_bzip2(&mut f, bytes),
        //     _=> panic!("encoding {} not yet supported",h.encoding)
        // };
        let mut f = File::create(header_p).unwrap();
        f.write_all(h.to_string().as_bytes()).unwrap();
    };
}

/// reads only the header of the nhdr or nrrd
pub fn read_header(nrrd:impl AsRef<Path>) -> NRRD {
    let mut f = File::open(nrrd.as_ref()).unwrap();
    let (header_bytes,..) = io::read_until_blank(&mut f).expect("failed to read header");
    let header_str = String::from_utf8(header_bytes).expect("failed to convert bytes to string");
    let mut header_lines = header_str.lines().collect::<Vec<&str>>();
    NRRD::from_lines_full(&mut header_lines)
}

/// reads the nrrd header and all associated data bytes into a single vector
pub fn read_payload(filepath:impl AsRef<Path>) -> (Vec<u8>, NRRD) {

    let mut f = File::open(&filepath).unwrap();
    let (header_bytes,_offset) = io::read_until_blank(&mut f).expect("failed to read header");
    let header_str = String::from_utf8(header_bytes).expect("failed to convert bytes to string");
    let mut header_lines = header_str.lines().collect::<Vec<&str>>();
    let h = NRRD::from_lines_full(&mut header_lines);

    let n_expected_bytes = h.expected_bytes();
    let mut bytes = vec![0u8;n_expected_bytes];
    let line_skip = h.line_skip.as_ref().map(|ls| ls.to_skip()).unwrap_or(0);
    let (byte_skip,read_tail) = h.byte_skip.as_ref().map(|bs| (bs.to_skip(),bs.read_tail())).unwrap_or((0,false));

    if let Some(datafile) = h.data_file.as_ref() {
        // this means the header is detached

        // resolve full paths if necessary
        let resolved_paths = datafile.paths().into_iter().map(|p|{
            if p.is_relative() {
                filepath.as_ref().parent().unwrap().join(p)
            }else {
                p
            }
        }).collect::<Vec<PathBuf>>();

        // check that all exist before attempting to read
        resolved_paths.iter().for_each(|file| {
            if !file.exists() {
                panic!("{} does not exist", file.display());
            }
        });

        let n_files = resolved_paths.len();
        let (bytes_per_file,rem) = n_expected_bytes.div_rem_euclid(&n_files);
        assert_eq!(rem,0,"number of files ({n_files}) doesn't divide total number of bytes evenly ({n_expected_bytes})");

        bytes.chunks_exact_mut(bytes_per_file).zip(&resolved_paths).for_each(|(chunk,file)|{
            let mut f = File::open(file).unwrap();
            io::skip_lines(&mut f, line_skip);
            match h.encoding {
                Encoding::raw => io::read_raw(&mut f, None, chunk, byte_skip),
                Encoding::rawgz => io::read_gzip(&mut f, None, chunk, byte_skip),
                Encoding::rawbz2 => io::read_bzip2(&mut f, None, chunk, byte_skip),
                _=> panic!("unsupported encoding ({}) for now", h.encoding)
            };
        });

        (bytes,h)

    } else {
        // this means the header is attached
        io::skip_lines(&mut f,line_skip);

        match h.encoding {
            Encoding::raw => {
                if read_tail {
                    io::read_tail(&mut f, &mut bytes);
                }else {
                    io::read_raw(&mut f, None, &mut bytes, byte_skip);
                }
                (bytes,h)
            }
            Encoding::rawgz => {
                io::read_gzip(&mut f,None, &mut bytes, byte_skip);
                (bytes,h)
            }
            Encoding::rawbz2 => {
                io::read_bzip2(&mut f,None, &mut bytes, byte_skip);
                (bytes,h)
            }
            _=> panic!("unsupported encoding ({}) for now",h.encoding)
        }

    }

}


#[derive(Debug,Clone)]
pub struct NRRD {

    /* BASIC FIELDS */
    pub magic: Magic,
    pub dimension: Dimension,
    pub dtype: DType,
    pub block_size: Option<BlockSize>,
    pub encoding: Encoding,
    pub endian: Endian,
    pub content: Option<Content>,
    pub min: Option<Min>,
    pub max: Option<Max>,
    pub old_min: Option<OldMin>,
    pub old_max: Option<OldMax>,
    pub data_file: Option<DataFile>,
    pub line_skip: Option<LineSkip>,
    pub byte_skip: Option<ByteSkip>,
    pub sample_units: Option<SampleUnits>,

    /* PER-AXIS FIELDS */
    pub sizes: Sizes,
    pub spacings: Option<Spacings>,
    pub thicknesses: Option<Thicknesses>,
    pub axis_mins: Option<AxisMins>,
    pub axis_maxs: Option<AxisMaxs>,
    pub centerings: Option<Centerings>,
    pub labels: Option<Labels>,
    pub units: Option<Units>,
    pub kinds: Option<Kinds>,

    /* SPACE and ORIENTATION */
    pub space : Option<Space>,
    pub space_dimension: Option<SpaceDimension>,
    pub space_units: Option<SpaceUnits>,
    pub space_origin: Option<SpaceOrigin>,
    pub space_directions: Option<SpaceDirections>,

    /* EXTRA KEY-VALUE DATA */
    pub key_vals: HashMap<String, Value>,

    /* COMMENTS */
    pub comments:Vec<String>,
}


impl NRRD {

    pub fn shape(&self) -> &[usize] {
        self.sizes.shape()
    }

    pub fn new_from_type_dims(t:DType,dims:&[usize]) -> NRRD {
        let mut nhdr = NRRD::new_from_dims::<u8>(dims);
        nhdr.dtype = t;
        nhdr
    }

    pub fn new_from_dims<T:NRRDType>(dims:&[usize]) -> NRRD {

        NRRD {
            magic: Magic::default(),
            dimension: Dimension::new(dims.len()),
            dtype: T::dtype(),
            block_size: None,
            encoding: Encoding::raw,
            endian: Endian::default(),
            content: None,
            min: None,
            max: None,
            old_min: None,
            old_max: None,
            data_file: None,
            line_skip: None,
            byte_skip: None,
            sample_units: None,
            sizes: Sizes::new(dims),
            spacings: None,
            thicknesses: None,
            axis_mins: None,
            axis_maxs: None,
            centerings: None,
            labels: None,
            units: None,
            kinds: None,
            space: None,
            space_dimension: None,
            space_units: None,
            space_origin: None,
            space_directions: None,
            key_vals: Default::default(),
            comments: vec![],
        }


    }

    fn expected_bytes(&self) -> usize {
        self.sizes.n_elements() * self.element_size()
    }

    /// returns the size of each element as determined by 'type' and 'block size' if necessary
    pub fn element_size(&self) -> usize {
        if let DType::block = self.dtype {
            let bs = self.block_size.as_ref().expect("block size must be defined for data type of 'block'");
            bs.size()
        }else {
            self.dtype.size()
        }
    }

    pub fn from_lines_full(lines:&mut Vec<&str>) -> NRRD {

        let mut h = Self::from_lines_minimal(lines);

        h.content = read_header_def(lines);
        h.min = read_header_def(lines);
        h.max = read_header_def(lines);
        h.old_min = read_header_def(lines);
        h.old_max = read_header_def(lines);

        h.line_skip = read_header_def(lines);
        h.byte_skip = read_header_def(lines);
        h.sample_units = read_header_def(lines);

        h.spacings = read_header_def(lines);
        h.thicknesses = read_header_def(lines);
        h.axis_mins = read_header_def(lines);
        h.axis_maxs = read_header_def(lines);
        h.centerings = read_header_def(lines);
        h.labels = read_header_def(lines);
        h.units = read_header_def(lines);
        h.kinds = read_header_def(lines);

        h.space = read_header_def(lines);
        h.space_dimension = read_header_def(lines);
        h.space_units = read_header_def(lines);
        h.space_origin = read_header_def(lines);
        h.space_directions = read_header_def(lines);

        h.key_vals = read_key_values(lines);

        h.comments = read_comments(lines);

        // parse data file last for reasons
        h.data_file = read_data_file(lines);

        h
    }

    /// construct a minimal NHDR from a string
    pub fn from_lines_minimal(lines:&mut Vec<&str>) -> NRRD {

        assert!(!lines.is_empty(),"lines must not be empty");

        let magic:Magic = read_header_def(lines).expect("failed to parse magic field");
        let dimension:Dimension = read_header_def(lines).expect("failed to get dimension field");
        let dtype:DType = read_header_def(lines).expect("failed to get dtype field");

        let block_size:Option<BlockSize> = if dtype == DType::block {
            Some(read_header_def(lines).expect("failed to get block size field"))
        }else {
            None
        };

        let encoding:Encoding = read_header_def(lines).expect("failed to get encoding field");
        let endian:Endian = read_header_def(lines).expect("failed to get endian field");
        let sizes:Sizes = read_header_def(lines).expect("failed to get sizes field");


        NRRD {
            magic,
            dimension,
            dtype,
            block_size,
            encoding,
            endian,
            content: None,
            min: None,
            max: None,
            old_min: None,
            old_max: None,

            line_skip: None,
            byte_skip: None,
            sample_units: None,
            sizes,
            spacings: None,
            thicknesses: None,
            axis_mins: None,
            axis_maxs: None,
            centerings: None,
            labels: None,
            units: None,
            kinds: None,
            space: None,
            space_dimension: None,
            space_units: None,
            space_origin: None,
            space_directions: None,

            key_vals: HashMap::new(),

            comments: vec![],

            data_file: None,
        }

    }
}

impl Display for NRRD {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {

        writeln!(f,"{}",self.magic)?;

        for comment in &self.comments {
            writeln!(f,"{comment}")?;
        }

        writeln!(f,"{}",self.dimension)?;
        writeln!(f,"{}",self.dtype)?;

        if let Some(block_size) = &self.block_size {
            writeln!(f,"{block_size}")?;
        }

        writeln!(f,"{}",self.encoding)?;
        writeln!(f,"{}",self.endian)?;

        if let Some(content) = &self.content {
            writeln!(f,"{content}")?;
        }

        if let Some(min) = &self.min {
            writeln!(f,"{min}")?;
        }

        if let Some(max) = &self.max {
            writeln!(f,"{max}")?;
        }

        if let Some(old_min) = &self.old_min {
            writeln!(f,"{old_min}")?;
        }

        if let Some(old_max) = &self.old_max {
            writeln!(f,"{old_max}")?;
        }

        if let Some(line_skip) = &self.line_skip {
            writeln!(f,"{line_skip}")?;
        }

        if let Some(byte_skip) = &self.byte_skip {
            writeln!(f,"{byte_skip}")?;
        }

        if let Some(sample_units) = &self.sample_units {
            writeln!(f,"{sample_units}")?;
        }

        writeln!(f,"{}",self.sizes)?;

        if let Some(spacings) = &self.spacings {
            writeln!(f,"{spacings}")?;
        }

        if let Some(thicknesses) = &self.thicknesses {
            writeln!(f,"{thicknesses}")?;
        }

        if let Some(axis_mins) = &self.axis_mins {
            writeln!(f,"{axis_mins}")?;
        }

        if let Some(axis_maxs) = &self.axis_maxs {
            writeln!(f,"{axis_maxs}")?;
        }

        if let Some(centerings) = &self.centerings {
            writeln!(f,"{centerings}")?;
        }

        if let Some(labels) = &self.labels {
            writeln!(f,"{labels}")?;
        }

        if let Some(units) = &self.units {
            writeln!(f,"{units}")?;
        }

        if let Some(kinds) = &self.kinds {
            writeln!(f,"{kinds}")?;
        }

        if let Some(space) = &self.space {
            writeln!(f,"{space}")?;
        }

        if let Some(space_dimension) = &self.space_dimension {
            writeln!(f,"{space_dimension}")?;
        }

        if let Some(space_units) = &self.space_units {
            writeln!(f,"{space_units}")?;
        }

        if let Some(space_origin) = &self.space_origin {
            writeln!(f,"{space_origin}")?;
        }

        if let Some(space_directions) = &self.space_directions {
            writeln!(f,"{space_directions}")?;
        }

        let mut keyvals:Vec<(String,Value)> = self.key_vals.iter().map(|(key,value)| (key.clone(),value.clone()) ).collect();
        keyvals.sort_by_key(|(a,_)|a.clone());
        for (key,val) in keyvals {
            writeln!(f,"{key}{val}")?;
        }

        if let Some(datafile) = &self.data_file {
            writeln!(f,"{datafile}")?;
        }

        Ok(())
    }
}


fn read_header_def<T:HeaderDef + FromStr>(header_lines: &mut Vec<&str>) -> Option<T> {
    let found = header_lines.iter().enumerate().find_map(|(i,x)|{
        if T::matches(x) {
            match T::from_str(x) {
                Ok(f) => Some((i,f)),
                Err(_) => panic!("failed to parse header line {x}")
            }
        }else {
            None
        }
    });
    if let Some((idx,field)) = found {
        header_lines.remove(idx);
        return Some(field);
    }
    None
}

fn read_data_file(header_lines: &mut Vec<&str>) -> Option<DataFile> {


    let mut found = header_lines.iter().enumerate().find_map(|(i,x)|{
        if DataFile::matches(x) {
            match DataFile::from_str(x) {
                Ok(f) => Some((i,f)),
                Err(_) => panic!("failed to parse header line {x}")
            }
        }else {
            None
        }
    });

    // insert remaining header lines if the data file spec is a list
    if let Some((idx,df)) = found.as_mut() {
        if let DataFile::List {file_paths: filepaths,.. } = df {
            // the remaining lines must be the files listed out
            //let mut c = 0;
            header_lines[(*idx+1)..].iter().for_each(|line|{
                filepaths.push(PathBuf::from(line));
                //c += 1;
            });

            // pop the data_file line and all files listed
            for _ in 0..header_lines[*idx..].len() {
                header_lines.pop();
            }

        }else {
            header_lines.remove(*idx);
        }
    }
    found.map(|(_,df)| df)
}

fn read_key_values(header_lines: &mut Vec<&str>) -> HashMap<String, Value> {
    let mut keyvals = HashMap::<String,Value>::new();
    header_lines.retain(|x| {
        if Value::matches_key_value(x) {
            let key =Value::key(x);
            let value = Value::from_str(x).expect("failed to parse value");
            keyvals.insert(key, value);
            false
        }else {
            true
        }
    });
    keyvals
}

fn read_comments(header_lines: &mut Vec<&str>) -> Vec<String> {
    let mut comments = Vec::new();
    header_lines.retain(|x| {
        if Comment::matches(x) {
            // from_str will error is comment is empty, so we ignore the line
            if let Ok(comment) = Comment::from_str(x) {
                comments.push(comment.to_string())
            }
            false
        }else {
            true
        }
    });
    comments
}
