// In-app viewer for audio, video and images. Streams files through Tauri's asset protocol (Range-capable),
// opens at the matched timestamp, and shows a clickable transcript when a sidecar transcript was indexed.

const esc = (s) => String(s ?? "").replace(/[&<>"']/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" }[c]));
export function fmtTime(t) {
  t = Math.max(0, Math.floor(t));
  const h = Math.floor(t / 3600), m = Math.floor((t % 3600) / 60), s = t % 60;
  return h ? `${h}:${String(m).padStart(2, "0")}:${String(s).padStart(2, "0")}` : `${m}:${String(s).padStart(2, "0")}`;
}

/**
 * @param {{kind:'audio'|'video'|'image', title:string, src:string, start?:number, match?:{start:number,end:number}|null,
 *          cues?:{start:number,end:number,text:string}[], onExternal?:()=>void}} o
 */
export function openMedia({ kind, title, src, start = 0, match = null, cues = [], onExternal }) {
  const root = document.getElementById("modalRoot");
  const timed = kind !== "image";
  root.innerHTML = `<div class="viewer media" role="dialog" aria-modal="true" aria-label="${esc(title)}">
    <header class="v-head">
      <div class="v-title">${esc(title)}</div>
      <div class="v-tools"><button class="btn sm" id="mExt">Open in default app</button>
        <button class="icon-btn" id="mClose" title="Close (Esc)">✕</button></div>
    </header>
    <div class="m-body">
      <div class="m-stage" id="mStage"></div>
      ${timed && cues.length ? `<aside class="m-cues" id="mCues" aria-label="Transcript">
        ${cues.map((c, i) => `<button class="cue" data-i="${i}"><span class="t">${fmtTime(c.start)}</span><span>${esc(c.text)}</span></button>`).join("")}</aside>` : ""}
    </div></div>`;
  const stage = root.querySelector("#mStage");
  let media = null;

  if (kind === "image") {
    stage.innerHTML = `<img class="m-img" alt="${esc(title)}" src="${esc(src)}">`;
  } else if (kind === "video") {
    stage.innerHTML = `<video class="m-video" controls preload="metadata" src="${esc(src)}"></video>`;
  } else {
    stage.innerHTML = `<div class="m-audio"><div class="m-art" aria-hidden="true">♪</div><audio controls preload="metadata" src="${esc(src)}"></audio></div>`;
  }
  media = stage.querySelector("video,audio");
  const fail = () => {
    stage.innerHTML = `<div class="m-err"><b>This file can't be shown in the viewer.</b><br>The format may not be supported by the built-in player.<br><br>
      <button class="btn primary" id="mExt2">Open in default app</button></div>`;
    stage.querySelector("#mExt2").onclick = () => onExternal?.();
  };
  (media || stage.querySelector("img"))?.addEventListener("error", fail, { once: true });

  const close = () => {
    if (media) { media.pause(); media.removeAttribute("src"); media.load(); }
    root.innerHTML = "";
    document.removeEventListener("keydown", onKey, true);
  };
  const onKey = (e) => { if (e.key === "Escape") { e.stopPropagation(); close(); } };
  document.addEventListener("keydown", onKey, true);
  root.querySelector("#mClose").onclick = close;
  root.querySelector("#mExt").onclick = () => onExternal?.();
  root.querySelector(".viewer").addEventListener("mousedown", (e) => { if (e.target === e.currentTarget) close(); });

  if (!media) return;
  const cueEls = [...root.querySelectorAll(".cue")];
  const panel = root.querySelector("#mCues");
  // highlight the cue(s) that produced the search hit
  if (match) {
    cues.forEach((c, i) => { if (c.end >= match.start - 0.05 && c.start <= match.end + 0.05) cueEls[i]?.classList.add("match"); });
    const first = root.querySelector(".cue.match");
    if (first && panel) panel.scrollTop = Math.max(0, first.offsetTop - panel.clientHeight / 3);
  }
  media.addEventListener("loadedmetadata", () => {
    if (start > 0) media.currentTime = Math.max(0, start - 0.25);
    if (start > 0 || match) media.play().catch(() => {});
  }, { once: true });

  let active = -1;
  media.addEventListener("timeupdate", () => {
    if (!cueEls.length) return;
    const t = media.currentTime;
    let i = -1;
    for (let k = 0; k < cues.length; k++) { if (cues[k].start <= t + 0.05) i = k; else break; }
    if (i !== active) {
      cueEls[active]?.classList.remove("active");
      active = i;
      if (i >= 0) { cueEls[i].classList.add("active"); cueEls[i].scrollIntoView({ block: "nearest", behavior: "smooth" }); }
    }
  });
  panel?.addEventListener("click", (e) => {
    const b = e.target.closest(".cue");
    if (!b) return;
    media.currentTime = cues[+b.dataset.i].start;
    media.play().catch(() => {});
  });
}
