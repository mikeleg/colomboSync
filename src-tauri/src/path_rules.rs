//! Rules on the two folders of a Sync. On Windows pure text: no filesystem access, so UNC paths
//! and disconnected drives are judged the same way (`D:\Foto`, `\\NAS\share`). On macOS and
//! Linux paths start with `/` and the links on them are resolved first, as far as the disk has
//! the folders: `/Volumes/Macintosh HD` is the root itself, whatever it is called.

pub const NOT_ABSOLUTE: &str = "Scrivi il percorso completo, che inizia con /.";
pub const SAME_FOLDER: &str = "La destinazione è la stessa cartella da copiare. Scegline un’altra.";
pub const INSIDE_SOURCE: &str = "La destinazione non può stare dentro la cartella da copiare.";
/// B above A: a mirror would purge everything else that lives in B.
pub const SOURCE_INSIDE_DESTINATION: &str =
    "La cartella da copiare non può stare dentro la destinazione.";

/// Ok when A and B are unrelated folders (neither equal, nor one inside the other); otherwise the
/// Italian message to show. Comparison ignores trailing and doubled separators and `.` and `..`
/// segments. On Windows it also ignores case, `/` vs `\` and the `\\?\` prefix; on macOS case
/// (its volumes usually do); on Linux case matters.
pub fn check_destination(source: &str, destination: &str) -> Result<(), &'static str> {
    // A relative path would mean the folder the app was started in: not what anyone typed.
    if !cfg!(windows) && !(is_absolute(source) && is_absolute(destination)) {
        return Err(NOT_ABSOLUTE);
    }
    let source = normalize(source);
    let destination = normalize(destination);
    if source == destination {
        Err(SAME_FOLDER)
    } else if is_inside(&destination, &source) {
        Err(INSIDE_SOURCE)
    } else if is_inside(&source, &destination) {
        Err(SOURCE_INSIDE_DESTINATION)
    } else {
        Ok(())
    }
}

fn is_inside(path: &str, folder: &str) -> bool {
    let separator = if cfg!(windows) { '\\' } else { '/' };
    path.starts_with(&format!("{folder}{separator}"))
}

fn is_absolute(path: &str) -> bool {
    path.trim().starts_with('/')
}

fn normalize(path: &str) -> String {
    if cfg!(windows) {
        normalize_windows(path)
    } else {
        normalize_unix(&resolve_links(path), cfg!(target_os = "macos"))
    }
}

/// The path with the links on it resolved: the deepest folder that exists is made canonical and
/// the rest, which is not there yet, is put back as written.
fn resolve_links(path: &str) -> String {
    let lexical = normalize_unix(path, false);
    let mut rest: Vec<&str> = Vec::new();
    let mut head = lexical.as_str();
    loop {
        let probe = if head.is_empty() { "/" } else { head };
        if let Ok(real) = std::fs::canonicalize(probe) {
            let mut resolved = real.to_string_lossy().into_owned();
            for part in rest.iter().rev() {
                resolved.push('/');
                resolved.push_str(part);
            }
            return resolved;
        }
        match head.rfind('/') {
            Some(at) => {
                rest.push(&head[at + 1..]);
                head = &head[..at];
            }
            None => return lexical,
        }
    }
}

/// `/`-rooted, no trailing `/`, `.`/`..` resolved without touching the disk. The root itself is
/// the empty string, so that `/Volumes` is inside it.
fn normalize_unix(path: &str, ignore_case: bool) -> String {
    let path = path.trim();
    let path = if ignore_case {
        path.to_lowercase()
    } else {
        path.to_string()
    };
    let mut parts: Vec<&str> = Vec::new();
    for part in path.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            part => parts.push(part),
        }
    }
    parts.iter().map(|part| format!("/{part}")).collect()
}

/// Lower-case, `\` separators, no trailing `\`, `.`/`..` resolved without touching the disk.
/// A drive (`d:`) or a UNC `\\server\share` is the root that `..` cannot climb above.
pub(crate) fn normalize_windows(path: &str) -> String {
    let mut path = path.trim().replace('/', "\\").to_lowercase();
    if let Some(rest) = path.strip_prefix(r"\\?\unc\") {
        path = format!(r"\\{rest}");
    } else if let Some(rest) = path.strip_prefix(r"\\?\") {
        path = rest.to_string();
    }
    let unc = path.starts_with(r"\\");
    let root_parts = if unc { 2 } else { 1 };
    let mut parts: Vec<&str> = Vec::new();
    for part in path.split('\\') {
        match part {
            "" | "." => {}
            ".." => {
                if parts.len() > root_parts {
                    parts.pop();
                }
            }
            part => parts.push(part),
        }
    }
    let joined = parts.join("\\");
    if unc { format!(r"\\{joined}") } else { joined }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(windows)]
    fn destination_equal_to_source_is_rejected() {
        for (a, b) in [
            (r"D:\Foto", r"D:\Foto"),
            (r"D:\Foto", r"d:\FOTO\"),
            (r"D:\Foto\\", r"D:/Foto"),
            (r"D:\", r"d:"),
            (r"\\NAS\Share\Foto", r"\\nas\SHARE\foto\"),
            (r"D:\Foto", r"D:\Foto\."),
            (r"D:\Foto", r"D:\Altro\..\Foto"),
            (r"D:\Foto", r"\\?\D:\Foto"),
            (r"\\NAS\Share\Foto", r"\\?\UNC\NAS\Share\Foto"),
        ] {
            assert_eq!(check_destination(a, b), Err(SAME_FOLDER), "{a} -> {b}");
        }
    }

    #[test]
    #[cfg(windows)]
    fn destination_inside_source_is_rejected() {
        for (a, b) in [
            (r"D:\Foto", r"D:\Foto\Backup"),
            (r"D:\Foto\", r"d:\foto\2024\Copia\"),
            (r"D:\", r"D:\Backup"),
            (r"\\NAS\Share\Foto", r"\\nas\share\FOTO\sub"),
            (r"D:\Foto", r"D:\Foto\Sub\..\Backup"),
            (r"D:\Foto", r"\\?\D:\Foto\Backup"),
        ] {
            assert_eq!(check_destination(a, b), Err(INSIDE_SOURCE), "{a} -> {b}");
        }
    }

    #[test]
    #[cfg(windows)]
    fn source_inside_destination_is_rejected() {
        for (a, b) in [
            (r"D:\Foto\Vacanze", r"D:\Foto"),
            (r"D:\Foto", r"D:\"),
            (r"D:\Foto", r"d:"),
            (r"\\NAS\Share\Foto\2024", r"\\nas\share\foto\"),
            (r"D:\Foto\Vacanze", r"D:\Foto\Altro\.."),
        ] {
            assert_eq!(
                check_destination(a, b),
                Err(SOURCE_INSIDE_DESTINATION),
                "{a} -> {b}"
            );
        }
    }

    #[test]
    #[cfg(windows)]
    fn unrelated_folders_are_accepted() {
        for (a, b) in [
            (r"D:\Foto", r"E:\Backup"),
            (r"D:\Foto", r"D:\Foto2"),
            (r"D:\Foto2", r"D:\Foto"),
            (r"D:\Foto", r"D:\Foto\..\Altro"),
            (r"\\NAS\Share\Foto", r"\\NAS\Share\Backup"),
            (r"\\NAS\Share", r"\\NAS2\Share"),
        ] {
            assert_eq!(check_destination(a, b), Ok(()), "{a} -> {b}");
        }
    }

    #[test]
    fn unix_paths_are_normalized_without_touching_the_disk() {
        for (path, folded, exact) in [
            ("/Users/Anna/Foto", "/users/anna/foto", "/Users/Anna/Foto"),
            ("/Users/Anna/Foto/", "/users/anna/foto", "/Users/Anna/Foto"),
            (
                "//Users///Anna/./Foto",
                "/users/anna/foto",
                "/Users/Anna/Foto",
            ),
            (
                "/Users/Anna/x/../Foto",
                "/users/anna/foto",
                "/Users/Anna/Foto",
            ),
            ("/../Users", "/users", "/Users"),
            (" /Users ", "/users", "/Users"),
            ("/", "", ""),
        ] {
            assert_eq!(normalize_unix(path, true), folded, "{path}");
            assert_eq!(normalize_unix(path, false), exact, "{path}");
        }
    }

    #[test]
    #[cfg(unix)]
    fn unix_destination_rules() {
        assert_eq!(
            check_destination("/Users/anna/Foto", "/Users/anna/Foto/"),
            Err(SAME_FOLDER)
        );
        assert_eq!(
            check_destination("/Users/anna/Foto", "/Users/anna/Foto/Backup"),
            Err(INSIDE_SOURCE)
        );
        assert_eq!(
            check_destination("/Users/anna/Foto/2024", "/Users/anna/Foto"),
            Err(SOURCE_INSIDE_DESTINATION)
        );
        assert_eq!(
            check_destination("/Users/anna/Foto", "/"),
            Err(SOURCE_INSIDE_DESTINATION)
        );
        assert_eq!(
            check_destination("/", "/Volumes/Backup"),
            Err(INSIDE_SOURCE)
        );
        for (a, b) in [
            ("/Users/anna/Foto", "/Volumes/Backup"),
            ("/Users/anna/Foto", "/Users/anna/Foto2"),
            ("/Users/anna/Foto2", "/Users/anna/Foto"),
        ] {
            assert_eq!(check_destination(a, b), Ok(()), "{a} -> {b}");
        }
    }

    #[test]
    #[cfg(unix)]
    fn a_path_that_does_not_start_with_a_slash_is_refused() {
        for (a, b) in [
            ("Foto", "/Volumes/Backup"),
            ("/Users/anna/Foto", "Backup"),
            ("~/Foto", "/Volumes/Backup"),
            ("", "/Volumes/Backup"),
            (r"D:\Foto", "/Volumes/Backup"),
        ] {
            assert_eq!(check_destination(a, b), Err(NOT_ABSOLUTE), "{a} -> {b}");
        }
        assert_eq!(
            NOT_ABSOLUTE,
            "Scrivi il percorso completo, che inizia con /."
        );
        assert_eq!(check_destination(" /a/x ", "/b"), Ok(()));
    }

    #[test]
    #[cfg(unix)]
    fn a_link_to_a_folder_cannot_hide_that_it_is_the_same_folder() {
        use std::os::unix::fs::symlink;
        let temp = tempfile::tempdir().unwrap();
        let real = temp.path().join("vera");
        std::fs::create_dir_all(real.join("dentro")).unwrap();
        let alias = temp.path().join("scorciatoia");
        symlink(&real, &alias).unwrap();
        let (real, alias) = (real.to_str().unwrap(), alias.to_str().unwrap());

        assert_eq!(check_destination(real, alias), Err(SAME_FOLDER));
        assert_eq!(check_destination(alias, real), Err(SAME_FOLDER));
        let inside = format!("{alias}/dentro");
        assert_eq!(check_destination(real, &inside), Err(INSIDE_SOURCE));
        let not_yet = format!("{alias}/nuova/cartella");
        assert_eq!(check_destination(real, &not_yet), Err(INSIDE_SOURCE));
        assert_eq!(
            check_destination(&inside, alias),
            Err(SOURCE_INSIDE_DESTINATION)
        );
        let sibling = temp.path().join("altra");
        assert_eq!(check_destination(alias, sibling.to_str().unwrap()), Ok(()));
    }

    #[test]
    #[cfg(unix)]
    fn an_alias_of_the_root_is_the_root() {
        // `/Volumes/Macintosh HD` on a Mac is a link to `/`.
        let temp = tempfile::tempdir().unwrap();
        let boot = temp.path().join("Macintosh HD");
        std::os::unix::fs::symlink("/", &boot).unwrap();
        let folder = temp.path().join("Foto");
        std::fs::create_dir(&folder).unwrap();
        let (boot, folder) = (boot.to_str().unwrap(), folder.to_str().unwrap());

        assert_eq!(
            check_destination(folder, boot),
            Err(SOURCE_INSIDE_DESTINATION)
        );
        assert_eq!(check_destination("/", boot), Err(SAME_FOLDER));
        assert_eq!(
            check_destination(&format!("{boot}{folder}"), folder),
            Err(SAME_FOLDER)
        );
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn on_macos_case_does_not_make_a_folder_another() {
        assert_eq!(
            check_destination("/Users/anna/Foto", "/users/ANNA/foto"),
            Err(SAME_FOLDER)
        );
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn on_linux_case_does_make_a_folder_another() {
        assert_eq!(
            check_destination("/home/anna/Foto", "/home/anna/foto"),
            Ok(())
        );
    }
}
