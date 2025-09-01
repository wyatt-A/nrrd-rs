use std::error::Error;
use std::fmt::Display;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::process::exit;
use std::str::FromStr;
use clap::Parser;
use nrrd_rs::NRRD;
use nrrd_rs::header_defs::{DType, DataFile, Encoding, Endian, Kind, Kinds, SpaceDimension, SpaceDirections, SpaceOrigin, SpaceUnits};

#[derive(Parser, Debug)]
struct Args {

    /// output nhdr file path
    nhdr:PathBuf,

    /// data type of .raw files.
    /// Example: `-d ushort` Data type specifiers are defined in the NRRD docs
    #[clap(short,long)]
    dtype: String,

    /// endianness of .raw files.
    /// Example: `-e big` or `-e little`
    #[clap(short,long)]
    endianness: String,

    /// image dimensions for each .raw file.
    /// Example: `--file-dims [128 128]`
    #[clap(long)]
    file_dims: String,

    /// format string to define the collection of files.
    /// Example: `--fmt-str /Volumes/data/img_slice_%03i.raw`
    #[clap(long)]
    fmt_str: String,

    /// number of raw files to read. This dimension will be appended to the ones provided.
    /// Example: `-n 360`
    #[clap(short,long)]
    n_raw_files:usize,

    /// starting index for fmt string.
    /// Example: `-s 1` will produce a range 1...=n. Default is 0
    #[clap(long)]
    start_idx:Option<usize>,

    /// the step size of for iterating through image index
    /// Example: `-s 2` will produce a range 0,2,4,6 ... n
    #[clap(long)]
    step:Option<usize>,

    /// specify the sample spacing in units of microns
    /// Example: `--spacing-um [30 30 30]`
    #[clap(long)]
    spacing_um: Option<String>,

    /// specify the sample spacing in units of millimeters
    /// Example: `--spacing-mm [0.03 0.03 0.03]`
    #[clap(long)]
    spacing_mm: Option<String>,

    /// center the origin of nhdr
    #[clap(short,long)]
    center_origin: bool
}

#[derive(Parser, Debug)]
/// Args for build a complex-valued nhdr
pub struct BuildArgs {

    /// file to save the header to
    output_nhdr:PathBuf,

    #[clap(long)]
    dims:String,

    /// data type of .raw files.
    /// Example: `-d ushort` Data type specifiers are defined in the NRRD docs
    #[clap(short,long)]
    dtype: String,

    #[clap(short,long)]
    /// specify the encoding of the image data.
    /// Example: `-e rawgz` or `-e rawbz2`, default is `raw`
    encoding: Option<String>,

    /// data is complex-valued. This adds a non-spatial dimension of 2 labeled as "complex" with the
    /// "kinds" field
    #[clap(short,long)]
    complex: bool,

    /// endianness of raw files.
    /// Example: `--endianness big` or `--endianness little`
    #[clap(long)]
    endianness: Option<String>,

    #[clap(short,long)]
    /// file path to raw data
    /// Example: `-f /my/special/image.raw`
    file:Option<PathBuf>,

    #[clap(long)]
    /// file 'sprintf' format string to read multiple files. You must also specify the number of
    /// files to read, and optionally, the start index and step size
    /// Example: `--file-fmt /my/special/image_%03d.raw`
    file_fmt:Option<String>,

    /// number of raw files to read. This dimension will be appended to the ones provided.
    /// Example: `-n 360`
    #[clap(short,long)]
    n_raw_files:Option<usize>,

    /// starting index for fmt string.
    /// Example: `-s 1` will produce a range 1...=n. Default is 0
    #[clap(long)]
    start_idx:Option<usize>,

    /// the step size of for iterating through image index
    /// Example: `-s 2` will produce a range 0,2,4,6 ... n
    #[clap(long)]
    step:Option<usize>,

    /// specify the voxel size for the spatial dimensions in mm. If none is given, voxel size is
    /// assumed to be 1mm isotropic
    #[clap(long)]
    vox_size_mm:Option<String>,

}

fn build_nrrd(args: &BuildArgs) -> Result<NRRD,String> {

    let dtype = DType::new(&args.dtype);
    let mut dims = parse_list_input::<usize>(&args.dims,'[',']')?;

    // if this is complex data, prepend a dimension of 2 for re and imag components
    if args.complex {
        dims.insert(0,2);
    }

    let mut nrrd = NRRD::new_from_type_dims(dtype,&dims);

    nrrd.endian = args.endianness.as_ref().map(|e|{
        match e.to_lowercase().as_str() {
            "big" => Endian::Big,
            "little" => Endian::Little,
            _=> Endian::native()
        }
    }).unwrap_or(Endian::native());

    let data_file = if let Some(file) = &args.file {
        DataFile::SingleFile {filename: file.to_path_buf()}
    }else if let Some(file_fmt) = &args.file_fmt {
        let n_files = args.n_raw_files.ok_or("number of raw file must be specified".to_string())?;
        let min = args.start_idx.unwrap_or(0);
        let max = min + n_files - 1;
        let step = args.step.unwrap_or(1);
        DataFile::FileFormat {
            fmt_string: file_fmt.to_string(),
            min: min as i32,
            max: max as i32,
            step: step as i32,
            sub_dim: None,
        }
    }else {
        return Err("a single file or sprintf file format must be specified".to_string());
    };

    nrrd.data_file = Some(data_file);

    let mut k = if args.complex {
        vec![Kind::complex]
    }else {
        vec![]
    };

    k.extend_from_slice(&vec![Kind::domain;dims.len()]);
    let kinds = Kinds::from_vec(
        k
    );

    nrrd.kinds = Some(kinds);
    nrrd.space_dimension = Some(SpaceDimension::new(dims.len()));

    // handle space directions for complex data. If vox spacing is not given, default to 1mm
    let mut sd = SpaceDirections::new();
    sd.extend_none();
    if let Some(vox_size_str) = &args.vox_size_mm {
        let vox_size = parse_list_input::<f64>(vox_size_str, '[', ']')?;
        sd.extend_from_spacing(&vox_size);
    }else {
        sd.extend_from_spacing(&vec![1.; dims.len()]);
    }
    nrrd.space_directions = Some(sd);
    nrrd.space_units = Some(SpaceUnits::new_mm(dims.len()));

    Ok(nrrd)

}



fn main() {
    let args = BuildArgs::parse();

    match build_nrrd(&args) {
        Ok(nrrd) => {
            let mut f = File::create(&args.output_nhdr.with_extension("nhdr")).unwrap_or_else(|e|{
                panic!("failed to create file {} with error: {}",&args.output_nhdr.display(),e)
            });
            f.write_all(nrrd.to_string().as_bytes()).unwrap_or_else(|e|{
                panic!("failed to write to file {} with error: {}",&args.output_nhdr.display(),e)
            });
        }
        Err(err) => {
            eprintln!("{}",err);
            exit(1);
        }
    }

}










// fn main() {
//
//     let args = Args::parse();
//
//     let dtype = DType::new(&args.dtype);
//
//     let start_idx = args.start_idx.unwrap_or(0);
//     let end_idx = start_idx + args.n_raw_files - 1;
//
//     let data_file = DataFile::FileFormat {
//         fmt_string: args.fmt_str.clone(),
//         min: start_idx as i32,
//         max: end_idx as i32,
//         step: args.step.unwrap_or(1) as i32,
//         sub_dim: None,
//     };
//
//     let mut dims = parse_list_input::<usize>(&args.file_dims,'[',']').unwrap();
//     dims.push(args.n_raw_files);
//
//     let space_info = if let Some(spacing_um) = &args.spacing_um {
//         let mut spacing = parse_list_input::<f64>(spacing_um,'[',']').unwrap();
//         spacing.iter_mut().for_each(|s| *s *= 1e-3); // convert to mm
//         let fov_mm:Vec<f64> = spacing.iter().zip(&dims).map(|(s,&d)| s * d as f64).collect();
//         Some((
//             SpaceDirections::from_spacing(&spacing),
//             SpaceUnits::new_mm(spacing.len()),
//             fov_mm
//         ))
//     }else if let Some(spacing_mm) = &args.spacing_mm {
//         let spacing_mm = parse_list_input::<f64>(spacing_mm,'[',']').unwrap();
//         let fov_mm:Vec<f64> = spacing_mm.iter().zip(&dims).map(|(s,&d)| s * d as f64).collect();
//         Some((
//             SpaceDirections::from_spacing(&spacing_mm),
//             SpaceUnits::new_mm(spacing_mm.len()),
//             fov_mm
//         ))
//     }else {
//         None
//     };
//
//     let mut nrrd = NRRD::new_from_type_dims(dtype,dims.as_slice());
//     nrrd.data_file = Some(data_file);
//
//     match args.endianness.to_lowercase().as_str() {
//         "little" => nrrd.endian = Endian::Little,
//         "big" => nrrd.endian = Endian::Big,
//         _ => panic!("Invalid endianness {}",args.endianness),
//     };
//
//     if let Some(spacing) = space_info {
//         let space_dim = spacing.0.len();
//         nrrd.space_directions = Some(spacing.0);
//         nrrd.space_units = Some(spacing.1);
//         nrrd.space_dimension = Some(SpaceDimension::new(space_dim));
//         nrrd.kinds = Some(Kinds::new(Kind::domain,space_dim));
//         let orig = if args.center_origin {
//             let center_mm:Vec<f64> = spacing.2.iter().map(|fov| -fov/2.).collect();
//             SpaceOrigin::new(&center_mm)
//         }else {
//             SpaceOrigin::new(&vec![0.;space_dim])
//         };
//         nrrd.space_origin = Some(orig)
//     }
//
//     let mut f = File::create(&args.nhdr).unwrap_or_else(|e|{
//         panic!("failed to create file {} with error: {}",&args.nhdr.display(),e)
//     });
//
//     f.write_all(nrrd.to_string().as_bytes()).unwrap_or_else(|e|{
//         panic!("failed to write to file {} with error: {}",&args.nhdr.display(),e)
//     });
//
// }

pub fn parse_list_input<T>(s: &str, open: char, close: char) -> Result<Vec<T>, String>
where
    T: FromStr,
    <T as FromStr>::Err: Display,
{
    let start = s.find(open).ok_or_else(|| format!("missing open delimiter '{}'", open))?;
    let end_rel = s[start + 1..]
        .find(close)
        .ok_or_else(|| format!("missing close delimiter '{}'", close))?;
    let end = start + 1 + end_rel; // make it absolute

    let inner = &s[start + 1..end];

    let vals = inner
        .split(|c: char| c.is_ascii_whitespace() || c == ',')
        .filter(|t| !t.is_empty())
        .map(|t| t.parse::<T>().map_err(|e| format!("failed to parse '{}': {e}", t)))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(vals)
}