use std::hint::black_box;
use std::time::Duration;

use actix_http::header::{
    HeaderMap as ActixHeaderMap, HeaderName as ActixHeaderName, HeaderValue as ActixHeaderValue,
};
use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use http::HeaderMap as HttpHeaderMap;

use chapter_4::header::{HeaderMap, HeaderMap2, HeaderName, HeaderValue, common_headers};

const HOST_VALUE_1_STR: &str = "example.com";
const HOST_VALUE_2_STR: &str = "example.org";

const ACCEPT_VALUE_1_STR: &str = "text/html";
const ACCEPT_VALUE_2_STR: &str = "application/json";

const COOKIE_VALUE_1_STR: &str = "session=60; user_id=1";
const COOKIE_VALUE_2_STR: &str = "session=61; user_id=2";
const COOKIE_VALUE_3_STR: &str = "session=62; user_id=3";
const COOKIE_VALUE_4_STR: &str = "session=63; user_id=4";

const ORIGIN_VALUE_1_STR: &str = "https://example.com";

mod http_headers {
    use super::*;

    pub const HOST_KEY: HeaderName = common_headers::HOST;
    pub const HOST_VALUE_1: HeaderValue = HeaderValue::from_static(HOST_VALUE_1_STR);
    pub const HOST_VALUE_2: HeaderValue = HeaderValue::from_static(HOST_VALUE_2_STR);

    pub const ACCEPT_KEY: HeaderName = common_headers::ACCEPT;
    pub const ACCEPT_VALUE_1: HeaderValue = HeaderValue::from_static(ACCEPT_VALUE_1_STR);
    pub const ACCEPT_VALUE_2: HeaderValue = HeaderValue::from_static(ACCEPT_VALUE_2_STR);

    pub const COOKIE_KEY: HeaderName = common_headers::COOKIE;
    pub const COOKIE_VALUE_1: HeaderValue = HeaderValue::from_static(COOKIE_VALUE_1_STR);
    pub const COOKIE_VALUE_2: HeaderValue = HeaderValue::from_static(COOKIE_VALUE_2_STR);
    pub const COOKIE_VALUE_3: HeaderValue = HeaderValue::from_static(COOKIE_VALUE_3_STR);
    pub const COOKIE_VALUE_4: HeaderValue = HeaderValue::from_static(COOKIE_VALUE_4_STR);

    pub const ORIGIN_KEY: HeaderName = common_headers::ORIGIN;
    pub const ORIGIN_VALUE_1: HeaderValue = HeaderValue::from_static(ORIGIN_VALUE_1_STR);
}

mod actix_headers {
    use super::*;

    pub const HOST_KEY: ActixHeaderName = ActixHeaderName::from_static("host");
    pub const HOST_VALUE_1: ActixHeaderValue = ActixHeaderValue::from_static(HOST_VALUE_1_STR);
    pub const HOST_VALUE_2: ActixHeaderValue = ActixHeaderValue::from_static(HOST_VALUE_2_STR);

    pub const ACCEPT_KEY: ActixHeaderName = ActixHeaderName::from_static("accept");
    pub const ACCEPT_VALUE_1: ActixHeaderValue = ActixHeaderValue::from_static(ACCEPT_VALUE_1_STR);
    pub const ACCEPT_VALUE_2: ActixHeaderValue = ActixHeaderValue::from_static(ACCEPT_VALUE_2_STR);

    pub const COOKIE_KEY: ActixHeaderName = ActixHeaderName::from_static("cookie");
    pub const COOKIE_VALUE_1: ActixHeaderValue = ActixHeaderValue::from_static(COOKIE_VALUE_1_STR);
    pub const COOKIE_VALUE_2: ActixHeaderValue = ActixHeaderValue::from_static(COOKIE_VALUE_2_STR);
    pub const COOKIE_VALUE_3: ActixHeaderValue = ActixHeaderValue::from_static(COOKIE_VALUE_3_STR);
    pub const COOKIE_VALUE_4: ActixHeaderValue = ActixHeaderValue::from_static(COOKIE_VALUE_4_STR);

    pub const ORIGIN_KEY: ActixHeaderName = ActixHeaderName::from_static("origin");
    pub const ORIGIN_VALUE_1: ActixHeaderValue = ActixHeaderValue::from_static(ORIGIN_VALUE_1_STR);
}

mod bytes {
    use super::*;

    pub const HOST_KEY: &[u8] = "host".as_bytes();
    pub const HOST_VALUE_1: &[u8] = HOST_VALUE_1_STR.as_bytes();
    pub const HOST_VALUE_2: &[u8] = HOST_VALUE_2_STR.as_bytes();

    pub const ACCEPT_KEY: &[u8] = "accept".as_bytes();
    pub const ACCEPT_VALUE_1: &[u8] = ACCEPT_VALUE_1_STR.as_bytes();
    pub const ACCEPT_VALUE_2: &[u8] = ACCEPT_VALUE_2_STR.as_bytes();

    pub const COOKIE_KEY: &[u8] = "cookie".as_bytes();
    pub const COOKIE_VALUE_1: &[u8] = COOKIE_VALUE_1_STR.as_bytes();
    pub const COOKIE_VALUE_2: &[u8] = COOKIE_VALUE_2_STR.as_bytes();
    pub const COOKIE_VALUE_3: &[u8] = COOKIE_VALUE_3_STR.as_bytes();
    pub const COOKIE_VALUE_4: &[u8] = COOKIE_VALUE_4_STR.as_bytes();

    pub const ORIGIN_KEY: &[u8] = "origin".as_bytes();
    pub const ORIGIN_VALUE_1: &[u8] = ORIGIN_VALUE_1_STR.as_bytes();
}

fn map_insert(c: &mut Criterion) {
    let mut group = c.benchmark_group("map-inserts");
    group.bench_function("HeaderMap - insert", |b| {
        b.iter_batched_ref(
            HeaderMap::new,
            |map| {
                map.insert(
                    black_box(http_headers::HOST_KEY),
                    black_box(http_headers::HOST_VALUE_1),
                )
                .unwrap();
                map.insert(
                    black_box(http_headers::HOST_KEY),
                    black_box(http_headers::HOST_VALUE_2),
                )
                .unwrap();
                map.insert(
                    black_box(http_headers::ACCEPT_KEY),
                    black_box(http_headers::ACCEPT_VALUE_1),
                )
                .unwrap();
                map.insert(
                    black_box(http_headers::ACCEPT_KEY),
                    black_box(http_headers::ACCEPT_VALUE_2),
                )
                .unwrap();
                map.insert(
                    black_box(http_headers::COOKIE_KEY),
                    black_box(http_headers::COOKIE_VALUE_1),
                )
                .unwrap();
                map.insert(
                    black_box(http_headers::COOKIE_KEY),
                    black_box(http_headers::COOKIE_VALUE_2),
                )
                .unwrap();
                map.insert(
                    black_box(http_headers::COOKIE_KEY),
                    black_box(http_headers::COOKIE_VALUE_3),
                )
                .unwrap();
                map.insert(
                    black_box(http_headers::COOKIE_KEY),
                    black_box(http_headers::COOKIE_VALUE_4),
                )
                .unwrap();
                map.insert(
                    black_box(http_headers::ORIGIN_KEY),
                    black_box(http_headers::ORIGIN_VALUE_1),
                )
                .unwrap();
            },
            BatchSize::SmallInput,
        )
    });
    // group.bench_function("HeaderMap2 - insert", |b| {
    //     b.iter_batched_ref(
    //         HeaderMap2::new,
    //         |map| {
    //             map.insert(
    //                 black_box(bytes::HOST_KEY.into()),
    //                 black_box(bytes::HOST_VALUE_1.into()),
    //             );
    //             map.insert(
    //                 black_box(bytes::HOST_KEY.into()),
    //                 black_box(bytes::HOST_VALUE_2.into()),
    //             );
    //             map.insert(
    //                 black_box(bytes::ACCEPT_KEY.into()),
    //                 black_box(bytes::ACCEPT_VALUE_1.into()),
    //             );
    //             map.insert(
    //                 black_box(bytes::ACCEPT_KEY.into()),
    //                 black_box(bytes::ACCEPT_VALUE_2.into()),
    //             );
    //             map.insert(
    //                 black_box(bytes::COOKIE_KEY.into()),
    //                 black_box(bytes::COOKIE_VALUE_1.into()),
    //             );
    //             map.insert(
    //                 black_box(bytes::COOKIE_KEY.into()),
    //                 black_box(bytes::COOKIE_VALUE_2.into()),
    //             );
    //             map.insert(
    //                 black_box(bytes::COOKIE_KEY.into()),
    //                 black_box(bytes::COOKIE_VALUE_3.into()),
    //             );
    //             map.insert(
    //                 black_box(bytes::COOKIE_KEY.into()),
    //                 black_box(bytes::COOKIE_VALUE_4.into()),
    //             );
    //             map.insert(
    //                 black_box(bytes::ORIGIN_KEY.into()),
    //                 black_box(bytes::ORIGIN_VALUE_1.into()),
    //             );
    //         },
    //         BatchSize::SmallInput,
    //     )
    // });
    group.bench_function("HttpHeaderMap - insert", |b| {
        b.iter_batched_ref(
            HttpHeaderMap::new,
            |map| {
                map.insert(
                    black_box(http_headers::HOST_KEY),
                    black_box(http_headers::HOST_VALUE_1),
                );
                map.insert(
                    black_box(http_headers::HOST_KEY),
                    black_box(http_headers::HOST_VALUE_2),
                );
                map.insert(
                    black_box(http_headers::ACCEPT_KEY),
                    black_box(http_headers::ACCEPT_VALUE_1),
                );
                map.insert(
                    black_box(http_headers::ACCEPT_KEY),
                    black_box(http_headers::ACCEPT_VALUE_2),
                );
                map.insert(
                    black_box(http_headers::COOKIE_KEY),
                    black_box(http_headers::COOKIE_VALUE_1),
                );
                map.insert(
                    black_box(http_headers::COOKIE_KEY),
                    black_box(http_headers::COOKIE_VALUE_2),
                );
                map.insert(
                    black_box(http_headers::COOKIE_KEY),
                    black_box(http_headers::COOKIE_VALUE_3),
                );
                map.insert(
                    black_box(http_headers::COOKIE_KEY),
                    black_box(http_headers::COOKIE_VALUE_4),
                );
                map.insert(
                    black_box(http_headers::ORIGIN_KEY),
                    black_box(http_headers::ORIGIN_VALUE_1),
                );
            },
            BatchSize::SmallInput,
        )
    });
    group.bench_function("ActixHeaderMap - insert", |b| {
        b.iter_batched_ref(
            ActixHeaderMap::new,
            |map| {
                map.insert(
                    black_box(actix_headers::HOST_KEY),
                    black_box(actix_headers::HOST_VALUE_1),
                );
                map.insert(
                    black_box(actix_headers::HOST_KEY),
                    black_box(actix_headers::HOST_VALUE_2),
                );
                map.insert(
                    black_box(actix_headers::ACCEPT_KEY),
                    black_box(actix_headers::ACCEPT_VALUE_1),
                );
                map.insert(
                    black_box(actix_headers::ACCEPT_KEY),
                    black_box(actix_headers::ACCEPT_VALUE_2),
                );
                map.insert(
                    black_box(actix_headers::COOKIE_KEY),
                    black_box(actix_headers::COOKIE_VALUE_1),
                );
                map.insert(
                    black_box(actix_headers::COOKIE_KEY),
                    black_box(actix_headers::COOKIE_VALUE_2),
                );
                map.insert(
                    black_box(actix_headers::COOKIE_KEY),
                    black_box(actix_headers::COOKIE_VALUE_3),
                );
                map.insert(
                    black_box(actix_headers::COOKIE_KEY),
                    black_box(actix_headers::COOKIE_VALUE_4),
                );
                map.insert(
                    black_box(actix_headers::ORIGIN_KEY),
                    black_box(actix_headers::ORIGIN_VALUE_1),
                );
            },
            BatchSize::SmallInput,
        )
    });
    group.finish();
}

fn map_append(c: &mut Criterion) {
    let mut group = c.benchmark_group("map-appends");
    group.bench_function("HeaderMap - append", |b| {
        b.iter_batched_ref(
            HeaderMap::new,
            |map| {
                map.append(
                    black_box(http_headers::HOST_KEY),
                    black_box(http_headers::HOST_VALUE_1),
                )
                .unwrap();
                map.append(
                    black_box(http_headers::HOST_KEY),
                    black_box(http_headers::HOST_VALUE_2),
                )
                .unwrap();
                map.append(
                    black_box(http_headers::ACCEPT_KEY),
                    black_box(http_headers::ACCEPT_VALUE_1),
                )
                .unwrap();
                map.append(
                    black_box(http_headers::ACCEPT_KEY),
                    black_box(http_headers::ACCEPT_VALUE_2),
                )
                .unwrap();
                map.append(
                    black_box(http_headers::COOKIE_KEY),
                    black_box(http_headers::COOKIE_VALUE_1),
                )
                .unwrap();
                map.append(
                    black_box(http_headers::COOKIE_KEY),
                    black_box(http_headers::COOKIE_VALUE_2),
                )
                .unwrap();
                map.append(
                    black_box(http_headers::COOKIE_KEY),
                    black_box(http_headers::COOKIE_VALUE_3),
                )
                .unwrap();
                map.append(
                    black_box(http_headers::COOKIE_KEY),
                    black_box(http_headers::COOKIE_VALUE_4),
                )
                .unwrap();
                map.append(
                    black_box(http_headers::ORIGIN_KEY),
                    black_box(http_headers::ORIGIN_VALUE_1),
                )
                .unwrap();
            },
            BatchSize::SmallInput,
        )
    });
    // group.bench_function("HeaderMap2 - append", |b| {
    //     b.iter_batched_ref(
    //         HeaderMap2::new,
    //         |map| {
    //             map.append(
    //                 black_box(bytes::HOST_KEY.into()),
    //                 black_box(bytes::HOST_VALUE_1.into()),
    //             );
    //             map.append(
    //                 black_box(bytes::HOST_KEY.into()),
    //                 black_box(bytes::HOST_VALUE_2.into()),
    //             );
    //             map.append(
    //                 black_box(bytes::ACCEPT_KEY.into()),
    //                 black_box(bytes::ACCEPT_VALUE_1.into()),
    //             );
    //             map.append(
    //                 black_box(bytes::ACCEPT_KEY.into()),
    //                 black_box(bytes::ACCEPT_VALUE_2.into()),
    //             );
    //             map.append(
    //                 black_box(bytes::COOKIE_KEY.into()),
    //                 black_box(bytes::COOKIE_VALUE_1.into()),
    //             );
    //             map.append(
    //                 black_box(bytes::COOKIE_KEY.into()),
    //                 black_box(bytes::COOKIE_VALUE_2.into()),
    //             );
    //             map.append(
    //                 black_box(bytes::COOKIE_KEY.into()),
    //                 black_box(bytes::COOKIE_VALUE_3.into()),
    //             );
    //             map.append(
    //                 black_box(bytes::COOKIE_KEY.into()),
    //                 black_box(bytes::COOKIE_VALUE_4.into()),
    //             );
    //             map.append(
    //                 black_box(bytes::ORIGIN_KEY.into()),
    //                 black_box(bytes::ORIGIN_VALUE_1.into()),
    //             );
    //         },
    //         BatchSize::SmallInput,
    //     )
    // });
    group.bench_function("HttpHeaderMap - append", |b| {
        b.iter_batched_ref(
            HttpHeaderMap::new,
            |map| {
                map.append(
                    black_box(http_headers::HOST_KEY),
                    black_box(http_headers::HOST_VALUE_1),
                );
                map.append(
                    black_box(http_headers::HOST_KEY),
                    black_box(http_headers::HOST_VALUE_2),
                );
                map.append(
                    black_box(http_headers::ACCEPT_KEY),
                    black_box(http_headers::ACCEPT_VALUE_1),
                );
                map.append(
                    black_box(http_headers::ACCEPT_KEY),
                    black_box(http_headers::ACCEPT_VALUE_2),
                );
                map.append(
                    black_box(http_headers::COOKIE_KEY),
                    black_box(http_headers::COOKIE_VALUE_1),
                );
                map.append(
                    black_box(http_headers::COOKIE_KEY),
                    black_box(http_headers::COOKIE_VALUE_2),
                );
                map.append(
                    black_box(http_headers::COOKIE_KEY),
                    black_box(http_headers::COOKIE_VALUE_3),
                );
                map.append(
                    black_box(http_headers::COOKIE_KEY),
                    black_box(http_headers::COOKIE_VALUE_4),
                );
                map.append(
                    black_box(http_headers::ORIGIN_KEY),
                    black_box(http_headers::ORIGIN_VALUE_1),
                );
            },
            BatchSize::SmallInput,
        )
    });
    group.bench_function("ActixHeaderMap - append", |b| {
        b.iter_batched_ref(
            ActixHeaderMap::new,
            |map| {
                map.append(
                    black_box(actix_headers::HOST_KEY),
                    black_box(actix_headers::HOST_VALUE_1),
                );
                map.append(
                    black_box(actix_headers::HOST_KEY),
                    black_box(actix_headers::HOST_VALUE_2),
                );
                map.append(
                    black_box(actix_headers::ACCEPT_KEY),
                    black_box(actix_headers::ACCEPT_VALUE_1),
                );
                map.append(
                    black_box(actix_headers::ACCEPT_KEY),
                    black_box(actix_headers::ACCEPT_VALUE_2),
                );
                map.append(
                    black_box(actix_headers::COOKIE_KEY),
                    black_box(actix_headers::COOKIE_VALUE_1),
                );
                map.append(
                    black_box(actix_headers::COOKIE_KEY),
                    black_box(actix_headers::COOKIE_VALUE_2),
                );
                map.append(
                    black_box(actix_headers::COOKIE_KEY),
                    black_box(actix_headers::COOKIE_VALUE_3),
                );
                map.append(
                    black_box(actix_headers::COOKIE_KEY),
                    black_box(actix_headers::COOKIE_VALUE_4),
                );
                map.append(
                    black_box(actix_headers::ORIGIN_KEY),
                    black_box(actix_headers::ORIGIN_VALUE_1),
                );
            },
            BatchSize::SmallInput,
        )
    });
    group.finish();
}

fn map_get(c: &mut Criterion) {
    let mut group = c.benchmark_group("map-get");
    group.bench_function("HeaderMap - get", |b| {
        b.iter_batched(
            || {
                let mut map = HeaderMap::new();
                map.append(http_headers::HOST_KEY, http_headers::HOST_VALUE_1)
                    .unwrap();
                map.append(http_headers::HOST_KEY, http_headers::HOST_VALUE_2)
                    .unwrap();
                map.append(http_headers::ACCEPT_KEY, http_headers::ACCEPT_VALUE_1)
                    .unwrap();
                map.append(http_headers::ACCEPT_KEY, http_headers::ACCEPT_VALUE_2)
                    .unwrap();
                map.append(http_headers::COOKIE_KEY, http_headers::COOKIE_VALUE_1)
                    .unwrap();
                map.append(http_headers::COOKIE_KEY, http_headers::COOKIE_VALUE_2)
                    .unwrap();
                map.append(http_headers::COOKIE_KEY, http_headers::COOKIE_VALUE_3)
                    .unwrap();
                map.append(http_headers::COOKIE_KEY, http_headers::COOKIE_VALUE_4)
                    .unwrap();
                map.append(http_headers::ORIGIN_KEY, http_headers::ORIGIN_VALUE_1)
                    .unwrap();
                map
            },
            |map| {
                black_box(map.get(http_headers::HOST_KEY).unwrap());
                black_box(map.get(http_headers::ACCEPT_KEY).unwrap());
                black_box(map.get(http_headers::COOKIE_KEY).unwrap());
                black_box(map.get(http_headers::ORIGIN_KEY).unwrap());
            },
            BatchSize::LargeInput,
        )
    });

    group.bench_function("HttpHeaderMap - get", |b| {
        b.iter_batched(
            || {
                let mut map = HttpHeaderMap::new();
                map.append(http_headers::HOST_KEY, http_headers::HOST_VALUE_1);
                map.append(http_headers::HOST_KEY, http_headers::HOST_VALUE_2);
                map.append(http_headers::ACCEPT_KEY, http_headers::ACCEPT_VALUE_1);
                map.append(http_headers::ACCEPT_KEY, http_headers::ACCEPT_VALUE_2);
                map.append(http_headers::COOKIE_KEY, http_headers::COOKIE_VALUE_1);
                map.append(http_headers::COOKIE_KEY, http_headers::COOKIE_VALUE_2);
                map.append(http_headers::COOKIE_KEY, http_headers::COOKIE_VALUE_3);
                map.append(http_headers::COOKIE_KEY, http_headers::COOKIE_VALUE_4);
                map.append(http_headers::ORIGIN_KEY, http_headers::ORIGIN_VALUE_1);
                map
            },
            |map| {
                black_box(map.get(http_headers::HOST_KEY).unwrap());
                black_box(map.get(http_headers::ACCEPT_KEY).unwrap());
                black_box(map.get(http_headers::COOKIE_KEY).unwrap());
                black_box(map.get(http_headers::ORIGIN_KEY).unwrap());
            },
            BatchSize::LargeInput,
        )
    });

    group.bench_function("ActixHeaderMap - get", |b| {
        b.iter_batched(
            || {
                let mut map = ActixHeaderMap::new();
                map.append(actix_headers::HOST_KEY, actix_headers::HOST_VALUE_1);
                map.append(actix_headers::HOST_KEY, actix_headers::HOST_VALUE_2);
                map.append(actix_headers::ACCEPT_KEY, actix_headers::ACCEPT_VALUE_1);
                map.append(actix_headers::ACCEPT_KEY, actix_headers::ACCEPT_VALUE_2);
                map.append(actix_headers::COOKIE_KEY, actix_headers::COOKIE_VALUE_1);
                map.append(actix_headers::COOKIE_KEY, actix_headers::COOKIE_VALUE_2);
                map.append(actix_headers::COOKIE_KEY, actix_headers::COOKIE_VALUE_3);
                map.append(actix_headers::COOKIE_KEY, actix_headers::COOKIE_VALUE_4);
                map.append(actix_headers::ORIGIN_KEY, actix_headers::ORIGIN_VALUE_1);
                map
            },
            |map| {
                black_box(map.get(actix_headers::HOST_KEY).unwrap());
                black_box(map.get(actix_headers::ACCEPT_KEY).unwrap());
                black_box(map.get(actix_headers::COOKIE_KEY).unwrap());
                black_box(map.get(actix_headers::ORIGIN_KEY).unwrap());
            },
            BatchSize::LargeInput,
        )
    });

    group.finish();
}

fn map_get_all(c: &mut Criterion) {
    let mut group = c.benchmark_group("map-get-all");
    group.bench_function("HeaderMap - get_all", |b| {
        b.iter_batched(
            || {
                let mut map = HeaderMap::new();
                map.append(http_headers::HOST_KEY, http_headers::HOST_VALUE_1)
                    .unwrap();
                map.append(http_headers::HOST_KEY, http_headers::HOST_VALUE_2)
                    .unwrap();
                map.append(http_headers::ACCEPT_KEY, http_headers::ACCEPT_VALUE_1)
                    .unwrap();
                map.append(http_headers::ACCEPT_KEY, http_headers::ACCEPT_VALUE_2)
                    .unwrap();
                map.append(http_headers::COOKIE_KEY, http_headers::COOKIE_VALUE_1)
                    .unwrap();
                map.append(http_headers::COOKIE_KEY, http_headers::COOKIE_VALUE_2)
                    .unwrap();
                map.append(http_headers::COOKIE_KEY, http_headers::COOKIE_VALUE_3)
                    .unwrap();
                map.append(http_headers::COOKIE_KEY, http_headers::COOKIE_VALUE_4)
                    .unwrap();
                map.append(http_headers::ORIGIN_KEY, http_headers::ORIGIN_VALUE_1)
                    .unwrap();
                map
            },
            |map| {
                let _ = map.get_all(black_box(http_headers::HOST_KEY)).unwrap();
                let _ = map.get_all(black_box(http_headers::ACCEPT_KEY)).unwrap();
                let _ = map.get_all(black_box(http_headers::COOKIE_KEY)).unwrap();
                let _ = map.get_all(black_box(http_headers::ORIGIN_KEY)).unwrap();
            },
            BatchSize::LargeInput,
        )
    });

    group.bench_function("HttpHeaderMap - get_all", |b| {
        b.iter_batched(
            || {
                let mut map = HttpHeaderMap::new();
                map.append(http_headers::HOST_KEY, http_headers::HOST_VALUE_1);
                map.append(http_headers::HOST_KEY, http_headers::HOST_VALUE_2);
                map.append(http_headers::ACCEPT_KEY, http_headers::ACCEPT_VALUE_1);
                map.append(http_headers::ACCEPT_KEY, http_headers::ACCEPT_VALUE_2);
                map.append(http_headers::COOKIE_KEY, http_headers::COOKIE_VALUE_1);
                map.append(http_headers::COOKIE_KEY, http_headers::COOKIE_VALUE_2);
                map.append(http_headers::COOKIE_KEY, http_headers::COOKIE_VALUE_3);
                map.append(http_headers::COOKIE_KEY, http_headers::COOKIE_VALUE_4);
                map.append(http_headers::ORIGIN_KEY, http_headers::ORIGIN_VALUE_1);
                map
            },
            |map| {
                map.get_all(black_box(http_headers::HOST_KEY));
                map.get_all(black_box(http_headers::ACCEPT_KEY));
                map.get_all(black_box(http_headers::COOKIE_KEY));
                map.get_all(black_box(http_headers::ORIGIN_KEY));
            },
            BatchSize::LargeInput,
        )
    });

    group.bench_function("ActixHeaderMap - get_all", |b| {
        b.iter_batched(
            || {
                let mut map = ActixHeaderMap::new();
                map.append(actix_headers::HOST_KEY, actix_headers::HOST_VALUE_1);
                map.append(actix_headers::HOST_KEY, actix_headers::HOST_VALUE_2);
                map.append(actix_headers::ACCEPT_KEY, actix_headers::ACCEPT_VALUE_1);
                map.append(actix_headers::ACCEPT_KEY, actix_headers::ACCEPT_VALUE_2);
                map.append(actix_headers::COOKIE_KEY, actix_headers::COOKIE_VALUE_1);
                map.append(actix_headers::COOKIE_KEY, actix_headers::COOKIE_VALUE_2);
                map.append(actix_headers::COOKIE_KEY, actix_headers::COOKIE_VALUE_3);
                map.append(actix_headers::COOKIE_KEY, actix_headers::COOKIE_VALUE_4);
                map.append(actix_headers::ORIGIN_KEY, actix_headers::ORIGIN_VALUE_1);
                map
            },
            |map| {
                let _ = map.get_all(black_box(actix_headers::HOST_KEY));
                let _ = map.get_all(black_box(actix_headers::ACCEPT_KEY));
                let _ = map.get_all(black_box(actix_headers::COOKIE_KEY));
                let _ = map.get_all(black_box(actix_headers::ORIGIN_KEY));
            },
            BatchSize::LargeInput,
        )
    });

    group.finish();
}

const WARMUP: Duration = Duration::from_millis(1000);
const MTIME: Duration = Duration::from_millis(5000);
const SAMPLES: usize = 1000;
criterion_group! {
    name = benches;
    config = Criterion::default().sample_size(SAMPLES).warm_up_time(WARMUP).measurement_time(MTIME);
    targets = map_insert, map_append, map_get, map_get_all
}
criterion_main!(benches);
