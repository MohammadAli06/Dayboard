use chrono::Utc;
use dayboard_core::{
    model::{Actor, Command, Response},
    Core,
};
use std::time::Instant;

fn main() {
    let dir = tempfile::tempdir().expect("temporary directory");
    let path = dir.path().join("bench.sqlite3");

    let start = Instant::now();
    let mut core = Core::open(&path).expect("open database");
    let open_ms = start.elapsed().as_secs_f64() * 1000.0;

    let count = 1000;
    let seed = Instant::now();
    for index in 0..count {
        core.execute(
            Command::CreateTask {
                text: format!("Benchmark task {index} #bench p2 30m"),
                parent_id: None,
            },
            Actor::Ui,
            Utc::now(),
        )
        .expect("create task");
    }
    let seed_ms = seed.elapsed().as_secs_f64() * 1000.0;

    let mut samples = Vec::new();
    for _ in 0..100 {
        let start = Instant::now();
        let response = core
            .execute(Command::Snapshot { search: None }, Actor::Ui, Utc::now())
            .expect("snapshot");
        if !matches!(response, Response::Snapshot(_)) {
            panic!("unexpected response");
        }
        samples.push(start.elapsed().as_secs_f64() * 1000.0);
    }
    samples.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let p50 = samples[samples.len() / 2];
    let p95 = samples[(samples.len() as f64 * 0.95) as usize - 1];
    let average = samples.iter().sum::<f64>() / samples.len() as f64;

    println!("tasks: {count}");
    println!("open: {open_ms:.2} ms");
    println!("seed ({count} tasks): {seed_ms:.1} ms");
    println!("snapshot min: {:.2} ms", samples[0]);
    println!("snapshot p50: {p50:.2} ms");
    println!("snapshot p95: {p95:.2} ms");
    println!("snapshot max: {:.2} ms", samples[samples.len() - 1]);
    println!("snapshot avg: {average:.2} ms");
}
