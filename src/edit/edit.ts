// Düzenleme katmanı: widget çerçevelerini sürükle/boyutlandır, değişiklikler canlı olarak host'a gider.
import { invoke } from "@tauri-apps/api/core";
import type { Widget } from "../types";
import { srcdoc } from "../widget-doc";
import { installBridge } from "../bridge";

installBridge();

type Dir = "n" | "s" | "e" | "w" | "ne" | "nw" | "se" | "sw";
const MIN = 40;
const DIRS: Dir[] = ["nw", "n", "ne", "e", "se", "s", "sw", "w"];

const gridBox = document.getElementById("grid") as HTMLInputElement;
const boxes = new Map<string, { w: Widget; el: HTMLDivElement; tag: HTMLSpanElement }>();
let selected: string | null = null;

const snap = (v: number, free: boolean) => (free || !gridBox.checked ? Math.round(v) : Math.round(v / 8) * 8);

function draw(id: string) {
  const b = boxes.get(id)!;
  const { x, y, w, h } = b.w;
  Object.assign(b.el.style, { left: `${x}px`, top: `${y}px`, width: `${w}px`, height: `${h}px` });
  b.tag.textContent = `${b.w.name} · ${Math.round(x)}, ${Math.round(y)} · ${Math.round(w)}×${Math.round(h)}`;
  // Ekranın üstüne yapışıksa etiketi kutunun içine al
  b.tag.style.top = y < 30 ? "4px" : "-26px";
  b.tag.style.left = y < 30 ? "4px" : "0";
}

// Host'a giden güncellemeleri kare başına bire indir
let pending: Widget | null = null;
function push(w: Widget) {
  if (!pending) requestAnimationFrame(() => {
    const p = pending!;
    pending = null;
    invoke("edit_move", { id: p.id, x: p.x, y: p.y, w: p.w, h: p.h });
  });
  pending = w;
}

function select(id: string | null) {
  selected = id;
  boxes.forEach((b, k) => b.el.classList.toggle("sel", k === id));
}

function clampMove(w: Widget) {
  w.x = Math.min(Math.max(0, w.x), innerWidth - w.w);
  w.y = Math.min(Math.max(0, w.y), innerHeight - w.h);
}

function startDrag(e: PointerEvent, b: { w: Widget; el: HTMLDivElement }, dir: Dir | null) {
  e.preventDefault();
  e.stopPropagation();
  select(b.w.id);
  const start = { px: e.clientX, py: e.clientY, x: b.w.x, y: b.w.y, w: b.w.w, h: b.w.h };
  const target = e.currentTarget as HTMLElement;
  target.setPointerCapture(e.pointerId);

  const move = (ev: PointerEvent) => {
    const dx = ev.clientX - start.px;
    const dy = ev.clientY - start.py;
    const free = ev.altKey;
    const w = b.w;
    if (!dir) {
      w.x = snap(start.x + dx, free);
      w.y = snap(start.y + dy, free);
      clampMove(w);
    } else {
      // Karşı kenar sabit kalır; sınırlar monitör dışına taşmaz
      if (dir.includes("e")) w.w = Math.max(MIN, Math.min(snap(start.x + start.w + dx, free), innerWidth) - start.x);
      if (dir.includes("s")) w.h = Math.max(MIN, Math.min(snap(start.y + start.h + dy, free), innerHeight) - start.y);
      if (dir.includes("w")) {
        const left = Math.min(Math.max(0, snap(start.x + dx, free)), start.x + start.w - MIN);
        w.w = start.x + start.w - left;
        w.x = left;
      }
      if (dir.includes("n")) {
        const top = Math.min(Math.max(0, snap(start.y + dy, free)), start.y + start.h - MIN);
        w.h = start.y + start.h - top;
        w.y = top;
      }
    }
    draw(w.id);
    push(w);
  };
  const up = () => {
    target.removeEventListener("pointermove", move);
    target.removeEventListener("pointerup", up);
  };
  target.addEventListener("pointermove", move);
  target.addEventListener("pointerup", up);
}

function build(list: Widget[]) {
  for (const w of list) {
    const el = document.createElement("div");
    el.className = "box";
    // Katman pencerelerin üstünde durduğu için gerçek widget görünmeyebilir; canlı kopyası gösterilir
    const live = document.createElement("iframe");
    live.setAttribute("sandbox", "allow-scripts");
    live.srcdoc = srcdoc(w);
    live.style.opacity = String(w.opacity);
    el.append(live);
    const tag = document.createElement("span");
    tag.className = "tag";
    el.append(tag);
    const b = { w, el, tag };
    for (const d of DIRS) {
      const h = document.createElement("div");
      h.className = "h";
      h.dataset.d = d;
      h.onpointerdown = (e) => startDrag(e, b, d);
      el.append(h);
    }
    el.onpointerdown = (e) => startDrag(e, b, null);
    document.body.append(el);
    boxes.set(w.id, b);
    draw(w.id);
  }
  if (!list.length) {
    const empty = document.createElement("div");
    empty.className = "empty";
    empty.textContent = "Bu monitörde gösterilen widget yok";
    document.body.append(empty);
  }
}

const finish = (save: boolean) => invoke("end_edit", { save });

document.getElementById("cancel")!.onclick = () => finish(false);
document.getElementById("done")!.onclick = () => finish(true);
document.body.onpointerdown = () => select(null);
document.addEventListener("keydown", (e) => {
  if (e.key === "Escape") return finish(false);
  if (e.key === "Enter") return finish(true);
  const b = selected && boxes.get(selected);
  const step = e.shiftKey ? 10 : 1;
  const delta = { ArrowLeft: [-step, 0], ArrowRight: [step, 0], ArrowUp: [0, -step], ArrowDown: [0, step] }[e.key];
  if (!b || !delta) return;
  e.preventDefault();
  b.w.x += delta[0];
  b.w.y += delta[1];
  clampMove(b.w);
  draw(b.w.id);
  push(b.w);
});

build(await invoke<Widget[]>("host_widgets"));
