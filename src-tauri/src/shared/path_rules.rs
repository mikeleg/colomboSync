//! Rules on the two folders of a Sync. On Windows pure text: no filesystem access, so UNC paths
//! and disconnected drives are judged the same way (`D:\Foto`, `\\NAS\share`). On macOS and
//! Linux paths start with `/` and the links on them are resolved first, as far as the disk has
//! the folders: `/Volumes/Macintosh HD` is the root itself, whatever it is called.

use super::error_messages::Explanation;
use std::borrow::Cow;

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
    let path = path.trim().replace('/', "\\");
    let path = strip_verbatim(&path).to_lowercase();
    let unc = is_unc(&path);
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

/// The verbatim prefixes of a path, in their lower-case spelling: `\\?\UNC\x` is `\\x` and
/// `\\?\D:\x` is `D:\x`. `strip_verbatim` matches them in any case; `robocopy.rs` compares them
/// after lower-casing a log line.
pub(crate) const VERBATIM_UNC: &str = r"\\?\unc\";
pub(crate) const VERBATIM: &str = r"\\?\";

/// The path without its verbatim prefix, typed in any case: `\\?\UNC\NAS\x` is `\\NAS\x` and
/// `\\?\D:\x` is `D:\x`. Only at the start; the case of the rest is kept. Any other verbatim path
/// (`\\?\Volume{…}\x`, `\\?\GLOBALROOT\…`) stays as written: it is no drive and no share we can name.
pub(crate) fn strip_verbatim(path: &str) -> Cow<'_, str> {
    let starts = |prefix: &str| {
        path.get(..prefix.len())
            .is_some_and(|start| start.eq_ignore_ascii_case(prefix))
    };
    if starts(VERBATIM_UNC) {
        Cow::Owned(format!(r"\\{}", &path[VERBATIM_UNC.len()..]))
    } else if starts(VERBATIM)
        && matches!(
            path.as_bytes()[VERBATIM.len()..],
            [letter, b':', ..] if letter.is_ascii_alphabetic()
        )
    {
        Cow::Borrowed(&path[VERBATIM.len()..])
    } else {
        Cow::Borrowed(path)
    }
}

/// A network share: `\\server\share`.
pub(crate) fn is_unc(path: &str) -> bool {
    strip_verbatim(path).starts_with(r"\\")
}

/// Characters Windows does not allow in a file or folder name (also controls), whatever the OS.
pub fn is_forbidden_in_name(c: char) -> bool {
    c.is_control() || "<>:\"/\\|?*".contains(c)
}

/// The trimmed name when it is something Windows accepts as a single folder name.
pub fn checked_name(name: &str) -> Result<&str, Explanation> {
    let name = name.trim();
    if name.is_empty() {
        return Err("Scrivi un nome per la nuova cartella.".into());
    }
    if name_length(name) > 255 {
        return Err("Il nome è troppo lungo.".into());
    }
    if name.chars().any(is_forbidden_in_name) {
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

/// The length the file system limits to 255: UTF-16 units on NTFS, bytes on ext4 and APFS.
fn name_length(name: &str) -> usize {
    if cfg!(windows) {
        name.encode_utf16().count()
    } else {
        name.len()
    }
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

/// One pair of folders per refusal, with its message: for the tests of whatever calls
/// `check_destination`.
#[cfg(test)]
pub(crate) fn refused_pairs() -> [(&'static str, &'static str, &'static str); 3] {
    if cfg!(windows) {
        [
            (r"D:\Foto", r"d:\foto\", SAME_FOLDER),
            (r"D:\Foto", r"D:\Foto\Backup", INSIDE_SOURCE),
            (r"D:\Foto\2024", r"D:\Foto", SOURCE_INSIDE_DESTINATION),
        ]
    } else {
        [
            ("/home/anna/Foto", "/home/anna/Foto/", SAME_FOLDER),
            ("/home/anna/Foto", "/home/anna/Foto/Backup", INSIDE_SOURCE),
            (
                "/home/anna/Foto/2024",
                "/home/anna/Foto",
                SOURCE_INSIDE_DESTINATION,
            ),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn refused(name: &str) -> Explanation {
        checked_name(name).unwrap_err()
    }

    #[test]
    fn normalize_windows_strips_a_verbatim_prefix_only_at_the_start() {
        assert_eq!(normalize_windows(r"\\?\D:\Foto"), r"d:\foto");
        assert_eq!(normalize_windows(r"\\?\UNC\NAS\Backup"), r"\\nas\backup");
        // In the middle it is no prefix: the empty part goes, the rest stays (as before).
        assert_eq!(normalize_windows(r"D:\a\\?\b"), r"d:\a\?\b");
    }

    #[test]
    fn strip_verbatim_removes_the_prefix_only_at_the_start() {
        assert_eq!(strip_verbatim(r"\\?\UNC\NAS\x"), r"\\NAS\x");
        assert_eq!(strip_verbatim(r"\\?\D:\Foto"), r"D:\Foto");
        assert_eq!(strip_verbatim(r"D:\a\\?\b"), r"D:\a\\?\b");
        assert_eq!(strip_verbatim(r"\\?\unc\NAS"), r"\\NAS");
    }

    #[test]
    fn a_verbatim_local_path_is_not_a_share_and_a_verbatim_unc_one_is() {
        assert!(!is_unc(r"\\?\D:\Foto"));
        assert!(is_unc(r"\\?\UNC\NAS\x"));
        assert!(is_unc(r"\\NAS\x"));
    }

    #[test]
    fn a_volume_or_device_verbatim_path_stays_as_written_and_counts_as_a_share_as_before() {
        for path in [
            r"\\?\Volume{1234-abcd}\Backup",
            r"\\?\GLOBALROOT\Device\X\y",
            r"\\?\Dx:\y",
        ] {
            assert_eq!(strip_verbatim(path), path);
            assert!(is_unc(path), "{path}");
        }
    }

    #[test]
    #[cfg(windows)]
    fn a_volume_path_written_the_same_way_on_both_sides_is_still_compared() {
        let a = r"\\?\Volume{1234-abcd}\Foto";
        assert_eq!(check_destination(a, a), Err(SAME_FOLDER));
        assert_eq!(
            check_destination(a, r"\\?\Volume{1234-abcd}\Foto\x"),
            Err(INSIDE_SOURCE)
        );
        assert_eq!(
            check_destination(r"\\?\Volume{1234-abcd}\Foto\x", a),
            Err(SOURCE_INSIDE_DESTINATION)
        );
        assert_eq!(
            check_destination(a, r"\\?\Volume{1234-abcd}\Foto\..\..\Altro"),
            Ok(())
        );
    }

    #[test]
    fn a_missing_or_unusable_name_is_refused() {
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
            assert_eq!(checked_name(name), Ok(name));
        }
        assert_eq!(checked_name("  Foto 2024  "), Ok("Foto 2024"));
    }

    #[test]
    fn an_ascii_name_longer_than_255_is_refused() {
        assert_eq!(refused(&"a".repeat(256)).title, "Il nome è troppo lungo.");
        assert!(checked_name(&"a".repeat(255)).is_ok());
    }

    /// NTFS counts UTF-16 units (200 here), ext4 and APFS count bytes (260 here).
    #[test]
    fn the_name_limit_is_the_one_of_the_file_system() {
        let accented = format!("{}{}", "a".repeat(140), "è".repeat(60));
        assert_eq!(accented.chars().count(), 200);

        assert_eq!(checked_name(&accented).is_ok(), cfg!(windows));
    }

    #[test]
    #[cfg(windows)]
    fn a_name_of_more_than_255_utf16_units_is_refused_on_windows() {
        let astral = "😀".repeat(128); // 2 units each
        assert_eq!(refused(&astral).title, "Il nome è troppo lungo.");
        assert!(checked_name(&"😀".repeat(127)).is_ok());
    }

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
