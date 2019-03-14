use std::env::args;
use std::time::Instant;
use std::process;
use std::mem::size_of;
use std::thread;
use std::sync::{Arc, RwLock, Mutex};

fn help(msg: &str) {
    if msg.len() > 0 {
        println!("error: {}", msg);
    }
    println!("usage: mem -t <thread count, default 4> -g <GB of long array total>");
    process::exit(1);
}

fn worker(mem_use: usize) {
    let start_f = Instant::now();
    let vec_size = mem_use / size_of::<usize>();
    println!("allocating {} GB in vec of {} entries", mem_use, vec_size);
    let mut v = vec![0usize; vec_size];
    let alloc_f = Instant::now();
    println!("allocated in {:?}", (alloc_f-start_f));

    for i in 0..vec_size {
        let a_v = i * 1982usize;
        v[i] = a_v;
    }
    let write_f = Instant::now();
    println!("wrote to all entries in {:?}", (write_f-alloc_f));
}


fn main() {
    let argv : Vec<String> = args().skip(1).map( |x| x).collect();
    let mut mem_use: usize = 1 * 1024 * 1024;
    let mut i = 0;
    let mut thread_no = 4;

    while i < argv.len() {
        match &argv[i][..] {
            "--help" => { // field list processing
                help("command line requested help info");
            },
            "-g" => {
                i += 1;
                mem_use = argv[i].parse::<usize>().unwrap();
                mem_use *= 1024*1024*1024;
            },
             "-t" => {
                i += 1;
                thread_no = argv[i].parse::<usize>().unwrap();
            },
            x => { dbg!(x); }
        }
        i += 1;
    }
    let per_thread_mem = mem_use / thread_no;
    println!("Running with {} threads each with {} MB each", thread_no, per_thread_mem);
    let mut worker_handles = vec![];
    for _i in 0..thread_no {
        let h = thread::spawn(move || worker(per_thread_mem));
        worker_handles.push(h);
    }
    for h in worker_handles {
        h.join().unwrap();
    }
    println!("DONE");
}
