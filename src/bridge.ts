// Widget iframe'lerinden gelen widget.fetch / widget.system isteklerini Rust'a iletir.
// Host, yönetim önizlemesi ve düzenleme katmanı aynı dinleyiciyi kullanır.
import { invoke } from "@tauri-apps/api/core";

interface BridgeMessage {
  __widget: 1;
  id: number;
  kind: "fetch" | "system";
  args: Record<string, unknown>;
}

const commands = { fetch: "widget_fetch", system: "system_info" } as const;

export function installBridge() {
  window.addEventListener("message", async (e) => {
    const msg = e.data as BridgeMessage;
    if (!msg || msg.__widget !== 1 || !(msg.kind in commands)) return;
    // Yalnızca bu sayfadaki iframe'lerden gelen istekler kabul edilir
    const fromOurFrame = [...document.querySelectorAll("iframe")].some((f) => f.contentWindow === e.source);
    if (!fromOurFrame || !e.source) return;
    const reply = (body: { result?: unknown; error?: string }) =>
      (e.source as Window).postMessage({ __widget: 1, id: msg.id, ...body }, "*");
    try {
      const args = msg.kind === "fetch" ? { request: msg.args } : {};
      reply({ result: await invoke(commands[msg.kind], args) });
    } catch (err) {
      reply({ error: String(err) });
    }
  });
}
