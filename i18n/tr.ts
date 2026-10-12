const tr = {
  app: { name: "Rodi Checker", tagline: "Hepsi bir arada kontrol aracı" },
  nav: {
    dashboard: "Panel",
    web: "Web / API İzleme",
    proxy: "Proxy Checker",
    files: "Dosya Bütünlüğü",
    system: "Sistem Sağlığı",
    reports: "Raporlar",
    settings: "Ayarlar",
  },
  status: { up: "Çalışıyor", degraded: "Sorunlu", down: "Erişilemiyor", error: "Hata" },
  common: {
    save: "Kaydet",
    saved: "Kaydedildi",
    saving: "Kaydediliyor…",
    reset: "Varsayılana dön",
    loading: "Yükleniyor…",
    comingSoon: "Yakında",
    phase: "Aşama",
    plannedFeatures: "Planlanan özellikler",
    monitors: "izleyici",
    open: "Aç",
  },
  dashboard: {
    title: "Panel",
    subtitle: "Tüm kontrol modüllerinin özeti",
    version: "Sürüm",
    database: "Veritabanı",
    recentEvents: "Son olaylar",
    noEvents: "Henüz olay yok. Bir modülde kontrol ekleyerek başlayın.",
  },
  modules: {
    web: {
      description: "Web siteleri ve API'lerin erişilebilirliğini, yanıt süresini ve SSL/DNS durumunu izler.",
      features: [
        "HTTP durum kodu, yanıt süresi ve anahtar kelime kontrolü",
        "SSL sertifika bitiş tarihi ve zincir doğrulaması",
        "DNS kayıtları (A, AAAA, MX, TXT, CNAME)",
        "TCP port erişilebilirliği",
        "Uptime yüzdesi, grafikler ve olay geçmişi",
      ],
    },
    proxy: {
      description: "Kendi proxy listelerinizi toplu olarak test eder.",
      features: [
        "HTTP / HTTPS / SOCKS4 / SOCKS5 desteği",
        "Gecikme, çıkış IP'si ve ülke bilgisi",
        "Anonimlik seviyesi: Transparent / Anonymous / Elite",
        "Yapıştırma, .txt ve .csv içe aktarma",
        "Filtreleme ve çalışanları dışa aktarma",
      ],
    },
    files: {
      description: "Dosya ve klasörlerin hash'lerini hesaplar, değişiklikleri tespit eder.",
      features: [
        "SHA-256 ve BLAKE3 ile paralel hash hesaplama",
        "Baseline kaydetme ve karşılaştırma (eklenen / silinen / değişen)",
        "Tek dosya hash doğrulama",
        "Canlı klasör izleme ve bildirim",
      ],
    },
    system: {
      description: "Bilgisayarınızın donanım ve servis durumunu izler.",
      features: [
        "Canlı CPU, RAM, disk ve ağ kullanımı",
        "Süreç listesi ve dinleyen portlar",
        "Windows servis durumu",
        "Eşik aşımı bildirimleri ve geçmiş grafikleri",
      ],
    },
    reports: {
      description: "Kontrol sonuçlarını dışa aktarır.",
      features: ["CSV, JSON ve HTML rapor", "Tarih aralığı ve modül filtresi"],
    },
  },
  settings: {
    title: "Ayarlar",
    subtitle: "Uygulama tercihleri",
    appearance: "Görünüm",
    language: "Dil",
    theme: "Tema",
    themes: { system: "Sistem", light: "Açık", dark: "Koyu" },
    checks: "Kontroller",
    timeout: "Varsayılan zaman aşımı (sn)",
    concurrency: "Varsayılan eşzamanlılık",
    retention: "Geçmişi sakla (gün)",
    proxyJudge: "Proxy anonimlik testi adresi (judge URL)",
    proxyJudgeHint: "Gelen istek başlıklarını geri döndüren bir adres. Boş bırakılırsa anonimlik testi yapılmaz.",
    behavior: "Davranış",
    notifications: "Masaüstü bildirimleri",
    minimizeToTray: "Kapatınca sistem tepsisine küçült",
    startMinimized: "Küçültülmüş başlat",
    autostart: "Windows ile birlikte başlat",
  },
};

export default tr;

type Widen<T> = T extends string
  ? string
  : T extends readonly (infer U)[]
    ? Widen<U>[]
    : { [K in keyof T]: Widen<T[K]> };

export type Dictionary = Widen<typeof tr>;
