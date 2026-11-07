//! Localization (i18n) for Attune Desktop.
//!
//! Provides comprehensive application-wide localized strings across English and
//! Wikang Filipino (Tagalog), covering navigation, playback controls, drawers,
//! library tables, stage headers, and settings preferences.
//! Also exports a clean translation dictionary template JSON so community
//! contributors can add new languages without hardcoding (enforcing Mandate 4).

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Language {
    English,
    Filipino,
}

impl Language {
    pub fn code(&self) -> &'static str {
        match self {
            Language::English => "en",
            Language::Filipino => "fil",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Language::English => "English (US)",
            Language::Filipino => "Wikang Filipino (Tagalog)",
        }
    }

    pub fn from_code(code: &str) -> Self {
        match code.trim().to_lowercase().as_str() {
            "fil" | "tagalog" | "tl" => Language::Filipino,
            _ => Language::English,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Strings {
    // Navigation & General App
    pub app_name: &'static str,
    pub home: &'static str,
    pub for_you: &'static str,
    pub library: &'static str,
    pub albums: &'static str,
    pub artists: &'static str,
    pub favorite_songs: &'static str,
    pub sources: &'static str,
    pub add_folder: &'static str,
    pub rescan: &'static str,
    pub scanning: &'static str,
    pub no_folder_chosen: &'static str,
    pub equalizer: &'static str,
    pub settings: &'static str,
    pub process_manager: &'static str,
    pub about_attune: &'static str,
    pub search_library_placeholder: &'static str,
    pub music_library: &'static str,

    // Player Bar & Drawers
    pub select_track_to_play: &'static str,
    pub attune_player: &'static str,
    pub lyrics: &'static str,
    pub queue: &'static str,
    pub synchronized_lyrics: &'static str,
    pub no_lyrics_available: &'static str,
    pub no_lyrics_guide: &'static str,
    pub auto_scroll: &'static str,
    pub playback_queue: &'static str,
    pub clear_queue: &'static str,
    pub empty_queue: &'static str,
    pub now_playing: &'static str,

    // Home Stage
    pub good_morning: &'static str,
    pub good_afternoon: &'static str,
    pub good_evening: &'static str,
    pub home_subtitle: &'static str,
    pub shuffle_all: &'static str,
    pub recent_tracks: &'static str,
    pub quick_playlists: &'static str,
    pub smart_shuffle_active: &'static str,

    // Library & Table Columns
    pub songs: &'static str,
    pub title: &'static str,
    pub artist: &'static str,
    pub album: &'static str,
    pub duration: &'static str,
    pub audio_profile: &'static str,
    pub play_all: &'static str,
    pub total_listening_time: &'static str,
    pub discography: &'static str,
    pub popular_releases: &'static str,
    pub singles_and_eps: &'static str,
    pub all_songs: &'static str,
    pub verified_artist: &'static str,
    pub songs_in_library: &'static str,
    pub albums_in_library: &'static str,
    pub artists_in_library: &'static str,
    pub releases: &'static str,

    // Settings Navigation & Categories
    pub settings_and_preferences: &'static str,
    pub general: &'static str,
    pub audio: &'static str,
    pub language: &'static str,
    pub api_providers: &'static str,
    pub directory_and_indexes: &'static str,
    pub backup_and_restore: &'static str,
    pub developer_options: &'static str,

    // Settings Items & Song Exclusion
    pub active_language: &'static str,
    pub excluded: &'static str,
    pub available: &'static str,
    pub restore: &'static str,
    pub exclude: &'static str,
    pub clear_search: &'static str,
    pub save: &'static str,
    pub cancel: &'static str,
    pub reset: &'static str,
    pub open_in_window: &'static str,
    pub close: &'static str,
    pub enabled: &'static str,
    pub disabled: &'static str,
    pub auto_scroll_long_text: &'static str,
    pub auto_scroll_long_text_desc: &'static str,

    // Context Menus & Track Actions
    pub play_now: &'static str,
    pub play_next: &'static str,
    pub add_to_queue: &'static str,
    pub add_to_favorites: &'static str,
    pub remove_favorite: &'static str,
    pub view_track_credits: &'static str,
    pub exclude_from_library: &'static str,
    pub copy_album_title: &'static str,
    pub add_songs: &'static str,

    // Favorites & Table Details
    pub favorited_tracks_in_collection: &'static str,
    pub bpm: &'static str,
    pub key: &'static str,
    pub time: &'static str,
    pub actions: &'static str,
    pub no_favorites_yet: &'static str,

    // Albums Stage
    pub more_by_artist: &'static str,
    pub more_albums_in_library: &'static str,
    pub no_albums_discovered: &'static str,
    pub no_other_albums: &'static str,

    // For You Stage
    pub smart_algorithmic_mixes: &'static str,
    pub smart_mix_subtitle: &'static str,
    pub catalog_hidden_mystery: &'static str,
    pub mystery_mix_desc: &'static str,

    // Fullscreen Stage
    pub instrumental_or_lyrics_pending: &'static str,
    pub fullscreen_lyrics_guide: &'static str,

    // Equalizer & Master FX
    pub equalizer_and_master_fx: &'static str,
    pub preamp_headroom: &'static str,
    pub graphic_equalizer: &'static str,
    pub pure_bass_synthesizer: &'static str,
    pub clarity_vocal_air: &'static str,
    pub stereo_soundstage: &'static str,

    // Process Manager & Hardware
    pub process_and_hardware_manager: &'static str,
    pub kernel_dac_status: &'static str,
    pub watchguard_stability: &'static str,
    pub certified_shared_libraries: &'static str,

    // About Stage
    pub architecture_and_technology: &'static str,
    pub open_source_distribution: &'static str,
}

pub fn get_strings(lang: Language) -> Strings {
    match lang {
        Language::English => Strings {
            app_name: "Attune",
            home: "Home",
            for_you: "For You",
            library: "Library",
            albums: "Albums",
            artists: "Artists",
            favorite_songs: "Favorite Songs",
            sources: "SOURCES",
            add_folder: "Add Folder",
            rescan: "Rescan",
            scanning: "Scanning...",
            no_folder_chosen: "No folder chosen",
            equalizer: "Equalizer",
            settings: "Settings",
            process_manager: "Process Manager",
            about_attune: "About Attune",
            search_library_placeholder: "Search library...",
            music_library: "Music Library",

            select_track_to_play: "Select a track to play",
            attune_player: "Attune Player",
            lyrics: "Lyrics",
            queue: "Queue",
            synchronized_lyrics: "Synchronized Lyrics",
            no_lyrics_available: "No Lyrics Available",
            no_lyrics_guide: "Place a companion .lrc file alongside your song to display synchronized singing lines.",
            auto_scroll: "Auto-Scroll",
            playback_queue: "Playback Queue",
            clear_queue: "Clear",
            empty_queue: "Queue is empty. Select songs or start Smart DJ Shuffle.",
            now_playing: "Now Playing",

            good_morning: "Good morning",
            good_afternoon: "Good afternoon",
            good_evening: "Good evening",
            home_subtitle: "Jump back into your personalized library mix and dynamic DJ transitions.",
            shuffle_all: "Shuffle All",
            recent_tracks: "Recent Tracks",
            quick_playlists: "Quick Playlists",
            smart_shuffle_active: "Attune Dynamic DJ Transition Engine is active",

            songs: "Songs",
            title: "Title",
            artist: "Artist",
            album: "Album",
            duration: "Duration",
            audio_profile: "Audio Profile",
            play_all: "Play All",
            total_listening_time: "total listening time",
            discography: "Discography",
            popular_releases: "Popular releases",
            singles_and_eps: "Singles and EPs",
            all_songs: "All Songs",
            verified_artist: "Verified Artist",
            songs_in_library: "songs in your library",
            albums_in_library: "albums in your library",
            artists_in_library: "artists in your library",
            releases: "releases",

            settings_and_preferences: "Settings & Preferences",
            general: "General",
            audio: "Audio",
            language: "Language",
            api_providers: "API Providers",
            directory_and_indexes: "Directory & Indexes",
            backup_and_restore: "Backup & Restore",
            developer_options: "Developer Options",

            active_language: "Active Language",
            excluded: "Excluded",
            available: "Available",
            restore: "Restore",
            exclude: "Exclude",
            clear_search: "Clear Search",
            save: "Save",
            cancel: "Cancel",
            reset: "Reset to Default",
            open_in_window: "Open in Separate Window",
            close: "Close Window",
            enabled: "Enabled",
            disabled: "Disabled",
            auto_scroll_long_text: "Auto-Scroll Long Titles & Names",
            auto_scroll_long_text_desc: "Allow long song titles, album names, and artist tags to smoothly move back and forth so you can view the complete name. When disabled, overflow text is truncated with '...'.",

            // Context Menus & Track Actions
            play_now: "Play Now",
            play_next: "Play Next",
            add_to_queue: "Add to Queue",
            add_to_favorites: "Add to Favorites",
            remove_favorite: "Remove Favorite",
            view_track_credits: "View Track Credits & Metadata",
            exclude_from_library: "Exclude from Library",
            copy_album_title: "Copy Album Title",
            add_songs: "Add Songs",

            // Favorites & Table Details
            favorited_tracks_in_collection: "favorited tracks in your collection",
            bpm: "BPM",
            key: "Key",
            time: "Time",
            actions: "Actions",
            no_favorites_yet: "No favorite songs yet. Tap the heart icon next to any song to add it here.",

            // Albums Stage
            more_by_artist: "More by",
            more_albums_in_library: "More Albums in Your Library",
            no_albums_discovered: "No albums discovered yet. Scan your music library to organize by album.",
            no_other_albums: "No other albums found in your library.",

            // For You Stage
            smart_algorithmic_mixes: "Smart Algorithmic Mixes",
            smart_mix_subtitle: "Custom DJ sets curated specifically for your listening vibes and tempo profiles.",
            catalog_hidden_mystery: "Catalog Hidden — Surprise Mystery Mix",
            mystery_mix_desc: "Track titles and artist identities are hidden until playback starts. Let Attune's dynamic transition engine surprise your ears.",

            // Fullscreen Stage
            instrumental_or_lyrics_pending: "Instrumental or Lyrics Pending",
            fullscreen_lyrics_guide: "Synchronized singing lines will appear here automatically when a companion .lrc file is provided or fetched via API.",

            // Equalizer & Master FX
            equalizer_and_master_fx: "Equalizer & Master FX",
            preamp_headroom: "Preamp Headroom Gain Pad",
            graphic_equalizer: "10-Band Graphic Equalizer",
            pure_bass_synthesizer: "Pure Bass+ Synthesizer",
            clarity_vocal_air: "Attune Clarity & Vocal Air",
            stereo_soundstage: "Stereo Soundstage & Brickwall Limiter",

            // Process Manager & Hardware
            process_and_hardware_manager: "Process & Hardware Resource Manager",
            kernel_dac_status: "ALSA Kernel Stream & Hardware DAC Status",
            watchguard_stability: "Watchguard Real-Time Stability & Hardware Guardrails",
            certified_shared_libraries: "Certified External Shared Libraries (.so / .dll)",

            // About Stage
            architecture_and_technology: "Architecture & Technology",
            open_source_distribution: "Open Source & Distribution",
        },
        Language::Filipino => Strings {
            app_name: "Attune",
            home: "Tahanan",
            for_you: "Para sa Iyo",
            library: "Koleksyon",
            albums: "Mga Album",
            artists: "Mga Artista",
            favorite_songs: "Mga Paboritong Kanta",
            sources: "MGA PINAGMULAN",
            add_folder: "Magdagdag ng Folder",
            rescan: "I-rescan",
            scanning: "Nagsusuri...",
            no_folder_chosen: "Walang napiling folder",
            equalizer: "Equalizer",
            settings: "Mga Setting",
            process_manager: "Tagapamahala ng Proseso",
            about_attune: "Tungkol sa Attune",
            search_library_placeholder: "Maghanap sa koleksyon...",
            music_library: "Koleksyon ng Musika",

            select_track_to_play: "Pumili ng kanta na patutugtugin",
            attune_player: "Attune Player",
            lyrics: "Titik",
            queue: "Pila",
            synchronized_lyrics: "Kasabay na Titik ng Kanta",
            no_lyrics_available: "Walang Makuhang Titik ng Kanta",
            no_lyrics_guide: "Maglagay ng kasamang .lrc file katabi ng iyong kanta upang maipakita ang kasabay na titik.",
            auto_scroll: "Kusang Pag-scroll",
            playback_queue: "Pila ng Pagpapatugtog",
            clear_queue: "Alisin",
            empty_queue: "Walang laman ang pila. Pumili ng mga kanta o simulan ang Smart DJ Shuffle.",
            now_playing: "Kasalukuyang Tumutugtog",

            good_morning: "Magandang umaga",
            good_afternoon: "Magandang hapon",
            good_evening: "Magandang gabi",
            home_subtitle: "Bumalik sa iyong sariling halo ng tugtugin at dynamic DJ transitions.",
            shuffle_all: "I-shuffle Lahat",
            recent_tracks: "Mga Kamakailang Kanta",
            quick_playlists: "Mga Mabilisang Playlist",
            smart_shuffle_active: "Aktibo ang Attune Dynamic DJ Transition Engine",

            songs: "Mga Kanta",
            title: "Pamagat",
            artist: "Artista",
            album: "Album",
            duration: "Haba",
            audio_profile: "Profile ng Tunog",
            play_all: "Patugtugin Lahat",
            total_listening_time: "kabuuang oras ng pakikinig",
            discography: "Diskograpiya",
            popular_releases: "Mga Sikat na Release",
            singles_and_eps: "Mga Single at EP",
            all_songs: "Lahat ng Kanta",
            verified_artist: "Beripikadong Artista",
            songs_in_library: "mga kanta sa iyong koleksyon",
            albums_in_library: "mga album sa iyong koleksyon",
            artists_in_library: "mga artista sa iyong koleksyon",
            releases: "mga release",

            settings_and_preferences: "Mga Setting at Kagustuhan",
            general: "Pangkalahatan",
            audio: "Tunog at Audio",
            language: "Wika",
            api_providers: "Mga Tagapagbigay ng API",
            directory_and_indexes: "Direktoryo at Index",
            backup_and_restore: "I-backup at Ibalik",
            developer_options: "Mga Opsyon ng Developer",

            active_language: "Kasalukuyang Aktibong Wika",
            excluded: "Naka-exclude",
            available: "Magagamit",
            restore: "Ibalik",
            exclude: "I-exclude",
            clear_search: "Alisin ang Paghahanap",
            save: "I-save",
            cancel: "Kanselahin",
            reset: "Ibalik sa Default",
            open_in_window: "Buksan sa Bagong Window",
            close: "Isara ang Window",
            enabled: "Pinagana",
            disabled: "Hindi Pinagana",
            auto_scroll_long_text: "Kusang Pag-scroll ng Mahahabang Titulo",
            auto_scroll_long_text_desc: "Pahintulutan ang mahahabang titulo ng kanta at album na gumalaw nang pabalik-balik upang makita ang buong pangalan. Kung naka-disable, puputulin ito ng '...'.",

            // Context Menus & Track Actions
            play_now: "Patugtugin Ngayon",
            play_next: "Patugtugin Susunod",
            add_to_queue: "Idagdag sa Pila",
            add_to_favorites: "Idagdag sa Paborito",
            remove_favorite: "Alisin sa Paborito",
            view_track_credits: "Tingnan ang Kredito at Metadata ng Kanta",
            exclude_from_library: "I-exclude mula sa Koleksyon",
            copy_album_title: "Kopyahin ang Pamagat ng Album",
            add_songs: "Magdagdag ng Kanta",

            // Favorites & Table Details
            favorited_tracks_in_collection: "mga paboritong kanta sa iyong koleksyon",
            bpm: "BPM",
            key: "Key",
            time: "Oras",
            actions: "Mga Aksyon",
            no_favorites_yet: "Wala pang paboritong kanta. Pindutin ang pusong icon sa tabi ng kanta upang idagdag dito.",

            // Albums Stage
            more_by_artist: "Iba pa mula kay",
            more_albums_in_library: "Iba pang Album sa Iyong Koleksyon",
            no_albums_discovered: "Wala pang natagpuang album. I-scan ang koleksyon upang maisaayos ayon sa album.",
            no_other_albums: "Walang ibang album na natagpuan sa iyong koleksyon.",

            // For You Stage
            smart_algorithmic_mixes: "Mga Matalinong Halo ng Musika",
            smart_mix_subtitle: "Mga custom na DJ set na inayos ayon sa iyong pakikinig at tempo.",
            catalog_hidden_mystery: "Nakatagong Katalogo — Sorpresang Misteryosong Halo",
            mystery_mix_desc: "Nakatago ang mga pamagat at artista hanggang sa magsimula ang tugtugin. Hayaan ang Attune na sorpresahin ang iyong pandinig.",

            // Fullscreen Stage
            instrumental_or_lyrics_pending: "Instrumental o Naghihintay ng Titik",
            fullscreen_lyrics_guide: "Lalabas ang kasabay na titik ng kanta kapag may kasamang .lrc file o kinuha mula sa API.",

            // Equalizer & Master FX
            equalizer_and_master_fx: "Equalizer at Master FX",
            preamp_headroom: "Preamp Headroom Gain Pad",
            graphic_equalizer: "10-Band Graphic Equalizer",
            pure_bass_synthesizer: "Pure Bass+ Synthesizer",
            clarity_vocal_air: "Attune Clarity at Vocal Air",
            stereo_soundstage: "Stereo Soundstage at Brickwall Limiter",

            // Process Manager & Hardware
            process_and_hardware_manager: "Tagapamahala ng Proseso at Hardware",
            kernel_dac_status: "ALSA Kernel Stream at Katayuan ng Hardware DAC",
            watchguard_stability: "Watchguard Real-Time na Katatagan at Guardrails",
            certified_shared_libraries: "Mga Sertipikadong External Shared Library (.so / .dll)",

            // About Stage
            architecture_and_technology: "Arkitektura at Teknolohiya",
            open_source_distribution: "Open Source at Pamamahagi",
        },
    }
}

/// Helper to load the current system/user configured language strings
pub fn current_strings() -> Strings {
    let cfg = crate::config::load_config();
    get_strings(Language::from_code(&cfg.language))
}

/// Generates a structured JSON template of all translation keys and English default strings.
pub fn export_language_template_json() -> String {
    let mut map = BTreeMap::new();
    let s = get_strings(Language::English);
    map.insert("app_name", s.app_name);
    map.insert("home", s.home);
    map.insert("for_you", s.for_you);
    map.insert("library", s.library);
    map.insert("albums", s.albums);
    map.insert("artists", s.artists);
    map.insert("favorite_songs", s.favorite_songs);
    map.insert("sources", s.sources);
    map.insert("add_folder", s.add_folder);
    map.insert("rescan", s.rescan);
    map.insert("scanning", s.scanning);
    map.insert("no_folder_chosen", s.no_folder_chosen);
    map.insert("equalizer", s.equalizer);
    map.insert("settings", s.settings);
    map.insert("process_manager", s.process_manager);
    map.insert("about_attune", s.about_attune);
    map.insert("search_library_placeholder", s.search_library_placeholder);
    map.insert("music_library", s.music_library);

    map.insert("select_track_to_play", s.select_track_to_play);
    map.insert("attune_player", s.attune_player);
    map.insert("lyrics", s.lyrics);
    map.insert("queue", s.queue);
    map.insert("synchronized_lyrics", s.synchronized_lyrics);
    map.insert("no_lyrics_available", s.no_lyrics_available);
    map.insert("no_lyrics_guide", s.no_lyrics_guide);
    map.insert("auto_scroll", s.auto_scroll);
    map.insert("playback_queue", s.playback_queue);
    map.insert("clear_queue", s.clear_queue);
    map.insert("empty_queue", s.empty_queue);
    map.insert("now_playing", s.now_playing);

    map.insert("good_morning", s.good_morning);
    map.insert("good_afternoon", s.good_afternoon);
    map.insert("good_evening", s.good_evening);
    map.insert("home_subtitle", s.home_subtitle);
    map.insert("shuffle_all", s.shuffle_all);
    map.insert("recent_tracks", s.recent_tracks);
    map.insert("quick_playlists", s.quick_playlists);
    map.insert("smart_shuffle_active", s.smart_shuffle_active);

    map.insert("songs", s.songs);
    map.insert("title", s.title);
    map.insert("artist", s.artist);
    map.insert("album", s.album);
    map.insert("duration", s.duration);
    map.insert("audio_profile", s.audio_profile);
    map.insert("play_all", s.play_all);
    map.insert("total_listening_time", s.total_listening_time);
    map.insert("discography", s.discography);
    map.insert("popular_releases", s.popular_releases);
    map.insert("singles_and_eps", s.singles_and_eps);
    map.insert("all_songs", s.all_songs);
    map.insert("verified_artist", s.verified_artist);
    map.insert("songs_in_library", s.songs_in_library);
    map.insert("albums_in_library", s.albums_in_library);
    map.insert("artists_in_library", s.artists_in_library);
    map.insert("releases", s.releases);

    map.insert("settings_and_preferences", s.settings_and_preferences);
    map.insert("general", s.general);
    map.insert("audio", s.audio);
    map.insert("language", s.language);
    map.insert("api_providers", s.api_providers);
    map.insert("directory_and_indexes", s.directory_and_indexes);
    map.insert("backup_and_restore", s.backup_and_restore);
    map.insert("developer_options", s.developer_options);

    map.insert("active_language", s.active_language);
    map.insert("excluded", s.excluded);
    map.insert("available", s.available);
    map.insert("restore", s.restore);
    map.insert("exclude", s.exclude);
    map.insert("clear_search", s.clear_search);
    map.insert("save", s.save);
    map.insert("cancel", s.cancel);
    map.insert("reset", s.reset);
    map.insert("open_in_window", s.open_in_window);
    map.insert("close", s.close);
    map.insert("enabled", s.enabled);
    map.insert("disabled", s.disabled);
    map.insert("auto_scroll_long_text", s.auto_scroll_long_text);
    map.insert("auto_scroll_long_text_desc", s.auto_scroll_long_text_desc);

    map.insert("play_now", s.play_now);
    map.insert("play_next", s.play_next);
    map.insert("add_to_queue", s.add_to_queue);
    map.insert("add_to_favorites", s.add_to_favorites);
    map.insert("remove_favorite", s.remove_favorite);
    map.insert("view_track_credits", s.view_track_credits);
    map.insert("exclude_from_library", s.exclude_from_library);
    map.insert("copy_album_title", s.copy_album_title);
    map.insert("add_songs", s.add_songs);

    map.insert("favorited_tracks_in_collection", s.favorited_tracks_in_collection);
    map.insert("bpm", s.bpm);
    map.insert("key", s.key);
    map.insert("time", s.time);
    map.insert("actions", s.actions);
    map.insert("no_favorites_yet", s.no_favorites_yet);

    map.insert("more_by_artist", s.more_by_artist);
    map.insert("more_albums_in_library", s.more_albums_in_library);
    map.insert("no_albums_discovered", s.no_albums_discovered);
    map.insert("no_other_albums", s.no_other_albums);

    map.insert("smart_algorithmic_mixes", s.smart_algorithmic_mixes);
    map.insert("smart_mix_subtitle", s.smart_mix_subtitle);
    map.insert("catalog_hidden_mystery", s.catalog_hidden_mystery);
    map.insert("mystery_mix_desc", s.mystery_mix_desc);

    map.insert("instrumental_or_lyrics_pending", s.instrumental_or_lyrics_pending);
    map.insert("fullscreen_lyrics_guide", s.fullscreen_lyrics_guide);

    map.insert("equalizer_and_master_fx", s.equalizer_and_master_fx);
    map.insert("preamp_headroom", s.preamp_headroom);
    map.insert("graphic_equalizer", s.graphic_equalizer);
    map.insert("pure_bass_synthesizer", s.pure_bass_synthesizer);
    map.insert("clarity_vocal_air", s.clarity_vocal_air);
    map.insert("stereo_soundstage", s.stereo_soundstage);

    map.insert("process_and_hardware_manager", s.process_and_hardware_manager);
    map.insert("kernel_dac_status", s.kernel_dac_status);
    map.insert("watchguard_stability", s.watchguard_stability);
    map.insert("certified_shared_libraries", s.certified_shared_libraries);

    map.insert("architecture_and_technology", s.architecture_and_technology);
    map.insert("open_source_distribution", s.open_source_distribution);

    serde_json::to_string_pretty(&map).unwrap_or_else(|_| "{}".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_language_from_code() {
        assert_eq!(Language::from_code("en"), Language::English);
        assert_eq!(Language::from_code("fil"), Language::Filipino);
        assert_eq!(Language::from_code("tagalog"), Language::Filipino);
        assert_eq!(Language::from_code("unknown"), Language::English);
    }

    #[test]
    fn test_export_template_json_contains_keys() {
        let json = export_language_template_json();
        assert!(json.contains("\"home\""));
        assert!(json.contains("\"general\""));
        assert!(json.contains("\"lyrics\""));
        assert!(json.contains("\"playback_queue\""));
        assert!(json.contains("\"discography\""));
        assert!(json.contains("\"backup_and_restore\""));
        assert!(json.contains("\"auto_scroll_long_text\""));
    }
}
