extern crate napi_build;

fn main() {
  // 添加自定义 cfg 检查配置，支持精确的目标平台匹配
  println!("cargo:rustc-check-cfg=cfg(target_triple, values(\"x86_64-unknown-linux-gnu\", \"aarch64-unknown-linux-gnu\", \"armv7-unknown-linux-gnueabihf\", \"armv7-linux-androideabi\"))");

  // 设置目标三元组环境变量
  if let Ok(target) = std::env::var("TARGET") {
    println!("cargo:rustc-cfg=target_triple=\"{}\"", target);
  }

  napi_build::setup();
}
