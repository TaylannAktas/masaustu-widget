// Yönetim arayüzü: tüm değişiklikler önce taslakta tutulur, "Kaydet" ile masaüstüne gider.
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { open, save } from "@tauri-apps/plugin-dialog";
import type { Config, Widget } from "../types";
import { srcdoc } from "../widget-doc";

interface MonitorInfo { name: string; primary: boolean; w: number; h: number; scale: number }
type Tab = "html" | "css" | "js";

const $ = <T extends HTMLElement = HTMLElement>(id: string) => document.getElementById(id) as T;
const num = (id: string) => $<HTMLInputElement>(id);

let draft: Config = { widgets: [], settings: { pause_when_covered: true, pause_on_battery: false } };
let savedJson = "";
let selected: string | null = null;
let tab: Tab = "html";
let monitors: MonitorInfo[] = [];

const current = () => draft.widgets.find((w) => w.id === selected) ?? null;
const isDirty = () => JSON.stringify(draft) !== savedJson;
const newId = () => crypto.randomUUID().slice(0, 8);

function say(msg: string, kind: "ok" | "err" | "" = "") {
  const s = $("status");
  s.textContent = msg;
  s.className = kind;
}

// ---------- liste ----------

function renderList() {
  const ul = $("list");
  ul.replaceChildren(
    ...draft.widgets.map((w) => {
      const li = document.createElement("li");
      li.className = w.id === selected ? "active" : "";
      li.onclick = () => select(w.id);
      const name = document.createElement("span");
      name.textContent = w.name || "(adsız)";
      const dot = document.createElement("input");
      dot.type = "checkbox";
      dot.checked = w.enabled;
      dot.title = "Masaüstünde göster";
      dot.onclick = (e) => {
        e.stopPropagation();
        w.enabled = dot.checked;
        if (w.id === selected) $<HTMLInputElement>("enabled").checked = w.enabled;
        changed();
      };
      li.append(dot, name);
      return li;
    }),
  );
}

function select(id: string | null) {
  selected = id;
  const w = current();
  $("empty").hidden = !!w;
  $("editor").hidden = !w;
  renderList();
  if (!w) return;
  $<HTMLInputElement>("name").value = w.name;
  $<HTMLInputElement>("enabled").checked = w.enabled;
  $<HTMLSelectElement>("monitor").value = w.monitor ?? "";
  for (const k of ["x", "y", "w", "h"] as const) num(k).value = String(Math.round(w[k]));
  num("opacity").value = String(w.opacity);
  $("opacityOut").textContent = `${Math.round(w.opacity * 100)}%`;
  showTab(tab);
  refreshPreview(true);
}

// ---------- düzenleyici ----------

function showTab(t: Tab) {
  tab = t;
  document.querySelectorAll<HTMLButtonElement>(".tabs button").forEach((b) => b.classList.toggle("active", b.dataset.tab === t));
  $<HTMLTextAreaElement>("code").value = current()?.[t] ?? "";
}

let previewTimer = 0;
let previewContent = "";
function refreshPreview(now = false) {
  clearTimeout(previewTimer);
  previewTimer = window.setTimeout(() => {
    const w = current();
    if (!w) return;
    const frame = $<HTMLIFrameElement>("preview");
    const doc = srcdoc(w);
    // Kod değişmediyse yeniden yükleme, yalnızca boyut/opaklık güncellensin
    if (doc !== previewContent) {
      frame.srcdoc = doc;
      previewContent = doc;
    }
    const stage = $("stage");
    const scale = Math.min(1, (stage.clientWidth - 24) / w.w, (stage.clientHeight - 24) / w.h);
    Object.assign(frame.style, {
      width: `${w.w}px`, height: `${w.h}px`, opacity: String(w.opacity), transform: `translate(-50%, -50%) scale(${scale})`,
    });
    $("sizeInfo").textContent = `${Math.round(w.w)}×${Math.round(w.h)}${scale < 1 ? ` · %${Math.round(scale * 100)}` : ""}`;
    renderMap();
  }, now ? 0 : 350);
}

function monitorOf(w: Widget): MonitorInfo | undefined {
  return monitors.find((m) => m.name === w.monitor) ?? monitors.find((m) => m.primary) ?? monitors[0];
}

/** Seçili widget'ın monitöründeki tüm widget'ları küçük bir harita olarak çizer. */
function renderMap() {
  const w = current();
  const map = $("map");
  const mon = w && monitorOf(w);
  if (!w || !mon) return map.replaceChildren();
  map.style.aspectRatio = `${mon.w} / ${mon.h}`;
  const rects = draft.widgets
    .filter((o) => o.enabled || o.id === w.id)
    .filter((o) => monitorOf(o)?.name === mon.name)
    .map((o) => {
      const r = document.createElement("div");
      r.className = o.id === w.id ? "rect sel" : "rect";
      r.title = o.name;
      Object.assign(r.style, {
        left: `${(o.x / mon.w) * 100}%`, top: `${(o.y / mon.h) * 100}%`,
        width: `${(o.w / mon.w) * 100}%`, height: `${(o.h / mon.h) * 100}%`,
      });
      r.onclick = () => select(o.id);
      return r;
    });
  map.replaceChildren(...rects);
}

function renderSettings() {
  $<HTMLInputElement>("pauseCovered").checked = draft.settings.pause_when_covered;
  $<HTMLInputElement>("pauseBattery").checked = draft.settings.pause_on_battery;
}

function changed() {
  $<HTMLButtonElement>("save").disabled = !isDirty();
  // document.title yerel pencere başlığına yansımıyor
  getCurrentWindow().setTitle(isDirty() ? "● Masaüstü Widget" : "Masaüstü Widget");
  if (isDirty()) say("Kaydedilmemiş değişiklikler var");
  else if ($("status").textContent === "Kaydedilmemiş değişiklikler var") say("");
}

function bind() {
  $<HTMLInputElement>("pauseCovered").onchange = (e) => {
    draft.settings.pause_when_covered = (e.target as HTMLInputElement).checked;
    changed();
  };
  $<HTMLInputElement>("pauseBattery").onchange = (e) => {
    draft.settings.pause_on_battery = (e.target as HTMLInputElement).checked;
    changed();
  };
  $<HTMLInputElement>("name").oninput = (e) => {
    current()!.name = (e.target as HTMLInputElement).value;
    renderList();
    changed();
  };
  $<HTMLInputElement>("enabled").onchange = (e) => {
    current()!.enabled = (e.target as HTMLInputElement).checked;
    renderList();
    renderMap();
    changed();
  };
  $<HTMLSelectElement>("monitor").onchange = (e) => {
    current()!.monitor = (e.target as HTMLSelectElement).value || null;
    renderMap();
    changed();
  };
  for (const k of ["x", "y", "w", "h"] as const) {
    num(k).oninput = () => {
      const v = Number(num(k).value);
      if (Number.isNaN(v)) return;
      current()![k] = k === "w" || k === "h" ? Math.max(20, v) : v;
      refreshPreview();
      changed();
    };
  }
  num("opacity").oninput = () => {
    const v = Number(num("opacity").value);
    current()!.opacity = v;
    $("opacityOut").textContent = `${Math.round(v * 100)}%`;
    refreshPreview(true);
    changed();
  };

  const code = $<HTMLTextAreaElement>("code");
  code.oninput = () => {
    current()![tab] = code.value;
    refreshPreview();
    changed();
  };
  // Tab tuşu odak değiştirmesin, iki boşluk eklesin
  code.onkeydown = (e) => {
    if (e.key !== "Tab") return;
    e.preventDefault();
    code.setRangeText("  ", code.selectionStart, code.selectionEnd, "end");
    code.dispatchEvent(new Event("input"));
  };
  document.querySelectorAll<HTMLButtonElement>(".tabs button").forEach((b) => (b.onclick = () => showTab(b.dataset.tab as Tab)));

  $("new").onclick = () => {
    const w: Widget = {
      id: newId(), name: "Yeni widget", monitor: null, x: 80, y: 80, w: 320, h: 180, opacity: 1, enabled: true,
      html: `<div class="card">Merhaba!</div>`,
      css: `.card {\n  height: 100vh; display: grid; place-items: center;\n  color: #fff; font: 600 28px system-ui;\n  background: rgba(20, 30, 60, .5); border-radius: 16px;\n}`,
      js: "",
    };
    draft.widgets.push(w);
    select(w.id);
    changed();
  };
  $("dup").onclick = () => {
    const w = current()!;
    const copy = { ...w, id: newId(), name: `${w.name} (kopya)`, x: w.x + 24, y: w.y + 24 };
    draft.widgets.splice(draft.widgets.indexOf(w) + 1, 0, copy);
    select(copy.id);
    changed();
  };
  // Tarayıcı confirm() yerine iki aşamalı buton: ilk tık uyarır, ikincisi siler
  const del = $<HTMLButtonElement>("del");
  del.onclick = () => {
    if (!del.classList.contains("confirm")) {
      del.classList.add("confirm");
      del.textContent = "Emin misin?";
      setTimeout(() => (del.classList.remove("confirm"), (del.textContent = "Sil")), 3000);
      return;
    }
    const i = draft.widgets.findIndex((w) => w.id === selected);
    draft.widgets.splice(i, 1);
    del.classList.remove("confirm");
    del.textContent = "Sil";
    select(draft.widgets[Math.min(i, draft.widgets.length - 1)]?.id ?? null);
    changed();
  };

  $("save").onclick = persist;
  document.addEventListener("keydown", (e) => {
    if (e.ctrlKey && e.key.toLowerCase() === "s") {
      e.preventDefault();
      if (isDirty()) persist();
    }
  });

  $("export").onclick = async () => {
    const path = await save({ defaultPath: "widgets.json", filters: [{ name: "JSON", extensions: ["json"] }] });
    if (!path) return;
    try {
      await invoke("export_config", { path, config: draft });
      say(`${draft.widgets.length} widget dışa aktarıldı`, "ok");
    } catch (e) {
      say(`Dışa aktarılamadı: ${e}`, "err");
    }
  };
  $("import").onclick = async () => {
    const path = await open({ filters: [{ name: "JSON", extensions: ["json"] }] });
    if (typeof path !== "string") return;
    try {
      const incoming = await invoke<Widget[]>("read_widgets_file", { path });
      // Mevcutların üzerine yazma: çakışan kimlik yeni kimlik alır
      const ids = new Set(draft.widgets.map((w) => w.id));
      for (const w of incoming) {
        if (!w.id || ids.has(w.id)) w.id = newId();
        ids.add(w.id);
        draft.widgets.push(w);
      }
      select(incoming[0]?.id ?? selected);
      changed();
      say(`${incoming.length} widget eklendi — masaüstüne uygulamak için Kaydet`, "ok");
    } catch (e) {
      say(`İçe aktarılamadı: ${e}`, "err");
    }
  };

  new ResizeObserver(() => refreshPreview(true)).observe($("stage"));
}

async function persist() {
  try {
    await invoke("save_config", { config: draft });
    savedJson = JSON.stringify(draft);
    changed();
    say("Kaydedildi, masaüstü güncellendi", "ok");
  } catch (e) {
    say(`Kaydedilemedi: ${e}`, "err");
  }
}

async function init() {
  monitors = await invoke<MonitorInfo[]>("list_monitors");
  $<HTMLSelectElement>("monitor").replaceChildren(
    new Option("Ana monitör (otomatik)", ""),
    ...monitors.map((m, i) =>
      new Option(`${i + 1}. ${m.name.replace(/^\\\\\.\\/, "")} — ${Math.round(m.w)}×${Math.round(m.h)}${m.primary ? " (ana)" : ""}`, m.name),
    ),
  );
  draft = await invoke<Config>("get_config");
  savedJson = JSON.stringify(draft);
  $("path").textContent = await invoke<string>("config_path");
  bind();
  renderSettings();
  select(draft.widgets[0]?.id ?? null);

  $("editMode").onclick = async () => {
    // Kaydedilmemiş taslak düzenleme moduna yansımaz; önce kaydettir
    if (isDirty()) return say("Düzenleme moduna geçmeden önce kaydet", "err");
    await invoke("start_edit");
  };
  await listen<boolean>("edit-mode", (e) => {
    $<HTMLButtonElement>("editMode").disabled = e.payload;
    if (e.payload) say("Masaüstünde düzenleniyor…");
  });
  // Ayarlar başka yerden (düzenleme modu) değişince taslağı tazele; kaydedilmemiş iş varsa ezme
  await listen("config-changed", async () => {
    const fresh = await invoke<Config>("get_config");
    if (JSON.stringify(fresh) === savedJson) return;
    if (isDirty()) return say("Ayarlar başka yerden değişti — kaydedersen o değişikliklerin üzerine yazılır", "err");
    draft = fresh;
    savedJson = JSON.stringify(fresh);
    renderSettings();
    select(current() ? selected : (draft.widgets[0]?.id ?? null));
    changed();
    say("Masaüstündeki değişiklikler alındı", "ok");
  });
  changed();
  say(`${draft.widgets.length} widget yüklendi`);
}

init();
