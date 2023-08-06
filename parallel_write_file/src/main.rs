use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::sync::Mutex;
use tokio::task::JoinSet;

fn main() {
    println!("parallel write file!");
    let max_tasks = 200;
    let _ = fs::remove_file("/tmp/parallel");
    let file_ref = OpenOptions::new()
        .create(true)
        .write(true)
        .append(true)
        .open("/tmp/parallel")
        .unwrap();

    let f = Arc::new(Mutex::new(file_ref));

    let mut set: JoinSet<()> = JoinSet::new();
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        loop {
            while set.len() >= max_tasks {
                set.join_next().await;
            }
            // 未做写互斥函数
            // let mut file_ref = OpenOptions::new()
            //     .create(true)
            //     .write(true)
            //     .append(true)
            //     .open("/tmp/parallel")
            //     .unwrap();
            // set.spawn(async move { write_line(&mut file_ref) });

            let mut file = Arc::clone(&f);
            set.spawn(async move { write_line_mutex(&mut file).await });
        }
    });
}

fn write_line(file: &mut File) {
    for i in 0..1000 {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        let mut content = now.as_secs().to_string();
        content.push_str("_");
        content.push_str(&i.to_string());

        file.write_all(content.as_bytes()).unwrap();
        file.write_all("\n".as_bytes()).unwrap();
        file.write_all("\n".as_bytes()).unwrap();
    }
}

async fn write_line_mutex(mutex_file: &Arc<Mutex<File>>) {
    for i in 0..1000 {
        let mut f = mutex_file.lock().await;
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        let mut content = now.as_secs().to_string();
        content.push_str("_");
        content.push_str(&i.to_string());

        f.write_all(content.as_bytes()).unwrap();
        f.write_all("\n".as_bytes()).unwrap();
        f.write_all("\n".as_bytes()).unwrap();
    }
}
