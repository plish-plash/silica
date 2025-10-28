pub mod image;
pub mod serde_util;

use std::{
    error::Error,
    fmt::Display,
    fs::File,
    io::{BufReader, Error as IoError, ErrorKind, Read, Seek},
    path::PathBuf,
};

use serde::de::DeserializeOwned;
use zip::{ZipArchive, read::ZipFileSeek};

use crate::image::Image;

type AssetPath = str;

#[derive(Debug)]
pub struct AssetError {
    source: String,
    path: Option<String>,
    error: IoError,
}

impl AssetError {
    pub fn new<S: Display, E: Into<IoError>>(source: S, error: E) -> Self {
        AssetError {
            source: source.to_string(),
            path: None,
            error: error.into(),
        }
    }
    pub fn with_path<S: Display, E: Into<IoError>>(source: S, path: &AssetPath, error: E) -> Self {
        AssetError {
            source: source.to_string(),
            path: Some(path.to_string()),
            error: error.into(),
        }
    }
}
impl Display for AssetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(path) = self.path.as_ref() {
            write!(f, "Error loading {} from {}: {}", path, self.source, self.error)
        } else {
            write!(f, "Error loading {}: {}", self.source, self.error)
        }
    }
}
impl Error for AssetError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.error)
    }
}

pub type Result<T> = std::result::Result<T, AssetError>;

pub trait AssetSource: Display {
    type Reader<'a>: Read + Seek
    where
        Self: 'a;
    fn load(&mut self, path: &AssetPath) -> Result<BufReader<Self::Reader<'_>>>;
    fn read_directory(&self, path: &AssetPath) -> Result<Vec<String>>;
}

#[derive(Debug)]
pub struct DirectorySource(PathBuf);

impl DirectorySource {
    pub fn new(path: PathBuf) -> Self {
        DirectorySource(path)
    }
}
impl Display for DirectorySource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.display().fmt(f)
    }
}
impl AssetSource for DirectorySource {
    type Reader<'a> = File;
    fn load(&mut self, path: &AssetPath) -> Result<BufReader<Self::Reader<'_>>> {
        let file_path = self.0.join(path);
        Ok(BufReader::new(
            File::open(file_path).map_err(|e| AssetError::with_path(self.0.display(), path, e))?,
        ))
    }
    fn read_directory(&self, path: &AssetPath) -> Result<Vec<String>> {
        let dir_path = self.0.join(path);
        let mut entries: Vec<_> = std::fs::read_dir(dir_path)
            .map_err(|e| AssetError::with_path(self.0.display(), path, e))?
            .filter_map(|res| {
                res.ok().filter(|e| e.file_type().unwrap().is_file()).map(|e| {
                    e.path()
                        .strip_prefix(&self.0)
                        .expect("invalid path")
                        .to_str()
                        .expect("path not UTF-8")
                        .to_string()
                })
            })
            .collect();
        entries.sort();
        Ok(entries)
    }
}

#[derive(Debug)]
pub struct ArchiveSource {
    path: PathBuf,
    archive: ZipArchive<BufReader<File>>,
}

impl ArchiveSource {
    pub fn new(path: PathBuf) -> Result<Self> {
        let reader = BufReader::new(File::open(&path).map_err(|e| AssetError::new(path.display(), e))?);
        let archive = ZipArchive::new(reader).map_err(|e| AssetError::new(path.display(), e))?;
        Ok(ArchiveSource { path, archive })
    }
}
impl Display for ArchiveSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.path.display().fmt(f)
    }
}
impl AssetSource for ArchiveSource {
    type Reader<'a> = ZipFileSeek<'a, BufReader<File>>;
    fn load(&mut self, path: &AssetPath) -> Result<BufReader<Self::Reader<'_>>> {
        self.archive
            .by_name_seek(path)
            .map(BufReader::new)
            .map_err(|e| AssetError::with_path(self.path.display(), path, e))
    }
    fn read_directory(&self, path: &AssetPath) -> Result<Vec<String>> {
        let mut entries = Vec::new();
        for index in 0..self.archive.len() {
            if let Some(name) = self.archive.name_for_index(index)
                && name.starts_with(path)
            {
                entries.push(name.to_string());
            }
        }
        entries.sort();
        Ok(entries)
    }
}

#[derive(Debug)]
pub struct SubdirectorySource<'a, S> {
    base: &'a mut S,
    path: String,
}

impl<'a, S> SubdirectorySource<'a, S> {
    pub fn new(base: &'a mut S, path: String) -> Self {
        SubdirectorySource { base, path }
    }
}
impl<'a, S> Display for SubdirectorySource<'a, S>
where
    S: Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}", self.base, self.path)
    }
}
impl<'a, S> AssetSource for SubdirectorySource<'a, S>
where
    S: AssetSource,
{
    type Reader<'b>
        = S::Reader<'b>
    where
        Self: 'b;
    fn load(&mut self, path: &AssetPath) -> Result<BufReader<Self::Reader<'_>>> {
        let path = format!("{}/{}", self.path, path);
        self.base.load(&path)
    }
    fn read_directory(&self, path: &AssetPath) -> Result<Vec<String>> {
        let path = format!("{}/{}", self.path, path);
        self.base.read_directory(&path)
    }
}

pub fn load_bytes<S: AssetSource>(asset_source: &mut S, path: &AssetPath) -> Result<Vec<u8>> {
    let mut buf = Vec::new();
    let result = asset_source.load(path)?.read_to_end(&mut buf);
    result.map_err(|e| AssetError::with_path(asset_source, path, e))?;
    Ok(buf)
}
pub fn load_string<S: AssetSource>(asset_source: &mut S, path: &AssetPath) -> Result<String> {
    let mut buf = String::new();
    let result = asset_source.load(path)?.read_to_string(&mut buf);
    result.map_err(|e| AssetError::with_path(asset_source, path, e))?;
    Ok(buf)
}
pub fn load_yaml<S: AssetSource, T: DeserializeOwned>(asset_source: &mut S, path: &AssetPath) -> Result<T> {
    let reader = asset_source.load(path)?;
    serde_yml::from_reader(reader)
        .map_err(|e| AssetError::with_path(asset_source, path, IoError::new(ErrorKind::InvalidData, e)))
}
pub fn load_image<S: AssetSource>(asset_source: &mut S, path: &AssetPath) -> Result<Image> {
    let reader = asset_source.load(path)?;
    Image::read(reader).map_err(|e| {
        let error = match e {
            png::DecodingError::IoError(error) => error,
            png::DecodingError::Format(_) => IoError::new(ErrorKind::InvalidData, e),
            png::DecodingError::Parameter(_) => IoError::new(ErrorKind::InvalidInput, e),
            png::DecodingError::LimitsExceeded => IoError::new(ErrorKind::FileTooLarge, e),
        };
        AssetError::with_path(asset_source, path, error)
    })
}
