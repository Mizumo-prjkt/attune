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
    pub favorites: &'static str,
    pub favorite_songs: &'static str,
    pub favorite_artists: &'static str,
    pub favorite_albums: &'static str,
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
    pub help_guide: &'static str,
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
    pub favorite_artists_in_collection: &'static str,
    pub favorite_albums_in_collection: &'static str,
    pub bpm: &'static str,
    pub key: &'static str,
    pub time: &'static str,
    pub actions: &'static str,
    pub no_favorites_yet: &'static str,
    pub no_favorite_artists_yet: &'static str,
    pub no_favorite_albums_yet: &'static str,

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
    pub reset_bands: &'static str,

    // Process Manager & Hardware
    pub process_and_hardware_manager: &'static str,
    pub kernel_dac_status: &'static str,
    pub watchguard_stability: &'static str,
    pub certified_shared_libraries: &'static str,
    pub cpu_timeline_title: &'static str,
    pub memory_timeline_title: &'static str,

    // About Stage
    pub architecture_and_technology: &'static str,
    pub open_source_distribution: &'static str,
    pub special_thanks_and_credits: &'static str,
    pub custom_lyrics_css: &'static str,
    pub custom_lyrics_css_desc: &'static str,

    // Lyrics Layout & Alignment
    pub sidebar_lyrics_align: &'static str,
    pub sidebar_lyrics_align_desc: &'static str,
    pub fullscreen_layout_placement: &'static str,
    pub fullscreen_layout_placement_desc: &'static str,
    pub fullscreen_lyrics_align: &'static str,
    pub fullscreen_lyrics_align_desc: &'static str,
    pub controls_left_lyrics_right: &'static str,
    pub controls_right_lyrics_left: &'static str,
    pub align_left: &'static str,
    pub align_center: &'static str,
    pub align_right: &'static str,
    pub custom_css_precedence_notice: &'static str,
    pub karaoke_preset_label: &'static str,
    pub css_reference_guide_title: &'static str,
    pub css_scope_note: &'static str,
    pub launch_lyrics_preview: &'static str,
    pub preview_lyrics_window_title: &'static str,
    pub preview_fullscreen_mode: &'static str,
    pub preview_sidebar_mode: &'static str,
    pub preview_simulating_line: &'static str,
    pub preview_speed: &'static str,
    pub preview_track_sample: &'static str,
    pub preview_notice_realtime: &'static str,
    pub preview_format_line_sync: &'static str,
    pub preview_format_word_sync: &'static str,
    pub preview_word_engine_active: &'static str,
    pub preview_syllable_progress: &'static str,
    pub lyrics_font_family: &'static str,
    pub lyrics_font_family_desc: &'static str,
    pub font_group_default: &'static str,
    pub font_group_system: &'static str,
    pub font_preview_sample: &'static str,
    pub lyrics_text_size: &'static str,
    pub sidebar_lyrics_size: &'static str,
    pub fullscreen_lyrics_size: &'static str,
    pub decrease_lyrics_size: &'static str,
    pub increase_lyrics_size: &'static str,
    pub reset_lyrics_size: &'static str,
    pub lyrics_text_size_desc: &'static str,

    // Artist Profile Finder & Matcher Stage
    pub select_artist_to_find_profile: &'static str,
    pub select_artist_to_find_profile_desc: &'static str,
    pub artist_profile_finder: &'static str,
    pub editing_profile_for_artist: &'static str,
    pub find_artist_manually: &'static str,
    pub artist_profile_not_found: &'static str,
    pub artist_profile_not_found_desc: &'static str,
    pub search_artist_placeholder: &'static str,
    pub apply_and_link_artist: &'static str,
    pub linked_profile_success: &'static str,
    pub custom_profile: &'static str,
    pub image_url: &'static str,
    pub banner_url: &'static str,
    pub bio_description: &'static str,
    pub no_artist_candidates_found: &'static str,
    pub no_artist_candidates_guide: &'static str,
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
            favorites: "Favorites",
            favorite_songs: "Favorite Songs",
            favorite_artists: "Favorite Artists",
            favorite_albums: "Favorite Albums",
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
            help_guide: "Help Guide",
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
            favorite_artists_in_collection: "favorite artists in your collection",
            favorite_albums_in_collection: "favorite albums in your collection",
            bpm: "BPM",
            key: "Key",
            time: "Time",
            actions: "Actions",
            no_favorites_yet: "No favorite songs yet. Tap the heart icon next to any song to add it here.",
            no_favorite_artists_yet: "No favorite artists yet. Tap the heart icon on any artist to add them here.",
            no_favorite_albums_yet: "No favorite albums yet. Tap the heart icon on any album to add it here.",

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
            reset_bands: "Reset",

            // Process Manager & Hardware
            process_and_hardware_manager: "Process & Hardware Resource Manager",
            kernel_dac_status: "ALSA Kernel Stream & Hardware DAC Status",
            watchguard_stability: "Watchguard Real-Time Stability & Hardware Guardrails",
            certified_shared_libraries: "Certified External Shared Libraries (.so / .dll)",
            cpu_timeline_title: "Process CPU Utilization Timeline",
            memory_timeline_title: "Process Memory (RSS) Footprint Timeline",

            // About Stage
            architecture_and_technology: "Architecture & Technology",
            open_source_distribution: "Open Source & Distribution",
            special_thanks_and_credits: "Special Thanks & Open Source Credits",
            custom_lyrics_css: "Custom Fullscreen Lyrics CSS Animation",
            custom_lyrics_css_desc: "Inject custom CSS keyframe animations, typography, and styling into the fullscreen lyrics stage (inspired by Better Lyrics).",

            // Lyrics Layout & Alignment
            sidebar_lyrics_align: "Sidebar Lyrics Alignment",
            sidebar_lyrics_align_desc: "Text alignment for singing lines in the docked right sidebar.",
            fullscreen_layout_placement: "Fullscreen Layout Placement",
            fullscreen_layout_placement_desc: "Choose which side album artwork and player controls appear on in fullscreen mode.",
            fullscreen_lyrics_align: "Fullscreen Lyrics Alignment",
            fullscreen_lyrics_align_desc: "Text alignment for singing lines in the edge-to-edge fullscreen window.",
            controls_left_lyrics_right: "Controls Left, Lyrics Right",
            controls_right_lyrics_left: "Controls Right, Lyrics Left",
            align_left: "Left",
            align_center: "Center",
            align_right: "Right",
            custom_css_precedence_notice: "Custom CSS is active: Fullscreen lyrics styling and text alignment are governed by your custom CSS rules below. Sidebar lyrics remain governed by sidebar settings.",
            karaoke_preset_label: "Karaoke Golden Bounce",
            css_reference_guide_title: "Custom CSS Guide & Class Reference",
            css_scope_note: "Custom CSS applies exclusively to the Fullscreen Lyrics stage and does not alter the sidebar or main player.",
            launch_lyrics_preview: "Preview Animations & Looks",
            preview_lyrics_window_title: "Attune — Lyrics Live Animation Preview",
            preview_fullscreen_mode: "Fullscreen View",
            preview_sidebar_mode: "Docked Sidebar",
            preview_simulating_line: "Simulating Line",
            preview_speed: "Speed",
            preview_track_sample: "Sample Track",
            preview_notice_realtime: "Live sync active: changes in Settings reflect here in real-time.",
            preview_format_line_sync: "Line Sync (LRC)",
            preview_format_word_sync: "Word Sync (TTML/ELRC)",
            preview_word_engine_active: "TTML / ELRC Word Engine Active",
            preview_syllable_progress: "Word Progress",
            lyrics_font_family: "Lyrics Font Family",
            lyrics_font_family_desc: "Select a typography font face for rendering singing lines across sidebar and fullscreen views.",
            font_group_default: "Default & Attune Fonts",
            font_group_system: "System Installed Fonts",
            font_preview_sample: "Live Typography Preview",
            lyrics_text_size: "Lyrics Text Size",
            sidebar_lyrics_size: "Sidebar Lyrics Text Size",
            fullscreen_lyrics_size: "Fullscreen Lyrics Text Size",
            decrease_lyrics_size: "Decrease Lyrics Text Size (A-)",
            increase_lyrics_size: "Increase Lyrics Text Size (A+)",
            reset_lyrics_size: "Reset Text Size",
            lyrics_text_size_desc: "Adjust the font size of synchronized singing lines for docked sidebar and edge-to-edge fullscreen views.",

            // Artist Profile Finder & Matcher Stage
            select_artist_to_find_profile: "Select an artist to find their profile",
            select_artist_to_find_profile_desc: "No valid artist is selected. Please choose an artist from your library or player to find, link, or edit their profile.",
            artist_profile_finder: "Artist Profile Finder & Matcher",
            editing_profile_for_artist: "Target Artist",
            find_artist_manually: "Find Artist Profile Manually...",
            artist_profile_not_found: "Artist profile not found automatically",
            artist_profile_not_found_desc: "This artist doesn't have an online portrait or bio yet. You can search online or link a custom profile manually.",
            search_artist_placeholder: "Search artist name or alternative spelling...",
            apply_and_link_artist: "Apply & Link Profile",
            linked_profile_success: "Successfully linked artist profile!",
            custom_profile: "Custom / Manual Entry",
            image_url: "Portrait Image URL",
            banner_url: "Banner Image URL",
            bio_description: "Biography & Notes",
            no_artist_candidates_found: "No matching artist profiles found.",
            no_artist_candidates_guide: "Try searching with alternative spelling, romanization, or Japanese characters, or enter a custom portrait URL in Custom Entry.",
        },
        Language::Filipino => Strings {
            app_name: "Attune",
            home: "Tahanan",
            for_you: "Para sa Iyo",
            library: "Koleksyon",
            albums: "Mga Album",
            artists: "Mga Artista",
            favorites: "Mga Paborito",
            favorite_songs: "Mga Paboritong Kanta",
            favorite_artists: "Mga Paboritong Artista",
            favorite_albums: "Mga Paboritong Album",
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
            help_guide: "Gabay sa Tulong",
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
            favorite_artists_in_collection: "mga paboritong artista sa iyong koleksyon",
            favorite_albums_in_collection: "mga paboritong album sa iyong koleksyon",
            bpm: "BPM",
            key: "Key",
            time: "Oras",
            actions: "Mga Aksyon",
            no_favorites_yet: "Wala pang paboritong kanta. Pindutin ang pusong icon sa tabi ng kanta upang idagdag dito.",
            no_favorite_artists_yet: "Wala pang paboritong artista. Pindutin ang pusong icon sa tabi ng artista upang idagdag dito.",
            no_favorite_albums_yet: "Wala pang paboritong album. Pindutin ang pusong icon sa tabi ng album upang idagdag dito.",

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
            reset_bands: "I-reset",

            // Process Manager & Hardware
            process_and_hardware_manager: "Tagapamahala ng Proseso at Hardware",
            kernel_dac_status: "ALSA Kernel Stream at Katayuan ng Hardware DAC",
            watchguard_stability: "Watchguard Real-Time na Katatagan at Guardrails",
            certified_shared_libraries: "Mga Sertipikadong External Shared Library (.so / .dll)",
            cpu_timeline_title: "Timeline ng Paggamit ng CPU ng Proseso",
            memory_timeline_title: "Timeline ng Paggamit ng Memory (RSS)",

            // About Stage
            architecture_and_technology: "Arkitektura at Teknolohiya",
            open_source_distribution: "Open Source at Pamamahagi",
            special_thanks_and_credits: "Espesyal na Pasasalamat at Kredito sa Open Source",
            custom_lyrics_css: "Pasadyang CSS Animation para sa Fullscreen Lyrics",
            custom_lyrics_css_desc: "Maglagay ng pasadyang CSS keyframe animation at estilo sa fullscreen lyrics (hango sa Better Lyrics).",

            // Lyrics Layout & Alignment
            sidebar_lyrics_align: "Pagkakapantay ng Lyrics sa Sidebar",
            sidebar_lyrics_align_desc: "Pagkakapantay ng teksto ng lyrics sa kanang nakadaong na sidebar.",
            fullscreen_layout_placement: "Posisyon ng Layout sa Fullscreen",
            fullscreen_layout_placement_desc: "Piliin kung saang bahagi lalabas ang album art at mga kontrol sa fullscreen mode.",
            fullscreen_lyrics_align: "Pagkakapantay ng Fullscreen Lyrics",
            fullscreen_lyrics_align_desc: "Pagkakapantay ng teksto ng mga linya sa edge-to-edge na fullscreen window.",
            controls_left_lyrics_right: "Controls sa Kaliwa, Lyrics sa Kanan",
            controls_right_lyrics_left: "Controls sa Kanan, Lyrics sa Kaliwa",
            align_left: "Kaliwa",
            align_center: "Gitna",
            align_right: "Kanan",
            custom_css_precedence_notice: "Aktibo ang Custom CSS: Ang estilo at pagkakapantay ng fullscreen lyrics ay pinamamahalaan ng iyong CSS sa ibaba. Ang sidebar lyrics ay patuloy na sinusunod ang mga setting ng sidebar.",
            karaoke_preset_label: "Karaoke Golden Bounce",
            css_reference_guide_title: "Gabay sa Custom CSS at Sanggunian ng Klase",
            css_scope_note: "Ang Custom CSS ay nailalapat lamang sa Fullscreen Lyrics stage at hindi binabago ang sidebar o ang pangunahing player.",
            launch_lyrics_preview: "I-preview ang Animasyon at Anyo",
            preview_lyrics_window_title: "Attune — Live na Preview ng Animasyon ng Liriko",
            preview_fullscreen_mode: "Buong Screen na Anyo",
            preview_sidebar_mode: "Naka-dock na Sidebar",
            preview_simulating_line: "Pinasisiglang Linya",
            preview_speed: "Bilis",
            preview_track_sample: "Halimbawang Kanta",
            preview_notice_realtime: "Aktibo ang live sync: agad makikita ang mga pagbabago sa Settings dito.",
            preview_format_line_sync: "Sabay sa Linya (LRC)",
            preview_format_word_sync: "Sabay sa Salita (TTML/ELRC)",
            preview_word_engine_active: "Aktibo ang TTML / ELRC Word Engine",
            preview_syllable_progress: "Pagsulong ng Salita",
            lyrics_font_family: "Pamilya ng Font ng Titik",
            lyrics_font_family_desc: "Pumili ng font para sa pagpapakita ng mga linya ng kanta sa sidebar at fullscreen na mga view.",
            font_group_default: "Mga Default at Font ng Attune",
            font_group_system: "Mga Naka-install na Font ng Sistema",
            font_preview_sample: "Live na Preview ng Typograpiya",
            lyrics_text_size: "Laki ng Titik ng Kanta",
            sidebar_lyrics_size: "Laki ng Titik sa Sidebar",
            fullscreen_lyrics_size: "Laki ng Titik sa Fullscreen",
            decrease_lyrics_size: "Paliitin ang Titik ng Kanta (A-)",
            increase_lyrics_size: "Palakihin ang Titik ng Kanta (A+)",
            reset_lyrics_size: "I-reset ang Laki ng Titik",
            lyrics_text_size_desc: "Baguhin ang laki ng font ng mga linyang kasabay sa pagkanta para sa sidebar at fullscreen na mga view.",

            // Artist Profile Finder & Matcher Stage
            select_artist_to_find_profile: "Pumili ng artista upang hanapin ang kanilang profile",
            select_artist_to_find_profile_desc: "Walang wastong artistang napili. Mangyaring pumili ng artista mula sa iyong koleksyon upang hanapin o i-link ang kanilang profile.",
            artist_profile_finder: "Panghanap at Pagtutugma ng Profile ng Artista",
            editing_profile_for_artist: "Target na Artista",
            find_artist_manually: "Manu-manong Hanapin ang Profile ng Artista...",
            artist_profile_not_found: "Hindi awtomatikong nahanap ang profile ng artista",
            artist_profile_not_found_desc: "Ang artistang ito ay wala pang online portrait o talambuhay. Maaari kang maghanap online o mag-link ng profile nang manu-mano.",
            search_artist_placeholder: "Maghanap ng pangalan ng artista o ibang baybay...",
            apply_and_link_artist: "Ilapat at I-link ang Profile",
            linked_profile_success: "Matagumpay na nai-link ang profile ng artista!",
            custom_profile: "Manu-manong Entri",
            image_url: "URL ng Larawan ng Portrait",
            banner_url: "URL ng Larawan ng Banner",
            bio_description: "Talambuhay at Mga Tala",
            no_artist_candidates_found: "Walang nahanap na tugmang profile ng artista.",
            no_artist_candidates_guide: "Subukang maghanap gamit ang ibang baybay, romanization, o mga karakter sa Hapon, o maglagay ng sariling URL sa Manu-manong Entri.",
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
    map.insert("favorites", s.favorites);
    map.insert("favorite_songs", s.favorite_songs);
    map.insert("favorite_artists", s.favorite_artists);
    map.insert("favorite_albums", s.favorite_albums);
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
    map.insert("help_guide", s.help_guide);
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
    map.insert("favorite_artists_in_collection", s.favorite_artists_in_collection);
    map.insert("favorite_albums_in_collection", s.favorite_albums_in_collection);
    map.insert("bpm", s.bpm);
    map.insert("key", s.key);
    map.insert("time", s.time);
    map.insert("actions", s.actions);
    map.insert("no_favorites_yet", s.no_favorites_yet);
    map.insert("no_favorite_artists_yet", s.no_favorite_artists_yet);
    map.insert("no_favorite_albums_yet", s.no_favorite_albums_yet);

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
    map.insert("reset_bands", s.reset_bands);

    map.insert("process_and_hardware_manager", s.process_and_hardware_manager);
    map.insert("kernel_dac_status", s.kernel_dac_status);
    map.insert("watchguard_stability", s.watchguard_stability);
    map.insert("certified_shared_libraries", s.certified_shared_libraries);
    map.insert("cpu_timeline_title", s.cpu_timeline_title);
    map.insert("memory_timeline_title", s.memory_timeline_title);

    map.insert("architecture_and_technology", s.architecture_and_technology);
    map.insert("open_source_distribution", s.open_source_distribution);
    map.insert("special_thanks_and_credits", s.special_thanks_and_credits);
    map.insert("custom_lyrics_css", s.custom_lyrics_css);
    map.insert("custom_lyrics_css_desc", s.custom_lyrics_css_desc);

    map.insert("sidebar_lyrics_align", s.sidebar_lyrics_align);
    map.insert("sidebar_lyrics_align_desc", s.sidebar_lyrics_align_desc);
    map.insert("fullscreen_layout_placement", s.fullscreen_layout_placement);
    map.insert("fullscreen_layout_placement_desc", s.fullscreen_layout_placement_desc);
    map.insert("fullscreen_lyrics_align", s.fullscreen_lyrics_align);
    map.insert("fullscreen_lyrics_align_desc", s.fullscreen_lyrics_align_desc);
    map.insert("controls_left_lyrics_right", s.controls_left_lyrics_right);
    map.insert("controls_right_lyrics_left", s.controls_right_lyrics_left);
    map.insert("align_left", s.align_left);
    map.insert("align_center", s.align_center);
    map.insert("align_right", s.align_right);
    map.insert("custom_css_precedence_notice", s.custom_css_precedence_notice);
    map.insert("karaoke_preset_label", s.karaoke_preset_label);
    map.insert("css_reference_guide_title", s.css_reference_guide_title);
    map.insert("css_scope_note", s.css_scope_note);
    map.insert("launch_lyrics_preview", s.launch_lyrics_preview);
    map.insert("preview_lyrics_window_title", s.preview_lyrics_window_title);
    map.insert("preview_fullscreen_mode", s.preview_fullscreen_mode);
    map.insert("preview_sidebar_mode", s.preview_sidebar_mode);
    map.insert("preview_simulating_line", s.preview_simulating_line);
    map.insert("preview_speed", s.preview_speed);
    map.insert("preview_track_sample", s.preview_track_sample);
    map.insert("preview_notice_realtime", s.preview_notice_realtime);
    map.insert("preview_format_line_sync", s.preview_format_line_sync);
    map.insert("preview_format_word_sync", s.preview_format_word_sync);
    map.insert("preview_word_engine_active", s.preview_word_engine_active);
    map.insert("preview_syllable_progress", s.preview_syllable_progress);
    map.insert("lyrics_font_family", s.lyrics_font_family);
    map.insert("lyrics_font_family_desc", s.lyrics_font_family_desc);
    map.insert("font_group_default", s.font_group_default);
    map.insert("font_group_system", s.font_group_system);
    map.insert("font_preview_sample", s.font_preview_sample);
    map.insert("lyrics_text_size", s.lyrics_text_size);
    map.insert("sidebar_lyrics_size", s.sidebar_lyrics_size);
    map.insert("fullscreen_lyrics_size", s.fullscreen_lyrics_size);
    map.insert("decrease_lyrics_size", s.decrease_lyrics_size);
    map.insert("increase_lyrics_size", s.increase_lyrics_size);
    map.insert("reset_lyrics_size", s.reset_lyrics_size);
    map.insert("lyrics_text_size_desc", s.lyrics_text_size_desc);

    map.insert("select_artist_to_find_profile", s.select_artist_to_find_profile);
    map.insert("select_artist_to_find_profile_desc", s.select_artist_to_find_profile_desc);
    map.insert("artist_profile_finder", s.artist_profile_finder);
    map.insert("editing_profile_for_artist", s.editing_profile_for_artist);
    map.insert("find_artist_manually", s.find_artist_manually);
    map.insert("artist_profile_not_found", s.artist_profile_not_found);
    map.insert("artist_profile_not_found_desc", s.artist_profile_not_found_desc);
    map.insert("search_artist_placeholder", s.search_artist_placeholder);
    map.insert("apply_and_link_artist", s.apply_and_link_artist);
    map.insert("linked_profile_success", s.linked_profile_success);
    map.insert("custom_profile", s.custom_profile);
    map.insert("image_url", s.image_url);
    map.insert("banner_url", s.banner_url);
    map.insert("bio_description", s.bio_description);
    map.insert("no_artist_candidates_found", s.no_artist_candidates_found);
    map.insert("no_artist_candidates_guide", s.no_artist_candidates_guide);

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
