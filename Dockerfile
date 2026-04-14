# ─── WASM 编译阶段（v1.6.0，可选）────────────────────────────────────────────
# 取消注释此阶段以在 Docker 内构建 WASM 模块。
# 注意：wasm-pack 安装需要网络访问，构建时间约 3-5 分钟。
# 如已在本地运行 `make wasm` 并提交了 static/wasm/*.wasm 和 *.js 文件，
# 则无需启用此阶段（推荐：在 CI/CD 中预构建 WASM 并提交）。
#
# FROM rust:bookworm as wasm-builder
# RUN apt-get update && apt-get install -y curl pkg-config libssl-dev && \
#     rm -rf /var/lib/apt/lists/* && \
#     curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
# WORKDIR /app
# COPY Cargo.toml Cargo.lock ./
# COPY crates/wasm ./crates/wasm
# RUN mkdir src && echo "fn main() {}" > src/main.rs && \
#     echo "pub fn noop() {}" > src/lib.rs
# RUN wasm-pack build crates/wasm \
#     --target web \
#     --out-dir /wasm-output \
#     --out-name obsidian_mirror_wasm \
#     --release

# ─── 服务端构建阶段 ────────────────────────────────────────────────────────

# 构建阶段 - 使用 Debian 12 (bookworm) 基础的最新 Rust 版本
FROM rust:bookworm as builder

# 安装构建依赖（直接使用默认源，通常 Rust 官方镜像下载速度尚可）
RUN apt-get update && \
    apt-get install -y pkg-config libssl-dev git && \
    rm -rf /var/lib/apt/lists/*

# 设置工作目录
WORKDIR /app

# 复制依赖配置文件
COPY Cargo.toml Cargo.lock ./

# 创建虚拟源文件以预构建依赖（优化缓存）
RUN mkdir src && \
    echo "fn main() {}" > src/main.rs && \
    cargo build --release && \
    rm -rf src target/release/obsidian_mirror target/release/obsidian_mirror.d

# 复制实际源代码
COPY src ./src
COPY templates ./templates
COPY static ./static

# 构建应用（强制重新编译 main.rs）
RUN touch src/main.rs && cargo build --release

# 运行阶段 - 使用相同的 Debian bookworm
FROM debian:bookworm-slim

# 安装运行时依赖（如果 GPG 验证失败，添加 --allow-unauthenticated）
RUN apt-get update && \
    apt-get install -y --allow-unauthenticated ca-certificates git && \
    rm -rf /var/lib/apt/lists/*

# 创建非 root 用户
RUN useradd -m -u 1000 appuser

# 设置工作目录
WORKDIR /app

# 从构建阶段复制二进制文件
COPY --from=builder /app/target/release/obsidian_mirror /app/

# 复制静态资源和模板
COPY --from=builder /app/templates ./templates
COPY --from=builder /app/static ./static
# 若启用了 wasm-builder 阶段，取消注释以下行以包含 WASM 模块：
# COPY --from=wasm-builder /wasm-output/obsidian_mirror_wasm_bg.wasm ./static/wasm/
# COPY --from=wasm-builder /wasm-output/obsidian_mirror_wasm.js ./static/wasm/

# 创建数据目录
RUN mkdir -p /app/data && chown -R appuser:appuser /app

# 切换到非 root 用户
USER appuser

# 暴露端口
EXPOSE 3080

# 设置环境变量
ENV RUST_LOG=info

# 启动命令
CMD ["/app/obsidian_mirror"]
