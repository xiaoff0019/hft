pub mod cctx;

#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
async fn main() {
    println!("Hello, world!");
}
