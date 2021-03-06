// Delta encoding algorithm
use std::io::Read;
use std::io::Result as IoResult;
use std::fs::File;
use std::str as Str;

use byteorder::ReadBytesExt;

pub fn patch(source: &[u8], delta: &[u8]) -> Vec<u8> {
    let mut patcher = DeltaPatcher::new(source, delta);
    patcher.run_to_end()
}

pub fn patch_file(source_path: &str, delta_path: &str) -> IoResult<()> {
    let mut source_file = try!(File::open(source_path));
    let mut source_contents = Vec::new();

    let mut delta_file = try!(File::open(delta_path));
    let mut delta_contents = Vec::new();

    try!(source_file.read_to_end(&mut source_contents));
    try!(delta_file.read_to_end(&mut delta_contents));

    let mut patcher = DeltaPatcher::new(&source_contents[..], &delta_contents[..]);
    let res = patcher.run_to_end();
    print!("{}", Str::from_utf8(&res[..]).unwrap());
    Ok(())
}

#[derive(Debug)]
struct DeltaHeader {
    source_len: usize,
    target_len: usize,
    get_offset: usize,
}

impl DeltaHeader {
    fn new(delta: &mut &[u8]) -> DeltaHeader {
        let (source, bytes_s) = DeltaHeader::decode_size(delta);
        let (target, bytes_t) = DeltaHeader::decode_size(delta);

        DeltaHeader {
            source_len: source,
            target_len: target,
            get_offset: bytes_s + bytes_t,
        }
    }

    fn decode_size(delta: &mut &[u8]) -> (usize, usize) {
        let mut byte = 0x80;
        let mut size = 0;
        let mut count = 0;

        while (byte & 0x80) > 0 {
            byte = delta.read_u8().unwrap() as usize;
            size += (byte & 127) << (7 * count);

            count += 1;
        }
        (size, count)
    }
}

#[derive(Debug)]
enum DeltaOp {
    Insert(usize),
    Copy(usize, usize)
}

struct DeltaPatcher<'a> {
    source: &'a [u8],
    delta: &'a [u8],
    target_len: usize,
}

impl<'a> DeltaPatcher<'a> {
    pub fn new(source: &'a [u8], mut delta: &'a [u8]) -> Self {
        let header = DeltaHeader::new(&mut delta);
        assert_eq!(header.source_len, source.len());

        DeltaPatcher {
            source: source,
            delta: delta,
            target_len: header.target_len
        }
    }

    fn run_to_end(&mut self) -> Vec<u8> {
        let target_len = self.target_len;
        let mut buf = Vec::with_capacity(target_len);

        while let Some(command) = self.read_command() {
            self.run_command(command, &mut buf);
        }
        assert_eq!(buf.len(), target_len);
        buf
    }

    fn read_command(&mut self) -> Option<DeltaOp> {
        self.delta.read_u8().ok().map(|cmd| {
            if cmd & 128 > 0 {
                let mut offset = 0usize;
                let mut shift = 0usize;
                let mut length = 0usize;

                // Read the offset to copy from
                for mask in &[0x01, 0x02, 0x04, 0x08] {
                    if cmd & mask > 0 {
                        let byte = self.delta.read_u8().unwrap() as u64;
                        offset += (byte as usize) << shift;
                    }
                    shift += 8;
                }

                // Read the length of the copy
                shift = 0;
                for mask in &[0x10, 0x20, 0x40] {
                    if cmd & mask > 0 {
                        let byte = self.delta.read_u8().unwrap() as u64;
                        length += (byte << shift) as usize;
                    }
                    shift += 8;
                }
                if length == 0 {
                    length = 0x10000;
                }
                DeltaOp::Copy(offset, length)
            } else {
                DeltaOp::Insert(cmd as usize)
            }
        })
    }

    fn run_command(&mut self, command: DeltaOp, buf: &mut Vec<u8>) {
        match command {
            DeltaOp::Copy(start, length) => {
                buf.extend_from_slice(&self.source[start..start + length]);
            },
            DeltaOp::Insert(length) => {
                buf.extend_from_slice(&self.delta[..length]);
                self.delta = &self.delta[length..];
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn delta_patching() {
        patch_file("tests/data/deltas/base1.txt", "tests/data/deltas/delta1").unwrap();
    }
}
