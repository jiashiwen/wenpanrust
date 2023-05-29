use tokio::{runtime, task};
pub fn main() {
    let core_ids = core_affinity::get_core_ids().unwrap();
    println!("core num {}", core_ids.len());
    // Get the core id 1.
    let core_id = core_ids[4];
    println!("{:?}", core_id);

    let rt = runtime::Builder::new_multi_thread()
        .worker_threads(8)
        .on_thread_start(move || {
            core_affinity::set_for_current(core_id.clone());
        })
        .enable_all()
        .build()
        .unwrap();

    rt.block_on(async {
        for i in 0..7 {
            println!("num {}", i);
            task::spawn(async move {
                // let res = core_affinity::set_for_current(core_id);
                // println!("{}", res);
                loop {
                    let mut sum: i32 = 0;
                    for i in 0..100000000 {
                        sum = sum.overflowing_add(i).0;
                    }
                    println!("sum {}", sum);
                    // std::hint::black_box(sum);
                }
            });
        }
    });
}
