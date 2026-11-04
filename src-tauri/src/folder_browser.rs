//! Lists drives and subfolders for the click-first folder picker.
//! Nothing here can fail: an unreadable or missing folder is shown as empty.

use serde::Serialize;
use std::os::windows::fs::MetadataExt;
use std::path::Path;

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
}

/// Empty `path` lists the drives of the computer, otherwise the subfolders of `path`.
pub fn browse(path: &str) -> FolderListing {
    let path = path.trim();
    if path.is_empty() {
        return FolderListing {
            parent: String::new(),
            folders: drives(),
        };
    }
    let parent = Path::new(path)
        .parent()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_default();
    FolderListing {
        parent,
        folders: subfolders(Path::new(path)),
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
fn subfolders(path: &Path) -> Vec<Folder> {
    let Ok(entries) = std::fs::read_dir(path) else {
        return Vec::new();
    };
    let mut folders: Vec<Folder> = entries
        .flatten()
        .filter(|entry| {
            // `Path::is_dir` follows junctions and symlinks, `DirEntry::file_type` does not.
            entry.path().is_dir()
                && entry
                    .metadata()
                    .map(|m| is_visible(m.file_attributes()))
                    .unwrap_or(true)
        })
        .map(|entry| Folder {
            name: entry.file_name().to_string_lossy().into_owned(),
            path: entry.path().to_string_lossy().into_owned(),
        })
        .collect();
    folders.sort_by_key(|f| f.name.to_lowercase());
    folders
}

fn is_visible(attributes: u32) -> bool {
    attributes & (FILE_ATTRIBUTE_HIDDEN | FILE_ATTRIBUTE_SYSTEM) == 0
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

        let names: Vec<_> = subfolders(dir.path()).into_iter().map(|f| f.name).collect();

        assert_eq!(names, ["Alfa", "beta"]);
    }

    #[test]
    fn missing_path_lists_nothing() {
        let dir = tempfile::tempdir().unwrap();

        assert!(subfolders(&dir.path().join("non-esiste")).is_empty());
    }

    #[test]
    fn hidden_and_system_folders_are_not_visible() {
        assert!(is_visible(0x10)); // plain directory
        assert!(!is_visible(0x10 | FILE_ATTRIBUTE_HIDDEN));
        assert!(!is_visible(0x10 | FILE_ATTRIBUTE_SYSTEM));
    }

    #[test]
    fn empty_path_lists_the_drives_including_the_system_one() {
        let system_drive = std::env::var("SystemDrive").unwrap(); // "C:"

        let listing = browse("");

        assert_eq!(listing.parent, "");
        assert!(listing
            .folders
            .iter()
            .any(|d| d.path == format!("{system_drive}\\")));
    }

    #[test]
    fn parent_of_a_folder_is_the_one_above_and_of_a_drive_root_is_empty() {
        assert_eq!(browse(r"C:\Non\Esiste").parent, r"C:\Non");
        assert_eq!(browse(r"C:\Non").parent, r"C:\");
        assert_eq!(browse(r"C:\").parent, "");
    }
}
