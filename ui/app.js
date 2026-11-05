"use strict";
const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;
const dialog = window.__TAURI__.dialog;

/* ---------------- icons ---------------- */
const P = {
  search: '<circle cx="11" cy="11" r="7"/><path d="m20 20-3.5-3.5"/>',
  x: '<path d="M18 6 6 18M6 6l12 12"/>',
  chevron: '<path d="m6 9 6 6 6-6"/>',
  right: '<path d="m9 6 6 6-6 6"/>',
  folder: '<path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"/>',
  folderPlus: '<path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"/><path d="M12 11v5M9.5 13.5h5"/>',
  star: '<path d="m12 3 2.7 5.6 6.1.9-4.4 4.3 1 6.1L12 17l-5.4 2.9 1-6.1L3.2 9.5l6.1-.9z"/>',
  library: '<path d="M4 4v16M9 4v16M14 5l5 15"/>',
  plus: '<path d="M12 5v14M5 12h14"/>',
  refresh: '<path d="M20 11a8 8 0 1 0-2.3 6.3"/><path d="M20 4v7h-7"/>',
  gear: '<circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.7 1.7 0 0 0 .3 1.8l.1.1a2 2 0 1 1-2.8 2.8l-.1-.1a1.7 1.7 0 0 0-1.8-.3 1.7 1.7 0 0 0-1 1.5V21a2 2 0 1 1-4 0v-.1a1.7 1.7 0 0 0-1.1-1.5 1.7 1.7 0 0 0-1.8.3l-.1.1a2 2 0 1 1-2.8-2.8l.1-.1a1.7 1.7 0 0 0 .3-1.8 1.7 1.7 0 0 0-1.5-1H3a2 2 0 1 1 0-4h.1a1.7 1.7 0 0 0 1.5-1.1 1.7 1.7 0 0 0-.3-1.8l-.1-.1a2 2 0 1 1 2.8-2.8l.1.1a1.7 1.7 0 0 0 1.8.3H9a1.7 1.7 0 0 0 1-1.5V3a2 2 0 1 1 4 0v.1a1.7 1.7 0 0 0 1 1.5 1.7 1.7 0 0 0 1.8-.3l.1-.1a2 2 0 1 1 2.8 2.8l-.1.1a1.7 1.7 0 0 0-.3 1.8V9a1.7 1.7 0 0 0 1.5 1H21a2 2 0 1 1 0 4h-.1a1.7 1.7 0 0 0-1.5 1z"/>',
  tag: '<path d="M20.6 13.4 13.4 20.6a2 2 0 0 1-2.8 0L3 13V3h10l7.6 7.6a2 2 0 0 1 0 2.8z"/><circle cx="7.5" cy="7.5" r="1"/>',
  open: '<path d="M14 4h6v6M20 4l-9 9"/><path d="M18 14v5a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1V7a1 1 0 0 1 1-1h5"/>',
  move: '<path d="M5 12h14M13 6l6 6-6 6"/>',
  check: '<path d="m5 12 5 5 9-10"/>',
  file: '<path d="M14 3H7a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2V8z"/><path d="M14 3v5h5"/>',
  audio: '<path d="M9 18V6l10-2v12"/><circle cx="6.5" cy="18" r="2.5"/><circle cx="16.5" cy="16" r="2.5"/>',
  video: '<rect x="3" y="5" width="13" height="14" rx="2"/><path d="m16 10 5-3v10l-5-3"/>',
  image: '<rect x="3" y="4" width="18" height="16" rx="2"/><circle cx="9" cy="10" r="1.6"/><path d="m21 16-5-5-9 9"/>',
  text: '<path d="M5 6h14M5 12h14M5 18h9"/>',
  clock: '<circle cx="12" cy="12" r="9"/><path d="M12 7v5l3 2"/>',
  upload: '<path d="M12 16V4M7 9l5-5 5 5"/><path d="M4 16v3a1 1 0 0 0 1 1h14a1 1 0 0 0 1-1v-3"/>',
  download: '<path d="M12 4v12M7 11l5 5 5-5"/><path d="M4 16v3a1 1 0 0 0 1 1h14a1 1 0 0 0 1-1v-3"/>',
  eye: '<path d="M2 12s3.6-7 10-7 10 7 10 7-3.6 7-10 7-10-7-10-7z"/><circle cx="12" cy="12" r="3"/>',
  reveal: '<path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"/><circle cx="12" cy="13" r="2.5"/>',
};
const ic = (n, extra = "") => `<svg viewBox="0 0 24 24" ${extra}>${P[n] || ""}</svg>`;
const MEDIA = {
  pdf: { label: "PDFs", badge: "PDF", icon: "file" },
  text: { label: "Plain Text", badge: "Text", icon: "text" },
  audio: { label: "Audio", badge: "Audio", icon: "audio" },
  video: { label: "Video", badge: "Video", icon: "video" },
  image: { label: "Images", badge: "Image", icon: "image" },
};
const SCOPES = [
  ["child", "Quotes"],
  ["parent", "Passages"],
  ["file", "Full media"],
];

/* ---------------- state ---------------- */
const S = {
  cfg: null,
  model: null,
  lib: null, // LibraryView
  types: new Set(Object.keys(MEDIA)),
  scope: "child",
  tags: new Set(),
  folder: null,
  query: "",
  hits: null, // null => browse mode
  searching: false,
  syncing: false,
  installing: false,
  shown: [],
};
let resolveReady;
S.ready = new Promise((r) => (resolveReady = r));
let pdfP = null;
const pdfMod = () => (pdfP ||= import("./pdfview.js"));
let searchSeq = 0;
let debounceT = null;

/* ---------------- helpers ---------------- */
const $ = (s, r = document) => r.querySelector(s);
const esc = (s) => String(s ?? "").replace(/[&<>"']/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" }[c]));
const baseName = (p) => p.split("/").pop();
const dirName = (p) => (p.includes("/") ? p.slice(0, p.lastIndexOf("/")) : "");
const plural = (n, w) => `${n} ${w}${n === 1 ? "" : "s"}`;
function fmtTime(t) {
  t = Math.max(0, Math.floor(t));
  const h = Math.floor(t / 3600), m = Math.floor((t % 3600) / 60), s = t % 60;
  return h ? `${h}:${String(m).padStart(2, "0")}:${String(s).padStart(2, "0")}` : `${m}:${String(s).padStart(2, "0")}`;
}
function fmtSize(b) {
  if (b < 1024) return `${b} B`;
  const u = ["KB", "MB", "GB", "TB"];
  let i = -1;
  do { b /= 1024; i++; } while (b >= 1024 && i < u.length - 1);
  return `${b.toFixed(b < 10 ? 1 : 0)} ${u[i]}`;
}
function toast(msg, kind = "") {
  const el = document.createElement("div");
  el.className = "toast " + kind;
  el.textContent = msg;
  $("#toasts").append(el);
  setTimeout(() => el.remove(), kind === "error" ? 6000 : 3200);
}
async function call(cmd, args) {
  try {
    return await invoke(cmd, args);
  } catch (e) {
    toast(String(e), "error");
    throw e;
  }
}
function highlight(text, q) {
  let out = esc(text);
  const words = [...new Set(q.toLowerCase().split(/[^\p{L}\p{N}]+/u).filter((w) => w.length >= 3))];
  if (!words.length) return out;
  const re = new RegExp(`(${words.map((w) => esc(w).replace(/[.*+?^${}()|[\]\\]/g, "\\$&")).join("|")})`, "gi");
  return out.replace(re, "<mark>$1</mark>");
}
const folderCount = (f) => S.lib.files.filter((x) => x.path.startsWith(f + "/")).length;
const inFolder = (path) => !S.folder || path.startsWith(S.folder + "/");
const hasTags = (tags) => [...S.tags].every((t) => tags.includes(t));
const fileById = (id) => S.lib?.files.find((f) => f.id === id);

/* ---------------- popovers & modals ---------------- */
function closePop() { $("#popRoot").innerHTML = ""; }
function openPop(anchor, html, onMount) {
  closePop();
  const wrap = document.createElement("div");
  wrap.className = "pop";
  wrap.innerHTML = html;
  $("#popRoot").append(wrap);
  const r = anchor.getBoundingClientRect();
  const w = wrap.offsetWidth, h = wrap.offsetHeight;
  wrap.style.left = Math.max(8, Math.min(r.right - w, innerWidth - w - 8)) + "px";
  wrap.style.top = (r.bottom + 6 + h > innerHeight ? Math.max(8, r.top - h - 6) : r.bottom + 6) + "px";
  onMount?.(wrap);
  return wrap;
}
document.addEventListener("mousedown", (e) => {
  if (!e.target.closest(".pop")) closePop();
  if (!e.target.closest(".lib-switch")) setLibMenu(false);
});
function modal({ title, body, confirm = "Done", cancel = "Cancel", onConfirm, danger }) {
  return new Promise((resolve) => {
    const root = $("#modalRoot");
    root.innerHTML = `<div class="scrim"><form class="modal" role="dialog" aria-modal="true">
      <h3>${esc(title)}</h3>${body}
      <div class="foot">${cancel ? `<button type="button" class="btn" data-x>${esc(cancel)}</button>` : ""}
      <button type="submit" class="btn primary ${danger ? "danger" : ""}">${esc(confirm)}</button></div></form></div>`;
    const form = $("form", root);
    const close = (v) => { root.innerHTML = ""; document.removeEventListener("keydown", onKey); resolve(v); };
    const onKey = (e) => e.key === "Escape" && close(null);
    document.addEventListener("keydown", onKey);
    $(".scrim", root).addEventListener("mousedown", (e) => e.target.classList.contains("scrim") && close(null));
    $("[data-x]", form)?.addEventListener("click", () => close(null));
    form.addEventListener("submit", async (e) => {
      e.preventDefault();
      const v = onConfirm ? await onConfirm(form) : true;
      if (v !== false) close(v ?? true);
    });
    $("input", form)?.focus();
  });
}

/* ---------------- library menu ---------------- */
function setLibMenu(open) {
  const m = $("#libMenu");
  m.hidden = !open;
  $("#recentBtn").setAttribute("aria-expanded", open);
  if (open) renderLibMenu();
}
function renderLibMenu() {
  const recents = S.cfg.recent_libraries;
  $("#libMenu").innerHTML = `
    <h6>Recent libraries</h6>
    ${recents.length ? recents.map((r) => `<button class="item ${S.lib && r.path === S.lib.path ? "current" : ""}" data-open="${esc(r.path)}">
        ${ic("library")}<span><b>${esc(r.name)}</b><span class="sub">${esc(r.path)}</span></span></button>`).join("")
      : '<div class="side-empty" style="padding:8px 10px">Libraries you open will show up here.</div>'}
    <hr>
    <button class="item" data-act="import">${ic("download")}Import library file…</button>
    ${S.lib ? `<button class="item" data-act="export">${ic("upload")}Export “${esc(S.lib.name)}” as a file…</button>` : ""}`;
}
$("#recentBtn").addEventListener("click", () => setLibMenu($("#libMenu").hidden));
$("#changeBtn").addEventListener("click", () => pickLibrary());
$("#newBtn").addEventListener("click", () => newLibraryModal());
$("#libMenu").addEventListener("click", (e) => {
  const b = e.target.closest("button");
  if (!b) return;
  setLibMenu(false);
  if (b.dataset.open) openLibrary(b.dataset.open);
  else if (b.dataset.act === "new") newLibraryModal();
  else if (b.dataset.act === "pick") pickLibrary();
  else if (b.dataset.act === "import") importLibrary();
  else if (b.dataset.act === "export") exportLibrary();
});

/* ---------------- library actions ---------------- */
async function pickLibrary() {
  const dir = await dialog.open({ directory: true, title: "Choose a library folder" });
  if (dir) openLibrary(dir);
}
async function newLibraryModal() {
  const dirHint = S.cfg.default_library_directory;
  await modal({
    title: "New library",
    body: `<p>Pick a name. fuzz creates the folder for you.</p>
      <input type="text" name="name" placeholder="e.g. AI Research 2026" autocomplete="off" spellcheck="false" />
      <p class="hint">Saved in ${esc(dirHint)}</p>`,
    confirm: "Create library",
    onConfirm: async (form) => {
      const name = form.name.value.trim();
      if (!name) { form.name.focus(); return false; }
      try {
        const v = await invoke("create_library", { name });
        await afterOpen(v, true);
      } catch (e) { toast(String(e), "error"); return false; }
    },
  });
}
async function exportLibrary() {
  const name = S.lib.name.replace(/[<>:"/\|?*]+/g, "_");
  const dest = await dialog.save({ title: "Export library", defaultPath: `${name}.fuzz`, filters: [{ name: "fuzz library", extensions: ["fuzz"] }] });
  if (!dest) return;
  try {
    const n = await invoke("export_library", { dest });
    toast(`Exported ${plural(n, "file")} to one .fuzz file`);
  } catch (e) { toast(String(e), "error"); } finally { $("#progress").hidden = true; }
}
async function importLibrary() {
  const archive = await dialog.open({ title: "Import library", multiple: false, filters: [{ name: "fuzz library", extensions: ["fuzz"] }] });
  if (!archive) return;
  try {
    const v = await invoke("import_library", { archive });
    toast(`Imported “${v.name}”`);
    await afterOpen(v);
  } catch (e) { toast(String(e), "error"); } finally { $("#progress").hidden = true; }
}
async function refreshCfg() { S.cfg = (await invoke("bootstrap")).config; }
async function openLibrary(path) {
  try {
    const v = await invoke("open_library", { path });
    await afterOpen(v);
  } catch (e) {
    toast(String(e), "error");
    await refreshCfg();
    renderAll();
  }
}
async function afterOpen(v, fresh = false) {
  S.lib = v;
  S.tags.clear(); S.folder = null; S.query = ""; S.hits = null;
  await refreshCfg();
  renderAll();
  if (!fresh) {
    await S.ready; // never re-index with a placeholder embedder while the model is still loading
    try { S.lib = await invoke("library_view"); } catch {}
    renderAll();
    if (!S.lib.model_mismatch) sync(true);
  }
}
async function sync(quiet = false) {
  if (S.syncing || !S.lib) return;
  S.syncing = true;
  renderTop();
  try {
    const r = await invoke("sync_library");
    S.lib = r.library;
    const parts = [];
    if (r.added) parts.push(`${r.added} added`);
    if (r.updated) parts.push(`${r.updated} updated`);
    if (r.removed) parts.push(`${r.removed} removed`);
    if (!quiet || parts.length) toast(parts.length ? `Indexed: ${parts.join(", ")}` : "Library is up to date");
  } catch (e) {
    toast(String(e), "error");
  } finally {
    S.syncing = false;
    $("#progress").hidden = true;
    renderAll();
    if (S.query.trim()) runSearch();
  }
}
listen("task-progress", (ev) => {
  const { label, done, total, current } = ev.payload;
  const bar = $("#progress");
  bar.hidden = total === 0 || done >= total;
  $("div", bar).style.width = total ? (done / total) * 100 + "%" : "0";
  const bytes = label.startsWith("Downloading");
  const f = (n) => (bytes ? fmtSize(n) : n);
  $("span", bar).textContent = bytes ? `${label} ${f(done)} / ${f(total)}` : current ? `${label} ${done}/${total} · ${current}` : `${label} ${done}/${total}`;
});
async function addFiles(paths, folder = S.folder || "") {
  if (!paths?.length) return;
  const n = await call("import_files", { paths, folder });
  if (!n) return toast("No supported files to add");
  toast(`Added ${plural(n, "file")}`);
  sync(true);
}

/* ---------------- tags / folders / moves ---------------- */
async function setTags(ids, add = [], remove = []) {
  const v = await call("set_tags", { fileIds: ids, add, remove });
  S.lib = v;
  renderSidebar();
  renderResults();
}
const toggleBookmark = (id) => {
  const f = fileById(id);
  return setTags([id], f.tags.includes("bookmark") ? [] : ["bookmark"], f.tags.includes("bookmark") ? ["bookmark"] : []);
};
function tagPopover(anchor, id) {
  const render = (wrap) => {
    const f = fileById(id);
    const all = S.lib.tags.map(([t]) => t).filter((t) => !f.tags.includes(t));
    wrap.innerHTML = `<h6>Tags · ${esc(baseName(f.path))}</h6>
      <div class="filter-tags" style="display:flex;flex-wrap:wrap;gap:6px;margin-bottom:8px">
        ${f.tags.map((t) => `<span class="filter-tag">${esc(t)}<button data-rm="${esc(t)}" aria-label="Remove ${esc(t)}">${ic("x", 'style="width:11px;height:11px"')}</button></span>`).join("") || '<span class="note">No tags yet</span>'}
      </div>
      <input type="text" placeholder="Add a tag, press Enter" list="taglist" autocomplete="off" spellcheck="false" />
      <datalist id="taglist">${all.map((t) => `<option value="${esc(t)}">`).join("")}</datalist>`;
    const input = $("input", wrap);
    input.focus();
    input.addEventListener("keydown", async (e) => {
      if (e.key === "Enter" && input.value.trim()) {
        const v = input.value;
        await setTags([id], v.split(",").map((s) => s.trim()).filter(Boolean));
        render(wrap);
      } else if (e.key === "Escape") closePop();
    });
    wrap.querySelectorAll("[data-rm]").forEach((b) => b.addEventListener("click", async () => { await setTags([id], [], [b.dataset.rm]); render(wrap); }));
  };
  openPop(anchor, "", render);
}
function movePopover(anchor, id) {
  const f = fileById(id);
  const cur = dirName(f.path);
  const folders = ["", ...S.lib.folders];
  openPop(anchor, `<h6>Move “${esc(baseName(f.path))}” to</h6>
    <div class="list menu-list" style="position:static;box-shadow:none;border:none;padding:0">
    ${folders.map((d) => `<button class="menu item" style="position:static;box-shadow:none;border:none;padding:7px 8px;min-width:0;${d === cur ? "opacity:.45;pointer-events:none" : ""}" data-to="${esc(d)}">
      ${ic("folder")}<span>${d ? esc(d) : "Library root"}</span></button>`).join("")}
    </div>
    <button class="menu item" style="position:static;box-shadow:none;border:none;padding:7px 8px;min-width:0" data-new>${ic("folderPlus")}New folder…</button>`, (wrap) => {
    wrap.addEventListener("click", async (e) => {
      const b = e.target.closest("button");
      if (!b) return;
      if (b.dataset.new) { closePop(); const name = await folderModal(); if (name) moveFiles([id], name); }
      else if (b.dataset.to !== undefined) { closePop(); moveFiles([id], b.dataset.to); }
    });
  });
}
async function moveFiles(ids, folder) {
  try {
    S.lib = await invoke("move_files", { fileIds: ids, folder });
    toast(`Moved to ${folder || "library root"}`);
    renderAll();
    if (S.query.trim()) runSearch();
  } catch (e) { toast(String(e), "error"); }
}
async function folderModal() {
  const parent = S.folder ? S.folder + "/" : "";
  return modal({
    title: "New folder",
    body: `<p>Subfolders are real folders on disk${parent ? ` inside <code>${esc(S.folder)}</code>` : ""}. Use “/” to nest.</p>
      <input type="text" name="name" placeholder="e.g. Lectures/Week 1" autocomplete="off" spellcheck="false" />`,
    confirm: "Create folder",
    onConfirm: async (form) => {
      const name = form.name.value.trim();
      if (!name) { form.name.focus(); return false; }
      const path = parent + name;
      try { S.lib = await invoke("create_folder", { path }); renderAll(); return path.replace(/\\/g, "/"); }
      catch (e) { toast(String(e), "error"); return false; }
    },
  });
}

/* ---------------- search ---------------- */
function scheduleSearch() {
  clearTimeout(debounceT);
  debounceT = setTimeout(runSearch, 220);
}
async function runSearch() {
  const q = S.query.trim();
  const seq = ++searchSeq;
  if (!q) { S.hits = null; S.searching = false; return renderResults(); }
  S.searching = true;
  if (S.hits === null) renderResults();
  try {
    const hits = await invoke("search", {
      params: { query: q, types: [...S.types], scope: S.scope, tags: [...S.tags], folder: S.folder, limit: 50 },
    });
    if (seq !== searchSeq) return;
    S.hits = hits;
  } catch (e) {
    if (seq === searchSeq) toast(String(e), "error");
    S.hits = [];
  }
  S.searching = false;
  renderResults();
}
const refresh = () => (S.query.trim() ? runSearch() : renderResults());

/* ---------------- rendering ---------------- */
function renderAll() {
  renderTop();
  renderBody();
}
function renderTop() {
  const lib = S.lib;
  $("#libName").innerHTML = `${ic("library")}<span title="${lib ? esc(lib.path) : ""}">${lib ? esc(lib.name) : "No library"}</span>`;
  $("#changeBtn").innerHTML = `${ic("folder")}Change folder`;
  $("#newBtn").innerHTML = `${ic("plus")}New library`;
  $("#recentBtn").innerHTML = `${ic("clock")}Recent${S.cfg?.recent_libraries?.length ? ` <span class="count-pill">${S.cfg.recent_libraries.length}</span>` : ""}${ic("chevron")}`;
  $("#addBtn").hidden = !lib;
  $("#addBtn").innerHTML = `${ic("plus")}Add files`;
  $("#syncBtn").hidden = !lib;
  $("#syncBtn").innerHTML = ic("refresh");
  $("#syncBtn").classList.toggle("spin", S.syncing);
  $("#settingsBtn").innerHTML = ic("gear");
  const m = S.model;
  const pill = $("#modelPill");
  if (m) {
    pill.className = "pill " + (m.loading ? "loading" : m.semantic ? "semantic" : "lexical");
    pill.innerHTML = `<span class="dot"></span>${m.loading ? "Loading model…" : m.semantic ? `EmbeddingGemma · ${esc(m.accelerator.split(">")[0].trim())}` : "Keyword mode"}`;
    pill.title = m.loading ? "Loading the semantic model…" : m.semantic ? `${m.active} on ${m.accelerator}` : "Semantic model not installed. Click to install.";
  }
}
function renderBody() {
  const body = $("#body");
  if (!S.lib) {
    const rec = S.cfg.recent_libraries;
    body.innerHTML = `<div class="welcome"><div class="welcome-card">
      <h1>Search by meaning.</h1>
      <p class="tagline">fuzz indexes your PDFs, notes, recordings and images on this device. Nothing leaves your computer.</p>
      <div class="btns"><button class="btn primary" data-act="new">${ic("plus")}New library</button>
        <button class="btn" data-act="pick">${ic("folder")}Open folder…</button></div>
      ${rec.length ? `<h6>Recent</h6><div class="recent-list">${rec.map((r) => `<button class="item" data-open="${esc(r.path)}">${ic("library")}<span><b>${esc(r.name)}</b><span class="sub">${esc(r.path)}</span></span></button>`).join("")}</div>` : ""}
    </div></div>`;
    body.onclick = (e) => {
      const b = e.target.closest("button");
      if (!b) return;
      if (b.dataset.open) openLibrary(b.dataset.open);
      else if (b.dataset.act === "new") newLibraryModal();
      else if (b.dataset.act === "pick") pickLibrary();
    };
    return;
  }
  body.onclick = null;
  if (!$("#sidebar")) {
    body.innerHTML = `<aside id="sidebar"></aside><main id="main">
      <div id="deck">
        <div class="search">${ic("search")}<input id="q" type="text" placeholder="Search by meaning…" autocomplete="off" spellcheck="false" /><kbd id="kbd">/</kbd>
          <button class="clear" id="qclear" hidden aria-label="Clear">${ic("x")}</button></div>
        <div class="controls"><div class="chips" id="chips"></div><div class="seg" id="scope" role="tablist"></div></div>
        <div class="active-filters" id="active"></div>
        <div id="notice"></div>
      </div><div id="results"></div></main>`;
    const q = $("#q");
    q.addEventListener("input", () => { S.query = q.value; $("#qclear").hidden = !q.value; $("#kbd").hidden = !!q.value; scheduleSearch(); });
    q.addEventListener("keydown", (e) => { if (e.key === "Enter") { clearTimeout(debounceT); runSearch(); } if (e.key === "Escape") { q.value = ""; q.dispatchEvent(new Event("input")); } });
    $("#qclear").addEventListener("click", () => { q.value = ""; q.dispatchEvent(new Event("input")); q.focus(); });
    $("#sidebar").addEventListener("click", onSidebarClick);
    $("#chips").addEventListener("click", (e) => {
      const c = e.target.closest("[data-type]");
      if (!c) return;
      const t = c.dataset.type;
      S.types.has(t) ? S.types.delete(t) : S.types.add(t);
      renderDeck(); refresh();
    });
    $("#scope").addEventListener("click", (e) => {
      const b = e.target.closest("[data-scope]");
      if (!b) return;
      S.scope = b.dataset.scope; renderDeck(); refresh();
    });
    $("#active").addEventListener("click", (e) => {
      const b = e.target.closest("[data-clear]");
      if (!b) return;
      if (b.dataset.clear === "folder") S.folder = null; else S.tags.delete(b.dataset.clear);
      renderSidebar(); renderDeck(); refresh();
    });
    $("#results").addEventListener("click", onResultClick);
  }
  $("#q").value = S.query;
  renderSidebar();
  renderDeck();
  renderResults();
}
function renderSidebar() {
  const sb = $("#sidebar");
  if (!sb) return;
  const lib = S.lib;
  const bm = lib.tags.find(([t]) => t === "bookmark")?.[1] || 0;
  const otherTags = lib.tags.filter(([t]) => t !== "bookmark");
  const nav = (attrs, active, icon, name, count) =>
    `<button class="nav ${active ? "active" : ""}" ${attrs}>${icon}<span class="name">${esc(name)}</span>${count != null ? `<span class="count">${count}</span>` : ""}</button>`;
  sb.innerHTML = `
    <div class="side-title">Library</div>
    ${nav('data-view="all"', !S.folder && !S.tags.size, ic("library"), "All media", lib.files.length)}
    ${nav('data-tag="bookmark"', S.tags.has("bookmark"), ic("star"), "Bookmarks", bm)}
    <div class="side-title">Folders<button data-act="newfolder" title="New folder">${ic("plus", 'style="width:14px;height:14px"')}</button></div>
    ${lib.folders.length ? lib.folders.map((f) => {
      const depth = f.split("/").length - 1;
      return `<button class="nav ${S.folder === f ? "active" : ""}" data-folder="${esc(f)}" style="padding-left:${8 + depth * 14}px" title="${esc(f)}">${ic("folder")}<span class="name">${esc(baseName(f))}</span><span class="count">${folderCount(f)}</span></button>`;
    }).join("") : '<div class="side-empty">No subfolders yet</div>'}
    <div class="side-title">Tags</div>
    ${otherTags.length ? otherTags.map(([t, n]) => nav(`data-tag="${esc(t)}"`, S.tags.has(t), '<span class="hash">#</span>', t, n)).join("") : '<div class="side-empty">Tag media to filter by it</div>'}`;
}
function onSidebarClick(e) {
  const b = e.target.closest("button");
  if (!b) return;
  if (b.dataset.act === "newfolder") { folderModal(); return; }
  if (b.dataset.view) { S.folder = null; S.tags.clear(); }
  else if (b.dataset.folder !== undefined) S.folder = S.folder === b.dataset.folder ? null : b.dataset.folder;
  else if (b.dataset.tag) S.tags.has(b.dataset.tag) ? S.tags.delete(b.dataset.tag) : S.tags.add(b.dataset.tag);
  renderSidebar(); renderDeck(); refresh();
}
function renderNotice() {
  const n = $("#notice");
  if (!n) return;
  const l = S.lib, m = S.model;
  const install = l && m && !m.semantic && !m.loading && !S.dismissInstall
    ? `<div class="banner info"><span><b>Search by meaning.</b> Install the on-device semantic model (${fmtSize(m.download_bytes)}, one-time download). Until then fuzz matches on shared words only.</span>
        <button class="btn sm primary" id="installModel" ${S.installing ? "disabled" : ""}>${S.installing ? "Installing…" : "Install model"}</button>
        <button class="icon-btn" id="dismissInstall" title="Not now" style="width:26px;height:26px">${ic("x")}</button></div>` : "";
  if (!l?.model_mismatch) { n.innerHTML = install; return; }
  n.innerHTML = install + `<div class="banner"><span>This library was indexed with <b>${esc(l.embedding_model)}</b>, but this device is using <b>${esc(m.active)}</b>. Search is paused until the library is re-indexed here${m.semantic ? "" : " (or install the semantic model to match)"}.</span>
    <button class="btn sm primary" id="reindex">Re-index</button></div>`;
}
document.addEventListener("click", (e) => {
  const id = e.target.closest("button")?.id;
  if (id === "reindex") sync();
  else if (id === "installModel") installModel();
  else if (id === "dismissInstall") { S.dismissInstall = true; renderNotice(); }
});
async function installModel() {
  if (S.installing) return;
  S.installing = true;
  renderNotice();
  try {
    S.model = await invoke("install_model");
    toast("Semantic model installed");
    if (S.lib) { S.lib = await invoke("library_view"); renderAll(); sync(true); }
  } catch (e) {
    toast(String(e), "error");
  } finally {
    S.installing = false;
    $("#progress").hidden = true;
    renderAll();
  }
}
function renderDeck() {
  renderNotice();
  $("#chips").innerHTML = Object.entries(MEDIA).map(([k, m]) =>
    `<button class="chip ${S.types.has(k) ? "on" : ""}" data-type="${k}" aria-pressed="${S.types.has(k)}"><span class="box">${ic("check")}</span>${m.label}</button>`).join("");
  const browse = !S.query.trim();
  $("#scope").title = "What a result is: a single sentence/quote, a passage, or a whole file";
  $("#scope").innerHTML = SCOPES.map(([k, l]) => `<button data-scope="${k}" class="${S.scope === k ? "on" : ""}">${l}</button>`).join("");
  $("#active").innerHTML =
    (S.folder ? `<span class="filter-tag">${ic("folder", 'style="width:12px;height:12px"')} ${esc(S.folder)}<button data-clear="folder" aria-label="Clear folder filter">${ic("x", 'style="width:11px;height:11px"')}</button></span>` : "") +
    [...S.tags].map((t) => `<span class="filter-tag">${t === "bookmark" ? "★ bookmarks" : "#" + esc(t)}<button data-clear="${esc(t)}" aria-label="Remove filter">${ic("x", 'style="width:11px;height:11px"')}</button></span>`).join("");
}
function actionsHtml(f) {
  const bm = f.tags.includes("bookmark");
  return `<div class="actions">
    ${f.media_type === "pdf" ? `<button class="icon-btn" data-act="view" title="View PDF">${ic("eye")}</button>` : ""}
    <button class="icon-btn ${bm ? "on" : ""}" data-act="bookmark" title="${bm ? "Remove bookmark" : "Bookmark"}">${ic("star", bm ? 'style="fill:currentColor"' : "")}</button>
    <button class="icon-btn" data-act="tags" title="Edit tags">${ic("tag")}</button>
    <button class="icon-btn" data-act="move" title="Move to folder">${ic("move")}</button>
    <button class="icon-btn" data-act="reveal" title="Show in file manager">${ic("reveal")}</button>
    <button class="icon-btn" data-act="open" title="Open">${ic("open")}</button></div>`;
}
const pathHtml = (p) => {
  const d = dirName(p);
  return `<span class="path" data-act="open" title="Open ${esc(p)}">${d ? esc(d) + "/" : ""}<b>${esc(baseName(p))}</b></span>`;
};
const badgeHtml = (t) => `<span class="badge t-${t}">${ic(MEDIA[t].icon)}${MEDIA[t].badge}</span>`;
const tagsHtml = (tags) => tags.filter((t) => t !== "bookmark").map((t) => `<button class="tag ${S.tags.has(t) ? "on" : ""}" data-tagfilter="${esc(t)}">#${esc(t)}</button>`).join("");

function meterHtml(score) {
  const [lo, hi] = S.model?.semantic ? [0.25, 0.75] : [0.08, 0.55];
  const pct = Math.round(Math.min(1, Math.max(0, (score - lo) / (hi - lo))) * 100);
  const r = 15, c = 2 * Math.PI * r;
  const tone = pct >= 70 ? "hi" : pct >= 40 ? "mid" : "lo";
  return `<div class="meter ${tone}" title="Match closeness (cosine similarity ${score.toFixed(3)})" role="img" aria-label="${pct}% match">
    <svg viewBox="0 0 40 40"><circle class="trk" cx="20" cy="20" r="${r}"/><circle class="val" cx="20" cy="20" r="${r}" stroke-dasharray="${((pct / 100) * c).toFixed(1)} ${c.toFixed(1)}"/></svg><span>${pct}</span></div>`;
}

function renderResults() {
  const box = $("#results");
  if (!box) return;
  const top = box.scrollTop;
  const q = S.query.trim();
  let html = "";
  if (!q) {
    const files = S.lib.files.filter((f) => S.types.has(f.media_type) && inFolder(f.path) && hasTags(f.tags)).sort((a, b) => a.path.localeCompare(b.path, undefined, { numeric: true }));
    if (!S.lib.files.length) {
      html = `<div class="empty"><div class="glyph">${ic("folderPlus")}</div><h2>This library is empty</h2>
        <p>Add PDFs, notes, recordings or images. You can also drop files anywhere in this window or copy them into the library folder, then re-index.</p>
        <div class="btns"><button class="btn primary" data-act="addfiles">${ic("plus")}Add files</button><button class="btn" data-act="openroot">${ic("folder")}Show folder</button></div></div>`;
    } else if (!files.length) {
      html = `<div class="empty"><div class="glyph">${ic("search")}</div><h2>Nothing matches these filters</h2><p>Try enabling more media types or clearing the folder and tag filters.</p></div>`;
    } else {
      html = `<div class="summary"><span>${plural(files.length, "item")}</span><span>Type above to search by meaning</span></div>` +
        files.map((f) => `<div class="card row" data-file="${f.id}">${badgeHtml(f.media_type)}${pathHtml(f.path)}
          <div class="grow">${f.has_text ? "" : '<span class="note">no text to search yet</span>'}${tagsHtml(f.tags)}</div>
          <span class="meta">${fmtSize(f.size)}</span>${actionsHtml(f)}</div>`).join("");
    }
  } else if (S.hits === null || (S.searching && S.hits === null)) {
    html = '<div class="skeleton"></div>'.repeat(4);
  } else {
    const hits = S.hits.map((h) => ({ ...h, tags: fileById(h.file_id)?.tags ?? h.tags })).filter((h) => hasTags(h.tags));
    S.shown = hits;
    if (!hits.length) {
      html = `<div class="empty"><div class="glyph">${ic("search")}</div><h2>No results</h2><p>Nothing indexed matched “${esc(q)}”. Try another scope (${SCOPES.map((s) => s[1]).join(", ")}) or enable more media types.</p></div>`;
    } else {
      const lexical = S.model && !S.model.semantic;
      html = `<div class="summary"><span>${plural(hits.length, "result")}</span><span>${lexical ? "Keyword mode · install the model for true semantic search" : ""}</span></div>` +
        hits.map((h, i) => {
          const loc = h.start_sec != null ? fmtTime(h.start_sec) : h.page ? `p. ${h.page}` : "";
          const ctx = h.parent_text && h.parent_text !== h.text;
          return `<article class="card" data-file="${h.file_id}" data-i="${i}">
            <div class="card-head">${badgeHtml(h.media_type)}${pathHtml(h.path)}${loc ? `<span class="loc" title="Where this match is">${loc}</span>` : ""}
              ${meterHtml(h.score)}${actionsHtml({ tags: h.tags, media_type: h.media_type })}</div>
            <div class="card-body ${h.media_type === "pdf" ? "with-thumb" : ""}">
              <div class="card-text"><p class="quote">${highlight(h.text, q)}</p>
                <div class="card-foot">${ctx ? `<button class="ctx-toggle" aria-expanded="false" data-act="ctx">${ic("right")}Context</button>` : ""}${tagsHtml(h.tags)}</div></div>
              ${h.media_type === "pdf" ? `<button class="thumb" data-i="${i}" title="Open this spot in the PDF" aria-label="Open PDF at match"><span class="thumb-skel"></span></button>` : ""}
            </div>
            ${ctx ? `<div class="context" hidden>${highlight(h.parent_text, q)}</div>` : ""}</article>`;
        }).join("");
    }
  }
  box.innerHTML = html;
  box.scrollTop = top;
  hydrateThumbs();
}
/* ---------- PDF thumbnails & viewer ---------- */
const thumbCache = new Map();
let thumbObs = null, thumbQ = Promise.resolve();
const pdfBytes = (fileId) => () => invoke("read_pdf", { fileId });
function paintThumb(el, t) {
  el.classList.add("ready");
  el.innerHTML = `<img src="${t.url}" alt="PDF excerpt with the match highlighted" /><span class="thumb-tag">${ic("eye", 'style="width:12px;height:12px"')} View in PDF</span>`;
}
function hydrateThumbs() {
  thumbObs?.disconnect();
  const els = [...document.querySelectorAll(".thumb[data-i]")];
  if (!els.length) return;
  thumbObs = new IntersectionObserver((entries) => {
    for (const en of entries) {
      if (!en.isIntersecting) continue;
      thumbObs.unobserve(en.target);
      thumbQ = thumbQ.then(() => fillThumb(en.target)).catch(() => {});
    }
  }, { root: $("#results"), rootMargin: "400px" });
  for (const el of els) {
    const h = S.shown[+el.dataset.i];
    const key = h && h.chunk_id;
    if (key && thumbCache.has(key)) paintThumb(el, thumbCache.get(key));
    else thumbObs.observe(el);
  }
}
async function fillThumb(el) {
  const h = S.shown[+el.dataset.i];
  if (!h || !el.isConnected) return;
  try {
    const pdf = await pdfMod();
    const doc = await pdf.getDoc(h.file_id, pdfBytes(h.file_id));
    const t = await pdf.renderThumb(doc, h.page || 1, h.granularity === "file" ? "" : h.text);
    thumbCache.set(h.chunk_id, t);
    if (el.isConnected) paintThumb(el, t);
  } catch (e) {
    console.warn("thumbnail failed", e);
    if (el.isConnected) el.innerHTML = '<span class="note">Preview unavailable</span>';
  }
}
async function openPdf(h) {
  try {
    const pdf = await pdfMod();
    const doc = await pdf.getDoc(h.file_id, pdfBytes(h.file_id));
    await pdf.openViewer({
      doc, title: h.path, page: h.page || 1,
      text: h.granularity === "file" ? "" : h.text,
      onExternal: () => call("open_file", { fileId: h.file_id }),
    });
  } catch (e) {
    toast("Could not open this PDF: " + e, "error");
  }
}

function onResultClick(e) {
  const thumb = e.target.closest(".thumb[data-i]");
  if (thumb) { const h = S.shown[+thumb.dataset.i]; if (h) openPdf(h); return; }
  const tagBtn = e.target.closest("[data-tagfilter]");
  if (tagBtn) { const t = tagBtn.dataset.tagfilter; S.tags.has(t) ? S.tags.delete(t) : S.tags.add(t); renderSidebar(); renderDeck(); refresh(); return; }
  const b = e.target.closest("[data-act]");
  if (!b) return;
  const act = b.dataset.act;
  if (act === "addfiles") return pickFiles();
  if (act === "openroot") return call("reveal_root", {}).catch(() => {});
  const card = b.closest("[data-file]");
  if (!card) return;
  const id = card.dataset.file;
  if (act === "ctx") {
    const open = b.getAttribute("aria-expanded") !== "true";
    b.setAttribute("aria-expanded", open);
    $(".context", card).hidden = !open;
  } else if (act === "bookmark") toggleBookmark(id);
  else if (act === "tags") tagPopover(b, id);
  else if (act === "move") movePopover(b, id);
  else if (act === "view") {
    const h = card.dataset.i != null ? S.shown[+card.dataset.i] : null;
    openPdf(h || { file_id: id, path: fileById(id).path, page: 1, text: "", granularity: "file" });
  }
  else if (act === "reveal") call("reveal_file", { fileId: id });
  else if (act === "open") call("open_file", { fileId: id });
}
async function pickFiles() {
  const exts = ["pdf", "txt", "md", "markdown", "rst", "log", "mp3", "wav", "flac", "m4a", "aac", "ogg", "opus", "mp4", "mkv", "mov", "avi", "webm", "m4v", "png", "jpg", "jpeg", "gif", "webp", "bmp", "tif", "tiff", "svg", "srt", "vtt"];
  const sel = await dialog.open({ multiple: true, title: "Add files to library", filters: [{ name: "Media & documents", extensions: exts }] });
  if (sel) addFiles(Array.isArray(sel) ? sel : [sel]);
}

/* ---------------- settings ---------------- */
async function settingsModal() {
  const m = S.model;
  await modal({
    title: "Settings",
    cancel: "",
    confirm: "Done",
    body: `
      <div class="field"><label>Default library folder</label>
        <div style="display:flex;gap:8px;align-items:center"><code style="flex:1">${esc(S.cfg.default_library_directory)}</code>
        <button type="button" class="btn sm" id="chgDir">Change</button></div></div>
      <div class="field"><label>Embedding engine</label>
        <div class="kv"><span>Model</span><span>${esc(m.active)}</span></div>
        <div class="kv"><span>Running on</span><span>${esc(m.accelerator)}</span></div>
        <p class="hint" style="margin-top:10px;line-height:1.5">${m.semantic ? "True semantic search is active." :
          `Keyword mode is active (works offline, matches on shared words). Install the EmbeddingGemma model (${fmtSize(m.download_bytes)}, one-time) for search by meaning. It runs entirely on this device.`}</p>
        <div style="display:flex;gap:8px;margin-top:8px">${m.semantic ? "" : `<button type="button" class="btn sm primary" id="installModelS">Install model</button>`}<button type="button" class="btn sm" id="openModel">Open model folder</button>
        <button type="button" class="btn sm" id="reloadModel">Reload model</button></div>
        <p class="hint">${esc(m.model_dir)}</p></div>`,
    onConfirm: () => true,
  }).catch(() => {});
}
document.addEventListener("click", async (e) => {
  if (e.target.id === "chgDir") {
    const d = await dialog.open({ directory: true, title: "Default folder for new libraries" });
    if (d) { S.cfg = await call("set_default_directory", { path: d }); $("#chgDir").previousElementSibling.textContent = d; }
  } else if (e.target.id === "installModelS") { $("#modalRoot").innerHTML = ""; installModel(); }
  else if (e.target.id === "openModel") call("open_model_folder");
  else if (e.target.id === "reloadModel") {
    S.model = await call("reload_model");
    renderTop();
    toast(S.model.semantic ? "Semantic model loaded" : "Model not found; staying in keyword mode");
    if (S.model.semantic && S.lib) { $("#modalRoot").innerHTML = ""; S.lib = await invoke("library_view"); renderAll(); sync(); }
  }
});
$("#settingsBtn").addEventListener("click", settingsModal);
$("#modelPill").addEventListener("click", settingsModal);
$("#addBtn").addEventListener("click", pickFiles);
$("#syncBtn").addEventListener("click", () => sync());

/* ---------------- shortcuts & drop ---------------- */
document.addEventListener("keydown", (e) => {
  const typing = /INPUT|TEXTAREA/.test(document.activeElement?.tagName);
  if ((e.key === "/" && !typing) || ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "k")) {
    const q = $("#q");
    if (q) { e.preventDefault(); q.focus(); q.select(); }
  }
  if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "o") { e.preventDefault(); pickLibrary(); }
  if (e.key === "Escape") { closePop(); setLibMenu(false); }
});
window.__TAURI__.webviewWindow.getCurrentWebviewWindow().onDragDropEvent((ev) => {
  const p = ev.payload;
  if (!S.lib) return;
  if (p.type === "enter" || p.type === "over") $("#dropHint").hidden = false;
  else if (p.type === "leave") $("#dropHint").hidden = true;
  else if (p.type === "drop") { $("#dropHint").hidden = true; addFiles(p.paths); }
});

listen("model-ready", async (ev) => {
  S.model = ev.payload;
  resolveReady();
  renderTop();
  if (S.lib) {
    try { S.lib = await invoke("library_view"); } catch {}
    renderAll();
    if (S.query.trim()) runSearch();
  }
});

/* ---------------- boot ---------------- */
(async function boot() {
  const b = await invoke("bootstrap");
  S.cfg = b.config;
  S.model = b.model;
  if (!b.model.loading) resolveReady();
  else {
    // fallback in case the model-ready event fired before the listener attached
    const iv = setInterval(async () => {
      const x = await invoke("bootstrap");
      if (!x.model.loading) { clearInterval(iv); S.model = x.model; resolveReady(); renderTop(); }
    }, 700);
  }
  renderAll();
  const last = S.cfg.last_opened_path;
  if (last && S.cfg.recent_libraries.some((r) => r.path === last)) openLibrary(last);
})();
