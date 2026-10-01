// Monitör başına tek host: widget'lar yalıtılmış iframe'lerde çalışır, Tauri IPC'ye erişemez.
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { Widget } from "../types";
import { srcdoc } from "../widget-doc";

const frames = new Map<string, { el: HTMLIFrameElement; content: string }>();
let paused = false;

function place(el: HTMLIFrameElement, w: Widget) {
  Object.assign(el.style, {
    left: `${w.x}px`, top: `${w.y}px`, width: `${w.w}px`, height: `${w.h}px`,
    opacity: String(w.opacity),
  });
}

function clear() {
  frames.forEach((f) => f.el.remove());
  frames.clear();
}

async function render() {
  // Duraklatınca iframe'ler tamamen kaldırılır: timer'lar, WebSocket'ler ve animasyonlar durur
  if (paused) return clear();
  const list = await invoke<Widget[]>("host_widgets");
  const seen = new Set<string>();
  for (const w of list) {
    seen.add(w.id);
    const content = srcdoc(w);
    let f = frames.get(w.id);
    if (!f) {
      const el = document.createElement("iframe");
      el.setAttribute("sandbox", "allow-scripts");
      el.setAttribute("allowtransparency", "true");
      document.body.append(el);
      f = { el, content: "" };
      frames.set(w.id, f);
    }
    // Yalnızca içerik değiştiyse yeniden yükle; konum/opaklık değişimi widget'ı sıfırlamaz
    if (f.content !== content) {
      f.el.srcdoc = content;
      f.content = content;
    }
    place(f.el, w);
  }
  for (const [id, f] of frames) if (!seen.has(id)) (f.el.remove(), frames.delete(id));
}

paused = await invoke<boolean>("is_paused");
await listen("config-changed", render);
await listen<boolean>("paused", (e) => {
  paused = e.payload;
  render();
});
render();
