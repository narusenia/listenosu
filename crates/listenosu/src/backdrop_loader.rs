//! Decodes song background images off the UI thread and keeps the last few.

use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender, channel};

/// Backgrounds are blurred and dimmed, so a big decode buys nothing.
const MAX_SIDE: u32 = 640;
const KEEP: usize = 8;

pub struct Decoded {
    pub path: PathBuf,
    pub rgba: Vec<u8>,
    pub width: usize,
    pub height: usize,
}

pub struct BackdropLoader {
    requests: Sender<PathBuf>,
    results: Receiver<Option<Decoded>>,
    wanted: Option<PathBuf>,
    cache: VecDeque<Arc<Decoded>>,
}

impl BackdropLoader {
    /// `ctx` is poked when an image is ready, so the UI repaints without polling.
    pub fn new(ctx: eframe::egui::Context) -> Self {
        let (requests, wanted) = channel::<PathBuf>();
        let (done, results) = channel();
        std::thread::spawn(move || {
            while let Ok(mut path) = wanted.recv() {
                // Only the newest request matters when the user skips quickly.
                while let Ok(newer) = wanted.try_recv() {
                    path = newer;
                }
                if done.send(decode(path)).is_err() {
                    break;
                }
                ctx.request_repaint();
            }
        });
        Self {
            requests,
            results,
            wanted: None,
            cache: VecDeque::new(),
        }
    }

    /// Asks for an image. A cached one is returned at once; otherwise `poll` delivers it later.
    pub fn request(&mut self, path: PathBuf) -> Option<Arc<Decoded>> {
        self.wanted = Some(path.clone());
        if let Some(hit) = self.cache.iter().find(|d| d.path == path) {
            return Some(hit.clone());
        }
        // A closed channel means the thread died; the backdrop just stays as it is.
        let _ = self.requests.send(path);
        None
    }

    /// Forgets the pending request, so a late result does not replace a cleared backdrop.
    pub fn cancel(&mut self) {
        self.wanted = None;
    }

    /// The requested image once it is decoded. Stale results are cached but not returned.
    pub fn poll(&mut self) -> Option<Arc<Decoded>> {
        let mut ready = None;
        for decoded in self.results.try_iter().flatten() {
            let decoded = Arc::new(decoded);
            if self.wanted.as_ref() == Some(&decoded.path) {
                ready = Some(decoded.clone());
            }
            self.cache.push_back(decoded);
            if self.cache.len() > KEEP {
                self.cache.pop_front();
            }
        }
        ready
    }
}

fn decode(path: PathBuf) -> Option<Decoded> {
    let image = image::open(&path).ok()?.thumbnail(MAX_SIDE, MAX_SIDE);
    let rgba = image.to_rgba8();
    Some(Decoded {
        path,
        width: rgba.width() as usize,
        height: rgba.height() as usize,
        rgba: rgba.into_raw(),
    })
}
