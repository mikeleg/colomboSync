import { fireEvent, screen } from "@testing-library/svelte";

export const click = (target: string | RegExp | HTMLElement) =>
  fireEvent.click(
    typeof target === "object" && "tagName" in target ? target : screen.getByRole("button", { name: target }),
  );
export const typeInto = (label: RegExp, value: string) =>
  fireEvent.input(screen.getByLabelText(label), { target: { value } });

/** From the first step of a new sync (Da dove copio?) to the preview, with D:\Foto and E:\Backup. */
export async function goToPreview() {
  await typeInto(/Percorso/, "D:\\Foto");
  await click("Avanti");
  await typeInto(/Percorso/, "E:\\Backup");
  await click("Avanti");
  await screen.findByDisplayValue("Copia di Foto");
  await click("Avanti"); // Come la chiamo?
  await click("Avanti"); // Opzioni
}
