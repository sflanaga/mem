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
    const KK : f64 = GROWTH;
    const MM : f64 = KK*GROWTH;
    const GG: f64 = MM*GROWTH;
    const TT: f64 = GG*GROWTH;
    const PP: f64 = TT*GROWTH;

    let a = v.abs();
    let t = if a > 0.0 && a < KK - GR_BACKOFF {
        (v, "B")
    } else if a >= KK - GR_BACKOFF && a < MM-(GR_BACKOFF*KK) {
        (v/KK, "K")
    } else if a >= MM-(GR_BACKOFF*KK) && a < GG-(GR_BACKOFF*MM) {
        (v/MM, "M")
    } else if a >= GG-(GR_BACKOFF*MM) && a < TT-(GR_BACKOFF*GG) {
        (v/GG, "G")
    } else if a >= TT-(GR_BACKOFF*GG) && a < PP-(GR_BACKOFF*TT) {
        (v/TT, "T")
    } else {
        (v/PP, "P")
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
        println!("error: {}", msg);
    }
    println!(
r#"usage: mem
    -u <ticker counts between updates of stats>
    -l <lifetime in ms of process before exit>
    -i <ms interval of stat ticker>
    -t <thread count, default 4>
    -[G|M|K] < GB|MB|KB of long array total in bytes>"#);
    process::exit(1);
}

fn ticker(sleep_interval_ms: u64, stat_iterations: Arc<AtomicUsize>, stat_passes: Arc<AtomicUsize>) {
    println!("ticker start with {} ms interval", sleep_interval_ms);
    let mut last_count = stat_iterations.fetch_add(0, Ordering::SeqCst);
    loop {
        let interval_millis = Duration::from_millis(sleep_interval_ms);
        let now = Instant::now();
        thread::sleep(interval_millis);
        let elapsed_time = now.elapsed();
        {
            // println!("wake up");
            let this_count = stat_iterations.fetch_add(0, Ordering::SeqCst);
            let delta_count = (this_count - last_count)*8; // convert to usize to bytes
            if delta_count > 0 {
                let raw = (delta_count as f64 * 1000.0) / elapsed_time.as_millis() as f64;
                println!("updates {}/s {} {}, array passes", greek(raw), greek((this_count*8) as f64), stat_passes.fetch_add(0, Ordering::SeqCst));
            }
            last_count = this_count;
        }
    }

}

fn exiter(lifetime_ms: u64) {
    thread::sleep(Duration::from_millis(lifetime_ms));
    println!("exiter thread exiting");
    process::exit(0);
}

fn worker(update_interval_count: usize, mem_use: usize, stat_iterations: Arc<AtomicUsize>, stat_passes: Arc<AtomicUsize>) {
    let start_f = Instant::now();
    let vec_size = mem_use / size_of::<usize>();
    println!("allocating {}B in vec of {} entries  update count: {}", greek(mem_use as f64), vec_size, update_interval_count);
    let mut v = vec![0usize; vec_size];
    let mut alloc_f = Instant::now();
    println!("allocated in {:?}", (alloc_f-start_f));

    let mut stat_update = 0;
    let mut pass_local = 0;
    loop {
  //      alloc_f = Instant::now();
        let stat_update_iterval = update_interval_count;
        for i in 0..vec_size {
            let a_v = i * 1982usize;
            v[i] = a_v;
            stat_update += 1;
            if stat_update >= stat_update_iterval {
                stat_iterations.fetch_add(stat_update, Ordering::SeqCst);
                stat_update = 0;
                stat_passes.fetch_add(pass_local, Ordering::SeqCst);
                pass_local = 0;
            }
        }
        pass_local += 1;
//        let write_f = Instant::now();
        // println!("wrote to all entries in {:?}  passes {}", (write_f - alloc_f), pass_local);
    }
}


fn main() {
    let argv : Vec<String> = args().skip(1).map( |x| x).collect();
    let mut mem_use: usize = 1 * 1024 * 1024;
    let mut i = 0;
    let mut thread_no = 4;
    let mut sleep_interval_ms = 1000;
    let mut update_interval_count : usize = 1_000_000;
    let mut lifetime_ms = 60000;
    while i < argv.len() {
        match &argv[i][..] {
            "--help" => { // field list processing
                help("command line requested help info");
            },
            "-G" => {
                i += 1;
                mem_use = argv[i].parse::<usize>().unwrap();
                mem_use *= 1024*1024*1024;
            },
            "-M" => {
                i += 1;
                mem_use = argv[i].parse::<usize>().unwrap();
                mem_use *= 1024*1024;
            },
            "-K" => {
                i += 1;
                mem_use = argv[i].parse::<usize>().unwrap();
                mem_use *= 1024;
            },
             "-t" => {
                i += 1;
                thread_no = argv[i].parse::<usize>().unwrap();
            },
            "-i" => {
                i += 1;
                sleep_interval_ms = argv[i].parse::<u64>().unwrap();
            },
            "-l" => {
                i += 1;
                lifetime_ms = argv[i].parse::<u64>().unwrap();
            },
            "-u" => {
                i += 1;
                update_interval_count = argv[i].parse::<usize>().unwrap();
            },
            x => { help(&format!("option {} not understood", x)); }
        }
        i += 1;
    }
    let per_thread_mem = mem_use / thread_no;
    println!("Running with {} threads each with {}B each", thread_no, greek(per_thread_mem as f64));
    let mut worker_handles = vec![];
    let iterations = Arc::new(AtomicUsize::new(0));
    let passes = Arc::new(AtomicUsize::new(0));
    for _i in 0..thread_no {
        let iters = Arc::clone(&iterations);
        let pass = Arc::clone(&passes);
        let h = thread::spawn(move || worker( update_interval_count, per_thread_mem, iters, pass));
        worker_handles.push(h);
    }

    {
        let iters = Arc::clone(&iterations);
        let pass = Arc::clone(&passes);
        let ticker = thread::spawn(move || ticker(sleep_interval_ms, iters, pass));
        worker_handles.push(ticker);
    }

    let exit_h = thread::spawn(move || exiter(lifetime_ms));

    for h in worker_handles {
        h.join().unwrap();
    }



    println!("DONE");
}
