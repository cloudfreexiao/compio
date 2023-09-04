use criterion::{async_executor::AsyncExecutor, criterion_group, criterion_main, Criterion};

criterion_group!(named_pipe, basic);
criterion_main!(named_pipe);

struct CompioRuntime;

impl AsyncExecutor for CompioRuntime {
    fn block_on<T>(&self, future: impl std::future::Future<Output = T>) -> T {
        compio::task::block_on(future)
    }
}

fn basic(c: &mut Criterion) {
    #[allow(dead_code)]
    const PACKET_LEN: usize = 1048576;
    #[allow(dead_code)]
    static PACKET: &[u8] = &[1u8; PACKET_LEN];

    let mut group = c.benchmark_group("basic");

    group.bench_function("tokio", |b| {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        b.to_async(&runtime).iter(|| async {
            #[cfg(target_os = "windows")]
            {
                use tokio::{
                    io::{AsyncReadExt, AsyncWriteExt},
                    net::windows::named_pipe::{ClientOptions, ServerOptions},
                };

                const PIPE_NAME: &str = r"\\.\pipe\tokio-named-pipe";

                let mut server = ServerOptions::new().create(PIPE_NAME).unwrap();
                let mut client = ClientOptions::new().open(PIPE_NAME).unwrap();

                server.connect().await.unwrap();

                server.write_all(PACKET).await.unwrap();

                let mut buffer = Vec::with_capacity(PACKET_LEN);
                while buffer.len() < PACKET_LEN {
                    client.read_buf(&mut buffer).await.unwrap();
                }
                buffer
            }
        })
    });

    group.bench_function("compio", |b| {
        b.to_async(CompioRuntime).iter(|| async {
            #[cfg(target_os = "windows")]
            {
                use compio::named_pipe::{ClientOptions, ServerOptions};

                const PIPE_NAME: &str = r"\\.\pipe\compio-named-pipe";

                let server = ServerOptions::new().create(PIPE_NAME).unwrap();
                let client = ClientOptions::new().open(PIPE_NAME).unwrap();

                server.connect().await.unwrap();

                let write = async {
                    let mut pos = 0;
                    while pos < PACKET_LEN {
                        let (res, _) = server.write(&PACKET[pos..]).await;
                        pos += res.unwrap();
                    }
                };
                let read = async {
                    let mut buffer = Vec::with_capacity(PACKET_LEN);
                    let mut res;
                    while buffer.len() < PACKET_LEN {
                        (res, buffer) = client.read(buffer).await;
                        res.unwrap();
                    }
                    buffer
                };
                futures_util::join!(write, read);
            }
        })
    });
}