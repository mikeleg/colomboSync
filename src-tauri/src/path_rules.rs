//! Rules on the two folders of a Sync. Pure text: no filesystem access, so UNC paths and
//! disconnected drives are judged the same way. Windows paths are `D:\Foto` or `\\NAS\share`;
//! macOS and Linux paths start with `/`.

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

fn normalize(path: &str) -> String {
    if cfg!(windows) {
        normalize_windows(path)
    } else {
        normalize_unix(path, cfg!(target_os = "macos"))
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
