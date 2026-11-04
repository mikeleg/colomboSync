//! Lists drives and subfolders for the click-first folder picker.
//! A folder that cannot be opened is reported in the listing (never a panic or a failed command).

use serde::Serialize;
use std::os::windows::fs::MetadataExt;
use std::path::Path;

const CANNOT_OPEN: &str =
    "Non riesco ad aprire questa cartella. Il disco o la rete sono collegati?";
const FILE_ATTRIBUTE_DIRECTORY: u32 = 0x10;
const FILE_ATTRIBUTE_HIDDEN: u32 = 0x2;
const FILE_ATTRIBUTE_SYSTEM: u32 = 0x4;

#[derive(Debug, PartialEq, Serialize)]
pub struct Folder {
    pub name: String,
    pub path: String,
}

#[derive(Debug, PartialEq, Serialize)]
pub struct FolderListing {
    /// The folder one level up; empty when that is the list of drives.
    pub parent: String,
    pub folders: Vec<Folder>,
    /// Plain Italian message when `path` could not be opened (disconnected disk or network, no access).
    pub error: Option<String>,
}

/// Empty `path` lists the drives of the computer, otherwise the subfolders of `path`.
pub fn browse(path: &str) -> FolderListing {
    let path = path.trim();
    if path.is_empty() {
        return FolderListing {
            parent: String::new(),
            folders: drives(),
            error: None,
        };
    }
    let parent = Path::new(path)
        .parent()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_default();
    let (folders, error) = match subfolders(Path::new(path)) {
        Ok(folders) => (folders, None),
        Err(_) => (Vec::new(), Some(CANNOT_OPEN.to_string())),
    };
    FolderListing {
        parent,
        folders,
        error,
    }
}

/// Drives that exist, found by probing `A:\` to `Z:\` (UNC shares cannot be enumerated).
fn drives() -> Vec<Folder> {
    ('A'..='Z')
        .filter_map(|letter| {
            let root = format!("{letter}:\\");
            Path::new(&root).exists().then(|| Folder {
                name: format!("Disco {letter}:"),
                path: root,
            })
        })
        .collect()
}

/// Visible directories directly inside `path`, sorted by name.
fn subfolders(path: &Path) -> std::io::Result<Vec<Folder>> {
    let entries = std::fs::read_dir(path)?;
    let mut folders: Vec<Folder> = entries
        .flatten()
        // One stat-free read per entry: on Windows `DirEntry::metadata` comes with the directory listing.
        // Junctions and directory symlinks carry the DIRECTORY attribute too.
        .filter(|entry| {
            entry
                .metadata()
                .is_ok_and(|m| is_visible_folder(m.file_attributes()))
        })
        .map(|entry| Folder {
            name: entry.file_name().to_string_lossy().into_owned(),
            path: entry.path().to_string_lossy().into_owned(),
        })
        .collect();
    folders.sort_by_key(|f| f.name.to_lowercase());
    Ok(folders)
}

fn is_visible_folder(attributes: u32) -> bool {
    attributes & FILE_ATTRIBUTE_DIRECTORY != 0
        && attributes & (FILE_ATTRIBUTE_HIDDEN | FILE_ATTRIBUTE_SYSTEM) == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_only_directories() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("beta")).unwrap();
        std::fs::create_dir(dir.path().join("Alfa")).unwrap();
        std::fs::write(dir.path().join("file.txt"), "x").unwrap();

        let names: Vec<_> = subfolders(dir.path())
            .unwrap()
            .into_iter()
            .map(|f| f.name)
            .collect();

        assert_eq!(names, ["Alfa", "beta"]);
    }

    #[test]
    fn a_folder_that_cannot_be_opened_is_an_error_not_an_empty_list() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("non-esiste");

        let listing = browse(missing.to_str().unwrap());

        assert!(listing.folders.is_empty());
        assert!(listing.error.is_some());
    }

    #[test]
    fn a_folder_with_no_subfolders_is_empty_without_error() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("file.txt"), "x").unwrap();

        let listing = browse(dir.path().to_str().unwrap());

        assert!(listing.folders.is_empty());
        assert_eq!(listing.error, None);
    }

    #[test]
    fn only_visible_directories_pass() {
        assert!(is_visible_folder(FILE_ATTRIBUTE_DIRECTORY));
        assert!(!is_visible_folder(
            FILE_ATTRIBUTE_DIRECTORY | FILE_ATTRIBUTE_HIDDEN
        ));
        assert!(!is_visible_folder(
            FILE_ATTRIBUTE_DIRECTORY | FILE_ATTRIBUTE_SYSTEM
        ));
        assert!(!is_visible_folder(0x20)); // a file
    }

    #[test]
    fn empty_path_lists_the_drives_including_the_system_one() {
        let system_drive = std::env::var("SystemDrive").unwrap(); // "C:"

        let listing = browse("");

        assert_eq!(listing.parent, "");
        assert!(
            listing
                .folders
                .iter()
                .any(|d| d.path == format!("{system_drive}\\"))
        );
    }

    #[test]
    fn parent_of_a_folder_is_the_one_above_and_of_a_drive_root_is_empty() {
        assert_eq!(browse(r"C:\Non\Esiste").parent, r"C:\Non");
        assert_eq!(browse(r"C:\Non").parent, r"C:\");
        assert_eq!(browse(r"C:\").parent, "");
    }
}
