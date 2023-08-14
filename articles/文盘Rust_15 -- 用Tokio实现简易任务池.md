# 文盘Rust -- 用Tokio实现简易任务池

Tokio 无疑是 Rust 世界中最优秀的异步Runtime实现。非阻塞的特性带来了优异的性能，但是在实际的开发中我们往往需要在某些情况下阻塞任务来实现某些功能。
我们看看下面的例子

```rust
fn main(){
        let max_task = 1;
        let rt = runtime::Builder::new_multi_thread()
            .worker_threads(max_task)
            
            .build()
            .unwrap();     

        rt.block_on(async {
            println!("tokio_multi_thread ");
            for i in 0..100 {
                println!("run {}", i);     
                tokio::spawn(async move {
                    println!("spawn {}", i);
                    thread::sleep(Duration::from_secs(2));
                });
            }
        });
    }
```

我们期待的运行结构是通过异步任务打印出99个 “spawn i"，但实际输出的结果大概这样

```shell
tokio_multi_thread
run 0
run 1
run 2
.......
run 16
spawn 0
run 17
......
run 99
spawn 1
spawn 2
......
spawn 29
......
spawn 58
spawn 59
```

59执行完后面就没有输出了，如果把max_task设置为2，情况会好一点，但是也没有执行完所有的异步操作，也就是说在资源不足的情况下，Tokio会抛弃某些任务，这不符合我们的预期。那么能不能再达到了某一阀值的情况下阻塞一下，不再给Tokio新的任务呢。这有点类似线程池，当达达最大线程数的时候阻塞后面的任务待有释放的线程后再继续。
我们看看下面的代码。

```rust
fn main(){
        let max_task = 2;
        let rt = runtime::Builder::new_multi_thread()
            .worker_threads(max_task)
            .enable_time()
            .build()
            .unwrap();     
        let mut set = JoinSet::new();
        rt.block_on(async {
            for i in 0..100 {
                println!("run {}", i);
                while set.len() >= max_task {
                    set.join_next().await;
                }
                set.spawn(async move {
                    sleep().await;
                    println!("spawn {}", i);
                });
            }
            while set.len() > 0 {
                set.join_next().await;
            }
        });
    }
```

我们使用JoinSet来管理派生出来的任务。set.join_next().await; 保证至少一个任务被执行完成。结合set的len，我们可以在任务达到上限时阻塞任务派生。当循环结束，可能还有未完成的任务，所以只要set.len()大于0就等待任务结束。

输出大概长这样

```rust
running 1 test
tokio_multi_thread
run 0
run 1
spawn 0
run 2
spawn 1
......
run 31
spawn 30
run 32
spawn 31
run 33
......
run 96
spawn 95
run 97
spawn 96
run 98
spawn 97
run 99
spawn 98
spawn 99
```

符合预期，代码不多，有兴趣的同学可以动手尝试一下。
