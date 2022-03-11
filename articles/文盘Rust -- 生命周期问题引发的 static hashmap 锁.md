# 文盘Rust -- 生命周期问题引发的 static hashmap 锁

好久没在tug灌水了。最近为了能更好的熟悉tikv入坑Rust。说起这门语言，最早是在2019年DevCon上东旭带货。其实19年下半年陆续看过一些入门书，后来忙别的事儿就放下了。2021年二刷rust，书又看了一遍，看完了手就痒，想写点儿东西。练手就从cli开整。
2021年上半年，撸了个rust cli开发的框架，基本上把交互模式，子命令提示这些cli该有的常用功能做进去了。项目地址：https://github.com/jiashiwen/interactcli-rs。
春节以前看到[axum](https://github.com/tokio-rs/axum)已经0.4.x了，于是想看看能不能用rust做个服务端的框架。
春节后开始动手，在做的过程中会碰到各种有趣的问题。于是记下来想和社区的小伙伴一起分享。社区里的小伙伴大部分是DBA和运维同学，如果想进一步了解更底层的东西，代码入手是个好路数。
我个人认为想看懂代码先要写好代码，起码了解开发的基本路数和工程的一般组织模式。但好多同学的主要工作并不是专职开发，所以也就没有机会下探研发技术。代码这个事儿光看书是不管用的。了解一门语言最好的方式是使用它。
那么，问题来了非研发人员如何熟悉语言呢？咏春拳里有句拳谚：”无师无对手，桩与镜中求“。解释两句，就是在没有师兄弟练习的情况下，对着镜子和木人桩练习。在这里我觉得所谓桩有两层含义，一个是木人桩，就是练习的工具，一个是”站桩“，传统武术训练基本功的方法。其实在实际的工作中DBA和运维同学会有很多场景需要编程，比如做一些运维方面的统计工作；分析问题时需要拿到某些数据。如果追求简单用Python的话可能对于其他语言就没有涉猎了。如果结合你运维数据库的原生开发语言，假以时日慢慢就能看懂相关的底层逻辑了。我个人有个观点，产品研发的原生语言是了解产品底层最好的入口。
后面如果在Rust的开发过程中有其他问题，我本人会把问题结合实际也写到这个系列里，也希望社区里对Rust感兴趣的小伙伴一起来”盘Rust“。 言归正传，说说这次在玩儿Rust时遇到的问题吧。

在 Rust 开发过程中，我们经常需要全局变量作为公共数据的存放位置。通常做法是利用 lazy_static/onecell 和 mux/rwlock 生成一个静态的 collection。

代码长这样

```Rust
use std::collections::HashMap;
use std::sync::RwLock;

lazy_static::lazy_static! {
    static ref GLOBAL_MAP: RwLock<HashMap<String,String>> = RwLock::new({
        let map = HashMap::new();
        map
    });
}
```

基本的数据存取这样实现

```Rust
use std::collections::HashMap;
use std::sync::RwLock;

lazy_static::lazy_static! {
    static ref GLOBAL_MAP: RwLock<HashMap<String,String>> = RwLock::new({
        let map = HashMap::new();
        map
    });
}

fn main() {
    for i in 0..3 {
        insert_global_map(i.to_string(), i.to_string())
    }
    print_global_map();
    println!("finished!");
}

fn insert_global_map(k: String, v: String) {
    let mut gpw = GLOBAL_MAP.write().unwrap();
    gpw.insert(k, v);
}

fn print_global_map() {
    let gpr = GLOBAL_MAP.read().unwrap();
    for pair in gpr.iter() {
        println!("{:?}", pair);
    }
}
```

insert_global_map函数用来向GLOBAL_MAP插入数据，print_global_map()用来读取数据，上面程序的运行结果如下

```shell
("0", "0")
("1", "1")
("2", "2")
```

下面我们来实现一个比较复杂一点儿的需求，从 GLOBAL_MAP 里取一个数，如果存在后面进行删除操作,直觉告诉我们代码似乎应该这样写

```Rust
use std::collections::HashMap;
use std::sync::RwLock;

lazy_static::lazy_static! {
    static ref GLOBAL_MAP: RwLock<HashMap<String,String>> = RwLock::new({
        let map = HashMap::new();
        map
    });
}

fn main() {
    for i in 0..3 {
        insert_global_map(i.to_string(), i.to_string())
    }
    print_global_map();
    get_and_remove(1.to_string());
    println!("finished!");
}

fn insert_global_map(k: String, v: String) {
    let mut gpw = GLOBAL_MAP.write().unwrap();
    gpw.insert(k, v);
}

fn print_global_map() {
    let gpr = GLOBAL_MAP.read().unwrap();
    for pair in gpr.iter() {
        println!("{:?}", pair);
    }
}

fn get_and_remove(k: String) {
    println!("execute get_and_remove");
    let gpr = GLOBAL_MAP.read().unwrap();
    let v = gpr.get(&*k.clone());
    let mut gpw = GLOBAL_MAP.write().unwrap();
    gpw.remove(&*k.clone());
}

```

上面这段代码输出长这样

```shell
("0", "0")
("1", "1")
("2", "2")
execute get_and_remove

```

代码没有结束，而是hang在了get_and_remove函数。 为啥会出现这样的情况呢？这也许与生命周期有关。gpr和gpw 这两个返回值分别为 RwLockReadGuard 和 RwLockWriteGuard，查看这两个
struct 发现确实可能引起死锁

```Rust
must_not_suspend = "holding a RwLockWriteGuard across suspend \
                    points can cause deadlocks, delays, \
                    and cause Future's to not implement `Send`"
```

问题找到了就可以着手解决办法了，既然是与rust的生命周期有关，那是不是可以把读和写分别放在两个不同的生命周期里呢，于是对代码进行改写

```Rust
use std::collections::HashMap;
use std::sync::RwLock;

lazy_static::lazy_static! {
    static ref GLOBAL_MAP: RwLock<HashMap<String,String>> = RwLock::new({
        let map = HashMap::new();
        map
    });
}

fn main() {
    for i in 0..3 {
        insert_global_map(i.to_string(), i.to_string())
    }
    print_global_map();
    get_and_remove(1);
    println!("finished!");
}

fn insert_global_map(k: String, v: String) {
    let mut gpw = GLOBAL_MAP.write().unwrap();
    gpw.insert(k, v);
}

fn print_global_map() {
    let gpr = GLOBAL_MAP.read().unwrap();
    for pair in gpr.iter() {
        println!("{:?}", pair);
    }
}

fn get_and_remove_deadlock(k: String) {
    println!("execute get_and_remove");
    let gpr = GLOBAL_MAP.read().unwrap();
    let _v = gpr.get(&*k.clone());
    let mut gpw = GLOBAL_MAP.write().unwrap();
    gpw.remove(&*k.clone());
}

fn get_and_remove(k: i32) {
    let v = {
        let gpr = GLOBAL_MAP.read().unwrap();
        let v = gpr.get(&*k.to_string().clone());
        match v {
            None => Err(anyhow!("")),
            Some(pair) => Ok(pair.to_string().clone()),
        }
    };
    let vstr = v.unwrap();
    println!("get value is {:?}", vstr.clone());
    let mut gpw = GLOBAL_MAP.write().unwrap();
    gpw.remove(&*vstr);
}

```

正确输出

```shell
("1", "1")
("0", "0")
("2", "2")
get value is "1"
("0", "0")
("2", "2")
finished!
```

Rust的生命周期是个很有意思的概念，从认识到理解确实有个过程。

[源码地址](https://github.com/jiashiwen/wenpanrust/blob/main/examples/static_deadlock.rs)

感谢社区小姐姐@YY社区小帮手的图片支持。