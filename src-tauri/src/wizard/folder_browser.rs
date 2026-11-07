//! Lists drives (Windows) or the home folder and the mounted disks (macOS, Linux) and the
//! subfolders for the click-first folder picker, and creates a new subfolder from it.
//! A folder that cannot be opened is reported in the listing (never a panic or a failed command).

use crate::shared::error_messages::{self, Explanation};
use crate::shared::path_rules;
use serde::Serialize;
use std::fs::DirEntry;
use std::path::Path;

const CANNOT_OPEN: &str =
    "Non riesco ad aprire questa cartella. Il disco o la rete sono collegati?";
#[cfg(windows)]
const FILE_ATTRIBUTE_DIRECTORY: u32 = 0x10;
#[cfg(windows)]
const FILE_ATTRIBUTE_HIDDEN: u32 = 0x2;
#[cfg(windows)]
const FILE_ATTRIBUTE_SYSTEM: u32 = 0x4;

#[derive(Debug, PartialEq, Serialize)]
pub struct Folder {
    pub name: String,
    pub path: String,
}

#[derive(Debug, PartialEq, Serialize)]
pub struct FolderListing {
    /// The folder one level up; empty when that is the list of drives (or of places).
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
            folders: roots(),
            error: None,
        };
    }
    let parent = if is_root(path) {
        String::new()
    } else {
        Path::new(path)
            .parent()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_default()
    };
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

/// Creates the folder `name` directly inside `parent` and returns its path. It is a new B for the
/// folder A `source`: a folder that `check_destination` would refuse (A itself, inside A) is not
/// created at all. The name is hostile input and the Windows rules apply on every OS (the folder
/// may end up on a stick Windows reads): one plain name, never a path. Fails when it already
/// exists; never creates `parent`.
pub fn create_folder(source: &str, parent: &str, name: &str) -> Result<String, Explanation> {
    let name = path_rules::checked_name(name)?;
    let parent = Path::new(parent.trim());
    if !parent.is_absolute() {
        return Err(error_messages::explain_missing_parent());
    }
    let path = parent.join(name);
    path_rules::check_destination(source, &path.to_string_lossy())?;
    std::fs::create_dir(&path).map_err(|e| error_messages::explain_create_error(&e))?;
    Ok(path.to_string_lossy().into_owned())
}

/// Drives that exist, found by probing `A:\` to `Z:\` (UNC shares cannot be enumerated).
#[cfg(windows)]
fn roots() -> Vec<Folder> {
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

/// The home folder, then the disks mounted where the system puts them: `/Volumes` on macOS,
/// `/media/<user>`, `/run/media/<user>` and `/mnt` on Linux.
#[cfg(unix)]
fn roots() -> Vec<Folder> {
    let mut roots = Vec::new();
    if let Some(home) = std::env::var_os("HOME") {
        roots.push(Folder {
            name: "Cartella personale".into(),
            path: home.to_string_lossy().into_owned(),
        });
    }
    let user = std::env::var("USER").unwrap_or_default();
    let mount_dirs = if cfg!(target_os = "macos") {
        vec!["/Volumes".to_string()]
    } else {
        vec![
            format!("/media/{user}"),
            format!("/run/media/{user}"),
            "/mnt".to_string(),
        ]
    };
    for dir in mount_dirs {
        roots.extend(without_boot_aliases(
            subfolders(Path::new(&dir)).unwrap_or_default(),
        ));
    }
    roots
}

/// `/Volumes/Macintosh HD` is a link to `/`: not a disk of its own, and a way to reach A from
/// inside B. Folders that resolve to the root are left out.
#[cfg(unix)]
fn without_boot_aliases(folders: Vec<Folder>) -> Vec<Folder> {
    folders
        .into_iter()
        .filter(|folder| {
            std::fs::canonicalize(&folder.path).is_ok_and(|real| real != Path::new("/"))
        })
        .collect()
}

/// A folder that sits directly in the list of drives: one level up is that list, not a folder.
/// On Windows `Path::parent` already says so for a drive.
#[cfg(windows)]
fn is_root(_path: &str) -> bool {
    false
}

#[cfg(unix)]
fn is_root(path: &str) -> bool {
    roots()
        .iter()
        .any(|root| Path::new(&root.path) == Path::new(path))
}

/// Visible directories directly inside `path`, sorted by name.
fn subfolders(path: &Path) -> std::io::Result<Vec<Folder>> {
    let entries = std::fs::read_dir(path)?;
    let mut folders: Vec<Folder> = entries
        .flatten()
        .filter(is_visible_folder)
        .map(|entry| Folder {
            name: entry.file_name().to_string_lossy().into_owned(),
            path: entry.path().to_string_lossy().into_owned(),
        })
        .collect();
    folders.sort_by_key(|f| f.name.to_lowercase());
    Ok(folders)
}

/// One stat-free read per entry: on Windows `DirEntry::metadata` comes with the directory
/// listing. Junctions and directory symlinks carry the DIRECTORY attribute too.
#[cfg(windows)]
fn is_visible_folder(entry: &DirEntry) -> bool {
    use std::os::windows::fs::MetadataExt;
    entry
        .metadata()
        .is_ok_and(|m| has_visible_folder_attributes(m.file_attributes()))
}

#[cfg(windows)]
fn has_visible_folder_attributes(attributes: u32) -> bool {
    attributes & FILE_ATTRIBUTE_DIRECTORY != 0
        && attributes & (FILE_ATTRIBUTE_HIDDEN | FILE_ATTRIBUTE_SYSTEM) == 0
}

/// A folder (also a link to one) whose name does not start with a dot. Only a link needs a stat
/// of its own, to see where it points.
#[cfg(unix)]
fn is_visible_folder(entry: &DirEntry) -> bool {
    !entry.file_name().to_string_lossy().starts_with('.')
        && entry
            .file_type()
            .is_ok_and(|kind| kind.is_dir() || (kind.is_symlink() && entry.path().is_dir()))
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

    /// A source that has nothing to do with the temp folders the tests create in.
    const ELSEWHERE: &str = if cfg!(windows) {
        r"Z:\Sorgente"
    } else {
        "/sorgente-inesistente"
    };

    #[test]
    fn a_folder_inside_the_source_is_never_created() {
        let source = tempfile::tempdir().unwrap();
        let sub = source.path().join("sotto");
        std::fs::create_dir(&sub).unwrap();
        let source_path = source.path().to_str().unwrap();

        let inside = create_folder(source_path, source_path, "Backup").unwrap_err();
        let deeper = create_folder(source_path, sub.to_str().unwrap(), "Backup").unwrap_err();

        assert_eq!(inside, Explanation::from(path_rules::INSIDE_SOURCE));
        assert_eq!(deeper, Explanation::from(path_rules::INSIDE_SOURCE));
        assert!(!source.path().join("Backup").exists());
        assert!(!sub.join("Backup").exists());
    }

    #[test]
    fn a_folder_next_to_the_source_is_created() {
        let parent = tempfile::tempdir().unwrap();
        let source = parent.path().join("Foto");
        std::fs::create_dir(&source).unwrap();

        let made = create_folder(
            source.to_str().unwrap(),
            parent.path().to_str().unwrap(),
            "Copia",
        )
        .unwrap();

        assert!(Path::new(&made).is_dir());
    }

    fn refused(name: &str) -> Explanation {
        let dir = tempfile::tempdir().unwrap();
        let error = create_folder(ELSEWHERE, dir.path().to_str().unwrap(), name).unwrap_err();
        assert_eq!(
            std::fs::read_dir(dir.path()).unwrap().count(),
            0,
            "{name:?} must create nothing"
        );
        error
    }

    #[test]
    fn a_new_folder_is_created_directly_inside_the_parent_and_its_path_returned() {
        let dir = tempfile::tempdir().unwrap();

        let path = create_folder(ELSEWHERE, dir.path().to_str().unwrap(), "  Foto 2024  ").unwrap();

        assert_eq!(Path::new(&path), dir.path().join("Foto 2024"));
        assert!(dir.path().join("Foto 2024").is_dir());
    }

    #[test]
    fn a_refused_name_creates_nothing() {
        assert!(
            refused(r"..\evil")
                .title
                .starts_with("Il nome non può contenere")
        );
    }

    #[test]
    fn an_existing_folder_or_file_is_never_reused() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("Foto")).unwrap();
        std::fs::write(dir.path().join("nota.txt"), "x").unwrap();
        let parent = dir.path().to_str().unwrap();

        for name in ["Foto", "nota.txt"] {
            let e = create_folder(ELSEWHERE, parent, name).unwrap_err();
            assert_eq!(e.title, "Esiste già una cartella con questo nome.");
            assert_eq!(e.advice, "Scegli un altro nome.");
        }
    }

    #[test]
    fn a_parent_that_is_not_there_is_an_error_and_is_not_created() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("non-esiste");

        let e = create_folder(ELSEWHERE, missing.to_str().unwrap(), "Foto").unwrap_err();

        assert_eq!(e.title, "Non trovo la cartella in cui creare la nuova.");
        assert_eq!(e.advice, "Il disco è collegato?");
        assert!(!missing.exists());
        assert!(create_folder(ELSEWHERE, "", "Foto").is_err());
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
    #[cfg(windows)]
    fn only_visible_directories_pass() {
        assert!(has_visible_folder_attributes(FILE_ATTRIBUTE_DIRECTORY));
        assert!(!has_visible_folder_attributes(
            FILE_ATTRIBUTE_DIRECTORY | FILE_ATTRIBUTE_HIDDEN
        ));
        assert!(!has_visible_folder_attributes(
            FILE_ATTRIBUTE_DIRECTORY | FILE_ATTRIBUTE_SYSTEM
        ));
        assert!(!has_visible_folder_attributes(0x20)); // a file
    }

    #[test]
    #[cfg(windows)]
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
    #[cfg(windows)]
    fn parent_of_a_folder_is_the_one_above_and_of_a_drive_root_is_empty() {
        assert_eq!(browse(r"C:\Non\Esiste").parent, r"C:\Non");
        assert_eq!(browse(r"C:\Non").parent, r"C:\");
        assert_eq!(browse(r"C:\").parent, "");
    }

    #[test]
    #[cfg(unix)]
    fn hidden_folders_are_left_out_and_links_to_folders_are_kept() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join(".nascosta")).unwrap();
        std::fs::create_dir(dir.path().join("vera")).unwrap();
        std::os::unix::fs::symlink(dir.path().join("vera"), dir.path().join("scorciatoia"))
            .unwrap();
        std::fs::write(dir.path().join("file.txt"), "x").unwrap();

        let names: Vec<_> = subfolders(dir.path())
            .unwrap()
            .into_iter()
            .map(|f| f.name)
            .collect();

        assert_eq!(names, ["scorciatoia", "vera"]);
    }

    #[test]
    #[cfg(unix)]
    fn an_alias_of_the_boot_volume_is_not_listed_as_a_disk() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("Backup")).unwrap();
        std::os::unix::fs::symlink("/", dir.path().join("Macintosh HD")).unwrap();
        std::os::unix::fs::symlink(dir.path().join("Backup"), dir.path().join("Scorciatoia"))
            .unwrap();

        let names: Vec<_> = without_boot_aliases(subfolders(dir.path()).unwrap())
            .into_iter()
            .map(|f| f.name)
            .collect();

        assert_eq!(names, ["Backup", "Scorciatoia"]);
    }

    #[test]
    #[cfg(unix)]
    fn empty_path_lists_the_home_folder_whose_parent_is_the_list() {
        let home = std::env::var("HOME").unwrap();

        let listing = browse("");
        assert_eq!(listing.parent, "");
        assert_eq!(
            listing.folders.first().map(|f| f.path.as_str()),
            Some(home.as_str())
        );

        assert_eq!(
            browse(&home).parent,
            "",
            "one level up from home is the list, not its folder"
        );
        assert_eq!(browse(&format!("{home}/non-esiste")).parent, home);
        assert_eq!(browse("/").parent, "");
    }
}
