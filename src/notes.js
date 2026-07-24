// The notes page. It draws today's prompt, and hands back what was typed when
// the window closes — there is no save button, and never a "discard?" dialog.

const { invoke } = window.__TAURI__.core;
const { getCurrentWindow } = window.__TAURI__.window;

const promptLine = document.getElementById("prompt");
const entry = document.getElementById("entry");

async function openTodaysPage() {
  let page;
  try {
    page = await invoke("notes_page");
  } catch (err) {
    // Nothing typed on a page that never opened could be saved, so say so
    // and close the page to writing rather than take words it would drop.
    promptLine.textContent = `${err}`;
    entry.placeholder = "";
    entry.readOnly = true;
    console.error(err);
    return;
  }

  promptLine.textContent = page.prompt;
  entry.value = page.text;
  entry.focus();
  // Reopening a day picks up after what's already written rather than in
  // front of it.
  entry.setSelectionRange(entry.value.length, entry.value.length);

  // Tauri only holds the window open for a close handler once this listener
  // exists, so a window closed before the page loaded just closes — which is
  // the right outcome, there being nothing typed on it yet. Without an
  // explicit preventDefault, Tauri closes the window as soon as this returns.
  await getCurrentWindow().onCloseRequested(async (event) => {
    try {
      await invoke("notes_close", { text: entry.value });
    } catch (err) {
      event.preventDefault();
      promptLine.textContent = `${err} — your writing is still here.`;
      console.error(err);
    }
  });
}

openTodaysPage();
