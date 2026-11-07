import type { Explanation } from "./types";

/**
 * What the UI says when Rust could not answer with an `Explanation` (Tauri's own failure: the IPC
 * call or its arguments broke). The only text the UI composes; every other message comes from Rust.
 */
const COULD_NOT_ANSWER: Explanation = {
  title: "Qualcosa è andato storto dentro l’app.",
  advice: "Riprova. Se succede ancora, chiudi e riapri ColomboSync.",
};

const isExplanation = (value: unknown): value is Explanation =>
  typeof value === "object" &&
  value !== null &&
  typeof (value as Explanation).title === "string" &&
  typeof (value as Explanation).advice === "string";

/** What a rejected `invoke` holds, as something the screen can show: Rust's `Explanation`, or a fixed sentence. */
export const toExplanation = (rejection: unknown): Explanation =>
  isExplanation(rejection) ? rejection : COULD_NOT_ANSWER;
