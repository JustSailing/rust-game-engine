use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::{env, fmt};

type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>;

pub enum FileModes {
    READ = 0b001,
    WRITE = 0b010,
    RW = 0b100,
}

#[derive(Debug)]
pub enum Error {
    CannotOpen,
    CannotReadln,
    CannotSeek,
    CannotWriteToFile,
    CannotFlushFile,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::CannotOpen => write!(f, "cannot open file {} {}", file!(), line!()),
            Error::CannotReadln => write!(f, "cannot read line {} {}", file!(), line!()),
            Error::CannotSeek => write!(f, "cannot seek file {} {}", file!(), line!()),
            Error::CannotWriteToFile => {
                write!(f, "cannot write to file {} {}", file!(), line!())
            }
            Error::CannotFlushFile => write!(f, "cannot flush file {} {}", file!(), line!()),
        }
    }
}

impl std::error::Error for Error {}

#[derive(Debug)]
pub struct FileHandle {
    pub file: File,
    //file_name: &'a str,
    is_valid: bool,
}

impl FileHandle {
    pub fn exists(path: &str) -> bool {
        let path = Path::new(path);
        path.exists()
    }

    pub fn open(path: &str, modes: FileModes, _binary: bool) -> Result<Self> {
        let current_dir = env::current_dir().unwrap();
        println!("current dir: {:?}", current_dir);
        let mut options = OpenOptions::new();
        match modes {
            FileModes::READ => options.read(true),
            FileModes::WRITE => options.write(true),
            FileModes::RW => options.read(true).write(true),
        };

        match options.open(path) {
            Ok(f) => Ok(Self {
                file: f,
                //file_name: path.clone(),
                is_valid: true,
            }),
            Err(_) => Err(Error::CannotOpen.into()),
        }
    }

    pub fn read_line(&self) -> Result<String> {
        let mut reader = BufReader::new(&self.file);
        let mut line = String::new();
        match reader.read_line(&mut line) {
            Ok(_) => {}
            Err(_) => return Err(Error::CannotReadln.into()),
        }
        Ok(line)
    }

    pub fn read_all(&mut self) -> Result<String> {
        match self.file.seek(SeekFrom::Start(0)) {
            Ok(_) => {}
            Err(_) => return Err(Error::CannotSeek.into()),
        }
        let mut reader = BufReader::new(&self.file);
        let mut line = String::new();
        match reader.read_to_string(&mut line) {
            Ok(_) => {}
            Err(_) => return Err(Error::CannotReadln.into()),
        }
        Ok(line)
    }

    pub fn write(&mut self, text: &str) -> Result<()> {
        match self.file.seek(SeekFrom::End(0)) {
            Ok(_) => {}
            Err(_) => return Err(Error::CannotSeek.into()),
        }

        match self.file.write_all(text.as_bytes()) {
            Ok(_) => {}
            Err(_) => return Err(Error::CannotWriteToFile.into()),
        }
        match self.file.flush() {
            Ok(_) => {}
            Err(_) => return Err(Error::CannotFlushFile.into()),
        }
        Ok(())
    }
}
