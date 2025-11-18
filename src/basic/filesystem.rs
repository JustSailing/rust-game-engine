use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::{fs, io};
use thiserror::Error;

type Result<T> = std::result::Result<T, FileHandleError>;

pub enum FileModes {
    READ = 0b001,
    WRITE = 0b010,
    RW = 0b100,
}

#[derive(Error, Debug)]
pub enum FileHandleError {
    #[error("could not open file {path} {} {}", file!(), line!())]
    CannotOpen { path: String },
    #[error("could not open file {} {}", file!(), line!())]
    CannotReadln,
    #[error("could not seek file {} {}", file!(), line!())]
    CannotSeek,
    #[error("could not write file {} {}", file!(), line!())]
    CannotWriteToFile,
    #[error("could not flush file {} {}", file!(), line!())]
    CannotFlushFile,
    #[error("could read all file {} {}", file!(), line!())]
    Error {
        #[from]
        source: io::Error,
    },
}

#[derive(Debug)]
pub struct FileHandle {
    pub file: File,
    file_name: String,
    is_valid: bool,
}

impl FileHandle {
    pub fn exists(path: &str) -> bool {
        let path = Path::new(path);
        path.exists()
    }

    pub fn open(path: &str, modes: FileModes, _binary: bool) -> Result<Self> {
        let mut options = OpenOptions::new();
        match modes {
            FileModes::READ => options.read(true),
            FileModes::WRITE => options.write(true).create(true),
            FileModes::RW => options.read(true).write(true).create(true),
        };
      
        match options.open(path) {
            Ok(f) => Ok(Self {
                file: f,
                file_name: path.to_string(),
                is_valid: true,
            }),
            Err(_) => Err(FileHandleError::CannotOpen {
                path: path.to_string(),
            }),
        }
    }

    pub fn read_line(&self) -> Result<(usize, String)> {
        let mut reader = BufReader::new(&self.file);
        let mut line = String::new();
        match reader.read_line(&mut line) {
            Ok(0) => Ok((0, "".to_string())),
            Ok(x) => Ok((x, line)),
            Err(_) => Err(FileHandleError::CannotReadln.into()),
        }
    }

    pub fn read_lines(&mut self) -> Result<Vec<String>> {
        match self.file.seek(SeekFrom::Start(0)) {
            Ok(_) => {}
            Err(_) => return Err(FileHandleError::CannotSeek.into()),
        }
        let mut reader = BufReader::new(&self.file);
        let mut all_lines = Vec::new();
        let mut line = String::new();
        loop {
            match reader.read_line(&mut line) {
                Ok(0) => return Ok(all_lines),
                Ok(_) => {
                    all_lines.push(line.clone());
                    line.clear();
                    //continue;
                }
                Err(_) => return Err(FileHandleError::CannotReadln.into()),
            }
        }
    }

    pub fn read_all(&mut self) -> Result<String> {
        match self.file.seek(SeekFrom::Start(0)) {
            Ok(_) => {}
            Err(_) => return Err(FileHandleError::CannotSeek.into()),
        }
        let mut reader = BufReader::new(&self.file);
        let mut line = String::new();
        match reader.read_to_string(&mut line) {
            Ok(_) => {}
            Err(_) => return Err(FileHandleError::CannotReadln.into()),
        }
        Ok(line)
    }

    pub fn read_all_bytes(&mut self) -> Result<Vec<u8>> {
        match self.file.seek(SeekFrom::Start(0)) {
            Ok(_) => {}
            Err(_) => return Err(FileHandleError::CannotSeek.into()),
        }
        let v = fs::read(&self.file_name)?;
        Ok(v)
    }

    pub fn write(&mut self, text: &str) -> Result<()> {
        match self.file.seek(SeekFrom::End(0)) {
            Ok(_) => {}
            Err(_) => return Err(FileHandleError::CannotSeek.into()),
        }

        match self.file.write_all(text.as_bytes()) {
            Ok(_) => {}
            Err(_) => return Err(FileHandleError::CannotWriteToFile.into()),
        }
        match self.file.flush() {
            Ok(_) => {}
            Err(_) => return Err(FileHandleError::CannotFlushFile.into()),
        }
        Ok(())
    }
}
