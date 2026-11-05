// PDF support: locating a search hit on its page, rendering highlighted thumbnails, and a scrollable viewer.
// Uses a locally bundled pdf.js (no network).
import * as pdfjs from "./vendor/pdf.min.js";

pdfjs.GlobalWorkerOptions.workerSrc = new URL("vendor/pdf.worker.min.js", location.href).href;
const DOC_OPTS = {
  cMapUrl: new URL("vendor/cmaps/", location.href).href,
  cMapPacked: true,
  standardFontDataUrl: new URL("vendor/standard_fonts/", location.href).href,
};

/* ---------- document cache ---------- */
const docs = new Map(); // fileId -> Promise<PDFDocumentProxy>
export function getDoc(fileId, fetchBytes) {
  if (docs.has(fileId)) {
    const p = docs.get(fileId);
    docs.delete(fileId);
    docs.set(fileId, p); // LRU bump
    return p;
  }
  const p = (async () => {
    const buf = await fetchBytes();
    return pdfjs.getDocument({ data: new Uint8Array(buf), ...DOC_OPTS }).promise;
  })();
  p.catch(() => docs.delete(fileId));
  docs.set(fileId, p);
  while (docs.size > 4) {
    const [k, v] = docs.entries().next().value;
    docs.delete(k);
    v.then((d) => d.destroy()).catch(() => {});
  }
  return p;
}
export function forgetDoc(fileId) {
  docs.delete(fileId);
}

/* ---------- locating text on a page ---------- */
const keep = /[^\p{L}\p{N}]/gu;
function compactChar(c) {
  return c.normalize("NFKD").replace(/\p{M}/gu, "").toLowerCase().replace(keep, "");
}
export function compact(s) {
  let out = "";
  for (const c of s) out += compactChar(c);
  return out;
}

const pageCache = new WeakMap(); // page -> {index}
async function pageIndex(page) {
  if (pageCache.has(page)) return pageCache.get(page);
  const tc = await page.getTextContent();
  const viewport = page.getViewport({ scale: 1 });
  let hay = "";
  const map = []; // per compact char: [itemIdx, charIdxInItem]
  const items = [];
  for (const it of tc.items) {
    if (!it.str) continue;
    const idx = items.length;
    const tx = pdfjs.Util.transform(viewport.transform, it.transform);
    const h = Math.hypot(tx[2], tx[3]) || it.height || 10;
    items.push({ str: it.str, x: tx[4], y: tx[5] - h, w: it.width, h, n: it.str.length });
    let ci = 0;
    for (const ch of it.str) {
      const c = compactChar(ch);
      for (let k = 0; k < c.length; k++) map.push([idx, ci]);
      hay += c;
      ci += ch.length;
    }
  }
  const r = { hay, map, items, width: viewport.width, height: viewport.height };
  pageCache.set(page, r);
  return r;
}

/** Find `text` on the page. Returns rects in page points (scale 1), or null. */
export async function locate(page, text) {
  const idx = await pageIndex(page);
  const needle = compact(text);
  if (!needle || !idx.hay) return { rects: [], width: idx.width, height: idx.height };
  let start = idx.hay.indexOf(needle);
  let end = start + needle.length;
  if (start < 0) {
    // extraction differences: probe several anchors across the text and align on the first that hits
    const n = needle.length;
    search: for (const size of [24, 14, 10]) {
      const probe = Math.min(size, n);
      for (const frac of [0, 0.5, 0.25, 0.75, 1]) {
        const off = Math.min(n - probe, Math.floor(n * frac));
        const at = idx.hay.indexOf(needle.slice(off, off + probe));
        if (at >= 0) {
          start = Math.max(0, at - off);
          end = Math.min(idx.hay.length, start + n);
          break search;
        }
      }
    }
  }
  if (start < 0) return { rects: [], width: idx.width, height: idx.height };
  // collect per-item character ranges
  const per = new Map();
  for (let i = start; i < end && i < idx.map.length; i++) {
    const [it, ci] = idx.map[i];
    const r = per.get(it);
    if (r) r[1] = ci + 1;
    else per.set(it, [ci, ci + 1]);
  }
  let rects = [];
  for (const [it, [a, b]] of per) {
    const item = idx.items[it];
    const n = Math.max(1, item.n);
    rects.push({ x: item.x + (item.w * a) / n, y: item.y, w: (item.w * (b - a)) / n, h: item.h * 1.15 });
  }
  // merge neighbours on the same line
  rects.sort((p, q) => Math.abs(p.y - q.y) < 2 ? p.x - q.x : p.y - q.y);
  const merged = [];
  for (const r of rects) {
    const l = merged[merged.length - 1];
    if (l && Math.abs(l.y - r.y) < 3 && r.x - (l.x + l.w) < 8) {
      const right = Math.max(l.x + l.w, r.x + r.w);
      l.x = Math.min(l.x, r.x);
      l.w = right - l.x;
      l.h = Math.max(l.h, r.h);
    } else merged.push({ ...r });
  }
  return { rects: merged, width: idx.width, height: idx.height };
}

const bbox = (rects) => ({
  x0: Math.min(...rects.map((r) => r.x)),
  y0: Math.min(...rects.map((r) => r.y)),
  x1: Math.max(...rects.map((r) => r.x + r.w)),
  y1: Math.max(...rects.map((r) => r.y + r.h)),
});

/* ---------- thumbnail ---------- */
/** Renders the region around the match with the match highlighted. Returns a data URL. */
export async function renderThumb(doc, pageNum, text, cssWidth = 340) {
  const page = await doc.getPage(Math.min(Math.max(1, pageNum), doc.numPages));
  const loc = text ? await locate(page, text) : { rects: [], width: 0, height: 0 };
  const vp1 = page.getViewport({ scale: 1 });
  let cx, cy, cw, ch;
  if (loc.rects.length) {
    const b = bbox(loc.rects);
    cw = Math.min(vp1.width, Math.max(430, b.x1 - b.x0 + 56));
    cx = Math.min(Math.max(0, (b.x0 + b.x1) / 2 - cw / 2), vp1.width - cw);
    ch = Math.min(vp1.height, Math.max(150, Math.min(280, b.y1 - b.y0 + 120)));
    cy = Math.min(Math.max(0, (b.y0 + b.y1) / 2 - ch / 2), vp1.height - ch);
  } else {
    cw = Math.min(vp1.width, 420);
    cx = (vp1.width - cw) / 2;
    ch = Math.min(vp1.height, 200);
    cy = vp1.height * 0.08;
  }
  const dpr = Math.min(2, window.devicePixelRatio || 1);
  const scale = (cssWidth * dpr) / cw;
  const canvas = document.createElement("canvas");
  canvas.width = Math.round(cw * scale);
  canvas.height = Math.round(ch * scale);
  const ctx = canvas.getContext("2d");
  ctx.fillStyle = "#fff";
  ctx.fillRect(0, 0, canvas.width, canvas.height);
  const viewport = page.getViewport({ scale });
  await page.render({ canvasContext: ctx, viewport, transform: [1, 0, 0, 1, -cx * scale, -cy * scale] }).promise;
  ctx.globalCompositeOperation = "multiply";
  ctx.fillStyle = "rgba(255, 208, 0, 0.55)";
  for (const r of loc.rects) {
    const pad = 1.5;
    ctx.fillRect((r.x - cx - pad) * scale, (r.y - cy - pad) * scale, (r.w + pad * 2) * scale, (r.h + pad * 2) * scale);
  }
  return { url: canvas.toDataURL("image/jpeg", 0.88), found: loc.rects.length > 0, w: canvas.width / dpr, h: canvas.height / dpr };
}

/* ---------- viewer ---------- */
const esc = (s) => String(s ?? "").replace(/[&<>"']/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" }[c]));

export async function openViewer({ doc, title, page: targetPage = 1, text = "", onExternal }) {
  const root = document.getElementById("modalRoot");
  root.innerHTML = `<div class="viewer" role="dialog" aria-modal="true" aria-label="${esc(title)}">
    <header class="v-head">
      <div class="v-title">${esc(title)}</div>
      <div class="v-page"><span id="vPage">${targetPage}</span> / ${doc.numPages}</div>
      <div class="v-tools">
        <button class="icon-btn" id="vOut" title="Zoom out">−</button>
        <button class="icon-btn" id="vIn" title="Zoom in">+</button>
        <button class="btn sm" id="vJump" title="Scroll back to the match">Jump to match</button>
        <button class="btn sm" id="vExt">Open in default app</button>
        <button class="icon-btn" id="vClose" title="Close (Esc)">✕</button>
      </div>
    </header>
    <div class="v-scroll" id="vScroll"><div class="v-pages" id="vPages"></div></div></div>`;
  const scroll = root.querySelector("#vScroll"), pagesEl = root.querySelector("#vPages");
  const close = () => { root.innerHTML = ""; document.removeEventListener("keydown", onKey, true); obs.disconnect(); };
  const onKey = (e) => { if (e.key === "Escape") { e.stopPropagation(); close(); } };
  document.addEventListener("keydown", onKey, true);
  root.querySelector("#vClose").onclick = close;
  root.querySelector("#vExt").onclick = () => onExternal?.();

  // page geometry
  const sizes = [];
  for (let i = 1; i <= doc.numPages; i += 20) {
    const batch = await Promise.all(
      Array.from({ length: Math.min(20, doc.numPages - i + 1) }, (_, k) => doc.getPage(i + k).then((p) => p.getViewport({ scale: 1 })))
    );
    batch.forEach((v) => sizes.push([v.width, v.height]));
  }
  const maxW = Math.max(...sizes.map((s) => s[0]));
  let zoom = 1;
  const baseWidth = () => Math.min(scroll.clientWidth - 48, 980);
  const pageEls = [];
  let hl = null; // match rects on target page

  const layout = () => {
    const scale = (baseWidth() * zoom) / maxW;
    pageEls.forEach((el, i) => {
      el.style.width = sizes[i][0] * scale + "px";
      el.style.height = sizes[i][1] * scale + "px";
      el.dataset.rendered = "";
      el.querySelector("canvas")?.remove();
      el.querySelector(".textLayer")?.remove();
    });
    obs.disconnect();
    pageEls.forEach((el) => obs.observe(el));
    return scale;
  };
  const obs = new IntersectionObserver(async (entries) => {
    for (const en of entries) {
      if (!en.isIntersecting) continue;
      const el = en.target;
      if (el.dataset.rendered) continue;
      el.dataset.rendered = "1";
      const n = +el.dataset.n;
      const p = await doc.getPage(n);
      const cssW = parseFloat(el.style.width);
      const dpr = Math.min(2, window.devicePixelRatio || 1);
      const cssScale = cssW / sizes[n - 1][0];
      const scale = cssScale * dpr;
      const vp = p.getViewport({ scale });
      const c = document.createElement("canvas");
      c.width = vp.width; c.height = vp.height;
      c.style.width = "100%"; c.style.height = "100%";
      await p.render({ canvasContext: c.getContext("2d"), viewport: vp }).promise;
      if (!el.dataset.rendered) return;
      el.prepend(c);
      // transparent, selectable text on top of the canvas (copy with Ctrl+C)
      const tl = document.createElement("div");
      tl.className = "textLayer";
      tl.style.setProperty("--scale-factor", cssScale);
      c.after(tl);
      try {
        await new pdfjs.TextLayer({ textContentSource: p.streamTextContent(), container: tl, viewport: p.getViewport({ scale: cssScale }) }).render();
      } catch (e) { console.warn("text layer failed", e); }
    }
  }, { root: scroll, rootMargin: "900px 0px" });

  for (let i = 0; i < doc.numPages; i++) {
    const el = document.createElement("div");
    el.className = "v-pg";
    el.dataset.n = i + 1;
    pagesEl.append(el);
    pageEls.push(el);
  }
  layout();

  // highlight on the target page (percent-based so it survives zooming)
  const tp = Math.min(Math.max(1, targetPage), doc.numPages);
  if (text) {
    const loc = await locate(await doc.getPage(tp), text);
    hl = loc;
    for (const r of loc.rects) {
      const d = document.createElement("div");
      d.className = "v-hl";
      d.style.cssText = `left:${(r.x / loc.width) * 100}%;top:${(r.y / loc.height) * 100}%;width:${(r.w / loc.width) * 100}%;height:${(r.h / loc.height) * 100}%`;
      pageEls[tp - 1].append(d);
    }
  }
  const jump = (smooth) => {
    const el = pageEls[tp - 1];
    let y = el.offsetTop;
    if (hl && hl.rects.length) y += (Math.min(...hl.rects.map((r) => r.y)) / hl.height) * el.offsetHeight - scroll.clientHeight / 3;
    scroll.scrollTo({ top: Math.max(0, y - 12), behavior: smooth ? "smooth" : "auto" });
  };
  jump(false);
  root.querySelector("#vJump").onclick = () => jump(true);
  const zoomBy = (f) => {
    const frac = scroll.scrollTop / Math.max(1, scroll.scrollHeight);
    zoom = Math.min(3, Math.max(0.4, zoom * f));
    layout();
    scroll.scrollTop = frac * scroll.scrollHeight;
  };
  root.querySelector("#vIn").onclick = () => zoomBy(1.2);
  root.querySelector("#vOut").onclick = () => zoomBy(1 / 1.2);
  scroll.addEventListener("scroll", () => {
    const mid = scroll.scrollTop + scroll.clientHeight / 2;
    const cur = pageEls.findIndex((el) => el.offsetTop + el.offsetHeight > mid);
    root.querySelector("#vPage").textContent = (cur < 0 ? pageEls.length : cur + 1);
  }, { passive: true });
  root.querySelector(".viewer").addEventListener("mousedown", (e) => { if (e.target === e.currentTarget) close(); });
}
