use std::cmp::min;
use std::fs::File;
use std::io;
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom, Write};
use bzip2::read::BzDecoder;
use bzip2::write::BzEncoder;
use flate2::write::{GzDecoder, GzEncoder};

#[cfg(test)]
mod tests {

    #[test]
    fn test() {
        println!("gzip test");
    }

}


pub fn read_until_blank(file: &mut File) -> io::Result<(Vec<u8>, Option<u64>)> {
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
    let mut enc = GzEncoder::new(f, flate2::Compression::default());
    enc.write_all(payload).expect("failed to write to GZ");
    enc.try_finish().unwrap();
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
    enc.try_finish().unwrap();
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