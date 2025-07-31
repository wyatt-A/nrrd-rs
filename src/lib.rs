use std::fs::File;
use std::path::Path;
use crate::nrrd::NRRD;
use crate::header_defs::Encoding;
use crate::io::skip_lines;

pub mod header_defs;
pub mod io;
mod nrrd;

#[cfg(test)]
mod tests {



}
