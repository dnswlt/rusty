use std::thread;
use std::time::Instant;

fn fib(n: i32) -> i32 {
    if n <= 1 {
        return 1;
    }
    return fib(n - 2) + fib(n - 1);
}

fn main() {
    let n = 42;
    for i in 1..16 {
        let start = Instant::now();
        let mut handles = Vec::with_capacity(i);
        for _ in 0..i {
            handles.push(thread::spawn(move || fib(n)));
        }
        for h in handles {
            h.join().unwrap();    
        }
        let duration = start.elapsed();
        println!("No. of thread: {}\n\tDone in {}ms", i, duration.as_millis());
    }
}
