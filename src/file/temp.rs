use crate::error::Result;
use crate::file::File;
use lz4_flex::frame::FrameDecoder;
use std::io::{Read, Write};
use tempfile::NamedTempFile;

struct Temp {
    _temp: NamedTempFile,
    file: File,
}

impl std::ops::Deref for Temp {
    type Target = File;

    fn deref(&self) -> &Self::Target {
        &self.file
    }
}

impl Temp {
    pub fn new() -> Result<Self> {
        // `NamedTempFile` creates an empty file, which jammdb rejects as malformed…
        let mut temp = NamedTempFile::new()?;
        let path = temp.path().to_path_buf();

        // …so we copy a valid file into place
        let mut decoder = FrameDecoder::new(BLANK.as_slice());
        let mut buffer = Vec::new();
        decoder.read_to_end(&mut buffer)?;
        temp.write_all(&buffer)?;
        drop(buffer);

        let file = File::file(path)?;
        Ok(Temp { _temp: temp, file })
    }
}

static BLANK: &[u8; 167] = include_bytes!("temp.rs.db.lz4");

#[cfg(test)]
mod test {
    use super::*;

    /// Then run `lz4 --best -v -f temp.rs.db` for the `include_bytes!` above
    #[test]
    #[ignore]
    fn make_temp_file() -> Result<()> {
        let mut path = std::path::PathBuf::from(file!());
        path.add_extension("db");

        let db = jammdb::OpenOptions::new()
            .pagesize(4096)
            .num_pages(4) // “this function will panic if you provide a value < 4”
            .open(path.as_path())?;

        let tx = db.tx(true)?;
        tx.commit()?;

        Ok(())
    }

    #[test]
    fn prefilled_temp_file_is_valid() -> Result<()> {
        let _ = Temp::new()?;

        Ok(())
    }
}
