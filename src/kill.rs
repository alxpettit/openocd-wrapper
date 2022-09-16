use async_std::task;
use heim::process::processes;
use tokio_stream::StreamExt;


pub(crate) fn process_kill(target_name: String) {
    task::block_on(async {
        let all_processes = if let Ok(processes) = processes().await {
            processes.filter_map(|process| process.ok()).collect().await
        } else {
            Vec::with_capacity(0)
        };
        for process in all_processes {
            if let Ok(name) = process.name().await {
                if name == target_name {
                    process.kill().await.unwrap();
                }
            }
        }
    });
}