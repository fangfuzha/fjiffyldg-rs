use criterion::{black_box, criterion_group, criterion_main, Criterion};
use fjiffyldg::Fjiffyldg;
use std::io::Write;
use tempfile::NamedTempFile;

/// 单行测试数据长度。
const LINE_LEN: usize = 80;
/// 基准文件行数，文件大小约 12MB，足以触发 mmap 路径。
const LINE_COUNT: usize = 150_000;

/// 创建可复用的大文件基准输入。
fn create_large_temp_file() -> NamedTempFile {
    let mut temp = NamedTempFile::new().expect("create benchmark temp file");
    let line = format!("{:079}\n", "x");

    for _ in 0..LINE_COUNT {
        temp.write_all(line.as_bytes())
            .expect("write benchmark line");
    }
    temp.flush().expect("flush benchmark temp file");
    temp
}

/// 加载并等待后台扫描完成。
fn load_and_wait(path: &std::path::Path) -> Fjiffyldg {
    let model = Fjiffyldg::new();
    model.load_and_scan(path).expect("load benchmark file");
    model.wait_scan();
    model
}

/// 基准：大文件 mmap 加载与完整行扫描。
fn bench_load_and_scan(c: &mut Criterion) {
    let temp = create_large_temp_file();

    c.bench_function("large_file_load_and_scan", |b| {
        b.iter(|| {
            let model = load_and_wait(temp.path());
            black_box(model.line_count());
        });
    });
}

/// 基准：百万行级索引上的随机行号与位置查询。
fn bench_random_line_queries(c: &mut Criterion) {
    let temp = create_large_temp_file();
    let model = load_and_wait(temp.path());
    let targets = [0, LINE_COUNT / 4, LINE_COUNT / 2, LINE_COUNT - 1];

    c.bench_function("large_file_random_line_queries", |b| {
        b.iter(|| {
            for target in targets {
                let line = target as i64;
                let pos = model.line_pos(line);
                black_box(pos);
                black_box(model.line_at_pos(pos));
            }
        });
    });
}

/// 基准：大文件 mmap 路径上的随机读取。
fn bench_random_reads(c: &mut Criterion) {
    let temp = create_large_temp_file();
    let model = load_and_wait(temp.path());
    let targets = [0, LINE_COUNT / 3, LINE_COUNT / 2, LINE_COUNT - 1];

    c.bench_function("large_file_random_reads", |b| {
        b.iter(|| {
            for target in targets {
                let pos = (target * LINE_LEN) as i64;
                black_box(model.read(pos, LINE_LEN));
            }
        });
    });
}

criterion_group!(
    large_file_benches,
    bench_load_and_scan,
    bench_random_line_queries,
    bench_random_reads
);
criterion_main!(large_file_benches);
