use clap::{Arg, Command};
use std::thread;
use std::time::{Duration, Instant};

// Arbitrary function that causes a lot of CPU usage without
// interacting with heap memory.
fn fib(n: i32) -> i32 {
    if n <= 1 {
        return 1;
    }
    return fib(n - 2) + fib(n - 1);
}

fn main() {
    let matches = Command::new("dupfinder")
        .version("0.1")
        .author("Dennis Walter <dennis.walter@gmail.com>")
        .about(concat!(
            "Execute useless CPU-bound work on multiple threads ",
            "to determine the number of cores."
        ))
        .arg(
            Arg::new("max-threads")
                .short('n')
                .long("max-threads")
                .help("Maximum number of threads to execute in parallel")
                .default_value("12")
                .takes_value(true),
        )
        .get_matches();
    let n = 42;
    let max_threads: usize = matches.value_of_t("max-threads").unwrap();
    let mut single_thread_dur = Duration::ZERO;
    for i in 1..max_threads + 1 {
        let start = Instant::now();
        let mut handles = Vec::with_capacity(i);
        for _ in 0..i {
            handles.push(thread::spawn(move || fib(n)));
        }
        for h in handles {
            h.join().unwrap();
        }
        let duration = start.elapsed();
        if i == 1 {
            single_thread_dur = duration;
        }
        println!(
            "{} threads: {}ms ({:.2}%)",
            i,
            duration.as_millis(),
            100.0 * (duration.as_micros() as f64) / (single_thread_dur.as_micros() as f64)
        );
    }
}
