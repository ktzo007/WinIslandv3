//! Media Player (Spotify / browser / VLC / dll.) via WinRT GSMTC.
//!
//! Event-driven penuh: satu thread tidur di `rx.recv()`; hanya terbangun saat
//! Windows memicu CurrentSessionChanged / MediaPropertiesChanged / PlaybackInfoChanged.
//! Saat Game Mode aktif semua handler di-unsubscribe (`suspend`) dan dipasang
//! lagi (`resume`) ketika kapsul tampil kembali.

use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::Mutex;
use std::time::Duration;

use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde::Serialize;
use tauri::{AppHandle, Emitter};
use windows::{
    Foundation::TypedEventHandler,
    Media::Control::{
        CurrentSessionChangedEventArgs,
        GlobalSystemMediaTransportControlsSession as Session,
        GlobalSystemMediaTransportControlsSessionManager as Manager,
        GlobalSystemMediaTransportControlsSessionMediaProperties as Props,
        GlobalSystemMediaTransportControlsSessionPlaybackStatus as Status,
        MediaPropertiesChangedEventArgs, PlaybackInfoChangedEventArgs, SessionsChangedEventArgs,
    },
    Storage::Streams::DataReader,
    Win32::System::WinRT::{RoInitialize, RO_INIT_MULTITHREADED},
};

#[derive(Serialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MediaInfo {
    title: String,
    artist: String,
    album: String,
    source: String,
    playing: bool,
    can_next: bool,
    can_prev: bool,
    /// Timeline (ms). Frontend meng-ekstrapolasi posisi: position + (now - updated).
    position_ms: i64,
    duration_ms: i64,
    updated_ms: i64,
    /// Berubah hanya saat sampul berganti → frontend mengambil gambar via `media_art`.
    art_id: u64,
}

#[derive(Clone, Copy)]
enum Control {
    PlayPause,
    Next,
    Prev,
    Seek(i64),
}

enum Msg {
    Rebind,
    Changed,
    Resend,
    Control(Control),
    Suspend,
    Resume,
    Stop,
}

enum Flow {
    Idle,
    Publish,
    Stop,
}

static TX: Mutex<Option<Sender<Msg>>> = Mutex::new(None);
static ART: Mutex<Option<String>> = Mutex::new(None);

fn send(m: Msg) {
    if let Some(tx) = TX.lock().unwrap().as_ref() {
        let _ = tx.send(m);
    }
}

pub fn suspend() {
    send(Msg::Suspend)
}
pub fn resume() {
    send(Msg::Resume)
}
pub fn stop() {
    send(Msg::Stop)
}

#[tauri::command]
pub fn media_refresh() {
    send(Msg::Resend)
}

#[tauri::command]
pub fn media_art() -> Option<String> {
    ART.lock().unwrap().clone()
}

#[tauri::command]
pub fn media_control(action: String) {
    let c = match action.as_str() {
        "play_pause" => Control::PlayPause,
        "next" => Control::Next,
        "prev" => Control::Prev,
        _ => return,
    };
    send(Msg::Control(c));
}

#[tauri::command]
pub fn media_seek(ms: i64) {
    send(Msg::Control(Control::Seek(ms.max(0))));
}

pub fn start(app: AppHandle) {
    let (tx, rx) = mpsc::channel();
    *TX.lock().unwrap() = Some(tx.clone());
    let _ = std::thread::Builder::new()
        .name("smtc".into())
        .spawn(move || worker(app, tx, rx));
}

type Cleanup = Box<dyn FnOnce()>;

struct Worker {
    app: AppHandle,
    tx: Sender<Msg>,
    manager: Manager,
    mgr_subs: Vec<Cleanup>,
    ses_subs: Vec<Cleanup>,
    session: Option<Session>,
    active: bool,
    force: bool,
    last: Option<MediaInfo>,
    art_key: String,
    art_id: u64,
    have_art: bool,
    art_tries: u8,
}

fn worker(app: AppHandle, tx: Sender<Msg>, rx: Receiver<Msg>) {
    unsafe {
        let _ = RoInitialize(RO_INIT_MULTITHREADED);
    }
    let manager = match Manager::RequestAsync().and_then(|op| op.get()) {
        Ok(m) => m,
        Err(_) => return,
    };

    let mut w = Worker {
        app,
        tx,
        manager,
        mgr_subs: Vec::new(),
        ses_subs: Vec::new(),
        session: None,
        active: false,
        force: true,
        last: None,
        art_key: String::new(),
        art_id: 0,
        have_art: false,
        art_tries: 0,
    };
    w.resume();
    w.publish();

    loop {
        // Sampul kadang tiba terlambat → coba lagi dengan timeout, bukan polling terus-menerus.
        let first = if w.wants_retry() {
            match rx.recv_timeout(Duration::from_millis(1500)) {
                Ok(m) => m,
                Err(RecvTimeoutError::Timeout) => Msg::Changed,
                Err(_) => break,
            }
        } else {
            match rx.recv() {
                Ok(m) => m,
                Err(_) => break,
            }
        };

        // Gabungkan rentetan event (Spotify memicu 3-5 event per pergantian lagu).
        let (mut publish, mut stop) = (false, false);
        let mut next = Some(first);
        while let Some(m) = next.take() {
            match w.handle(m) {
                Flow::Publish => publish = true,
                Flow::Stop => {
                    stop = true;
                    break;
                }
                Flow::Idle => {}
            }
            next = rx.recv_timeout(Duration::from_millis(120)).ok();
        }
        if stop {
            break;
        }
        if publish {
            w.publish();
        }
    }
    w.suspend(); // unsubscribe semua handler
}

impl Worker {
    fn handle(&mut self, m: Msg) -> Flow {
        match m {
            Msg::Rebind => {
                if self.active {
                    self.bind_session();
                }
                Flow::Publish
            }
            Msg::Changed => Flow::Publish,
            Msg::Resend => {
                self.force = true;
                Flow::Publish
            }
            Msg::Control(c) => {
                self.control(c);
                Flow::Idle
            }
            Msg::Suspend => {
                self.suspend();
                Flow::Idle
            }
            Msg::Resume => {
                self.resume();
                Flow::Publish
            }
            Msg::Stop => Flow::Stop,
        }
    }

    fn wants_retry(&self) -> bool {
        self.active && self.last.is_some() && !self.have_art && self.art_tries < 6
    }

    // ---------- subscribe / unsubscribe ----------
    fn resume(&mut self) {
        if self.active {
            return;
        }
        self.active = true;
        self.force = true;
        self.subscribe_manager();
        self.bind_session();
    }

    fn suspend(&mut self) {
        if !self.active {
            return;
        }
        self.active = false;
        for f in self.mgr_subs.drain(..) {
            f();
        }
        for f in self.ses_subs.drain(..) {
            f();
        }
        self.session = None;
    }

    fn subscribe_manager(&mut self) {
        let t = self.tx.clone();
        let handler = TypedEventHandler::<Manager, CurrentSessionChangedEventArgs>::new(move |_, _| {
            let _ = t.send(Msg::Rebind);
            Ok(())
        });
        if let Ok(token) = self.manager.CurrentSessionChanged(&handler) {
            let m = self.manager.clone();
            self.mgr_subs.push(Box::new(move || {
                let _ = m.RemoveCurrentSessionChanged(token);
            }));
        }

        let t = self.tx.clone();
        let handler = TypedEventHandler::<Manager, SessionsChangedEventArgs>::new(move |_, _| {
            let _ = t.send(Msg::Rebind);
            Ok(())
        });
        if let Ok(token) = self.manager.SessionsChanged(&handler) {
            let m = self.manager.clone();
            self.mgr_subs.push(Box::new(move || {
                let _ = m.RemoveSessionsChanged(token);
            }));
        }
    }

    fn bind_session(&mut self) {
        for f in self.ses_subs.drain(..) {
            f();
        }
        self.session = self.manager.GetCurrentSession().ok();
        let Some(s) = self.session.clone() else { return };

        let t = self.tx.clone();
        let handler = TypedEventHandler::<Session, MediaPropertiesChangedEventArgs>::new(move |_, _| {
            let _ = t.send(Msg::Changed);
            Ok(())
        });
        if let Ok(token) = s.MediaPropertiesChanged(&handler) {
            let s2 = s.clone();
            self.ses_subs.push(Box::new(move || {
                let _ = s2.RemoveMediaPropertiesChanged(token);
            }));
        }

        let t = self.tx.clone();
        let handler = TypedEventHandler::<Session, PlaybackInfoChangedEventArgs>::new(move |_, _| {
            let _ = t.send(Msg::Changed);
            Ok(())
        });
        if let Ok(token) = s.PlaybackInfoChanged(&handler) {
            let s2 = s.clone();
            self.ses_subs.push(Box::new(move || {
                let _ = s2.RemovePlaybackInfoChanged(token);
            }));
        }
    }

    // ---------- aksi ----------
    fn control(&self, c: Control) {
        let Some(s) = &self.session else { return };
        let op = match c {
            Control::PlayPause => s.TryTogglePlayPauseAsync(),
            Control::Next => s.TrySkipNextAsync(),
            Control::Prev => s.TrySkipPreviousAsync(),
            Control::Seek(ms) => s.TryChangePlaybackPositionAsync(ms * 10_000),
        };
        let _ = op.and_then(|o| o.get());
    }

    // ---------- publish ----------
    fn publish(&mut self) {
        if !self.active {
            return;
        }
        let info = self.read();
        if self.force || info != self.last {
            self.force = false;
            self.last = info.clone();
            let _ = self.app.emit("media-update", info);
        }
    }

    fn read(&mut self) -> Option<MediaInfo> {
        let s = self.session.clone()?;
        let props = s.TryGetMediaPropertiesAsync().ok()?.get().ok()?;
        let title = props.Title().map(|v| v.to_string()).unwrap_or_default();
        let artist = props.Artist().map(|v| v.to_string()).unwrap_or_default();
        let album = props.AlbumTitle().map(|v| v.to_string()).unwrap_or_default();
        if title.is_empty() && artist.is_empty() {
            return None;
        }

        let key = format!("{title}\u{1}{artist}\u{1}{album}");
        if key != self.art_key {
            self.art_key = key;
            self.have_art = false;
            self.art_tries = 0;
            self.art_id += 1;
            *ART.lock().unwrap() = None;
        }
        if !self.have_art && self.art_tries < 6 {
            self.art_tries += 1;
            if let Some(a) = read_thumbnail(&props) {
                *ART.lock().unwrap() = Some(a);
                self.have_art = true;
                self.art_id += 1;
            }
        }

        let pb = s.GetPlaybackInfo().ok()?;
        let playing = pb.PlaybackStatus().map(|st| st == Status::Playing).unwrap_or(false);
        let ctl = pb.Controls().ok();
        let can_next = ctl.as_ref().and_then(|c| c.IsNextEnabled().ok()).unwrap_or(false);
        let can_prev = ctl.as_ref().and_then(|c| c.IsPreviousEnabled().ok()).unwrap_or(false);
        let source = s.SourceAppUserModelId().map(|v| v.to_string()).unwrap_or_default();

        // Timeline: satuan WinRT = 100 ns. LastUpdatedTime = FILETIME (sejak 1601) → epoch ms.
        let (position_ms, duration_ms, updated_ms) = s
            .GetTimelineProperties()
            .ok()
            .map(|t| {
                let start = t.StartTime().map(|d| d.Duration / 10_000).unwrap_or(0);
                let end = t.EndTime().map(|d| d.Duration / 10_000).unwrap_or(0);
                let pos = t.Position().map(|d| d.Duration / 10_000).unwrap_or(0);
                let upd = t
                    .LastUpdatedTime()
                    .map(|d| (d.UniversalTime - 116_444_736_000_000_000) / 10_000)
                    .unwrap_or(0);
                ((pos - start).max(0), (end - start).max(0), upd)
            })
            .unwrap_or((0, 0, 0));

        Some(MediaInfo {
            title,
            artist,
            album,
            source,
            playing,
            can_next,
            can_prev,
            position_ms,
            duration_ms,
            updated_ms,
            art_id: self.art_id,
        })
    }
}

fn read_thumbnail(props: &Props) -> Option<String> {
    let stream = props.Thumbnail().ok()?.OpenReadAsync().ok()?.get().ok()?;
    let size = stream.Size().ok()? as u32;
    if size == 0 || size > 2_000_000 {
        return None;
    }
    let reader = DataReader::CreateDataReader(&stream.GetInputStreamAt(0).ok()?).ok()?;
    reader.LoadAsync(size).ok()?.get().ok()?;
    let mut buf = vec![0u8; size as usize];
    reader.ReadBytes(&mut buf).ok()?;
    let mime = if buf.starts_with(&[0x89, b'P', b'N', b'G']) { "image/png" } else { "image/jpeg" };
    Some(format!("data:{mime};base64,{}", STANDARD.encode(&buf)))
}
