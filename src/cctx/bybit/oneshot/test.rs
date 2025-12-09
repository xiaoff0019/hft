use super::*;
use std::sync::LazyLock;

static BYBIT: LazyLock<Bybit> = LazyLock::new(|| test_bybit());

fn test_bybit() -> Bybit {
    Bybit::new("123", "456").unwrap()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_fetch_time() {
    let time = BYBIT.fetch_time().await;
    assert!(time.is_ok());
    println!("{time:#?}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_fetch_currencies() {
    let currencies = BYBIT.fetch_currencies().await;
    assert!(currencies.is_ok());
    println!("{currencies:#?}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_fetch_spot_markets() {
    let spot_markets = BYBIT.fetch_spot_markets(&[]).await;
    // assert!(spot_markets.is_ok());
    // let spot_markets = spot_markets.unwrap();
    println!("next is {:#?}", spot_markets);
}
