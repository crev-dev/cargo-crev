use blake2;
use failure_derive::Fail;
use std::io::BufRead;
use std::{
    collections::{BTreeMap, HashSet},
    ffi::OsString,
    fs,
    path::{Component, Path, PathBuf},
};

fn read_file_to_digest_input(path: &Path, input: &mut impl digest::Digest) -> std::io::Result<()> {
    let file = fs::File::open(path)?;

    let mut reader = std::io::BufReader::new(file);

    loop {
        let length = {
            let buffer = reader.fill_buf()?;
            input.input(buffer);
            buffer.len()
        };
        if length == 0 {
            break;
        }
        reader.consume(length);
    }

    Ok(())
}

/// Sorted list of all descendants of a directory
type Descendants = BTreeMap<OsString, Entry>;

#[derive(Default)]
struct Entry(Descendants);

struct RecursiveDigest<Digest = blake2::Blake2b> {
    root_path: PathBuf,
    root: Entry,
    digest: std::marker::PhantomData<Digest>,
}

#[derive(Debug, Fail)]
pub enum DigestError {
    #[fail(display = "could not convert OsStr string to utf8")]
    OsStrConversionError,
    #[fail(display = "io Error: {}", _0)]
    IoError(std::io::Error),
    #[fail(display = "walkdir Error: {}", _0)]
    WalkdirError(walkdir::Error),
    #[fail(display = "an entry that was supposed to be a file, contains sub-entries")]
    FileWithSubentriesError,
    #[fail(display = "file not supported: {}", _0)]
    FileNotSupported(String),
}

impl From<std::io::Error> for DigestError {
    fn from(err: std::io::Error) -> Self {
        DigestError::IoError(err)
    }
}

impl From<walkdir::Error> for DigestError {
    fn from(err: walkdir::Error) -> Self {
        DigestError::WalkdirError(err)
    }
}

impl<Digest, OutputSize> RecursiveDigest<Digest>
where
    Digest: digest::Digest<OutputSize = OutputSize> + digest::FixedOutput,
    OutputSize: generic_array::ArrayLength<u8>,
{
    fn new<I>(root_path: PathBuf, rel_paths: I) -> Self
    where
        I: IntoIterator<Item = PathBuf>,
    {
        let mut s = Self {
            root_path,
            root: Entry(Default::default()),
            digest: std::marker::PhantomData,
        };

        for path in rel_paths.into_iter() {
            assert!(
                !path.is_absolute(),
                "RecursiveDigest: Expected only relative paths: {}",
                path.display()
            );
            s.insert_path(&path);
        }

        s
    }

    fn get_digest(self) -> Result<Vec<u8>, DigestError> {
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

    fn read_content_of(
        &self,
        full_path: &Path,
        entry: &Entry,
        hasher: &mut Digest,
    ) -> Result<(), DigestError> {
        let attr = fs::symlink_metadata(full_path)?;
        if attr.is_file() {
            self.read_content_of_file(full_path, entry, hasher)
        } else if attr.is_dir() {
            self.read_content_of_dir(full_path, entry, hasher)
        } else if attr.file_type().is_symlink() {
            self.read_content_of_symlink(full_path, entry, hasher)
        } else {
            Err(DigestError::FileNotSupported(
                full_path.to_string_lossy().to_string(),
            ))
        }
    }

    fn read_content_of_dir(
        &self,
        full_path: &Path,
        entry: &Entry,
        parent_hasher: &mut Digest,
    ) -> Result<(), DigestError> {
        parent_hasher.input(b"D");
        for (k, v) in &entry.0 {
            let mut hasher = Digest::new();
            hasher.input(
                k.to_str()
                    .ok_or(DigestError::OsStrConversionError)?
                    .as_bytes(),
            );
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
    ) -> Result<(), DigestError> {
        if !entry.0.is_empty() {
            return Err(DigestError::FileWithSubentriesError);
        }

        parent_hasher.input(b"F");
        read_file_to_digest_input(full_path, parent_hasher)?;
        Ok(())
    }

    fn read_content_of_symlink(
        &self,
        full_path: &Path,
        entry: &Entry,
        parent_hasher: &mut Digest,
    ) -> Result<(), DigestError> {
        assert!(entry.0.is_empty());
        parent_hasher.input(b"L");
        parent_hasher.input(
            full_path
                .read_link()?
                .to_str()
                .ok_or(DigestError::OsStrConversionError)?
                .as_bytes(),
        );
        Ok(())
    }
}

pub fn get_recursive_digest_for_paths<Digest: digest::Digest + digest::FixedOutput, H>(
    root_path: &Path,
    paths: HashSet<PathBuf, H>,
) -> Result<Vec<u8>, DigestError>
where
    H: std::hash::BuildHasher,
{
    RecursiveDigest::<Digest>::new(root_path.into(), paths).get_digest()
}

/// A helper function that strips a root folder from a path. If the root folder
/// is not part of the path it will simply return.
fn strip_root_path_if_included<'a>(root_path: &Path, path: &'a Path) -> &'a Path {
    path.strip_prefix(&root_path).unwrap_or(path)
}

#[test]
fn test_strip_root_path_if_included() {
    let root_path = Path::new("some/root/path");

    // Should strip the root path here
    let path_with_root = Path::new("some/root/path/and/subfolder");
    assert_eq!(
        strip_root_path_if_included(&root_path, path_with_root),
        Path::new("and/subfolder")
    );

    // Should keep this path intact
    let path_without_root = Path::new("other/path/and/subfolder");
    assert_eq!(
        strip_root_path_if_included(&root_path, path_without_root),
        path_without_root
    );
}

pub fn get_recursive_digest_for_dir<
    Digest: digest::Digest + digest::FixedOutput,
    H: std::hash::BuildHasher,
>(
    root_path: &Path,
    rel_path_ignore_list: &HashSet<PathBuf, H>,
) -> Result<Vec<u8>, DigestError> {
    let mut hasher = RecursiveDigest::<Digest>::new(root_path.into(), None);

    for entry in walkdir::WalkDir::new(root_path)
        .into_iter()
        .filter_entry(|entry| {
            let path = strip_root_path_if_included(&root_path, entry.path());
            !rel_path_ignore_list.contains(path)
        })
    {
        let entry = entry?;
        let path = strip_root_path_if_included(&root_path, entry.path());
        if !rel_path_ignore_list.contains(path) {
            hasher.insert_path(path);
        }
    }

    hasher.get_digest()
}
