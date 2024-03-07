use std::time::Duration;
use tokio_cron_scheduler::{Job, JobScheduler};

async fn run_job_scheduler() -> anyhow::Result<JobsSchedulerLocked> {
    let mut sched = JobScheduler::new().await?;

    // Add async job
    sched.add(
        Job::new_async("1/7 * * * * *", |uuid, mut l| {
            Box::pin(async move {
                println!("I run async every 7 seconds");

                // Query the next execution time for this job
                let next_tick = l.next_tick_for_job(uuid).await;
                match next_tick {
                    Ok(Some(ts)) => println!("Next time for 7s job is {:?}", ts),
                    _ => println!("Could not get next tick for 7s job"),
                }
            })
        })?
    ).await?;

    sched.start().await?;

    Ok(sched)
}


#[ignore]
#[tokio::test]
async fn test_all() -> anyhow::Result<()> {
    let task = tokio::spawn(run_job_scheduler());

    // task.await.expect("failed");


    // Wait while the jobs run
    tokio::time::sleep(Duration::from_secs(100)).await;

    Ok(())
}