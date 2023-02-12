#[macro_use]
extern crate criterion;

use std::process::Command;

use integrity_checker::database::{Database, Features};

use criterion::Criterion;

use tempfile::tempdir;

fn build(c: &mut Criterion) {
    let dir = tempdir().unwrap();
    let tarball = dir.path().join("linux-4.16.7.tar.xz");
    let url = "https://cdn.kernel.org/pub/linux/kernel/v4.x/linux-4.16.7.tar.xz";
    let test_dir = dir.path().join("linux-4.16.7");

    assert!(Command::new("curl")
        .arg(url)
        .arg("-o")
        .arg(tarball.clone())
        .current_dir(dir.path())
        .status()
        .expect("failed to execute curl")
        .success());

    assert!(Command::new("tar")
        .arg("xfJ")
        .arg(tarball)
        .current_dir(dir.path())
        .status()
        .expect("failed to execute tar")
        .success());

    let n = num_cpus::get();
    println!("Running benchmark on {} cores", n);
    let mut g = c.benchmark_group("build");
    g.sample_size(10);
    g.bench_function("linux", move |b| {
        b.iter(|| Database::build(&test_dir, Features::default(), n, false))
    });
    g.finish();
}

criterion_group!(benches, build);
criterion_main!(benches);
