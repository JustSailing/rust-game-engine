use bitflags::bitflags;
use std::env;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom, Write};
use std::path::Path;

bitflags! {
  pub struct FileModes: u8 {
        const READ = 0b001;
        const WRITE = 0b010;
       //const BINARY = 0b100;
    }
}

bitflags! {
  pub struct FileError: usize {
    const CANNOT_OPEN = 1 << 0;
    const CANNOT_READLN = 1 << 1;
    const CANNOT_SEEK = 1 << 2;
    const CANNOT_WRITE_TO_FILE = 1 << 3;
    const CANNOT_FLUSH_FILE = 1 << 4;
  }
}

#[derive(Debug)]
pub struct FileHandle {
    pub file: File,
    is_valid: bool,
}

impl FileHandle {
    pub fn exists(path: &str) -> bool {
        let path = Path::new(path);
        path.exists()
    }

    pub fn open(path: &str, modes: FileModes, binary: bool) -> Result<Self, FileError> {
        let current_dir = env::current_dir().unwrap();
        println!("current dir: {:?}", current_dir);
        let mut options = OpenOptions::new();
        if modes.contains(FileModes::READ) {
            options.read(true);
        }
        if modes.contains(FileModes::WRITE) {
            options.write(true);
        }
        match options.open(path) {
            Ok(f) => Ok(Self {
                file: f,
                is_valid: true,
            }),
            Err(_) => Err(FileError::CANNOT_OPEN),
        }
    }

    pub fn read_line(&self) -> Result<String, FileError> {
        let mut reader = BufReader::new(&self.file);
        let mut line = String::new();
        match reader.read_line(&mut line) {
            Ok(_) => {}
            Err(_) => return Err(FileError::CANNOT_READLN),
        }
        Ok(line)
    }

    pub fn read_all(&mut self) -> Result<String, FileError> {
        match self.file.seek(SeekFrom::Start(0)) {
            Ok(_) => {}
            Err(_) => return Err(FileError::CANNOT_SEEK),
        }
        let mut reader = BufReader::new(&self.file);
        let mut line = String::new();
        match reader.read_to_string(&mut line) {
            Ok(_) => {}
            Err(_) => return Err(FileError::CANNOT_READLN),
        }
        Ok(line)
    }

    pub fn write(&mut self, text: &str) -> Result<(), FileError> {
        match self.file.seek(SeekFrom::End(0)) {
            Ok(_) => {}
            Err(_) => return Err(FileError::CANNOT_SEEK),
        }

        match self.file.write_all(text.as_bytes()) {
            Ok(_) => {}
            Err(_) => return Err(FileError::CANNOT_WRITE_TO_FILE),
        }
        match self.file.flush() {
            Ok(_) => {}
            Err(_) => return Err(FileError::CANNOT_FLUSH_FILE),
        }
        Ok(())
    }
}
