use std::collections::BTreeMap;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum JobState {
    Queued,
    Cancelled,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Job {
    pub id: u64,
    pub task: String,
    pub state: JobState,
}

#[derive(Default)]
pub struct JobScheduler {
    next_id: u64,
    jobs: BTreeMap<u64, Job>,
}

#[derive(Debug, thiserror::Error)]
pub enum JobError {
    #[error("unknown job: {0}")]
    Unknown(u64),
}

impl JobScheduler {
    pub fn new() -> Self {
        Self {
            next_id: 1,
            ..Self::default()
        }
    }
    pub fn enqueue(&mut self, task: impl Into<String>) -> Job {
        let job = Job {
            id: self.next_id,
            task: task.into(),
            state: JobState::Queued,
        };
        self.jobs.insert(job.id, job.clone());
        self.next_id += 1;
        job
    }
    pub fn get(&self, id: u64) -> Option<&Job> {
        self.jobs.get(&id)
    }
    pub fn cancel(&mut self, id: u64) -> Result<(), JobError> {
        let job = self.jobs.get_mut(&id).ok_or(JobError::Unknown(id))?;
        job.state = JobState::Cancelled;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn enqueues_queries_and_cancels_without_executing() {
        let mut scheduler = JobScheduler::new();
        let job = scheduler.enqueue("summarize");
        assert_eq!(scheduler.get(job.id).unwrap().state, JobState::Queued);
        scheduler.cancel(job.id).unwrap();
        assert_eq!(scheduler.get(job.id).unwrap().state, JobState::Cancelled);
    }
    #[test]
    fn rejects_unknown_jobs() {
        assert!(matches!(
            JobScheduler::new().cancel(9),
            Err(JobError::Unknown(9))
        ));
    }
}
