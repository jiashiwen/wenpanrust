use tokio::runtime;

// pub fn main() {
//     let rt = runtime::Builder::new_multi_thread()
//         .enable_all()
//         .build()
//         .unwrap();

//     rt.block_on(async {
//         for i in 0..16 {
//             println!("num {}", i);
//             tokio::spawn(async move {
//                 loop {
//                     let mut sum: i32 = 0;
//                     for i in 0..100000000 {
//                         sum = sum.overflowing_add(i).0;
//                     }
//                     println!("sum {}", sum);
//                 }
//             });
//         }
//     });
// }
// pub fn main() {
//     let core_ids = core_affinity::get_core_ids().unwrap();
//     println!("core num {}", core_ids.len());
//     let core_id = core_ids[1];

//     let rt = runtime::Builder::new_multi_thread()
//         .on_thread_start(move || {
//             core_affinity::set_for_current(core_id.clone());
//         })
//         .enable_all()
//         .build()
//         .unwrap();

//     rt.block_on(async {
//         for i in 0..8 {
//             println!("num {}", i);
//             tokio::spawn(async move {
//                 // let res = core_affinity::set_for_current(core_id);
//                 // println!("{}", res);
//                 loop {
//                     let mut sum: i32 = 0;
//                     for i in 0..100000000 {
//                         sum = sum.overflowing_add(i).0;
//                     }
//                     println!("sum {}", sum);
//                     // std::hint::black_box(sum);
//                 }
//             });
//         }
//     });
// }

pub fn main() {
    let core_ids = core_affinity::get_core_ids().unwrap();
    println!("core num {}", core_ids.len());

    let rt = runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();

    let mut idx = 2;

    rt.block_on(async {
        for i in 0..8 {
            println!("num {}", i);
            let core_id = core_ids[idx];
            if idx.eq(&(core_ids.len() - 1)) {
                idx = 2;
            } else {
                idx += 1;
            }

            tokio::spawn(async move {
                let res = core_affinity::set_for_current(core_id);
                println!("{}", res);
                loop {
                    let mut sum: i32 = 0;
                    for i in 0..100000000 {
                        sum = sum.overflowing_add(i).0;
                    }
                    println!("sum {}", sum);
                    }
            });
        }
    });
}
