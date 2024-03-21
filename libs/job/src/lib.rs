use std::time::Duration;

use log::info;
use reqwest::Client;
use tokio_cron_scheduler::{Job, JobScheduler};

#[derive(Clone)]
pub struct JobConfig{
    pub name: String,
    pub cron: String,
    pub url: String,
    pub params: Vec<(String/*key*/, String/*value*/)>
}


pub async fn run_job_scheduler(job_configs : Vec<JobConfig>) -> anyhow::Result<()> {
    let mut sched = JobScheduler::new().await?;

    for config in job_configs {
        // Add async job
        let cron = config.cron.to_string();
        sched.add(
            Job::new_async(cron.as_str(), move |uuid, mut l| {
                let copy_config = config.clone();
                Box::pin(async move {
                    info!("job execute finished : {:?}", execute_job(&copy_config).await);
                    // Query the next execution time for this job
                    let next_tick = l.next_tick_for_job(uuid).await;
                    match next_tick {
                        Ok(Some(ts)) => info!("Next time for {} job is {:?}", copy_config.name, ts),
                        _ => info!("Could not get next tick for  job: {}", copy_config.name),
                    }
                })
            })?
        ).await?;
    }


    sched.start().await?;

    Ok(())
}


async fn execute_job(config: &JobConfig)->anyhow::Result<()>{
    info!("ready to invoke job : {},  url :{}", config.name, config.url);

    let client = Client::builder()
        .timeout(Duration::from_secs(5))
        .build()?;

    // Use the client to make a GET request
    let res = client.get(&config.url)
        .query(&config.params)
        .send()
        .await?.text().await?;;
    info!("invoke job : {} completed ,  res :{}",config.name, res);


    Ok(())
}
