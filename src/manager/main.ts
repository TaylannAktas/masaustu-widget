import { invoke } from "@tauri-apps/api/core";
import type { Config } from "../types";

const $ = <T extends HTMLElement>(id: string) => document.getElementById(id) as T;
const json = $<HTMLTextAreaElement>("json");
const status = $("status");

function say(msg: string, ok = true) {
  status.textContent = msg;
  status.className = ok ? "ok" : "err";
}

async function load() {
  json.value = JSON.stringify(await invoke<Config>("get_config"), null, 2);
  say("Yüklendi");
}

$("save").onclick = async () => {
  try {
    await invoke("save_config", { config: JSON.parse(json.value) });
    say("Kaydedildi, masaüstü güncellendi");
  } catch (e) {
    say(`Hata: ${e}`, false);
  }
};
$("reload").onclick = load;
$("path").textContent = await invoke<string>("config_path");
load();
