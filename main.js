import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";

let vaultPath = localStorage.getItem("vaultPath") || null;
let currentNote = null;
let saveTimer = null;
let allNotes = [];

const $ = (id) => document.getElementById(id);

async function pickVault() {
  const selected = await open({ directory: true, multiple: false });
  if (selected) {
    vaultPath = selected;
    localStorage.setItem("vaultPath", vaultPath);
    enableUI();
    refreshNotes();
  }
}

function enableUI() {
  $("new-note").disabled = false;
  $("search").disabled = false;
  $("editor").disabled = false;
  $("status").textContent = `vault: ${vaultPath}`;
}

async function refreshNotes() {
  if (!vaultPath) return;
  try {
    allNotes = await invoke("list_notes", { vaultPath });
    const query = $("search").value.toLowerCase();
    renderNoteList(await filterByQuery(query));
  } catch (e) {
    console.error("list_notes failed:", e);
    $("status").textContent = `error: ${e}`;
  }
}

function renderNoteList(notes) {
  const ul = $("note-list");
  ul.innerHTML = "";
  for (const note of notes) {
    const li = document.createElement("li");
    li.textContent = note.filename.replace(/\.md$/, "");
    li.dataset.filename = note.filename;
    if (note.filename === currentNote) li.classList.add("active");
    li.addEventListener("click", () => openNote(note.filename));
    ul.appendChild(li);
  }
}

async function openNote(filename) {
  if (!vaultPath) return;
  currentNote = filename;
  try {
    const content = await invoke("read_note", { vaultPath, filename });
    $("editor").value = content;
    document.querySelectorAll("#note-list li").forEach((li) => {
      li.classList.toggle("active", li.dataset.filename === filename);
    });
    $("status").textContent = `editing: ${filename}`;
  } catch (e) {
    console.error("read_note failed:", e);
  }
}

async function newNote() {
  if (!vaultPath) return;
  const filename = await invoke("create_note", { vaultPath });
  await refreshNotes();
  await openNote(filename);
  $("editor").focus();
}

function scheduleSave() {
  if (saveTimer) clearTimeout(saveTimer);
  saveTimer = setTimeout(saveNow, 500);
}

async function saveNow() {
  if (!vaultPath || !currentNote) return;
  const content = $("editor").value;
  try {
    await invoke("write_note", { vaultPath, filename: currentNote, content });
    $("status").textContent = `saved: ${currentNote}`;
  } catch (e) {
    console.error("write_note failed:", e);
  }
}

async function filterByQuery(q) {
  if (!q) return allNotes;
  const matches = [];
  for (const note of allNotes) {
    if (note.filename.toLowerCase().includes(q)) {
      matches.push(note);
      continue;
    }
    try {
      const content = await invoke("read_note", { vaultPath, filename: note.filename });
      if (content.toLowerCase().includes(q)) {
        matches.push(note);
      }
    } catch {}
  }
  return matches;
}

$("pick-vault").addEventListener("click", pickVault);
$("new-note").addEventListener("click", newNote);
$("editor").addEventListener("input", scheduleSave);
$("search").addEventListener("input", async (e) => {
  renderNoteList(await filterByQuery(e.target.value.toLowerCase()));
});

if (vaultPath) {
  enableUI();
  refreshNotes();
}

// Refresh note list every 3 seconds — picks up changes from cloud sync
setInterval(refreshNotes, 3000);