use std::hint::black_box;
use std::time::Duration;

use criterion::{BatchSize, Criterion, Throughput, criterion_group, criterion_main};

use chapter_4::request::RequestParser;

const REQ_SHORT: &[u8] = b"\
GET / HTTP/1.1\r\n\
Host: example.com\r\n\
Cookie: session=60; user_id=1\r\n\r\n";

const REQ: &[u8] = b"\
GET /wp-content/uploads/2010/03/hello-kitty-darth-vader-pink.jpg HTTP/1.1\r\n\
Host: www.kittyhell.com\r\n\
User-Agent: Mozilla/5.0 (Macintosh; U; Intel Mac OS X 10.6; ja-JP-mac; rv:1.9.2.3) Gecko/20100401 Firefox/3.6.3 Pathtraq/0.9\r\n\
Accept: text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8\r\n\
Accept-Language: ja,en-us;q=0.7,en;q=0.3\r\n\
Accept-Encoding: gzip,deflate\r\n\
Accept-Charset: Shift_JIS,utf-8;q=0.7,*;q=0.7\r\n\
Keep-Alive: 115\r\n\
Connection: keep-alive\r\n\
Cookie: wp_ozh_wsa_visits=2; wp_ozh_wsa_visit_lasttime=xxxxxxxxxx; __utma=xxxxxxxxx.xxxxxxxxxx.xxxxxxxxxx.xxxxxxxxxx.xxxxxxxxxx.x; __utmz=xxxxxxxxx.xxxxxxxxxx.x.x.utmccn=(referral)|utmcsr=reader.livedoor.com|utmcct=/reader/|utmcmd=referral|padding=under256\r\n\r\n";

fn req(c: &mut Criterion) {
    let mut group = c.benchmark_group("req");

    group.throughput(Throughput::Bytes(REQ.len() as u64));

    group.bench_function("req - httparse", |b| {
        b.iter_batched_ref(
            || {
                [httparse::Header {
                    name: "",
                    value: &[],
                }; 16]
            },
            |headers| {
                let mut req = httparse::Request::new(headers);
                assert_eq!(
                    req.parse(black_box(REQ)).unwrap(),
                    httparse::Status::Complete(REQ.len())
                );
            },
            BatchSize::SmallInput,
        )
    });

    group.bench_function("req - request parser", |b| {
        b.iter(|| {
            let mut parser = RequestParser::new(0);
            // assert_eq!(parser.parse(black_box(REQ)).unwrap(), REQ.len());
            assert_eq!(parser.parse(black_box(REQ)).unwrap(), 74);
        })
    });
}

fn req_short(c: &mut Criterion) {
    let mut group = c.benchmark_group("req_short");

    group.throughput(Throughput::Bytes(REQ_SHORT.len() as u64));

    group.bench_function("req_short - httparse", |b| {
        b.iter_batched_ref(
            || {
                [httparse::Header {
                    name: "",
                    value: &[],
                }; 16]
            },
            |headers| {
                let mut req = httparse::Request::new(headers);
                assert_eq!(
                    req.parse(black_box(REQ_SHORT)).unwrap(),
                    httparse::Status::Complete(REQ_SHORT.len())
                );
            },
            BatchSize::SmallInput,
        )
    });

    group.bench_function("req - request parser", |b| {
        b.iter(|| {
            let mut parser = RequestParser::new(0);
            // assert_eq!(parser.parse(black_box(REQ_SHORT)).unwrap(), REQ_SHORT.len());
            assert_eq!(parser.parse(black_box(REQ_SHORT)).unwrap(), 15);
        })
    });
}

const WARMUP: Duration = Duration::from_millis(1000);
const MTIME: Duration = Duration::from_millis(1000);
const SAMPLES: usize = 200;
criterion_group! {
    name = benches;
    config = Criterion::default().sample_size(SAMPLES).warm_up_time(WARMUP).measurement_time(MTIME);
    targets = req, req_short
}
criterion_main!(benches);
