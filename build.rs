extern crate napi_build;

fn main() {
  let target_triple = std::env::var("TARGET").unwrap();
  // 列出不使用 mimalloc 的目标平台
  let excluded_targets = [
    "x86_64-unknown-linux-gnu",
    "aarch64-unknown-linux-gnu",
    "armv7-unknown-linux-gnueabihf",
    "armv7-linux-androideabi",
  ];
  // 检查当前目标是否在排除列表中
  let use_mimalloc = !excluded_targets.contains(&target_triple.as_str());
  if use_mimalloc {
    // 如果当前目标不在排除列表中，则启用 mimalloc_allocator 特性
    println!("cargo:rustc-cfg=feature=\"mimalloc_allocator\"");
    println!("cargo:rerun-if-changed=build.rs"); // 确保 build.rs 更改时重新运行
    println!("cargo:rerun-if-env-changed=TARGET"); // 确保 TARGET 环境变量更改时重新运行
  } else {
    // 对于排除的平台，不执行任何操作，mimalloc_allocator 特性将不会被激活
    println!(
      "INFO: mimalloc allocator is disabled for target: {}",
      target_triple
    );
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=TARGET");
  }
  napi_build::setup();
}
