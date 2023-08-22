pub mod pb {
    tonic::include_proto!("stream");
    pub const STREAM_DESCRIPTOR_SET: &[u8] =
        tonic::include_file_descriptor_set!("stream_descriptor");
}

// use h2::Error as h2_Error;
use anyhow::Result;

use futures_util::FutureExt;
use pb::{EchoRequest, EchoResponse};
use std::{error::Error, io::ErrorKind, net::SocketAddr, pin::Pin, thread, time::Duration};
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
pub struct StreamEchoServer {
    shutdown_tx: Mutex<Option<Sender<()>>>,
    serve_state: Mutex<Option<Receiver<Result<()>>>>,
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

        // spawn and channel are required if you want handle "disconnect" functionality
        // the `out_stream` will not be polled after client disconnect
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

        // this spawn here is required if you want to handle connection error.
        // If we just map `in_stream` and write it back as `out_stream` the `out_stream`
        // will be drooped when connection error occurs and error will never be propagated
        // to mapped version of `in_stream`.
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
                                // here you can handle special case when client
                                // disconnected in unexpected way
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

impl StreamEchoServer {
    pub async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn start(
        &self,
        addr: SocketAddr,
    ) -> Result<JoinHandle<Result<(), Result<(), anyhow::Error>>>> {
        let (tx, rx) = oneshot::channel();
        let (listener, addr) = {
            let mut shutdown_tx = self.shutdown_tx.lock().await;
            // ensure!(
            //     shutdown_tx.is_none(),
            //     AlreadyStartedSnafu { server: "gRPC" }
            // );
            // if !shutdown_tx.is_none() {
            //     panic!("server already started");
            // }

            let listener = TcpListener::bind(addr).await.unwrap();
            let addr = listener.local_addr().unwrap();
            println!("gRPC server is bound to {}", addr);

            *shutdown_tx = Some(tx);

            (listener, addr)
        };

        let reflection_service = tonic_reflection::server::Builder::configure()
            .register_encoded_file_descriptor_set(pb::STREAM_DESCRIPTOR_SET)
            .build()
            .unwrap();

        let mut builder = tonic::transport::Server::builder();

        let builder = builder
            .add_service(reflection_service)
            .add_service(pb::echo_server::EchoServer::new(EchoServer {}));

        let (serve_state_tx, serve_state_rx) = oneshot::channel();
        let mut serve_state = self.serve_state.lock().await;
        *serve_state = Some(serve_state_rx);

        // let _handle = common_runtime::spawn_bg(async move {
        //     let result = builder
        //         .serve_with_incoming_shutdown(TcpListenerStream::new(listener), rx.map(drop))
        //         .await
        //         .unwrap();
        //     serve_state_tx.send(result)
        // });

        let handle = task::spawn(async move {
            let result = builder
                .serve_with_incoming_shutdown(TcpListenerStream::new(listener), rx.map(drop))
                .await
                .map_err(|e| anyhow::Error::from(e));
            println!("{:?}", result);

            serve_state_tx.send(result)
        });

        Ok(handle)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 基础server
    // let server = EchoServer {};
    // Server::builder()
    //     .add_service(pb::echo_server::EchoServer::new(server))
    //     .serve("[::1]:50051".to_socket_addrs().unwrap().next().unwrap())
    //     .await
    //     .unwrap();

    // tonic_reflection 简化注册
    // let service = tonic_reflection::server::Builder::configure()
    //     .register_encoded_file_descriptor_set(pb::STREAM_DESCRIPTOR_SET)
    //     .with_service_name("stream.Echo")
    //     .build()
    //     .unwrap();

    // let addr = "[::1]:50051".parse().unwrap();

    // let server = EchoServer {};

    // Server::builder()
    //     .add_service(service)
    //     .add_service(pb::echo_server::EchoServer::new(server))
    //     .serve(addr)
    //     .await?;

    let addr = "[::1]:50051".parse().unwrap();

    let server = StreamEchoServer {
        shutdown_tx: Mutex::new(None),
        serve_state: Mutex::new(None),
    };

    let rs = server.start(addr).await.unwrap();
    task::spawn(async move {
        loop {
            println!("{:?}", server.serve_state);
            thread::sleep(Duration::from_secs(10))
        }
    });

    rs.await.unwrap();

    Ok(())
}
