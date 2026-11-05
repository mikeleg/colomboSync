//! Lists drives (Windows) or the home folder and the mounted disks (macOS, Linux) and the
//! subfolders for the click-first folder picker, and creates a new subfolder from it.
//! A folder that cannot be opened is reported in the listing (never a panic or a failed command).

use crate::error_messages::Explanation;
use serde::Serialize;
use std::fs::DirEntry;
use std::io;
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

/// Creates the folder `name` directly inside `parent` and returns its path. The name is hostile
/// input and the Windows rules apply on every OS (the folder may end up on a stick Windows
/// reads): one plain name, never a path. Fails when it already exists; never creates `parent`.
pub fn create_folder(parent: &str, name: &str) -> Result<String, Explanation> {
    let name = checked_name(name)?;
    let parent = Path::new(parent.trim());
    if !parent.is_absolute() {
        return Err(explain_create_error(&io::Error::from(
            io::ErrorKind::NotFound,
        )));
    }
    let path = parent.join(name);
    std::fs::create_dir(&path).map_err(|e| explain_create_error(&e))?;
    Ok(path.to_string_lossy().into_owned())
}

/// The trimmed name when it is something Windows accepts as a single folder name.
fn checked_name(name: &str) -> Result<&str, Explanation> {
    let name = name.trim();
    if name.is_empty() {
        return Err("Scrivi un nome per la nuova cartella.".into());
    }
    if name.len() > 255 {
        return Err("Il nome è troppo lungo.".into());
    }
    if name
        .chars()
        .any(|c| c.is_control() || "<>:\"/\\|?*".contains(c))
    {
        return Err(Explanation {
            title: "Il nome non può contenere questi caratteri: \\ / : * ? \" < > |".into(),
            advice: "Scrivi solo il nome della cartella, senza percorso.".into(),
        });
    }
    if name.ends_with('.') || is_reserved_name(name) {
        return Err(Explanation {
            title: "Questo nome non si può usare.".into(),
            advice: "Scegline un altro: niente punto in fondo, né nomi come CON o NUL.".into(),
        });
    }
    Ok(name)
}

/// Device names Windows keeps for itself, with or without an extension (`NUL.txt`).
fn is_reserved_name(name: &str) -> bool {
    let stem = name.split('.').next().unwrap_or_default().trim_end();
    let stem = stem.to_uppercase();
    if matches!(
        stem.as_str(),
        "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$"
    ) {
        return true;
    }
    let Some(number) = stem
        .strip_prefix("COM")
        .or_else(|| stem.strip_prefix("LPT"))
    else {
        return false;
    };
    let mut chars = number.chars();
    matches!(
        (chars.next(), chars.next()),
        (Some('1'..='9' | '¹' | '²' | '³'), None)
    )
}

fn explain_create_error(error: &io::Error) -> Explanation {
    let (title, advice) = match error.kind() {
        io::ErrorKind::AlreadyExists => (
            "Esiste già una cartella con questo nome.",
            "Scegli un altro nome.",
        ),
        io::ErrorKind::NotFound => (
            "Non trovo la cartella in cui creare la nuova.",
            "Il disco è collegato?",
        ),
        io::ErrorKind::PermissionDenied => (
            "Non ho il permesso di creare una cartella qui.",
            "Scegli un altro posto o controlla i permessi.",
        ),
        io::ErrorKind::ReadOnlyFilesystem => (
            "Il disco è in sola lettura: non posso crearci cartelle.",
            "Scegli un altro disco.",
        ),
        _ => (
            "Non riesco a creare la cartella.",
            "Controlla che il disco o la rete siano collegati e riprova.",
        ),
    };
    Explanation {
        title: title.into(),
        advice: advice.into(),
    }
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

/// A folder (also a link to one) whose name does not start with a dot.
#[cfg(unix)]
fn is_visible_folder(entry: &DirEntry) -> bool {
    !entry.file_name().to_string_lossy().starts_with('.') && entry.path().is_dir()
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

    fn refused(name: &str) -> Explanation {
        let dir = tempfile::tempdir().unwrap();
        let error = create_folder(dir.path().to_str().unwrap(), name).unwrap_err();
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

        let path = create_folder(dir.path().to_str().unwrap(), "  Foto 2024  ").unwrap();

        assert_eq!(Path::new(&path), dir.path().join("Foto 2024"));
        assert!(dir.path().join("Foto 2024").is_dir());
    }

    #[test]
    fn a_missing_or_unusable_name_is_refused_before_touching_the_disk() {
        assert_eq!(refused("").title, "Scrivi un nome per la nuova cartella.");
        assert_eq!(
            refused("  \t ").title,
            "Scrivi un nome per la nuova cartella."
        );
        for name in [
            ".", "..", "Foto.", "Foto. ", "...", "CON", "con", "Nul.txt", "COM1", "com9.log",
            "LPT3", "COM¹", "CONIN$", "aux ",
        ] {
            assert!(
                refused(name)
                    .title
                    .starts_with("Questo nome non si può usare"),
                "{name:?}"
            );
        }
    }

    #[test]
    fn a_name_that_is_a_path_or_has_characters_windows_refuses_is_refused() {
        for name in [
            r"a\b", "a/b", r"..\evil", "../evil", "/abs", r"C:\x", "a:b", "a*b", "a?b", "a\"b",
            "a<b", "a>b", "a|b", "a\0b", "a\nb", "a\u{7f}b",
        ] {
            assert!(
                refused(name).title.starts_with("Il nome non può contenere"),
                "{name:?}"
            );
        }
    }

    #[test]
    fn dots_and_reserved_words_inside_a_longer_name_are_fine() {
        let dir = tempfile::tempdir().unwrap();
        let parent = dir.path().to_str().unwrap();
        for name in [
            ".nascosta",
            "Foto.2024",
            "CONTO",
            "COM10",
            "COM",
            "LPT0",
            "a b",
            "Città",
        ] {
            assert!(create_folder(parent, name).is_ok(), "{name:?}");
        }
    }

    #[test]
    fn a_name_longer_than_255_bytes_is_refused() {
        assert_eq!(refused(&"a".repeat(256)).title, "Il nome è troppo lungo.");
        let dir = tempfile::tempdir().unwrap();
        assert!(create_folder(dir.path().to_str().unwrap(), &"a".repeat(255)).is_ok());
    }

    #[test]
    fn an_existing_folder_or_file_is_never_reused() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("Foto")).unwrap();
        std::fs::write(dir.path().join("nota.txt"), "x").unwrap();
        let parent = dir.path().to_str().unwrap();

        for name in ["Foto", "nota.txt"] {
            let e = create_folder(parent, name).unwrap_err();
            assert_eq!(e.title, "Esiste già una cartella con questo nome.");
            assert_eq!(e.advice, "Scegli un altro nome.");
        }
    }

    #[test]
    fn a_parent_that_is_not_there_is_an_error_and_is_not_created() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("non-esiste");

        let e = create_folder(missing.to_str().unwrap(), "Foto").unwrap_err();

        assert_eq!(e.title, "Non trovo la cartella in cui creare la nuova.");
        assert_eq!(e.advice, "Il disco è collegato?");
        assert!(!missing.exists());
        assert!(create_folder("", "Foto").is_err());
    }

    #[test]
    fn the_os_error_decides_the_message() {
        let explain = |kind| explain_create_error(&std::io::Error::from(kind));
        assert_eq!(
            explain(std::io::ErrorKind::PermissionDenied).title,
            "Non ho il permesso di creare una cartella qui."
        );
        assert_eq!(
            explain(std::io::ErrorKind::ReadOnlyFilesystem).title,
            "Il disco è in sola lettura: non posso crearci cartelle."
        );
        assert_eq!(
            explain(std::io::ErrorKind::TimedOut).title,
            "Non riesco a creare la cartella."
        );
    }

    #[test]
    #[cfg(windows)]
    fn a_write_protected_windows_disk_is_read_only() {
        const ERROR_WRITE_PROTECT: i32 = 19;
        let e = explain_create_error(&std::io::Error::from_raw_os_error(ERROR_WRITE_PROTECT));
        assert_eq!(
            e.title,
            "Il disco è in sola lettura: non posso crearci cartelle."
        );
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
