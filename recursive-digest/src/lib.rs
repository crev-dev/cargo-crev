use blake2;
use default::default;
use std::io::BufRead;
use std::{
    collections::{BTreeMap, HashSet},
    ffi::OsString,
    fs,
    os::unix::ffi::OsStrExt,
    path::{Component, Path, PathBuf},
};

/// Sorted list of all descendants of a directory
type Descendants = BTreeMap<OsString, Entry>;

pub type Result<T> = std::io::Result<T>;

#[derive(Default)]
struct Entry(Descendants);

struct RecursiveDigest<Digest = blake2::Blake2b, IO = FsIoOps> {
    root_path: PathBuf,
    root: Entry,
    digest: std::marker::PhantomData<Digest>,
    io: IO,
}

enum FileType {
    File,
    Symlink,
    Directory,
}

trait IoOps: Default {
    fn get_file_type(&self, path: &Path) -> Result<FileType>;
    fn read_file_into_hasher(&self, path: &Path, hasher: &mut impl digest::Digest) -> Result<()>;
    fn read_symlink_into_hasher(&self, path: &Path, hasher: &mut impl digest::Digest)
        -> Result<()>;
}

#[derive(Default)]
struct FsIoOps;

impl IoOps for FsIoOps {
    fn get_file_type(&self, path: &Path) -> Result<FileType> {
        use self::FileType::*;
        let attr = fs::symlink_metadata(path)?;

        Ok(if attr.is_file() {
            File
        } else if attr.file_type().is_symlink() {
            Symlink
        } else if attr.is_dir() {
            Directory
        } else {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("unsupported file type: {}", path.display()),
            ));
        })
    }

    fn read_file_into_hasher(&self, path: &Path, hasher: &mut impl digest::Digest) -> Result<()> {
        let file = fs::File::open(path)?;

        let mut reader = std::io::BufReader::new(file);

        loop {
            let length = {
                let buffer = reader.fill_buf()?;
                hasher.process(buffer);
                buffer.len()
            };
            if length == 0 {
                break;
            }
            reader.consume(length);
        }

        Ok(())
    }

    fn read_symlink_into_hasher(
        &self,
        path: &Path,
        hasher: &mut impl digest::Digest,
    ) -> Result<()> {
        hasher.process(path.read_link()?.as_os_str().as_bytes());
        Ok(())
    }
}

impl<Digest, IO, OutputSize> RecursiveDigest<Digest, IO>
where
    Digest: digest::Digest<OutputSize = OutputSize>,
    OutputSize: generic_array::ArrayLength<u8>,
    IO: IoOps,
{
    fn new<I>(root_path: PathBuf, rel_paths: I) -> Self
    where
        I: IntoIterator<Item = PathBuf>,
    {
        let mut s = Self {
            root_path,
            root: Entry(Default::default()),
            digest: std::marker::PhantomData,
            io: default(),
        };

        for path in rel_paths.into_iter() {
            if path.is_absolute() {
                panic!(
                    "RecursiveDigest: Expected only relative paths: {}",
                    path.display()
                );
            }
            s.insert_path(&path);
        }

        s
    }

    fn get_digest(self) -> Result<Vec<u8>> {
        let mut hasher = Digest::new();

        self.read_content_of(&self.root_path, &self.root, &mut hasher)?;

        Ok(hasher.result().to_vec())
    }

    fn insert_path(&mut self, path: &Path) {
        let mut cur_entry = &mut self.root;

        for comp in path.components() {
            match comp {
                Component::Normal(osstr) => {
                    cur_entry = cur_entry.0.entry(osstr.to_owned()).or_default();
                }
                _ => panic!("Didn't expect {:?}", comp),
            }
        }
    }

    fn read_content_of(&self, full_path: &Path, entry: &Entry, hasher: &mut Digest) -> Result<()> {
        use self::FileType::*;
        match self.io.get_file_type(full_path)? {
            File => self.read_content_of_file(full_path, entry, hasher)?,
            Symlink => self.read_content_of_symlink(full_path, entry, hasher)?,
            Directory => self.read_content_of_dir(full_path, entry, hasher)?,
        }

        Ok(())
    }

    fn read_content_of_dir(
        &self,
        full_path: &Path,
        entry: &Entry,
        parent_hasher: &mut Digest,
    ) -> Result<()> {
        parent_hasher.input("D".as_bytes());
        for (k, v) in &entry.0 {
            let mut hasher = Digest::new();
            hasher.input(k.as_bytes());
            parent_hasher.input(hasher.fixed_result().as_slice());

            let mut hasher = Digest::new();
            let full_path = full_path.join(k);
            self.read_content_of(&full_path, &v, &mut hasher)?;
            parent_hasher.input(hasher.fixed_result().as_slice());
        }

        Ok(())
    }

    fn read_content_of_file(
        &self,
        full_path: &Path,
        entry: &Entry,
        parent_hasher: &mut Digest,
    ) -> Result<()> {
        if !entry.0.is_empty() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "an entry that was supposed to be a file, contains sub-entries",
            ));
        }

        parent_hasher.input("F".as_bytes());
        self.io.read_file_into_hasher(full_path, parent_hasher)?;
        Ok(())
    }

    fn read_content_of_symlink(
        &self,
        full_path: &Path,
        entry: &Entry,
        parent_hasher: &mut Digest,
    ) -> Result<()> {
        if !entry.0.is_empty() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "an entry that was supposed to be a symlink, contains sub-entries",
            ));
        }
        parent_hasher.input("L".as_bytes());
        self.io
            .read_symlink_into_hasher(&full_path, parent_hasher)?;
        Ok(())
    }
}

pub fn get_recursive_digest_for_paths<Digest: digest::Digest, H>(
    root_path: &Path,
    rel_paths: HashSet<PathBuf, H>,
) -> Result<Vec<u8>>
where
    H: std::hash::BuildHasher,
{
    RecursiveDigest::<Digest>::new(root_path.into(), rel_paths).get_digest()
}

pub fn get_recursive_digest_for_dir<Digest: digest::Digest, H: std::hash::BuildHasher>(
    root_path: &Path,
    rel_path_ignore_list: HashSet<PathBuf, H>,
) -> Result<Vec<u8>> {
    let mut hasher = RecursiveDigest::<Digest>::new(root_path.into(), None);

    for entry in walkdir::WalkDir::new(root_path) {
        let entry = entry?;
        let path = entry
            .path()
            .strip_prefix(&root_path)
            .unwrap_or_else(|_| entry.path());
        if !rel_path_ignore_list.contains(path) {
            hasher.insert_path(path)
        }
    }

    Ok(hasher.get_digest()?)
}
