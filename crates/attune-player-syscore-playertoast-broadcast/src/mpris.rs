//! MPRIS v2 (Media Player Remote Interfacing Specification) implementation for Linux.
//!
//! Exposes standard D-Bus interfaces on `org.mpris.MediaPlayer2.attune`:
//! - `org.mpris.MediaPlayer2`: App identity, desktop entry, quit, raise.
//! - `org.mpris.MediaPlayer2.Player`: Playback transport, timeline scrubber, metadata, volume, seek.
//!
//! Specifically resolves MPRIS timeline scrubber synchronization for KDE Plasma 6 (`libkmpris`),
//! GNOME Shell, waybar, and `playerctl` by:
//! 1. Broadcasting both `Position` in `PropertiesChanged` on timeline state updates.
//! 2. Broadcasting `Seeked(micros)` on initial playback start, track transition, unpausing, and discontinuous jumps.
//! 3. Accepting remote `SetPosition` and `Seek` commands and routing them into `MediaControlCommand`.

use std::collections::HashMap;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};

use zbus::blocking::Connection;
use zbus::interface;
use zbus::object_server::SignalEmitter;
use zvariant::{ObjectPath, OwnedValue, Value};

use crate::{MediaControlCommand, SystemPlaybackState, SystemRepeatMode, SystemTrackMetadata};

const OBJECT_PATH: &str = "/org/mpris/MediaPlayer2";
const NO_TRACK_PATH: &str = "/org/mpris/MediaPlayer2/TrackList/NoTrack";
const TRACK_ID_PREFIX: &str = "/org/mizproject/attune/track";

/// Helper converting Repeat enum to MPRIS LoopStatus string.
fn repeat_to_loop_status(mode: SystemRepeatMode) -> &'static str {
    match mode {
        SystemRepeatMode::Off => "None",
        SystemRepeatMode::One => "Track",
        SystemRepeatMode::All => "Playlist",
    }
}

/// Helper converting MPRIS LoopStatus string to Repeat enum.
fn loop_status_to_repeat(status: &str) -> SystemRepeatMode {
    match status {
        "Track" => SystemRepeatMode::One,
        "Playlist" => SystemRepeatMode::All,
        _ => SystemRepeatMode::Off,
    }
}

/// Generates a valid D-Bus ObjectPath for a track id.
fn track_object_path(track: Option<&SystemTrackMetadata>) -> ObjectPath<'static> {
    let Some(t) = track else {
        return ObjectPath::try_from(NO_TRACK_PATH).unwrap();
    };

    if t.id.trim().is_empty() {
        return ObjectPath::try_from(NO_TRACK_PATH).unwrap();
    }

    let sanitized: String = t
        .id
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();

    ObjectPath::try_from(format!("{TRACK_ID_PREFIX}/{sanitized}"))
        .unwrap_or_else(|_| ObjectPath::try_from(NO_TRACK_PATH).unwrap())
}

/// Builds the MPRIS metadata dictionary.
fn build_metadata_map(state: &SystemPlaybackState, art_url: &str) -> HashMap<String, OwnedValue> {
    let mut map = HashMap::new();
    let mut insert = |k: &str, v: Value<'_>| {
        if let Ok(owned) = OwnedValue::try_from(v) {
            map.insert(k.to_string(), owned);
        }
    };

    insert("mpris:trackid", Value::from(track_object_path(state.track.as_ref())));

    if let Some(ref t) = state.track {
        insert("xesam:title", Value::from(t.title.clone()));
        insert("xesam:album", Value::from(t.album.clone()));
        insert("xesam:artist", Value::from(t.artists.clone()));

        if !art_url.is_empty() {
            insert("mpris:artUrl", Value::from(art_url.to_string()));
        }

        if state.duration_ms > 0 {
            let micros = (state.duration_ms as i64).saturating_mul(1_000);
            insert("mpris:length", Value::from(micros));
        }

        if !t.url.is_empty() {
            insert("xesam:url", Value::from(t.url.clone()));
        }
    }

    map
}

struct SharedState {
    state: SystemPlaybackState,
    art_url: String,
    cmd_tx: Sender<MediaControlCommand>,
}

impl SharedState {
    fn send_cmd(&self, cmd: MediaControlCommand) {
        let _ = self.cmd_tx.send(cmd);
    }
}

pub struct RootInterface {
    shared: Arc<Mutex<SharedState>>,
}

#[interface(name = "org.mpris.MediaPlayer2")]
impl RootInterface {
    fn raise(&self) {
        self.shared.lock().unwrap().send_cmd(MediaControlCommand::Raise);
    }

    fn quit(&self) {
        self.shared.lock().unwrap().send_cmd(MediaControlCommand::Quit);
    }

    #[zbus(property)]
    fn can_quit(&self) -> bool {
        true
    }

    #[zbus(property)]
    fn can_raise(&self) -> bool {
        true
    }

    #[zbus(property)]
    fn fullscreen(&self) -> bool {
        false
    }

    #[zbus(property)]
    fn set_fullscreen(&self, _value: bool) {}

    #[zbus(property)]
    fn can_set_fullscreen(&self) -> bool {
        false
    }

    #[zbus(property)]
    fn has_track_list(&self) -> bool {
        false
    }

    #[zbus(property)]
    fn identity(&self) -> String {
        "Attune".to_string()
    }

    #[zbus(property)]
    fn desktop_entry(&self) -> String {
        "attune".to_string()
    }

    #[zbus(property)]
    fn supported_uri_schemes(&self) -> Vec<String> {
        vec!["file".to_string(), "http".to_string(), "https".to_string()]
    }

    #[zbus(property)]
    fn supported_mime_types(&self) -> Vec<String> {
        vec![
            "audio/flac".to_string(),
            "audio/mp3".to_string(),
            "audio/ogg".to_string(),
            "audio/wav".to_string(),
            "audio/x-wav".to_string(),
            "audio/aac".to_string(),
            "audio/m4a".to_string(),
        ]
    }
}

pub struct PlayerInterface {
    shared: Arc<Mutex<SharedState>>,
}

#[interface(name = "org.mpris.MediaPlayer2.Player")]
impl PlayerInterface {
    fn next(&self) {
        self.shared.lock().unwrap().send_cmd(MediaControlCommand::Next);
    }

    fn previous(&self) {
        self.shared.lock().unwrap().send_cmd(MediaControlCommand::Previous);
    }

    fn pause(&self) {
        self.shared.lock().unwrap().send_cmd(MediaControlCommand::Pause);
    }

    fn play_pause(&self) {
        self.shared.lock().unwrap().send_cmd(MediaControlCommand::TogglePlayPause);
    }

    fn stop(&self) {
        self.shared.lock().unwrap().send_cmd(MediaControlCommand::Stop);
    }

    fn play(&self) {
        self.shared.lock().unwrap().send_cmd(MediaControlCommand::Play);
    }

    fn seek(&self, offset: i64) {
        let secs = offset as f64 / 1_000_000.0;
        self.shared.lock().unwrap().send_cmd(MediaControlCommand::SeekBySec(secs));
    }

    fn set_position(&self, track_id: ObjectPath<'_>, position: i64) {
        let shared = self.shared.lock().unwrap();
        let current_path = track_object_path(shared.state.track.as_ref());
        if track_id != current_path {
            return;
        }

        if position >= 0 {
            let ms = (position as u64) / 1_000;
            shared.send_cmd(MediaControlCommand::SeekToMs(ms));
        }
    }

    fn open_uri(&self, _uri: String) {}

    #[zbus(signal)]
    pub async fn seeked(emitter: &SignalEmitter<'_>, position: i64) -> zbus::Result<()>;

    #[zbus(property)]
    fn playback_status(&self) -> String {
        let shared = self.shared.lock().unwrap();
        if shared.state.track.is_none() {
            "Stopped".to_string()
        } else if shared.state.is_playing {
            "Playing".to_string()
        } else {
            "Paused".to_string()
        }
    }

    #[zbus(property)]
    fn loop_status(&self) -> String {
        let shared = self.shared.lock().unwrap();
        repeat_to_loop_status(shared.state.repeat).to_string()
    }

    #[zbus(property)]
    fn set_loop_status(&self, value: String) {
        let rep = loop_status_to_repeat(&value);
        let shared = self.shared.lock().unwrap();
        shared.send_cmd(MediaControlCommand::SetRepeat(rep));
    }

    #[zbus(property)]
    fn shuffle(&self) -> bool {
        self.shared.lock().unwrap().state.shuffle
    }

    #[zbus(property)]
    fn set_shuffle(&self, value: bool) {
        let shared = self.shared.lock().unwrap();
        shared.send_cmd(MediaControlCommand::SetShuffle(value));
    }

    #[zbus(property)]
    fn volume(&self) -> f64 {
        self.shared.lock().unwrap().state.volume.clamp(0.0, 1.0)
    }

    #[zbus(property)]
    fn set_volume(&self, value: f64) {
        let clamped = if value.is_finite() { value.clamp(0.0, 1.0) } else { 1.0 };
        let shared = self.shared.lock().unwrap();
        shared.send_cmd(MediaControlCommand::SetVolume(clamped));
    }

    #[zbus(property)]
    fn metadata(&self) -> HashMap<String, OwnedValue> {
        let shared = self.shared.lock().unwrap();
        build_metadata_map(&shared.state, &shared.art_url)
    }

    /// Track position in microseconds.
    /// Emits `PropertiesChanged` when updated so KDE Plasma 6 `libkmpris` and waybar
    /// receive accurate elapsed time updates smoothly.
    #[zbus(property)]
    fn position(&self) -> i64 {
        let shared = self.shared.lock().unwrap();
        (shared.state.position_ms as i64).saturating_mul(1_000)
    }

    #[zbus(property)]
    fn rate(&self) -> f64 {
        1.0
    }

    #[zbus(property)]
    fn set_rate(&self, _value: f64) {}

    #[zbus(property)]
    fn minimum_rate(&self) -> f64 {
        1.0
    }

    #[zbus(property)]
    fn maximum_rate(&self) -> f64 {
        1.0
    }

    #[zbus(property)]
    fn can_go_next(&self) -> bool {
        self.shared.lock().unwrap().state.can_go_next
    }

    #[zbus(property)]
    fn can_go_previous(&self) -> bool {
        self.shared.lock().unwrap().state.can_go_previous
    }

    #[zbus(property)]
    fn can_play(&self) -> bool {
        self.shared.lock().unwrap().state.track.is_some()
    }

    #[zbus(property)]
    fn can_pause(&self) -> bool {
        self.shared.lock().unwrap().state.track.is_some()
    }

    #[zbus(property)]
    fn can_seek(&self) -> bool {
        self.shared.lock().unwrap().state.can_seek
    }

    #[zbus(property)]
    fn can_control(&self) -> bool {
        true
    }
}

/// Linux MPRIS host managing D-Bus lifecycle and signal emissions.
pub struct MprisHost {
    connection: Connection,
    shared: Arc<Mutex<SharedState>>,
    last_state: Option<SystemPlaybackState>,
    last_art_url: String,
    has_emitted_initial_seek: bool,
}

impl MprisHost {
    pub fn new(cmd_tx: Sender<MediaControlCommand>) -> Result<Self, zbus::Error> {
        let shared = Arc::new(Mutex::new(SharedState {
            state: SystemPlaybackState::default(),
            art_url: String::new(),
            cmd_tx,
        }));

        let root = RootInterface { shared: shared.clone() };
        let player = PlayerInterface { shared: shared.clone() };

        let connection = Connection::session()?;

        connection.object_server().at(OBJECT_PATH, root)?;
        connection.object_server().at(OBJECT_PATH, player)?;

        if connection.request_name("org.mpris.MediaPlayer2.attune").is_err() {
            let instance_name = format!("org.mpris.MediaPlayer2.attune.instance_{}", std::process::id());
            let _ = connection.request_name(instance_name.as_str());
        }

        Ok(Self {
            connection,
            shared,
            last_state: None,
            last_art_url: String::new(),
            has_emitted_initial_seek: false,
        })
    }

    pub fn update_state(&mut self, state: &SystemPlaybackState, art_url: &str) {
        {
            let mut shared = self.shared.lock().unwrap();
            shared.state = state.clone();
            shared.art_url = art_url.to_string();
        }

        let Ok(player_ref) = self.connection.object_server().interface::<_, PlayerInterface>(OBJECT_PATH) else {
            return;
        };

        let emitter = player_ref.signal_emitter();
        let interface = player_ref.get();

        let prev = self.last_state.take();
        let prev_art = std::mem::replace(&mut self.last_art_url, art_url.to_string());

        let track_changed = prev.as_ref().map(|old| old.track != state.track || old.duration_ms != state.duration_ms).unwrap_or(true);
        let art_changed = prev_art != art_url;
        if track_changed || art_changed {
            let _ = zbus::block_on(interface.metadata_changed(emitter));
        }

        let play_changed = prev.as_ref().map(|old| old.is_playing != state.is_playing || (old.track.is_some() != state.track.is_some())).unwrap_or(true);
        if play_changed {
            let _ = zbus::block_on(interface.playback_status_changed(emitter));
            let _ = zbus::block_on(interface.can_play_changed(emitter));
            let _ = zbus::block_on(interface.can_pause_changed(emitter));
        }

        let loop_changed = prev.as_ref().map(|old| old.repeat != state.repeat).unwrap_or(true);
        if loop_changed {
            let _ = zbus::block_on(interface.loop_status_changed(emitter));
        }

        let shuffle_changed = prev.as_ref().map(|old| old.shuffle != state.shuffle).unwrap_or(true);
        if shuffle_changed {
            let _ = zbus::block_on(interface.shuffle_changed(emitter));
        }

        let vol_changed = prev.as_ref().map(|old| (old.volume - state.volume).abs() > 0.001).unwrap_or(true);
        if vol_changed {
            let _ = zbus::block_on(interface.volume_changed(emitter));
        }

        let cap_changed = prev.as_ref().map(|old| {
            old.can_go_next != state.can_go_next
                || old.can_go_previous != state.can_go_previous
                || old.can_seek != state.can_seek
        }).unwrap_or(true);
        if cap_changed {
            let _ = zbus::block_on(interface.can_go_next_changed(emitter));
            let _ = zbus::block_on(interface.can_go_previous_changed(emitter));
            let _ = zbus::block_on(interface.can_seek_changed(emitter));
        }

        // Always notify Position change on every tick so D-Bus subscribers (KDE Plasma 6 libkmpris,
        // Waybar, etc.) receive real-time timeline movement smoothly without waiting for seeks.
        let _ = zbus::block_on(interface.position_changed(emitter));

        // Evaluate whether to emit MPRIS Seeked(micros) signal:
        // - On initial playback start with a track loaded.
        // - On track transition / change.
        // - When unpausing (transitioning from Paused to Playing) to establish the clock anchor.
        // - When position exhibits discontinuous jump (|current - (old + elapsed)| > 1000ms).
        let current_micros = (state.position_ms as i64).saturating_mul(1_000);
        let unpaused = prev.as_ref().map(|old| !old.is_playing && state.is_playing).unwrap_or(false);
        let drift = prev.as_ref().map(|old| {
            (state.position_ms as i64 - old.position_ms as i64).abs()
        }).unwrap_or(0);

        let need_seeked = (!self.has_emitted_initial_seek && state.is_playing && state.track.is_some())
            || track_changed
            || unpaused
            || (drift > 1200);

        if need_seeked {
            self.has_emitted_initial_seek = true;
            let _ = zbus::block_on(PlayerInterface::seeked(emitter, current_micros));
        }

        self.last_state = Some(state.clone());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc::channel;

    #[test]
    fn test_loop_status_conversion_roundtrip() {
        assert_eq!(repeat_to_loop_status(SystemRepeatMode::Off), "None");
        assert_eq!(repeat_to_loop_status(SystemRepeatMode::One), "Track");
        assert_eq!(repeat_to_loop_status(SystemRepeatMode::All), "Playlist");

        assert_eq!(loop_status_to_repeat("None"), SystemRepeatMode::Off);
        assert_eq!(loop_status_to_repeat("Track"), SystemRepeatMode::One);
        assert_eq!(loop_status_to_repeat("Playlist"), SystemRepeatMode::All);
        assert_eq!(loop_status_to_repeat("invalid"), SystemRepeatMode::Off);
    }

    #[test]
    fn test_track_object_path_sanitization() {
        let no_track = track_object_path(None);
        assert_eq!(no_track.as_str(), NO_TRACK_PATH);

        let track = SystemTrackMetadata::new("/path/to/my song (remix).flac", "Song", "Artist");
        let path = track_object_path(Some(&track));
        assert!(path.as_str().starts_with(TRACK_ID_PREFIX));
        assert!(!path.as_str().contains(' '));
        assert!(!path.as_str().contains('('));
        assert!(!path.as_str().contains(')'));
    }

    #[test]
    fn test_metadata_map_structure() {
        let mut state = SystemPlaybackState::default();
        let track = SystemTrackMetadata::new("track-99", "Bad Apple", "Nomico");
        state.track = Some(track);
        state.duration_ms = 210000;

        let map = build_metadata_map(&state, "file:///tmp/cover.jpg");
        assert!(map.contains_key("mpris:trackid"));
        assert!(map.contains_key("xesam:title"));
        assert!(map.contains_key("xesam:artist"));
        assert!(map.contains_key("mpris:artUrl"));
        assert!(map.contains_key("mpris:length"));
    }

    #[test]
    fn test_mpris_host_lifecycle() {
        let (tx, _rx) = channel();
        // Connecting to session bus might fail in some headless environments, so test gracefully
        if let Ok(mut host) = MprisHost::new(tx) {
            let mut state = SystemPlaybackState::default();
            state.track = Some(SystemTrackMetadata::new("id-1", "Test Title", "Test Artist"));
            state.is_playing = true;
            state.position_ms = 42000;
            state.duration_ms = 180000;

            host.update_state(&state, "file:///art.jpg");
            assert!(host.has_emitted_initial_seek);
        }
    }
}
