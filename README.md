# Rodi Checker

Rust + **Tauri 2** ve **Next.js + Tailwind CSS** ile yazılmış, Windows için hepsi bir arada kontrol uygulaması.

- 🌐 **Web / API İzleme**: HTTP, SSL, DNS, TCP port
- 🧦 **Proxy Checker**: HTTP/HTTPS/SOCKS4/SOCKS5, gecikme, anonimlik, ülke
- 📁 **Dosya Bütünlüğü**: SHA-256 / BLAKE3, baseline karşılaştırma, canlı izleme
- 🖥️ **Sistem Sağlığı**: CPU, RAM, disk, ağ, süreçler, portlar, servisler
- ⌨️ **CLI**: `rodi-checker` (aynı çekirdek)

Ayrıntılı plan ve yol haritası: [PLAN.md](PLAN.md).

## Proje yapısı

| Klasör | İçerik |
|---|---|
| `crates/checker-core` | Tauri'den bağımsız çekirdek: `Checker` trait, JobRunner, Scheduler, SQLite depolama, ayarlar |
| `crates/checker-cli` | `rodi-checker` komut satırı aracı |
| `src-tauri` | Masaüstü uygulaması: komutlar, sistem tepsisi, eklentiler |
| `app/`, `components/`, `lib/`, `i18n/` | Next.js arayüzü (statik export), TR/EN sözlükler |

## Geliştirme

Gereksinimler: Rust (stable), Node.js 22, Windows'ta WebView2 (Windows 10/11'de yüklü gelir).

```bash
npm install
npm run tauri dev          # uygulamayı geliştirme modunda açar
npm run tauri build        # .msi ve .exe kurulum dosyalarını üretir
cargo run -p checker-cli -- kinds
```

Kontroller:

```bash
npm run lint && npm run typecheck && npm test && npm run build
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

> Not: `cargo check`/`clippy` için önce `npm run build` çalıştırılmalıdır; Tauri derleme sırasında `out/` klasörünü gömer.

Uygulama verileri `%APPDATA%\com.rodi.checker\rodi-checker.db` dosyasında tutulur.
