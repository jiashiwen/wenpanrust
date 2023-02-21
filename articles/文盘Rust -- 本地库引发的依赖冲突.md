
# 文盘Rust -- 本地库引发的依赖冲突

## 问题描述

clickhouse 的原生 rust 客户端目前比较好的有两个[clickhouse-rs](https://github.com/suharev7/clickhouse-rs) 和 [clickhouse.rs](https://github.com/loyd/clickhouse.rs) 。clickhouse-rs 是 tcp 连接；clickhouse.rs 是 http 连接。两个库在单独使用时没有任何问题，但是，在同一工程同时引用时会报错。

* Cargo.toml

  ```toml
  # clickhouse http
  clickhouse = {git = "https://github.com/loyd/clickhouse.rs", features =      ["test-util"]}
  
  # clickhouse tcp
  clickhouse-rs = { git = "https://github.com/suharev7/clickhouse-rs",     features = ["default"]}
  
  ```

* 报错如下
  
  ```shell
      Blocking waiting for file lock on package cache
      Updating git repository `https://github.com/suharev7/clickhouse-rs`
      Updating crates.io index
  error: failed to select a version for `clickhouse-rs-cityhash-sys`.
      ... required by package `clickhouse-rs v1.0.0-alpha.1 (https://github.  com/suharev7/clickhouse-rs#ecf28f46)`
      ... which satisfies git dependency `clickhouse-rs` of package   `conflict v0.1.0 (/Users/jiashiwen/rustproject/conflict)`
  versions that meet the requirements `^0.1.2` are: 0.1.2
  
  the package `clickhouse-rs-cityhash-sys` links to the native library   `clickhouse-rs`, but it conflicts with a previous package which links to   `clickhouse-rs` as well:
  package `clickhouse-rs-cityhash-sys v0.1.2`
      ... which satisfies dependency `clickhouse-rs-cityhash-sys = "^0.1.2"`   (locked to 0.1.2) of package `clickhouse v0.11.2 (https://github.com/  loyd/clickhouse.rs#4ba31e65)`
      ... which satisfies git dependency `clickhouse` (locked to 0.11.2) of   package `conflict v0.1.0 (/Users/jiashiwen/rustproject/conflict)`
  Only one package in the dependency graph may specify the same links value.   This helps ensure that only one copy of a native library is linked in the   final binary. Try to adjust your dependencies so that only one package   uses the links ='clickhouse-rs-cityhash-sys' value. For more information,   see https://doc.rust-lang.org/cargo/reference/resolver.html#links.
  
  failed to select a version for `clickhouse-rs-cityhash-sys` which could   resolve this conflict
  ```

错误描述还是很清楚的，clickhouse-rs-cityhash-sys 这个库冲突了。仔细看了一下两个库的源码，引用 clickhouse-rs-cityhash-sys 库的方式是不一样的。clickhouse.rs 是在其Cargo.toml 文件中使用最普遍的方式引用的

```toml
clickhouse-rs-cityhash-sys = { version = "0.1.2", optional = true }
```

clickhouse-rs 是通过本地方式引用的

```toml
[dependencies.clickhouse-rs-cityhash-sys]
path = "clickhouse-rs-cityhash-sys"
version = "0.1.2"
```

clickhouse-rs-cityhash-sys 的源码直接放在 clickhouse-rs 工程目录下面。

一开始是有个直观的想法，如果在一个工程中通过workspace 进行隔离，是不是会解决冲突问题呢？
于是，工程的目录结构从这样

```shell
.
├── Cargo.lock
├── Cargo.toml
└── src
    └── main.rs
```

改成了这样

```shell
.
├── Cargo.lock
├── Cargo.toml
├── ck_http
│   ├── Cargo.toml
│   └── src
├── ck_tcp
│   ├── Cargo.toml
│   └── src
└── src
    └── main.rs
```

新建了两个lib

```toml
cargo new ck_http --lib
cargo new ck_tcp --lib
```

在 workspace 中分别应用 clickhouse-rs 和 clickhouse.rs ,删除根下 Cargo.toml 文件中的依赖关系。
很可惜，workspace 没有解决问题，报错没有一点儿差别。

又仔细看了看报错，里面有这样一段

```
  the package `clickhouse-rs-cityhash-sys` links to the native library   `clickhouse-rs`, but it conflicts with a previous package which links to   `clickhouse-rs`
```

难道是 clickhouse-rs 这个名字冲突了？
直接把 clickhouse-rs 源码拉下来作为本地库来试试呢？
于是把 clickhouse-rs clone 到本地，稍稍修改一下ck_tcp workspace 的 Cargo.toml

```toml
clickhouse-rs = { path = "../../clickhouse-rs", features = ["default"]}
```

编译后冲突依旧存在。
翻翻 clickhouse-rs/clickhouse-rs-cityhash-sys/Cargo.toml，里面的一个配置很可疑

```toml
[package]
...
...
links = "clickhouse-rs"
```

把 links 随便改个名字比如：links = "ck-rs-cityhash-sys"，编译就通过了。

错误提示中这句话很重要

```
Only one package in the dependency graph may specify the same links value.
```

看了一下 links 字段的含义

```
The links field
The links field specifies the name of a native library that is being linked to. More information can be found in the links section of the build script guide.
```

links 指定了本地包被链接的名字，在这里引起了冲突，改掉本地包中的名字自然解决了冲突，在依赖图中保证唯一性很重要。

本文涉及代码github[仓库](https://github.com/jiashiwen/ck_dependency_conflict_sample)，有兴趣的同学可以亲自试一试

下期见。