#[macro_use]
extern crate log;
extern crate env_logger;
extern crate chrono;

use chrono::{DateTime, Utc};

use env_logger::Env;
use env_logger::fmt::Color;

use log::Level;

use std::io::Write;
use std::env::args;
use std::time::{Instant, Duration};
use std::process;
use std::mem::size_of;
use std::thread;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

pub fn greek(v: f64) -> String {
    const GR_BACKOFF: f64 = 24.0;
    const GROWTH: f64 = 1024.0;
    const KK: f64 = GROWTH;
    const MM: f64 = KK * GROWTH;
    const GG: f64 = MM * GROWTH;
    const TT: f64 = GG * GROWTH;
    const PP: f64 = TT * GROWTH;

    let a = v.abs();
    let t = if a > 0.0 && a < KK - GR_BACKOFF {
        (v, "B")
    } else if a >= KK - GR_BACKOFF && a < MM - (GR_BACKOFF * KK) {
        (v / KK, "K")
    } else if a >= MM - (GR_BACKOFF * KK) && a < GG - (GR_BACKOFF * MM) {
        (v / MM, "M")
    } else if a >= GG - (GR_BACKOFF * MM) && a < TT - (GR_BACKOFF * GG) {
        (v / GG, "G")
    } else if a >= TT - (GR_BACKOFF * GG) && a < PP - (GR_BACKOFF * TT) {
        (v / TT, "T")
    } else {
        (v / PP, "P")
    };

    let mut s = format!("{}", t.0);
    s.truncate(4);
    if s.ends_with(".") {
        s.pop();
    }


    format!("{}{}", s, t.1)
    // match v {
    // 	(std::ops::Range {start: 0, end: (KK - GR_BACKOFF)}) => v,
    // 	//KK-GR_BACKOFF .. MM-(GR_BACKOFF*KK)			=> v,
    // 	_ => v,
    // }
}


fn help(msg: &str) {
    if msg.len() > 0 {
        eprintln!("error: {}", msg);
    }
    eprintln!(
        r#"usage: mem
    -u <ticker counts between updates of stats>
    -l <lifetime in ms of process before exit>
    -i <ms interval of stat ticker>
    -t <thread count, default 4>
    -[G|M|K] < GB|MB|KB of long array total in bytes>"#);
    process::exit(1);
}

fn ticker(sleep_interval_ms: u64, stat_iterations: Arc<AtomicUsize>, stat_passes: Arc<AtomicUsize>) {
    info!("ticker start with {} ms interval", sleep_interval_ms);
    let mut last_count = stat_iterations.fetch_add(0, Ordering::SeqCst);
    loop {
        let interval_millis = Duration::from_millis(sleep_interval_ms);
        let now = Instant::now();
        thread::sleep(interval_millis);
        let elapsed_time = now.elapsed();
        {
            // info!("wake up");
            let this_count = stat_iterations.fetch_add(0, Ordering::SeqCst);
            let delta_count = (this_count - last_count) * 8; // convert to usize to bytes
            if delta_count > 0 {
                let raw = (delta_count as f64 * 1000.0) / elapsed_time.as_millis() as f64;
                error!("updates {}/s {}, {} array passes", greek(raw), greek((this_count * 8) as f64), stat_passes.fetch_add(0, Ordering::SeqCst));
            }
            last_count = this_count;
        }
    }
}

fn exiter(lifetime_ms: u64) {
    thread::sleep(Duration::from_millis(lifetime_ms));
    warn!("exiter thread exiting");
    process::exit(0);
}

fn worker(update_interval_count: usize, mem_use: usize, stat_iterations: Arc<AtomicUsize>, stat_passes: Arc<AtomicUsize>) {
    let start_f = Instant::now();
    let vec_size = mem_use / size_of::<usize>();
    info!("allocating {}B in vec of usize[{}] = {} bytes", greek(mem_use as f64), vec_size, mem_use);
    let mut v = vec![0usize; vec_size];
    let mut alloc_f = Instant::now();
    info!("allocated in {:?} usize[{}]", (alloc_f - start_f), vec_size);

    let mut stat_update = 0;
    let mut pass_local = 0;
    loop {
        let stat_update_iterval = update_interval_count;
        for i in 0..vec_size {
            let a_v = i * 1982usize;
            v[i] = a_v;
            if v[i] != i * 1982usize {
                info!("bad value");
                process::exit(1);
            }
            stat_update += 1;
            if stat_update >= stat_update_iterval {
                stat_iterations.fetch_add(stat_update, Ordering::SeqCst);
                stat_update = 0;
                stat_passes.fetch_add(pass_local, Ordering::SeqCst);
                pass_local = 0;
            }
        }
        pass_local += 1;
    }
}


fn main() {
    let mut builder = env_logger::Builder::new();

    builder.format(|buf, record| {
        writeln!(buf, "{} [{:4}] [{:>12}:{:3}] {:>5}: {} ",Utc::now().format("%Y-%m-%d %H:%M:%S%.3f"),
                 thread::current().name().unwrap(),
                 record.file().unwrap(),
                 record.line().unwrap(),
                 record.level(),
                 record.args())
    });

    builder.filter_level(log::LevelFilter::Info);

    builder.init();

    //env_logger::from_env(Env::default().default_filter_or("info").wr).init();

    let argv: Vec<String> = args().skip(1).map(|x| x).collect();
    let mut mem_use: usize = 1 * 1024 * 1024;
    let mut i = 0;
    let mut thread_no = 4;
    let mut sleep_interval_ms = 1000;
    let mut update_interval_count: usize = 1_000_000;
    let mut lifetime_ms = 60000;
    while i < argv.len() {
        match &argv[i][..] {
            "--help" => { // field list processing
                help("command line requested help info");
            }
            "-G" => {
                i += 1;
                mem_use = argv[i].parse::<usize>().unwrap();
                mem_use *= 1024 * 1024 * 1024;
            }
            "-M" => {
                i += 1;
                mem_use = argv[i].parse::<usize>().unwrap();
                mem_use *= 1024 * 1024;
            }
            "-K" => {
                i += 1;
                mem_use = argv[i].parse::<usize>().unwrap();
                mem_use *= 1024;
            }
            "-t" => {
                i += 1;
                thread_no = argv[i].parse::<usize>().unwrap();
            }
            "-i" => {
                i += 1;
                sleep_interval_ms = argv[i].parse::<u64>().unwrap();
            }
            "-l" => {
                i += 1;
                lifetime_ms = argv[i].parse::<u64>().unwrap();
            }
            "-u" => {
                i += 1;
                update_interval_count = argv[i].parse::<usize>().unwrap();
            }
            x => { help(&format!("option {} not understood", x)); }
        }
        i += 1;
    }
    let per_thread_mem = mem_use / thread_no;
    info!("Running with {} threads each with {}B each, total={}", thread_no, greek(per_thread_mem as f64), thread_no * per_thread_mem);
    let mut worker_handles = vec![];
    let iterations = Arc::new(AtomicUsize::new(0));
    let passes = Arc::new(AtomicUsize::new(0));
    for _i in 0..thread_no {
        let iters = Arc::clone(&iterations);
        let pass = Arc::clone(&passes);
        let h = thread::Builder::new().name(format!("w{}", i).to_string())
            .spawn(move || worker(update_interval_count, per_thread_mem, iters, pass));
        worker_handles.push(h);
    }

    {
        let iters = Arc::clone(&iterations);
        let pass = Arc::clone(&passes);
        let ticker = thread::Builder::new().name("tick".to_string())
            .spawn(move || ticker(sleep_interval_ms, iters, pass));
        worker_handles.push(ticker);
    }

    let _exit_h = thread::Builder::new().name("exit".to_string()).spawn(move || exiter(lifetime_ms));

    for h in worker_handles {
        h.unwrap().join().unwrap();
    }


    info!("DONE");
}
