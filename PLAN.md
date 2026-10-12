# Rodi Checker — Geliştirme Planı

Rust + **Tauri 2** tabanlı, tek masaüstü uygulamasında dört kontrol modülünü birleştiren "her şey dahil" checker.

| Modül | Ne kontrol eder |
|---|---|
| 🌐 **Web / API İzleme** | URL erişilebilirliği, HTTP durum kodu, yanıt süresi, içerik/anahtar kelime, SSL sertifika bitişi, DNS kayıtları, TCP port |
| 🧦 **Proxy Checker** | Kullanıcının kendi HTTP/HTTPS/SOCKS4/SOCKS5 proxy listesi: çalışıyor mu, gecikme, anonimlik seviyesi, çıkış IP'si / ülke |
| 📁 **Dosya Bütünlüğü** | Klasör/dosya hash'leri (SHA-256, BLAKE3), referans "baseline" ile karşılaştırma, canlı değişiklik izleme |
| 🖥️ **Sistem Sağlığı** | CPU, RAM, disk, ağ, süreçler, servisler ve yerel portlar; eşik aşımı uyarıları |

Ortak özellikler: zamanlayıcı, bildirimler, geçmiş + grafikler, raporlama/dışa aktarma, sistem tepsisi, açık/koyu tema, TR/EN dil desteği, otomatik güncelleme.

---

## 1. Teknoloji Yığını

### Backend (Rust)
| Amaç | Crate |
|---|---|
| Uygulama çatısı | `tauri` 2.x |
| Async çalışma zamanı | `tokio` |
| HTTP istemcisi / proxy | `reqwest` (rustls, `socks` özelliği) |
| SSL sertifika analizi | `tokio-rustls`, `x509-parser` |
| DNS | `hickory-resolver` |
| Veritabanı | `sqlx` (SQLite, migration'lar) |
| Hash | `sha2`, `blake3`, `walkdir`, `rayon` |
| Dosya izleme | `notify` |
| Sistem metrikleri | `sysinfo` |
| Zamanlama | `tokio-cron-scheduler` |
| GeoIP (çevrimdışı) | `maxminddb` (opsiyonel, kullanıcı DB dosyası sağlar) |
| Hata / log | `thiserror`, `anyhow`, `tracing` |
| Serileştirme | `serde`, `serde_json`, `csv` |
| TS tip üretimi | `specta` + `tauri-specta` |

### Tauri eklentileri
`notification`, `dialog`, `fs`, `store`, `autostart`, `updater`, `single-instance`, `log`, `window-state`, `clipboard-manager` + yerleşik sistem tepsisi.

### Frontend
- **React 18 + TypeScript + Vite**
- **Tailwind CSS + shadcn/ui** (bileşenler), **lucide-react** (ikonlar)
- **TanStack Query** (backend çağrıları/önbellek), **Zustand** (UI durumu)
- **TanStack Table** (büyük listeler, sanal kaydırma), **Recharts** (grafikler)
- **react-router**, **react-hook-form + zod**, **i18next** (TR/EN)

---

## 2. Mimari

```
┌─────────────────────── Frontend (WebView) ───────────────────────┐
│  Dashboard │ Web İzleme │ Proxy │ Dosya │ Sistem │ Raporlar │ Ayarlar │
│        TanStack Query  ◄── tauri-specta tipli komutlar            │
└──────────────▲──────────────────────────────┬────────────────────┘
               │ events (ilerleme, sonuç)      │ invoke (komutlar)
┌──────────────┴──────────────────────────────▼────────────────────┐
│ src-tauri  (komut katmanı, tepsi, bildirim, pencere)              │
├───────────────────────────────────────────────────────────────────┤
│ checker-core (saf Rust kütüphanesi, Tauri'den bağımsız)           │
│   trait Checker ─┬─ web::HttpCheck / SslCheck / DnsCheck / TcpCheck│
│                  ├─ proxy::ProxyCheck                              │
│                  ├─ files::IntegrityCheck / Watcher                │
│                  └─ system::HealthCheck                            │
│   Scheduler · JobRunner (eşzamanlılık limiti, iptal) · Alerts     │
│   Storage (SQLite/sqlx) · Export (CSV/JSON/HTML)                  │
└───────────────────────────────────────────────────────────────────┘
```

**Temel soyutlama:**
```rust
#[async_trait]
pub trait Checker: Send + Sync {
    type Target: Serialize + DeserializeOwned;
    fn kind(&self) -> CheckKind;
    async fn check(&self, target: &Self::Target, ctx: &CheckContext) -> CheckResult;
}

pub struct CheckResult {
    pub status: Status,            // Up | Degraded | Down | Error
    pub latency_ms: Option<u64>,
    pub details: serde_json::Value,
    pub checked_at: DateTime<Utc>,
}
```
- `JobRunner`: `tokio::Semaphore` ile eşzamanlılık sınırı, `CancellationToken` ile iptal, ilerleme `tauri::Emitter` ile frontend'e akar.
- `checker-core` ayrı crate olduğu için birim testleri GUI olmadan çalışır ve ileride bir **CLI** (`rodi-checker-cli`) eklemek kolaydır.

---

## 3. Klasör Yapısı

```
rodi-checker/
├─ Cargo.toml                 # workspace
├─ crates/
│  └─ checker-core/
│     ├─ src/{lib,checker,scheduler,runner,alerts,export}.rs
│     ├─ src/web/{http,ssl,dns,tcp}.rs
│     ├─ src/proxy/{mod,anonymity,parser}.rs
│     ├─ src/files/{hash,baseline,watcher}.rs
│     ├─ src/system/{metrics,services,ports}.rs
│     ├─ src/storage/{mod,models}.rs
│     └─ migrations/*.sql
├─ src-tauri/
│  ├─ src/{main,lib,tray,state}.rs
│  ├─ src/commands/{web,proxy,files,system,reports,settings}.rs
│  ├─ capabilities/default.json
│  ├─ icons/
│  └─ tauri.conf.json
├─ src/                       # React frontend
│  ├─ pages/  components/  hooks/  stores/  lib/bindings.ts  i18n/
├─ tests/e2e/                 # WebDriver (tauri-driver) testleri
├─ .github/workflows/{ci,release}.yml
└─ PLAN.md
```

---

## 4. Veritabanı Şeması (SQLite)

| Tablo | Alanlar (özet) |
|---|---|
| `monitors` | id, kind (http/ssl/dns/tcp/system), name, config_json, interval_sec, enabled, tags |
| `check_results` | id, monitor_id, status, latency_ms, details_json, checked_at *(indeks: monitor_id, checked_at)* |
| `incidents` | id, monitor_id, started_at, resolved_at, cause |
| `proxy_lists` / `proxies` | liste; host, port, protocol, auth, son durum, latency, anonymity, exit_ip, country |
| `file_baselines` / `file_entries` | baseline adı, kök klasör, algoritma; path, size, mtime, hash |
| `system_samples` | ts, cpu, mem, disk_json, net_json *(yaşa göre otomatik budama)* |
| `alert_rules` | hedef, koşul (ör. latency > 2000 ms, SSL < 14 gün, CPU > %90), kanal |
| `settings` | key, value_json |

Veri saklama süresi ayarlanabilir (ör. 30 gün) ve arka planda budama işi çalışır.

---

## 5. Modül Detayları

### 5.1 Web / API İzleme
- Yöntem (GET/POST/HEAD…), header, body, timeout, yönlendirme takibi
- Beklenen durum kodu, **anahtar kelime var/yok**, JSON yol doğrulaması
- SSL: geçerlilik, kalan gün, issuer, SAN listesi, zincir hatası
- DNS: A/AAAA/MX/TXT/CNAME sorgusu ve beklenen değer karşılaştırması
- TCP port açık mı + bağlantı süresi
- Uptime yüzdesi (24s/7g/30g), yanıt süresi grafiği, olay (incident) zaman çizelgesi

### 5.2 Proxy Checker
- İçe aktarma: yapıştır, `.txt`, `.csv` (`ip:port`, `ip:port:user:pass`, `proto://user:pass@ip:port`)
- Toplu test: ayarlanabilir eşzamanlılık (varsayılan 50), timeout, yeniden deneme
- Ölçümler: çalışıyor mu, gecikme, protokol, çıkış IP'si
- **Anonimlik**: kullanıcının yapılandırdığı "judge" URL'sine (kendi sunucusu veya httpbin benzeri) giden header'lar incelenerek Transparent / Anonymous / Elite
- Ülke: çevrimdışı MaxMind GeoLite2 dosyası (opsiyonel)
- Filtrele/sırala, çalışanları dışa aktar, tekrar test et

### 5.3 Dosya Bütünlüğü
- Klasör seç → `rayon` ile paralel hash (SHA-256 / BLAKE3), hariç tutma desenleri (glob)
- Baseline kaydet → sonra karşılaştır: **eklenen / silinen / değişen** dosyalar
- Tek dosya hash hesaplama + beklenen hash ile doğrulama (indirme kontrolü)
- `notify` ile canlı izleme modu ve değişiklik bildirimi
- Büyük klasörler için ilerleme çubuğu ve iptal

### 5.4 Sistem Sağlığı
- Canlı CPU (çekirdek başına), RAM/swap, disk doluluk, ağ trafiği, sıcaklık (destekleniyorsa)
- Süreç listesi (CPU/RAM'e göre sıralı)
- Yerel dinleyen portlar ve izlenen servislerin çalışıp çalışmadığı
- Eşik kuralları → bildirim; geçmiş grafikleri

### 5.5 Ortak
- **Dashboard**: tüm modüllerin özet kartları, son olaylar, uyarılar
- **Bildirimler**: masaüstü bildirimi; opsiyonel Webhook (Discord/Slack/Telegram URL'si)
- **Raporlar**: CSV, JSON, kendi içinde HTML rapor dışa aktarımı
- **Ayarlar**: tema, dil, başlangıçta çalıştır, tepsiye küçült, saklama süresi, varsayılan timeout/eşzamanlılık, ayar yedekle/geri yükle
- **Sistem tepsisi**: genel durum ikonu (yeşil/sarı/kırmızı), hızlı menü

---

## 6. Güvenlik
- Tauri **capabilities** ile en az yetki; `fs` sadece kullanıcının seçtiği yollar
- Sıkı **CSP**, uzak içerik yüklenmez; tüm ağ istekleri Rust tarafında
- Proxy/webhook şifreleri işletim sistemi anahtarlığında (`keyring` crate)
- Girdi doğrulama (URL, host, port) hem frontend (zod) hem backend'de
- Güncellemeler imzalı (`tauri-plugin-updater` + imza anahtarı)
- `cargo audit` / `cargo deny` CI'da

---

## 7. Test ve Kalite
| Katman | Araç |
|---|---|
| Rust birim/entegrasyon | `cargo test`, `wiremock` (sahte HTTP sunucu), `tempfile` |
| Frontend birim | `vitest` + Testing Library |
| Uçtan uca | `tauri-driver` + WebdriverIO (Linux/Windows) |
| Lint/format | `clippy -D warnings`, `rustfmt`, `eslint`, `prettier` |

## 8. CI/CD (GitHub Actions)
- `ci.yml`: her PR'da fmt, clippy, test, frontend lint/test, build kontrolü
- `release.yml`: tag ile `tauri-action` → **Windows (.msi/.exe)**, **macOS (.dmg, universal)**, **Linux (.AppImage/.deb/.rpm)**; updater için `latest.json` yayını

---

## 9. Yol Haritası (Aşamalar)

| # | Aşama | Çıktı |
|---|---|---|
| **0** | İskelet | Workspace, Tauri 2 + React + Tailwind kurulumu, tauri-specta, CI, SQLite + migration'lar, layout/navigasyon, tema |
| **1** | Çekirdek | `Checker` trait, JobRunner, Scheduler, storage katmanı, event akışı, ayarlar |
| **2** | Web İzleme | HTTP/SSL/DNS/TCP kontrolleri, monitör CRUD, geçmiş grafikleri, uptime |
| **3** | Proxy Checker | İçe aktarma/parser, toplu test, anonimlik, GeoIP, filtre/dışa aktarma |
| **4** | Dosya Bütünlüğü | Paralel hash, baseline, karşılaştırma, canlı izleme |
| **5** | Sistem Sağlığı | Canlı metrikler, süreç/port/servis, eşik uyarıları |
| **6** | Ortak özellikler | Dashboard, bildirim + webhook, raporlar, tepsi, autostart, i18n |
| **7** | Cilalama & Yayın | E2E testler, performans, ikonlar, imzalı updater, release pipeline, README |

Her aşama ayrı commit/PR olarak ilerler ve sonunda çalışır durumda olur.

---

## 10. Netleştirilmesi Gerekenler (varsayılanlar parantez içinde)
1. Hedef platformlar? *(Windows + Linux + macOS)*
2. Frontend: React mi, Svelte mi? *(React + TS)*
3. Arayüz dili? *(Türkçe varsayılan + İngilizce)*
4. Webhook bildirimleri gerekli mi? *(Evet, Discord/Slack/Telegram)*
5. Proxy anonimlik testi için judge URL'si? *(Kullanıcı ayarlardan girer)*
6. Ek bir CLI sürümü istenir mi? *(Şimdilik hayır, mimari hazır)*
