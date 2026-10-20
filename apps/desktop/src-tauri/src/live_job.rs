use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, PoisonError};

use voxel_protocol::WorkerProcess;

struct TrackedJob {
    port: u16,
    token: String,
    job_id: String,
}

/// Job de imagem que ainda pode ser interrompido.
#[derive(Default)]
pub struct LiveJobs {
    active: AtomicBool,
    stop: AtomicBool,
    current: Mutex<Option<TrackedJob>>,
}

pub struct Session<'a> {
    jobs: &'a LiveJobs,
}

impl LiveJobs {
    pub fn enter(&self) -> Session<'_> {
        self.active.store(true, Ordering::SeqCst);
        *self.current.lock().unwrap_or_else(PoisonError::into_inner) = None;
        Session { jobs: self }
    }

    pub fn stopped(&self) -> bool {
        self.stop.load(Ordering::SeqCst)
    }

    pub fn track(&self, port: u16, token: &str, job_id: &str) {
        *self.current.lock().unwrap_or_else(PoisonError::into_inner) = Some(TrackedJob {
            port,
            token: token.to_owned(),
            job_id: job_id.to_owned(),
        });
    }

    pub fn request_stop(&self) {
        if !self.active.load(Ordering::SeqCst) {
            return;
        }
        self.stop.store(true, Ordering::SeqCst);
        let job = self
            .current
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .as_ref()
            .map(|job| (job.port, job.token.clone(), job.job_id.clone()));
        if let Some((port, token, job_id)) = job {
            let _ = WorkerProcess::cancel_at(port, &token, &job_id);
        }
    }

    fn finish(&self) {
        self.active.store(false, Ordering::SeqCst);
        self.stop.store(false, Ordering::SeqCst);
        *self.current.lock().unwrap_or_else(PoisonError::into_inner) = None;
    }
}

impl Drop for Session<'_> {
    fn drop(&mut self) {
        self.jobs.finish();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn o_cancelamento_so_vale_enquanto_a_geracao_esta_ativa() {
        let jobs = LiveJobs::default();
        jobs.request_stop();
        assert!(!jobs.stopped());
        let session = jobs.enter();
        assert!(!jobs.stopped());
        jobs.request_stop();
        assert!(jobs.stopped());
        drop(session);
        assert!(!jobs.stopped());
    }
}
