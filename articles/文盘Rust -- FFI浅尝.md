# 文盘Rust --  FFI 浅尝

rust FFI 是rust与其他语言互调的桥梁，通过FFI rust 可以有效继承 C 语言的历史资产。本期通过几个例子来聊聊rust与C 语言交互的具体步骤。

## 场景一 调用C代码

创建工程

```shell
cargo new --bin ffi_sample
```

Cargo.toml 配置

```toml
[package]
name = "ffi_sample"
version = "0.1.0"
edition = "2021"
build = "build.rs"

# See more keys and their definitions at https://doc.rust-lang.org/cargo/reference/manifest.html

[build-dependencies]
cc = "1.0.79"

[dependencies]
libc = "0.2.146"
libloading = "0.8.0"
```

编写一个简单的c程序sample.c

```C
int add(int a,int b){
    return a+b;
}
```

main.rs

```rust
use std::os::raw::c_int;


#[link(name = "sample")]
extern "C" {
    fn add(a: c_int, b: c_int) -> c_int;
}

fn main() {
    let r = unsafe { add(2, 18) };
    println!("{:?}", r);
}
```

build.rs

```rust

fn main() {
    cc::Build::new().file("sample.c").compile("sample");
}

```

代码目录树

```shell
.
├── Cargo.lock
├── Cargo.toml
├── build.rs
├── sample.c
└── src
    └── main.rs
```

```shell
cargo run
```

## 场景二  使用bindgen 通过头文件绑定c语言动态链接库

修改Cargo.toml,新增bindgen依赖

```toml
[package]
name = "ffi_sample"
version = "0.1.0"
edition = "2021"
build = "build.rs"

# See more keys and their definitions at https://doc.rust-lang.org/cargo/reference/manifest.html

[build-dependencies]
cc = "1.0.79"
bindgen = "0.65.1"

[dependencies]
libc = "0.2.146"
libloading = "0.8.0"
```

新增 sample.h 头文件

```C
#ifndef ADD_H
#define ADD_H

int add(int a, int b);

#endif
```

新增 wrapper.h 头文件
wrapper.h 文件将包括所有各种头文件，这些头文件包含我们想要绑定的结构和函数的声明

```C
#include "sample.h";
```

改写build.rs
编译 sample.c 生成动态链接库sample.so;通过bindgen生成rust binding c 的代码并输出到 bindings 目录

```rust
use std::path::PathBuf;

fn main() {
    // 参考cc 文档
    println!("cargo:rerun-if-changed=sample.c");
    cc::Build::new()
        .file("sample.c")
        .shared_flag(true)
        .compile("sample.so");
    // 参考 https://doc.rust-lang.org/cargo/reference/build-scripts.html
    println!("cargo:rustc-link-lib=sample.so");
    println!("cargo:rerun-if-changed=sample.h");
    let bindings = bindgen::Builder::default()
        .header("wrapper.h")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .generate()
        .expect("Unable to generate bindings");

    let out_path = PathBuf::from("bindings");
    bindings
        .write_to_file(out_path.join("sample_bindings.rs"))
        .expect("Couldn't write bindings!");
}
```

修改main.rs
include 宏引入sample 动态链接库的binding。以前我们自己手写的C函数绑定就不需要了，看看bindings/sample_bindings.rs 的内容与我们手写的函数绑定是等效的

```Rust
include!("../bindings/sample_bindings.rs");

// #[link(name = "sample")]
// extern "C" {
//     fn add(a: c_int, b: c_int) -> c_int;
// }

fn main() {
    let r = unsafe { add(2, 18) };
    println!("{:?}", r);
}

```

代码目录树

```shell
.
├── Cargo.lock
├── Cargo.toml
├── bindings
│   └── sample_bindings.rs
├── build.rs
├── sample.c
├── sample.h
├── src
│   └── main.rs
└── wrapper.h
```

ffi_sample 工程的完整代码[位置](https://github.com/jiashiwen/wenpanrust/tree/main/ffi_sample),读者可以clone https://github.com/jiashiwen/wenpanrust，直接运行

```shell
cargo run -p ffi_sample
```

即可

## 场景三 封装一个c编写的库

secp256k1是一个椭圆曲线计算的 clib，这玩意儿在密码学和隐私计算方面的常用算法，下面我们从工程方面看看封装secp256k1如何操作

```shell
cargo new --lib wrapper_secp256k1
```

cargo.toml

```toml
[package]
name = "wrapper_secp256k1"
version = "0.1.0"
edition = "2021"

# See more keys and their definitions at https://doc.rust-lang.org/cargo/reference/manifest.html
[build-dependencies]
cc = "1.0.79"
bindgen = "0.65.1"

[dependencies]
```

git 引入 submodule

```shell
cd wrapper_secp256k1
git submodule add https://github.com/bitcoin-core/secp256k1  wrapper_secp256k1/secp256k1_sys/secp256k1_sys
```

工程下新建bindings目录用来存放绑定文件，该目录与src平级

wrapper.h

```c
#include "secp256k1_sys/secp256k1/include/secp256k1.h"
```

build.rs

```rust
use std::path::PathBuf;

fn main() {
    println!("cargo:rustc-link-lib=secp256k1");
    println!("cargo:rerun-if-changed=wrapper.h");
    let bindings = bindgen::Builder::default()
        .header("wrapper.h")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .generate()
        .expect("Unable to generate bindings");

    let out_path = PathBuf::from("bindings");
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}
```


cargo build 通过

编写测试 lib.rs

```rust
include!("../bindings/secp256k1.rs");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_pubkey() {
        // secp256k1返回公钥
        let mut pubkey: secp256k1_pubkey = secp256k1_pubkey { data: [0; 64] };
        let prikey: u8 = 1;

        unsafe {
            let context = secp256k1_context_create(SECP256K1_CONTEXT_SIGN);
            assert!(!context.is_null());
            let ret = secp256k1_ec_pubkey_create(&*context, &mut pubkey, &prikey);
            assert_eq!(ret, 1);
        }
    }
}
```

运行测试 cargo test 报错

```shell
warning: `wrapper_secp256k1` (lib) generated 5 warnings
error: linking with `cc` failed: exit status: 1
  |
  = note: LC_ALL="C" PATH="/usr/local/Cellar/rust/1.70.0/lib/rustlib/x86_64-apple-darwin/bin:/Users/jiashiwen/local/Cellar/rust/1.70.0/lib/rustlib/x86_64-apple-darwin/lib/libobject-6d1da0e5d7930106.rlib" "/usr/local/Cellar/rust/1.70.0/lib/rustlib/x86_64-apple-darwin/lib/libmemchr-d6d74858e37ed726.rlib" "/usr/local/Cellar/rust/1.70.0/lib/rustlib/x86_64-apple-darwin/lib/libaddr2line-d75e66c6c1b76fdd.rlib" "/usr/local/Cellar/rust/1.70.0/lib/rustlib/x86_64-apple-darwin/lib/libgimli-546ea342344e3761.rlib" "/usr/local/Cellar/rust/1.70.0/lib/rustlib/x86_64-apple-darwin/lib/librustc_demangle-8ad10e36ca13f067.rlib" "/usr/local/Cellar/rust/1.70.0/lib/rustlib/x86_64-apple-darwin/lib/libstd_detect-0543b8486ac00cf6.rlib" "/usr/local/Cellar/rust/1.70.0/lib/rustlib/x86_64-apple-darwin/lib/libhashbrown-7f0d42017ce08763.rlib" "/usr/local/Cellar/rust/1.70.0/lib/rustlib/x86_64-apple-darwin/lib/libminiz_oxide-65e6b9c4725e3b7f.rlib" "/usr/local/Cellar/rust/1.70.0/lib/rustlib/x86_64-apple-darwin/lib/libadler-131157f72607aea7.rlib" "/usr/local/Cellar/rust/1.70.0/lib/rustlib/x86_64-apple-darwin/lib/librustc_std_workspace_alloc-f7d15060b16c135d.rlib" "/usr/local/Cellar/rust/1.70.0/lib/rustlib/x86_64-apple-darwin/lib/libunwind-a52bfac5ae872be2.rlib" "/usr/local/Cellar/rust/1.70.0/lib/rustlib/x86_64-apple-darwin/lib/libcfg_if-1762d9ac100ea3e7.rlib" "/usr/local/Cellar/rust/1.70.0/lib/rustlib/x86_64-apple-darwin/lib/liblibc-f8e0e4708f61f3f4.rlib" "/usr/local/Cellar/rust/1.70.0/lib/rustlib/x86_64-apple-darwin/lib/liballoc-af9a608dd9cb26b2.rlib" "/usr/local/Cellar/rust/1.70.0/lib/rustlib/x86_64-apple-darwin/lib/librustc_std_workspace_core-9777023438fd3d6a.rlib" "/usr/local/Cellar/rust/1.70.0/lib/rustlib/x86_64-apple-darwin/lib/libcore-83ca6d61eb70e9b8.rlib" "/usr/local/Cellar/rust/1.70.0/lib/rustlib/x86_64-apple-darwin/lib/libcompiler_builtins-ea2ca6e1df0449b8.rlib" "-lSystem" "-lc" "-lm" "-L" "/usr/local/Cellar/rust/1.70.0/lib/rustlib/x86_64-apple-darwin/lib" "-o" "/Users/jiashiwen/rustproject/wrapper_secp256k1/target/debug/deps/wrapper_secp256k1-4bf30c62ecfdf2a7" "-Wl,-dead_strip" "-nodefaultlibs"
  = note: ld: library not found for -lsecp256k1
          clang: error: linker command failed with exit code 1 (use -v to see invocation)


warning: `wrapper_secp256k1` (lib test) generated 5 warnings (5 duplicates)
error: could not compile `wrapper_secp256k1` (lib test) due to previous error; 5 warnings emitted
```

报错显示找不到编译 secp256k1 相对应的库。

手动编译secp256k1

```shell
cd secp256k1_sys
./autogen.sh
./configure
make
make install
```

编译完成后，测试通过

其实 secp256k1 有对应的 [rust wrapper](https://github.com/rust-bitcoin/rust-secp256k1),我们这里只是展示一下封装的过程。

wrapper_secp256k1 工程的完整代码[位置](https://github.com/jiashiwen/wenpanrust/tree/main/wrapper_secp256k1),有兴趣的朋友可以clone https://github.com/jiashiwen/wenpanrust。通过一下操作查看运行结果：

* clone 项目
  
  ```shell
  git clone https://github.com/jiashiwen/wenpanrust
  cd wenpanrust
  ```

* update submodule
  
  ```shell
  git submodule update
  ```
 
* 编译 secp256k1
  
  ```shell
  cd wrapper_secp256k1/secp256k1_sys 
  ./autogen.sh
  ./configure
  make
  make install  
  ``` 

* run test
  ```
  cargo test -p 
  ```

参考资料

[Rust FFI (C vs Rust)学习杂记.pdf](https://github.com/yujinliang/my_writing/blob/master/Rust%20FFI%20(C%20vs%20Rust)%E5%AD%A6%E4%B9%A0%E6%9D%82%E8%AE%B0.pdf)  
[bindgen官方文档](https://rust-lang.github.io/rust-bindgen/introduction.html)
[Rust FFI 编程 - bindgen 使用示例](https://rustcc.cn/article?id=9219a366-84d3-49c8-b957-dfbade1257fc)
