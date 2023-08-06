# Mutex解决并发写文件乱序问题

在实际开发过程中，我们可能会遇到并发写文件的场景，如果处理不当很可能出现文件内容乱序问题。下面我们通过一个示例程序描述这一过程并给出解决该问题的方法。

```Rust
use std::{
    fs::{self, File, OpenOptions},
    io::{Write},
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};
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

    let mut set: JoinSet<()> = JoinSet::new();
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        loop {
            while set.len() >= max_tasks {
                set.join_next().await;
            }
            未做写互斥函数
            let mut file_ref = OpenOptions::new()
                .create(true)
                .write(true)
                .append(true)
                .open("/tmp/parallel")
                .unwrap();
            set.spawn(async move { write_line(&mut file_ref) });
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
```

代码不复杂，tokio 实现一个并发runtime，写文件函数是直接写时间戳，为了方便展示乱序所以写入两次换行。

输出的文本大概长这样

```
1691287258_979





1691287258_7931691287258_301

1691287258_7431691287258_603

1691287258_8941691287258_47






1691287258_895
1691287258_553

1691287258_950
1691287258_980


1691287258_48
1691287258_302

1691287258_896
1691287258_744




1691287258_6041691287258_554
```
很明显，写入并未达到预期，间隔并不平均，函数内部的执行步骤是乱序的。

我们把上面的程序改造一下

```rust
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

            let mut file = Arc::clone(&f);
            set.spawn(async move { write_line_mutex(&mut file).await });
        }
    });
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


```