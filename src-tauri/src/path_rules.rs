//! Rules on the two folders of a Sync. Pure text: no filesystem access, so UNC paths and
//! disconnected drives are judged the same way.

pub const SAME_FOLDER: &str = "La destinazione è la stessa cartella da copiare. Scegline un’altra.";
pub const INSIDE_SOURCE: &str = "La destinazione non può stare dentro la cartella da copiare.";

/// Ok when B is a folder other than A and not inside it; otherwise the Italian message to show.
/// Comparison ignores case, `/` vs `\` and trailing slashes.
pub fn check_destination(source: &str, destination: &str) -> Result<(), &'static str> {
    let source = normalize(source);
    let destination = normalize(destination);
    if source == destination {
        Err(SAME_FOLDER)
    } else if destination.starts_with(&format!("{source}\\")) {
        Err(INSIDE_SOURCE)
    } else {
        Ok(())
    }
}

fn normalize(path: &str) -> String {
    path.trim()
        .replace('/', "\\")
        .trim_end_matches('\\')
        .to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn destination_equal_to_source_is_rejected() {
        for (a, b) in [
            (r"D:\Foto", r"D:\Foto"),
            (r"D:\Foto", r"d:\FOTO\"),
            (r"D:\Foto\\", r"D:/Foto"),
            (r"D:\", r"d:"),
            (r"\\NAS\Share\Foto", r"\\nas\SHARE\foto\"),
        ] {
            assert_eq!(check_destination(a, b), Err(SAME_FOLDER), "{a} -> {b}");
        }
    }

    #[test]
    fn destination_inside_source_is_rejected() {
        for (a, b) in [
            (r"D:\Foto", r"D:\Foto\Backup"),
            (r"D:\Foto\", r"d:\foto\2024\Copia\"),
            (r"D:\", r"D:\Backup"),
            (r"\\NAS\Share\Foto", r"\\nas\share\FOTO\sub"),
        ] {
            assert_eq!(check_destination(a, b), Err(INSIDE_SOURCE), "{a} -> {b}");
        }
    }

    #[test]
    fn other_destinations_are_accepted() {
        for (a, b) in [
            (r"D:\Foto", r"E:\Backup"),
            (r"D:\Foto", r"D:\Foto2"),
            (r"D:\Foto\Vacanze", r"D:\Foto"),
            (r"\\NAS\Share\Foto", r"\\NAS\Share\Backup"),
            (r"\\NAS\Share", r"\\NAS2\Share"),
        ] {
            assert_eq!(check_destination(a, b), Ok(()), "{a} -> {b}");
        }
    }
}
