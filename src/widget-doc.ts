// Host ve yönetim önizlemesi aynı belgeyi kullanır; önizleme masaüstüyle birebir aynı görünür.
import type { Widget } from "./types";

export function srcdoc(w: Widget): string {
  // Kullanıcı JS'i içindeki </script> belgeyi erken kapatmasın
  const js = w.js.replace(/<\/script/gi, "<\\/script");
  return `<!doctype html><html><head><meta charset="utf-8"><style>
:root{color-scheme:normal}html,body{margin:0;height:100%;overflow:hidden}
/* html tam şeffafsa body arka planı tuvale yayılır ve border-radius kaybolur; 1/255 alfa (daha küçüğü 0a yuvarlanır) bunu engeller */
html{background:rgba(0,0,0,.004)}body{background:transparent}
${w.css}</style></head><body>${w.html}<script>${js}</script></body></html>`;
}
