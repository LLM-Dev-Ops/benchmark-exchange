//! Job scheduler with cron-like functionality

use crate::config::WorkerConfig;
use crate::queue::job::{CleanupExpiredDataJob, CleanupType, JobPriority, JobType};
use crate::queue::JobProducer;
use anyhow::Result;
use chrono::{DateTime, Datelike, Timelike, Utc};
use std::time::Duration;
use tokio::task::JoinHandle;
use tracing::{error, info};

/// Cron-like schedule expression
#[derive(Debug, Clone)]
pub struct Schedule {
    /// Minute (0-59)
    pub minute: Option<u32>,
    /// Hour (0-23)
    pub hour: Option<u32>,
    /// Day of month (1-31)
    pub day: Option<u32>,
    /// Month (1-12)
    pub month: Option<u32>,
    /// Day of week (0-6, where 0 is Sunday)
    pub day_of_week: Option<u32>,
}

impl Schedule {
    /// Create a schedule that runs every minute
    pub fn every_minute() -> Self {
        Self {
            minute: None,
            hour: None,
            day: None,
            month: None,
            day_of_week: None,
        }
    }

    /// Create a schedule that runs hourly at a specific minute
    pub fn hourly(minute: u32) -> Self {
        Self {
            minute: Some(minute),
            hour: None,
            day: None,
            month: None,
            day_of_week: None,
        }
    }

    /// Create a schedule that runs daily at a specific time
    pub fn daily(hour: u32, minute: u32) -> Self {
        Self {
            minute: Some(minute),
            hour: Some(hour),
            day: None,
            month: None,
            day_of_week: None,
        }
    }

    /// Create a schedule that runs weekly on a specific day and time
    pub fn weekly(day_of_week: u32, hour: u32, minute: u32) -> Self {
        Self {
            minute: Some(minute),
            hour: Some(hour),
            day: None,
            month: None,
            day_of_week: Some(day_of_week),
        }
    }

    /// Check if the schedule matches the given time
    pub fn matches(&self, time: &DateTime<Utc>) -> bool {
        if let Some(minute) = self.minute {
            if time.minute() != minute {
                return false;
            }
        }
        if let Some(hour) = self.hour {
            if time.hour() != hour {
                return false;
            }
        }
        if let Some(day) = self.day {
            if time.day() != day {
                return false;
            }
        }
        if let Some(month) = self.month {
            if time.month() != month {
                return false;
            }
        }
        if let Some(day_of_week) = self.day_of_week {
            if time.weekday().num_days_from_sunday() != day_of_week {
                return false;
            }
        }
        true
    }

    /// Parse a cron-like expression (simplified)
    /// Format: "minute hour day month day_of_week"
    /// Use * for any value
    pub fn parse(expr: &str) -> Result<Self> {
        let parts: Vec<&str> = expr.split_whitespace().collect();
        if parts.len() != 5 {
            return Err(anyhow::anyhow!(
                "Invalid cron expression, expected 5 fields"
            ));
        }

        let parse_field = |s: &str| -> Option<u32> {
            if s == "*" {
                None
            } else {
                s.parse().ok()
            }
        };

        Ok(Self {
            minute: parse_field(parts[0]),
            hour: parse_field(parts[1]),
            day: parse_field(parts[2]),
            month: parse_field(parts[3]),
            day_of_week: parse_field(parts[4]),
        })
    }
}

/// Scheduled job definition
#[derive(Debug, Clone)]
pub struct ScheduledJob {
    /// Job name
    pub name: String,
    /// Schedule
    pub schedule: Schedule,
    /// Job type to enqueue
    pub job_type: JobType,
    /// Job priority
    pub priority: JobPriority,
}

impl ScheduledJob {
    /// Create a new scheduled job
    pub fn new(
        name: impl Into<String>,
        schedule: Schedule,
        job_type: JobType,
        priority: JobPriority,
    ) -> Self {
        Self {
            name: name.into(),
            schedule,
            job_type,
            priority,
        }
    }
}

/// Job scheduler
pub struct Scheduler {
    config: WorkerConfig,
    producer: JobProducer,
    jobs: Vec<ScheduledJob>,
}

impl Scheduler {
    /// Create a new scheduler
    pub async fn new(config: WorkerConfig, producer: JobProducer) -> Result<Self> {
        let jobs = Self::default_scheduled_jobs();

        Ok(Self {
            config,
            producer,
            jobs,
        })
    }

    /// Get default scheduled jobs
    fn default_scheduled_jobs() -> Vec<ScheduledJob> {
        vec![
            // Clean up expired sessions daily at 2 AM
            ScheduledJob::new(
                "cleanup_expired_sessions",
                Schedule::daily(2, 0),
                JobType::CleanupExpiredData(CleanupExpiredDataJob {
                    cleanup_type: CleanupType::ExpiredSessions,
                    older_than_days: 7,
                }),
                JobPriority::Low,
            ),
            // Clean up temp files daily at 3 AM
            ScheduledJob::new(
                "cleanup_temp_files",
                Schedule::daily(3, 0),
                JobType::CleanupExpiredData(CleanupExpiredDataJob {
                    cleanup_type: CleanupType::TempFiles,
                    older_than_days: 1,
                }),
                JobPriority::Low,
            ),
            // Archive old submissions weekly on Sunday at 4 AM
            ScheduledJob::new(
                "cleanup_old_submissions",
                Schedule::weekly(0, 4, 0),
                JobType::CleanupExpiredData(CleanupExpiredDataJob {
                    cleanup_type: CleanupType::OldSubmissions,
                    older_than_days: 90,
                }),
                JobPriority::Low,
            ),
        ]
    }

    /// Add a scheduled job
    pub fn add_job(&mut self, job: ScheduledJob) {
        self.jobs.push(job);
    }

    /// Start the scheduler
    pub fn start(mut self) -> JoinHandle<()> {
        tokio::spawn(async move {
            if let Err(e) = self.run().await {
                error!(error = %e, "Scheduler error");
            }
        })
    }

    /// Run the scheduler loop
    async fn run(&mut self) -> Result<()> {
        if !self.config.scheduler.enabled {
            info!("Scheduler is disabled");
            return Ok(());
        }

        info!(
            tick_interval = self.config.scheduler.tick_interval,
            num_jobs = self.jobs.len(),
            "Starting scheduler"
        );

        let tick_interval = Duration::from_secs(self.config.scheduler.tick_interval);
        let mut last_check = Utc::now();

        loop {
            tokio::time::sleep(tick_interval).await;

            let now = Utc::now();

            // Check each scheduled job
            for scheduled_job in &self.jobs {
                // Only enqueue if this is the first time we've seen this minute
                // This prevents duplicate jobs within the same minute
                if scheduled_job.schedule.matches(&now)
                    && !scheduled_job.schedule.matches(&last_check)
                {
                    info!(
                        job_name = %scheduled_job.name,
                        "Enqueueing scheduled job"
                    );

                    if let Err(e) = self
                        .producer
                        .enqueue_with_priority(
                            scheduled_job.job_type.clone(),
                            scheduled_job.priority,
                        )
                        .await
                    {
                        error!(
                            job_name = %scheduled_job.name,
                            error = %e,
                            "Failed to enqueue scheduled job"
                        );
                    }
                }
            }

            last_check = now;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn test_schedule_every_minute() {
        let schedule = Schedule::every_minute();
        let time = Utc.with_ymd_and_hms(2024, 1, 15, 10, 30, 0).unwrap();
        assert!(schedule.matches(&time));
    }

    #[test]
    fn test_schedule_hourly() {
        let schedule = Schedule::hourly(30);
        let time1 = Utc.with_ymd_and_hms(2024, 1, 15, 10, 30, 0).unwrap();
        let time2 = Utc.with_ymd_and_hms(2024, 1, 15, 10, 45, 0).unwrap();

        assert!(schedule.matches(&time1));
        assert!(!schedule.matches(&time2));
    }

    #[test]
    fn test_schedule_daily() {
        let schedule = Schedule::daily(14, 30);
        let time1 = Utc.with_ymd_and_hms(2024, 1, 15, 14, 30, 0).unwrap();
        let time2 = Utc.with_ymd_and_hms(2024, 1, 15, 15, 30, 0).unwrap();

        assert!(schedule.matches(&time1));
        assert!(!schedule.matches(&time2));
    }

    #[test]
    fn test_schedule_parse() {
        let schedule = Schedule::parse("30 14 * * *").unwrap();
        let time = Utc.with_ymd_and_hms(2024, 1, 15, 14, 30, 0).unwrap();
        assert!(schedule.matches(&time));
    }

    #[test]
    fn test_schedule_parse_invalid() {
        let result = Schedule::parse("30 14");
        assert!(result.is_err());
    }
}
