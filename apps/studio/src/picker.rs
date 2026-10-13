//! Never wait for a desktop portal/native folder dialog on the render thread.
use std::{
    path::PathBuf,
    sync::mpsc::{self, Receiver, TryRecvError},
};
type PickResult = Result<(u64, Option<PathBuf>), String>;
#[derive(Default)]
pub struct FolderPicker {
    pending: Option<Receiver<PickResult>>,
}
impl FolderPicker {
    pub fn is_open(&self) -> bool {
        self.pending.is_some()
    }
    pub fn start(&mut self, generation: u64, ctx: eframe::egui::Context) -> bool {
        self.start_with(generation, ctx, || {
            rfd::FileDialog::new()
                .set_title("Open Kedr workspace")
                .pick_folder()
        })
    }
    fn start_with(
        &mut self,
        generation: u64,
        ctx: eframe::egui::Context,
        pick: impl FnOnce() -> Option<PathBuf> + Send + 'static,
    ) -> bool {
        if self.is_open() {
            return false;
        }
        let (send, receive) = mpsc::channel();
        self.pending = Some(receive);
        std::thread::spawn(move || {
            let result=std::panic::catch_unwind(std::panic::AssertUnwindSafe(pick)).map(|path|(generation,path)).map_err(|_|"Folder picker failed; open a workspace by command-line path or check the desktop portal".to_string());
            let _ = send.send(result);
            ctx.request_repaint();
        });
        true
    }
    pub fn poll(&mut self) -> Option<Result<(u64, Option<PathBuf>), String>> {
        match self.pending.as_ref()?.try_recv() {
            Ok(result) => {
                self.pending = None;
                Some(result)
            }
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => {
                self.pending = None;
                Some(Err("Folder picker disconnected".into()))
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn delayed_dialog_does_not_block_and_duplicates_are_ignored() {
        let mut p = FolderPicker::default();
        let (release, wait) = mpsc::channel();
        let ctx = eframe::egui::Context::default();
        assert!(p.start_with(42, ctx.clone(), move || {
            wait.recv().unwrap();
            Some(PathBuf::from("workspace"))
        }));
        assert!(p.is_open());
        assert!(p.poll().is_none());
        assert!(!p.start_with(43, ctx, || None));
        release.send(()).unwrap();
        for _ in 0..1000 {
            if let Some(result) = p.poll() {
                let (id, path) = result.unwrap();
                assert_eq!(id, 42);
                assert_eq!(path, Some(PathBuf::from("workspace")));
                assert!(!p.is_open());
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        panic!("worker did not complete")
    }
    #[test]
    fn cancellation_and_failures_leave_no_pending_picker() {
        let mut p = FolderPicker::default();
        let (send, receive) = mpsc::channel();
        p.pending = Some(receive);
        send.send(Ok((7, None))).unwrap();
        assert_eq!(p.poll().unwrap().unwrap(), (7, None));
        assert!(!p.is_open());
        let (send, receive) = mpsc::channel();
        p.pending = Some(receive);
        drop(send);
        assert!(p.poll().unwrap().is_err());
        assert!(!p.is_open());
    }
}
