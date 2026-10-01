# CLAUDE.md — Masaüstü Widget geliştirme rehberi

Bu dosya, projede çalışmaya başlayan bir ajanın (ya da insanın) ihtiyaç duyduğu her şeyi içerir. Kullanıcıya ait kullanım belgesi `README.md`'de.

## Çalışma kuralları (proje sahibinin tercihleri)

- **İletişim Türkçe.** Commit/PR başlıkları Türkçe ve **edilgen çatıda**: "Düzenleme modu eklendi", "Gömme hatası düzeltildi" (emir kipi değil). Commit gövdesinde madde madde ne değiştiği.
- **Commit kimliği:** Yeni bir klonda ilk iş, aksi halde genel git ayarındaki okul e-postası public repoya sızar:
  ```
  git config user.name "TaylannAktas"
  git config user.email "245744281+TaylannAktas@users.noreply.github.com"
  ```
- **Başlarken güncellik:** `git fetch` + `git status -sb` ile geride/ileride durumunu raporla; fetch'siz status yanıltır.
- **Önce çalışan basit sürüm.** Aşırı mühendislik yok, bağımlılık eklemeden önce düşün (şu ana kadar eklenen her paketin gerekçesi aşağıda).
- **Yeni kütüphane/API kullanmadan önce güncel dokümantasyonu doğrula** (context7 ya da resmi doküman); Tauri ve WebView2 davranışlarını varsayma.
- **Çok dosyalı değişiklikten önce kısa plan yaz** (onay beklemeden uygula); küçük düzeltmeleri doğrudan yap.
- **"Bitti" demeden önce kanıt göster:** komut çıktısı, ekran görüntüsü, ölçüm. Bu projede bunun için `scripts/dev/` betikleri var (aşağıda). Test edemediğin şeyi açıkça "denenmedi" diye söyle.
- Kodda kişisel veri (yol, e-posta, anahtar) olmasın; repo public.

## Proje özeti

Windows masaüstüne, **duvar kağıdı katmanına (ikonların arkasına)** gömülen, tıklama almayan, içeriği serbest HTML/CSS/JS olan widget'lar. Sistem tepsisinde yaşar, açılışta başlar. En önemli gereksinim **düşük kaynak kullanımı**.

Kilitlenmiş kararlar (değiştirmeden önce kullanıcıya sor):

- **Tauri 2** (Rust + WebView2), frontend **Vite + düz TypeScript** (framework yok).
- **Yalnızca Windows.** Android planlanmıştı, iptal edildi.
- İçerik yalnızca **özel HTML/JS**; widget'lar `sandbox="allow-scripts"` iframe `srcdoc` içinde çalışır.
- **Her monitöre tek host penceresi**, widget'lar onun içinde iframe. Widget başına pencere açma (RAM).
- Cihazlar arası senkron yok; **JSON içe/dışa aktarma** var.
- Dağıtım: GitHub Releases, **NSIS (önerilen) + MSI**, imzasız.

## Ortam kurulumu (yeni bilgisayar)

1. Rust stable MSVC (`rustup`), Visual Studio C++ Build Tools, Node.js 20+, Git, `gh` CLI.
2. ```
   gh repo clone TaylannAktas/masaustu-widget && cd masaustu-widget
   git config user.name "TaylannAktas"
   git config user.email "245744281+TaylannAktas@users.noreply.github.com"
   npm install
   npm run tauri dev
   ```
3. Kurulu sürüm (`%LOCALAPPDATA%\Masaüstü Widget\`) çalışıyorsa **önce tepsiden kapat**: uygulama tek örnek çalışır, dev sürümü açılınca kendini kapatıp kurulu olanın yönetim penceresini açar.
4. Etiket push edip sürüm çıkaracaksan: `gh auth refresh -h github.com -s workflow` (workflow dosyası push etmek için de gerekir).

Komutlar:

| Komut | Ne |
|---|---|
| `npm run tauri dev` | Geliştirme. Rust değişince yeniden derler; TS/HTML değişince sayfayı yeniler |
| `npx tsc --noEmit` | Tip denetimi |
| `cd src-tauri && cargo test --lib` | Birim testleri (config, içe/dışa aktarma, köprü güvenliği) |
| `npm run tauri build` | Release + `src-tauri/target/release/bundle/{nsis,msi}/` |
| `npm run tauri build -- --no-bundle` | Yalnızca exe (ölçüm için hızlı) |

Ajan olarak dev sürecini arka planda başlatırken zaman aşımını uzun ver (ör. 2 saat); varsayılan kısa süre dolunca süreç öldürülür.

## Mimari

```
src-tauri/src/
  lib.rs       AppState, Tauri komutları, host pencereleri, tepsi, gözcü, otomatik dondurma, düzenleme modu
  desktop.rs   Win32: WorkerW/Progman'a gömme, z-sırası koruma, kaplanma/kilit/pil algılama
  config.rs    widgets.json modeli (Widget, Settings, Config), atomik kaydetme
  bridge.rs    widget.fetch (ureq + Windows TLS) ve widget.system (CPU/RAM)
src/
  host/host.ts       monitör başına gömülü sayfa: widget iframe'lerini dizer, yalnızca değişeni yeniler
  manager/           yönetim penceresi (liste, editör, önizleme, monitör haritası, şablon menüsü)
  edit/edit.ts       düzenleme katmanı (sürükle/boyutlandır)
  widget-doc.ts      widget srcdoc'unu üretir + widget.* çalışma zamanı (host, önizleme, katman aynı belgeyi kullanır)
  bridge.ts          iframe'lerden gelen widget.* isteklerini Rust'a iletir
  templates.ts       hazır şablonlar
  types.ts           Widget/Config/Settings tipleri (config.rs ile eşit tutulmalı)
host.html, index.html (yönetim), edit.html     vite çoklu sayfa girişleri (vite.config.ts)
scripts/dev/        doğrulama betikleri
```

**Pencereler:**

| Etiket | Ne | Not |
|---|---|---|
| `host-{kuşak}-{i}` | Monitör başına gömülü host | Yeniden kurulumda kuşak artar, etiket çakışmaz |
| `main` | Yönetim penceresi | İstek üzerine açılır, kapatınca yok edilir |
| `edit-{i}` | Düzenleme katmanı | En üstte, geçici |

Yetkiler: `capabilities/default.json` (main: core + opener + dialog + set-title), `capabilities/host.json` (host-*, edit-*: yalnızca core). Host'a/widget'a yetki ekleme.

**Veri akışı:**
- Config `%APPDATA%\com.taylan.masaustuwidget\widgets.json`. `save_config` → dosya + `config-changed` olayı → host'lar `host_widgets` ile kendi monitörlerinin widget'larını çeker.
- Widget konumu monitörün sol üstüne göre **CSS pikseli** (DPI'dan bağımsız). `monitor: null` ya da bilinmeyen monitör → ana monitör.
- Düzenleme modu: Rust'ta `edit_draft` taslağı. Host'lar düzenleme sürerken taslağı okur (canlı hareket). `end_edit(save)` yalnızca x/y/w/h'yi config'e aktarır, başka değişiklikleri ezmez.
- Gözcü iş parçacığı: her 1 sn otomatik dondurma, her 3 sn gömme sağlamlığı + monitör düzeni (değiştiyse host'ları yeniden kurar).
- Dondurma iki tür:
  - **Elle "Duraklat":** iframe'leri kaldırır + host'u gizler (en büyük tasarruf, durum kaybolur).
  - **Otomatik:** WebView2 `SetIsVisible(false)` + `TrySuspend` (durum korunur). Tetikleyiciler: öndeki pencere monitörü kaplıyor (büyütülmüş/tam ekran, kendi pencerelerimiz hariç), oturum kilitli, isteğe bağlı pil.
- `masaustu-widget.exe --pause/--resume`: single-instance eklentisi çalışan örneğe iletir (test ve kısayol için).

**Widget köprüsü:** `widget-doc.ts` her widget'a küçük bir `RUNTIME` ekler → `parent.postMessage({__widget:1,...})` → `bridge.ts` yalnızca sayfadaki iframe'lerden gelen mesajı kabul eder → `widget_fetch` / `system_info`. `widget_fetch` yalnızca http/https, 5 MB, 15 sn.

**Bağımlılıklar ve gerekçeleri:** `windows` 0.62 (Tauri'ninkiyle aynı sürüm, HWND tipleri uyuşsun), `webview2-com` 0.39.1 (Tauri/wry'ninkiyle aynı; controller erişimi), `tauri-plugin-{autostart,single-instance,dialog,opener}`, `ureq` 3 (`native-tls`, rustls/ring gömülmesin). Tauri ya da wry güncellenirse `windows` ve `webview2-com` sürümlerini `Cargo.lock`'taki karşılıklarıyla eşitle.

## Tuzaklar (hepsi bu projede yaşandı)

**Windows / Win32**
- Build 26200 (24H2+) yapısı: `SHELLDLL_DefView` (ikonlar) ve `WorkerW` (duvar kağıdı) **Progman'ın çocukları**. Host `SetParent(progman)` + `SetWindowPos(insertAfter = DefView)`. Klasik yapı (Win10) kodda var ama **denenmedi**.
- Gömülü pencerede `GetParent` ebeveyni değil sahibi döndürür → `GetAncestor(GA_PARENT)` kullan.
- `ShowWindow` host'u kardeşlerin en üstüne, **ikonların önüne** taşır (ikonlar kaybolur, tıklamaları host yer) → `keep_below_icons`; gözcü de her turda düzeltir.
- Host'u gizlemek ya da üstünün kapalı olması WebView2'nin CPU'sunu **düşürmez** (ölçüldü). Dondurmak için controller `SetIsVisible(false)` + `TrySuspend` şart.
- Explorer yeniden başlayınca host'lar yok olur; uygulama kapanmasın diye `RunEvent::ExitRequested { code: None }` engelleniyor, gözcü yeniden kuruyor. Yalnızca `app.exit()` çıkar.

**Tauri**
- Pencere açan komutlar **`async fn`** olmalı. Senkron komut içinde pencere oluşturmak Windows'ta ana iş parçacığını kilitler (pencere 800×600 gizli kalır).
- `with_webview` ana iş parçacığına gider; çağırmadan önce `hosts` kilidini bırak (aynı kilidi `host_widgets` ister).
- `document.title` yerel pencere başlığına yansımaz → `getCurrentWindow().setTitle()` + `core:window:allow-set-title`.

**Widget belgesi**
- `html` tam şeffafsa `body` arka planı tuvale yayılır, `border-radius` kaybolur → `html{background:rgba(0,0,0,.004)}` (1/255; daha küçüğü 0'a yuvarlanır).
- Kullanıcı JS'indeki `</script>` kaçırılmalı (`<\/script`).
- Sandbox nedeniyle widget'ta `localStorage` yok.

**Araçlar / test**
- Windows PowerShell 5.1 BOM'suz UTF-8 betikteki Türkçe karakterleri bozar. `scripts/dev/*.ps1` BOM'lu; yeni betik yazarsan BOM ekle ya da Türkçe metni parametreyle ver.
- P/Invoke'ta string parametreye `$null` boş dizeye dönüşür → `[NullString]::Value`.
- bash içinde `node -e "..."` yazarken `$(` komut ikamesi olur, `\\` teke iner; karmaşık düzenlemede Edit aracını ya da betik dosyasını kullan.
- Win11 Not Defteri başlatıcı süreç kullanır; pencereyi `FindWindow("Notepad")` ile bul. Kullanıcının açık Not Defteri'ne dokunmamaya dikkat.
- Windows dosya diyaloğu UIA ile doldurulamadı → içe/dışa aktarma Rust birim testleriyle doğrulanıyor. Diyalog takılırsa `WM_CLOSE` ile kapat.
- `aria-haspopup` butonu UIA'da Invoke değil ExpandCollapse sunar (`uia.ps1` `PressEl` ikisini de dener).

## Doğrulama

Masaüstünde gerçekten çalıştığını kanıtlamadan özellik bitmiş sayılmaz. Betikler (PowerShell):

| Betik | Ne | Örnek |
|---|---|---|
| `zorder.ps1` | Gömme sırası ve tıklamanın kime gittiği. Beklenen: `SHELLDLL_DefView > Tauri Window > WorkerW`, isabet DefView | `.\scripts\dev\zorder.ps1 -X 300 -Y 200` |
| `shot.ps1` | Masaüstünü gösterip görüntü alır, pencereleri geri getirir. `-HideIcons` simge etiketlerini gizler (kişisel ad içerebilir) | `.\scripts\dev\shot.ps1 -Name test -Half` |
| `winshot.ps1` | Tek pencerenin görüntüsü | `.\scripts\dev\winshot.ps1 -Title "Masaüstü Widget" -Name yonetim` |
| `olc.ps1` | CPU + bellek (Görev Yöneticisi ölçütü), `-Detail` süreç dökümü | `.\scripts\dev\olc.ps1 -Sec 15 -Detail` |
| `uia.ps1` | Yönetim penceresini sürmek: `Win`, `ClickBtn`, `SetVal`, `Drag`, `Cond` | `. .\scripts\dev\uia.ps1; $w = Win "Masaüstü Widget"; ClickBtn $w "Kaydet*"` |

Görüntüleri Read ile açıp bak; gerekirse piksel rengi örnekle. Duvar kağıdı düz renkse şeffaflık "boş alan = duvar kağıdı rengi" diye ölçülebilir.

Hızlı uçtan uca kontrol listesi:
- Gömme: `zorder.ps1` sırası doğru, `shot.ps1` görüntüsünde widget ikonların arkasında.
- Kaydetme: yönetimden değiştir → `widgets.json` ve masaüstü güncel.
- Duraklat: `masaustu-widget.exe --pause` → host gizli, `--resume` sonrası sıra hâlâ doğru.
- Otomatik dondurma: büyütülmüş bir pencere öndeyken `olc.ps1` CPU ~0, dev günlüğünde "donduruldu (üstü kapalı)".
- Explorer yeniden başlatma: `Stop-Process -Name explorer -Force` (kullanıcıyı uyar) → 3 sn içinde host yeni Progman'a gömülü.
- Düzenleme modu: `Drag` ile sürükle, Enter → dosya güncel; Esc → değişmemiş.

**Ölçüm referansları (v0.1.0, release):** exe 4,9 MB, NSIS 1,7 MB, MSI 2,5 MB. Tek saat widget'ı CPU %0,04, bellek ~70–105 MB (çoğu WebView2). 7 widget (GIF + canlı grafik dahil) ~%1,4 CPU. Tüm widget iframe'leri tek renderer paylaşır. Üstü kapalı monitörde CPU ~%0. Bu değerleri bozan değişiklik yaparsan ölç ve söyle.

## Yayın

1. Sürümü **üç yerde** eşit artır: `package.json`, `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`.
2. Commit + push, ardından `git tag -a vX.Y.Z -m "..." && git push origin vX.Y.Z`.
3. `.github/workflows/release.yml` birim testlerini çalıştırır, `tauri-action` ile NSIS + MSI'ı **taslak** Release'e yükler (~10 dk). İzle: `gh run watch <id>`.
4. Dosyaları kontrol et (`gh release download vX.Y.Z`), sonra `gh release edit vX.Y.Z --draft=false --latest`.
5. GitHub dosya adlarında boşluğu noktaya, `ü`'yü `u`'ya çevirir: `Masaustu.Widget_X.Y.Z_x64-setup.exe`.

Kurulu sürümle ilgili: NSIS kullanıcı klasörüne kurar (yönetici izni yok), masaüstüne kısayol koyar. İlk açılışta (config yoksa) başlangıca kaydolur (`HKCU\...\Run`). Dev sürümü başlangıca kaydedilmez. Uygulama kimliği `com.taylan.masaustuwidget`; değiştirirsen kullanıcıların config klasörü değişir, widget'ları kaybolur.

## Açık işler / fikirler

- **Çoklu monitör ve karışık DPI gerçekte denenmedi.** İkinci monitör bağlanınca ilk iş: konum, DPI, düzenleme katmanı, monitör değişiminde yeniden kurulum.
- Windows 10 (klasik WorkerW yolu) denenmedi.
- Kilit ekranı ve pil tetikleyicileri gerçek ortamda denenmedi.
- Ekran yalnızca kapanınca (kilitlenmeden) dondurma yok.
- Widget'ı sürükleyerek başka monitöre taşıma yok.
- Kod imzası yok (SmartScreen uyarısı).
- Otomatik güncelleme (`tauri-plugin-updater`) yok.
- İsteğe bağlı ~15 MB tasarruf: sandbox iframe'leri ayrı süreçte; `IsolateSandboxedIframes` kapatılırsa bir renderer azalır ama widget kodu Tauri erişimli host'la aynı sürece girer. Bilinçli olarak yapılmadı.
