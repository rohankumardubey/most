use futures::future::try_join_all;
use hyper::{body::HttpBody, Uri};
use num_bigint::BigUint;
use std::collections::VecDeque;
use std::io::IoSlice;
use std::time::{Duration, Instant};
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;

const N: usize = 512;

#[tokio::main]
async fn main() {
    let mut body = hyper::Client::new()
        .get(Uri::from_static("http://47.95.111.217:10001"))
        .await
        .unwrap();

    let (tx, mut rx) = tokio::sync::mpsc::channel::<(Instant, Vec<u8>)>(8);
    // responsor
    tokio::spawn(async move {
        let mut dsum = Duration::default();
        let mut count = 0;
        loop {
            let mut stream = TcpStream::connect("47.95.111.217:10002").await.unwrap();
            let (t0, body) = rx.recv().await.unwrap();

            const HEADER: &str = "POST /submit?user=omicron&passwd=y8J6IGKr HTTP/1.1\r\nHost: 47.95.111.217:10002\r\nUser-Agent: Go-http-client/1.1\r\nContent-Type: application/x-www-form-urlencoded\r\n";
            let content_length = format!("Content-Length: {}\r\n\r\n", body.len());
            let iov = [
                IoSlice::new(HEADER.as_bytes()),
                IoSlice::new(content_length.as_bytes()),
                IoSlice::new(&body),
            ];
            stream.write_vectored(&iov).await.unwrap();

            let d = t0.elapsed();
            dsum += d;
            count += 1;
            println!("{:?}\tavg: {:?}", d, dsum / count);
        }
    });

    // $ factor 20220209192254
    // 20220209192254: 2 23 122509 3588061
    // $ factor 104648257118348370704723099
    // 104648257118348370704723099: 104648257118348370704723099
    // $ factor 125000000000000064750000000000009507500000000000294357
    // factor: ‘125000000000000064750000000000009507500000000000294357’ is too large
    let m1 = 20220209192254_u64;
    let m2 = 104648257118348370704723099_u128;
    let m3 = "125000000000000064750000000000009507500000000000294357"
        .parse::<BigUint>()
        .unwrap();
    let m3s = &*Vec::leak((0u8..10).map(|i| &m3 * i).collect());

    let mut deque = VecDeque::new();
    let mut rem: Vec<(u64, u128, BigUint)> = vec![];
    // rem[i][k] = deque[i..] % m[k]
    while let Some(item) = body.data().await {
        let t0 = Instant::now();
        let bytes = item.unwrap();
        let tail_len = deque.len();
        deque.extend(bytes.clone());

        let mut tasks = vec![];
        for i in 0..deque.len() {
            let mut f = match rem.get_mut(i) {
                Some(f) => std::mem::take(f),
                None => Default::default(),
            };
            let deque: &VecDeque<u8> = unsafe { std::mem::transmute(&deque) };
            let tx = tx.clone();
            tasks.push(tokio::spawn(async move {
                if deque[i] == b'0' {
                    return Default::default();
                }
                let zero = BigUint::default();
                for j in tail_len.max(i)..deque.len() {
                    let len = j + 1 - i;
                    if len > N {
                        return f;
                    }
                    let x = deque[j] - b'0';

                    f.0 = (f.0 * 10 + x as u64) % m1;
                    f.1 = (f.1 * 10 + x as u128) % m2;
                    // f.2 = (f.2 * 10u8 + x) % m3;
                    f.2 = f.2 * 10u8 + x;
                    let idx = m3s.partition_point(|m| &f.2 >= m);
                    if idx > 0 {
                        f.2 -= &m3s[idx - 1];
                    }

                    if f.0 == 0 || f.1 == 0 || f.2 == zero {
                        let n: Vec<u8> = deque.range(i..=j).cloned().collect();
                        tx.send((t0, n)).await.unwrap();
                        // println!(
                        //     "{:?}: {}: {}",
                        //     t0.elapsed(),
                        //     ms[_k][0],
                        //     std::str::from_utf8(&n).unwrap()
                        // );
                    }
                }
                f
            }));
        }
        let mut new_rem = try_join_all(tasks).await.unwrap();
        if deque.len() >= N {
            rem = new_rem.split_off(deque.len() - (N - 1));
        } else {
            rem = new_rem;
        }

        while deque.len() >= N {
            deque.pop_front();
        }
    }
}
