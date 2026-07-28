// The browse window. It lists the days there is something to read and shows
// the one that's picked. Nothing here writes: what a day says is what it said.

const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

const dayList = document.getElementById("days");
const reading = document.getElementById("reading");
const dayHeading = document.getElementById("day");
const entry = document.getElementById("entry");

// Reflect keeps a day as `YYYY-MM-DD`, which is a filename, not a date anyone
// reads. Both of these hand it back the way this machine writes a date — the
// list has a narrow column to do it in, the heading above the entry has room
// for the whole thing.
const IN_LIST = { year: "numeric", month: "short", day: "numeric" };
const IN_FULL = { weekday: "long", year: "numeric", month: "long", day: "numeric" };

// The day on the right, as `YYYY-MM-DD`, or `null` when there is nothing to
// show. Kept so that re-listing puts the reader back on the day they were
// reading rather than snapping them to the top.
let showing = null;

async function drawDays() {
  let dates;
  try {
    dates = await invoke("browse_dates");
  } catch (err) {
    // No list means no day to pick, so the column goes rather than sitting
    // there empty beside the reason it's empty.
    showList(false);
    dayHeading.textContent = "";
    trouble(err);
    return;
  }

  dayList.replaceChildren(...dates.map(dayButton));
  showList(dates.length > 0);

  if (dates.length === 0) {
    showing = null;
    dayHeading.textContent = "";
    say("Nothing written yet.");
    return;
  }

  // Opening on the most recent day rather than on an empty pane — the window
  // was asked for in order to read something. A day already being read keeps
  // its place.
  await show(dates.includes(showing) ? showing : dates[0]);
}

// The column and the pane beside it are one arrangement rather than two: with
// no days to offer there is no column, and what's left has the window to
// itself and centres in it.
function showList(visible) {
  dayList.hidden = !visible;
  reading.classList.toggle("alone", !visible);
}

function dayButton(date) {
  const button = document.createElement("button");
  button.type = "button";
  button.className = "day-button";
  button.dataset.date = date;
  button.textContent = inWords(date, IN_LIST);
  button.addEventListener("click", () => show(date));
  return button;
}

async function show(date) {
  showing = date;
  dayHeading.textContent = inWords(date, IN_FULL);
  markCurrent(date);

  let text;
  try {
    text = await invoke("browse_entry", { date });
  } catch (err) {
    if (showing !== date) return;
    trouble(err);
    return;
  }

  // A day clicked while a slower one was still loading is the day the reader
  // meant; the one that arrives late has been overtaken and is dropped.
  if (showing !== date) return;
  entry.classList.remove("aside", "trouble");
  entry.textContent = text;
}

function markCurrent(date) {
  for (const button of dayList.children) {
    const current = button.dataset.date === date;
    button.setAttribute("aria-current", current);
    // A day restored after a re-list, or reached by keyboard, may be well down
    // a long column.
    if (current) button.scrollIntoView({ block: "nearest" });
  }
}

// Reflect speaking for itself rather than showing something written — set
// apart from the entry it stands in for, so the two can't be confused.
function say(message) {
  entry.classList.add("aside");
  entry.classList.remove("trouble");
  entry.textContent = message;
}

function trouble(err) {
  say(`${err}`);
  entry.classList.add("trouble");
  console.error(err);
}

function inWords(isoDate, style) {
  const [year, month, day] = isoDate.split("-").map(Number);
  // Built from the parts rather than parsed: `new Date("2026-07-24")` is read
  // as UTC, which in a western timezone renders as the day before.
  return new Date(year, month - 1, day).toLocaleDateString([], style);
}

// Coming back to a window left open since before today's writing should show
// today. The tray says so when it brings this window forward.
listen("browse-again", drawDays);

drawDays();
