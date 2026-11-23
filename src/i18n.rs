//! Interface strings. Every user-visible label lives here so translations stay complete.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Lang {
    #[default]
    En,
    Tr,
}

impl Lang {
    pub const ALL: [Lang; 2] = [Lang::En, Lang::Tr];

    /// Picks Turkish when the OS locale is Turkish, English otherwise.
    pub fn detect() -> Self {
        let locale = sys_locale::get_locale().unwrap_or_default().to_lowercase();
        if locale.starts_with("tr") {
            Self::Tr
        } else {
            Self::En
        }
    }

    pub fn native_name(self) -> &'static str {
        match self {
            Lang::En => "English",
            Lang::Tr => "Türkçe",
        }
    }

    /// Uppercases with the language's casing rules (Turkish `i` → `İ`).
    pub fn upper(self, text: &str) -> String {
        match self {
            Lang::Tr => text.replace('i', "İ").to_uppercase(),
            Lang::En => text.to_uppercase(),
        }
    }

    pub fn strings(self) -> &'static Strings {
        match self {
            Lang::En => &EN,
            Lang::Tr => &TR,
        }
    }
}

pub struct Strings {
    // Toolbar
    pub add_torrent: &'static str,
    pub resume: &'static str,
    pub pause: &'static str,
    pub remove: &'static str,
    pub search: &'static str,
    pub settings: &'static str,

    // Filters
    pub all: &'static str,
    pub downloading: &'static str,
    pub seeding: &'static str,
    pub completed: &'static str,
    pub paused: &'static str,
    pub errors: &'static str,

    // Statuses
    pub checking: &'static str,
    pub error: &'static str,
    pub fetching_metadata: &'static str,

    // Columns
    pub name: &'static str,
    pub size: &'static str,
    pub down: &'static str,
    pub up: &'static str,
    pub eta: &'static str,
    pub peers: &'static str,

    // Empty states
    pub empty_title: &'static str,
    pub empty_hint: &'static str,
    pub no_match: &'static str,
    pub drop_to_add: &'static str,

    // Details
    pub general: &'static str,
    pub files: &'static str,
    pub save_path: &'static str,
    pub info_hash: &'static str,
    pub downloaded: &'static str,
    pub uploaded: &'static str,
    pub ratio: &'static str,
    pub total_size: &'static str,
    pub status: &'static str,
    pub no_peers: &'static str,
    pub client: &'static str,
    pub address: &'static str,

    // Context menu
    pub open_folder: &'static str,
    pub copy_magnet: &'static str,
    pub remove_with_files: &'static str,

    // Remove confirmation
    pub remove_title: &'static str,
    pub also_delete_files: &'static str,
    pub cancel: &'static str,

    // Add dialog
    pub magnet_placeholder: &'static str,
    pub choose_file: &'static str,
    pub save_to: &'static str,
    pub change: &'static str,
    pub start_now: &'static str,
    pub add: &'static str,
    pub resolving: &'static str,
    pub resolving_hint: &'static str,
    pub select_all: &'static str,
    pub select_none: &'static str,
    pub invalid_source: &'static str,

    // Notifications
    pub added: &'static str,
    pub finished: &'static str,
    pub already_added: &'static str,
    pub failed: &'static str,
    pub copied: &'static str,

    // Settings
    pub downloads: &'static str,
    pub default_folder: &'static str,
    pub speed_limits: &'static str,
    pub download_limit: &'static str,
    pub upload_limit: &'static str,
    pub unlimited: &'static str,
    pub network: &'static str,
    pub port: &'static str,
    pub upnp: &'static str,
    pub restart_note: &'static str,
    pub interface: &'static str,
    pub language: &'static str,
    pub save: &'static str,
    pub magnet_handler: &'static str,
    pub magnet_handler_hint: &'static str,
    pub make_default: &'static str,
    pub is_default: &'static str,

    pub eta_units: crate::format::EtaUnits,

    pub torrents_count: fn(usize) -> String,
    pub remove_count: fn(usize) -> String,
    pub selection_summary: fn(usize, usize, &str) -> String,
    pub peers_summary: fn(u32, u32) -> String,
}

pub static EN: Strings = Strings {
    add_torrent: "Add torrent",
    resume: "Resume",
    pause: "Pause",
    remove: "Remove",
    search: "Search",
    settings: "Settings",

    all: "All",
    downloading: "Downloading",
    seeding: "Seeding",
    completed: "Completed",
    paused: "Paused",
    errors: "Errors",

    checking: "Checking",
    error: "Error",
    fetching_metadata: "Fetching metadata",

    name: "Name",
    size: "Size",
    down: "Down",
    up: "Up",
    eta: "ETA",
    peers: "Peers",

    empty_title: "No torrents yet",
    empty_hint: "Paste a magnet link with Ctrl+V or drop a .torrent file anywhere in this window.",
    no_match: "Nothing here",
    drop_to_add: "Drop to add",

    general: "General",
    files: "Files",
    save_path: "Saved in",
    info_hash: "Info hash",
    downloaded: "Downloaded",
    uploaded: "Uploaded",
    ratio: "Ratio",
    total_size: "Size",
    status: "Status",
    no_peers: "Not connected to any peers",
    client: "Client",
    address: "Address",

    open_folder: "Open folder",
    copy_magnet: "Copy magnet link",
    remove_with_files: "Remove with files",

    remove_title: "Remove torrent",
    also_delete_files: "Also delete downloaded files",
    cancel: "Cancel",

    magnet_placeholder: "Magnet link or .torrent URL",
    choose_file: "Open .torrent file",
    save_to: "Save to",
    change: "Change",
    start_now: "Start downloading right away",
    add: "Add",
    resolving: "Fetching file list",
    resolving_hint: "You can add it now. Files appear once peers respond.",
    select_all: "All",
    select_none: "None",
    invalid_source: "That doesn't look like a magnet link or torrent URL",

    added: "Added",
    finished: "Finished",
    already_added: "Already in your list",
    failed: "Couldn't add",
    copied: "Magnet link copied",

    downloads: "Downloads",
    default_folder: "Default folder",
    speed_limits: "Speed limits",
    download_limit: "Download",
    upload_limit: "Upload",
    unlimited: "Unlimited",
    network: "Network",
    port: "Listening port",
    upnp: "Forward the port automatically (UPnP)",
    restart_note: "Network changes apply after a restart.",
    interface: "Interface",
    language: "Language",
    save: "Save",
    magnet_handler: "Magnet links",
    magnet_handler_hint: "Open magnet links and .torrent files from your browser with Riptide.",
    make_default: "Make default",
    is_default: "Riptide opens magnet links and .torrent files",

    eta_units: ["s", "m", "h", "d"],

    torrents_count: |n| match n {
        1 => "1 torrent".to_owned(),
        n => format!("{n} torrents"),
    },
    remove_count: |n| match n {
        1 => "Remove this torrent from the list?".to_owned(),
        n => format!("Remove {n} torrents from the list?"),
    },
    selection_summary: |sel, total, size| format!("{sel} of {total} files · {size}"),
    peers_summary: |peers, seeds| format!("{peers} peers · {seeds} seeds"),
};

pub static TR: Strings = Strings {
    add_torrent: "Torrent ekle",
    resume: "Devam et",
    pause: "Duraklat",
    remove: "Kaldır",
    search: "Ara",
    settings: "Ayarlar",

    all: "Tümü",
    downloading: "İndiriliyor",
    seeding: "Paylaşılıyor",
    completed: "Tamamlandı",
    paused: "Duraklatıldı",
    errors: "Hatalı",

    checking: "Kontrol ediliyor",
    error: "Hata",
    fetching_metadata: "Bilgiler alınıyor",

    name: "Ad",
    size: "Boyut",
    down: "İndirme",
    up: "Yükleme",
    eta: "Kalan",
    peers: "Eşler",

    empty_title: "Henüz torrent yok",
    empty_hint: "Ctrl+V ile bir magnet bağlantısı yapıştırın ya da bir .torrent dosyasını pencereye sürükleyin.",
    no_match: "Burada bir şey yok",
    drop_to_add: "Eklemek için bırakın",

    general: "Genel",
    files: "Dosyalar",
    save_path: "Konum",
    info_hash: "Info hash",
    downloaded: "İndirilen",
    uploaded: "Yüklenen",
    ratio: "Oran",
    total_size: "Boyut",
    status: "Durum",
    no_peers: "Hiçbir eşe bağlı değil",
    client: "İstemci",
    address: "Adres",

    open_folder: "Klasörü aç",
    copy_magnet: "Magnet bağlantısını kopyala",
    remove_with_files: "Dosyalarla birlikte kaldır",

    remove_title: "Torrent kaldır",
    also_delete_files: "İndirilen dosyaları da sil",
    cancel: "Vazgeç",

    magnet_placeholder: "Magnet bağlantısı veya .torrent adresi",
    choose_file: ".torrent dosyası aç",
    save_to: "Kayıt yeri",
    change: "Değiştir",
    start_now: "Hemen indirmeye başla",
    add: "Ekle",
    resolving: "Dosya listesi alınıyor",
    resolving_hint: "Şimdi ekleyebilirsiniz. Eşler yanıt verince dosyalar listelenir.",
    select_all: "Tümü",
    select_none: "Hiçbiri",
    invalid_source: "Bu bir magnet bağlantısı ya da torrent adresi gibi görünmüyor",

    added: "Eklendi",
    finished: "Tamamlandı",
    already_added: "Zaten listenizde",
    failed: "Eklenemedi",
    copied: "Magnet bağlantısı kopyalandı",

    downloads: "İndirmeler",
    default_folder: "Varsayılan klasör",
    speed_limits: "Hız sınırları",
    download_limit: "İndirme",
    upload_limit: "Yükleme",
    unlimited: "Sınırsız",
    network: "Ağ",
    port: "Dinlenen port",
    upnp: "Portu otomatik yönlendir (UPnP)",
    restart_note: "Ağ değişiklikleri yeniden başlatınca uygulanır.",
    interface: "Arayüz",
    language: "Dil",
    save: "Kaydet",
    magnet_handler: "Magnet bağlantıları",
    magnet_handler_hint: "Tarayıcıdaki magnet bağlantılarını ve .torrent dosyalarını Riptide ile açın.",
    make_default: "Varsayılan yap",
    is_default: "Magnet bağlantıları ve .torrent dosyaları Riptide ile açılıyor",

    eta_units: [" sn", " dk", " sa", " g"],

    torrents_count: |n| format!("{n} torrent"),
    remove_count: |n| match n {
        1 => "Bu torrent listeden kaldırılsın mı?".to_owned(),
        n => format!("{n} torrent listeden kaldırılsın mı?"),
    },
    selection_summary: |sel, total, size| format!("{total} dosyadan {sel} seçili · {size}"),
    peers_summary: |peers, seeds| format!("{peers} eş · {seeds} kaynak"),
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uppercases_turkish_dotted_i() {
        assert_eq!(Lang::Tr.upper("İndirme"), "İNDİRME");
        assert_eq!(Lang::Tr.upper("Eşler"), "EŞLER");
        assert_eq!(Lang::En.upper("Peers"), "PEERS");
    }
}
