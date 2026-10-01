// Host ve yönetim önizlemesi aynı belgeyi kullanır; önizleme masaüstüyle birebir aynı görünür.
import type { Widget } from "./types";

/**
 * Her widget'a eklenen küçük çalışma zamanı: `widget.fetch` ve `widget.system`.
 * İstekler postMessage ile ebeveyne (host) gider, o da Rust'a iletir.
 */
const RUNTIME = `(()=>{let n=0;const p=new Map();
addEventListener("message",e=>{const d=e.data;if(e.source!==parent||!d||d.__widget!==1)return;
const r=p.get(d.id);if(!r)return;p.delete(d.id);d.error!==undefined?r[1](new Error(d.error)):r[0](d.result)});
const call=(kind,args)=>new Promise((res,rej)=>{const id=++n;p.set(id,[res,rej]);parent.postMessage({__widget:1,id,kind,args},"*")});
window.widget={
fetch:(url,o={})=>call("fetch",{url:String(url),method:o.method||"GET",headers:o.headers||{},body:o.body==null?null:String(o.body)})
.then(r=>({...r,ok:r.status>=200&&r.status<300,json:()=>JSON.parse(r.text)})),
system:()=>call("system",{})};})();`;

export function srcdoc(w: Widget): string {
  // Kullanıcı JS'i içindeki </script> belgeyi erken kapatmasın
  const js = w.js.replace(/<\/script/gi, "<\\/script");
  return `<!doctype html><html><head><meta charset="utf-8"><script>${RUNTIME}</script><style>
:root{color-scheme:normal}html,body{margin:0;height:100%;overflow:hidden}
/* html tam şeffafsa body arka planı tuvale yayılır ve border-radius kaybolur; 1/255 alfa (daha küçüğü 0a yuvarlanır) bunu engeller */
html{background:rgba(0,0,0,.004)}body{background:transparent}
${w.css}</style></head><body>${w.html}<script>${js}</script></body></html>`;
}
