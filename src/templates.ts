// "+ Yeni widget" menüsündeki hazır şablonlar. Hepsi düz HTML/CSS/JS; kopyalanıp serbestçe değiştirilebilir.
import type { Widget } from "./types";

export type Template = Pick<Widget, "name" | "w" | "h" | "html" | "css" | "js"> & { key: string; hint: string };

// Ortak cam kart görünümü
const CARD = `body {
  box-sizing: border-box; height: 100vh; padding: 14px 16px;
  color: #e8ecf4; font: 14px "Segoe UI Variable Text", system-ui, sans-serif;
  background: rgba(16, 20, 32, .55); border: 1px solid rgba(255, 255, 255, .08); border-radius: 16px;
}
.muted { color: #9aa3b5; }
`;

export const TEMPLATES: Template[] = [
  {
    key: "blank", name: "Yeni widget", hint: "Boş kart", w: 320, h: 180,
    html: `<div class="card">Merhaba!</div>`,
    css: `.card {
  height: 100vh; display: grid; place-items: center;
  color: #fff; font: 600 28px system-ui;
  background: rgba(20, 30, 60, .5); border-radius: 16px;
}`,
    js: "",
  },
  {
    key: "clock", name: "Saat", hint: "Saat ve tarih", w: 320, h: 160,
    html: `<div id="time"></div>\n<div id="date" class="muted"></div>`,
    css: CARD + `body { display: grid; place-content: center; text-align: center; gap: 2px; }
#time { font: 600 56px/1 "Segoe UI Variable Display", system-ui; font-variant-numeric: tabular-nums; }`,
    js: `const tick = () => {
  const now = new Date();
  time.textContent = now.toLocaleTimeString("tr-TR", { hour: "2-digit", minute: "2-digit" });
  date.textContent = now.toLocaleDateString("tr-TR", { weekday: "long", day: "numeric", month: "long" });
};
tick();
setInterval(tick, 1000);`,
  },
  {
    key: "image", name: "Görsel / GIF", hint: "Bir resim ya da GIF adresi", w: 300, h: 300,
    html: `<!-- Adresi kendi görselin ya da GIF'inle değiştir -->
<img src="https://upload.wikimedia.org/wikipedia/commons/2/2c/Rotating_earth_%28large%29.gif" alt="">`,
    css: `img { width: 100vw; height: 100vh; object-fit: cover; border-radius: 16px; display: block; }`,
    js: "",
  },
  {
    key: "crypto-table", name: "Kripto fiyatları", hint: "Canlı tablo (Binance, 15 sn)", w: 340, h: 230,
    html: `<table>
  <thead><tr><th>Varlık</th><th>Fiyat ($)</th><th>24 sa</th></tr></thead>
  <tbody id="rows"><tr><td colspan="3" class="muted">Yükleniyor…</td></tr></tbody>
</table>
<div id="foot" class="muted"></div>`,
    css: CARD + `table { width: 100%; border-collapse: collapse; font-variant-numeric: tabular-nums; }
th { text-align: left; font-weight: 500; color: #9aa3b5; font-size: 12px; padding-bottom: 6px; }
td { padding: 5px 0; border-top: 1px solid rgba(255, 255, 255, .06); }
th:not(:first-child), td:not(:first-child) { text-align: right; }
.up { color: #5fd38d; } .down { color: #ff7b7b; }
#foot { font-size: 11px; margin-top: 6px; }`,
    js: `const SYMBOLS = ["BTC", "ETH", "SOL", "BNB", "XRP"];

async function refresh() {
  try {
    const q = encodeURIComponent(JSON.stringify(SYMBOLS.map((s) => s + "USDT")));
    // widget.fetch: istek uygulama üzerinden gider, CORS'a takılmaz
    const res = await widget.fetch("https://api.binance.com/api/v3/ticker/24hr?symbols=" + q);
    const data = res.json();
    rows.innerHTML = data.map((t) => {
      const ch = Number(t.priceChangePercent);
      const price = Number(t.lastPrice).toLocaleString("tr-TR", { maximumFractionDigits: Number(t.lastPrice) < 10 ? 4 : 2 });
      return "<tr><td>" + t.symbol.replace("USDT", "") + "</td><td>" + price + "</td><td class='" + (ch >= 0 ? "up" : "down") + "'>" + (ch >= 0 ? "+" : "") + ch.toFixed(2) + "%</td></tr>";
    }).join("");
    foot.textContent = "Güncellendi " + new Date().toLocaleTimeString("tr-TR");
  } catch (e) {
    foot.textContent = "Bağlanılamadı: " + e.message;
  }
}
refresh();
setInterval(refresh, 15000);`,
  },
  {
    key: "crypto-chart", name: "Canlı grafik", hint: "BTC/USDT, anlık (WebSocket)", w: 420, h: 220,
    html: `<div class="head"><b id="sym">BTC/USDT</b><span id="price">—</span><span id="change" class="muted"></span></div>
<canvas id="chart"></canvas>`,
    css: CARD + `body { display: flex; flex-direction: column; gap: 8px; }
.head { display: flex; align-items: baseline; gap: 10px; font-variant-numeric: tabular-nums; }
#price { font: 600 22px system-ui; }
.up { color: #5fd38d; } .down { color: #ff7b7b; }
canvas { flex: 1; min-height: 0; width: 100%; }`,
    js: `const SYMBOL = "BTCUSDT";      // ör. ETHUSDT
const INTERVAL = "1m";         // mum aralığı
const POINTS = 120;            // grafikte kaç mum

let closes = [];
const ctx = chart.getContext("2d");

function draw() {
  const dpr = devicePixelRatio || 1;
  const w = chart.clientWidth, h = chart.clientHeight;
  if (chart.width !== w * dpr) { chart.width = w * dpr; chart.height = h * dpr; }
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
  ctx.clearRect(0, 0, w, h);
  if (closes.length < 2) return;
  const min = Math.min(...closes), max = Math.max(...closes), span = max - min || 1;
  const x = (i) => (i / (closes.length - 1)) * w;
  const y = (v) => h - 4 - ((v - min) / span) * (h - 8);
  const up = closes.at(-1) >= closes[0];
  const color = up ? "#5fd38d" : "#ff7b7b";
  ctx.beginPath();
  closes.forEach((v, i) => (i ? ctx.lineTo(x(i), y(v)) : ctx.moveTo(x(i), y(v))));
  ctx.strokeStyle = color; ctx.lineWidth = 1.6; ctx.stroke();
  ctx.lineTo(w, h); ctx.lineTo(0, h); ctx.closePath();
  const g = ctx.createLinearGradient(0, 0, 0, h);
  g.addColorStop(0, color + "55"); g.addColorStop(1, color + "00");
  ctx.fillStyle = g; ctx.fill();

  const last = closes.at(-1), pct = ((last - closes[0]) / closes[0]) * 100;
  price.textContent = last.toLocaleString("tr-TR", { maximumFractionDigits: 2 });
  change.textContent = (pct >= 0 ? "+" : "") + pct.toFixed(2) + "%";
  change.className = up ? "up" : "down";
}

async function loadHistory() {
  const res = await widget.fetch("https://api.binance.com/api/v3/klines?symbol=" + SYMBOL + "&interval=" + INTERVAL + "&limit=" + POINTS);
  closes = res.json().map((k) => Number(k[4]));
  draw();
}

function connect() {
  const ws = new WebSocket("wss://stream.binance.com:9443/ws/" + SYMBOL.toLowerCase() + "@kline_" + INTERVAL);
  let openTime = null;
  ws.onmessage = (e) => {
    const k = JSON.parse(e.data).k;
    if (openTime !== null && k.t !== openTime) { closes.push(Number(k.c)); if (closes.length > POINTS) closes.shift(); }
    else closes[closes.length - 1] = Number(k.c);
    openTime = k.t;
    draw();
  };
  // Bağlantı koparsa (uyku, ağ) geçmişi tazeleyip yeniden bağlan
  ws.onclose = () => setTimeout(() => loadHistory().then(connect, connect), 5000);
}

sym.textContent = SYMBOL.replace("USDT", "/USDT");
new ResizeObserver(draw).observe(chart);
loadHistory().then(connect, (e) => { price.textContent = "Bağlanılamadı"; change.textContent = e.message; setTimeout(() => loadHistory().then(connect), 10000); });`,
  },
  {
    key: "system", name: "Sistem", hint: "CPU, RAM ve çalışma süresi", w: 300, h: 160,
    html: `<div class="row"><span>CPU</span><b id="cpuT"></b></div><div class="bar"><i id="cpuB"></i></div>
<div class="row"><span>RAM</span><b id="memT"></b></div><div class="bar"><i id="memB"></i></div>
<div id="up" class="muted"></div>`,
    css: CARD + `.row { display: flex; justify-content: space-between; margin-top: 4px; font-variant-numeric: tabular-nums; }
.bar { height: 6px; border-radius: 3px; background: rgba(255, 255, 255, .08); margin: 5px 0 8px; overflow: hidden; }
.bar i { display: block; height: 100%; width: 0; border-radius: 3px; background: #7c9cff; transition: width .6s; }
#up { font-size: 12px; }`,
    js: `async function refresh() {
  // widget.system: uygulamadan CPU/RAM bilgisi
  const s = await widget.system();
  cpuT.textContent = s.cpu.toFixed(0) + "%";
  cpuB.style.width = s.cpu + "%";
  memT.textContent = (s.mem_used_mb / 1024).toFixed(1) + " / " + (s.mem_total_mb / 1024).toFixed(1) + " GB";
  memB.style.width = s.mem_percent + "%";
  const h = Math.floor(s.uptime_s / 3600), m = Math.floor((s.uptime_s % 3600) / 60);
  up.textContent = "Açık kalma süresi: " + h + " sa " + m + " dk";
}
refresh();
setInterval(refresh, 2000);`,
  },
  {
    key: "weather", name: "Hava durumu", hint: "Open-Meteo (anahtar gerekmez)", w: 300, h: 150,
    html: `<div class="top"><span id="icon">…</span><div><div id="temp">—</div><div id="desc" class="muted"></div></div></div>
<div id="meta" class="muted"></div>`,
    css: CARD + `.top { display: flex; align-items: center; gap: 14px; }
#icon { font-size: 48px; line-height: 1; }
#temp { font: 600 34px system-ui; font-variant-numeric: tabular-nums; }
#meta { margin-top: 10px; font-size: 12px; }`,
    js: `// Konumu değiştir: enlem / boylam (varsayılan Ankara)
const PLACE = { name: "Ankara", lat: 39.93, lon: 32.86 };

const CODES = {
  0: ["☀️", "Açık"], 1: ["🌤️", "Az bulutlu"], 2: ["⛅", "Parçalı bulutlu"], 3: ["☁️", "Kapalı"],
  45: ["🌫️", "Sisli"], 48: ["🌫️", "Kırağı sisi"], 51: ["🌦️", "Çisenti"], 53: ["🌦️", "Çisenti"], 55: ["🌦️", "Yoğun çisenti"],
  61: ["🌧️", "Hafif yağmur"], 63: ["🌧️", "Yağmur"], 65: ["🌧️", "Kuvvetli yağmur"], 71: ["🌨️", "Hafif kar"], 73: ["🌨️", "Kar"],
  75: ["❄️", "Yoğun kar"], 80: ["🌦️", "Sağanak"], 81: ["🌧️", "Sağanak"], 82: ["⛈️", "Şiddetli sağanak"], 95: ["⛈️", "Gök gürültülü"],
};

async function refresh() {
  try {
    const url = "https://api.open-meteo.com/v1/forecast?latitude=" + PLACE.lat + "&longitude=" + PLACE.lon +
      "&current=temperature_2m,weather_code,wind_speed_10m&daily=temperature_2m_max,temperature_2m_min&timezone=auto&forecast_days=1";
    const d = (await widget.fetch(url)).json();
    const [ic, text] = CODES[d.current.weather_code] ?? ["🌡️", "—"];
    icon.textContent = ic;
    temp.textContent = Math.round(d.current.temperature_2m) + "°";
    desc.textContent = PLACE.name + " · " + text;
    meta.textContent = "En yüksek " + Math.round(d.daily.temperature_2m_max[0]) + "° · en düşük " +
      Math.round(d.daily.temperature_2m_min[0]) + "° · rüzgâr " + Math.round(d.current.wind_speed_10m) + " km/sa";
  } catch (e) {
    desc.textContent = "Bağlanılamadı: " + e.message;
  }
}
refresh();
setInterval(refresh, 15 * 60 * 1000);`,
  },
];
