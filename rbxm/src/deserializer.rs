use std::{
    io::{self, Read},
    collections::HashMap,
    marker::PhantomData,
    borrow::Cow,
    fmt,
    str,
};

use byteorder::{ReadBytesExt, LittleEndian};
use rbx_tree::{RbxTree, RbxId};

static FILE_MAGIC_HEADER: &[u8] = b"<roblox!\x89\xff\x0d\x0a\x1a\x0a\x00\x00";

#[derive(Debug)]
pub enum DecodeError {
    MissingMagicFileHeader,
    IoError(io::Error),
}

impl From<io::Error> for DecodeError {
    fn from(error: io::Error) -> DecodeError {
        DecodeError::IoError(error)
    }
}

/// Decodes source from the given buffer into the instance in the given tree.
///
/// Roblox model files can contain multiple instances at the top level. This
/// happens in the case of places as well as Studio users choosing multiple
/// objects when saving a model file.
pub fn decode<R: Read>(tree: &mut RbxTree, parent_id: RbxId, mut source: R) -> Result<(), DecodeError> {
    let header = decode_file_header(&mut source)?;
    let mut buffer = Vec::new();

    loop {
        let header = decode_chunk(&mut source, &mut buffer)?;

        // TODO: decode specific chunk

        buffer.clear();

        if &header.name == b"END\0" {
            break;
        }
    }

    Ok(())
}

struct FileHeader {
    num_instance_types: u32,
    num_instances: u32,
}

fn decode_file_header<R: Read>(mut source: R) -> Result<FileHeader, DecodeError> {
    let mut magic_header = [0; 16];
    source.read_exact(&mut magic_header)?;

    if &magic_header != FILE_MAGIC_HEADER {
        assert_eq!(&magic_header, FILE_MAGIC_HEADER);
        return Err(DecodeError::MissingMagicFileHeader);
    }

    let num_instance_types = source.read_u32::<LittleEndian>()?;
    let num_instances = source.read_u32::<LittleEndian>()?;

    let mut reserved = [0; 8];
    source.read_exact(&mut reserved)?;

    Ok(FileHeader {
        num_instance_types,
        num_instances,
    })
}

#[derive(Debug)]
struct ChunkHeader {
    pub name: [u8; 4],
    pub compressed_len: u32,
    pub len: u32,
    pub reserved: u32,
}

impl fmt::Display for ChunkHeader {
    fn fmt(&self, output: &mut fmt::Formatter) -> fmt::Result {
        let name = if let Ok(name) = str::from_utf8(&self.name) {
            Cow::Borrowed(name)
        } else {
            Cow::Owned(format!("{:?}", self.name))
        };

        write!(output, "Chunk \"{}\" (compressed: {}, len: {}, reserved: {})", name, self.compressed_len, self.len, self.reserved)
    }
}

fn decode_chunk_header<R: Read>(mut source: R) -> io::Result<ChunkHeader> {
    let mut name = [0; 4];
    source.read_exact(&mut name)?;

    let compressed_len = source.read_u32::<LittleEndian>()?;
    let len = source.read_u32::<LittleEndian>()?;
    let reserved = source.read_u32::<LittleEndian>()?;

    Ok(ChunkHeader {
        name,
        compressed_len,
        len,
        reserved,
    })
}

fn decode_chunk<R: Read>(mut source: R, output: &mut Vec<u8>) -> io::Result<ChunkHeader> {
    let header = decode_chunk_header(&mut source)?;

    println!("{}", header);

    if header.compressed_len == 0 {
        (&mut source).take(header.len as u64).read_to_end(output)?;
    } else {
        let mut compressed_data = Vec::new();
        (&mut source).take(header.compressed_len as u64).read_to_end(&mut compressed_data)?;

        let data = lz4::block::decompress(&compressed_data, Some(header.len as i32))?;
        output.extend_from_slice(&data);
    }

    assert_eq!(output.len(), header.len as usize);

    Ok(header)
}

fn decode_metadata_chunk<R: Read>(mut source: R) -> io::Result<HashMap<String, String>> {
    let mut output = HashMap::new();
    let len = source.read_u32::<LittleEndian>()?;

    for _ in 0..len {
        let key = decode_string(&mut source)?;
        let value = decode_string(&mut source)?;

        output.insert(key, value);
    }

    Ok(output)
}

fn decode_string<R: Read>(mut source: R) -> io::Result<String> {
    let length = source.read_u32::<LittleEndian>()?;

    let mut value = String::new();
    (&mut source).take(length as u64).read_to_string(&mut value)?;

    Ok(value)
}

#[cfg(test)]
mod test {
    use super::*;

    use std::collections::HashMap;

    use rbx_tree::RbxInstance;

    fn new_test_tree() -> RbxTree {
        let root = RbxInstance {
            name: "Folder".to_string(),
            class_name: "Folder".to_string(),
            properties: HashMap::new(),
        };

        RbxTree::new(root)
    }

    #[test]
    fn decode_a() {
        static CONTENTS: &[u8] = include_bytes!("../test-files/model-a.rbxm");

        let mut tree = new_test_tree();
        let root_id = tree.get_root_id();

        decode(&mut tree, root_id, CONTENTS).unwrap();
    }
}