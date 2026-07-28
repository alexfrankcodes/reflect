// The settings page. A change applies the moment it's made — there is no Save
// button here for the same reason the notes window has none.

const { invoke } = window.__TAURI__.core;

const dailyTime = document.getElementById("daily-time");
const showPrompts = document.getElementById("show-prompts");
const startAtLogin = document.getElementById("start-at-login");
const startAtLoginRow = document.getElementById("start-at-login-row");
const note = document.getElementById("note");

async function openSettings() {
  try {
    show(await invoke("settings_page"));
  } catch (err) {
    // A page that couldn't read the settings mustn't offer to write them:
    // saving from here would put its own empty boxes into force.
    trouble(err);
    dailyTime.disabled = true;
    showPrompts.disabled = true;
    startAtLogin.disabled = true;
    return;
  }

  // `change` rather than `input`: a time being typed passes through hours that
  // aren't the one the user means, and each of those would otherwise be put
  // into force and reschedule the reminder on the way past.
  dailyTime.addEventListener("change", apply);
  showPrompts.addEventListener("change", apply);
  startAtLogin.addEventListener("change", apply);
}

async function apply() {
  // An empty box is a time half-edited, not a request to change anything.
  if (!dailyTime.value) return;

  try {
    show(
      await invoke("settings_save", {
        dailyTime: dailyTime.value,
        showPrompts: showPrompts.checked,
        // A row that was never offered doesn't get to change the preference
        // behind it; Reflect keeps whatever is already stored.
        startAtLogin: startAtLoginRow.hidden ? null : startAtLogin.checked,
      }),
    );
  } catch (err) {
    // The boxes are left as the user set them, so the change they meant is
    // still in front of them to correct rather than snapped back.
    trouble(err);
  }
}

// Drawn from what Reflect handed back rather than from what's in the boxes, so
// the page can only ever show settings that actually took.
function show(settings) {
  dailyTime.value = settings.dailyTime;
  showPrompts.checked = settings.showPrompts;
  // `null` is Reflect saying this platform doesn't start apps at login at all,
  // which is different from saying it's turned off.
  startAtLoginRow.hidden = settings.startAtLogin === null;
  if (!startAtLoginRow.hidden) startAtLogin.checked = settings.startAtLogin;
  note.textContent = `Reflect will nudge you at ${inWords(settings.dailyTime)}, every day.`;
  note.classList.remove("trouble");
}

// Reflect keeps times as `HH:MM`, but the box above this line is drawn by the
// OS and shows them however this machine writes a time — "09:00 PM" here,
// "21:00" elsewhere. The sentence has to follow it: a page telling you 21:00
// directly underneath a box reading 09:00 PM is a page arguing with itself.
function inWords(dailyTime) {
  const [hours, minutes] = dailyTime.split(":").map(Number);
  const clock = new Date();
  clock.setHours(hours, minutes, 0, 0);
  return clock.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
}

function trouble(err) {
  note.textContent = `${err}`;
  note.classList.add("trouble");
  console.error(err);
}

openSettings();
