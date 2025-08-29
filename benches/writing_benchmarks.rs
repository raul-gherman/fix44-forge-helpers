use criterion::{
    Criterion,
    criterion_group,
    criterion_main, //
};
use fix44_forge_helpers::*;
use std::hint::black_box;

fn benchmark_writing_integers(c: &mut Criterion) {
    let mut group = c.benchmark_group("writing_integers");
    let mut buf = vec![0u8; 100];

    // u16 benchmarks
    group.bench_function("write_u16_small", |b| {
        b.iter(|| write_u16(black_box(123), black_box(&mut buf), 0))
    });

    group.bench_function("write_u16_max", |b| {
        b.iter(|| write_u16(black_box(65535), black_box(&mut buf), 0))
    });

    // u32 benchmarks
    group.bench_function("write_u32_small", |b| {
        b.iter(|| write_u32(black_box(12345), black_box(&mut buf), 0))
    });

    group.bench_function("write_u32_large", |b| {
        b.iter(|| write_u32(black_box(4294967295), black_box(&mut buf), 0))
    });

    // u64 benchmarks
    group.bench_function("write_u64_small", |b| {
        b.iter(|| write_u64(black_box(123456789), black_box(&mut buf), 0))
    });

    group.bench_function("write_u64_large", |b| {
        b.iter(|| write_u64(black_box(18446744073709551615), black_box(&mut buf), 0))
    });

    // u128 benchmarks
    group.bench_function("write_u128_small", |b| {
        b.iter(|| write_u128(black_box(123456789), black_box(&mut buf), 0))
    });

    group.bench_function("write_u128_large", |b| {
        b.iter(|| {
            write_u128(
                black_box(340282366920938463463374607431768211455),
                black_box(&mut buf),
                0,
            )
        })
    });

    // Signed integer benchmarks
    group.bench_function("write_i32_positive", |b| {
        b.iter(|| write_i32(black_box(123456789), black_box(&mut buf), 0))
    });

    group.bench_function("write_i32_negative", |b| {
        b.iter(|| write_i32(black_box(-123456789), black_box(&mut buf), 0))
    });

    group.bench_function("write_i32_min", |b| {
        b.iter(|| write_i32(black_box(i32::MIN), black_box(&mut buf), 0))
    });

    group.bench_function("write_i64_positive", |b| {
        b.iter(|| write_i64(black_box(9223372036854775807), black_box(&mut buf), 0))
    });

    group.bench_function("write_i64_negative", |b| {
        b.iter(|| write_i64(black_box(-9223372036854775808), black_box(&mut buf), 0))
    });

    group.finish();
}

fn benchmark_writing_floats(c: &mut Criterion) {
    let mut group = c.benchmark_group("writing_floats");
    let mut buf = vec![0u8; 100];

    group.bench_function("write_f32_integer", |b| {
        b.iter(|| write_f32(black_box(123.0), black_box(&mut buf), 0))
    });

    group.bench_function("write_f32_decimal", |b| {
        b.iter(|| write_f32(black_box(123.456789), black_box(&mut buf), 0))
    });

    group.bench_function("write_f32_negative", |b| {
        b.iter(|| write_f32(black_box(-123.456789), black_box(&mut buf), 0))
    });

    group.bench_function("write_f32_zero", |b| {
        b.iter(|| write_f32(black_box(0.0), black_box(&mut buf), 0))
    });

    group.bench_function("write_f64_integer", |b| {
        b.iter(|| write_f64(black_box(123456789.0), black_box(&mut buf), 0))
    });

    group.bench_function("write_f64_decimal", |b| {
        b.iter(|| write_f64(black_box(123.456789012345), black_box(&mut buf), 0))
    });

    group.bench_function("write_f64_negative", |b| {
        b.iter(|| write_f64(black_box(-123.456789012345), black_box(&mut buf), 0))
    });

    group.bench_function("write_f64_zero", |b| {
        b.iter(|| write_f64(black_box(0.0), black_box(&mut buf), 0))
    });

    group.finish();
}

fn benchmark_tag_writing(c: &mut Criterion) {
    let mut group = c.benchmark_group("tag_writing");
    let mut buf = vec![0u8; 100];

    group.bench_function("write_tag_and_bool", |b| {
        b.iter(|| write_tag_and_bool(black_box(&mut buf), 0, black_box(b"54="), black_box(true)))
    });

    group.bench_function("write_tag_and_str", |b| {
        b.iter(|| write_tag_and_str(black_box(&mut buf), 0, black_box(b"35="), black_box("D")))
    });

    group.bench_function("write_tag_and_u32", |b| {
        b.iter(|| write_tag_and_u32(black_box(&mut buf), 0, black_box(b"34="), black_box(12345)))
    });

    group.bench_function("write_tag_and_u64", |b| {
        b.iter(|| {
            write_tag_and_u64(
                black_box(&mut buf),
                0,
                black_box(b"38="),
                black_box(1000000),
            )
        })
    });

    group.bench_function("write_tag_and_f32", |b| {
        b.iter(|| {
            write_tag_and_f32(
                black_box(&mut buf),
                0,
                black_box(b"44="),
                black_box(123.456),
            )
        })
    });

    group.bench_function("write_tag_and_f64", |b| {
        b.iter(|| {
            write_tag_and_f64(
                black_box(&mut buf),
                0,
                black_box(b"44="),
                black_box(123.456789),
            )
        })
    });

    group.bench_function("write_tag_and_bytes", |b| {
        b.iter(|| write_tag_and_bytes(black_box(&mut buf), 0, black_box(b"35="), black_box(b"D")))
    });

    group.bench_function("write_tag_and_u16", |b| {
        b.iter(|| {
            write_tag_and_u16(
                black_box(&mut buf),
                0,
                black_box(b"34="),
                black_box(12345u16),
            )
        })
    });

    group.bench_function("write_tag_and_i16", |b| {
        b.iter(|| {
            write_tag_and_i16(
                black_box(&mut buf),
                0,
                black_box(b"34="),
                black_box(-12345i16),
            )
        })
    });

    group.bench_function("write_tag_and_i32", |b| {
        b.iter(|| {
            write_tag_and_i32(
                black_box(&mut buf),
                0,
                black_box(b"34="),
                black_box(-123456789i32),
            )
        })
    });

    group.bench_function("write_tag_and_i64", |b| {
        b.iter(|| {
            write_tag_and_i64(
                black_box(&mut buf),
                0,
                black_box(b"52="),
                black_box(-1234567890123i64),
            )
        })
    });

    group.finish();
}

fn benchmark_special_functions(c: &mut Criterion) {
    let mut group = c.benchmark_group("special_functions");
    let mut buf = vec![0u8; 100];

    group.bench_function("write_tag_and_current_timestamp", |b| {
        b.iter(|| write_tag_and_current_timestamp(black_box(&mut buf), 0, black_box(b"52=")))
    });

    group.bench_function("format_timestamp_from_timespec", |b| {
        // Fixed timespec to measure pure formatting cost (avoids syscall in loop)
        let ts = libc::timespec {
            tv_sec: 1_700_000_000,
            tv_nsec: 987_654_321,
        };
        b.iter(|| {
            format_timestamp_from_timespec(
                black_box(&mut buf),
                0,
                black_box(b"52="),
                black_box(&ts),
            )
        })
    });

    group.bench_function("write_current_logging_timestamp", |b| {
        b.iter(|| write_current_logging_timestamp(black_box(&mut buf), 0))
    });

    group.bench_function("format_logging_timestamp_from_timespec", |b| {
        let ts = libc::timespec {
            tv_sec: 1_700_000_000,
            tv_nsec: 123_456_789,
        };
        b.iter(|| format_logging_timestamp_from_timespec(black_box(&mut buf), 0, black_box(&ts)))
    });

    group.bench_function("write_tag_and_ClOrdID", |b| {
        b.iter(|| write_tag_and_ClOrdID(black_box(&mut buf), 0, black_box(b"11=")))
    });

    group.bench_function("encode_base36_fixed13", |b| {
        b.iter(|| encode_base36_fixed13(black_box(&mut buf), 0, black_box(12345678901234567890)))
    });

    group.finish();
}

fn benchmark_mixed_writing(c: &mut Criterion) {
    let mut group = c.benchmark_group("mixed_writing");
    let mut buf = vec![0u8; 500];

    // Simulate writing multiple fields as might occur in a FIX message
    group.bench_function("write_fix_like_message", |b| {
        b.iter(|| {
            let mut pos = 0;
            pos += write_tag_and_str(black_box(&mut buf), pos, b"8=", "FIX.4.4");
            pos += write_tag_and_u32(black_box(&mut buf), pos, b"34=", 123);
            pos += write_tag_and_str(black_box(&mut buf), pos, b"35=", "D");
            pos += write_tag_and_str(black_box(&mut buf), pos, b"49=", "SENDER");
            pos += write_tag_and_str(black_box(&mut buf), pos, b"56=", "TARGET");
            pos += write_tag_and_current_timestamp(black_box(&mut buf), pos, b"52=");
            pos += write_tag_and_ClOrdID(black_box(&mut buf), pos, b"11=");
            pos += write_tag_and_str(black_box(&mut buf), pos, b"55=", "MSFT");
            pos += write_tag_and_u64(black_box(&mut buf), pos, b"38=", 1000);
            pos += write_tag_and_f64(black_box(&mut buf), pos, b"44=", 123.45);
            pos += write_tag_and_bool(black_box(&mut buf), pos, b"59=", false);
            black_box(pos);
        })
    });

    group.finish();
}

fn benchmark_forge_buffer(c: &mut Criterion) {
    let mut group = c.benchmark_group("forge_buffer");

    group.bench_function("forge_out_buffer_creation", |b| {
        b.iter(|| black_box(forge_out_buffer("FIX.4.4")))
    });

    group.bench_function("forge_out_buffer_with_immediate_write", |b| {
        b.iter(|| {
            let mut buffer = forge_out_buffer("FIX.4.4");
            let mut pos = FORGE_WRITE_START;

            // Write MsgType value directly (tag "35=" already there)
            black_box(&mut buffer)[pos] = b'D';
            pos += 1;
            black_box(&mut buffer)[pos] = 0x01;
            pos += 1;

            pos += write_tag_and_u32(
                black_box(&mut buffer),
                pos,
                black_box(b"34="),
                black_box(123),
            );

            update_body_length(black_box(&mut buffer), pos);
            black_box(buffer)
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    benchmark_writing_integers,
    benchmark_writing_floats,
    benchmark_tag_writing,
    benchmark_special_functions,
    benchmark_mixed_writing,
    benchmark_forge_buffer
);
criterion_main!(benches);
