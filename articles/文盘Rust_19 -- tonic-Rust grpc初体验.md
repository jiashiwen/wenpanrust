# tonic-Rust grpc初体验

gRPC 是开发中常用的开源高性能远程过程调用（RPC）框架，tonic 是基于 HTTP/2 的 gRPC 实现，专注于高性能、互操作性和灵活性。该库的创建是为了对 async/await 提供一流的支持，并充当用 Rust 编写的生产系统的核心构建块。今天我们聊聊通过使用tonic 调用grpc的的具体过程。

## 工程规划

rpc程序一般包含server端和client端，为了方便我们把两个程序打包到一个工程里面
新建tonic_sample工程
```toml
cargo new tonic_sample
```

Cargo.toml 如下

```toml
[package]
name = "tonic_sample"
version = "0.1.0"
edition = "2021"

[[bin]] # Bin to run the gRPC server
name = "stream-server"
path = "src/stream_server.rs"

[[bin]] # Bin to run the gRPC client
name = "stream-client"
path = "src/stream_client.rs"


[dependencies]
tokio.workspace = true
tonic = "0.9"
tonic-reflection = "0.9.2"
prost = "0.11"
tokio-stream = "0.1"
async-stream = "0.2"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
rand = "0.7"
h2 = { version = "0.3" }
anyhow = "1.0.75"
futures-util = "0.3.28"

[build-dependencies]
tonic-build = "0.9"
```

tonic 的示例代码还是比较齐全的，本次我们参考 tonic 的 [streaming example](https://github.com/hyperium/tonic/tree/master/examples/src/streaming)。

首先编写 proto 文件，用来描述报文。
proto/echo.proto

```proto
syntax = "proto3";

package stream;

// EchoRequest is the request for echo.
message EchoRequest { string message = 1; }

// EchoResponse is the response for echo.
message EchoResponse { string message = 1; }

// Echo is the echo service.
service Echo {
  // UnaryEcho is unary echo.
  rpc UnaryEcho(EchoRequest) returns (EchoResponse) {}
  // ServerStreamingEcho is server side streaming.
  rpc ServerStreamingEcho(EchoRequest) returns (stream EchoResponse) {}
  // ClientStreamingEcho is client side streaming.
  rpc ClientStreamingEcho(stream EchoRequest) returns (EchoResponse) {}
  // BidirectionalStreamingEcho is bidi streaming.
  rpc BidirectionalStreamingEcho(stream EchoRequest)
      returns (stream EchoResponse) {}
}
```

文件并不复杂，只有两个 message 一个请求一个返回，之所以选择这个示例是因为该示例包含了rpc中的流式处理，包扩了server 流、client 流以及双向流的操作。
编辑build.rs 文件

```rust
use std::{env, path::PathBuf};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::compile_protos("proto/echo.proto")?;
    Ok(())
}
```

该文件用来通过 tonic-build  生成 grpc 的 rust 基础代码

完成上述工作后就可以构建 server 和 client 代码了

stream_server.rs

```rust 
pub mod pb {
    tonic::include_proto!("stream");
}

use anyhow::Result;
use futures_util::FutureExt;
use pb::{EchoRequest, EchoResponse};
use std::{
    error::Error,
    io::ErrorKind,
    net::{SocketAddr, ToSocketAddrs},
    pin::Pin,
    thread,
    time::Duration,
};
use tokio::{
    net::TcpListener,
    sync::{
        mpsc,
        oneshot::{self, Receiver, Sender},
        Mutex,
    },
    task::{self, JoinHandle},
};
use tokio_stream::{
    wrappers::{ReceiverStream, TcpListenerStream},
    Stream, StreamExt,
};
use tonic::{transport::Server, Request, Response, Status, Streaming};
type EchoResult<T> = Result<Response<T>, Status>;
type ResponseStream = Pin<Box<dyn Stream<Item = Result<EchoResponse, Status>> + Send>>;

fn match_for_io_error(err_status: &Status) -> Option<&std::io::Error> {
    let mut err: &(dyn Error + 'static) = err_status;

    loop {
        if let Some(io_err) = err.downcast_ref::<std::io::Error>() {
            return Some(io_err);
        }

        // h2::Error do not expose std::io::Error with `source()`
        // https://github.com/hyperium/h2/pull/462
        if let Some(h2_err) = err.downcast_ref::<h2::Error>() {
            if let Some(io_err) = h2_err.get_io() {
                return Some(io_err);
            }
        }

        err = match err.source() {
            Some(err) => err,
            None => return None,
        };
    }
}

#[derive(Debug)]
pub struct EchoServer {}

#[tonic::async_trait]
impl pb::echo_server::Echo for EchoServer {
    async fn unary_echo(&self, req: Request<EchoRequest>) -> EchoResult<EchoResponse> {
        let req_str = req.into_inner().message;

        let response = EchoResponse { message: req_str };
        Ok(Response::new(response))
    }

    type ServerStreamingEchoStream = ResponseStream;

    async fn server_streaming_echo(
        &self,
        req: Request<EchoRequest>,
    ) -> EchoResult<Self::ServerStreamingEchoStream> {
        println!("EchoServer::server_streaming_echo");
        println!("\tclient connected from: {:?}", req.remote_addr());

        // creating infinite stream with requested message
        let repeat = std::iter::repeat(EchoResponse {
            message: req.into_inner().message,
        });
        let mut stream = Box::pin(tokio_stream::iter(repeat).throttle(Duration::from_millis(200)));

        let (tx, rx) = mpsc::channel(128);
        tokio::spawn(async move {
            while let Some(item) = stream.next().await {
                match tx.send(Result::<_, Status>::Ok(item)).await {
                    Ok(_) => {
                        // item (server response) was queued to be send to client
                    }
                    Err(_item) => {
                        // output_stream was build from rx and both are dropped
                        break;
                    }
                }
            }
            println!("\tclient disconnected");
        });

        let output_stream = ReceiverStream::new(rx);
        Ok(Response::new(
            Box::pin(output_stream) as Self::ServerStreamingEchoStream
        ))
    }

    async fn client_streaming_echo(
        &self,
        _: Request<Streaming<EchoRequest>>,
    ) -> EchoResult<EchoResponse> {
        Err(Status::unimplemented("not implemented"))
    }

    type BidirectionalStreamingEchoStream = ResponseStream;

    async fn bidirectional_streaming_echo(
        &self,
        req: Request<Streaming<EchoRequest>>,
    ) -> EchoResult<Self::BidirectionalStreamingEchoStream> {
        println!("EchoServer::bidirectional_streaming_echo");

        let mut in_stream = req.into_inner();
        let (tx, rx) = mpsc::channel(128);

        tokio::spawn(async move {
            while let Some(result) = in_stream.next().await {
                match result {
                    Ok(v) => tx
                        .send(Ok(EchoResponse { message: v.message }))
                        .await
                        .expect("working rx"),
                    Err(err) => {
                        if let Some(io_err) = match_for_io_error(&err) {
                            if io_err.kind() == ErrorKind::BrokenPipe {
                                eprintln!("\tclient disconnected: broken pipe");
                                break;
                            }
                        }

                        match tx.send(Err(err)).await {
                            Ok(_) => (),
                            Err(_err) => break, // response was droped
                        }
                    }
                }
            }
            println!("\tstream ended");
        });

        // echo just write the same data that was received
        let out_stream = ReceiverStream::new(rx);

        Ok(Response::new(
            Box::pin(out_stream) as Self::BidirectionalStreamingEchoStream
        ))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 基础server
    let server = EchoServer {};
    Server::builder()
        .add_service(pb::echo_server::EchoServer::new(server))
        .serve("0.0.0.0:50051".to_socket_addrs().unwrap().next().unwrap())
        .await
        .unwrap();
    Ok(())
}

```

server 端的代码还是比较清晰的，首先通过 tonic::include_proto! 宏引入grpc定义，参数是 proto 文件中定义的 package 。我们重点说说 server_streaming_echo function 。这个function 的处理流程明白了，其他的流式处理大同小异。首先 通过std::iter::repeat function 定义一个迭代器；然后构建 tokio_stream 在本示例中 每 200毫秒产生一个 repeat；最后构建一个 channel ，tx 用来发送从stream中获取的内容太，rx 封装到response 中返回。
最后 main 函数 拉起服务。

client 代码如下

```rust
pub mod pb {
    tonic::include_proto!("stream");
}

use std::time::Duration;
use tokio_stream::{Stream, StreamExt};
use tonic::transport::Channel;

use pb::{echo_client::EchoClient, EchoRequest};

fn echo_requests_iter() -> impl Stream<Item = EchoRequest> {
    tokio_stream::iter(1..usize::MAX).map(|i| EchoRequest {
        message: format!("msg {:02}", i),
    })
}

async fn unary_echo(client: &mut EchoClient<Channel>, num: usize) {
    for i in 0..num {
        let req = tonic::Request::new(EchoRequest {
            message: "msg".to_string() + &i.to_string(),
        });
        let resp = client.unary_echo(req).await.unwrap();
        println!("resp:{}", resp.into_inner().message);
    }
}

async fn streaming_echo(client: &mut EchoClient<Channel>, num: usize) {
    let stream = client
        .server_streaming_echo(EchoRequest {
            message: "foo".into(),
        })
        .await
        .unwrap()
        .into_inner();

    // stream is infinite - take just 5 elements and then disconnect
    let mut stream = stream.take(num);
    while let Some(item) = stream.next().await {
        println!("\treceived: {}", item.unwrap().message);
    }
    // stream is droped here and the disconnect info is send to server
}

async fn bidirectional_streaming_echo(client: &mut EchoClient<Channel>, num: usize) {
    let in_stream = echo_requests_iter().take(num);

    let response = client
        .bidirectional_streaming_echo(in_stream)
        .await
        .unwrap();

    let mut resp_stream = response.into_inner();

    while let Some(received) = resp_stream.next().await {
        let received = received.unwrap();
        println!("\treceived message: `{}`", received.message);
    }
}

async fn bidirectional_streaming_echo_throttle(client: &mut EchoClient<Channel>, dur: Duration) {
    let in_stream = echo_requests_iter().throttle(dur);

    let response = client
        .bidirectional_streaming_echo(in_stream)
        .await
        .unwrap();

    let mut resp_stream = response.into_inner();

    while let Some(received) = resp_stream.next().await {
        let received = received.unwrap();
        println!("\treceived message: `{}`", received.message);
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = EchoClient::connect("http://127.0.0.1:50051").await.unwrap();
    println!("Unary echo:");
    unary_echo(&mut client, 10).await;
    tokio::time::sleep(Duration::from_secs(1)).await;

    println!("Streaming echo:");
    streaming_echo(&mut client, 5).await;
    tokio::time::sleep(Duration::from_secs(1)).await; //do not mess server println functions

    // Echo stream that sends 17 requests then graceful end that connection
    println!("\r\nBidirectional stream echo:");
    bidirectional_streaming_echo(&mut client, 17).await;

    // Echo stream that sends up to `usize::MAX` requests. One request each 2s.
    // Exiting client with CTRL+C demonstrate how to distinguish broken pipe from
    // graceful client disconnection (above example) on the server side.
    println!("\r\nBidirectional stream echo (kill client with CTLR+C):");
    bidirectional_streaming_echo_throttle(&mut client, Duration::from_secs(2)).await;

    Ok(())
}

```

测试一下,分别运行 server 和 client

```shell
cargo run --bin stream-server
cargo run --bin stream-client
```

在开发中，我们通常不会再 client 和 server都开发好的情况下才开始测试。通常在开发server 端的时候采用 grpcurl 工具进行测试工作

```shell
grpcurl -import-path ./proto -proto echo.proto list
grpcurl -import-path ./proto -proto  echo.proto describe stream.Echo
grpcurl -plaintext -import-path ./proto -proto  echo.proto -d '{"message":"1234"}' 127.0.0.1:50051 stream.Echo/UnaryEcho
```

此时，如果我们不指定 -import-path 参数，执行如下命令

```shell
grpcurl -plaintext 127.0.0.1:50051 list
```

会出现如下报错信息

```shell
Failed to list services: server does not support the reflection API
```

## 让服务端程序支持 reflection API

首先改造build.rs

```rust
use std::{env, path::PathBuf};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    tonic_build::configure()
        .file_descriptor_set_path(out_dir.join("stream_descriptor.bin"))
        .compile(&["proto/echo.proto"], &["proto"])
        .unwrap();
    Ok(())
}
```

file_descriptor_set_path  生成一个文件，其中包含为协议缓冲模块编码的 `prost_types::FileDescriptorSet` 文件。这是实现 gRPC 服务器反射所必需的。

接下来改造一下 stream-server.rs，涉及两处更改。

新增 STREAM_DESCRIPTOR_SET 常量

```rust
pub mod pb {
    tonic::include_proto!("stream");
    pub const STREAM_DESCRIPTOR_SET: &[u8] =
        tonic::include_file_descriptor_set!("stream_descriptor");
}
```

修改main函数

```rust
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 基础server
    // let server = EchoServer {};
    // Server::builder()
    //     .add_service(pb::echo_server::EchoServer::new(server))
    //     .serve("0.0.0.0:50051".to_socket_addrs().unwrap().next().unwrap())
    //     .await
    //     .unwrap();

    // tonic_reflection 
    let service = tonic_reflection::server::Builder::configure()
        .register_encoded_file_descriptor_set(pb::STREAM_DESCRIPTOR_SET)
        .with_service_name("stream.Echo")
        .build()
        .unwrap();

    let addr = "0.0.0.0:50051".parse().unwrap();

    let server = EchoServer {};

    Server::builder()
        .add_service(service)
        .add_service(pb::echo_server::EchoServer::new(server))
        .serve(addr)
        .await?;
    Ok(())
}
```
register_encoded_file_descriptor_set 将包含编码的 `prost_types::FileDescriptorSet` 的 byte slice 注册到
 gRPC Reflection 服务生成器注册。

再次测试

```shell
grpcurl -plaintext 127.0.0.1:50051 list
grpcurl -plaintext 127.0.0.1:50051 describe stream.Echo
```

返回正确结果。


[以上完整代码地址](https://github.com/jiashiwen/wenpanrust/tree/main/tonic_sample)