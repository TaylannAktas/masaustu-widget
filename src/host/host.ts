// Monitör başına tek host: widget'lar yalıtılmış iframe'lerde çalışır, Tauri IPC'ye erişemez.
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { Widget } from "../types";

const frames = new Map<string, { el: HTMLIFrameElement; content: string }>();
let paused = false;

function srcdoc(w: Widget): string {
  // Kullanıcı JS'i içindeki </script> belgeyi erken kapatmasın
  const js = w.js.replace(/<\/script/gi, "<\/script");
  return `<!doctype html><html><head><meta charset="utf-8"><style>
:root{color-scheme:normal}html,body{margin:0;height:100%;overflow:hidden}
/* html tam şeffafsa body arka planı tuvale yayılır ve border-radius kaybolur; 1/255 alfa (daha küçüğü 0a yuvarlanır) bunu engeller */
html{background:rgba(0,0,0,.004)}body{background:transparent}
${w.css}</style></head><body>${w.html}<script>${js}</script></body></html>`;
}

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
