//! Why a copy failed, in plain Italian. The cause is read from the Windows error code in the log
//! (`ERROR 53 (0x00000035)`), never from the labels around it: those are localized.

/// What the user reads about a failed Run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Explanation {
    /// The result screen: "Non completato: <what happened>. <what to do>".
    pub message: String,
    /// The note on the home card of the sync.
    pub note: String,
}

pub const STOPPED_NOTE: &str = "La copia era stata fermata a mano: puoi riprenderla quando vuoi.";
pub const GENERIC_NOTE: &str = "L’ultima copia non è stata completata. Nel registro trovi i dettagli.";
const GENERIC_MESSAGE: &str =
    "Non completato: la copia si è fermata per un problema. Apri il registro per vedere i dettagli.";

/// Explains the first mapped error in a robocopy log. `source` (A) and `destination` (B) tell
/// which folder the error is about.
pub fn explain(log: &str, source: &str, destination: &str) -> Explanation {
    let Some((code, line)) = first_mapped_error(log) else {
        return Explanation {
            message: GENERIC_MESSAGE.into(),
            note: GENERIC_NOTE.into(),
        };
    };
    let folder = folder_of(line, source, destination);
    match code {
        53 | 67 => {
            let (lower, upper) = unreachable_name(folder.path);
            Explanation {
                message: format!(
                    "Non completato: {lower} non è raggiungibile. Controlla la connessione e riprova."
                ),
                note: format!("{upper} non era raggiungibile. Controlla che sia collegato e riprova."),
            }
        }
        2 | 3 => from_sentence(missing_folder(folder.name, folder.path)),
        _ => from_sentence(format!(
            "Non ho il permesso di usare la cartella {} ({}). Controlla i permessi e riprova.",
            folder.name, folder.path
        )),
    }
}

/// Preview of a sync whose A is not there (typically an unplugged disk).
pub fn missing_source(source: &str) -> String {
    missing_folder('A', source)
}

fn missing_folder(name: char, path: &str) -> String {
    let question = if is_network(path) {
        "La cartella di rete è raggiungibile?"
    } else {
        "Il disco è collegato?"
    };
    format!("Non trovo la cartella {name} ({path}). {question}")
}

/// The sentence is the home note as it is, and the result message once it follows "Non completato:".
fn from_sentence(sentence: String) -> Explanation {
    let mut chars = sentence.chars();
    let lowered: String = chars
        .next()
        .map(|c| c.to_lowercase().chain(chars).collect())
        .unwrap_or_default();
    Explanation {
        message: format!("Non completato: {lowered}"),
        note: sentence,
    }
}

const MAPPED_CODES: [u32; 5] = [2, 3, 5, 53, 67];

/// First log line holding `<n> (0x<hex>)` with a mapped `n`, and that `n`. Any label before
/// the number (`ERROR`, `ERRORE`…) is ignored.
fn first_mapped_error(log: &str) -> Option<(u32, &str)> {
    log.lines().find_map(|line| {
        let tokens: Vec<&str> = line.split_whitespace().collect();
        tokens.windows(2).find_map(|pair| {
            let hex = pair[1].strip_prefix("(0x")?.strip_suffix(')')?;
            let code: u32 = pair[0].parse().ok()?;
            let is_hex = !hex.is_empty() && hex.chars().all(|c| c.is_ascii_hexdigit());
            (is_hex && MAPPED_CODES.contains(&code)).then_some((code, line))
        })
    })
}

struct Folder<'a> {
    name: char,
    path: &'a str,
}

/// The folder an error line talks about: the one whose path the line mentions. A line that
/// names neither (the path is cut, or relative) falls back to the folder most likely to be on
/// the network: B if it is a share, else A if it is, else B.
fn folder_of<'a>(line: &str, source: &'a str, destination: &'a str) -> Folder<'a> {
    let (a, b) = (
        Folder {
            name: 'A',
            path: source.trim(),
        },
        Folder {
            name: 'B',
            path: destination.trim(),
        },
    );
    let line = line.replace('/', "\\").to_lowercase();
    match (mentions(&line, a.path), mentions(&line, b.path)) {
        (true, false) => a,
        (false, true) => b,
        _ if is_network(a.path) && !is_network(b.path) => a,
        _ => b,
    }
}

/// `line` (lower case, `\` separators) holds `path` as a whole folder name: not `D:\Foto2` for `D:\Foto`.
fn mentions(line: &str, path: &str) -> bool {
    let path = path.replace('/', "\\").trim_end_matches('\\').to_lowercase();
    !path.is_empty()
        && line.match_indices(&path).any(|(at, _)| {
            line[at + path.len()..]
                .chars()
                .next()
                .is_none_or(|c| c == '\\' || c.is_whitespace() || c == '"')
        })
}

fn is_network(path: &str) -> bool {
    path.starts_with(r"\\")
}

/// "la cartella di rete \\NAS" or "il disco D:", lower case and capitalized.
fn unreachable_name(path: &str) -> (String, String) {
    if is_network(path) {
        let server = path.trim_start_matches('\\').split('\\').next().unwrap_or("");
        let tag = format!(r"\\{server}");
        (
            format!("la cartella di rete {tag}"),
            format!("La cartella di rete {tag}"),
        )
    } else {
        let tag: String = path.chars().take(2).collect::<String>().to_uppercase();
        (format!("il disco {tag}"), format!("Il disco {tag}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const A: &str = r"D:\Foto";
    const NAS_B: &str = r"\\NAS\Backup";

    /// A real log: label wording changes with the Windows language, the code does not.
    fn log(code: u32, label: &str, what: &str, path: &str) -> String {
        format!(
            "\r\n2026/11/04 11:58:26 {label} {code} (0x{code:08X}) {what} {path}\r\nIl messaggio di Windows, in qualunque lingua.\r\n"
        )
    }

    fn english(code: u32, path: &str) -> String {
        log(code, "ERROR", "Accessing Destination Directory", path)
    }

    fn italian(code: u32, path: &str) -> String {
        log(code, "ERRORE", "Accesso alla directory di destinazione", path)
    }

    #[test]
    fn error_53_is_an_unreachable_share() {
        for text in [
            english(53, r"\\NAS\Backup\"),
            italian(53, r"\\NAS\Backup\"),
        ] {
            let e = explain(&text, A, NAS_B);
            assert_eq!(
                e.message,
                r"Non completato: la cartella di rete \\NAS non è raggiungibile. Controlla la connessione e riprova."
            );
            assert_eq!(
                e.note,
                r"La cartella di rete \\NAS non era raggiungibile. Controlla che sia collegato e riprova."
            );
        }
    }

    #[test]
    fn error_67_is_an_unreachable_share_too() {
        let e = explain(&italian(67, r"\\NAS\Backup\x\"), A, NAS_B);
        assert_eq!(
            e.message,
            r"Non completato: la cartella di rete \\NAS non è raggiungibile. Controlla la connessione e riprova."
        );
    }

    #[test]
    fn an_unreachable_drive_letter_says_disco() {
        let e = explain(&english(53, r"Z:\Backup\"), A, r"Z:\Backup");
        assert_eq!(
            e.message,
            "Non completato: il disco Z: non è raggiungibile. Controlla la connessione e riprova."
        );
        assert_eq!(
            e.note,
            "Il disco Z: non era raggiungibile. Controlla che sia collegato e riprova."
        );
    }

    #[test]
    fn error_2_and_3_say_which_folder_cannot_be_found() {
        for code in [2, 3] {
            for text in [
                log(code, "ERROR", "Accessing Source Directory", r"D:\Foto\"),
                log(code, "ERRORE", "Accesso alla directory di origine", r"D:\Foto\"),
            ] {
                let e = explain(&text, A, r"E:\Backup");
                assert_eq!(
                    e.message,
                    r"Non completato: non trovo la cartella A (D:\Foto). Il disco è collegato?"
                );
                assert_eq!(e.note, r"Non trovo la cartella A (D:\Foto). Il disco è collegato?");
            }
        }
    }

    #[test]
    fn error_3_on_a_network_folder_asks_about_the_network() {
        let e = explain(&english(3, r"\\NAS\Backup\"), A, NAS_B);
        assert_eq!(
            e.message,
            r"Non completato: non trovo la cartella B (\\NAS\Backup). La cartella di rete è raggiungibile?"
        );
    }

    #[test]
    fn error_5_says_access_was_denied_on_that_folder() {
        for text in [
            log(5, "ERROR", "Copying File", r"E:\Backup\2024\a.jpg"),
            log(5, "ERRORE", "Copia file", r"E:\Backup\2024\a.jpg"),
        ] {
            let e = explain(&text, A, r"E:\Backup");
            assert_eq!(
                e.message,
                r"Non completato: non ho il permesso di usare la cartella B (E:\Backup). Controlla i permessi e riprova."
            );
            assert_eq!(
                e.note,
                r"Non ho il permesso di usare la cartella B (E:\Backup). Controlla i permessi e riprova."
            );
        }
    }

    #[test]
    fn any_other_code_or_no_code_is_the_generic_message() {
        for text in [
            english(32, r"E:\Backup\a.jpg"), // sharing violation: not mapped
            "robocopy ha detto qualcosa senza codice\r\n".to_string(),
            String::new(),
        ] {
            let e = explain(&text, A, r"E:\Backup");
            assert_eq!(e.message, GENERIC_MESSAGE);
            assert_eq!(e.note, GENERIC_NOTE);
        }
    }

    #[test]
    fn the_folder_is_found_by_path_not_by_a_folder_with_the_same_start() {
        // `D:\Foto2` is a different folder than A (`D:\Foto`): the error is about B.
        let e = explain(&english(5, r"D:\Foto2\a.jpg"), A, r"D:\Foto2");
        assert!(e.message.contains(r"cartella B (D:\Foto2)"), "{}", e.message);
        let e = explain(&english(5, r"d:/foto/a.jpg"), A, r"D:\Foto2");
        assert!(e.message.contains(r"cartella A (D:\Foto)"), "{}", e.message);
    }

    #[test]
    fn a_line_without_a_known_path_blames_the_share_then_b() {
        let line = english(53, r"\\ALTRO\x\");
        assert!(explain(&line, A, NAS_B).message.contains(r"\\NAS"));
        assert!(explain(&line, NAS_B, r"E:\Backup").message.contains(r"\\NAS"));
        assert!(explain(&english(3, "x"), A, r"E:\Backup").message.contains("cartella B"));
    }

    #[test]
    fn the_first_mapped_error_wins() {
        let text = english(32, r"E:\a") + &english(5, r"E:\Backup\b") + &english(53, NAS_B);
        assert!(explain(&text, A, r"E:\Backup").message.contains("permesso"));
    }

    #[test]
    fn a_missing_source_asks_about_the_disk_or_the_network() {
        assert_eq!(
            missing_source(r"D:\Foto"),
            r"Non trovo la cartella A (D:\Foto). Il disco è collegato?"
        );
        assert_eq!(
            missing_source(r"\\NAS\Foto"),
            r"Non trovo la cartella A (\\NAS\Foto). La cartella di rete è raggiungibile?"
        );
    }
}
