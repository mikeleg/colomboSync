// Demo nel browser con il backend Rust simulato (mockIPC), per screenshot e controlli visivi. Avvio: npx vite --port 5199 --strictPort, poi http://localhost:5199/demo.html.
import { emit } from "@tauri-apps/api/event";
import { mockIPC, mockWindows } from "@tauri-apps/api/mocks";
import type { Counts, Explanation, Place, Run, RunLog, RunReport, Sync, SyncMode } from "./shared/types";
import type { FolderListing, Preview } from "./wizard/types";
import type { RunProgress } from "./run/types";
import type { HistorySummary, RunList, SyncStatus } from "./history/types";
import type { ListedSyncs } from "./syncs/types";

const MINUTE = 60_000;
const HOUR = 60 * MINUTE;
const DAY = 24 * HOUR;
const ago = (ms: number) => new Date(Date.now() - ms).toISOString();
const logPath = (name: string) => "C:\\Users\\Maria\\AppData\\Local\\ColomboSync\\logs\\" + name + ".log";
const wait = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));

const FOTO_FROM = "C:\\Users\\Maria\\Immagini\\Foto";
const FOTO_TO = "E:\\Backup\\Foto";
const LAVORO_FROM = "C:\\Users\\Maria\\Documenti\\Lavoro";
const LAVORO_TO = "\\\\NAS\\Archivio\\Lavoro";
const MUSICA_FROM = "D:\\Musica";
const MUSICA_TO = "E:\\Backup\\Musica";

const syncs: Sync[] = [
  { id: 1, name: "Foto di famiglia", source: FOTO_FROM, destination: FOTO_TO, mode: "Copy" },
  { id: 2, name: "Documenti di lavoro", source: LAVORO_FROM, destination: LAVORO_TO, mode: "Mirror" },
  { id: 3, name: "Musica", source: MUSICA_FROM, destination: MUSICA_TO, mode: "Copy" },
];
let nextId = 4;

// Rust words the texts of a run (shared/copy_text.rs); here a small imitation of it.
const num = (n: number) => n.toLocaleString("it-IT");
const whenText = (startedAt: string) =>
  `${startedAt.slice(8, 10)}/${startedAt.slice(5, 7)}/${startedAt.slice(0, 4)} ${startedAt.slice(11, 16)}`;
const durationText = (ms: number) => (Math.round(ms / MINUTE) < 1 ? "< 1 min" : `${Math.round(ms / MINUTE)} min`);
const partialText = (n: number) => `${num(n)} ${n === 1 ? "file copiato" : "file copiati"} (parziale)`;

/** What kind of place a path is, as `describe_place` says it. */
function placeOf(path: string): Place {
  if (path.startsWith("\\\\")) return { isNetwork: true, label: `Rete · ${path.slice(2).split("\\")[0]}` };
  return { isNetwork: false, label: /^[A-Za-z]:/.test(path) ? `Disco ${path.slice(0, 2).toUpperCase()}` : "Cartella" };
}

/** A run as `run_sync` and `list_runs` send it: the stored fields, the explanation and the finished texts. */
function toReport(run: Run & { explanation: Explanation | null }): RunReport {
  const noneCopied = "Nessun file è stato copiato.";
  const resultNote =
    run.partial === undefined
      ? null
      : run.partial === 0
        ? noneCopied
        : run.outcome === "Cancelled"
          ? `${partialText(run.partial)}. I file già copiati restano in B.`
          : partialText(run.partial);
  const deleted =
    run.mode === "Mirror" && run.deleted > 0
      ? `, ${num(run.deleted)} ${run.deleted === 1 ? "cancellato" : "cancellati"} da B`
      : "";
  const title =
    run.outcome === "Ok"
      ? `Fatto: ${num(run.new)} ${run.new === 1 ? "file copiato" : "file copiati"}, ${num(run.updated)} ${run.updated === 1 ? "aggiornato" : "aggiornati"}${deleted} in ${durationText(run.durationMs)}`
      : run.outcome === "Cancelled"
        ? "Annullato: ho fermato la copia."
        : (run.explanation?.title ?? "Non completato");
  return {
    ...run,
    title,
    caption: run.outcome === "Ok" ? "Fatto." : run.outcome === "Cancelled" ? "Annullato." : "Non completato.",
    resultNote,
    partialText: run.partial === undefined ? null : partialText(run.partial),
    when: whenText(run.startedAt),
    durationText: durationText(run.durationMs),
  };
}

const seeds: (Run & { explanation: Explanation | null })[] = [
  {
    syncName: "Foto di famiglia",
    source: FOTO_FROM,
    destination: FOTO_TO,
    mode: "Copy",
    new: 42,
    updated: 11,
    deleted: 0,
    durationMs: 128_000,
    outcome: "Ok",
    startedAt: ago(3 * HOUR),
    logPath: logPath("foto-1"),
    explanation: null,
  },
  {
    syncName: "Documenti di lavoro",
    source: LAVORO_FROM,
    destination: LAVORO_TO,
    mode: "Mirror",
    new: 3,
    updated: 0,
    deleted: 0,
    durationMs: 41_000,
    outcome: "Error",
    startedAt: ago(2 * HOUR),
    logPath: logPath("lavoro-1"),
    partial: 2,
    explanation: {
      title: "Non completato: la cartella di rete \\\\NAS non è raggiungibile.",
      advice: "Controlla la connessione e riprova.",
    },
  },
  {
    syncName: "Musica",
    source: MUSICA_FROM,
    destination: MUSICA_TO,
    mode: "Copy",
    new: 118,
    updated: 0,
    deleted: 0,
    durationMs: 305_000,
    outcome: "Ok",
    startedAt: ago(DAY + HOUR),
    logPath: logPath("musica-1"),
    explanation: null,
  },
  {
    syncName: "Documenti di lavoro",
    source: LAVORO_FROM,
    destination: LAVORO_TO,
    mode: "Mirror",
    new: 6,
    updated: 9,
    deleted: 2,
    durationMs: 96_000,
    outcome: "Ok",
    startedAt: ago(DAY + 5 * HOUR),
    logPath: logPath("lavoro-2"),
    explanation: null,
  },
  {
    syncName: "Foto di famiglia",
    source: FOTO_FROM,
    destination: FOTO_TO,
    mode: "Copy",
    new: 60,
    updated: 0,
    deleted: 0,
    durationMs: 54_000,
    outcome: "Cancelled",
    startedAt: ago(2 * DAY + 4 * HOUR),
    logPath: logPath("foto-2"),
    partial: 37,
    explanation: null,
  },
  {
    syncName: "Foto di famiglia",
    source: FOTO_FROM,
    destination: FOTO_TO,
    mode: "Copy",
    new: 15,
    updated: 4,
    deleted: 0,
    durationMs: 88_000,
    outcome: "Ok",
    startedAt: ago(3 * DAY + 2 * HOUR),
    logPath: logPath("foto-3"),
    explanation: null,
  },
  {
    syncName: "Musica",
    source: MUSICA_FROM,
    destination: MUSICA_TO,
    mode: "Copy",
    new: 0,
    updated: 0,
    deleted: 0,
    durationMs: 4_200,
    outcome: "Ok",
    startedAt: ago(4 * DAY + 7 * HOUR),
    logPath: logPath("musica-2"),
    explanation: null,
  },
  {
    syncName: "Documenti di lavoro",
    source: LAVORO_FROM,
    destination: LAVORO_TO,
    mode: "Mirror",
    new: 20,
    updated: 3,
    deleted: 1,
    durationMs: 33_000,
    outcome: "Cancelled",
    startedAt: ago(6 * DAY + HOUR),
    logPath: logPath("lavoro-3"),
    partial: 12,
    explanation: null,
  },
];
const runs = seeds.map(toReport);

/** Runs sorted by startedAt, newest first (ISO timestamps compare as strings). */
const newestFirst = <T extends Run>(list: T[]): T[] => [...list].sort((a, b) => b.startedAt.localeCompare(a.startedAt));

const NAS_NOTE =
  "Il NAS non risponde: la copia si è fermata a metà. Controlla che sia acceso e collegato, poi riprova.";

function listSyncs(): ListedSyncs {
  const sorted = newestFirst(runs);
  return {
    setAside: false,
    syncs: syncs.map((sync) => {
      const last = sorted.find((run) => run.syncName === sync.name);
      const note = last?.outcome === "Error" ? NAS_NOTE : last?.outcome === "Cancelled" ? "Annullata da te." : null;
      return {
        ...sync,
        sourcePlace: placeOf(sync.source),
        destinationPlace: placeOf(sync.destination),
        lastRun: last ? { when: last.when, outcome: last.outcome, note } : null,
      };
    }),
  };
}

function historySummary(): HistorySummary {
  const sorted = newestFirst(runs);
  const count = (outcome: Run["outcome"]) => sorted.filter((r) => r.outcome === outcome).length;
  const status = syncs.flatMap((sync): SyncStatus[] => {
    const own = sorted.filter((r) => r.syncName === sync.name);
    if (!own.length) return [];
    const failed = own[0].outcome === "Error";
    const lastOk = own.find((r) => r.outcome === "Ok");
    return [
      {
        id: sync.id,
        name: sync.name,
        lastSuccessText: lastOk ? `Ultima copia riuscita: ${lastOk.when}` : "Nessuna copia riuscita finora.",
        warning: failed ? `L’ultima copia è finita con un errore il ${own[0].when}.` : null,
      },
    ];
  });
  return {
    runs: sorted.length,
    ok: count("Ok"),
    failed: count("Error"),
    cancelled: count("Cancelled"),
    filesCopied: sorted.reduce((sum, r) => sum + (r.partial ?? r.new + r.updated), 0),
    filesDeleted: sorted.reduce((sum, r) => sum + r.deleted, 0),
    syncs: status.sort(
      (a, b) => Number(b.warning !== null) - Number(a.warning !== null) || a.name.localeCompare(b.name),
    ),
  };
}

interface Sample {
  new: string[];
  updated: string[];
  deleted: string[];
}
const SAMPLES: Record<string, Sample> = {
  [FOTO_FROM]: {
    new: [
      "Vacanze 2025\\IMG_4021.HEIC",
      "Vacanze 2025\\IMG_4022.HEIC",
      "Vacanze 2025\\VID_0107.MOV",
      "Compleanni\\IMG_4190.JPG",
      "Compleanni\\IMG_4191.JPG",
      "Famiglia\\Natale 2025.jpg",
      "Famiglia\\Nonna Rosa.png",
      "Screenshot\\Schermata 2026-10-28.png",
    ],
    updated: [
      "Famiglia\\Estate al mare.jpg",
      "Compleanni\\Torta di Luca.jpg",
      "Vacanze 2025\\IMG_3998.HEIC",
      "Album.xmp",
      "Famiglia\\Ritratto 2024.tif",
    ],
    deleted: [
      "Bozze\\Vecchia copia.jpg",
      "Bozze\\IMG_0001.JPG",
      "Temp\\anteprima.db",
      "Screenshot\\vecchio.png",
      "Compleanni\\doppione.jpg",
    ],
  },
  [LAVORO_FROM]: {
    new: [
      "2026\\Offerta Rossi srl.docx",
      "2026\\Preventivo maggio.xlsx",
      "2026\\Contratto manutenzione.pdf",
      "Modelli\\Fattura v3.dotx",
      "2026\\Riunione 2026-10-30.pptx",
      "2026\\Note consegna.odt",
      "2026\\Bilancio parziale.xlsx",
      "2026\\Email cliente.eml",
    ],
    updated: [
      "2025\\Bilancio finale.xlsx",
      "Modelli\\Lettera di incarico.docx",
      "2026\\Preventivo aprile.xlsx",
      "2026\\Verbale ottobre.docx",
      "2026\\Listino prezzi.pdf",
    ],
    deleted: [
      "2024\\Bozza vecchia.docx",
      "Temp\\~$Preventivo.docx",
      "2024\\Listino 2024 (copia).pdf",
      "2023\\Appunti.txt",
      "2023\\Foto ufficio.jpg",
    ],
  },
  [MUSICA_FROM]: {
    new: [
      "Jazz\\Chet Baker\\My Funny Valentine.flac",
      "Jazz\\Chet Baker\\Almost Blue.flac",
      "Classica\\Bach\\Goldberg Variations.mp3",
      "Podcast\\Ascolti del lunedì 42.mp3",
      "Jazz\\Miles Davis\\So What.flac",
      "Classica\\Chopin\\Notturno op. 9.mp3",
      "Podcast\\Storie di strada 17.mp3",
      "Jazz\\Bill Evans\\Peace Piece.flac",
    ],
    updated: [
      "Jazz\\Bill Evans\\Waltz for Debby.flac",
      "Classica\\Mozart\\Requiem.m4a",
      "Podcast\\Ascolti del lunedì 41.mp3",
      "Jazz\\Coltrane\\Giant Steps.flac",
      "Classica\\Vivaldi\\Le quattro stagioni.mp3",
    ],
    deleted: [
      "Jazz\\Duplicati\\Take Five (1).mp3",
      "Temp\\cover.jpg",
      "Podcast\\vecchio.m3u",
      "Classica\\Bach\\Partita (incompleta).mp3",
      "Jazz\\Nuovi\\provino.wav",
    ],
  },
};
const sampleFor = (source: string) => SAMPLES[source] ?? SAMPLES[FOTO_FROM];

/** Like Rust (`PreviewMemo`): the last preview with its token, A, B and mode; a run takes its counts from here. */
let lastPreview: { token: string; source: string; destination: string; mode: SyncMode; counts: Counts } | null = null;
let previews = 0;

/** The Mirror of this folder is blocked in the demo, as MirrorGuard blocks an A that looks empty or unplugged. */
const EMPTY_SOURCE = "E:\\Chiavetta";
const SOURCE_EMPTY =
  "A sembra vuota o scollegata: con lo specchio verrebbe svuotato B. Controlla che il disco di A sia collegato.";

async function previewSync(source: string, destination: string, mode: SyncMode): Promise<Preview> {
  lastPreview = null; // a preview that starts makes every earlier token stale
  await wait(600);
  const sample = sampleFor(source);
  if (mode === "Mirror" && source === EMPTY_SOURCE) {
    return {
      new: { count: 0, files: [] },
      updated: { count: 0, files: [] },
      deleted: { count: 0, files: [] },
      bytes: 0,
      blocked: SOURCE_EMPTY,
      previewToken: null, // a blocked preview confirms nothing
      caption: SOURCE_EMPTY,
      summary: [],
    };
  }
  const previewToken = `demo-${++previews}`;
  const preview: Preview = {
    new: { count: 120, files: sample.new },
    updated: { count: 40, files: sample.updated },
    deleted: mode === "Mirror" ? { count: 25, files: sample.deleted } : { count: 0, files: [] },
    bytes: Math.round(1.8 * 1024 ** 3),
    blocked: null,
    previewToken,
    caption: `Da fare: copiare ${num(120)} file nuovi, aggiornarne ${num(40)}${mode === "Mirror" ? `, cancellarne ${num(25)} da B` : ""}.`,
    summary:
      mode === "Mirror"
        ? [
            { text: "Ecco cosa succederà in B: ", bold: false },
            { text: num(120), bold: true },
            { text: " file nuovi, ", bold: false },
            { text: num(40), bold: true },
            { text: " da aggiornare, ", bold: false },
            { text: num(25), bold: true },
            { text: " da cancellare.", bold: false },
          ]
        : [
            { text: "Ecco cosa verrà fatto: ", bold: false },
            { text: num(120), bold: true },
            { text: " file nuovi, ", bold: false },
            { text: num(40), bold: true },
            { text: " da aggiornare.", bold: false },
          ],
  };
  lastPreview = {
    token: previewToken,
    source,
    destination,
    mode,
    counts: { new: preview.new.count, updated: preview.updated.count, deleted: preview.deleted.count },
  };
  return preview;
}

const RUN_MS = 8000;
const RUN_STEPS = 40;

const REDO_PREVIEW = "Controlla di nuovo le cartelle con l’anteprima, poi avvia.";

/** Like Rust's `PreviewMemo::confirm`: the counts of the preview, or a refusal (thrown) in the same order and words. */
function confirmPreview(sync: Sync, previewToken: string | null, consent: boolean): Counts {
  const refuse = (title: string, advice: string): never => {
    const refused: Explanation = { title: `Non ho avviato la copia: ${title}`, advice };
    throw refused;
  };
  if (!previewToken) {
    return refuse("manca l’anteprima.", "Prima controlla le due cartelle con l’anteprima, poi avvia.");
  }
  if (!lastPreview || lastPreview.token !== previewToken) {
    return refuse("l’anteprima non è più valida.", REDO_PREVIEW);
  }
  const { source, destination, mode, counts } = lastPreview;
  lastPreview = null; // a preview confirms one run, also a refused one
  if (source !== sync.source || destination !== sync.destination || mode !== sync.mode) {
    return refuse("l’anteprima è di altre cartelle o di un’altra modalità.", REDO_PREVIEW);
  }
  if (mode === "Mirror" && counts.deleted > 0 && !consent) {
    return refuse(
      "manca la tua conferma.",
      "Con lo specchio i file che ci sono solo in B vengono cancellati. Conferma di aver capito e riprova.",
    );
  }
  return counts;
}

async function runSync(sync: Sync, previewToken: string | null, consent: boolean): Promise<RunReport> {
  const counts = confirmPreview(sync, previewToken, consent);
  const total = counts.new + counts.updated + counts.deleted;
  const started = Date.now();
  const names = sampleFor(sync.source).new;
  for (let tick = 1; tick <= RUN_STEPS; tick++) {
    await wait(RUN_MS / RUN_STEPS);
    const done = Math.round((total * tick) / RUN_STEPS);
    const elapsedMs = Date.now() - started;
    // The words and rules of `eta_text` in Rust (shared/copy_text.rs).
    const left = Math.ceil((total - done) / (done / (elapsedMs / 1000)));
    const progress: RunProgress = {
      syncName: sync.name,
      done,
      currentFile: names[tick % names.length],
      eta:
        elapsedMs < 1000 || done === 0
          ? "Calcolo il tempo che manca…"
          : left <= 1
            ? "Quasi finito."
            : `Mancano circa ${num(left)} secondi.`,
    };
    await emit("run-progress", progress);
  }
  return toReport({
    syncName: sync.name,
    source: sync.source,
    destination: sync.destination,
    mode: sync.mode,
    new: counts.new,
    updated: counts.updated,
    deleted: counts.deleted,
    durationMs: Date.now() - started,
    outcome: "Ok",
    startedAt: new Date(started).toISOString(),
    logPath: logPath("demo-" + started),
    explanation: null,
  });
}

const DRIVES = ["C:\\", "D:\\", "E:\\"];
const TREE: Record<string, string[]> = {
  "C:\\": ["Users", "Programmi", "Windows"],
  "C:\\Users": ["Maria"],
  "C:\\Users\\Maria": ["Desktop", "Documenti", "Immagini", "Musica", "Scaricati"],
  "C:\\Users\\Maria\\Documenti": ["Fatture", "Lavoro", "Scuola"],
  "C:\\Users\\Maria\\Documenti\\Lavoro": ["2025", "2026", "Modelli"],
  "C:\\Users\\Maria\\Immagini": ["Foto", "Screenshot"],
  "C:\\Users\\Maria\\Immagini\\Foto": ["Compleanni", "Famiglia", "Vacanze 2025"],
  "D:\\": ["Musica", "Progetti", "Video"],
  "D:\\Musica": ["Classica", "Jazz", "Podcast"],
  "E:\\": ["Backup", "Chiavetta"],
  "E:\\Backup": ["Foto", "Musica"],
  "\\\\NAS\\": ["Archivio"],
  "\\\\NAS\\Archivio": ["Film", "Lavoro"],
};

const joinPath = (folder: string, name: string) => (folder.endsWith("\\") ? folder + name : folder + "\\" + name);

function parentOf(path: string): string {
  const cut = path.replace(/\\+$/, "").lastIndexOf("\\");
  const up = path.slice(0, cut + 1);
  return cut < 0 || /^\\*$/.test(up) ? "" : up;
}

function browseFolders(path: string): FolderListing {
  if (path === "") {
    return { parent: "", folders: DRIVES.map((drive) => ({ name: drive.slice(0, 2), path: drive })), error: null };
  }
  return {
    parent: parentOf(path),
    folders: (TREE[path] ?? []).map((name) => ({ name, path: joinPath(path, name) })),
    error: null,
  };
}

/** Adds the folder to its parent's listing; a name already there (Windows ignores case) is refused. */
function createFolder(parent: string, rawName: string): string {
  const name = rawName.trim();
  const siblings = TREE[parent] ?? [];
  if (siblings.some((existing) => existing.toLowerCase() === name.toLowerCase())) {
    const refused: Explanation = { title: "Esiste già una cartella con questo nome.", advice: "Scegli un altro nome." };
    throw refused;
  }
  TREE[parent] = [...siblings, name];
  return joinPath(parent, name);
}

const lastSegment = (path: string) => path.split("\\").filter(Boolean).pop() ?? "";

const LOG_TEXT = `-------------------------------------------------------------------------------
   ROBOCOPY     ::     Robust File Copy for Windows
-------------------------------------------------------------------------------

  Origine      : C:\\Users\\Maria\\Immagini\\Foto\\
  Destinazione : E:\\Backup\\Foto\\

     Nuovo file       1,2 MB    Vacanze 2025\\IMG_4021.HEIC
     Nuovo file       3,4 MB    Compleanni\\IMG_4190.JPG
     Nuovo file       84 MB     Vacanze 2025\\VID_0107.MOV
  100%  Copiato: Famiglia\\Natale 2025.jpg
2026/11/05 09:13:40 ERROR 53 (0x00000035) Copia del file \\\\NAS\\Archivio\\Lavoro\\Bilancio 2025.xlsx
Impossibile trovare il percorso di rete.

               Totale    Copiati   Saltati  Non riusciti
   File:           52         52         0            0
  Byte:       1,8 g      1,8 g          0            0

   Ended : mercoledì 5 novembre 2026 09:14:02
`;

mockWindows("main");

mockIPC(
  async (cmd, payload) => {
    const args = (payload ?? {}) as Record<string, unknown>;
    const text = (key: string) => String(args[key] ?? "");
    switch (cmd) {
      case "list_syncs":
        return listSyncs();
      case "list_runs":
        return { runs: newestFirst(runs), summary: historySummary() } satisfies RunList;
      case "preview_sync":
        return previewSync(text("source"), text("destination"), args.mode as SyncMode);
      case "browse_folders":
        return browseFolders(text("path"));
      case "create_folder":
        return createFolder(text("parent"), text("name"));
      case "default_sync_name":
        return lastSegment(text("source"));
      case "describe_place":
        return placeOf(text("path"));
      case "check_step": {
        const missing: Record<string, [string, string]> = {
          source: [text("source"), "Scegli prima la cartella da copiare."],
          destination: [text("destination"), "Scegli prima dove mettere la copia."],
          name: [text("name"), "Scrivi un nome per riconoscere questa sincronizzazione."],
        };
        const [value, title] = missing[text("step")] ?? ["x", ""];
        if (value.trim()) return undefined;
        const refused: Explanation = { title, advice: "" };
        throw refused;
      }
      case "save_sync": {
        const sync: Sync = {
          id: nextId++,
          name: text("name"),
          source: text("source"),
          destination: text("destination"),
          mode: args.mode as SyncMode,
        };
        syncs.push(sync);
        return sync;
      }
      case "is_elevated":
        return false;
      case "read_run_log":
        return {
          text: LOG_TEXT,
          errorLines: LOG_TEXT.split("\n").flatMap((line, index) =>
            /ERROR \d+ \(0x[0-9A-Fa-f]{8}\)/.test(line) ? [index] : [],
          ),
        } satisfies RunLog;
      case "cancel_run":
        return true;
      case "run_sync":
        return runSync(args.sync as Sync, (args.previewToken as string | null) ?? null, args.consent === true);
      default:
        console.warn(`demo: comando non simulato: ${cmd}`);
        return null;
    }
  },
  { shouldMockEvents: true },
);

await import("./main");
